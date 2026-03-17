//! End-to-end tests for the saga / compensation pattern (FEAT-28).
//!
//! Covers: reverse-order execution, per-item failure capture, forward-error
//! skip, empty no-op, error code correctness, checkpoint sequence, and
//! partial rollback resume.
//!
//! All tests use MockDurableContext — no AWS credentials needed.

use std::sync::Arc;
use std::sync::Mutex;

use aws_sdk_lambda::types::{OperationAction, OperationStatus, OperationType};
use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::operation_id::OperationIdGenerator;
use durable_lambda_core::types::CompensationStatus;
use durable_lambda_testing::mock_backend::MockBackend;
use durable_lambda_testing::prelude::*;

// ============================================================================
// Test 1: Compensations fire in reverse registration order
// ============================================================================

#[tokio::test]
async fn test_compensation_reverse_order() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let execution_order: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Register 3 compensations: step_a, step_b, step_c
    for label in &["step_a", "step_b", "step_c"] {
        let order = execution_order.clone();
        let label_owned = label.to_string();
        let result: Result<i32, String> = ctx
            .step_with_compensation(
                label,
                move || async move { Ok::<i32, String>(1) },
                move |_value| {
                    let order = order.clone();
                    let label = label_owned.clone();
                    async move {
                        order.lock().unwrap().push(label);
                        Ok(())
                    }
                },
            )
            .await
            .expect("step_with_compensation should not fail");
        assert!(result.is_ok(), "forward step should succeed");
    }

    let comp_result = ctx
        .run_compensations()
        .await
        .expect("run_compensations should not fail");

    assert!(
        comp_result.all_succeeded,
        "all compensations should succeed"
    );
    assert_eq!(
        comp_result.items.len(),
        3,
        "should have 3 compensation items"
    );

    // Registered order: step_a, step_b, step_c → execution order: step_c, step_b, step_a (LIFO)
    let order = execution_order.lock().unwrap();
    assert_eq!(
        order.as_slice(),
        &["step_c", "step_b", "step_a"],
        "compensations must run in reverse registration order, got: {order:?}"
    );
}

// ============================================================================
// Test 2: Compensation failure captured per-item; remaining still run
// ============================================================================

#[tokio::test]
async fn test_compensation_failure_captured_per_item() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    // Register step_a — succeeds
    let result_a: Result<i32, String> = ctx
        .step_with_compensation(
            "step_a",
            move || async move { Ok::<i32, String>(1) },
            |_| async move { Ok(()) },
        )
        .await
        .expect("step_a should not fail");
    assert!(result_a.is_ok());

    // Register step_b — compensation FAILS
    let result_b: Result<i32, String> = ctx
        .step_with_compensation(
            "step_b",
            move || async move { Ok::<i32, String>(2) },
            |_| async move { Err(DurableError::compensation_failed("step_b", "refund failed")) },
        )
        .await
        .expect("step_b forward should not fail");
    assert!(result_b.is_ok());

    // Register step_c — succeeds
    let result_c: Result<i32, String> = ctx
        .step_with_compensation(
            "step_c",
            move || async move { Ok::<i32, String>(3) },
            |_| async move { Ok(()) },
        )
        .await
        .expect("step_c should not fail");
    assert!(result_c.is_ok());

    let comp_result = ctx
        .run_compensations()
        .await
        .expect("run_compensations should not fail");

    assert_eq!(comp_result.items.len(), 3, "should have 3 items");

    // LIFO: step_c runs first, then step_b, then step_a
    assert_eq!(
        comp_result.items[0].name, "step_c",
        "first item (LIFO) should be step_c"
    );
    assert_eq!(
        comp_result.items[0].status,
        CompensationStatus::Succeeded,
        "step_c should succeed"
    );

    assert_eq!(
        comp_result.items[1].name, "step_b",
        "second item (LIFO) should be step_b"
    );
    assert_eq!(
        comp_result.items[1].status,
        CompensationStatus::Failed,
        "step_b compensation should fail"
    );
    assert!(
        comp_result.items[1].error.is_some(),
        "step_b should have an error message"
    );

    assert_eq!(
        comp_result.items[2].name, "step_a",
        "third item (LIFO) should be step_a"
    );
    assert_eq!(
        comp_result.items[2].status,
        CompensationStatus::Succeeded,
        "step_a should succeed"
    );

    assert!(
        !comp_result.all_succeeded,
        "all_succeeded must be false when any compensation fails"
    );
}

// ============================================================================
// Test 3: Forward step returning Err does not register compensation
// ============================================================================

#[tokio::test]
async fn test_compensation_not_registered_on_forward_error() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    // Forward step returns a user error — no compensation should be registered
    let result: Result<i32, String> = ctx
        .step_with_compensation(
            "failing_step",
            move || async move { Err::<i32, String>("user error".to_string()) },
            |_value| async move { Ok(()) },
        )
        .await
        .expect("outer DurableError should not occur");

    assert_eq!(result, Err("user error".to_string()));

    let comp_result = ctx
        .run_compensations()
        .await
        .expect("run_compensations should not fail");

    assert!(
        comp_result.items.is_empty(),
        "no compensation should be registered when forward step fails"
    );
    assert!(
        comp_result.all_succeeded,
        "empty result should be all_succeeded=true"
    );
}

// ============================================================================
// Test 4: run_compensations with 0 registered compensations is a no-op
// ============================================================================

#[tokio::test]
async fn test_compensation_empty_is_noop() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;

    let comp_result = ctx
        .run_compensations()
        .await
        .expect("run_compensations should not fail on empty");

    assert!(
        comp_result.items.is_empty(),
        "items should be empty with no compensations registered"
    );
    assert!(
        comp_result.all_succeeded,
        "all_succeeded should be true when no compensations run"
    );

    // No checkpoint calls should be made for empty compensation run
    let captured = calls.lock().await;
    assert_eq!(
        captured.len(),
        0,
        "no checkpoints should be made for empty run, got {}",
        captured.len()
    );
}

// ============================================================================
// Test 5: DurableError::CompensationFailed has code "COMPENSATION_FAILED"
// ============================================================================

#[tokio::test]
async fn test_compensation_error_code() {
    let err = DurableError::compensation_failed("my_op", "something went wrong");
    assert_eq!(
        err.code(),
        "COMPENSATION_FAILED",
        "CompensationFailed error code must be COMPENSATION_FAILED, got: {}",
        err.code()
    );
}

// ============================================================================
// Test 6: Checkpoint sequence — Context/START + Context/SUCCEED for compensation
// ============================================================================

#[tokio::test]
async fn test_compensation_checkpoint_sequence() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;

    let result: Result<i32, String> = ctx
        .step_with_compensation(
            "refund",
            move || async move { Ok::<i32, String>(99) },
            |_value| async move { Ok(()) },
        )
        .await
        .expect("step_with_compensation should not fail");
    assert!(result.is_ok());

    // Capture step checkpoint count before running compensations
    let step_call_count = calls.lock().await.len();

    let comp_result = ctx
        .run_compensations()
        .await
        .expect("run_compensations should not fail");
    assert!(comp_result.all_succeeded);

    let all_calls = calls.lock().await;
    let comp_calls = &all_calls[step_call_count..];

    assert_eq!(
        comp_calls.len(),
        2,
        "compensation should produce exactly 2 checkpoint calls (START + SUCCEED), got {}",
        comp_calls.len()
    );

    // First call: Context/START with sub_type "Compensation"
    let start_update = &comp_calls[0].updates[0];
    assert_eq!(
        start_update.r#type(),
        &OperationType::Context,
        "compensation START must use OperationType::Context"
    );
    assert_eq!(
        start_update.action(),
        &OperationAction::Start,
        "compensation first checkpoint must be Start"
    );
    assert_eq!(
        start_update.sub_type(),
        Some("Compensation"),
        "compensation START must have sub_type='Compensation'"
    );
    assert_eq!(
        start_update.name(),
        Some("refund"),
        "compensation START must carry the step name"
    );

    // Second call: Context/SUCCEED with sub_type "Compensation"
    let succeed_update = &comp_calls[1].updates[0];
    assert_eq!(
        succeed_update.r#type(),
        &OperationType::Context,
        "compensation SUCCEED must use OperationType::Context"
    );
    assert_eq!(
        succeed_update.action(),
        &OperationAction::Succeed,
        "compensation second checkpoint must be Succeed"
    );
    assert_eq!(
        succeed_update.sub_type(),
        Some("Compensation"),
        "compensation SUCCEED must have sub_type='Compensation'"
    );
}

// ============================================================================
// Test 7: Partial rollback resume — pre-loaded compensations skip closure execution
// ============================================================================

#[tokio::test]
async fn test_compensation_partial_rollback_resume() {
    // Strategy: register 3 compensations via step_with_compensation.
    // The forward steps consume op IDs 1, 2, 3.
    // run_compensations (LIFO) generates comp op IDs 4 (step_c), 5 (step_b), 6 (step_a).
    // Pre-load IDs 4 and 5 as Succeeded — only step_a (ID 6) should execute.

    let mut id_gen = OperationIdGenerator::new(None);

    // Advance past the 3 step op IDs (positions 1, 2, 3)
    let _step_a_id = id_gen.next_id();
    let _step_b_id = id_gen.next_id();
    let _step_c_id = id_gen.next_id();

    // Compensation IDs (LIFO: step_c first, step_b second, step_a third)
    let comp_step_c_id = id_gen.next_id(); // position 4 — step_c compensation (runs 1st, LIFO)
    let comp_step_b_id = id_gen.next_id(); // position 5 — step_b compensation (runs 2nd, LIFO)
                                           // comp_step_a_id would be position 6 — will actually execute

    // Build a context with comp_step_c and comp_step_b already in history as Succeeded
    let comp_c_op = aws_sdk_lambda::types::Operation::builder()
        .id(&comp_step_c_id)
        .r#type(OperationType::Context)
        .status(OperationStatus::Succeeded)
        .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
        .build()
        .expect("failed to build context op for step_c compensation");

    let comp_b_op = aws_sdk_lambda::types::Operation::builder()
        .id(&comp_step_b_id)
        .r#type(OperationType::Context)
        .status(OperationStatus::Succeeded)
        .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
        .build()
        .expect("failed to build context op for step_b compensation");

    let (backend, calls, _ops) = MockBackend::new("mock-token");
    let mut ctx = DurableContext::new(
        std::sync::Arc::new(backend),
        "arn:test".to_string(),
        "tok".to_string(),
        vec![comp_c_op, comp_b_op],
        None,
    )
    .await
    .expect("DurableContext::new should not fail");

    // Track which compensation closures actually execute
    let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Register 3 compensable steps in order: step_a, step_b, step_c
    // (In execute mode since our pre-loaded ops are Context type, not Step type;
    //  the replay engine will be in Executing mode for steps and Replaying for comps)
    for label in &["step_a", "step_b", "step_c"] {
        let executed_clone = executed.clone();
        let label_owned = label.to_string();
        let result: Result<Result<i32, String>, DurableError> = ctx
            .step_with_compensation(
                label,
                move || async move { Ok::<i32, String>(42) },
                move |_value| {
                    let exec = executed_clone.clone();
                    let lbl = label_owned.clone();
                    async move {
                        exec.lock().unwrap().push(lbl);
                        Ok(())
                    }
                },
            )
            .await;
        result
            .expect("step_with_compensation should not fail")
            .expect("forward step should succeed");
    }

    let comp_result = ctx
        .run_compensations()
        .await
        .expect("run_compensations should not fail");

    // Verify all 3 items present (2 replayed, 1 executed)
    assert_eq!(
        comp_result.items.len(),
        3,
        "should have 3 compensation items"
    );
    assert!(
        comp_result.all_succeeded,
        "all compensations should succeed"
    );

    // Only step_a's compensation should have actually executed
    let exec = executed.lock().unwrap();
    assert_eq!(
        exec.as_slice(),
        &["step_a"],
        "only step_a compensation should execute; step_b and step_c are replayed, got: {exec:?}"
    );

    // Verify checkpoint calls: only 2 (START + SUCCEED for step_a)
    let captured = calls.lock().await;
    // The 3 forward steps also produce checkpoints — we only count compensation calls.
    // Compensation calls come after all step calls.
    // Each step: 2 calls (START + SUCCEED) × 3 = 6 step calls, then 2 comp calls for step_a
    let step_checkpoint_count = 6; // 3 steps × 2 checkpoints each
    let comp_checkpoint_count = captured.len() - step_checkpoint_count;
    assert_eq!(
        comp_checkpoint_count, 2,
        "only 2 checkpoint calls (START+SUCCEED) for step_a compensation, got {comp_checkpoint_count}"
    );
}
