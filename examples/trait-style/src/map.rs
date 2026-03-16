//! Map example — trait-style API.

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use durable_lambda_core::context::DurableContext;
use durable_lambda_trait::prelude::*;

struct MapHandler;

#[async_trait]
impl DurableHandler for MapHandler {
    async fn handle(
        &self,
        _event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        let order_ids = vec!["order-1", "order-2", "order-3", "order-4"];

        let batch = ctx
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
                        let order_id_for_step = order_id.clone();
                        let _r: Result<String, String> = child_ctx
                            .step("process", move || {
                                let oid = order_id_for_step.clone();
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
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(MapHandler).await
}
