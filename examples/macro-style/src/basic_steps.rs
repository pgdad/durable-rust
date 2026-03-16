//! Basic steps example — macro-style API.
//!
//! The `#[durable_execution]` macro generates the Lambda runtime boilerplate
//! and `main()` function. You write only the handler logic.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(
    event: serde_json::Value,
    mut ctx: DurableContext,
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
