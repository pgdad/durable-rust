//! Combined workflow example — trait-style API.
//!
//! End-to-end multi-operation workflow using the `DurableHandler` trait.

use async_trait::async_trait;
use durable_lambda_core::context::DurableContext;
use durable_lambda_trait::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct OrderValidation {
    order_id: String,
    total: f64,
    valid: bool,
}

struct CombinedHandler;

#[async_trait]
impl DurableHandler for CombinedHandler {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        ctx.log("Starting combined order workflow");

        let validation: Result<OrderValidation, String> = ctx
            .step("validate_order", || {
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

        let fulfillment: serde_json::Value = ctx
            .invoke(
                "start_fulfillment",
                "fulfillment-lambda",
                &serde_json::json!({ "order_id": &validation.order_id }),
            )
            .await?;

        ctx.wait("cooling_period", 30).await?;

        let post_processing: serde_json::Value = ctx
            .child_context(
                "post_processing",
                |mut child_ctx: DurableContext| async move {
                    let _r: Result<String, String> = child_ctx
                        .step("send_receipt", || async { Ok("receipt_sent".to_string()) })
                        .await?;
                    let _r2: Result<String, String> = child_ctx
                        .step("update_inventory", || async {
                            Ok("inventory_updated".to_string())
                        })
                        .await?;
                    Ok(serde_json::json!({"receipt": "sent", "inventory": "updated"}))
                },
            )
            .await?;

        ctx.log("Combined workflow complete");

        Ok(serde_json::json!({
            "order_id": validation.order_id,
            "payment": payment.unwrap_or_default(),
            "fulfillment": fulfillment,
            "post_processing": post_processing,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(CombinedHandler).await
}
