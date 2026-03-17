//! Saga/compensation example — closure-style API.
//!
//! Demonstrates the saga pattern for distributed rollback. Each forward step is
//! followed by a durable compensation step (prefixed "compensate_") when rollback
//! is triggered. Compensation steps use the standard `ctx.step()` API so they are
//! checkpointed with the same SUCCEED/FAIL protocol as any other step.
//!
//! When `notify_vendor` fails, compensation steps are executed in LIFO order:
//!   compensate_charge_card → compensate_book_flight → compensate_book_hotel

use durable_lambda_closure::prelude::*;

async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Step 1: Book hotel.
    let hotel: Result<String, String> = ctx
        .step("book_hotel", || async {
            Ok::<String, String>("HOTEL-001".to_string())
        })
        .await?;
    let hotel_id = hotel.unwrap_or_default();

    // Step 2: Book flight.
    let flight: Result<String, String> = ctx
        .step("book_flight", || async {
            Ok::<String, String>("FLIGHT-001".to_string())
        })
        .await?;
    let flight_id = flight.unwrap_or_default();

    // Step 3: Charge card.
    let charge: Result<String, String> = ctx
        .step("charge_card", || async {
            Ok::<String, String>("CHARGE-001".to_string())
        })
        .await?;
    let charge_id = charge.unwrap_or_default();

    // Step 4: Notify vendor — always fails to trigger rollback.
    let notify_result: Result<String, String> = ctx
        .step("notify_vendor", || async {
            Err::<String, String>("vendor_unavailable".to_string())
        })
        .await?;

    // When step 4 fails, run durable compensation steps in LIFO order.
    if notify_result.is_err() {
        // Compensate in reverse order (LIFO): charge_card first, then flight, then hotel.
        let c3 = charge_id.clone();
        let _: Result<String, String> = ctx
            .step("compensate_charge_card", move || async move {
                Ok::<String, String>(format!("refunded:{c3}"))
            })
            .await?;

        let f2 = flight_id.clone();
        let _: Result<String, String> = ctx
            .step("compensate_book_flight", move || async move {
                Ok::<String, String>(format!("cancelled:{f2}"))
            })
            .await?;

        let h1 = hotel_id.clone();
        let _: Result<String, String> = ctx
            .step("compensate_book_hotel", move || async move {
                Ok::<String, String>(format!("cancelled:{h1}"))
            })
            .await?;

        return Ok(serde_json::json!({
            "status": "rolled_back",
            "compensation_sequence": ["charge_card", "book_flight", "book_hotel"],
            "all_succeeded": true
        }));
    }

    Ok(serde_json::json!({ "status": "completed" }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
