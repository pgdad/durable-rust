//! Error-path tests for single-operation and batch-operation failure scenarios.
//!
//! Prove that every single-operation failure mode surfaces the correct typed
//! [`DurableError`] variant rather than a panic or silent swallow. Also
//! proves that batch-level failures (parallel/map) are captured per-item in
//! [`BatchResult`] or propagated as typed errors when panics occur. Covers:
//!
//! - TEST-01: Replay mismatch (wrong operation status in history)
//! - TEST-02: Deserialization type mismatch during replay
//! - TEST-03: Checkpoint write failure propagated as [`DurableError::CheckpointFailed`]
//! - TEST-04: Retry exhaustion surfaces the final user error
//! - TEST-05: Callback timeout returns [`DurableError::CallbackFailed`]
//! - TEST-06: Callback explicit failure signal returns [`DurableError::CallbackFailed`]
//! - TEST-07: Invoke error returns [`DurableError::InvokeFailed`]
//! - TEST-08: All parallel branches failing returns `Ok(BatchResult)` with all items Failed
//! - TEST-09: Map item failures at first, middle, and last positions captured per-item
//! - TEST-11: Panic in a parallel branch returns `Err(DurableError::ParallelFailed)`

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
use aws_sdk_lambda::types::{
    CallbackDetails, ChainedInvokeDetails, ErrorObject, Operation, OperationStatus, OperationType,
    OperationUpdate, StepDetails,
};
use aws_smithy_types::DateTime;
use durable_lambda_core::backend::DurableBackend;
use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::operation_id::OperationIdGenerator;
use durable_lambda_core::types::{BatchItemStatus, MapOptions, ParallelOptions, StepOptions};

// ============================================================================
// Shared mock backends
// ============================================================================

/// A backend where every `checkpoint()` call returns `CheckpointFailed`.
///
/// Used to verify that checkpoint write failures propagate correctly as
/// [`DurableError::CheckpointFailed`] rather than panicking or being swallowed.
struct FailingMockBackend;

#[async_trait]
impl DurableBackend for FailingMockBackend {
    async fn checkpoint(
        &self,
        _arn: &str,
        _checkpoint_token: &str,
        _updates: Vec<OperationUpdate>,
        _client_token: Option<&str>,
    ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
        Err(DurableError::checkpoint_failed(
            "test_op",
            io::Error::new(io::ErrorKind::TimedOut, "simulated network timeout"),
        ))
    }

    async fn get_execution_state(
        &self,
        _arn: &str,
        _checkpoint_token: &str,
        _next_marker: &str,
        _max_items: i32,
    ) -> Result<GetDurableExecutionStateOutput, DurableError> {
        Ok(GetDurableExecutionStateOutput::builder().build().unwrap())
    }
}

/// A backend that always succeeds, returning a stable token and no new state.
///
/// Used for tests that need a working backend (retry exhaustion, callback,
/// invoke) where the goal is to test error handling in the replay path, not
/// the checkpoint path.
struct PassingMockBackend;

#[async_trait]
impl DurableBackend for PassingMockBackend {
    async fn checkpoint(
        &self,
        _arn: &str,
        _checkpoint_token: &str,
        _updates: Vec<OperationUpdate>,
        _client_token: Option<&str>,
    ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
        Ok(CheckpointDurableExecutionOutput::builder()
            .checkpoint_token("mock-token")
            .build())
    }

    async fn get_execution_state(
        &self,
        _arn: &str,
        _checkpoint_token: &str,
        _next_marker: &str,
        _max_items: i32,
    ) -> Result<GetDurableExecutionStateOutput, DurableError> {
        Ok(GetDurableExecutionStateOutput::builder().build().unwrap())
    }
}

// ============================================================================
// Helper to compute deterministic operation IDs the same way DurableContext does
// ============================================================================

/// Return the first operation ID generated for a root (no parent) context.
fn first_op_id() -> String {
    let mut gen = OperationIdGenerator::new(None);
    gen.next_id()
}

// ============================================================================
// TEST-01: Replay mismatch — wrong operation status in history
// ============================================================================

/// Verify that a history operation in `Cancelled` status (not Succeeded or
/// Failed) causes `ctx.step()` to return `DurableError::ReplayMismatch`.
///
/// The `extract_step_result` function in `step.rs` only handles `Succeeded` and
/// `Failed`. Any other completed status falls through to the mismatch arm.
#[tokio::test]
async fn test_replay_mismatch_wrong_status() {
    let op_id = first_op_id();

    // Build a Step operation with Cancelled status — neither Succeeded nor Failed.
    let cancelled_op = Operation::builder()
        .id(&op_id)
        .r#type(OperationType::Step)
        .status(OperationStatus::Cancelled)
        .start_timestamp(DateTime::from_secs(0))
        .step_details(StepDetails::builder().attempt(1).build())
        .build()
        .expect("failed to build cancelled Operation");

    let mut ctx = DurableContext::new(
        Arc::new(PassingMockBackend),
        "arn:test".to_string(),
        "initial-token".to_string(),
        vec![cancelled_op],
        None,
    )
    .await
    .expect("DurableContext::new should not fail with mock backend");

    let result: Result<Result<i32, String>, DurableError> =
        ctx.step("test_step", || async { Ok(42) }).await;

    assert!(
        matches!(result, Err(DurableError::ReplayMismatch { .. })),
        "expected DurableError::ReplayMismatch, got: {result:?}"
    );
}

// ============================================================================
// TEST-02: Deserialization type mismatch during replay
// ============================================================================

/// Verify that attempting to replay a step whose cached result is the wrong
/// JSON type returns `DurableError::Deserialization` rather than panicking.
///
/// The history contains `true` (a JSON boolean), but the handler expects `i32`.
/// `serde_json::from_str::<i32>("true")` fails, triggering the deserialization
/// error path in `extract_step_result`.
#[tokio::test]
async fn test_serialization_type_mismatch_returns_deserialization_error() {
    let op_id = first_op_id();

    // Cached result is boolean `true` — incompatible with the expected `i32`.
    let bool_result_op = Operation::builder()
        .id(&op_id)
        .r#type(OperationType::Step)
        .status(OperationStatus::Succeeded)
        .start_timestamp(DateTime::from_secs(0))
        .step_details(StepDetails::builder().attempt(1).result("true").build())
        .build()
        .expect("failed to build bool-result Operation");

    let mut ctx = DurableContext::new(
        Arc::new(PassingMockBackend),
        "arn:test".to_string(),
        "initial-token".to_string(),
        vec![bool_result_op],
        None,
    )
    .await
    .expect("DurableContext::new should not fail");

    // Expect i32 but history has a boolean — must return Deserialization error.
    let result: Result<Result<i32, String>, DurableError> =
        ctx.step("test_step", || async { Ok(0) }).await;

    assert!(
        matches!(result, Err(DurableError::Deserialization { .. })),
        "expected DurableError::Deserialization, got: {result:?}"
    );
}

// ============================================================================
// TEST-03: Checkpoint write failure propagates as CheckpointFailed
// ============================================================================

/// Verify that a checkpoint backend failure on the START checkpoint is
/// propagated as `DurableError::CheckpointFailed`, not silently swallowed.
///
/// Uses `FailingMockBackend` which returns `Err(CheckpointFailed)` on every
/// call. The step sends a START checkpoint immediately on the execute path,
/// so the error surfaces before the closure even runs.
#[tokio::test]
async fn test_checkpoint_failure_propagates() {
    // Empty history → execute mode → first thing step() does is send START.
    let mut ctx = DurableContext::new(
        Arc::new(FailingMockBackend),
        "arn:test".to_string(),
        "initial-token".to_string(),
        vec![],
        None,
    )
    .await
    .expect("DurableContext::new should not fail with failing mock backend");

    let result: Result<Result<i32, String>, DurableError> =
        ctx.step("test_step", || async { Ok(42) }).await;

    assert!(
        matches!(result, Err(DurableError::CheckpointFailed { .. })),
        "expected DurableError::CheckpointFailed, got: {result:?}"
    );
}

// ============================================================================
// TEST-04: Retry exhaustion — surfaces final user error, not StepRetryScheduled
// ============================================================================

/// Verify that `step_with_options` with `retries(3)` and an existing operation
/// at `attempt(4)` exhausts all retries and returns `Ok(Err(user_error))` rather
/// than `Err(DurableError::StepRetryScheduled)`.
///
/// The retry guard is `(current_attempt as u32) <= max_retries`. With
/// `current_attempt = 4` and `max_retries = 3`, the condition is false, so no
/// retry is scheduled and the closure error is returned directly to the caller.
#[tokio::test]
async fn test_retry_exhaustion_surfaces_user_error() {
    let op_id = first_op_id();

    // Simulate re-invocation at attempt 4 (retries(3) means up to 4 attempts total).
    let exhausted_op = Operation::builder()
        .id(&op_id)
        .r#type(OperationType::Step)
        .status(OperationStatus::Pending)
        .start_timestamp(DateTime::from_secs(0))
        .step_details(StepDetails::builder().attempt(4).build())
        .build()
        .expect("failed to build exhausted-attempt Operation");

    let mut ctx = DurableContext::new(
        Arc::new(PassingMockBackend),
        "arn:test".to_string(),
        "initial-token".to_string(),
        vec![exhausted_op],
        None,
    )
    .await
    .expect("DurableContext::new should not fail");

    let options = StepOptions::new().retries(3).backoff_seconds(5);
    let result: Result<Result<i32, String>, DurableError> = ctx
        .step_with_options("exhaust_step", options, || async {
            Err("final failure".to_string())
        })
        .await;

    // Must be Ok(Err(...)) — retries exhausted means the user error is returned.
    let inner = result.expect("outer result should be Ok when retries exhausted");
    let user_error = inner.expect_err("inner result should be the user error");
    assert_eq!(
        user_error, "final failure",
        "user error message should be preserved"
    );
}

// ============================================================================
// TEST-05: Callback timeout returns CallbackFailed
// ============================================================================

/// Verify that a callback in `TimedOut` status causes `callback_result` to
/// return `DurableError::CallbackFailed` containing the callback ID.
#[tokio::test]
async fn test_callback_timeout_returns_callback_failed() {
    let op_id = first_op_id();

    let timed_out_op = Operation::builder()
        .id(&op_id)
        .r#type(OperationType::Callback)
        .status(OperationStatus::TimedOut)
        .name("approval")
        .start_timestamp(DateTime::from_secs(0))
        .callback_details(
            CallbackDetails::builder()
                .callback_id("cb-timeout-1")
                .build(),
        )
        .build()
        .expect("failed to build timed-out callback Operation");

    let mut ctx = DurableContext::new(
        Arc::new(PassingMockBackend),
        "arn:test".to_string(),
        "initial-token".to_string(),
        vec![timed_out_op],
        None,
    )
    .await
    .expect("DurableContext::new should not fail");

    // Replay the callback registration — no checkpoint sent, uses cached op.
    let handle = ctx
        .create_callback("approval", durable_lambda_core::types::CallbackOptions::new())
        .await
        .expect("create_callback should succeed when op is in history");

    // Verify the handle carries the correct callback ID.
    assert_eq!(
        handle.callback_id, "cb-timeout-1",
        "handle should carry the cached callback_id"
    );

    // Now check the result — should fail because status is TimedOut.
    let err = ctx
        .callback_result::<String>(&handle)
        .expect_err("callback_result should return an error for TimedOut status");

    assert!(
        matches!(err, DurableError::CallbackFailed { .. }),
        "expected DurableError::CallbackFailed, got: {err:?}"
    );

    let msg = err.to_string();
    assert!(
        msg.contains("cb-timeout-1"),
        "error message should contain callback_id: {msg}"
    );
}

// ============================================================================
// TEST-06: Callback explicit failure signal returns CallbackFailed
// ============================================================================

/// Verify that a callback in `Failed` status with an `ErrorObject` causes
/// `callback_result` to return `DurableError::CallbackFailed` with the error
/// message from the `ErrorObject`.
#[tokio::test]
async fn test_callback_explicit_failure_returns_callback_failed() {
    let op_id = first_op_id();

    let error_obj = ErrorObject::builder()
        .error_type("RejectionError")
        .error_data("reviewer declined the request")
        .build();

    let failed_op = Operation::builder()
        .id(&op_id)
        .r#type(OperationType::Callback)
        .status(OperationStatus::Failed)
        .name("approval")
        .start_timestamp(DateTime::from_secs(0))
        .callback_details(
            CallbackDetails::builder()
                .callback_id("cb-fail-2")
                .error(error_obj)
                .build(),
        )
        .build()
        .expect("failed to build failed callback Operation");

    let mut ctx = DurableContext::new(
        Arc::new(PassingMockBackend),
        "arn:test".to_string(),
        "initial-token".to_string(),
        vec![failed_op],
        None,
    )
    .await
    .expect("DurableContext::new should not fail");

    let handle = ctx
        .create_callback("approval", durable_lambda_core::types::CallbackOptions::new())
        .await
        .expect("create_callback should succeed when op is in history");

    assert_eq!(handle.callback_id, "cb-fail-2");

    let err = ctx
        .callback_result::<String>(&handle)
        .expect_err("callback_result should return an error for Failed status");

    assert!(
        matches!(err, DurableError::CallbackFailed { .. }),
        "expected DurableError::CallbackFailed, got: {err:?}"
    );

    let msg = err.to_string();
    assert!(
        msg.contains("cb-fail-2"),
        "error message should contain callback_id: {msg}"
    );
    assert!(
        msg.contains("RejectionError"),
        "error message should contain error type: {msg}"
    );
    assert!(
        msg.contains("reviewer declined"),
        "error message should contain error data: {msg}"
    );
}

// ============================================================================
// TEST-07: Invoke error returns InvokeFailed
// ============================================================================

/// Verify that a `ChainedInvoke` operation in `Failed` status causes `ctx.invoke()`
/// to return `DurableError::InvokeFailed` carrying the error details from the
/// `ChainedInvokeDetails`.
#[tokio::test]
async fn test_invoke_error_returns_invoke_failed() {
    let op_id = first_op_id();

    let error_obj = ErrorObject::builder()
        .error_type("TargetFunctionError")
        .error_data("target lambda crashed with OOM")
        .build();

    let failed_invoke_op = Operation::builder()
        .id(&op_id)
        .r#type(OperationType::ChainedInvoke)
        .status(OperationStatus::Failed)
        .name("call_processor")
        .start_timestamp(DateTime::from_secs(0))
        .chained_invoke_details(
            ChainedInvokeDetails::builder().error(error_obj).build(),
        )
        .build()
        .expect("failed to build failed ChainedInvoke Operation");

    let mut ctx = DurableContext::new(
        Arc::new(PassingMockBackend),
        "arn:test".to_string(),
        "initial-token".to_string(),
        vec![failed_invoke_op],
        None,
    )
    .await
    .expect("DurableContext::new should not fail");

    let err = ctx
        .invoke::<String, _>(
            "call_processor",
            "target-lambda",
            &serde_json::json!({"order_id": 123}),
        )
        .await
        .expect_err("invoke should return an error when operation status is Failed");

    assert!(
        matches!(err, DurableError::InvokeFailed { .. }),
        "expected DurableError::InvokeFailed, got: {err:?}"
    );

    let msg = err.to_string();
    assert!(
        msg.contains("call_processor"),
        "error message should contain operation name: {msg}"
    );
    assert!(
        msg.contains("TargetFunctionError"),
        "error message should contain error type: {msg}"
    );
    assert!(
        msg.contains("target lambda crashed"),
        "error message should contain error data: {msg}"
    );
}

// ============================================================================
// TEST-08: All parallel branches failing — captured in BatchResult, not Err
// ============================================================================

/// Verify that when all parallel branches return `Err(DurableError::...)`,
/// the outer `parallel()` call returns `Ok(BatchResult)` with every item
/// having `BatchItemStatus::Failed`.
///
/// This is the key behavioral distinction: branch-level `DurableError` returns
/// are captured per-item in the `BatchResult`. Only panics (JoinError) cause
/// `parallel()` to return `Err(DurableError::ParallelFailed)`.
#[tokio::test]
async fn test_parallel_all_branches_fail() {
    // Empty history → execute mode (no pre-loaded operations).
    let mut ctx = DurableContext::new(
        Arc::new(PassingMockBackend),
        "arn:test".to_string(),
        "initial-token".to_string(),
        vec![],
        None,
    )
    .await
    .expect("DurableContext::new should succeed with passing mock backend");

    // Both branches return DurableError — these are captured as BatchItem::Failed,
    // not propagated as Err from parallel().
    type BranchFn = Box<
        dyn FnOnce(DurableContext) -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>>
            + Send,
    >;

    let branches: Vec<BranchFn> = vec![
        Box::new(|_ctx: DurableContext| {
            Box::pin(async move {
                Err(DurableError::parallel_failed("b0", "branch 0 failed"))
            })
        }),
        Box::new(|_ctx: DurableContext| {
            Box::pin(async move {
                Err(DurableError::parallel_failed("b1", "branch 1 failed"))
            })
        }),
    ];

    let result = ctx
        .parallel("all_fail", branches, ParallelOptions::new())
        .await;

    // Must be Ok — branch-level errors do NOT propagate as Err from parallel().
    let batch_result = result.expect(
        "parallel() should return Ok(BatchResult) even when all branches return Err; \
         branch errors are captured per-item, not propagated as DurableError",
    );

    assert_eq!(
        batch_result.results.len(),
        2,
        "BatchResult should have one item per branch"
    );

    // Every item must be Failed with an error message.
    for item in &batch_result.results {
        assert_eq!(
            item.status,
            BatchItemStatus::Failed,
            "item {} should be Failed but was {:?}",
            item.index,
            item.status
        );
        assert!(
            item.error.is_some(),
            "item {} should have an error message",
            item.index
        );
        assert!(
            item.result.is_none(),
            "item {} should have no result value when Failed",
            item.index
        );
    }
}

// ============================================================================
// TEST-09: Map item failures at first, middle, and last positions
// ============================================================================

/// Verify that map item failures at the first (index 0), middle (index 2),
/// and last (index 4) positions are each captured as `BatchItemStatus::Failed`
/// with an error message, while the passing items (index 1, 3) are captured
/// as `BatchItemStatus::Succeeded` with correct computed values.
///
/// This confirms that per-item error isolation works at all positions in the
/// collection, not just a single hardcoded index.
#[tokio::test]
async fn test_map_item_failures_at_different_positions() {
    // Empty history → execute mode.
    let mut ctx = DurableContext::new(
        Arc::new(PassingMockBackend),
        "arn:test".to_string(),
        "initial-token".to_string(),
        vec![],
        None,
    )
    .await
    .expect("DurableContext::new should succeed with passing mock backend");

    // Items 0, 2, 4 fail; items 1 and 3 succeed with value item * 10.
    let items = vec![0i32, 1, 2, 3, 4];
    let result = ctx
        .map(
            "position_test",
            items,
            MapOptions::new(),
            |item: i32, _ctx: DurableContext| async move {
                if item == 0 || item == 2 || item == 4 {
                    Err(DurableError::map_failed(
                        "item",
                        format!("item {item} failed"),
                    ))
                } else {
                    Ok(item * 10)
                }
            },
        )
        .await
        .expect("map() should return Ok(BatchResult) even with per-item failures");

    assert_eq!(
        result.results.len(),
        5,
        "BatchResult should have one item per input"
    );

    // Items at index 0, 2, 4 should be Failed.
    for failed_idx in [0usize, 2, 4] {
        let item = &result.results[failed_idx];
        assert_eq!(
            item.status,
            BatchItemStatus::Failed,
            "item at index {failed_idx} should be Failed"
        );
        assert!(
            item.error.is_some(),
            "item at index {failed_idx} should have an error message"
        );
        assert!(
            item.result.is_none(),
            "item at index {failed_idx} should have no result when Failed"
        );
    }

    // Items at index 1 and 3 should be Succeeded with item * 10.
    let item1 = &result.results[1];
    assert_eq!(
        item1.status,
        BatchItemStatus::Succeeded,
        "item at index 1 should be Succeeded"
    );
    assert_eq!(
        item1.result,
        Some(10),
        "item 1 (value=1) * 10 should equal 10"
    );

    let item3 = &result.results[3];
    assert_eq!(
        item3.status,
        BatchItemStatus::Succeeded,
        "item at index 3 should be Succeeded"
    );
    assert_eq!(
        item3.result,
        Some(30),
        "item 3 (value=3) * 10 should equal 30"
    );
}

// ============================================================================
// TEST-11: Panic in a parallel branch returns Err(DurableError::ParallelFailed)
// ============================================================================

/// Verify that a panic inside a parallel branch causes `parallel()` to return
/// `Err(DurableError::ParallelFailed)`, not a process abort.
///
/// `tokio::spawn` catches panics as `JoinError`. The parallel implementation
/// converts the `JoinError` via `map_err` to `DurableError::ParallelFailed`
/// and propagates it with `?`. This is distinct from TEST-08: branch `Err`
/// returns are captured in `BatchResult`, but panics propagate as `Err`.
#[tokio::test]
async fn test_parallel_branch_panic_returns_error() {
    // Empty history → execute mode.
    let mut ctx = DurableContext::new(
        Arc::new(PassingMockBackend),
        "arn:test".to_string(),
        "initial-token".to_string(),
        vec![],
        None,
    )
    .await
    .expect("DurableContext::new should succeed with passing mock backend");

    type BranchFn = Box<
        dyn FnOnce(DurableContext) -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>>
            + Send,
    >;

    let branches: Vec<BranchFn> = vec![
        // First branch succeeds normally.
        Box::new(|_ctx: DurableContext| Box::pin(async move { Ok(42i32) })),
        // Second branch panics — tokio catches this as JoinError.
        Box::new(|_ctx: DurableContext| {
            Box::pin(async move {
                panic!("deliberate branch panic for testing");
                #[allow(unreachable_code)]
                Ok(0i32)
            })
        }),
    ];

    let result = ctx
        .parallel("panic_test", branches, ParallelOptions::new())
        .await;

    // Must be Err(ParallelFailed) — panics propagate via JoinError, not BatchResult.
    assert!(
        matches!(result, Err(DurableError::ParallelFailed { .. })),
        "expected Err(DurableError::ParallelFailed) when a branch panics, got: {result:?}"
    );
}
