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
use durable_lambda_core::response::wrap_handler_result;
use lambda_runtime::{service_fn, LambdaEvent};

use crate::context::BuilderContext;

/// A builder for constructing durable Lambda handlers.
///
/// Created via the [`handler`] function. Call [`.run()`](Self::run) to start
/// the Lambda runtime. Optionally configure tracing and error handling before
/// calling `.run()` using the builder methods.
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
    tracing_subscriber: Option<Box<dyn tracing::Subscriber + Send + Sync + 'static>>,
    error_handler: Option<Box<dyn Fn(DurableError) -> DurableError + Send + Sync>>,
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
        tracing_subscriber: None,
        error_handler: None,
    }
}

impl<F, Fut> DurableHandlerBuilder<F, Fut>
where
    F: Fn(serde_json::Value, BuilderContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    /// Configure a tracing subscriber to install before the Lambda runtime starts.
    ///
    /// The provided subscriber is installed via [`tracing::subscriber::set_global_default`]
    /// when [`run()`](Self::run) is called, before any Lambda invocations are processed.
    ///
    /// # Arguments
    ///
    /// * `subscriber` — Any type implementing [`tracing::Subscriber`] + `Send + Sync + 'static`
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
    ///     .with_tracing(tracing_subscriber::fmt().finish())
    ///     .run()
    ///     .await
    /// }
    /// ```
    pub fn with_tracing(
        mut self,
        subscriber: impl tracing::Subscriber + Send + Sync + 'static,
    ) -> Self {
        self.tracing_subscriber = Some(Box::new(subscriber));
        self
    }

    /// Configure a custom error handler to transform errors before they propagate.
    ///
    /// The provided function is called whenever the user handler returns an `Err(DurableError)`,
    /// allowing error transformation, logging, or enrichment before the error is returned
    /// to the Lambda runtime.
    ///
    /// # Arguments
    ///
    /// * `handler` — A closure `Fn(DurableError) -> DurableError` that transforms errors
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use durable_lambda_builder::prelude::*;
    /// use durable_lambda_core::error::DurableError;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), lambda_runtime::Error> {
    ///     durable_lambda_builder::handler(|event: serde_json::Value, mut ctx: BuilderContext| async move {
    ///         Ok(serde_json::json!({"ok": true}))
    ///     })
    ///     .with_error_handler(|e: DurableError| {
    ///         // Log or transform the error before it propagates.
    ///         e
    ///     })
    ///     .run()
    ///     .await
    /// }
    /// ```
    pub fn with_error_handler(
        mut self,
        handler: impl Fn(DurableError) -> DurableError + Send + Sync + 'static,
    ) -> Self {
        self.error_handler = Some(Box::new(handler));
        self
    }

    /// Consume the builder and start the Lambda runtime.
    ///
    /// This method:
    /// 1. Installs the tracing subscriber (if configured via [`with_tracing`](Self::with_tracing))
    /// 2. Initializes AWS configuration and creates a Lambda client
    /// 3. Creates a [`RealBackend`] for durable execution API calls
    /// 4. Registers with `lambda_runtime` to receive invocations
    /// 5. On each invocation, extracts durable execution metadata from the event,
    ///    creates a [`BuilderContext`], and calls the user handler
    /// 6. Routes handler errors through the error handler (if configured via
    ///    [`with_error_handler`](Self::with_error_handler))
    ///
    /// # Errors
    ///
    /// Returns `lambda_runtime::Error` if the Lambda runtime fails to start or
    /// encounters a fatal error.
    ///
    /// # Panics
    ///
    /// Panics if a tracing subscriber is configured and a global default subscriber
    /// has already been set (e.g., by another library or test framework).
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
        // Install the tracing subscriber before Lambda runtime starts, if configured.
        if let Some(subscriber) = self.tracing_subscriber {
            tracing::subscriber::set_global_default(subscriber)
                .expect("tracing subscriber already set");
        }

        let error_handler = self.error_handler;

        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = aws_sdk_lambda::Client::new(&config);
        let backend = Arc::new(RealBackend::new(client));

        lambda_runtime::run(service_fn(|event: LambdaEvent<serde_json::Value>| {
            let backend = backend.clone();
            let handler = &self.handler;
            let error_handler = &error_handler;
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
                let result = handler(invocation.user_event, builder_ctx).await;

                // Route errors through the custom error handler if configured.
                let result = match result {
                    Ok(v) => Ok(v),
                    Err(e) => {
                        let transformed = if let Some(ref h) = error_handler {
                            h(e)
                        } else {
                            e
                        };
                        Err(transformed)
                    }
                };

                // Wrap the result in the durable execution invocation output envelope.
                wrap_handler_result(result)
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
