//! Callbacks example — trait-style API.

use async_trait::async_trait;
use durable_lambda_trait::prelude::*;

struct CallbackHandler;

#[async_trait]
impl DurableHandler for CallbackHandler {
    async fn handle(
        &self,
        _event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        let handle = ctx
            .create_callback(
                "approval_callback",
                CallbackOptions::new().timeout_seconds(300),
            )
            .await?;

        let approval: serde_json::Value = ctx.callback_result(&handle)?;

        let outcome = ctx
            .step("process_approval", || async move {
                let approved = approval["approved"].as_bool().unwrap_or(false);
                Ok::<_, String>(serde_json::json!({
                    "approved": approved,
                    "callback_id": handle.callback_id,
                }))
            })
            .await?;

        Ok(serde_json::json!({ "outcome": outcome.unwrap_or_default() }))
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(CallbackHandler).await
}
