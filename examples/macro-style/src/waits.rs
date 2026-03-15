//! Waits example — macro-style API.
//!
//! Demonstrates `ctx.wait()` for time-based suspension using the
//! `#[durable_execution]` macro.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(
    _event: serde_json::Value,
    mut ctx: DurableContext,
) -> Result<serde_json::Value, DurableError> {
    let started = ctx
        .step("start_processing", || async {
            Ok::<_, String>(serde_json::json!({"status": "started"}))
        })
        .await?;

    ctx.wait("cooling_period", 60).await?;

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
