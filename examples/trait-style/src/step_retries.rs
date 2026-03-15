//! Step retries example — trait-style API.

use async_trait::async_trait;
use durable_lambda_trait::prelude::*;

struct RetryHandler;

#[async_trait]
impl DurableHandler for RetryHandler {
    async fn handle(
        &self,
        _event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        let result = ctx
            .step_with_options(
                "call_flaky_api",
                StepOptions::new().retries(3).backoff_seconds(2),
                || async { Ok::<_, String>(serde_json::json!({"api_response": "success"})) },
            )
            .await?;

        Ok(serde_json::json!({ "result": result.unwrap_or_default() }))
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(RetryHandler).await
}
