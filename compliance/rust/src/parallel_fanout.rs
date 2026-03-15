//! Parallel Fanout compliance workflow.
//!
//! Validates input, fans out to three parallel processing branches
//! (enrich, score, tag), then aggregates results.
//! Mirrors the Python reference implementation in `compliance/python/workflows/parallel_fanout.py`.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;

/// Execute the parallel fanout workflow against the given context.
///
/// Operations recorded:
/// 1. `step:validate_input`
/// 2. `step:enrich_data`
/// 3. `step:score_data`
/// 4. `step:tag_data`
/// 5. `step:aggregate_results`
///
/// Note: The parallel branches are implemented as individual steps to match
/// the Python SDK's operation sequence. Both SDKs record each branch as a
/// separate step operation.
pub async fn run(ctx: &mut DurableContext) -> Result<serde_json::Value, DurableError> {
    // Step 1: Validate input
    let input: Result<serde_json::Value, String> = ctx
        .step("validate_input", || async {
            Ok(serde_json::json!({"data_id": "DATA-001", "valid": true}))
        })
        .await?;

    // Steps 2-4: Parallel branches (executed as individual steps for compliance)
    let enriched: Result<serde_json::Value, String> = ctx
        .step("enrich_data", || async {
            Ok(serde_json::json!({"enriched": true, "source": "external_db"}))
        })
        .await?;

    let scored: Result<serde_json::Value, String> = ctx
        .step("score_data", || async {
            Ok(serde_json::json!({"score": 0.95, "model": "v2"}))
        })
        .await?;

    let tagged: Result<serde_json::Value, String> = ctx
        .step("tag_data", || async {
            Ok(serde_json::json!({"tags": ["important", "reviewed"]}))
        })
        .await?;

    // Step 5: Aggregate results
    let aggregated: Result<serde_json::Value, String> = ctx
        .step("aggregate_results", || async {
            Ok(serde_json::json!({"status": "complete", "branches": 3}))
        })
        .await?;

    Ok(serde_json::json!({
        "input": input.unwrap(),
        "enriched": enriched.unwrap(),
        "scored": scored.unwrap(),
        "tagged": tagged.unwrap(),
        "aggregated": aggregated.unwrap(),
    }))
}
