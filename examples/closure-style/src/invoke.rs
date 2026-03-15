//! Invoke example — closure-style API.
//!
//! Demonstrates `ctx.invoke()` for durable Lambda-to-Lambda invocation.
//! The invocation is checkpointed — on replay, the cached result is returned.

use durable_lambda_closure::prelude::*;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    let order_id = event["order_id"].as_str().unwrap_or("unknown");

    // Invoke another durable Lambda by its function name.
    // The payload is serialized and sent; the response is deserialized.
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

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
