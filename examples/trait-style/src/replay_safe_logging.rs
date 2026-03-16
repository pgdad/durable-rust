//! Replay-safe logging example — trait-style API.

use async_trait::async_trait;
use durable_lambda_trait::prelude::*;

struct LoggingHandler;

#[async_trait]
impl DurableHandler for LoggingHandler {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        ctx.log("Starting order processing");
        ctx.log_debug("Event payload received");

        let event_for_step = event.clone();
        let order_id = ctx
            .step("extract_order", move || {
                let event = event_for_step.clone();
                async move {
                    Ok::<_, String>(event["order_id"].as_str().unwrap_or("unknown").to_string())
                }
            })
            .await?;

        ctx.log_with_data(
            "Order extracted",
            &serde_json::json!({ "order_id": order_id }),
        );

        let result = ctx
            .step("process_order", || async {
                Ok::<_, String>(serde_json::json!({"processed": true}))
            })
            .await?;

        ctx.log("Order processing complete");

        if event.get("dry_run").is_some() {
            ctx.log_warn("Running in dry-run mode — no side effects");
        }

        Ok(serde_json::json!({
            "order_id": order_id.unwrap_or_default(),
            "result": result.unwrap_or_default(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(LoggingHandler).await
}
