//! Parallel example — builder-style API.

use std::future::Future;
use std::pin::Pin;

use durable_lambda_builder::prelude::*;
use durable_lambda_core::context::DurableContext;

type BranchFn = Box<
    dyn FnOnce(
            DurableContext,
        )
            -> Pin<Box<dyn Future<Output = Result<serde_json::Value, DurableError>> + Send>>
        + Send,
>;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |_event: serde_json::Value, mut ctx: BuilderContext| async move {
            let branches: Vec<BranchFn> = vec![
                Box::new(|mut child_ctx: DurableContext| {
                    Box::pin(async move {
                        let _r: Result<String, String> = child_ctx
                            .step("branch_a_work", || async { Ok("result_a".to_string()) })
                            .await?;
                        Ok(serde_json::json!({"branch": "a"}))
                    })
                }),
                Box::new(|mut child_ctx: DurableContext| {
                    Box::pin(async move {
                        let _r: Result<String, String> = child_ctx
                            .step("branch_b_work", || async { Ok("result_b".to_string()) })
                            .await?;
                        Ok(serde_json::json!({"branch": "b"}))
                    })
                }),
                Box::new(|mut child_ctx: DurableContext| {
                    Box::pin(async move {
                        let _r: Result<String, String> = child_ctx
                            .step("branch_c_work", || async { Ok("result_c".to_string()) })
                            .await?;
                        Ok(serde_json::json!({"branch": "c"}))
                    })
                }),
            ];

            let batch = ctx
                .parallel("fan_out", branches, ParallelOptions::new())
                .await?;

            let results: Vec<_> = batch
                .results
                .iter()
                .filter_map(|item| item.result.as_ref())
                .collect();

            Ok(serde_json::json!({ "parallel_results": results }))
        },
    )
    .run()
    .await
}
