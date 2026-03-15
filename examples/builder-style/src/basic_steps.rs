//! Basic steps example — builder-style API.
//!
//! Use `durable_lambda_builder::handler(|event, ctx| async { ... }).run().await`
//! for a concise, closure-based builder pattern.

use durable_lambda_builder::prelude::*;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |event: serde_json::Value, mut ctx: BuilderContext| async move {
            let order_id = ctx
                .step("extract_order_id", || {
                    let event = event.clone();
                    async move {
                        let id = event["order_id"].as_str().unwrap_or("unknown").to_string();
                        Ok::<_, String>(id)
                    }
                })
                .await?;

            let details = ctx
                .step("lookup_order", || async {
                    Ok::<_, String>(serde_json::json!({
                        "status": "found",
                        "items": 3
                    }))
                })
                .await?;

            Ok(serde_json::json!({
                "order_id": order_id.unwrap_or_default(),
                "details": details.unwrap_or_default(),
            }))
        },
    )
    .run()
    .await
}
