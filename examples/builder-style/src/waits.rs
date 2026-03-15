//! Waits example — builder-style API.

use durable_lambda_builder::prelude::*;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |_event: serde_json::Value, mut ctx: BuilderContext| async move {
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
        },
    )
    .run()
    .await
}
