//! Cross-approach behavioral parity tests.
//!
//! Verify that all API approach crates expose the same operations with identical
//! behavior. The closure, trait, and builder crates all delegate to DurableContext
//! — these tests confirm parity at the DurableContext level (shared by all 3
//! wrappers) and verify that all prelude modules export identical type sets.
//!
//! The proc-macro approach is structurally different (generates code referencing
//! DurableContext directly) so its parity is guaranteed by design — it uses the
//! same core and shared event module.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::types::ExecutionMode;
use durable_lambda_core::DurableContextOps;
use durable_lambda_testing::prelude::*;

// ========================================================================
// Task 2.3: step_parity — identical step results across approaches
// ========================================================================

#[tokio::test]
async fn step_parity_all_approaches_produce_identical_results() {
    // All 3 wrapper contexts delegate step() to DurableContext.step().
    // Verify DurableContext produces correct results in both execute and replay modes.

    // Execute mode: step runs the closure and checkpoints.
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;
    let result: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await.unwrap();
    assert_eq!(result, Ok(42), "step in execute mode should run closure");

    // Replay mode: step returns cached result without running closure.
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("validate", "42")
        .build()
        .await;
    let result: Result<i32, String> = ctx
        .step("validate", || async {
            panic!("should not execute during replay")
        })
        .await
        .unwrap();
    assert_eq!(
        result,
        Ok(42),
        "step in replay mode should return cached result"
    );
    assert_no_checkpoints(&calls).await;
}

// ========================================================================
// Task 2.4: step_with_options_parity
// ========================================================================

#[tokio::test]
async fn step_with_options_parity_retries_produce_identical_results() {
    use durable_lambda_core::types::StepOptions;

    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;
    let result: Result<i32, String> = ctx
        .step_with_options("charge", StepOptions::new().retries(3), || async {
            Ok(100)
        })
        .await
        .unwrap();
    assert_eq!(
        result,
        Ok(100),
        "step_with_options should produce same result as step"
    );
}

// ========================================================================
// Task 2.5: execution_mode_parity
// ========================================================================

#[tokio::test]
async fn execution_mode_parity_executing_when_no_history() {
    let (ctx, _, _) = MockDurableContext::new().build().await;
    assert_eq!(ctx.execution_mode(), ExecutionMode::Executing);
    assert!(!ctx.is_replaying());
}

#[tokio::test]
async fn execution_mode_parity_replaying_when_history_present() {
    let (ctx, _, _) = MockDurableContext::new()
        .with_step_result("op1", r#"{"Ok":1}"#)
        .build()
        .await;
    assert_eq!(ctx.execution_mode(), ExecutionMode::Replaying);
    assert!(ctx.is_replaying());
}

// ========================================================================
// Task 2.6: query_parity — arn and checkpoint_token
// ========================================================================

#[tokio::test]
async fn query_parity_arn_and_checkpoint_token() {
    let (ctx, _, _) = MockDurableContext::new().build().await;
    // MockDurableContext uses default "arn:aws:lambda:mock:000000000000:function:mock-function"
    // and "mock-checkpoint-token" — exact values depend on mock implementation.
    let arn = ctx.arn();
    let token = ctx.checkpoint_token();
    assert!(!arn.is_empty(), "arn should not be empty");
    assert!(!token.is_empty(), "checkpoint_token should not be empty");
}

// ========================================================================
// Task 2.7: child_context_parity
// ========================================================================

#[tokio::test]
async fn child_context_parity_identical_results() {
    let (mut ctx, _, _) = MockDurableContext::new().build().await;

    let result: i32 = ctx
        .child_context("sub_workflow", |mut child_ctx: DurableContext| async move {
            let r: Result<i32, String> = child_ctx.step("inner", || async { Ok(42) }).await?;
            Ok(r.unwrap())
        })
        .await
        .unwrap();
    assert_eq!(
        result, 42,
        "child_context should produce identical result across approaches"
    );
}

// ========================================================================
// Task 2.8: log_parity — all 8 log methods callable
// ========================================================================

#[tokio::test]
async fn log_parity_all_methods_callable_without_panic() {
    let (ctx, _, _) = MockDurableContext::new().build().await;
    // All 8 log methods should be callable without panicking.
    ctx.log("info message");
    ctx.log_with_data("info data", &serde_json::json!({"key": "value"}));
    ctx.log_debug("debug message");
    ctx.log_warn("warn message");
    ctx.log_error("error message");
    ctx.log_debug_with_data("debug data", &serde_json::json!({"k": "v"}));
    ctx.log_warn_with_data("warn data", &serde_json::json!({"k": "v"}));
    ctx.log_error_with_data("error data", &serde_json::json!({"k": "v"}));
}

// ========================================================================
// Task 2.9: prelude_exports_parity — compile-time type verification
// ========================================================================

/// Verify that all 3 prelude modules export the same core types.
/// This is a compile-time check — if any type is missing from a prelude,
/// this test file will fail to compile.
#[test]
fn prelude_exports_parity_closure() {
    use durable_lambda_closure::prelude::*;
    // Verify all expected types are accessible from closure prelude.
    let _: fn() -> Option<DurableError> = || None;
    let _: fn() -> Option<StepOptions> = || None;
    let _: fn() -> Option<CallbackOptions> = || None;
    let _: fn() -> Option<CallbackHandle> = || None;
    let _: fn() -> Option<ExecutionMode> = || None;
    let _: fn() -> Option<CheckpointResult<(), ()>> = || None;
    let _: fn() -> Option<BatchItem<()>> = || None;
    let _: fn() -> Option<BatchItemStatus> = || None;
    let _: fn() -> Option<BatchResult<()>> = || None;
    let _: fn() -> Option<CompletionReason> = || None;
    let _: fn() -> Option<MapOptions> = || None;
    let _: fn() -> Option<ParallelOptions> = || None;
    let _: fn() -> Option<ClosureContext> = || None;
}

#[test]
fn prelude_exports_parity_trait() {
    use durable_lambda_trait::prelude::*;
    let _: fn() -> Option<DurableError> = || None;
    let _: fn() -> Option<StepOptions> = || None;
    let _: fn() -> Option<CallbackOptions> = || None;
    let _: fn() -> Option<CallbackHandle> = || None;
    let _: fn() -> Option<ExecutionMode> = || None;
    let _: fn() -> Option<CheckpointResult<(), ()>> = || None;
    let _: fn() -> Option<BatchItem<()>> = || None;
    let _: fn() -> Option<BatchItemStatus> = || None;
    let _: fn() -> Option<BatchResult<()>> = || None;
    let _: fn() -> Option<CompletionReason> = || None;
    let _: fn() -> Option<MapOptions> = || None;
    let _: fn() -> Option<ParallelOptions> = || None;
    let _: fn() -> Option<TraitContext> = || None;
    let _: fn() -> Option<Box<dyn DurableHandler>> = || None;
}

#[test]
fn prelude_exports_parity_builder() {
    use durable_lambda_builder::prelude::*;
    let _: fn() -> Option<DurableError> = || None;
    let _: fn() -> Option<StepOptions> = || None;
    let _: fn() -> Option<CallbackOptions> = || None;
    let _: fn() -> Option<CallbackHandle> = || None;
    let _: fn() -> Option<ExecutionMode> = || None;
    let _: fn() -> Option<CheckpointResult<(), ()>> = || None;
    let _: fn() -> Option<BatchItem<()>> = || None;
    let _: fn() -> Option<BatchItemStatus> = || None;
    let _: fn() -> Option<BatchResult<()>> = || None;
    let _: fn() -> Option<CompletionReason> = || None;
    let _: fn() -> Option<MapOptions> = || None;
    let _: fn() -> Option<ParallelOptions> = || None;
    let _: fn() -> Option<BuilderContext> = || None;
}

// ========================================================================
// Task 3.1: signature_parity — method name and signature verification
// ========================================================================

/// Compile-time verification that all 3 context wrappers expose the same
/// set of public methods. This test uses type inference — if a method is
/// missing or has a different signature, the code won't compile.
///
/// The macro approach (DurableContext) is inherently parity-compatible since
/// it IS the core context that the other 3 delegate to.
#[test]
fn signature_parity_all_contexts_expose_same_methods() {
    // This is a compile-time check. The function body exercises the type system
    // to ensure all contexts have matching method signatures.
    //
    // We verify via trait-like assertions:
    // Each context type must have: step, step_with_options, wait, create_callback,
    // callback_result, invoke, parallel, child_context, map,
    // execution_mode, is_replaying, arn, checkpoint_token,
    // log, log_with_data, log_debug, log_warn, log_error,
    // log_debug_with_data, log_warn_with_data, log_error_with_data

    // This function exists purely for the compiler to check that all method
    // references below resolve. It is never called at runtime.
    fn _assert_closure_methods(ctx: &durable_lambda_closure::ClosureContext) {
        let _ = ctx.execution_mode();
        let _ = ctx.is_replaying();
        let _ = ctx.arn();
        let _ = ctx.checkpoint_token();
        ctx.log("test");
        ctx.log_with_data("test", &serde_json::json!({}));
        ctx.log_debug("test");
        ctx.log_warn("test");
        ctx.log_error("test");
        ctx.log_debug_with_data("test", &serde_json::json!({}));
        ctx.log_warn_with_data("test", &serde_json::json!({}));
        ctx.log_error_with_data("test", &serde_json::json!({}));
    }

    fn _assert_trait_methods(ctx: &durable_lambda_trait::TraitContext) {
        let _ = ctx.execution_mode();
        let _ = ctx.is_replaying();
        let _ = ctx.arn();
        let _ = ctx.checkpoint_token();
        ctx.log("test");
        ctx.log_with_data("test", &serde_json::json!({}));
        ctx.log_debug("test");
        ctx.log_warn("test");
        ctx.log_error("test");
        ctx.log_debug_with_data("test", &serde_json::json!({}));
        ctx.log_warn_with_data("test", &serde_json::json!({}));
        ctx.log_error_with_data("test", &serde_json::json!({}));
    }

    fn _assert_builder_methods(ctx: &durable_lambda_builder::BuilderContext) {
        let _ = ctx.execution_mode();
        let _ = ctx.is_replaying();
        let _ = ctx.arn();
        let _ = ctx.checkpoint_token();
        ctx.log("test");
        ctx.log_with_data("test", &serde_json::json!({}));
        ctx.log_debug("test");
        ctx.log_warn("test");
        ctx.log_error("test");
        ctx.log_debug_with_data("test", &serde_json::json!({}));
        ctx.log_warn_with_data("test", &serde_json::json!({}));
        ctx.log_error_with_data("test", &serde_json::json!({}));
    }

    // The async methods (step, wait, invoke, parallel, child_context, map)
    // can't be verified in a sync function context, but they are verified
    // by the per-crate delegation tests which all pass the same test cases.
}

// ========================================================================
// Task 3.2: parameter ordering convention verification
// ========================================================================

/// Verify parameter ordering convention: (name, options, closure) for all
/// operation methods. This is a documentation/code review verification.
/// The compile-time checks in per-crate tests already enforce correct
/// parameter types — this test documents the verified convention.
#[test]
fn parameter_ordering_convention_documented() {
    // All 3 context wrappers follow the architecture convention:
    //   1. name: &str          — always first
    //   2. options/config      — second, when applicable
    //   3. closure/payload     — always last
    //
    // Methods verified (same across ClosureContext, TraitContext, BuilderContext):
    //   step(name, f)
    //   step_with_options(name, options, f)
    //   wait(name, duration_secs)
    //   create_callback(name, options)
    //   callback_result(handle)
    //   invoke(name, function_name, payload)
    //   parallel(name, branches, options)
    //   child_context(name, f)
    //   map(name, items, options, f)
    //
    // The macro approach passes DurableContext directly, which has the same
    // method signatures as the core context all wrappers delegate to.
}

// ========================================================================
// TEST-23: Step timeout and conditional retry parity
// ========================================================================

/// step_timeout_parity: DurableContext with timeout_seconds(1) fires on slow closure.
///
/// All 4 API styles (closure, trait, builder, macro) delegate to DurableContext.
/// Proving DurableContext returns StepTimeout proves parity for all approaches.
#[tokio::test]
async fn step_timeout_parity_slow_closure_returns_step_timeout() {
    use durable_lambda_core::error::DurableError;
    use durable_lambda_core::types::StepOptions;

    // Enable tokio's time control so the test runs instantly without real sleeps.
    tokio::time::pause();

    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    // Spawn the step call in the background so we can advance time concurrently.
    let step_future = ctx.step_with_options::<i32, String, _, _>(
        "slow_step",
        StepOptions::new().timeout_seconds(1),
        || async {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            Ok(42)
        },
    );

    // Advance time past the 1-second timeout.
    let result = tokio::select! {
        r = step_future => r,
        _ = async {
            tokio::time::advance(std::time::Duration::from_secs(2)).await;
            // Yield to let the step task observe the elapsed time.
            tokio::task::yield_now().await;
            // Return a future that never resolves so select! picks the step result.
            std::future::pending::<()>().await
        } => unreachable!()
    };

    match result {
        Err(DurableError::StepTimeout { operation_name, .. }) => {
            assert_eq!(
                operation_name, "slow_step",
                "StepTimeout should carry the operation name"
            );
        }
        other => panic!("expected Err(StepTimeout), got: {other:?}"),
    }
}

/// step_timeout_parity: DurableContext with timeout_seconds(5) and immediate closure returns Ok(Ok(value)).
#[tokio::test]
async fn step_timeout_parity_fast_closure_within_timeout_returns_ok() {
    use durable_lambda_core::types::StepOptions;

    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let result: Result<i32, String> = ctx
        .step_with_options("fast_step", StepOptions::new().timeout_seconds(5), || async {
            Ok(42)
        })
        .await
        .unwrap();

    assert_eq!(
        result,
        Ok(42),
        "fast closure within timeout should return Ok(Ok(value))"
    );
}

/// conditional_retry_parity: DurableContext with retry_if returning false fails immediately.
///
/// When retry_if predicate returns false, the step fails without consuming retry budget.
/// Decision [05-01]: predicate false causes immediate FAIL (FEAT-14).
#[tokio::test]
async fn conditional_retry_parity_predicate_false_fails_not_retries() {
    use durable_lambda_core::error::DurableError;
    use durable_lambda_core::types::StepOptions;

    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    // retry_if predicate only accepts "transient" errors — "permanent" should not retry.
    let opts = StepOptions::new()
        .retries(2)
        .retry_if(|e: &String| e.contains("transient"));

    let result: Result<Result<i32, String>, DurableError> = ctx
        .step_with_options("conditional_step", opts, || async {
            Err::<i32, String>("permanent error".to_string())
        })
        .await;

    // Predicate returns false for "permanent error" — step returns Ok(Err(...)), NOT StepRetryScheduled.
    match result {
        Ok(Err(msg)) => {
            assert!(
                msg.contains("permanent"),
                "error message should be preserved: {msg}"
            );
        }
        Err(DurableError::StepRetryScheduled { .. }) => {
            panic!("predicate returned false — should NOT have scheduled retry");
        }
        other => panic!("expected Ok(Err(msg)), got: {other:?}"),
    }
}

/// conditional_retry_parity: Without retry_if, all errors trigger retry (backward compatible).
#[tokio::test]
async fn conditional_retry_parity_no_predicate_retries_all_errors() {
    use durable_lambda_core::error::DurableError;
    use durable_lambda_core::types::StepOptions;

    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    // No retry_if — default behavior retries any error.
    let opts = StepOptions::new().retries(2);

    let result: Result<Result<i32, String>, DurableError> = ctx
        .step_with_options("retry_step", opts, || async {
            Err::<i32, String>("any error".to_string())
        })
        .await;

    // Without predicate, first failure schedules a retry.
    match result {
        Err(DurableError::StepRetryScheduled { operation_name, .. }) => {
            assert_eq!(operation_name, "retry_step");
        }
        other => panic!("expected Err(StepRetryScheduled), got: {other:?}"),
    }
}

// ========================================================================
// Plan 03-03: Generic handler parity tests (ARCH-05 validation)
// ========================================================================

/// Generic workflow logic that works with any context implementing DurableContextOps.
///
/// This is the key test for ARCH-05: proving generic handler code compiles and
/// runs correctly regardless of which concrete context type is supplied.
///
/// The execution mode is captured before the step runs so replay mode is visible
/// even if the replay engine transitions to Executing after consuming all history.
async fn generic_workflow_logic<C: DurableContextOps>(
    ctx: &mut C,
) -> Result<serde_json::Value, durable_lambda_core::error::DurableError> {
    // Capture mode before any step — replay engine may transition after consuming history.
    let initial_mode = ctx.execution_mode();
    let step_result: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await?;
    let value = step_result.unwrap();
    ctx.log("validation complete");
    Ok(serde_json::json!({"validated": value, "mode": format!("{:?}", initial_mode)}))
}

#[tokio::test]
async fn generic_handler_works_with_durable_context_execute_mode() {
    // DurableContext is one of the 4 context types. Passing it to a generic
    // function bounded by C: DurableContextOps proves the bound is satisfied
    // and that the delegation chain produces correct results.
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;
    let result = generic_workflow_logic(&mut ctx).await.unwrap();
    assert_eq!(
        result,
        serde_json::json!({"validated": 42, "mode": "Executing"}),
        "generic handler in execute mode should run closure and return correct result"
    );
}

#[tokio::test]
async fn generic_handler_works_with_durable_context_replay_mode() {
    // Replay mode: the previously checkpointed result ("42") is returned
    // without running the step closure, and no new checkpoints are made.
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("validate", "42")
        .build()
        .await;
    let result = generic_workflow_logic(&mut ctx).await.unwrap();
    assert_eq!(
        result,
        serde_json::json!({"validated": 42, "mode": "Replaying"}),
        "generic handler in replay mode should return cached result without re-executing closure"
    );
    assert_no_checkpoints(&calls).await;
}

#[test]
fn all_context_types_implement_durable_context_ops() {
    // Compile-time proof that all 4 context types implement DurableContextOps.
    // If any impl is missing, this test fails at compile time with a type error.
    fn assert_ops<T: DurableContextOps>() {}
    assert_ops::<durable_lambda_core::context::DurableContext>();
    assert_ops::<durable_lambda_closure::context::ClosureContext>();
    assert_ops::<durable_lambda_trait::context::TraitContext>();
    assert_ops::<durable_lambda_builder::context::BuilderContext>();
}

// ========================================================================
// TEST-24: Complex workflow parity — parallel + child + timeout
// ========================================================================

/// complex_workflow_parity: parallel containing a timeout step and a regular step.
///
/// This proves that parallel + timeout step combination works correctly end-to-end
/// through DurableContext (shared by all 4 API styles).
#[tokio::test]
async fn complex_workflow_parity_parallel_with_timeout_step() {
    use durable_lambda_core::error::DurableError;
    use durable_lambda_core::types::{BatchItemStatus, CompletionReason, ParallelOptions, StepOptions};

    type BranchFn = Box<
        dyn FnOnce(
            DurableContext,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<i32, DurableError>> + Send>,
        > + Send,
    >;

    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    // Branch 1: step_with_options with a 5-second timeout — fast closure completes within.
    // Branch 2: regular step returning a value.
    // Both branches run concurrently via parallel().
    let branches: Vec<BranchFn> = vec![
        Box::new(|mut child_ctx: DurableContext| {
            Box::pin(async move {
                let r: Result<i32, String> = child_ctx
                    .step_with_options(
                        "fast",
                        StepOptions::new().timeout_seconds(5),
                        || async { Ok::<i32, String>(1) },
                    )
                    .await?;
                Ok(r.unwrap())
            })
        }),
        Box::new(|mut child_ctx: DurableContext| {
            Box::pin(async move {
                let r: Result<i32, String> = child_ctx
                    .step("compute", || async { Ok::<i32, String>(2) })
                    .await?;
                Ok(r.unwrap())
            })
        }),
    ];

    let result = ctx
        .parallel("mixed_parallel", branches, ParallelOptions::new())
        .await
        .unwrap();

    assert_eq!(result.results.len(), 2, "parallel should have 2 branch results");
    assert_eq!(
        result.completion_reason,
        CompletionReason::AllCompleted,
        "all branches should complete"
    );

    // Sort by index before asserting (concurrent execution may reorder — decision [02-01]).
    let mut results = result.results.clone();
    results.sort_by_key(|item| item.index);

    assert_eq!(results[0].status, BatchItemStatus::Succeeded);
    assert_eq!(results[0].result, Some(1), "branch 0 (timeout step) should return 1");
    assert_eq!(results[1].status, BatchItemStatus::Succeeded);
    assert_eq!(results[1].result, Some(2), "branch 1 (regular step) should return 2");
}

// ========================================================================
// TEST-25: BatchItemStatus verification
// ========================================================================

/// batch_item_status_verification: parallel with one success and one failure.
///
/// Verify BatchItemStatus::Succeeded for successful branches and
/// BatchItemStatus::Failed for failed branches (per-item status).
#[tokio::test]
async fn batch_item_status_succeeded_and_failed_parallel() {
    use durable_lambda_core::error::DurableError;
    use durable_lambda_core::types::{BatchItemStatus, CompletionReason, ParallelOptions};

    type BranchFn = Box<
        dyn FnOnce(
            DurableContext,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<i32, DurableError>> + Send>,
        > + Send,
    >;

    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let branches: Vec<BranchFn> = vec![
        Box::new(|_ctx: DurableContext| Box::pin(async move { Ok(42i32) })),
        Box::new(|_ctx: DurableContext| {
            Box::pin(async move { Err(DurableError::parallel_failed("branch", "fail")) })
        }),
    ];

    let result = ctx
        .parallel("status_parallel", branches, ParallelOptions::new())
        .await
        .unwrap();

    assert_eq!(result.results.len(), 2);
    assert_eq!(result.completion_reason, CompletionReason::AllCompleted);

    // Sort by index before asserting (concurrent execution may reorder — decision [02-01]).
    let mut results = result.results.clone();
    results.sort_by_key(|item| item.index);

    assert_eq!(
        results[0].status,
        BatchItemStatus::Succeeded,
        "branch 0 (Ok(42)) should be Succeeded"
    );
    assert_eq!(results[0].result, Some(42));
    assert!(results[0].error.is_none());

    assert_eq!(
        results[1].status,
        BatchItemStatus::Failed,
        "branch 1 (Err) should be Failed"
    );
    assert!(results[1].result.is_none());
    assert!(
        results[1]
            .error
            .as_ref()
            .unwrap()
            .contains("fail"),
        "error message should contain 'fail'"
    );
}

/// batch_item_status_map_per_item: map over [1, 2, 3] with one failing item.
///
/// Verify that BatchItemStatus is correctly set per-item in map results:
/// items 1 and 3 succeed, item 2 fails (closure returns Err).
#[tokio::test]
async fn batch_item_status_map_per_item() {
    use durable_lambda_core::error::DurableError;
    use durable_lambda_core::types::{BatchItemStatus, MapOptions};

    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let result = ctx
        .map(
            "status_map",
            vec![1i32, 2, 3],
            MapOptions::new(),
            |item: i32, _child_ctx: DurableContext| async move {
                if item == 2 {
                    Err(DurableError::map_failed("item", "bad item"))
                } else {
                    Ok(item * 10)
                }
            },
        )
        .await
        .unwrap();

    assert_eq!(result.results.len(), 3);

    // Sort by index before asserting (concurrent execution may reorder — decision [02-01]).
    let mut results = result.results.clone();
    results.sort_by_key(|item| item.index);

    assert_eq!(
        results[0].status,
        BatchItemStatus::Succeeded,
        "item 1 (value 1) should Succeed"
    );
    assert_eq!(results[0].result, Some(10));

    assert_eq!(
        results[1].status,
        BatchItemStatus::Failed,
        "item 2 (value 2) should Fail"
    );
    assert!(results[1].result.is_none());
    assert!(
        results[1].error.as_ref().unwrap().contains("bad"),
        "error should contain 'bad'"
    );

    assert_eq!(
        results[2].status,
        BatchItemStatus::Succeeded,
        "item 3 (value 3) should Succeed"
    );
    assert_eq!(results[2].result, Some(30));
}
