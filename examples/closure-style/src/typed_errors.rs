//! Typed errors example — closure-style API.
//!
//! Demonstrates returning `Result<T, E>` from steps where `E` is a custom
//! serializable error type, allowing callers to match on domain-specific failures.

use durable_lambda_closure::prelude::*;
use serde::{Deserialize, Serialize};

/// A domain-specific error that can be returned from steps.
#[derive(Debug, Serialize, Deserialize)]
enum PaymentError {
    InsufficientFunds { balance: f64, required: f64 },
    CardDeclined { reason: String },
}

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    let amount: f64 = event["amount"].as_f64().unwrap_or(100.0);

    // The step returns Result<String, PaymentError> — both sides are checkpointed.
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

    // Match on the typed error to decide next steps.
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

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
