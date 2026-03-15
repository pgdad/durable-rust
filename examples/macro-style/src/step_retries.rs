//! Step retries example — macro-style API.
//!
//! Demonstrates `ctx.step_with_options()` with automatic retries using the
//! `#[durable_execution]` macro for zero-boilerplate setup.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::types::StepOptions;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(
    _event: serde_json::Value,
    mut ctx: DurableContext,
) -> Result<serde_json::Value, DurableError> {
    let result = ctx
        .step_with_options(
            "call_flaky_api",
            StepOptions::new().retries(3).backoff_seconds(2),
            || async { Ok::<_, String>(serde_json::json!({"api_response": "success"})) },
        )
        .await?;

    Ok(serde_json::json!({ "result": result.unwrap_or_default() }))
}
