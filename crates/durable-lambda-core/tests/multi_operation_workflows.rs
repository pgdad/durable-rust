//! Integration tests for multi-operation durable workflows.
//!
//! These tests exercise realistic workflows that combine multiple
//! operation types (steps, waits, callbacks, invokes) in sequence,
//! verifying that the replay engine handles mixed-operation histories
//! correctly. All tests use MockDurableContext — no AWS credentials needed.

use durable_lambda_core::types::CallbackOptions;
use durable_lambda_testing::prelude::*;

/// A workflow that validates an order, waits for a cooldown, then charges.
/// Tests: step → wait → step replay sequence.
#[tokio::test]
async fn test_step_wait_step_workflow_replays_correctly() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("validate", r#"{"order_id": 1, "valid": true}"#)
        .with_wait("cooldown")
        .with_step_result("charge", r#"100"#)
        .build()
        .await;

    // Step 1: validate order
    let validate_result: Result<serde_json::Value, String> = ctx
        .step("validate", || async { panic!("not executed") })
        .await
        .unwrap();
    assert!(validate_result.unwrap()["valid"].as_bool().unwrap());

    // Step 2: wait for cooldown (replays as Ok(()))
    ctx.wait("cooldown", 30).await.unwrap();

    // Step 3: charge payment
    let charge_result: Result<i32, String> = ctx
        .step("charge", || async { panic!("not executed") })
        .await
        .unwrap();
    assert_eq!(charge_result.unwrap(), 100);

    // Pure replay — no checkpoints
    assert_no_checkpoints(&calls).await;
}

/// A workflow that creates a callback, then checks the result.
/// Tests: callback replay with deserialization.
#[tokio::test]
async fn test_callback_workflow_replays_correctly() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_callback("approval", "cb-server-42", r#""approved by alice""#)
        .build()
        .await;

    // Create callback — should replay and return handle with callback_id
    let handle = ctx
        .create_callback("approval", CallbackOptions::new())
        .await
        .unwrap();
    assert_eq!(handle.callback_id, "cb-server-42");

    // Get callback result — should return deserialized value
    let result: String = ctx.callback_result(&handle).unwrap();
    assert_eq!(result, "approved by alice");

    // Pure replay — no checkpoints
    assert_no_checkpoints(&calls).await;
}

/// A workflow that invokes a target Lambda and gets the result back.
/// Tests: invoke replay with deserialization.
#[tokio::test]
async fn test_invoke_workflow_replays_correctly() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_invoke("call_processor", r#"{"status": "processed", "amount": 99}"#)
        .build()
        .await;

    // Invoke should replay the cached result
    let result: serde_json::Value = ctx
        .invoke(
            "call_processor",
            "payment-processor-lambda",
            &serde_json::json!({"order": 123}),
        )
        .await
        .unwrap();
    assert_eq!(result["status"], "processed");
    assert_eq!(result["amount"], 99);

    // Pure replay — no checkpoints
    assert_no_checkpoints(&calls).await;
}

/// A complex workflow combining all Epic 2 operations:
/// step → wait → callback → invoke → step
/// Tests: mixed-operation replay with correct operation ID sequencing.
#[tokio::test]
async fn test_full_epic2_workflow_replays_correctly() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("validate_order", r#"{"order_id": 42, "total": 99.99}"#)
        .with_wait("rate_limit_cooldown")
        .with_callback("manager_approval", "cb-mgr-1", r#""approved""#)
        .with_invoke("charge_payment", r#"{"tx_id": "tx-abc-123"}"#)
        .with_step_result("send_confirmation", r#""email sent""#)
        .build()
        .await;

    // 1. Validate order
    let order: Result<serde_json::Value, String> = ctx
        .step("validate_order", || async { panic!("not executed") })
        .await
        .unwrap();
    let order = order.unwrap();
    assert_eq!(order["order_id"], 42);

    // 2. Rate limit cooldown
    ctx.wait("rate_limit_cooldown", 5).await.unwrap();

    // 3. Manager approval callback
    let approval_handle = ctx
        .create_callback("manager_approval", CallbackOptions::new())
        .await
        .unwrap();
    assert_eq!(approval_handle.callback_id, "cb-mgr-1");
    let approval: String = ctx.callback_result(&approval_handle).unwrap();
    assert_eq!(approval, "approved");

    // 4. Charge payment via invoke
    let payment: serde_json::Value = ctx
        .invoke(
            "charge_payment",
            "payment-service",
            &serde_json::json!({"amount": order["total"]}),
        )
        .await
        .unwrap();
    assert_eq!(payment["tx_id"], "tx-abc-123");

    // 5. Send confirmation
    let confirm: Result<String, String> = ctx
        .step("send_confirmation", || async { panic!("not executed") })
        .await
        .unwrap();
    assert_eq!(confirm.unwrap(), "email sent");

    // Pure replay — no checkpoints sent
    assert_no_checkpoints(&calls).await;
}

/// Verify that after replaying all operations, the context transitions
/// from Replaying to Executing mode.
#[tokio::test]
async fn test_context_transitions_to_executing_after_full_replay() {
    let (mut ctx, _, _ops) = MockDurableContext::new()
        .with_step_result("step1", r#"1"#)
        .with_wait("delay")
        .with_step_result("step2", r#"2"#)
        .build()
        .await;

    assert!(ctx.is_replaying(), "should start in replaying mode");

    let _: Result<i32, String> = ctx
        .step("step1", || async { panic!("not executed") })
        .await
        .unwrap();
    assert!(ctx.is_replaying(), "should still be replaying");

    ctx.wait("delay", 10).await.unwrap();
    assert!(ctx.is_replaying(), "should still be replaying");

    let _: Result<i32, String> = ctx
        .step("step2", || async { panic!("not executed") })
        .await
        .unwrap();
    assert!(
        !ctx.is_replaying(),
        "should transition to executing after all ops replayed"
    );
}

/// Verify that step errors are replayed correctly in a mixed workflow.
#[tokio::test]
async fn test_step_error_in_mixed_workflow() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("validate", r#"true"#)
        .with_wait("delay")
        .with_step_error("charge", "PaymentError", r#""insufficient_funds""#)
        .build()
        .await;

    let _: Result<bool, String> = ctx
        .step("validate", || async { panic!("not executed") })
        .await
        .unwrap();

    ctx.wait("delay", 5).await.unwrap();

    let result: Result<i32, String> = ctx
        .step("charge", || async { panic!("not executed") })
        .await
        .unwrap();
    assert_eq!(result.unwrap_err(), "insufficient_funds");

    assert_no_checkpoints(&calls).await;
}
