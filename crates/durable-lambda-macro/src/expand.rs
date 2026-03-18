//! Macro expansion logic — code generation for `#[durable_execution]`.
//!
//! Generates `lambda_runtime` registration + `DurableContext` setup,
//! mirroring the `#[tokio::main]` pattern.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, FnArg, ItemFn, PatType, ReturnType, Type};

/// Validate the annotated function and generate the expanded code.
///
/// Returns the original handler function plus a generated `main()` that wires
/// up the Lambda runtime, AWS backend, and durable execution event parsing.
pub(crate) fn expand_durable_execution(func: ItemFn) -> Result<TokenStream, Error> {
    validate_signature(&func)?;

    let fn_name = &func.sig.ident;

    Ok(quote! {
        #func

        #[tokio::main]
        async fn main() -> ::std::result::Result<(), ::lambda_runtime::Error> {
            let config = ::aws_config::load_defaults(::aws_config::BehaviorVersion::latest()).await;
            let client = ::aws_sdk_lambda::Client::new(&config);
            let backend = ::std::sync::Arc::new(
                ::durable_lambda_core::backend::RealBackend::new(client),
            );

            ::lambda_runtime::run(::lambda_runtime::service_fn(
                |event: ::lambda_runtime::LambdaEvent<::serde_json::Value>| {
                    let backend = backend.clone();
                    async move {
                        let (payload, _lambda_ctx) = event.into_parts();

                        let invocation =
                            ::durable_lambda_core::event::parse_invocation(&payload)
                                .map_err(|e| {
                                    ::std::boxed::Box::<
                                        dyn ::std::error::Error
                                            + ::std::marker::Send
                                            + ::std::marker::Sync,
                                    >::from(e)
                                })?;

                        let durable_ctx = ::durable_lambda_core::context::DurableContext::new(
                            backend,
                            invocation.durable_execution_arn,
                            invocation.checkpoint_token,
                            invocation.operations,
                            invocation.next_marker,
                        )
                        .await
                        .map_err(|e| {
                            ::std::boxed::Box::new(e)
                                as ::std::boxed::Box<
                                    dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync,
                                >
                        })?;

                        let result = #fn_name(invocation.user_event, durable_ctx).await;

                        // Wrap the result in the durable execution invocation output envelope.
                        ::durable_lambda_core::response::wrap_handler_result(result)
                    }
                },
            ))
            .await
        }
    })
}

/// Validate that the annotated function has the correct signature.
///
/// Requirements:
/// - Must be `async`
/// - Must have exactly 2 parameters
/// - Second parameter must be `DurableContext`
/// - Return type must be `Result<...>`
fn validate_signature(func: &ItemFn) -> Result<(), Error> {
    if func.sig.asyncness.is_none() {
        return Err(Error::new_spanned(
            func.sig.fn_token,
            "#[durable_execution] requires an async function",
        ));
    }

    let param_count = func.sig.inputs.len();
    if param_count != 2 {
        return Err(Error::new_spanned(
            &func.sig.inputs,
            format!(
                "#[durable_execution] requires exactly 2 parameters \
                 (event: serde_json::Value, ctx: DurableContext), found {param_count}"
            ),
        ));
    }

    // FEAT-29: Second parameter must be DurableContext.
    // Safety: param_count == 2 is guaranteed above, so nth(1) always yields Some.
    let second = func.sig.inputs.iter().nth(1).expect("param_count == 2");
    if let FnArg::Typed(PatType { ty, .. }) = second {
        let is_durable_context = if let Type::Path(type_path) = ty.as_ref() {
            type_path
                .path
                .segments
                .last()
                .map(|seg| seg.ident == "DurableContext")
                .unwrap_or(false)
        } else {
            false
        };
        if !is_durable_context {
            return Err(Error::new_spanned(
                ty,
                "#[durable_execution] second parameter must be DurableContext — \
                 expected: async fn handler(event: serde_json::Value, ctx: DurableContext) \
                 -> Result<serde_json::Value, DurableError>",
            ));
        }
    }

    // FEAT-30: Return type must be Result<...>.
    match &func.sig.output {
        ReturnType::Default => {
            return Err(Error::new_spanned(
                func.sig.fn_token,
                "#[durable_execution] must explicitly return \
                 Result<serde_json::Value, DurableError>",
            ));
        }
        ReturnType::Type(_, boxed_type) => {
            let is_result = if let Type::Path(type_path) = boxed_type.as_ref() {
                type_path
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident == "Result")
                    .unwrap_or(false)
            } else {
                false
            };
            if !is_result {
                return Err(Error::new_spanned(
                    boxed_type,
                    "#[durable_execution] return type must be \
                     Result<serde_json::Value, DurableError> — found non-Result type",
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn valid_async_handler_expands() {
        let func: ItemFn = parse_quote! {
            async fn handler(
                event: serde_json::Value,
                mut ctx: DurableContext,
            ) -> Result<serde_json::Value, DurableError> {
                Ok(event)
            }
        };
        let result = expand_durable_execution(func);
        assert!(result.is_ok(), "expansion should succeed for valid handler");
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("async fn main"), "should generate main()");
        assert!(
            tokens.contains("handler"),
            "should reference the handler function"
        );
        assert!(
            tokens.contains("parse_invocation"),
            "should call parse_invocation"
        );
    }

    #[test]
    fn rejects_non_async_function() {
        let func: ItemFn = parse_quote! {
            fn handler(event: serde_json::Value, ctx: DurableContext) -> Result<serde_json::Value, DurableError> {
                todo!()
            }
        };
        let result = expand_durable_execution(func);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("async"),
            "error should mention async requirement: {err}"
        );
    }

    #[test]
    fn rejects_wrong_parameter_count_zero() {
        let func: ItemFn = parse_quote! {
            async fn handler() -> Result<serde_json::Value, DurableError> {
                todo!()
            }
        };
        let result = expand_durable_execution(func);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("2 parameters"),
            "error should mention 2 params: {err}"
        );
    }

    #[test]
    fn rejects_wrong_parameter_count_one() {
        let func: ItemFn = parse_quote! {
            async fn handler(event: serde_json::Value) -> Result<serde_json::Value, DurableError> {
                todo!()
            }
        };
        let result = expand_durable_execution(func);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("found 1"), "error should say found 1: {err}");
    }

    #[test]
    fn rejects_wrong_parameter_count_three() {
        let func: ItemFn = parse_quote! {
            async fn handler(a: i32, b: i32, c: i32) -> Result<serde_json::Value, DurableError> {
                todo!()
            }
        };
        let result = expand_durable_execution(func);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("found 3"), "error should say found 3: {err}");
    }

    #[test]
    fn preserves_original_function_name() {
        let func: ItemFn = parse_quote! {
            async fn my_custom_handler(
                event: serde_json::Value,
                ctx: DurableContext,
            ) -> Result<serde_json::Value, DurableError> {
                Ok(event)
            }
        };
        let result = expand_durable_execution(func).unwrap();
        let tokens = result.to_string();
        assert!(
            tokens.contains("my_custom_handler"),
            "should preserve original function name"
        );
    }

    #[test]
    fn generated_code_uses_fully_qualified_paths() {
        let func: ItemFn = parse_quote! {
            async fn handler(
                event: serde_json::Value,
                mut ctx: DurableContext,
            ) -> Result<serde_json::Value, DurableError> {
                Ok(event)
            }
        };
        let tokens = expand_durable_execution(func).unwrap().to_string();
        assert!(
            tokens.contains("durable_lambda_core"),
            "should use fully qualified core paths"
        );
        assert!(
            tokens.contains("lambda_runtime"),
            "should reference lambda_runtime"
        );
        assert!(tokens.contains("aws_config"), "should reference aws_config");
    }

    #[test]
    fn rejects_wrong_second_param_type() {
        let func: ItemFn = parse_quote! {
            async fn handler(x: i32, y: i32) -> Result<serde_json::Value, DurableError> {
                todo!()
            }
        };
        let result = expand_durable_execution(func);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("DurableContext"),
            "error should mention DurableContext: {err}"
        );
    }

    #[test]
    fn rejects_non_result_return_type() {
        let func: ItemFn = parse_quote! {
            async fn handler(event: serde_json::Value, ctx: DurableContext) -> String {
                String::new()
            }
        };
        let result = expand_durable_execution(func);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Result"), "error should mention Result: {err}");
    }

    #[test]
    fn rejects_missing_return_type() {
        let func: ItemFn = parse_quote! {
            async fn handler(event: serde_json::Value, ctx: DurableContext) {
                let _ = (event, ctx);
            }
        };
        let result = expand_durable_execution(func);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Result"), "error should mention Result: {err}");
    }

    #[test]
    fn accepts_mut_binding_on_context() {
        let func: ItemFn = parse_quote! {
            async fn handler(
                event: serde_json::Value,
                mut ctx: DurableContext,
            ) -> Result<serde_json::Value, DurableError> {
                Ok(event)
            }
        };
        let result = expand_durable_execution(func);
        assert!(
            result.is_ok(),
            "mut ctx binding should be accepted: {:?}",
            result.err()
        );
    }
}
