//! Basic steps example — closure-style API.
//!
//! Demonstrates `ctx.step()` for checkpointed work. Each step runs exactly once;
//! on replay, the cached result is returned without re-executing the closure.

use durable_lambda_closure::prelude::*;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Step 1: Extract and validate the order ID from the incoming event.
    let order_id = ctx
        .step("extract_order_id", move || {
            let event = event.clone();
            async move {
                let id = event["order_id"].as_str().unwrap_or("unknown").to_string();
                Ok::<_, String>(id)
            }
        })
        .await?;

    // Step 2: Look up the order details (simulated).
    let details = ctx
        .step("lookup_order", || async {
            Ok::<_, String>(serde_json::json!({
                "status": "found",
                "items": 3
            }))
        })
        .await?;

    Ok(serde_json::json!({
        "order_id": order_id.unwrap_or_default(),
        "details": details.unwrap_or_default(),
    }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
