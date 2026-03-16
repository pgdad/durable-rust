//! End-to-end workflow tests for the durable-lambda SDK.
//!
//! These tests exercise realistic multi-operation durable workflows in
//! execute mode, covering all operation types (step, wait, callback,
//! invoke, parallel, map, child_context, logging) and their combinations.
//!
//! All tests use MockDurableContext — no AWS credentials needed.

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::types::{
    BatchItemStatus, CallbackOptions, CompletionReason, MapOptions, ParallelOptions, StepOptions,
};
use durable_lambda_testing::prelude::*;

// ========================================================================
// 1. Execute-mode workflows: verify steps run closures and checkpoint
// ========================================================================

#[tokio::test]
async fn execute_mode_step_runs_closure_and_checkpoints() {
    let (mut ctx, calls, ops) = MockDurableContext::new().build().await;

    let result: Result<i32, String> = ctx.step("compute", || async { Ok(42) }).await.unwrap();

    assert_eq!(result.unwrap(), 42);

    // Execute mode should produce checkpoints (START + SUCCEED)
    let captured = calls.lock().await;
    assert!(
        captured.len() >= 1,
        "execute mode should produce checkpoint calls"
    );

    // Operation should be recorded
    assert_operations(&ops, &["step:compute"]).await;
}

#[tokio::test]
async fn execute_mode_multi_step_workflow_checkpoints_each_step() {
    let (mut ctx, calls, ops) = MockDurableContext::new().build().await;

    let r1: Result<String, String> = ctx
        .step("validate", || async { Ok("valid".to_string()) })
        .await
        .unwrap();
    assert_eq!(r1.unwrap(), "valid");

    let r2: Result<i32, String> = ctx.step("charge", || async { Ok(100) }).await.unwrap();
    assert_eq!(r2.unwrap(), 100);

    let r3: Result<bool, String> = ctx.step("confirm", || async { Ok(true) }).await.unwrap();
    assert_eq!(r3.unwrap(), true);

    // All 3 steps should produce checkpoints
    let captured = calls.lock().await;
    assert!(
        captured.len() >= 3,
        "each step should produce checkpoint calls, got {}",
        captured.len()
    );

    assert_operations(&ops, &["step:validate", "step:charge", "step:confirm"]).await;
}

// ========================================================================
// 2. Step error handling: typed errors are checkpointed correctly
// ========================================================================

#[tokio::test]
async fn execute_mode_step_error_is_checkpointed() {
    let (mut ctx, calls, ops) = MockDurableContext::new().build().await;

    let result: Result<i32, String> = ctx
        .step("charge", || async {
            Err("insufficient_funds".to_string())
        })
        .await
        .unwrap();

    assert_eq!(result.unwrap_err(), "insufficient_funds");

    // Error should still produce checkpoints
    let captured = calls.lock().await;
    assert!(
        captured.len() >= 1,
        "step error should still checkpoint, got {}",
        captured.len()
    );

    assert_operations(&ops, &["step:charge"]).await;
}

#[tokio::test]
async fn replay_mode_step_error_replays_identically() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_error("charge", "PaymentError", r#""insufficient_funds""#)
        .build()
        .await;

    let result: Result<i32, String> = ctx
        .step("charge", || async { panic!("not executed") })
        .await
        .unwrap();

    assert_eq!(result.unwrap_err(), "insufficient_funds");
    assert_no_checkpoints(&calls).await;
}

// ========================================================================
// 3. Mixed replay/execute transitions
// ========================================================================

#[tokio::test]
async fn workflow_transitions_from_replay_to_execute_mid_stream() {
    // Pre-load only the first step — second step should execute
    let (mut ctx, calls, ops) = MockDurableContext::new()
        .with_step_result("step1", r#"10"#)
        .build()
        .await;

    assert!(ctx.is_replaying(), "should start in replay mode");

    // Step 1: replayed from history
    let r1: Result<i32, String> = ctx
        .step("step1", || async { panic!("should not execute") })
        .await
        .unwrap();
    assert_eq!(r1.unwrap(), 10);

    // After consuming all history, should transition to executing
    assert!(
        !ctx.is_replaying(),
        "should be in execute mode after history exhausted"
    );

    // Step 2: executes closure and checkpoints
    let r2: Result<i32, String> = ctx.step("step2", || async { Ok(20) }).await.unwrap();
    assert_eq!(r2.unwrap(), 20);

    // Only step2 should produce checkpoint calls
    let captured = calls.lock().await;
    assert!(
        !captured.is_empty(),
        "execute-mode step should produce checkpoints"
    );

    // Only step2 recorded (replay doesn't record)
    assert_operations(&ops, &["step:step2"]).await;
}

// ========================================================================
// 4. Complex multi-operation workflow (step + wait + callback + invoke)
// ========================================================================

#[tokio::test]
async fn full_workflow_replays_all_operation_types() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("validate_order", r#"{"order_id": 42, "total": 99.99}"#)
        .with_wait("rate_limit")
        .with_callback("manager_approval", "cb-mgr-1", r#""approved""#)
        .with_invoke("charge_payment", r#"{"tx_id": "tx-abc"}"#)
        .with_step_result("send_receipt", r#""email sent""#)
        .build()
        .await;

    // 1. Validate
    let order: Result<serde_json::Value, String> = ctx
        .step("validate_order", || async { panic!("replay") })
        .await
        .unwrap();
    assert_eq!(order.unwrap()["order_id"], 42);

    // 2. Rate limit wait
    ctx.wait("rate_limit", 5).await.unwrap();

    // 3. Manager approval callback
    let handle = ctx
        .create_callback("manager_approval", CallbackOptions::new())
        .await
        .unwrap();
    assert_eq!(handle.callback_id, "cb-mgr-1");
    let approval: String = ctx.callback_result(&handle).unwrap();
    assert_eq!(approval, "approved");

    // 4. Charge payment via invoke
    let payment: serde_json::Value = ctx
        .invoke(
            "charge_payment",
            "payment-service",
            &serde_json::json!({"amount": 99.99}),
        )
        .await
        .unwrap();
    assert_eq!(payment["tx_id"], "tx-abc");

    // 5. Send receipt
    let receipt: Result<String, String> = ctx
        .step("send_receipt", || async { panic!("replay") })
        .await
        .unwrap();
    assert_eq!(receipt.unwrap(), "email sent");

    // Pure replay — no checkpoints
    assert_no_checkpoints(&calls).await;
}

// ========================================================================
// 5. Parallel + steps in execute mode
// ========================================================================

#[tokio::test]
async fn parallel_with_steps_executes_and_returns_results() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    type BranchFn = Box<
        dyn FnOnce(
                DurableContext,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<i32, DurableError>> + Send>,
            > + Send,
    >;

    let branches: Vec<BranchFn> = vec![
        Box::new(|mut ctx| {
            Box::pin(async move {
                let r: Result<i32, String> = ctx.step("a", || async { Ok(10) }).await?;
                Ok(r.unwrap())
            })
        }),
        Box::new(|mut ctx| {
            Box::pin(async move {
                let r: Result<i32, String> = ctx.step("b", || async { Ok(20) }).await?;
                Ok(r.unwrap())
            })
        }),
        Box::new(|mut ctx| {
            Box::pin(async move {
                let r: Result<i32, String> = ctx.step("c", || async { Ok(30) }).await?;
                Ok(r.unwrap())
            })
        }),
    ];

    let result = ctx
        .parallel("fan_out", branches, ParallelOptions::new())
        .await
        .unwrap();

    assert_eq!(result.results.len(), 3);
    assert_eq!(result.completion_reason, CompletionReason::AllCompleted);

    // Verify all branches succeeded with correct values
    for item in &result.results {
        assert_eq!(item.status, BatchItemStatus::Succeeded);
    }
    assert_eq!(result.results[0].result, Some(10));
    assert_eq!(result.results[1].result, Some(20));
    assert_eq!(result.results[2].result, Some(30));
}

#[tokio::test]
async fn parallel_with_mixed_success_and_failure() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    type BranchFn = Box<
        dyn FnOnce(
                DurableContext,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<i32, DurableError>> + Send>,
            > + Send,
    >;

    let branches: Vec<BranchFn> = vec![
        Box::new(|_ctx| Box::pin(async move { Ok(42) })),
        Box::new(|_ctx| {
            Box::pin(async move {
                Err(DurableError::parallel_failed("branch", "branch 1 failed"))
            })
        }),
        Box::new(|_ctx| Box::pin(async move { Ok(99) })),
    ];

    let result = ctx
        .parallel("mixed", branches, ParallelOptions::new())
        .await
        .unwrap();

    assert_eq!(result.results.len(), 3);
    assert_eq!(result.results[0].status, BatchItemStatus::Succeeded);
    assert_eq!(result.results[0].result, Some(42));
    assert_eq!(result.results[1].status, BatchItemStatus::Failed);
    assert!(result.results[1].error.as_ref().unwrap().contains("failed"));
    assert_eq!(result.results[2].status, BatchItemStatus::Succeeded);
    assert_eq!(result.results[2].result, Some(99));
}

// ========================================================================
// 6. Map operation in execute mode
// ========================================================================

#[tokio::test]
async fn map_processes_all_items_and_returns_ordered_results() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let items = vec![1, 2, 3, 4, 5];
    let result = ctx
        .map(
            "double_all",
            items,
            MapOptions::new(),
            |item: i32, mut child_ctx: DurableContext| async move {
                let r: Result<i32, String> = child_ctx
                    .step("double", move || async move { Ok(item * 2) })
                    .await?;
                Ok(r.unwrap())
            },
        )
        .await
        .unwrap();

    assert_eq!(result.results.len(), 5);
    assert_eq!(result.completion_reason, CompletionReason::AllCompleted);

    for (i, item) in result.results.iter().enumerate() {
        assert_eq!(item.index, i);
        assert_eq!(item.status, BatchItemStatus::Succeeded);
        assert_eq!(item.result, Some(((i + 1) * 2) as i32));
    }
}

#[tokio::test]
async fn map_with_batch_size_processes_in_batches() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let items = vec![10, 20, 30, 40];
    let result = ctx
        .map(
            "batched",
            items,
            MapOptions::new().batch_size(2),
            |item: i32, _ctx: DurableContext| async move { Ok(item + 1) },
        )
        .await
        .unwrap();

    assert_eq!(result.results.len(), 4);
    assert_eq!(result.results[0].result, Some(11));
    assert_eq!(result.results[1].result, Some(21));
    assert_eq!(result.results[2].result, Some(31));
    assert_eq!(result.results[3].result, Some(41));
}

#[tokio::test]
async fn map_with_item_failure_captures_error() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let items = vec![1, 2, 3];
    let result = ctx
        .map(
            "partial_fail",
            items,
            MapOptions::new(),
            |item: i32, _ctx: DurableContext| async move {
                if item == 2 {
                    Err(DurableError::map_failed("item", "item 2 failed"))
                } else {
                    Ok(item * 10)
                }
            },
        )
        .await
        .unwrap();

    assert_eq!(result.results.len(), 3);
    assert_eq!(result.results[0].status, BatchItemStatus::Succeeded);
    assert_eq!(result.results[0].result, Some(10));
    assert_eq!(result.results[1].status, BatchItemStatus::Failed);
    assert!(result.results[1]
        .error
        .as_ref()
        .unwrap()
        .contains("item 2 failed"));
    assert_eq!(result.results[2].status, BatchItemStatus::Succeeded);
    assert_eq!(result.results[2].result, Some(30));
}

// ========================================================================
// 7. Child context in execute mode
// ========================================================================

#[tokio::test]
async fn child_context_executes_isolated_subflow() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    // Parent step
    let parent: Result<String, String> = ctx
        .step("setup", || async { Ok("ready".to_string()) })
        .await
        .unwrap();
    assert_eq!(parent.unwrap(), "ready");

    // Child context with its own steps
    let sub_result: i32 = ctx
        .child_context("payment_flow", |mut child_ctx: DurableContext| async move {
            let validate: Result<bool, String> =
                child_ctx.step("validate", || async { Ok(true) }).await?;
            assert!(validate.unwrap());

            let charge: Result<i32, String> =
                child_ctx.step("charge", || async { Ok(100) }).await?;
            Ok(charge.unwrap())
        })
        .await
        .unwrap();
    assert_eq!(sub_result, 100);

    // Parent step after child context
    let confirm: Result<bool, String> = ctx.step("confirm", || async { Ok(true) }).await.unwrap();
    assert_eq!(confirm.unwrap(), true);
}

#[tokio::test]
async fn nested_child_contexts() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let result: i32 = ctx
        .child_context(
            "outer",
            |mut outer_child: DurableContext| async move {
                let inner_result: i32 = outer_child
                    .child_context(
                        "inner",
                        |mut inner_child: DurableContext| async move {
                            let r: Result<i32, String> =
                                inner_child.step("deep", || async { Ok(7) }).await?;
                            Ok(r.unwrap())
                        },
                    )
                    .await?;
                Ok(inner_result * 6)
            },
        )
        .await
        .unwrap();

    assert_eq!(result, 42);
}

// ========================================================================
// 8. Logging operations (replay-safe)
// ========================================================================

#[tokio::test]
async fn logging_operations_do_not_affect_workflow() {
    let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;

    // Log various levels
    ctx.log("info message");
    ctx.log_debug("debug info");
    ctx.log_warn("warning");
    ctx.log_error("error occurred");
    ctx.log_with_data("structured", &serde_json::json!({"key": "value"}));
    ctx.log_debug_with_data("debug data", &serde_json::json!({"level": "debug"}));
    ctx.log_warn_with_data("warn data", &serde_json::json!({"level": "warn"}));
    ctx.log_error_with_data("error data", &serde_json::json!({"level": "error"}));

    // Logging should NOT produce durable operations
    let r: Result<i32, String> = ctx.step("after_log", || async { Ok(1) }).await.unwrap();
    assert_eq!(r.unwrap(), 1);

    // Only the step should be recorded, not the logs
    assert_operations(&ops, &["step:after_log"]).await;
}

// ========================================================================
// 9. Complex real-world workflow: order processing pipeline
// ========================================================================

#[tokio::test]
async fn e2e_order_processing_pipeline() {
    let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;

    // Step 1: Validate order
    let order: Result<serde_json::Value, String> = ctx
        .step("validate_order", || async {
            Ok(serde_json::json!({"id": "ORD-001", "items": 3, "total": 149.97}))
        })
        .await
        .unwrap();
    let order = order.unwrap();
    assert_eq!(order["id"], "ORD-001");

    // Step 2: Check inventory (parallel check for each item)
    type BranchFn = Box<
        dyn FnOnce(
                DurableContext,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<bool, DurableError>> + Send>,
            > + Send,
    >;

    let inventory_checks: Vec<BranchFn> = vec![
        Box::new(|mut ctx| {
            Box::pin(async move {
                let r: Result<bool, String> =
                    ctx.step("check_item_1", || async { Ok(true) }).await?;
                Ok(r.unwrap())
            })
        }),
        Box::new(|mut ctx| {
            Box::pin(async move {
                let r: Result<bool, String> =
                    ctx.step("check_item_2", || async { Ok(true) }).await?;
                Ok(r.unwrap())
            })
        }),
        Box::new(|mut ctx| {
            Box::pin(async move {
                let r: Result<bool, String> =
                    ctx.step("check_item_3", || async { Ok(true) }).await?;
                Ok(r.unwrap())
            })
        }),
    ];

    let inventory_result = ctx
        .parallel("check_inventory", inventory_checks, ParallelOptions::new())
        .await
        .unwrap();
    assert!(inventory_result
        .results
        .iter()
        .all(|r| r.result == Some(true)));

    // Step 3: Process payment in child context
    let payment: serde_json::Value = ctx
        .child_context(
            "payment",
            |mut child_ctx: DurableContext| async move {
                let charge: Result<serde_json::Value, String> = child_ctx
                    .step("charge_card", || async {
                        Ok(serde_json::json!({"tx_id": "TX-42", "status": "charged"}))
                    })
                    .await?;
                Ok(charge.unwrap())
            },
        )
        .await
        .unwrap();
    assert_eq!(payment["status"], "charged");

    // Step 4: Send confirmation
    let confirmation: Result<String, String> = ctx
        .step("send_confirmation", || async {
            Ok("confirmation sent".to_string())
        })
        .await
        .unwrap();
    assert_eq!(confirmation.unwrap(), "confirmation sent");

    // Verify the complete operation sequence
    let recorded = ops.lock().await;
    assert!(
        recorded.len() >= 3,
        "should have at least validate, confirm, and send steps recorded"
    );
    assert_eq!(recorded[0].to_type_name(), "step:validate_order");
}

// ========================================================================
// 10. Step with options (retries configuration)
// ========================================================================

#[tokio::test]
async fn step_with_options_retries_configuration() {
    let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;

    // Successful step with retry options configured
    let result: Result<i32, String> = ctx
        .step_with_options("resilient_op", StepOptions::new().retries(3).backoff_seconds(5), || async {
            Ok(42)
        })
        .await
        .unwrap();

    assert_eq!(result.unwrap(), 42);
    assert_operations(&ops, &["step:resilient_op"]).await;
}

// ========================================================================
// 11. Empty/edge-case workflows
// ========================================================================

#[tokio::test]
async fn empty_context_starts_in_execute_mode() {
    let (ctx, _calls, _ops) = MockDurableContext::new().build().await;

    assert!(!ctx.is_replaying());
    assert_eq!(
        ctx.execution_mode(),
        durable_lambda_core::types::ExecutionMode::Executing
    );
    assert!(!ctx.arn().is_empty());
    assert!(!ctx.checkpoint_token().is_empty());
}

#[tokio::test]
async fn single_step_workflow() {
    let (mut ctx, calls, ops) = MockDurableContext::new().build().await;

    let result: Result<String, String> = ctx
        .step("only_step", || async { Ok("done".to_string()) })
        .await
        .unwrap();

    assert_eq!(result.unwrap(), "done");

    let captured = calls.lock().await;
    assert!(
        !captured.is_empty(),
        "single step should produce checkpoints"
    );

    assert_operations(&ops, &["step:only_step"]).await;
}

#[tokio::test]
async fn map_with_empty_collection() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let items: Vec<i32> = vec![];
    let result = ctx
        .map(
            "empty_map",
            items,
            MapOptions::new(),
            |_item: i32, _ctx: DurableContext| async move { Ok(0) },
        )
        .await
        .unwrap();

    assert_eq!(result.results.len(), 0);
    assert_eq!(result.completion_reason, CompletionReason::AllCompleted);
}

#[tokio::test]
async fn map_with_single_item() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let result = ctx
        .map(
            "single_map",
            vec![42],
            MapOptions::new(),
            |item: i32, _ctx: DurableContext| async move { Ok(item * 2) },
        )
        .await
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].result, Some(84));
}

// ========================================================================
// 12. Sequential replay of all operation types
// ========================================================================

#[tokio::test]
async fn replay_step_wait_callback_invoke_in_sequence() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("s1", r#"1"#)
        .with_step_result("s2", r#"2"#)
        .with_wait("delay")
        .with_callback("approval", "cb-1", r#""yes""#)
        .with_invoke("remote_call", r#"{"result": "ok"}"#)
        .with_step_result("s3", r#"3"#)
        .build()
        .await;

    let r1: Result<i32, String> = ctx
        .step("s1", || async { panic!("replay") })
        .await
        .unwrap();
    assert_eq!(r1.unwrap(), 1);

    let r2: Result<i32, String> = ctx
        .step("s2", || async { panic!("replay") })
        .await
        .unwrap();
    assert_eq!(r2.unwrap(), 2);

    ctx.wait("delay", 10).await.unwrap();

    let handle = ctx
        .create_callback("approval", CallbackOptions::new())
        .await
        .unwrap();
    let cb_result: String = ctx.callback_result(&handle).unwrap();
    assert_eq!(cb_result, "yes");

    let invoke_result: serde_json::Value = ctx
        .invoke(
            "remote_call",
            "target-lambda",
            &serde_json::json!({"input": "data"}),
        )
        .await
        .unwrap();
    assert_eq!(invoke_result["result"], "ok");

    let r3: Result<i32, String> = ctx
        .step("s3", || async { panic!("replay") })
        .await
        .unwrap();
    assert_eq!(r3.unwrap(), 3);

    assert_no_checkpoints(&calls).await;
}

// ========================================================================
// 13. Callback with options
// ========================================================================

#[tokio::test]
async fn callback_with_timeout_options_replays() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_callback("timed_approval", "cb-timed-1", r#""approved""#)
        .build()
        .await;

    let handle = ctx
        .create_callback(
            "timed_approval",
            CallbackOptions::new()
                .timeout_seconds(300)
                .heartbeat_timeout_seconds(30),
        )
        .await
        .unwrap();

    assert_eq!(handle.callback_id, "cb-timed-1");
    let result: String = ctx.callback_result(&handle).unwrap();
    assert_eq!(result, "approved");

    assert_no_checkpoints(&calls).await;
}

// ========================================================================
// 14. Workflow with steps before and after parallel
// ========================================================================

#[tokio::test]
async fn steps_before_and_after_parallel() {
    let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;

    // Step before parallel
    let setup: Result<String, String> = ctx
        .step("setup", || async { Ok("initialized".to_string()) })
        .await
        .unwrap();
    assert_eq!(setup.unwrap(), "initialized");

    // Parallel block
    type BranchFn = Box<
        dyn FnOnce(
                DurableContext,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<i32, DurableError>> + Send>,
            > + Send,
    >;

    let branches: Vec<BranchFn> = vec![
        Box::new(|_ctx| Box::pin(async move { Ok(1) })),
        Box::new(|_ctx| Box::pin(async move { Ok(2) })),
    ];

    let par_result = ctx
        .parallel("work", branches, ParallelOptions::new())
        .await
        .unwrap();
    assert_eq!(par_result.results.len(), 2);

    // Step after parallel
    let cleanup: Result<String, String> = ctx
        .step("cleanup", || async { Ok("done".to_string()) })
        .await
        .unwrap();
    assert_eq!(cleanup.unwrap(), "done");

    // Verify the outer operations are recorded in order
    let recorded = ops.lock().await;
    assert!(recorded.len() >= 2);
    assert_eq!(recorded[0].to_type_name(), "step:setup");
    assert_eq!(recorded.last().unwrap().to_type_name(), "step:cleanup");
}

// ========================================================================
// 15. Complex data types through serialization
// ========================================================================

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Order {
    id: String,
    items: Vec<String>,
    total: f64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct PaymentResult {
    transaction_id: String,
    charged: bool,
}

#[tokio::test]
async fn complex_types_serialize_through_steps() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let order: Result<Order, String> = ctx
        .step("create_order", || async {
            Ok(Order {
                id: "ORD-42".to_string(),
                items: vec!["widget".to_string(), "gadget".to_string()],
                total: 29.99,
            })
        })
        .await
        .unwrap();

    let order = order.unwrap();
    assert_eq!(order.id, "ORD-42");
    assert_eq!(order.items.len(), 2);
    assert_eq!(order.total, 29.99);

    let payment: Result<PaymentResult, String> = ctx
        .step("process_payment", || async {
            Ok(PaymentResult {
                transaction_id: "TX-123".to_string(),
                charged: true,
            })
        })
        .await
        .unwrap();

    let payment = payment.unwrap();
    assert_eq!(payment.transaction_id, "TX-123");
    assert!(payment.charged);
}

#[tokio::test]
async fn complex_types_replay_from_history() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result(
            "create_order",
            r#"{"id":"ORD-42","items":["widget","gadget"],"total":29.99}"#,
        )
        .build()
        .await;

    let order: Result<Order, String> = ctx
        .step("create_order", || async { panic!("replay") })
        .await
        .unwrap();

    let order = order.unwrap();
    assert_eq!(order.id, "ORD-42");
    assert_eq!(order.items, vec!["widget", "gadget"]);
    assert_eq!(order.total, 29.99);

    assert_no_checkpoints(&calls).await;
}

// ========================================================================
// 16. Assertion helpers validation
// ========================================================================

#[tokio::test]
async fn assert_operation_count_works() {
    let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;

    let _: Result<i32, String> = ctx.step("a", || async { Ok(1) }).await.unwrap();
    let _: Result<i32, String> = ctx.step("b", || async { Ok(2) }).await.unwrap();
    let _: Result<i32, String> = ctx.step("c", || async { Ok(3) }).await.unwrap();

    assert_operation_count(&ops, 3).await;
    assert_operation_names(&ops, &["a", "b", "c"]).await;
    assert_operations(&ops, &["step:a", "step:b", "step:c"]).await;
}

// ========================================================================
// 17. Map inside child context
// ========================================================================

#[tokio::test]
async fn map_inside_child_context() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let total: i32 = ctx
        .child_context(
            "batch_processor",
            |mut child_ctx: DurableContext| async move {
                let result = child_ctx
                    .map(
                        "process_batch",
                        vec![1, 2, 3],
                        MapOptions::new(),
                        |item: i32, _ctx: DurableContext| async move { Ok(item * 10) },
                    )
                    .await?;

                let sum: i32 = result
                    .results
                    .iter()
                    .filter_map(|r| r.result)
                    .sum();
                Ok(sum)
            },
        )
        .await
        .unwrap();

    assert_eq!(total, 60); // 10 + 20 + 30
}

// ========================================================================
// 18. Parallel inside child context
// ========================================================================

#[tokio::test]
async fn parallel_inside_child_context() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let result: Vec<i32> = ctx
        .child_context(
            "parallel_sub",
            |mut child_ctx: DurableContext| async move {
                type BranchFn = Box<
                    dyn FnOnce(
                            DurableContext,
                        )
                            -> std::pin::Pin<
                            Box<
                                dyn std::future::Future<Output = Result<i32, DurableError>>
                                    + Send,
                            >,
                        > + Send,
                >;

                let branches: Vec<BranchFn> = vec![
                    Box::new(|_ctx| Box::pin(async move { Ok(10) })),
                    Box::new(|_ctx| Box::pin(async move { Ok(20) })),
                ];

                let par_result = child_ctx
                    .parallel("inner_parallel", branches, ParallelOptions::new())
                    .await?;

                let values: Vec<i32> =
                    par_result.results.into_iter().filter_map(|r| r.result).collect();
                Ok(values)
            },
        )
        .await
        .unwrap();

    assert_eq!(result, vec![10, 20]);
}
