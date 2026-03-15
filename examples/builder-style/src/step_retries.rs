//! Step retries example — builder-style API.

use durable_lambda_builder::prelude::*;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |_event: serde_json::Value, mut ctx: BuilderContext| async move {
            let result = ctx
                .step_with_options(
                    "call_flaky_api",
                    StepOptions::new().retries(3).backoff_seconds(2),
                    || async { Ok::<_, String>(serde_json::json!({"api_response": "success"})) },
                )
                .await?;

            Ok(serde_json::json!({ "result": result.unwrap_or_default() }))
        },
    )
    .run()
    .await
}
