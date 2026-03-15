//! Child contexts example — closure-style API.
//!
//! Demonstrates `ctx.child_context()` for isolated subflows. A child context
//! has its own operation namespace, preventing name collisions with the parent.

use durable_lambda_closure::prelude::*;
use durable_lambda_core::context::DurableContext;

async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Run an isolated subflow. The child context has its own step namespace,
    // so step names like "validate" won't collide with the parent's steps.
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

    // Parent can reuse step names without conflict.
    let result: Result<String, String> = ctx
        .step("validate", || async { Ok("parent_validation".to_string()) })
        .await?;

    Ok(serde_json::json!({
        "child_result": validation,
        "parent_result": result.unwrap_or_default(),
    }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
