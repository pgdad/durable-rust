//! Boundary condition tests for options and operation edge cases.
//!
//! Prove that all option boundary values and operation name edge cases have
//! defined, tested behavior. This module covers:
//!
//! - TEST-12: Zero-duration wait — valid on both execute and replay paths
//! - TEST-13: Map `batch_size` edge cases — zero panics, one processes sequentially,
//!            larger-than-collection processes all in a single batch
//! - TEST-14: Parallel with 0 branches (empty `BatchResult` + 2 checkpoints) and 1 branch
//! - TEST-15: Operation names — empty string, unicode, and 255+ character names accepted
//! - TEST-16: Negative option values panic with descriptive messages (integration-level)

use std::future::Future;
use std::pin::Pin;

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::types::{
    BatchItemStatus, CallbackOptions, MapOptions, ParallelOptions, StepOptions,
};
use durable_lambda_testing::prelude::*;

// ============================================================================
// TEST-12: Zero-duration wait
// ============================================================================

/// Verify that `wait("zero_wait", 0)` on the execute path returns
/// `Err(DurableError::WaitSuspended)`.
///
/// Zero-duration wait is semantically valid — the server interprets it as
/// "resume immediately" — and must not be rejected at the SDK layer.
#[tokio::test]
async fn test_zero_duration_wait_execute_path() {
    // Empty history → execute mode. No pre-loaded ops.
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let result = ctx.wait("zero_wait", 0).await;

    // On execute path, wait suspends the function (WaitSuspended).
    assert!(
        matches!(result, Err(DurableError::WaitSuspended { .. })),
        "expected Err(DurableError::WaitSuspended) for zero-duration wait on execute path, got: {result:?}"
    );
}

/// Verify that `wait("zero_wait", 0)` on the replay path returns `Ok(())`.
///
/// When the wait operation is already in history (completed), the replay
/// engine finds it and returns `Ok(())` immediately, regardless of the
/// original duration.
#[tokio::test]
async fn test_zero_duration_wait_replay_path() {
    // Pre-loaded wait → replay mode. The wait has already completed.
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_wait("zero_wait")
        .build()
        .await;

    let result = ctx.wait("zero_wait", 0).await;

    assert!(
        matches!(result, Ok(())),
        "expected Ok(()) for zero-duration wait on replay path, got: {result:?}"
    );

    // Pure replay — no checkpoints should have been made.
    assert_no_checkpoints(&calls).await;
}

// ============================================================================
// TEST-13: Map batch_size edge cases
// ============================================================================

/// Verify that `MapOptions::new().batch_size(0)` panics with a descriptive message.
///
/// Confirms the Phase 4 builder validation is enforced at integration level:
/// zero batch_size is an invalid configuration that would cause infinite loops.
#[test]
#[should_panic(expected = "MapOptions::batch_size: size must be > 0, got 0")]
fn test_map_batch_size_zero_panics() {
    MapOptions::new().batch_size(0);
}

/// Verify that `batch_size(1)` processes items sequentially (one at a time).
///
/// With batch_size=1, each item is processed alone before the next begins.
/// All items should still succeed and return correct results.
#[tokio::test]
async fn test_map_batch_size_one_processes_sequentially() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let items = vec![1i32, 2, 3];
    let result = ctx
        .map(
            "batch1",
            items,
            MapOptions::new().batch_size(1),
            |item: i32, _ctx: DurableContext| async move { Ok(item * 10) },
        )
        .await
        .unwrap();

    assert_eq!(
        result.results.len(),
        3,
        "batch_size(1) should process all 3 items"
    );

    // Verify all items succeeded with correct computed values.
    let mut sorted = result.results.clone();
    sorted.sort_by_key(|r| r.index);

    assert_eq!(sorted[0].status, BatchItemStatus::Succeeded);
    assert_eq!(sorted[0].result, Some(10)); // 1 * 10

    assert_eq!(sorted[1].status, BatchItemStatus::Succeeded);
    assert_eq!(sorted[1].result, Some(20)); // 2 * 10

    assert_eq!(sorted[2].status, BatchItemStatus::Succeeded);
    assert_eq!(sorted[2].result, Some(30)); // 3 * 10
}

/// Verify that `batch_size` larger than the collection processes all items in one batch.
///
/// When batch_size=100 but only 2 items exist, the SDK processes both items
/// concurrently in a single batch — no division errors or truncation.
#[tokio::test]
async fn test_map_batch_size_exceeds_collection() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let items = vec![1i32, 2];
    let result = ctx
        .map(
            "big_batch",
            items,
            MapOptions::new().batch_size(100),
            |item: i32, _ctx: DurableContext| async move { Ok(item * 5) },
        )
        .await
        .unwrap();

    assert_eq!(
        result.results.len(),
        2,
        "all 2 items should be processed even when batch_size > collection"
    );

    let mut sorted = result.results.clone();
    sorted.sort_by_key(|r| r.index);

    assert_eq!(sorted[0].status, BatchItemStatus::Succeeded);
    assert_eq!(sorted[0].result, Some(5)); // 1 * 5

    assert_eq!(sorted[1].status, BatchItemStatus::Succeeded);
    assert_eq!(sorted[1].result, Some(10)); // 2 * 5
}

// ============================================================================
// TEST-14: Parallel with 0 and 1 branches
// ============================================================================

/// Verify that `parallel()` with zero branches returns an empty `BatchResult`
/// and still produces exactly 2 checkpoints (outer Context/START + Context/SUCCEED).
///
/// An empty parallel block is a valid operation — the workflow should not
/// crash or hang when given no branches.
#[tokio::test]
async fn test_parallel_zero_branches() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;

    type BranchFn = Box<
        dyn FnOnce(
                DurableContext,
            ) -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>>
            + Send,
    >;

    let branches: Vec<BranchFn> = Vec::new();

    let result = ctx
        .parallel("empty", branches, ParallelOptions::new())
        .await
        .unwrap();

    assert_eq!(
        result.results.len(),
        0,
        "zero branches should produce empty BatchResult"
    );

    // The outer parallel block still sends START and SUCCEED checkpoints.
    let captured = calls.lock().await;
    assert_eq!(
        captured.len(),
        2,
        "zero-branch parallel should produce exactly 2 checkpoints (START + SUCCEED), got {}",
        captured.len()
    );
}

/// Verify that `parallel()` with a single branch works correctly.
///
/// A single-branch parallel is an edge case — the result should have exactly
/// one item with status Succeeded and the correct return value.
#[tokio::test]
async fn test_parallel_one_branch() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    type BranchFn = Box<
        dyn FnOnce(
                DurableContext,
            ) -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>>
            + Send,
    >;

    let branches: Vec<BranchFn> = vec![Box::new(|_ctx: DurableContext| {
        Box::pin(async move { Ok(42i32) })
    })];

    let result = ctx
        .parallel("single", branches, ParallelOptions::new())
        .await
        .unwrap();

    assert_eq!(
        result.results.len(),
        1,
        "single branch should produce exactly 1 result"
    );
    assert_eq!(
        result.results[0].status,
        BatchItemStatus::Succeeded,
        "single branch should have Succeeded status"
    );
    assert_eq!(
        result.results[0].result,
        Some(42),
        "single branch result should match the returned value"
    );
}

// ============================================================================
// TEST-15: Operation name edge cases
// ============================================================================

/// Verify that an empty string `""` is accepted as a step operation name.
///
/// The SDK must not reject empty names — they are positional, not name-based,
/// and the checkpoint protocol does not require non-empty names.
#[tokio::test]
async fn test_operation_name_empty_string() {
    let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;

    let result: Result<i32, String> = ctx.step("", || async { Ok(1i32) }).await.unwrap();

    assert_eq!(
        result.unwrap(),
        1,
        "step with empty name should return the closure result"
    );

    // Verify the operation was recorded with the empty name.
    assert_operations(&ops, &["step:"]).await;
}

/// Verify that unicode characters are accepted as a step operation name.
///
/// Operation names may contain arbitrary unicode — the SDK passes names
/// through to the checkpoint API without character-set restrictions.
#[tokio::test]
async fn test_operation_name_unicode() {
    let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;

    let result: Result<i32, String> = ctx
        .step("こんにちは世界", || async { Ok(1i32) })
        .await
        .unwrap();

    assert_eq!(
        result.unwrap(),
        1,
        "step with unicode name should return the closure result"
    );

    // Verify the operation was recorded with the full unicode name.
    assert_operations(&ops, &["step:こんにちは世界"]).await;
}

/// Verify that names exceeding 255 characters are accepted as a step operation name.
///
/// The SDK does not impose length limits on operation names — that constraint,
/// if any, belongs to the checkpoint API layer.
#[tokio::test]
async fn test_operation_name_long_255_plus_chars() {
    let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;

    let long_name = "a".repeat(300);

    let result: Result<i32, String> = ctx.step(&long_name, || async { Ok(1i32) }).await.unwrap();

    assert_eq!(
        result.unwrap(),
        1,
        "step with 300-char name should return the closure result"
    );

    // Verify the operation was recorded with the full long name.
    let recorded = ops.lock().await;
    assert_eq!(
        recorded.len(),
        1,
        "exactly one operation should be recorded"
    );
    assert_eq!(
        recorded[0].name, long_name,
        "operation name should be preserved in full (300 characters)"
    );
}

// ============================================================================
// TEST-16: Negative option values (integration-level confirmation)
// ============================================================================

/// Verify that `StepOptions::new().retries(-1)` panics with a descriptive message.
///
/// Integration-level confirmation that the Phase 4 builder validation is
/// enforced when assembling real option structs in test contexts.
#[test]
#[should_panic(expected = "StepOptions::retries: count must be >= 0, got -1")]
fn test_negative_retries_panics() {
    StepOptions::new().retries(-1);
}

/// Verify that `StepOptions::new().backoff_seconds(-1)` panics with a descriptive message.
///
/// Negative backoff is meaningless and would produce undefined server behavior.
/// The builder rejects it immediately.
#[test]
#[should_panic(expected = "StepOptions::backoff_seconds: seconds must be >= 0, got -1")]
fn test_negative_backoff_panics() {
    StepOptions::new().backoff_seconds(-1);
}

/// Verify that `CallbackOptions::new().timeout_seconds(0)` panics with a descriptive message.
///
/// Zero timeout would expire immediately — the server requires strictly
/// positive values. The builder enforces this constraint.
#[test]
#[should_panic(expected = "CallbackOptions::timeout_seconds: seconds must be > 0, got 0")]
fn test_zero_callback_timeout_panics() {
    CallbackOptions::new().timeout_seconds(0);
}

// ============================================================================
// TEST-17: 5-level nested child contexts
// ============================================================================

/// Verify that 5 levels of nested `child_context` calls all execute correctly
/// with no operation ID collisions at any nesting depth.
///
/// Each level runs a step and the innermost level's value (5) propagates
/// back up through the nesting chain. All 5 levels completing without panic
/// proves that the blake2b namespacing at each depth produces unique IDs.
#[tokio::test]
async fn test_five_level_nested_child_contexts() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;

    let result: i32 = ctx
        .child_context("level1", |mut l1| async move {
            let r1: Result<i32, String> = l1.step("l1_step", || async { Ok(1i32) }).await?;
            assert_eq!(r1.unwrap(), 1);

            let inner: i32 = l1
                .child_context("level2", |mut l2| async move {
                    let r2: Result<i32, String> = l2.step("l2_step", || async { Ok(2i32) }).await?;
                    assert_eq!(r2.unwrap(), 2);

                    let inner: i32 = l2
                        .child_context("level3", |mut l3| async move {
                            let r3: Result<i32, String> =
                                l3.step("l3_step", || async { Ok(3i32) }).await?;
                            assert_eq!(r3.unwrap(), 3);

                            let inner: i32 = l3
                                .child_context("level4", |mut l4| async move {
                                    let r4: Result<i32, String> =
                                        l4.step("l4_step", || async { Ok(4i32) }).await?;
                                    assert_eq!(r4.unwrap(), 4);

                                    let inner: i32 = l4
                                        .child_context("level5", |mut l5| async move {
                                            let r5: Result<i32, String> =
                                                l5.step("l5_step", || async { Ok(5i32) }).await?;
                                            Ok(r5.unwrap())
                                        })
                                        .await?;
                                    Ok(inner)
                                })
                                .await?;
                            Ok(inner)
                        })
                        .await?;
                    Ok(inner)
                })
                .await?;
            Ok(inner)
        })
        .await
        .unwrap();

    // The innermost level's value (5) should propagate all the way back.
    assert_eq!(result, 5, "deepest level value should propagate back up");

    // Verify checkpoints were produced — at minimum 10 Context checkpoints
    // (START + SUCCEED for each of the 5 child_context levels) plus step checkpoints.
    let captured = calls.lock().await;
    assert!(
        captured.len() >= 10,
        "expected at least 10 checkpoints from 5 levels of child_context, got {}",
        captured.len()
    );
}

// ============================================================================
// TEST-18: 3-level parallel-in-child-in-parallel
// ============================================================================

/// Verify that 3-level nesting (parallel > child_context > parallel) completes
/// correctly with no operation ID collisions across nesting levels or branches.
///
/// Structure:
/// - Level 1: outer `parallel` with 2 branches (branch_a, branch_b)
/// - Level 2: each branch creates a `child_context`
/// - Level 3: each child_context runs an inner `parallel` with 2 steps
///
/// Expected results (sorted): branch_a sums 10+20=30, branch_b sums 100+200=300.
#[tokio::test]
async fn test_parallel_in_child_in_parallel() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    type OuterBranch = Box<
        dyn FnOnce(
                DurableContext,
            ) -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>>
            + Send,
    >;

    let branches: Vec<OuterBranch> = vec![
        Box::new(|mut branch_ctx: DurableContext| {
            Box::pin(async move {
                // Level 2: child context inside parallel branch a
                let child_result: i32 = branch_ctx
                    .child_context("child_a", |mut child_ctx| async move {
                        // Level 3: inner parallel inside child context a
                        type InnerBranch = Box<
                            dyn FnOnce(
                                    DurableContext,
                                ) -> Pin<
                                    Box<dyn Future<Output = Result<i32, DurableError>> + Send>,
                                > + Send,
                        >;
                        let inner_branches: Vec<InnerBranch> = vec![
                            Box::new(|mut inner_ctx: DurableContext| {
                                Box::pin(async move {
                                    let r: Result<i32, String> =
                                        inner_ctx.step("inner_a1", || async { Ok(10i32) }).await?;
                                    Ok(r.unwrap())
                                })
                            }),
                            Box::new(|mut inner_ctx: DurableContext| {
                                Box::pin(async move {
                                    let r: Result<i32, String> =
                                        inner_ctx.step("inner_a2", || async { Ok(20i32) }).await?;
                                    Ok(r.unwrap())
                                })
                            }),
                        ];
                        let inner_result = child_ctx
                            .parallel("inner_parallel_a", inner_branches, ParallelOptions::new())
                            .await?;
                        let sum: i32 = inner_result
                            .results
                            .iter()
                            .filter_map(|item| item.result)
                            .sum();
                        Ok(sum)
                    })
                    .await?;
                Ok(child_result)
            })
        }),
        Box::new(|mut branch_ctx: DurableContext| {
            Box::pin(async move {
                // Level 2: child context inside parallel branch b
                let child_result: i32 = branch_ctx
                    .child_context("child_b", |mut child_ctx| async move {
                        // Level 3: inner parallel inside child context b
                        type InnerBranch = Box<
                            dyn FnOnce(
                                    DurableContext,
                                ) -> Pin<
                                    Box<dyn Future<Output = Result<i32, DurableError>> + Send>,
                                > + Send,
                        >;
                        let inner_branches: Vec<InnerBranch> = vec![
                            Box::new(|mut inner_ctx: DurableContext| {
                                Box::pin(async move {
                                    let r: Result<i32, String> =
                                        inner_ctx.step("inner_b1", || async { Ok(100i32) }).await?;
                                    Ok(r.unwrap())
                                })
                            }),
                            Box::new(|mut inner_ctx: DurableContext| {
                                Box::pin(async move {
                                    let r: Result<i32, String> =
                                        inner_ctx.step("inner_b2", || async { Ok(200i32) }).await?;
                                    Ok(r.unwrap())
                                })
                            }),
                        ];
                        let inner_result = child_ctx
                            .parallel("inner_parallel_b", inner_branches, ParallelOptions::new())
                            .await?;
                        let sum: i32 = inner_result
                            .results
                            .iter()
                            .filter_map(|item| item.result)
                            .sum();
                        Ok(sum)
                    })
                    .await?;
                Ok(child_result)
            })
        }),
    ];

    let result = ctx
        .parallel("outer_parallel", branches, ParallelOptions::new())
        .await
        .unwrap();

    // Outer parallel has 2 branches — both should succeed.
    assert_eq!(
        result.results.len(),
        2,
        "outer parallel should have 2 results"
    );
    for item in &result.results {
        assert_eq!(
            item.status,
            BatchItemStatus::Succeeded,
            "all outer branches should have Succeeded status"
        );
    }

    // Collect and sort results — tokio::spawn execution order is non-deterministic.
    let mut values: Vec<i32> = result
        .results
        .iter()
        .filter_map(|item| item.result)
        .collect();
    values.sort();
    // Branch a: 10 + 20 = 30, Branch b: 100 + 200 = 300
    assert_eq!(values, vec![30, 300], "branch sums should be [30, 300]");
}
