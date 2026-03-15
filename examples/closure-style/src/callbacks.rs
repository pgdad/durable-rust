//! Callbacks example — closure-style API.
//!
//! Demonstrates `ctx.create_callback()` and `ctx.callback_result()` for
//! external event-driven suspension. The Lambda suspends until an external
//! system sends a callback via the callback ID.

use durable_lambda_closure::prelude::*;

async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Create a callback that waits up to 300 seconds for an external signal.
    let handle = ctx
        .create_callback(
            "approval_callback",
            CallbackOptions::new().timeout_seconds(300),
        )
        .await?;

    // The callback_id would be sent to an external system (e.g., a human approval UI).
    // The Lambda suspends until the external system calls back with a result.
    let approval: serde_json::Value = ctx.callback_result(&handle)?;

    // Process the callback result.
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

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
