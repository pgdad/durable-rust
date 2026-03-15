use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_macro::durable_execution;

#[durable_execution]
fn handler(
    event: serde_json::Value,
    ctx: DurableContext,
) -> Result<serde_json::Value, DurableError> {
    let _ = ctx;
    Ok(event)
}

fn main() {}
