//! Combined workflow example — closure-style API.
//!
//! End-to-end multi-operation workflow demonstrating steps, retries, waits,
//! callbacks, invocations, and child contexts working together in a single
//! order-processing pipeline.

use durable_lambda_closure::prelude::*;
use durable_lambda_core::context::DurableContext;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct OrderValidation {
    order_id: String,
    total: f64,
    valid: bool,
}

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    ctx.log("Starting combined order workflow");

    // Step 1: Validate the order.
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
    let validation = validation.map_err(|e| DurableError::child_context_failed("validate", e))?;

    ctx.log_with_data(
        "Order validated",
        &serde_json::json!({ "order_id": &validation.order_id }),
    );

    // Step 2: Charge payment with retries.
    let payment = ctx
        .step_with_options(
            "charge_payment",
            StepOptions::new().retries(3).backoff_seconds(2),
            || async { Ok::<_, String>(serde_json::json!({"charged": true, "txn": "txn_123"})) },
        )
        .await?;

    // Step 3: Invoke an external Lambda for fulfillment.
    let fulfillment: serde_json::Value = ctx
        .invoke(
            "start_fulfillment",
            "fulfillment-lambda",
            &serde_json::json!({ "order_id": &validation.order_id }),
        )
        .await?;

    // Step 4: Wait for a cooling-off period.
    ctx.wait("cooling_period", 30).await?;

    // Step 5: Run post-processing in a child context.
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

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
