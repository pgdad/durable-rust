//! Map operation — parallel collection processing.
//!
//! Implement FR23-FR25: process items in parallel, batching configuration,
//! BatchResult<T> return type.
//!
//! The map operation uses `OperationType::Context` on the wire with
//! sub_type "Map" for the outer block and "MapItem" for each item.
//! Each item gets its own child `DurableContext` with an isolated
//! operation ID namespace.

use std::future::Future;

use aws_sdk_lambda::types::{OperationAction, OperationStatus, OperationType, OperationUpdate};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::context::DurableContext;
use crate::error::DurableError;
use crate::types::{BatchItem, BatchItemStatus, BatchResult, CompletionReason, MapOptions};

impl DurableContext {
    /// Process a collection of items in parallel and return their results.
    ///
    /// Apply the closure `f` to each item concurrently. Each item receives an
    /// owned child [`DurableContext`] with an isolated checkpoint namespace.
    /// Items execute via `tokio::spawn` and must satisfy `Send + 'static`.
    ///
    /// When `batch_size` is configured via [`MapOptions`], items process in
    /// sequential batches — each batch completes before the next begins.
    ///
    /// During replay mode, returns the cached [`BatchResult`] without
    /// re-executing any item closures.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the map operation
    /// * `items` — Collection of items to process
    /// * `options` — Map configuration (batching)
    /// * `f` — Closure applied to each item with an owned child context
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::MapFailed`] if the map operation itself fails
    /// (e.g., checkpoint error, task panic). Individual item failures are
    /// captured in the [`BatchResult`] rather than propagated as errors.
    /// Returns [`DurableError::CheckpointFailed`] if checkpoint API calls fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_core::types::MapOptions;
    /// use durable_lambda_core::context::DurableContext;
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let items = vec![1, 2, 3];
    /// let result = ctx.map(
    ///     "process_items",
    ///     items,
    ///     MapOptions::new(),
    ///     |item: i32, mut child_ctx: DurableContext| async move {
    ///         let r: Result<i32, String> = child_ctx.step("double", move || async move { Ok(item * 2) }).await?;
    ///         Ok(r.unwrap())
    ///     },
    /// ).await?;
    /// assert_eq!(result.results.len(), 3);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::await_holding_lock)]
    pub async fn map<T, I, F, Fut>(
        &mut self,
        name: &str,
        items: Vec<I>,
        options: MapOptions,
        f: F,
    ) -> Result<BatchResult<T>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        I: Send + 'static,
        F: FnOnce(I, DurableContext) -> Fut + Send + 'static + Clone,
        Fut: Future<Output = Result<T, DurableError>> + Send + 'static,
    {
        let op_id = self.replay_engine_mut().generate_operation_id();

        let span = tracing::info_span!(
            "durable_operation",
            op.name = name,
            op.type = "map",
            op.id = %op_id,
        );
        let _guard = span.enter();
        tracing::trace!("durable_operation");

        // Replay path: check for completed outer map operation.
        if let Some(op) = self.replay_engine().check_result(&op_id) {
            if op.status == OperationStatus::Succeeded {
                let result_str =
                    op.context_details()
                        .and_then(|d| d.result())
                        .ok_or_else(|| {
                            DurableError::checkpoint_failed(
                                name,
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "map succeeded but no result in context_details",
                                ),
                            )
                        })?;

                let batch_result: BatchResult<T> = serde_json::from_str(result_str)
                    .map_err(|e| DurableError::deserialization("BatchResult", e))?;

                self.replay_engine_mut().track_replay(&op_id);
                return Ok(batch_result);
            } else {
                let error_message = op
                    .context_details()
                    .and_then(|d| d.error())
                    .map(|e| {
                        format!(
                            "{}: {}",
                            e.error_type().unwrap_or("Unknown"),
                            e.error_data().unwrap_or("")
                        )
                    })
                    .unwrap_or_else(|| "map failed".to_string());
                return Err(DurableError::map_failed(name, error_message));
            }
        }

        // Execute path: send outer Context/START for the map block.
        let outer_start = OperationUpdate::builder()
            .id(op_id.clone())
            .r#type(OperationType::Context)
            .action(OperationAction::Start)
            .sub_type("Map")
            .name(name)
            .build()
            .map_err(|e| DurableError::checkpoint_failed(name, e))?;

        let start_response = self
            .backend()
            .checkpoint(self.arn(), self.checkpoint_token(), vec![outer_start], None)
            .await?;

        let new_token = start_response.checkpoint_token().ok_or_else(|| {
            DurableError::checkpoint_failed(
                name,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "checkpoint response missing checkpoint_token",
                ),
            )
        })?;
        self.set_checkpoint_token(new_token.to_string());

        if let Some(new_state) = start_response.new_execution_state() {
            for op in new_state.operations() {
                self.replay_engine_mut()
                    .insert_operation(op.id().to_string(), op.clone());
            }
        }

        // Process items in batches.
        let item_count = items.len();
        let batch_size = options.get_batch_size().unwrap_or(item_count).max(1);
        let mut all_results: Vec<(usize, Result<T, DurableError>)> = Vec::with_capacity(item_count);

        // Single OperationIdGenerator for deterministic IDs across all batches.
        let mut item_id_gen = crate::operation_id::OperationIdGenerator::new(Some(op_id.clone()));

        let mut items_iter = items.into_iter().enumerate().peekable();

        while items_iter.peek().is_some() {
            let batch: Vec<(usize, I)> = items_iter.by_ref().take(batch_size).collect();
            let mut handles = Vec::with_capacity(batch.len());

            for (index, item) in batch {
                let item_op_id = item_id_gen.next_id();
                let child_ctx = self.create_child_context(&item_op_id);
                let config = ItemConfig {
                    backend: self.backend().clone(),
                    arn: self.arn().to_string(),
                    token: self.checkpoint_token().to_string(),
                    item_op_id,
                    parent_op_id: op_id.clone(),
                    item_name: format!("map-item-{index}"),
                };
                let f_clone = f.clone();

                let handle = tokio::spawn(async move {
                    let result = execute_item(child_ctx, config, item, f_clone).await;
                    (index, result)
                });

                handles.push(handle);
            }

            // Await all handles in this batch before starting next batch.
            for handle in handles {
                let (index, result) = handle
                    .await
                    .map_err(|e| DurableError::map_failed(name, format!("item panicked: {e}")))?;
                all_results.push((index, result));
            }
        }

        // Sort by index to maintain correspondence with input order.
        all_results.sort_by_key(|(index, _)| *index);

        // Build BatchResult from outcomes.
        let results: Vec<BatchItem<T>> = all_results
            .into_iter()
            .map(|(index, result)| match result {
                Ok(value) => BatchItem {
                    index,
                    status: BatchItemStatus::Succeeded,
                    result: Some(value),
                    error: None,
                },
                Err(err) => BatchItem {
                    index,
                    status: BatchItemStatus::Failed,
                    result: None,
                    error: Some(err.to_string()),
                },
            })
            .collect();

        let batch_result = BatchResult {
            results,
            completion_reason: CompletionReason::AllCompleted,
        };

        // Send outer Context/SUCCEED with serialized BatchResult.
        let serialized_result = serde_json::to_string(&batch_result)
            .map_err(|e| DurableError::serialization("BatchResult", e))?;

        let ctx_opts = aws_sdk_lambda::types::ContextOptions::builder()
            .replay_children(false)
            .build();

        let outer_succeed = OperationUpdate::builder()
            .id(op_id.clone())
            .r#type(OperationType::Context)
            .action(OperationAction::Succeed)
            .sub_type("Map")
            .payload(serialized_result)
            .context_options(ctx_opts)
            .build()
            .map_err(|e| DurableError::checkpoint_failed(name, e))?;

        let succeed_response = self
            .backend()
            .checkpoint(
                self.arn(),
                self.checkpoint_token(),
                vec![outer_succeed],
                None,
            )
            .await?;

        let new_token = succeed_response.checkpoint_token().ok_or_else(|| {
            DurableError::checkpoint_failed(
                name,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "checkpoint response missing checkpoint_token",
                ),
            )
        })?;
        self.set_checkpoint_token(new_token.to_string());

        if let Some(new_state) = succeed_response.new_execution_state() {
            for op in new_state.operations() {
                self.replay_engine_mut()
                    .insert_operation(op.id().to_string(), op.clone());
            }
        }

        self.replay_engine_mut().track_replay(&op_id);
        Ok(batch_result)
    }
}

/// Configuration for executing a single item within a map operation.
struct ItemConfig {
    backend: std::sync::Arc<dyn crate::backend::DurableBackend>,
    arn: String,
    token: String,
    item_op_id: String,
    parent_op_id: String,
    item_name: String,
}

/// Execute a single item within the map operation.
///
/// Send Context/START and Context/SUCCEED checkpoints around the item
/// closure execution.
async fn execute_item<T, I, F, Fut>(
    child_ctx: DurableContext,
    config: ItemConfig,
    item: I,
    f: F,
) -> Result<T, DurableError>
where
    T: Serialize + Send + 'static,
    I: Send + 'static,
    F: FnOnce(I, DurableContext) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, DurableError>> + Send + 'static,
{
    // Send Context/START for this item.
    let item_start = OperationUpdate::builder()
        .id(config.item_op_id.clone())
        .r#type(OperationType::Context)
        .action(OperationAction::Start)
        .sub_type("MapItem")
        .name(&config.item_name)
        .parent_id(config.parent_op_id.clone())
        .build()
        .map_err(|e| DurableError::checkpoint_failed(&config.item_name, e))?;

    let _ = config
        .backend
        .checkpoint(&config.arn, &config.token, vec![item_start], None)
        .await?;

    // Execute the closure with the item and child context.
    let result = f(item, child_ctx).await?;

    // Send Context/SUCCEED for this item.
    let serialized = serde_json::to_string(&result)
        .map_err(|e| DurableError::serialization(&config.item_name, e))?;

    let ctx_opts = aws_sdk_lambda::types::ContextOptions::builder()
        .replay_children(false)
        .build();

    let item_succeed = OperationUpdate::builder()
        .id(config.item_op_id.clone())
        .r#type(OperationType::Context)
        .action(OperationAction::Succeed)
        .sub_type("MapItem")
        .name(&config.item_name)
        .parent_id(config.parent_op_id.clone())
        .payload(serialized)
        .context_options(ctx_opts)
        .build()
        .map_err(|e| DurableError::checkpoint_failed(&config.item_name, e))?;

    let _ = config
        .backend
        .checkpoint(&config.arn, &config.token, vec![item_succeed], None)
        .await?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
    use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
    use aws_sdk_lambda::types::{
        ContextDetails, Operation, OperationAction, OperationStatus, OperationType, OperationUpdate,
    };
    use aws_smithy_types::DateTime;
    use tokio::sync::Mutex;
    use tracing_test::traced_test;

    use crate::backend::DurableBackend;
    use crate::context::DurableContext;
    use crate::error::DurableError;
    use crate::types::MapOptions;

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct CheckpointCall {
        arn: String,
        checkpoint_token: String,
        updates: Vec<OperationUpdate>,
    }

    /// MockBackend that records all checkpoint calls.
    struct MapMockBackend {
        calls: Arc<Mutex<Vec<CheckpointCall>>>,
    }

    impl MapMockBackend {
        fn new() -> (Self, Arc<Mutex<Vec<CheckpointCall>>>) {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let backend = Self {
                calls: calls.clone(),
            };
            (backend, calls)
        }
    }

    #[async_trait::async_trait]
    impl DurableBackend for MapMockBackend {
        async fn checkpoint(
            &self,
            arn: &str,
            checkpoint_token: &str,
            updates: Vec<OperationUpdate>,
            _client_token: Option<&str>,
        ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
            self.calls.lock().await.push(CheckpointCall {
                arn: arn.to_string(),
                checkpoint_token: checkpoint_token.to_string(),
                updates,
            });
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

    fn first_op_id() -> String {
        let mut gen = crate::operation_id::OperationIdGenerator::new(None);
        gen.next_id()
    }

    // ─── map tests ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_map_executes_items_concurrently() {
        let (backend, calls) = MapMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let items = vec![10, 20, 30];
        let result = ctx
            .map(
                "process",
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

        assert_eq!(result.results.len(), 3);
        // Results should be ordered by index
        assert_eq!(result.results[0].index, 0);
        assert_eq!(result.results[1].index, 1);
        assert_eq!(result.results[2].index, 2);
        assert_eq!(result.results[0].result, Some(20));
        assert_eq!(result.results[1].result, Some(40));
        assert_eq!(result.results[2].result, Some(60));

        // Verify checkpoints were sent
        let captured = calls.lock().await;
        assert!(
            captured.len() >= 2,
            "should have at least outer START and outer SUCCEED"
        );

        // First checkpoint: outer START with type Context, sub_type "Map"
        assert_eq!(captured[0].updates[0].r#type(), &OperationType::Context);
        assert_eq!(captured[0].updates[0].action(), &OperationAction::Start);
        assert_eq!(captured[0].updates[0].sub_type(), Some("Map"));

        // Last checkpoint: outer SUCCEED
        let last = &captured[captured.len() - 1];
        assert_eq!(last.updates[0].r#type(), &OperationType::Context);
        assert_eq!(last.updates[0].action(), &OperationAction::Succeed);
        assert_eq!(last.updates[0].sub_type(), Some("Map"));
        assert!(
            last.updates[0].payload().is_some(),
            "should have BatchResult payload"
        );
    }

    #[tokio::test]
    async fn test_map_replays_from_cached_result() {
        let op_id = first_op_id();

        let batch_json = r#"{"results":[{"index":0,"status":"Succeeded","result":100,"error":null},{"index":1,"status":"Succeeded","result":200,"error":null}],"completion_reason":"AllCompleted"}"#;

        let map_op = Operation::builder()
            .id(&op_id)
            .r#type(OperationType::Context)
            .status(OperationStatus::Succeeded)
            .start_timestamp(DateTime::from_secs(0))
            .context_details(
                ContextDetails::builder()
                    .replay_children(false)
                    .result(batch_json)
                    .build(),
            )
            .build()
            .unwrap();

        let (backend, calls) = MapMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![map_op],
            None,
        )
        .await
        .unwrap();

        // This closure should NOT execute during replay
        let result: crate::types::BatchResult<i32> = ctx
            .map(
                "process",
                vec![1],
                MapOptions::new(),
                |_item: i32, _ctx: DurableContext| async move {
                    panic!("item should not execute during replay")
                },
            )
            .await
            .unwrap();

        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].result, Some(100));
        assert_eq!(result.results[1].result, Some(200));

        // No checkpoints during replay
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0, "no checkpoints during replay");
    }

    #[tokio::test]
    async fn test_map_items_have_isolated_namespaces() {
        let (backend, _calls) = MapMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // All items use the same step name "work" — should NOT collide
        let items = vec!["alpha", "beta"];
        let result = ctx
            .map(
                "isolated_test",
                items,
                MapOptions::new(),
                |item: &str, mut child_ctx: DurableContext| async move {
                    let r: Result<String, String> = child_ctx
                        .step("work", move || async move { Ok(format!("result-{item}")) })
                        .await?;
                    Ok(r.unwrap())
                },
            )
            .await
            .unwrap();

        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].result.as_deref(), Some("result-alpha"));
        assert_eq!(result.results[1].result.as_deref(), Some("result-beta"));
    }

    #[tokio::test]
    async fn test_map_sends_correct_checkpoint_sequence() {
        let (backend, calls) = MapMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let items = vec![1, 2];
        let _ = ctx
            .map(
                "seq_test",
                items,
                MapOptions::new(),
                |_item: i32, _ctx: DurableContext| async move { Ok(0i32) },
            )
            .await
            .unwrap();

        let captured = calls.lock().await;

        // Expected: outer START, item0 START, item0 SUCCEED, item1 START, item1 SUCCEED, outer SUCCEED
        // (item order may vary due to tokio scheduling)
        assert!(
            captured.len() >= 6,
            "expected at least 6 checkpoints, got {}",
            captured.len()
        );

        // First: outer Context/START with sub_type "Map"
        assert_eq!(captured[0].updates[0].sub_type(), Some("Map"));
        assert_eq!(captured[0].updates[0].action(), &OperationAction::Start);

        // Last: outer Context/SUCCEED with sub_type "Map"
        let last_idx = captured.len() - 1;
        assert_eq!(captured[last_idx].updates[0].sub_type(), Some("Map"));
        assert_eq!(
            captured[last_idx].updates[0].action(),
            &OperationAction::Succeed
        );

        // Middle checkpoints should contain MapItem START and SUCCEED pairs
        let item_checkpoints: Vec<_> = captured[1..last_idx]
            .iter()
            .filter(|c| c.updates[0].sub_type() == Some("MapItem"))
            .collect();
        assert_eq!(
            item_checkpoints.len(),
            4,
            "expected 4 item checkpoints (2 START + 2 SUCCEED)"
        );
    }

    #[tokio::test]
    async fn test_map_item_failure_is_captured() {
        let (backend, _calls) = MapMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let items = vec![1, 2];
        let result = ctx
            .map(
                "fail_test",
                items,
                MapOptions::new(),
                |item: i32, _ctx: DurableContext| async move {
                    if item == 2 {
                        Err(DurableError::map_failed("item", "intentional failure"))
                    } else {
                        Ok(item * 10)
                    }
                },
            )
            .await
            .unwrap();

        assert_eq!(result.results.len(), 2);
        assert_eq!(
            result.results[0].status,
            crate::types::BatchItemStatus::Succeeded
        );
        assert_eq!(result.results[0].result, Some(10));
        assert_eq!(
            result.results[1].status,
            crate::types::BatchItemStatus::Failed
        );
        assert!(result.results[1].error.is_some());
        assert!(result.results[1]
            .error
            .as_ref()
            .unwrap()
            .contains("intentional failure"));
    }

    #[tokio::test]
    async fn test_map_batching_processes_sequentially() {
        let (backend, _calls) = MapMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // Track execution order using an atomic counter.
        // Batch 1 (items 0,1) should complete before batch 2 (items 2,3).
        let execution_order = Arc::new(AtomicUsize::new(0));

        let items = vec![0usize, 1, 2, 3];
        let order = execution_order.clone();
        let result = ctx
            .map(
                "batch_test",
                items,
                MapOptions::new().batch_size(2),
                move |item: usize, _ctx: DurableContext| {
                    let order = order.clone();
                    async move {
                        let seq = order.fetch_add(1, Ordering::SeqCst);
                        // Return the execution sequence number for this item
                        Ok((item, seq))
                    }
                },
            )
            .await
            .unwrap();

        assert_eq!(result.results.len(), 4);

        // Items 0 and 1 (batch 1) should have sequence numbers 0 and 1
        // Items 2 and 3 (batch 2) should have sequence numbers 2 and 3
        let item0 = result.results[0].result.as_ref().unwrap();
        let item1 = result.results[1].result.as_ref().unwrap();
        let item2 = result.results[2].result.as_ref().unwrap();
        let item3 = result.results[3].result.as_ref().unwrap();

        // Batch 1 items should have seq < batch 2 items
        assert!(item0.1 < 2, "batch 1 item should execute before batch 2");
        assert!(item1.1 < 2, "batch 1 item should execute before batch 2");
        assert!(item2.1 >= 2, "batch 2 item should execute after batch 1");
        assert!(item3.1 >= 2, "batch 2 item should execute after batch 1");
    }

    #[tokio::test]
    async fn test_map_default_options_all_concurrent() {
        let (backend, _calls) = MapMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // With default options, all items should be in a single batch
        let items = vec![1, 2, 3, 4, 5];
        let result = ctx
            .map(
                "all_concurrent",
                items,
                MapOptions::new(), // No batch_size = all concurrent
                |item: i32, _ctx: DurableContext| async move { Ok(item) },
            )
            .await
            .unwrap();

        assert_eq!(result.results.len(), 5);
        for (i, r) in result.results.iter().enumerate() {
            assert_eq!(r.index, i);
            assert_eq!(r.result, Some((i + 1) as i32));
        }
    }

    // ─── span tests (FEAT-17) ─────────────────────────────────────────────

    #[traced_test]
    #[tokio::test]
    async fn test_map_emits_span() {
        let (backend, _calls) = MapMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();
        // empty items — returns empty BatchResult
        let _ = ctx
            .map(
                "process",
                Vec::<i32>::new(),
                MapOptions::new(),
                |item: i32, _ctx: DurableContext| async move { Ok(item) },
            )
            .await;
        assert!(logs_contain("durable_operation"));
        assert!(logs_contain("process"));
        assert!(logs_contain("map"));
    }
}
