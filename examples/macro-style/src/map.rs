//! Map example — macro-style API.
//!
//! Demonstrates `ctx.map()` for parallel collection processing using the
//! `#[durable_execution]` macro.

use std::future::Future;
use std::pin::Pin;

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::types::MapOptions;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(
    _event: serde_json::Value,
    mut ctx: DurableContext,
) -> Result<serde_json::Value, DurableError> {
    let order_ids = vec!["order-1", "order-2", "order-3", "order-4"];

    let batch =
        ctx
            .map(
                "process_orders",
                order_ids.into_iter().map(String::from).collect(),
                MapOptions::new().batch_size(2),
                |order_id: String,
                 mut child_ctx: DurableContext|
                 -> Pin<
                    Box<dyn Future<Output = Result<serde_json::Value, DurableError>> + Send>,
                > {
                    Box::pin(async move {
                        let _r: Result<String, String> = child_ctx
                            .step("process", || {
                                let oid = order_id.clone();
                                async move { Ok(format!("processed_{oid}")) }
                            })
                            .await?;
                        Ok(serde_json::json!({ "order_id": order_id, "status": "done" }))
                    })
                },
            )
            .await?;

    let processed: Vec<_> = batch
        .results
        .iter()
        .filter_map(|item| item.result.as_ref())
        .collect();

    Ok(serde_json::json!({ "processed_orders": processed }))
}
