//! Child contexts example — trait-style API.

use async_trait::async_trait;
use durable_lambda_core::context::DurableContext;
use durable_lambda_trait::prelude::*;

struct ChildContextHandler;

#[async_trait]
impl DurableHandler for ChildContextHandler {
    async fn handle(
        &self,
        _event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        let validation: serde_json::Value = ctx
            .child_context(
                "validation_flow",
                |mut child_ctx: DurableContext| async move {
                    let _r: Result<String, String> = child_ctx
                        .step("validate", || async { Ok("valid".to_string()) })
                        .await?;

                    let _r2: Result<String, String> = child_ctx
                        .step("normalize", || async { Ok("normalized".to_string()) })
                        .await?;

                    Ok(serde_json::json!({"validation": "passed", "normalized": true}))
                },
            )
            .await?;

        let result: Result<String, String> = ctx
            .step("validate", || async { Ok("parent_validation".to_string()) })
            .await?;

        Ok(serde_json::json!({
            "child_result": validation,
            "parent_result": result.unwrap_or_default(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(ChildContextHandler).await
}
