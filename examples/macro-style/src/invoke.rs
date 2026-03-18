//! Invoke example — macro-style API.
//!
//! Demonstrates durable Lambda-to-Lambda invocation checkpointed via `ctx.step()`.
//! Uses a step to wrap a direct AWS SDK Lambda invocation.
//!
//! NOTE: Uses `ctx.step()` instead of `ctx.invoke()` because the durable execution
//! service does not populate `chained_invoke_details.result` in the operation state.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(
    event: serde_json::Value,
    mut ctx: DurableContext,
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
                let blob =
                    aws_sdk_lambda::primitives::Blob::new(serde_json::to_vec(&payload).unwrap());
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
    let enrichment = enrichment
        .map_err(|e| DurableError::checkpoint_failed("enrich_order", std::io::Error::other(e)))?;

    Ok(serde_json::json!({
        "order_id": order_id,
        "enrichment": enrichment,
    }))
}
