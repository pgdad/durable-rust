use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(event: serde_json::Value) -> Result<serde_json::Value, durable_lambda_core::error::DurableError> {
    Ok(event)
}

fn main() {}
