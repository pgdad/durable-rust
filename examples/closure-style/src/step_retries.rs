//! Step retries example — closure-style API.
//!
//! Demonstrates `ctx.step_with_options()` to configure automatic retries
//! with exponential backoff for steps that may transiently fail.

use durable_lambda_closure::prelude::*;

async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Retry up to 3 times with 2-second backoff between attempts.
    let result = ctx
        .step_with_options(
            "call_flaky_api",
            StepOptions::new().retries(3).backoff_seconds(2),
            || async {
                // Simulate calling an external API that might fail transiently.
                Ok::<_, String>(serde_json::json!({"api_response": "success"}))
            },
        )
        .await?;

    Ok(serde_json::json!({ "result": result.unwrap_or_default() }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
