//! Conditional retry example — closure-style API.
//!
//! Demonstrates `StepOptions::retry_if()` which applies a predicate to determine
//! whether a step error should be retried. If the predicate returns `false`,
//! the step fails immediately without consuming the retry budget.

use durable_lambda_closure::prelude::*;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Read error_type from event, defaulting to "non_retryable".
    let error_type = event["error_type"]
        .as_str()
        .unwrap_or("non_retryable")
        .to_string();

    // Clone error_type before moving into the step closure.
    let err_type = error_type.clone();

    // Single step with 3 retries, but only retry errors equal to "transient".
    // - "transient": retry_if returns true → StepRetryScheduled (retries up to 3x)
    // - "non_retryable": retry_if returns false → FAIL immediately (no retries)
    let result: Result<String, String> = ctx
        .step_with_options(
            "call_api",
            StepOptions::new()
                .retries(3)
                .retry_if(|e: &String| e == "transient"),
            move || {
                let err = err_type.clone();
                async move { Err::<String, String>(err) }
            },
        )
        .await?;

    Ok(serde_json::json!({ "result": result }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
