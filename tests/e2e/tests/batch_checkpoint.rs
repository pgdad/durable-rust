//! Batch checkpoint tests for the durable-lambda SDK.
//!
//! Tests that verify batch mode accumulates checkpoint updates and
//! sends them in a single AWS API call, reducing API call count vs
//! individual mode.

use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_batch_mode_defers_checkpoints() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;
    ctx.enable_batch_mode();
    let _: Result<i32, String> = ctx.step("s1", || async { Ok(1) }).await.unwrap();
    assert_eq!(
        calls.lock().await.len(),
        0,
        "batch mode should defer checkpoint calls"
    );
    assert_eq!(ctx.pending_update_count(), 2, "START + SUCCEED deferred");
}

#[tokio::test]
async fn test_flush_batch_sends_accumulated_updates() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;
    ctx.enable_batch_mode();
    let _: Result<i32, String> = ctx.step("s1", || async { Ok(1) }).await.unwrap();
    let _: Result<i32, String> = ctx.step("s2", || async { Ok(2) }).await.unwrap();
    assert_eq!(ctx.pending_update_count(), 4);
    ctx.flush_batch().await.unwrap();
    assert_eq!(calls.lock().await.len(), 1, "single batch call");
    assert_eq!(
        calls.lock().await[0].updates.len(),
        4,
        "4 updates in one call"
    );
    assert_eq!(ctx.pending_update_count(), 0, "cleared after flush");
}

#[tokio::test]
async fn test_batch_reduces_checkpoint_count() {
    // Individual mode: 5 steps = 10 checkpoint calls (START + SUCCEED each)
    let (mut ctx_individual, calls_ind, _) = MockDurableContext::new().build().await;
    for i in 0..5 {
        let _: Result<i32, String> = ctx_individual
            .step(&format!("s{i}"), move || async move { Ok(i) })
            .await
            .unwrap();
    }
    let individual_count = calls_ind.lock().await.len();
    assert_eq!(individual_count, 10);

    // Batch mode: 5 steps + flush = 1 batch call
    let (mut ctx_batch, calls_batch, _) = MockDurableContext::new().build().await;
    ctx_batch.enable_batch_mode();
    for i in 0..5 {
        let _: Result<i32, String> = ctx_batch
            .step(&format!("s{i}"), move || async move { Ok(i) })
            .await
            .unwrap();
    }
    ctx_batch.flush_batch().await.unwrap();
    let batch_count = calls_batch.lock().await.len();
    assert_eq!(batch_count, 1, "batch mode: single call");
    assert!(batch_count < individual_count, "batch < individual calls");
}

#[tokio::test]
async fn test_individual_mode_still_works() {
    let (mut ctx, calls, _) = MockDurableContext::new().build().await;
    // No enable_batch_mode() — default individual mode
    let _: Result<i32, String> = ctx.step("s1", || async { Ok(1) }).await.unwrap();
    assert_eq!(calls.lock().await.len(), 2, "individual: START + SUCCEED");
}

#[tokio::test]
async fn test_flush_batch_noop_when_empty() {
    let (mut ctx, calls, _) = MockDurableContext::new().build().await;
    ctx.enable_batch_mode();
    ctx.flush_batch().await.unwrap(); // no-op
    assert_eq!(calls.lock().await.len(), 0);
}
