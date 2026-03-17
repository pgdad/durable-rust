use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(x: i32, y: i32) -> Result<serde_json::Value, durable_lambda_core::error::DurableError> {
    Ok(serde_json::json!({}))
}

fn main() {}
