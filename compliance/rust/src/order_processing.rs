//! Order Processing compliance workflow.
//!
//! Multi-step workflow: validate order → charge payment (with retries) → send confirmation.
//! Mirrors the Python reference implementation in `compliance/python/workflows/order_processing.py`.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::types::StepOptions;

/// Execute the order processing workflow against the given context.
///
/// Operations recorded:
/// 1. `step:validate_order`
/// 2. `step:charge_payment`
/// 3. `step:send_confirmation`
pub async fn run(ctx: &mut DurableContext) -> Result<serde_json::Value, DurableError> {
    // Step 1: Validate the order
    let validated: Result<serde_json::Value, String> = ctx
        .step("validate_order", || async {
            Ok(serde_json::json!({"order_id": "ORD-001", "valid": true}))
        })
        .await?;

    // Step 2: Charge payment with retries
    let charged: Result<serde_json::Value, String> = ctx
        .step_with_options(
            "charge_payment",
            StepOptions::new().retries(3).backoff_seconds(2),
            || async { Ok(serde_json::json!({"charged": true, "amount": 99.99})) },
        )
        .await?;

    // Step 3: Send confirmation
    let confirmed: Result<serde_json::Value, String> = ctx
        .step("send_confirmation", || async {
            Ok(serde_json::json!({"confirmed": true, "email_sent": true}))
        })
        .await?;

    Ok(serde_json::json!({
        "validated": validated.unwrap(),
        "charged": charged.unwrap(),
        "confirmed": confirmed.unwrap(),
    }))
}
