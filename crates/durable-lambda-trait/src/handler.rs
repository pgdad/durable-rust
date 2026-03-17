//! DurableHandler trait definition and runner.
//!
//! Provide the [`DurableHandler`] trait and [`run`] entry point for trait-based
//! durable Lambda handlers (FR33). Internally wires up `lambda_runtime`, AWS config,
//! and `DurableContext` creation so users never interact with these directly.

use std::sync::Arc;

use durable_lambda_core::backend::RealBackend;
use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::event::parse_invocation;
use durable_lambda_core::response::wrap_handler_result;
use lambda_runtime::{service_fn, LambdaEvent};

use crate::context::TraitContext;

/// Trait for defining durable Lambda handlers with a structured, object-oriented approach.
///
/// Implement this trait on your struct to define handler logic. The [`handle`](DurableHandler::handle)
/// method receives the deserialized user event payload and a [`TraitContext`] with access
/// to all durable operations (step, wait, invoke, parallel, etc.).
///
/// Use [`run`] to wire up the Lambda runtime — you never interact with `lambda_runtime` directly.
///
/// # Examples
///
/// ```no_run
/// use durable_lambda_trait::prelude::*;
/// use async_trait::async_trait;
///
/// struct OrderProcessor;
///
/// #[async_trait]
/// impl DurableHandler for OrderProcessor {
///     async fn handle(
///         &self,
///         event: serde_json::Value,
///         mut ctx: TraitContext,
///     ) -> Result<serde_json::Value, DurableError> {
///         let result: Result<i32, String> = ctx.step("validate", || async {
///             Ok(42)
///         }).await?;
///         Ok(serde_json::json!({"result": result.unwrap()}))
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), lambda_runtime::Error> {
///     durable_lambda_trait::run(OrderProcessor).await
/// }
/// ```
#[async_trait::async_trait]
pub trait DurableHandler: Send + Sync + 'static {
    /// Handle a durable Lambda invocation.
    ///
    /// # Arguments
    ///
    /// * `event` — The deserialized user event payload
    /// * `ctx` — A [`TraitContext`] providing access to all durable operations
    ///
    /// # Errors
    ///
    /// Return [`DurableError`] on failure. The runtime converts this to a Lambda error response.
    async fn handle(
        &self,
        event: serde_json::Value,
        ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError>;
}

/// Run a durable Lambda handler using the trait-based approach.
///
/// This is the single entry point for trait-based durable Lambdas. It:
/// 1. Initializes AWS configuration and creates a Lambda client
/// 2. Creates a [`RealBackend`] for durable execution API calls
/// 3. Registers with `lambda_runtime` to receive invocations
/// 4. On each invocation, extracts durable execution metadata from the event,
///    creates a [`TraitContext`], and calls [`DurableHandler::handle`]
///
/// # Arguments
///
/// * `handler` — A struct implementing [`DurableHandler`]
///
/// # Errors
///
/// Returns `lambda_runtime::Error` if the Lambda runtime fails to start or
/// encounters a fatal error.
///
/// # Examples
///
/// ```no_run
/// use durable_lambda_trait::prelude::*;
/// use async_trait::async_trait;
///
/// struct MyHandler;
///
/// #[async_trait]
/// impl DurableHandler for MyHandler {
///     async fn handle(
///         &self,
///         event: serde_json::Value,
///         mut ctx: TraitContext,
///     ) -> Result<serde_json::Value, DurableError> {
///         let result: Result<i32, String> = ctx.step("process", || async {
///             Ok(42)
///         }).await?;
///         Ok(serde_json::json!({"done": true}))
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), lambda_runtime::Error> {
///     durable_lambda_trait::run(MyHandler).await
/// }
/// ```
pub async fn run<H: DurableHandler>(handler: H) -> Result<(), lambda_runtime::Error> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = aws_sdk_lambda::Client::new(&config);
    let backend = Arc::new(RealBackend::new(client));

    let handler = Arc::new(handler);

    lambda_runtime::run(service_fn(|event: LambdaEvent<serde_json::Value>| {
        let backend = backend.clone();
        let handler = handler.clone();
        async move {
            let (payload, _lambda_ctx) = event.into_parts();

            // Parse all durable execution fields from the Lambda event.
            let invocation = parse_invocation(&payload)
                .map_err(Box::<dyn std::error::Error + Send + Sync>::from)?;

            // Create DurableContext and wrap in TraitContext.
            let durable_ctx = DurableContext::new(
                backend,
                invocation.durable_execution_arn,
                invocation.checkpoint_token,
                invocation.operations,
                invocation.next_marker,
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let trait_ctx = TraitContext::new(durable_ctx);

            // Call the handler's handle method.
            let result = handler.handle(invocation.user_event, trait_ctx).await;

            // Wrap the result in the durable execution invocation output envelope.
            wrap_handler_result(result)
        }
    }))
    .await
}
