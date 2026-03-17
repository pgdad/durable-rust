//! Builder-pattern handler construction.
//!
//! Provides the [`DurableHandlerBuilder`] and [`handler`] entry point for
//! builder-pattern durable Lambda handlers (FR35).
//! Internally wires up `lambda_runtime`, AWS config, and `DurableContext` creation
//! so users never interact with these directly.

use std::future::Future;
use std::sync::Arc;

use durable_lambda_core::backend::RealBackend;
use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::event::parse_invocation;
use lambda_runtime::{service_fn, LambdaEvent};

use crate::context::BuilderContext;

/// A builder for constructing durable Lambda handlers.
///
/// Created via the [`handler`] function. Call [`.run()`](Self::run) to start
/// the Lambda runtime.
///
/// # Examples
///
/// ```no_run
/// use durable_lambda_builder::prelude::*;
///
/// #[tokio::main]
/// async fn main() -> Result<(), lambda_runtime::Error> {
///     durable_lambda_builder::handler(|event: serde_json::Value, mut ctx: BuilderContext| async move {
///         let result: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await?;
///         Ok(serde_json::json!({"result": result.unwrap()}))
///     })
///     .run()
///     .await
/// }
/// ```
pub struct DurableHandlerBuilder<F, Fut>
where
    F: Fn(serde_json::Value, BuilderContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    handler: F,
    _phantom: std::marker::PhantomData<Fut>,
}

/// Create a new [`DurableHandlerBuilder`] from a handler function.
///
/// This is the entry point for the builder-pattern API. The returned builder
/// can be configured and then executed with [`.run()`](DurableHandlerBuilder::run).
///
/// # Arguments
///
/// * `f` — An async function taking the user event and a `BuilderContext`,
///   returning `Result<serde_json::Value, DurableError>`
///
/// # Examples
///
/// ```no_run
/// use durable_lambda_builder::prelude::*;
///
/// #[tokio::main]
/// async fn main() -> Result<(), lambda_runtime::Error> {
///     durable_lambda_builder::handler(|event: serde_json::Value, mut ctx: BuilderContext| async move {
///         let result: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await?;
///         Ok(serde_json::json!({"result": result.unwrap()}))
///     })
///     .run()
///     .await
/// }
/// ```
pub fn handler<F, Fut>(f: F) -> DurableHandlerBuilder<F, Fut>
where
    F: Fn(serde_json::Value, BuilderContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    DurableHandlerBuilder {
        handler: f,
        _phantom: std::marker::PhantomData,
    }
}

impl<F, Fut> DurableHandlerBuilder<F, Fut>
where
    F: Fn(serde_json::Value, BuilderContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    /// Consume the builder and start the Lambda runtime.
    ///
    /// This method:
    /// 1. Initializes AWS configuration and creates a Lambda client
    /// 2. Creates a [`RealBackend`] for durable execution API calls
    /// 3. Registers with `lambda_runtime` to receive invocations
    /// 4. On each invocation, extracts durable execution metadata from the event,
    ///    creates a [`BuilderContext`], and calls the user handler
    ///
    /// # Errors
    ///
    /// Returns `lambda_runtime::Error` if the Lambda runtime fails to start or
    /// encounters a fatal error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use durable_lambda_builder::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), lambda_runtime::Error> {
    ///     durable_lambda_builder::handler(|event: serde_json::Value, mut ctx: BuilderContext| async move {
    ///         Ok(serde_json::json!({"ok": true}))
    ///     })
    ///     .run()
    ///     .await
    /// }
    /// ```
    pub async fn run(self) -> Result<(), lambda_runtime::Error> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = aws_sdk_lambda::Client::new(&config);
        let backend = Arc::new(RealBackend::new(client));

        lambda_runtime::run(service_fn(|event: LambdaEvent<serde_json::Value>| {
            let backend = backend.clone();
            let handler = &self.handler;
            async move {
                let (payload, _lambda_ctx) = event.into_parts();

                // Parse all durable execution fields from the Lambda event.
                let invocation = parse_invocation(&payload)
                    .map_err(Box::<dyn std::error::Error + Send + Sync>::from)?;

                // Create DurableContext and wrap in BuilderContext.
                let durable_ctx = DurableContext::new(
                    backend,
                    invocation.durable_execution_arn,
                    invocation.checkpoint_token,
                    invocation.operations,
                    invocation.next_marker,
                )
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                let builder_ctx = BuilderContext::new(durable_ctx);

                // Call the user handler with owned context.
                let result = handler(invocation.user_event, builder_ctx)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                Ok::<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>(result)
            }
        }))
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::BuilderContext;
    use tracing_subscriber::fmt;

    #[test]
    fn test_builder_construction_and_type_correctness() {
        // Verify the handler() constructor creates a DurableHandlerBuilder
        // and the type system enforces correct handler signatures.
        let _builder = handler(
            |_event: serde_json::Value, _ctx: BuilderContext| async move {
                Ok(serde_json::json!({"ok": true}))
            },
        );
        // If this compiles, the builder type is correct.
    }

    #[test]
    fn test_builder_run_returns_future() {
        // Verify that .run() returns a Future (type-level check).
        let builder = handler(
            |_event: serde_json::Value, _ctx: BuilderContext| async move {
                Ok(serde_json::json!({"ok": true}))
            },
        );
        // run() is async — calling it without .await produces a Future.
        // We just verify the method exists and returns the right type.
        let _future = builder.run();
        // Drop without awaiting — we can't start lambda_runtime in tests.
    }

    #[test]
    fn test_with_tracing_stores_subscriber() {
        // Verify handler(fn).with_tracing(subscriber) compiles and stores the subscriber.
        let subscriber = fmt().finish();
        let _builder = handler(
            |_event: serde_json::Value, _ctx: BuilderContext| async move {
                Ok(serde_json::json!({"ok": true}))
            },
        )
        .with_tracing(subscriber);
        // If this compiles, the with_tracing() method exists and accepts a Subscriber.
    }

    #[test]
    fn test_with_error_handler_stores_handler() {
        // Verify handler(fn).with_error_handler(fn) compiles and stores the error handler.
        let _builder = handler(
            |_event: serde_json::Value, _ctx: BuilderContext| async move {
                Ok(serde_json::json!({"ok": true}))
            },
        )
        .with_error_handler(|e: DurableError| e);
        // If this compiles, the with_error_handler() method exists and accepts a closure.
    }

    #[test]
    fn test_builder_chaining() {
        // Verify method chaining: handler(fn).with_tracing(sub).with_error_handler(fn) compiles.
        let subscriber = fmt().finish();
        let _builder = handler(
            |_event: serde_json::Value, _ctx: BuilderContext| async move {
                Ok(serde_json::json!({"ok": true}))
            },
        )
        .with_tracing(subscriber)
        .with_error_handler(|e: DurableError| e);
        // If this compiles, method chaining is correctly supported.
    }

    #[test]
    fn test_builder_without_config_backward_compatible() {
        // Verify that handler(fn).run() still works without calling with_tracing or with_error_handler.
        let builder = handler(
            |_event: serde_json::Value, _ctx: BuilderContext| async move {
                Ok(serde_json::json!({"ok": true}))
            },
        );
        // Confirm .run() method still exists and returns a Future (backward compat).
        let _future = builder.run();
        // Drop without awaiting — we can't start lambda_runtime in tests.
    }
}
