//! Callback Approval compliance workflow.
//!
//! Callback-based approval workflow: submit request → create callback for
//! external approval → wait for timeout → process approval → finalize.
//! Mirrors the Python reference implementation in `compliance/python/workflows/callback_approval.py`.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::types::CallbackOptions;

/// Execute the callback approval workflow against the given context.
///
/// Operations recorded:
/// 1. `step:submit_request`
/// 2. `callback:approval`
/// 3. `wait:approval_timeout`
/// 4. `step:process_approval`
/// 5. `step:finalize`
pub async fn run(ctx: &mut DurableContext) -> Result<serde_json::Value, DurableError> {
    // Step 1: Submit approval request
    let request: Result<serde_json::Value, String> = ctx
        .step("submit_request", || async {
            Ok(serde_json::json!({"request_id": "REQ-001", "submitted": true}))
        })
        .await?;

    // Step 2: Create callback for external approval
    let handle = ctx
        .create_callback("approval", CallbackOptions::new())
        .await?;
    let approval_result: String = ctx.callback_result(&handle)?;

    // Step 3: Wait for approval timeout
    ctx.wait("approval_timeout", 300).await?;

    // Step 4: Process the approval decision
    let processed: Result<serde_json::Value, String> = ctx
        .step("process_approval", || async {
            Ok(serde_json::json!({"approved": true, "decision": "approved"}))
        })
        .await?;

    // Step 5: Finalize
    let finalized: Result<serde_json::Value, String> = ctx
        .step("finalize", || async {
            Ok(serde_json::json!({"finalized": true, "status": "complete"}))
        })
        .await?;

    Ok(serde_json::json!({
        "request": request.unwrap(),
        "approval": approval_result,
        "processed": processed.unwrap(),
        "finalized": finalized.unwrap(),
    }))
}
