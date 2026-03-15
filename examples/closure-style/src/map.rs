//! Map example — closure-style API.
//!
//! Demonstrates `ctx.map()` for parallel processing of a collection.
//! Each item is processed in its own durable sub-execution.

use std::future::Future;
use std::pin::Pin;

use durable_lambda_closure::prelude::*;
use durable_lambda_core::context::DurableContext;

async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    let order_ids = vec!["order-1", "order-2", "order-3", "order-4"];

    // Process each order in parallel. Each item gets its own DurableContext.
    let batch: BatchResult<serde_json::Value> =
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

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
