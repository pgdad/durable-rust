//! Test assertion helpers for durable Lambda testing.
//!
//! Provide convenience assertions for inspecting checkpoint calls
//! recorded by [`MockBackend`](crate::mock_backend::MockBackend).

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::mock_backend::CheckpointCall;

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
/// let (mut ctx, calls) = MockDurableContext::new()
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
/// let (mut ctx, calls) = MockDurableContext::new()
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
