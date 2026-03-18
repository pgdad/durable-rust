//! Waits example — trait-style API.
//!
//! The wait duration can be controlled via the `wait_seconds` field in the event
//! payload (defaults to 60 seconds if not provided).

use async_trait::async_trait;
use durable_lambda_trait::prelude::*;

struct WaitHandler;

#[async_trait]
impl DurableHandler for WaitHandler {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        let started = ctx
            .step("start_processing", || async {
                Ok::<_, String>(serde_json::json!({"status": "started"}))
            })
            .await?;

        let wait_secs = event["wait_seconds"].as_i64().unwrap_or(60) as i32;
        ctx.wait("cooling_period", wait_secs).await?;

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
