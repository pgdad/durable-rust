//! Combined workflow example — builder-style API.
//!
//! End-to-end multi-operation workflow using the builder pattern.

use durable_lambda_builder::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct OrderValidation {
    order_id: String,
    total: f64,
    valid: bool,
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |event: serde_json::Value, mut ctx: BuilderContext| async move {
            ctx.log("Starting combined order workflow");

            let validation: Result<OrderValidation, String> = ctx
                .step("validate_order", move || {
                    let event = event.clone();
                    async move {
                        Ok(OrderValidation {
                            order_id: event["order_id"].as_str().unwrap_or("unknown").to_string(),
                            total: event["total"].as_f64().unwrap_or(0.0),
                            valid: true,
                        })
                    }
                })
                .await?;
            let validation =
                validation.map_err(|e| DurableError::child_context_failed("validate", e))?;

            ctx.log_with_data(
                "Order validated",
                &serde_json::json!({ "order_id": &validation.order_id }),
            );

            let payment = ctx
                .step_with_options(
                    "charge_payment",
                    StepOptions::new().retries(3).backoff_seconds(2),
                    || async {
                        Ok::<_, String>(serde_json::json!({"charged": true, "txn": "txn_123"}))
                    },
                )
                .await?;

            let fulfillment_fn = std::env::var("FULFILLMENT_FUNCTION")
                .unwrap_or_else(|_| "fulfillment-lambda".to_string());
            let fulfillment_payload = serde_json::json!({ "order_id": &validation.order_id });
            let fulfillment_result: Result<serde_json::Value, String> = ctx
                .step("start_fulfillment", move || {
                    let fulfillment_fn = fulfillment_fn.clone();
                    let payload = fulfillment_payload.clone();
                    async move {
                        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                            .region(aws_config::Region::new(
                                std::env::var("AWS_REGION")
                                    .unwrap_or_else(|_| "us-east-2".to_string()),
                            ))
                            .load()
                            .await;
                        let client = aws_sdk_lambda::Client::new(&config);
                        let blob = aws_sdk_lambda::primitives::Blob::new(
                            serde_json::to_vec(&payload).unwrap(),
                        );
                        let resp = client
                            .invoke()
                            .function_name(&fulfillment_fn)
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
            let fulfillment = fulfillment_result.map_err(|e| {
                DurableError::checkpoint_failed("start_fulfillment", std::io::Error::other(e))
            })?;

            ctx.wait("cooling_period", 30).await?;

            let post_processing: Result<serde_json::Value, String> = ctx
                .step("post_processing", || async {
                    Ok(serde_json::json!({"receipt": "sent", "inventory": "updated"}))
                })
                .await?;
            let post_processing = post_processing.unwrap_or_default();

            ctx.log("Combined workflow complete");

            Ok(serde_json::json!({
                "order_id": validation.order_id,
                "payment": payment.unwrap_or_default(),
                "fulfillment": fulfillment,
                "post_processing": post_processing,
            }))
        },
    )
    .run()
    .await
}
