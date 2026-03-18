//! Invoke example — trait-style API.
//!
//! Uses `ctx.step()` to wrap a direct AWS SDK Lambda invocation for durability.

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

        let enrichment_fn = std::env::var("ENRICHMENT_FUNCTION")
            .unwrap_or_else(|_| "order-enrichment-lambda".to_string());
        let payload = serde_json::json!({ "order_id": order_id });
        let enrichment: Result<serde_json::Value, String> = ctx
            .step("enrich_order", move || {
                let enrichment_fn = enrichment_fn.clone();
                let payload = payload.clone();
                async move {
                    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                        .region(aws_config::Region::new(
                            std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-2".to_string()),
                        ))
                        .load()
                        .await;
                    let client = aws_sdk_lambda::Client::new(&config);
                    let blob = aws_sdk_lambda::primitives::Blob::new(
                        serde_json::to_vec(&payload).unwrap(),
                    );
                    let resp = client
                        .invoke()
                        .function_name(&enrichment_fn)
                        .payload(blob)
                        .send()
                        .await
                        .map_err(|e| format!("Lambda invoke failed: {e}"))?;
                    let payload_bytes = resp
                        .payload()
                        .map(|b| b.as_ref().to_vec())
                        .unwrap_or_default();
                    let result: serde_json::Value = serde_json::from_slice(&payload_bytes)
                        .map_err(|e| format!("Failed to parse response: {e}"))?;
                    Ok(result)
                }
            })
            .await?;
        let enrichment = enrichment.map_err(|e| {
            DurableError::checkpoint_failed("enrich_order", std::io::Error::other(e))
        })?;

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
