//! Minimal durable Lambda handler using the closure-native approach.
//!
//! This example demonstrates:
//! - Single `run()` entry point (no manual lambda_runtime wiring)
//! - `ctx.step()` for checkpointed work
//! - `ctx.step_with_options()` for retries
//! - Owned `ClosureContext` pattern
//!
//! Build and deploy as a container image:
//! ```sh
//! docker build -f examples/Dockerfile -t my-durable-lambda .
//! ```

use durable_lambda_closure::prelude::*;

/// Handler for a simple order-processing durable Lambda.
///
/// Each step is checkpointed — on replay, cached results are returned
/// without re-executing the closure.
async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Step 1: Validate the order from the input event.
    let validated = ctx
        .step("validate_order", || {
            let event = event.clone();
            async move {
                Ok::<_, String>(serde_json::json!({"order_id": event["order_id"], "valid": true}))
            }
        })
        .await?;

    // Step 2: Charge payment with automatic retries on failure.
    let charged = ctx
        .step_with_options(
            "charge_payment",
            StepOptions::new().retries(3).backoff_seconds(2),
            || async { Ok::<_, String>(serde_json::json!({"charged": true})) },
        )
        .await?;

    // Step 3: Send confirmation.
    let confirmed = ctx
        .step("send_confirmation", || async {
            Ok::<_, String>(serde_json::json!({"confirmed": true}))
        })
        .await?;

    Ok(serde_json::json!({
        "validated": validated,
        "charged": charged,
        "confirmed": confirmed,
    }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
