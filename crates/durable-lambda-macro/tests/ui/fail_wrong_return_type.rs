use durable_lambda_core::context::DurableContext;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(event: serde_json::Value, ctx: DurableContext) -> String {
    String::from("bad return type")
}

fn main() {}
