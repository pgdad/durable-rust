//! Typed errors example — macro-style API.
//!
//! Demonstrates `Result<T, E>` with custom serializable error types
//! using the `#[durable_execution]` macro.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_macro::durable_execution;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum PaymentError {
    InsufficientFunds { balance: f64, required: f64 },
    CardDeclined { reason: String },
}

#[durable_execution]
async fn handler(
    event: serde_json::Value,
    mut ctx: DurableContext,
) -> Result<serde_json::Value, DurableError> {
    let amount: f64 = event["amount"].as_f64().unwrap_or(100.0);

    let payment_result: Result<String, PaymentError> = ctx
        .step("process_payment", || async move {
            if amount > 1000.0 {
                Err(PaymentError::InsufficientFunds {
                    balance: 500.0,
                    required: amount,
                })
            } else {
                Ok(format!("txn_{}", amount as u64))
            }
        })
        .await?;

    match payment_result {
        Ok(txn_id) => Ok(serde_json::json!({ "transaction_id": txn_id })),
        Err(PaymentError::InsufficientFunds { balance, required }) => Ok(serde_json::json!({
            "error": "insufficient_funds",
            "balance": balance,
            "required": required,
        })),
        Err(PaymentError::CardDeclined { reason }) => Ok(serde_json::json!({
            "error": "card_declined",
            "reason": reason,
        })),
    }
}
