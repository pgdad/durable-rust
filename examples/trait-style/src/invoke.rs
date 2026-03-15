//! Invoke example — trait-style API.

use async_trait::async_trait;
use durable_lambda_trait::prelude::*;

struct InvokeHandler;

#[async_trait]
impl DurableHandler for InvokeHandler {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
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
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(InvokeHandler).await
}
