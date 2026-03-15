//! Invoke example — macro-style API.
//!
//! Demonstrates `ctx.invoke()` for durable Lambda-to-Lambda invocation using the
//! `#[durable_execution]` macro.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(
    event: serde_json::Value,
    mut ctx: DurableContext,
) -> Result<serde_json::Value, DurableError> {
    let order_id = event["order_id"].as_str().unwrap_or("unknown");

    let enrichment: serde_json::Value = ctx
        .invoke(
            "enrich_order",
            "order-enrichment-lambda",
            &serde_json::json!({ "order_id": order_id }),
        )
        .await?;

    Ok(serde_json::json!({
        "order_id": order_id,
        "enrichment": enrichment,
    }))
}
