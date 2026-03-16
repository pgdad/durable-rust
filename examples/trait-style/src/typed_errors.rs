//! Typed errors example — trait-style API.

use async_trait::async_trait;
use durable_lambda_trait::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum PaymentError {
    InsufficientFunds { balance: f64, required: f64 },
    CardDeclined { reason: String },
}

struct PaymentHandler;

#[async_trait]
impl DurableHandler for PaymentHandler {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        let amount: f64 = event["amount"].as_f64().unwrap_or(100.0);

        let payment_result: Result<String, PaymentError> = ctx
            .step("process_payment", move || async move {
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
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(PaymentHandler).await
}
