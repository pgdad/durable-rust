//! Replay-safe logging example — closure-style API.
//!
//! Demonstrates the replay-safe logging methods. These are no-ops during replay
//! to prevent duplicate log output, and emit structured logs during execution.

use durable_lambda_closure::prelude::*;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // These log calls are automatically deduplicated — they only emit during
    // first execution, not during replay.
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

    // Log with structured data for observability.
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

    // Warn and error level logging for exceptional conditions.
    if event.get("dry_run").is_some() {
        ctx.log_warn("Running in dry-run mode — no side effects");
    }

    Ok(serde_json::json!({
        "order_id": order_id.unwrap_or_default(),
        "result": result.unwrap_or_default(),
    }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
