//! Builder-pattern handler construction.
//!
//! Provides the [`DurableHandlerBuilder`] and [`handler`] entry point for
//! builder-pattern durable Lambda handlers (FR35).
//! Internally wires up `lambda_runtime`, AWS config, and `DurableContext` creation
//! so users never interact with these directly.

use std::future::Future;
use std::sync::Arc;

use aws_sdk_lambda::types::{Operation, OperationStatus, OperationType, StepDetails};
use durable_lambda_core::backend::RealBackend;
use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
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

                // Create DurableContext and wrap in BuilderContext.
                let durable_ctx = DurableContext::new(
                    backend,
                    durable_execution_arn,
                    checkpoint_token,
                    operations,
                    next_marker,
                )
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                let builder_ctx = BuilderContext::new(durable_ctx);

                // Call the user handler with owned context.
                let result = handler(user_event, builder_ctx)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                Ok::<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>(result)
            }
        }))
        .await
    }
}

/// Parse operations from the InitialExecutionState JSON.
///
/// Constructs `Operation` objects from the JSON array using the builder pattern.
/// Operations that cannot be parsed are silently skipped.
fn parse_operations(initial_state: &serde_json::Value) -> Vec<Operation> {
    let Some(ops_array) = initial_state["Operations"].as_array() else {
        return vec![];
    };

    ops_array
        .iter()
        .filter_map(|op_json| {
            let id = op_json["Id"].as_str()?;
            let op_type = parse_operation_type(op_json["Type"].as_str()?)?;
            let status = parse_operation_status(op_json["Status"].as_str()?)?;

            let timestamp = op_json["StartTimestamp"]
                .as_f64()
                .map(aws_smithy_types::DateTime::from_secs_f64)
                .unwrap_or_else(|| aws_smithy_types::DateTime::from_secs(0));

            let mut builder = Operation::builder()
                .id(id)
                .r#type(op_type)
                .status(status)
                .start_timestamp(timestamp);

            // Parse step details if present.
            if let Some(step_details_json) = op_json.get("StepDetails") {
                let mut sd_builder = StepDetails::builder();

                if let Some(result) = step_details_json["Result"].as_str() {
                    sd_builder = sd_builder.result(result);
                }

                if let Some(error_json) = step_details_json.get("Error") {
                    if let (Some(error_type), Some(error_data)) = (
                        error_json["ErrorType"].as_str(),
                        error_json["ErrorData"].as_str(),
                    ) {
                        sd_builder = sd_builder.error(
                            aws_sdk_lambda::types::ErrorObject::builder()
                                .error_type(error_type)
                                .error_data(error_data)
                                .build(),
                        );
                    }
                }

                if let Some(attempt) = step_details_json["Attempt"].as_i64() {
                    sd_builder = sd_builder.attempt(attempt as i32);
                }

                builder = builder.step_details(sd_builder.build());
            }

            // Parse execution details if present.
            if let Some(exec_json) = op_json.get("ExecutionDetails") {
                let mut ed_builder = aws_sdk_lambda::types::ExecutionDetails::builder();
                if let Some(input) = exec_json["InputPayload"].as_str() {
                    ed_builder = ed_builder.input_payload(input);
                }
                builder = builder.execution_details(ed_builder.build());
            }

            builder.build().ok()
        })
        .collect()
}

/// Parse an operation type string into the AWS SDK enum.
fn parse_operation_type(s: &str) -> Option<OperationType> {
    match s {
        "Step" | "STEP" => Some(OperationType::Step),
        "Execution" | "EXECUTION" => Some(OperationType::Execution),
        "Wait" | "WAIT" => Some(OperationType::Wait),
        "Callback" | "CALLBACK" => Some(OperationType::Callback),
        "ChainedInvoke" | "CHAINED_INVOKE" => Some(OperationType::ChainedInvoke),
        _ => None,
    }
}

/// Parse an operation status string into the AWS SDK enum.
fn parse_operation_status(s: &str) -> Option<OperationStatus> {
    match s {
        "Succeeded" | "SUCCEEDED" => Some(OperationStatus::Succeeded),
        "Failed" | "FAILED" => Some(OperationStatus::Failed),
        "Pending" | "PENDING" => Some(OperationStatus::Pending),
        "Ready" | "READY" => Some(OperationStatus::Ready),
        "Started" | "STARTED" => Some(OperationStatus::Started),
        _ => None,
    }
}

/// Extract the user's original event payload from the InitialExecutionState JSON.
///
/// The first operation with type EXECUTION contains the user's input payload
/// in its `ExecutionDetails.InputPayload` field. If not found, returns an
/// empty JSON object.
fn extract_user_event(initial_state: &serde_json::Value) -> serde_json::Value {
    if let Some(ops) = initial_state["Operations"].as_array() {
        for op in ops {
            if op["Type"].as_str() == Some("Execution") || op["Type"].as_str() == Some("EXECUTION")
            {
                if let Some(input) = op
                    .get("ExecutionDetails")
                    .and_then(|ed| ed["InputPayload"].as_str())
                {
                    if let Ok(parsed) = serde_json::from_str(input) {
                        return parsed;
                    }
                }
            }
        }
    }
    serde_json::Value::Object(serde_json::Map::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::BuilderContext;

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
}
