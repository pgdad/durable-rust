//! Invoke example — builder-style API.

use durable_lambda_builder::prelude::*;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |event: serde_json::Value, mut ctx: BuilderContext| async move {
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
        },
    )
    .run()
    .await
}
