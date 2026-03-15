//! Test assertion helpers for durable Lambda testing.
//!
//! Provide convenience assertions for inspecting checkpoint calls
//! recorded by [`MockBackend`](crate::mock_backend::MockBackend).

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::mock_backend::{CheckpointCall, OperationRecord};

/// Assert the exact number of checkpoint calls made.
///
/// # Panics
///
/// Panics if the actual count doesn't match `expected`.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::prelude::*;
///
/// let (mut ctx, calls, _ops) = MockDurableContext::new()
///     .with_step_result("s1", r#"1"#)
///     .build()
///     .await;
///
/// // replay step — no checkpoints
/// let _: Result<i32, String> = ctx.step("s1", || async { Ok(0) }).await.unwrap();
///
/// assert_checkpoint_count(&calls, 0).await;
/// # }
/// ```
pub async fn assert_checkpoint_count(calls: &Arc<Mutex<Vec<CheckpointCall>>>, expected: usize) {
    let captured = calls.lock().await;
    assert_eq!(
        captured.len(),
        expected,
        "expected {expected} checkpoint calls, got {}",
        captured.len()
    );
}

/// Assert that no checkpoint calls were made (pure replay test).
///
/// Equivalent to `assert_checkpoint_count(calls, 0)`.
///
/// # Panics
///
/// Panics if any checkpoint calls were recorded.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::prelude::*;
///
/// let (mut ctx, calls, _ops) = MockDurableContext::new()
///     .with_step_result("validate", r#"true"#)
///     .build()
///     .await;
///
/// let _: Result<bool, String> = ctx.step("validate", || async { Ok(false) }).await.unwrap();
///
/// assert_no_checkpoints(&calls).await;
/// # }
/// ```
pub async fn assert_no_checkpoints(calls: &Arc<Mutex<Vec<CheckpointCall>>>) {
    assert_checkpoint_count(calls, 0).await;
}

/// Assert the recorded operation sequence matches the expected `"type:name"` strings.
///
/// Each string should be in `"type:name"` format (e.g., `"step:validate"`,
/// `"wait:cooldown"`). The assertion checks both the count and exact order.
///
/// # Panics
///
/// Panics with a diff showing the first divergence if the sequences don't match.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::prelude::*;
///
/// let (mut ctx, _calls, ops) = MockDurableContext::new()
///     .build()
///     .await;
///
/// let _: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await.unwrap();
///
/// assert_operations(&ops, &["step:validate"]).await;
/// # }
/// ```
pub async fn assert_operations(operations: &Arc<Mutex<Vec<OperationRecord>>>, expected: &[&str]) {
    let recorded = operations.lock().await;
    let actual: Vec<String> = recorded.iter().map(|r| r.to_type_name()).collect();
    let expected: Vec<&str> = expected.to_vec();

    if actual.len() != expected.len() {
        panic!(
            "Operation sequence length mismatch:\n  Expected {} operations: {:?}\n  Actual {} operations:   {:?}",
            expected.len(),
            expected,
            actual.len(),
            actual,
        );
    }

    for (i, (actual_op, expected_op)) in actual.iter().zip(expected.iter()).enumerate() {
        if actual_op != expected_op {
            panic!(
                "Operation sequence mismatch at position {i}:\n  Expected: {expected:?}\n  Actual:   {actual:?}\n  First difference: expected \"{expected_op}\" but got \"{actual_op}\"",
            );
        }
    }
}

/// Assert the recorded operation names match (ignoring types).
///
/// A simplified version of [`assert_operations`] that checks only the
/// operation names without the `"type:"` prefix.
///
/// # Panics
///
/// Panics with a diff if the name sequences don't match.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::prelude::*;
///
/// let (mut ctx, _calls, ops) = MockDurableContext::new()
///     .build()
///     .await;
///
/// let _: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await.unwrap();
///
/// assert_operation_names(&ops, &["validate"]).await;
/// # }
/// ```
pub async fn assert_operation_names(
    operations: &Arc<Mutex<Vec<OperationRecord>>>,
    expected: &[&str],
) {
    let recorded = operations.lock().await;
    let actual: Vec<&str> = recorded.iter().map(|r| r.name.as_str()).collect();
    let expected: Vec<&str> = expected.to_vec();

    if actual.len() != expected.len() {
        panic!(
            "Operation name sequence length mismatch:\n  Expected {} operations: {:?}\n  Actual {} operations:   {:?}",
            expected.len(),
            expected,
            actual.len(),
            actual,
        );
    }

    for (i, (actual_name, expected_name)) in actual.iter().zip(expected.iter()).enumerate() {
        if actual_name != expected_name {
            panic!(
                "Operation name mismatch at position {i}:\n  Expected: {expected:?}\n  Actual:   {actual:?}\n  First difference: expected \"{expected_name}\" but got \"{actual_name}\"",
            );
        }
    }
}

/// Assert the total count of recorded operations.
///
/// # Panics
///
/// Panics if the actual count doesn't match `expected`.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::prelude::*;
///
/// let (mut ctx, _calls, ops) = MockDurableContext::new()
///     .build()
///     .await;
///
/// let _: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await.unwrap();
///
/// assert_operation_count(&ops, 1).await;
/// # }
/// ```
pub async fn assert_operation_count(
    operations: &Arc<Mutex<Vec<OperationRecord>>>,
    expected: usize,
) {
    let recorded = operations.lock().await;
    assert_eq!(
        recorded.len(),
        expected,
        "expected {expected} operations, got {}",
        recorded.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_context::MockDurableContext;
    use durable_lambda_core::context::DurableContext;

    // --- Task 4.1: Single step recording ---

    #[tokio::test]
    async fn test_record_single_step_operation() {
        let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;
        let _: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await.unwrap();

        let recorded = ops.lock().await;
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].name, "validate");
        assert_eq!(recorded[0].operation_type, "step");
        assert_eq!(recorded[0].to_type_name(), "step:validate");
    }

    // --- Task 4.2: Multi-step workflow sequence ---

    #[tokio::test]
    async fn test_record_multi_step_workflow_preserves_order() {
        let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;
        let _: Result<i32, String> = ctx.step("validate", || async { Ok(1) }).await.unwrap();
        let _: Result<i32, String> = ctx.step("charge", || async { Ok(2) }).await.unwrap();
        let _: Result<i32, String> = ctx.step("confirm", || async { Ok(3) }).await.unwrap();

        let recorded = ops.lock().await;
        assert_eq!(recorded.len(), 3);
        assert_eq!(recorded[0].to_type_name(), "step:validate");
        assert_eq!(recorded[1].to_type_name(), "step:charge");
        assert_eq!(recorded[2].to_type_name(), "step:confirm");
    }

    // --- Task 4.3: assert_operations passes for matching ---

    #[tokio::test]
    async fn test_assert_operations_passes_for_matching_sequence() {
        let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;
        let _: Result<i32, String> = ctx.step("validate", || async { Ok(1) }).await.unwrap();
        let _: Result<i32, String> = ctx.step("charge", || async { Ok(2) }).await.unwrap();

        assert_operations(&ops, &["step:validate", "step:charge"]).await;
    }

    // --- Task 4.4: assert_operations panics for mismatch ---

    #[tokio::test]
    #[should_panic(expected = "Operation sequence mismatch")]
    async fn test_assert_operations_panics_for_wrong_order() {
        let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;
        let _: Result<i32, String> = ctx.step("validate", || async { Ok(1) }).await.unwrap();
        let _: Result<i32, String> = ctx.step("charge", || async { Ok(2) }).await.unwrap();

        // Wrong order should panic
        assert_operations(&ops, &["step:charge", "step:validate"]).await;
    }

    #[tokio::test]
    #[should_panic(expected = "Operation sequence length mismatch")]
    async fn test_assert_operations_panics_for_wrong_count() {
        let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;
        let _: Result<i32, String> = ctx.step("validate", || async { Ok(1) }).await.unwrap();

        assert_operations(&ops, &["step:validate", "step:extra"]).await;
    }

    // --- Task 4.5: assert_operation_names ---

    #[tokio::test]
    async fn test_assert_operation_names_passes_for_matching() {
        let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;
        let _: Result<i32, String> = ctx.step("validate", || async { Ok(1) }).await.unwrap();
        let _: Result<i32, String> = ctx.step("charge", || async { Ok(2) }).await.unwrap();

        assert_operation_names(&ops, &["validate", "charge"]).await;
    }

    #[tokio::test]
    #[should_panic(expected = "Operation name mismatch")]
    async fn test_assert_operation_names_panics_for_mismatch() {
        let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;
        let _: Result<i32, String> = ctx.step("validate", || async { Ok(1) }).await.unwrap();

        assert_operation_names(&ops, &["wrong_name"]).await;
    }

    // --- Task 4.6: assert_operation_count ---

    #[tokio::test]
    async fn test_assert_operation_count_passes() {
        let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;
        let _: Result<i32, String> = ctx.step("s1", || async { Ok(1) }).await.unwrap();
        let _: Result<i32, String> = ctx.step("s2", || async { Ok(2) }).await.unwrap();

        assert_operation_count(&ops, 2).await;
    }

    #[tokio::test]
    #[should_panic(expected = "expected 5 operations")]
    async fn test_assert_operation_count_panics_for_mismatch() {
        let (_ctx, _calls, ops) = MockDurableContext::new().build().await;
        assert_operation_count(&ops, 5).await;
    }

    // --- Task 4.7: Child context nesting ---

    #[tokio::test]
    async fn test_child_context_operations_recorded_in_sequence() {
        let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;

        let _: Result<i32, String> = ctx.step("before", || async { Ok(1) }).await.unwrap();

        let _: i32 = ctx
            .child_context("sub", |mut child_ctx: DurableContext| async move {
                let r: Result<i32, String> = child_ctx.step("inner", || async { Ok(42) }).await?;
                Ok(r.unwrap())
            })
            .await
            .unwrap();

        let _: Result<i32, String> = ctx.step("after", || async { Ok(3) }).await.unwrap();

        // Child context operations appear in flat sequence: before, sub (start), inner, after
        let recorded = ops.lock().await;
        // At minimum, "before" and "after" should be recorded.
        // The child context and inner step should also produce checkpoint START calls.
        assert!(
            recorded.len() >= 3,
            "expected at least 3 operations, got {}",
            recorded.len()
        );
        assert_eq!(recorded[0].to_type_name(), "step:before");
        // The last recorded operation should be "after"
        assert_eq!(recorded.last().unwrap().to_type_name(), "step:after");
    }

    // --- Task 4.1 extra: replay mode produces no operation records ---

    #[tokio::test]
    async fn test_replay_mode_produces_no_operation_records() {
        let (mut ctx, _calls, ops) = MockDurableContext::new()
            .with_step_result("validate", "42")
            .build()
            .await;

        let _: Result<i32, String> = ctx
            .step("validate", || async { panic!("not executed") })
            .await
            .unwrap();

        // Replay skips checkpoints, so no operations are recorded
        assert_operation_count(&ops, 0).await;
    }
}
