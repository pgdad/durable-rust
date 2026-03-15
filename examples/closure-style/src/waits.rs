//! Waits example — closure-style API.
//!
//! Demonstrates `ctx.wait()` for time-based suspension. The Lambda suspends
//! execution and resumes after the specified duration without consuming compute.

use durable_lambda_closure::prelude::*;

async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Step 1: Start processing.
    let started = ctx
        .step("start_processing", || async {
            Ok::<_, String>(serde_json::json!({"status": "started"}))
        })
        .await?;

    // Wait 60 seconds (the Lambda suspends — no compute cost during the wait).
    ctx.wait("cooling_period", 60).await?;

    // Step 2: Continue after the wait.
    let completed = ctx
        .step("finish_processing", || async {
            Ok::<_, String>(serde_json::json!({"status": "completed"}))
        })
        .await?;

    Ok(serde_json::json!({
        "started": started.unwrap_or_default(),
        "completed": completed.unwrap_or_default(),
    }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
