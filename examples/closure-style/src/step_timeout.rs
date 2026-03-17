//! Step timeout example — closure-style API.
//!
//! Demonstrates `StepOptions::timeout_seconds()` which wraps a step closure
//! in `tokio::time::timeout`. If the closure does not complete within the
//! configured duration, the step returns `DurableError::StepTimeout` and
//! the spawned task is aborted.

use std::time::Duration;

use durable_lambda_closure::prelude::*;

async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // This step is configured with a 2-second timeout but sleeps for 60 seconds.
    // The timeout fires first, causing DurableError::StepTimeout to propagate
    // back to the Lambda runtime as a FunctionError.
    let _result: Result<String, String> = ctx
        .step_with_options(
            "slow_operation",
            StepOptions::new().timeout_seconds(2),
            || async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok::<String, String>("done".to_string())
            },
        )
        .await?;

    Ok(serde_json::json!({ "status": "completed" }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
