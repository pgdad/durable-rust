//! Replay-safe logging example — macro-style API.
//!
//! Demonstrates replay-safe logging methods using the `#[durable_execution]` macro.
//! Log calls are no-ops during replay to prevent duplicate output.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(
    event: serde_json::Value,
    mut ctx: DurableContext,
) -> Result<serde_json::Value, DurableError> {
    ctx.log("Starting order processing");
    ctx.log_debug("Event payload received");

    let order_id = ctx
        .step("extract_order", || {
            let event = event.clone();
            async move {
                Ok::<_, String>(event["order_id"].as_str().unwrap_or("unknown").to_string())
            }
        })
        .await?;

    ctx.log_with_data(
        "Order extracted",
        &serde_json::json!({ "order_id": order_id }),
    );

    let result = ctx
        .step("process_order", || async {
            Ok::<_, String>(serde_json::json!({"processed": true}))
        })
        .await?;

    ctx.log("Order processing complete");

    if event.get("dry_run").is_some() {
        ctx.log_warn("Running in dry-run mode — no side effects");
    }

    Ok(serde_json::json!({
        "order_id": order_id.unwrap_or_default(),
        "result": result.unwrap_or_default(),
    }))
}
