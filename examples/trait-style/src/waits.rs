//! Waits example — trait-style API.

use async_trait::async_trait;
use durable_lambda_trait::prelude::*;

struct WaitHandler;

#[async_trait]
impl DurableHandler for WaitHandler {
    async fn handle(
        &self,
        _event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        let started = ctx
            .step("start_processing", || async {
                Ok::<_, String>(serde_json::json!({"status": "started"}))
            })
            .await?;

        ctx.wait("cooling_period", 60).await?;

        let completed = ctx
            .step("finish_processing", || async {
                Ok::<_, String>(serde_json::json!({"status": "completed"}))
            })
            .await?;

        Ok(serde_json::json!({
            "started": started.unwrap_or_default(),
            "completed": completed.unwrap_or_default(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(WaitHandler).await
}
