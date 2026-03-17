//! Saga/compensation example — closure-style API.
//!
//! Demonstrates `ctx.step_with_compensation()` for registering durable rollback
//! logic alongside each step. When a later step fails, `ctx.run_compensations()`
//! executes all registered compensations in LIFO order.

use durable_lambda_closure::prelude::*;

async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Step 1: Book hotel with compensation to cancel it.
    let _hotel: Result<String, String> = ctx
        .step_with_compensation(
            "book_hotel",
            || async { Ok::<String, String>("HOTEL-001".to_string()) },
            |booking_id| async move {
                println!("Cancelling hotel booking: {booking_id}");
                Ok(())
            },
        )
        .await?;

    // Step 2: Book flight with compensation to cancel it.
    let _flight: Result<String, String> = ctx
        .step_with_compensation(
            "book_flight",
            || async { Ok::<String, String>("FLIGHT-001".to_string()) },
            |booking_id| async move {
                println!("Cancelling flight booking: {booking_id}");
                Ok(())
            },
        )
        .await?;

    // Step 3: Charge card with compensation to refund.
    let _charge: Result<String, String> = ctx
        .step_with_compensation(
            "charge_card",
            || async { Ok::<String, String>("CHARGE-001".to_string()) },
            |charge_id| async move {
                println!("Refunding charge: {charge_id}");
                Ok(())
            },
        )
        .await?;

    // Step 4: Notify vendor — always fails to trigger rollback.
    let notify_result: Result<String, String> = ctx
        .step("notify_vendor", || async {
            Err::<String, String>("vendor_unavailable".to_string())
        })
        .await?;

    // When step 4 fails, run all registered compensations in LIFO order.
    if notify_result.is_err() {
        let comp_result = ctx.run_compensations().await?;
        let names_vec: Vec<String> = comp_result.items.iter().map(|i| i.name.clone()).collect();
        return Ok(serde_json::json!({
            "status": "rolled_back",
            "compensation_sequence": names_vec,
            "all_succeeded": comp_result.all_succeeded
        }));
    }

    Ok(serde_json::json!({ "status": "completed" }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
