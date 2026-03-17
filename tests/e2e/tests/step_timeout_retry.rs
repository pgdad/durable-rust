//! E2E tests for step timeout and conditional retry features.
//!
//! Verify that `timeout_seconds` and `retry_if` on [`StepOptions`] work
//! correctly end-to-end using [`MockDurableContext`]. Covers FEAT-12
//! (per-step timeouts) and FEAT-16 (conditional retry predicates).
//!
//! # Test coverage
//!
//! ## FEAT-12: Step timeouts
//! - [`test_step_timeout_fires`] — step exceeding timeout returns [`DurableError::StepTimeout`]
//! - [`test_step_within_timeout_succeeds`] — fast step returns `Ok(Ok(42))`
//! - [`test_step_timeout_zero_panics`] — `timeout_seconds(0)` panics at construction
//! - [`test_step_timeout_error_code`] — `StepTimeout` code is `"STEP_TIMEOUT"`
//!
//! ## FEAT-16: Conditional retry predicates
//! - [`test_conditional_retry_transient_error_retries`] — predicate returns true → RETRY checkpoint
//! - [`test_conditional_retry_non_transient_fails_fast`] — predicate returns false → FAIL checkpoint
//! - [`test_no_retry_if_retries_all_errors`] — no predicate retries all errors (backward compatible)

use durable_lambda_core::error::DurableError;
use durable_lambda_core::types::StepOptions;
use durable_lambda_testing::prelude::*;
use std::time::Duration;

// ============================================================================
// FEAT-12: Step timeout tests
// ============================================================================

/// A step that sleeps longer than its timeout returns `DurableError::StepTimeout`.
///
/// The timeout fires before the 60-second sleep completes. The task is aborted
/// and `Err(DurableError::StepTimeout { operation_name: "slow_step" })` is returned.
#[tokio::test]
async fn test_step_timeout_fires() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let result: Result<Result<i32, String>, DurableError> = ctx
        .step_with_options(
            "slow_step",
            StepOptions::new().timeout_seconds(1),
            || async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok::<i32, String>(42)
            },
        )
        .await;

    // Outer result should be Err (SDK-level error, not a user error).
    assert!(result.is_err(), "expected Err from timed-out step, got Ok");

    let err = result.unwrap_err();
    match &err {
        DurableError::StepTimeout { operation_name, .. } => {
            assert!(
                operation_name.contains("slow_step"),
                "operation_name should contain 'slow_step', got: {operation_name}"
            );
        }
        other => panic!("expected DurableError::StepTimeout, got: {other:?}"),
    }
}

/// A step that completes before its timeout returns the normal `Ok(Ok(42))`.
///
/// The 5-second timeout is well beyond the immediate return, so no timeout fires.
#[tokio::test]
async fn test_step_within_timeout_succeeds() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let result: Result<Result<i32, String>, DurableError> = ctx
        .step_with_options(
            "fast_step",
            StepOptions::new().timeout_seconds(5),
            || async { Ok::<i32, String>(42) },
        )
        .await;

    assert!(
        result.is_ok(),
        "expected Ok from fast step, got: {result:?}"
    );
    let inner = result.unwrap();
    assert!(
        inner.is_ok(),
        "expected inner Ok(42), got inner Err: {inner:?}"
    );
    assert_eq!(inner.unwrap(), 42);
}

/// `StepOptions::new().timeout_seconds(0)` panics at construction time.
///
/// Zero is not a valid timeout — the guard fires synchronously before any
/// async execution begins. This test uses `#[test]` (not `#[tokio::test]`)
/// because the panic happens in synchronous builder code.
#[test]
#[should_panic(expected = "StepOptions::timeout_seconds: seconds must be > 0, got 0")]
fn test_step_timeout_zero_panics() {
    StepOptions::new().timeout_seconds(0);
}

/// `DurableError::step_timeout("op").code()` returns `"STEP_TIMEOUT"`.
///
/// Also verifies that the display message contains the operation name.
#[test]
fn test_step_timeout_error_code() {
    let err = DurableError::step_timeout("op");
    assert_eq!(err.code(), "STEP_TIMEOUT");
    assert!(
        err.to_string().contains("op"),
        "display message should contain operation name 'op', got: {}",
        err.to_string()
    );
}

// ============================================================================
// FEAT-16: Conditional retry predicate tests
// ============================================================================

/// A transient error matching the `retry_if` predicate triggers a RETRY checkpoint.
///
/// With `retries(2)` and a predicate matching `"transient"`, an error containing
/// `"transient error"` satisfies the predicate. The SDK sends a RETRY checkpoint
/// and returns `Err(DurableError::StepRetryScheduled { .. })` to signal exit.
#[tokio::test]
async fn test_conditional_retry_transient_error_retries() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;

    let result: Result<Result<i32, String>, DurableError> = ctx
        .step_with_options(
            "check",
            StepOptions::new()
                .retries(2)
                .retry_if(|e: &String| e.contains("transient")),
            || async { Err::<i32, String>("transient error".into()) },
        )
        .await;

    // Should be Err(StepRetryScheduled) — a RETRY was checkpointed.
    assert!(
        result.is_err(),
        "expected Err(StepRetryScheduled) for transient error, got Ok"
    );
    match result.unwrap_err() {
        DurableError::StepRetryScheduled { operation_name, .. } => {
            assert!(
                operation_name.contains("check"),
                "operation_name should contain 'check', got: {operation_name}"
            );
        }
        other => panic!("expected DurableError::StepRetryScheduled, got: {other:?}"),
    }

    // Verify checkpoint calls include a RETRY action.
    let captured = calls.lock().await;
    let has_retry = captured.iter().any(|call| {
        call.updates
            .iter()
            .any(|u| u.action() == &aws_sdk_lambda::types::OperationAction::Retry)
    });
    assert!(
        has_retry,
        "expected at least one checkpoint call with RETRY action, got calls: {captured:?}"
    );
}

/// A non-transient error that fails the `retry_if` predicate triggers FAIL immediately.
///
/// With `retries(2)` and a predicate matching `"transient"`, an error containing
/// `"permanent error"` fails the predicate. The SDK sends a FAIL checkpoint without
/// consuming the retry budget. Result is `Ok(Err("permanent error"))`.
#[tokio::test]
async fn test_conditional_retry_non_transient_fails_fast() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;

    let result: Result<Result<i32, String>, DurableError> = ctx
        .step_with_options(
            "check",
            StepOptions::new()
                .retries(2)
                .retry_if(|e: &String| e.contains("transient")),
            || async { Err::<i32, String>("permanent error".into()) },
        )
        .await;

    // Should be Ok(Err("permanent error")) — error checkpointed as FAIL, not RETRY.
    assert!(
        result.is_ok(),
        "expected Ok(Err(...)) for non-transient error, got: {result:?}"
    );
    let inner = result.unwrap();
    assert!(
        inner.is_err(),
        "expected inner Err(\"permanent error\"), got inner Ok: {inner:?}"
    );
    assert_eq!(inner.unwrap_err(), "permanent error");

    // Verify checkpoint calls include a FAIL action, not RETRY.
    let captured = calls.lock().await;
    let has_fail = captured.iter().any(|call| {
        call.updates
            .iter()
            .any(|u| u.action() == &aws_sdk_lambda::types::OperationAction::Fail)
    });
    let has_retry = captured.iter().any(|call| {
        call.updates
            .iter()
            .any(|u| u.action() == &aws_sdk_lambda::types::OperationAction::Retry)
    });
    assert!(
        has_fail,
        "expected at least one FAIL checkpoint, got calls: {captured:?}"
    );
    assert!(
        !has_retry,
        "expected NO RETRY checkpoint for non-transient error, but found one in calls: {captured:?}"
    );
}

/// Without a `retry_if` predicate, all errors are retried (backward compatible behavior).
///
/// With `retries(2)` and no predicate, any error causes a RETRY checkpoint.
/// This proves the default behavior is preserved — no predicate means retry all errors.
#[tokio::test]
async fn test_no_retry_if_retries_all_errors() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;

    let result: Result<Result<i32, String>, DurableError> = ctx
        .step_with_options("check", StepOptions::new().retries(2), || async {
            Err::<i32, String>("any error".into())
        })
        .await;

    // Should be Err(StepRetryScheduled) — existing behavior preserved.
    assert!(
        result.is_err(),
        "expected Err(StepRetryScheduled) for error with retries and no predicate, got Ok"
    );
    match result.unwrap_err() {
        DurableError::StepRetryScheduled { operation_name, .. } => {
            assert!(
                operation_name.contains("check"),
                "operation_name should contain 'check', got: {operation_name}"
            );
        }
        other => panic!("expected DurableError::StepRetryScheduled, got: {other:?}"),
    }

    // Verify checkpoint includes a RETRY action.
    let captured = calls.lock().await;
    let has_retry = captured.iter().any(|call| {
        call.updates
            .iter()
            .any(|u| u.action() == &aws_sdk_lambda::types::OperationAction::Retry)
    });
    assert!(
        has_retry,
        "expected at least one RETRY checkpoint when no retry_if predicate is set, got calls: {captured:?}"
    );
}
