//! Waits example — macro-style API.
//!
//! Demonstrates `ctx.wait()` for time-based suspension using the
//! `#[durable_execution]` macro.
//!
//! The wait duration can be controlled via the `wait_seconds` field in the event
//! payload (defaults to 60 seconds if not provided).

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(
    event: serde_json::Value,
    mut ctx: DurableContext,
) -> Result<serde_json::Value, DurableError> {
    let started = ctx
        .step("start_processing", || async {
            Ok::<_, String>(serde_json::json!({"status": "started"}))
        })
        .await?;

    let wait_secs = event["wait_seconds"].as_i64().unwrap_or(60) as i32;
    ctx.wait("cooling_period", wait_secs).await?;

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
