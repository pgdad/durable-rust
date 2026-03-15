//! Closure-based handler registration.
//!
//! Provide the [`run`] entry point for closure-native durable Lambda handlers (FR34, FR47).
//! Internally wires up `lambda_runtime`, AWS config, and `DurableContext` creation
//! so users never interact with these directly.

use std::future::Future;
use std::sync::Arc;

use durable_lambda_core::backend::RealBackend;
use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::event::{extract_user_event, parse_operations};
use lambda_runtime::{service_fn, LambdaEvent};

use crate::context::ClosureContext;

/// Run a durable Lambda handler using the closure-native approach.
///
/// This is the single entry point for closure-native durable Lambdas. It:
/// 1. Initializes AWS configuration and creates a Lambda client
/// 2. Creates a [`RealBackend`] for durable execution API calls
/// 3. Registers with `lambda_runtime` to receive invocations
/// 4. On each invocation, extracts durable execution metadata from the event,
///    creates a [`ClosureContext`], and calls the user handler
///
/// The handler function receives the deserialized user event payload and an
/// owned [`ClosureContext`] (take it as `mut` to call operations), and returns
/// a JSON result or a [`DurableError`].
///
/// # Arguments
///
/// * `handler` — An async function taking the user event and a `ClosureContext`,
///   returning `Result<serde_json::Value, DurableError>`
///
/// # Errors
///
/// Returns `lambda_runtime::Error` if the Lambda runtime fails to start or
/// encounters a fatal error.
///
/// # Examples
///
/// ```no_run
/// use durable_lambda_closure::prelude::*;
///
/// async fn handler(
///     event: serde_json::Value,
///     mut ctx: ClosureContext,
/// ) -> Result<serde_json::Value, DurableError> {
///     let result: Result<i32, String> = ctx.step("validate", || async {
///         Ok(42)
///     }).await?;
///     Ok(serde_json::json!({"result": result.unwrap()}))
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), lambda_runtime::Error> {
///     durable_lambda_closure::run(handler).await
/// }
/// ```
pub async fn run<F, Fut>(handler: F) -> Result<(), lambda_runtime::Error>
where
    F: Fn(serde_json::Value, ClosureContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = aws_sdk_lambda::Client::new(&config);
    let backend = Arc::new(RealBackend::new(client));

    lambda_runtime::run(service_fn(|event: LambdaEvent<serde_json::Value>| {
        let backend = backend.clone();
        let handler = &handler;
        async move {
            let (payload, _lambda_ctx) = event.into_parts();

            // Extract durable execution envelope from the Lambda event.
            let durable_execution_arn = payload["DurableExecutionArn"]
                .as_str()
                .ok_or("missing DurableExecutionArn in event")?
                .to_string();

            let checkpoint_token = payload["CheckpointToken"]
                .as_str()
                .ok_or("missing CheckpointToken in event")?
                .to_string();

            let initial_state = &payload["InitialExecutionState"];

            // Parse operations from the initial execution state.
            let operations = parse_operations(initial_state);

            let next_marker = initial_state["NextMarker"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            // Extract user event payload from the first EXECUTION operation.
            let user_event = extract_user_event(initial_state);

            // Create DurableContext and wrap in ClosureContext.
            let durable_ctx = DurableContext::new(
                backend,
                durable_execution_arn,
                checkpoint_token,
                operations,
                next_marker,
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let closure_ctx = ClosureContext::new(durable_ctx);

            // Call the user handler with owned context.
            let result = handler(user_event, closure_ctx)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            Ok::<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>(result)
        }
    }))
    .await
}
