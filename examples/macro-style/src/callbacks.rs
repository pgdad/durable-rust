//! Callbacks example — macro-style API.
//!
//! Demonstrates `ctx.create_callback()` and `ctx.callback_result()` using the
//! `#[durable_execution]` macro.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::types::CallbackOptions;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(
    _event: serde_json::Value,
    mut ctx: DurableContext,
) -> Result<serde_json::Value, DurableError> {
    let handle = ctx
        .create_callback(
            "approval_callback",
            CallbackOptions::new().timeout_seconds(300),
        )
        .await?;

    let approval: serde_json::Value = ctx.callback_result(&handle)?;

    let outcome = ctx
        .step("process_approval", || async move {
            let approved = approval["approved"].as_bool().unwrap_or(false);
            Ok::<_, String>(serde_json::json!({
                "approved": approved,
                "callback_id": handle.callback_id,
            }))
        })
        .await?;

    Ok(serde_json::json!({ "outcome": outcome.unwrap_or_default() }))
}
