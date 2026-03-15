//! Basic steps example — trait-style API.
//!
//! Implement the `DurableHandler` trait on a struct to define your handler.

use async_trait::async_trait;
use durable_lambda_trait::prelude::*;

struct BasicStepsHandler;

#[async_trait]
impl DurableHandler for BasicStepsHandler {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        let order_id = ctx
            .step("extract_order_id", || {
                let event = event.clone();
                async move {
                    let id = event["order_id"].as_str().unwrap_or("unknown").to_string();
                    Ok::<_, String>(id)
                }
            })
            .await?;

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
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(BasicStepsHandler).await
}
