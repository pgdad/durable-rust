//! Batch checkpoint example — closure-style API.
//!
//! Demonstrates `ctx.enable_batch_mode()` which buffers multiple sequential
//! step checkpoints into a single AWS API call. After all steps complete,
//! `ctx.flush_batch()` flushes the buffered checkpoints.

use durable_lambda_closure::prelude::*;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Read batch flag from event, defaulting to false.
    let use_batch = event["batch"].as_bool().unwrap_or(false);

    // Enable batch mode if requested — subsequent steps buffer instead of
    // immediately sending checkpoint API calls.
    if use_batch {
        ctx.enable_batch_mode();
    }

    // Run 5 sequential steps.
    for i in 0..5i32 {
        let step_name = format!("step_{i}");
        let _: Result<i32, String> = ctx
            .step(&step_name, move || async move { Ok::<i32, String>(i) })
            .await?;
    }

    // Flush the batch if batch mode is active — sends all buffered checkpoints.
    if use_batch {
        ctx.flush_batch().await?;
    }

    Ok(serde_json::json!({
        "steps_completed": 5,
        "batch_mode": use_batch
    }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
