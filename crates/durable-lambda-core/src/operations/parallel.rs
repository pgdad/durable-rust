//! Parallel operation — fan-out with completion criteria.
//!
//! Implement FR19-FR22: concurrent branches, completion criteria,
//! independent checkpoint namespaces, tokio::spawn with Send + 'static.
//!
//! The parallel operation uses `OperationType::Context` on the wire with
//! sub_type "Parallel" for the outer block and "ParallelBranch" for each
//! branch. Each branch gets its own child `DurableContext` with an isolated
//! operation ID namespace.

use std::future::Future;

use aws_sdk_lambda::types::{OperationAction, OperationStatus, OperationType, OperationUpdate};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::context::DurableContext;
use crate::error::DurableError;
use crate::types::{BatchItem, BatchItemStatus, BatchResult, CompletionReason, ParallelOptions};

impl DurableContext {
    /// Execute multiple branches concurrently and return their results.
    ///
    /// Each branch receives an owned child [`DurableContext`] with an isolated
    /// checkpoint namespace. Branches execute concurrently via `tokio::spawn`
    /// and must satisfy `Send + 'static` bounds.
    ///
    /// During replay mode, returns the cached [`BatchResult`] without
    /// re-executing any branches.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the parallel operation
    /// * `branches` — Collection of branch closures, each receiving an owned `DurableContext`
    /// * `_options` — Parallel configuration (reserved for future completion criteria)
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::ParallelFailed`] if the parallel operation itself
    /// fails (e.g., checkpoint error). Individual branch failures are captured
    /// in the [`BatchResult`] rather than propagated as errors.
    /// Returns [`DurableError::CheckpointFailed`] if checkpoint API calls fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_core::types::ParallelOptions;
    /// use durable_lambda_core::context::DurableContext;
    /// use durable_lambda_core::error::DurableError;
    /// use std::pin::Pin;
    /// use std::future::Future;
    ///
    /// type BranchFn = Box<dyn FnOnce(DurableContext) -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>> + Send>;
    ///
    /// let branches: Vec<BranchFn> = vec![
    ///     Box::new(|mut ctx| Box::pin(async move {
    ///         let r: Result<i32, String> = ctx.step("validate", || async { Ok(1) }).await?;
    ///         Ok(r.unwrap())
    ///     })),
    ///     Box::new(|mut ctx| Box::pin(async move {
    ///         let r: Result<i32, String> = ctx.step("check", || async { Ok(2) }).await?;
    ///         Ok(r.unwrap())
    ///     })),
    /// ];
    ///
    /// let result = ctx.parallel("fan_out", branches, ParallelOptions::new()).await?;
    /// assert_eq!(result.results.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn parallel<T, F, Fut>(
        &mut self,
        name: &str,
        branches: Vec<F>,
        _options: ParallelOptions,
    ) -> Result<BatchResult<T>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce(DurableContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, DurableError>> + Send + 'static,
    {
        let op_id = self.replay_engine_mut().generate_operation_id();

        // Replay path: check for completed outer parallel operation.
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
                                    "parallel succeeded but no result in context_details",
                                ),
                            )
                        })?;

                let batch_result: BatchResult<T> = serde_json::from_str(result_str)
                    .map_err(|e| DurableError::deserialization("BatchResult", e))?;

                self.replay_engine_mut().track_replay(&op_id);
                return Ok(batch_result);
            } else {
                // Failed/Cancelled/TimedOut/Stopped
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
                    .unwrap_or_else(|| "parallel failed".to_string());
                return Err(DurableError::parallel_failed(name, error_message));
            }
        }

        // Execute path: send outer Context/START for the parallel block.
        let outer_start = OperationUpdate::builder()
            .id(op_id.clone())
            .r#type(OperationType::Context)
            .action(OperationAction::Start)
            .sub_type("Parallel")
            .name(name)
            .build()
            .map_err(|e| DurableError::checkpoint_failed(name, e))?;

        let start_response = self
            .backend()
            .checkpoint(self.arn(), self.checkpoint_token(), vec![outer_start], None)
            .await?;

        if let Some(token) = start_response.checkpoint_token() {
            self.set_checkpoint_token(token.to_string());
        }

        if let Some(new_state) = start_response.new_execution_state() {
            for op in new_state.operations() {
                self.replay_engine_mut()
                    .insert_operation(op.id().to_string(), op.clone());
            }
        }

        // Spawn each branch with its own child context.
        let branch_count = branches.len();
        let mut handles = Vec::with_capacity(branch_count);

        // Generate branch operation IDs using a child generator scoped to the parallel op.
        let mut branch_id_gen = crate::operation_id::OperationIdGenerator::new(Some(op_id.clone()));

        for (i, branch_fn) in branches.into_iter().enumerate() {
            let branch_op_id = branch_id_gen.next_id();

            let child_ctx = self.create_child_context(&branch_op_id);
            let config = BranchConfig {
                backend: self.backend().clone(),
                arn: self.arn().to_string(),
                token: self.checkpoint_token().to_string(),
                branch_op_id,
                parent_op_id: op_id.clone(),
                branch_name: format!("parallel-branch-{i}"),
            };

            let handle =
                tokio::spawn(async move { execute_branch(child_ctx, config, branch_fn).await });

            handles.push(handle);
        }

        // Collect results from all branches.
        let mut results = Vec::with_capacity(branch_count);
        for (i, handle) in handles.into_iter().enumerate() {
            let branch_outcome = handle.await.map_err(|e| {
                DurableError::parallel_failed(name, format!("branch {i} panicked: {e}"))
            })?;

            match branch_outcome {
                Ok(value) => {
                    results.push(BatchItem {
                        index: i,
                        status: BatchItemStatus::Succeeded,
                        result: Some(value),
                        error: None,
                    });
                }
                Err(err) => {
                    results.push(BatchItem {
                        index: i,
                        status: BatchItemStatus::Failed,
                        result: None,
                        error: Some(err.to_string()),
                    });
                }
            }
        }

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
            .sub_type("Parallel")
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

        if let Some(token) = succeed_response.checkpoint_token() {
            self.set_checkpoint_token(token.to_string());
        }

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

/// Configuration for executing a single branch within a parallel operation.
struct BranchConfig {
    backend: std::sync::Arc<dyn crate::backend::DurableBackend>,
    arn: String,
    token: String,
    branch_op_id: String,
    parent_op_id: String,
    branch_name: String,
}

/// Execute a single branch within the parallel operation.
///
/// Sends Context/START and Context/SUCCEED checkpoints around the branch
/// closure execution.
async fn execute_branch<T, F, Fut>(
    child_ctx: DurableContext,
    config: BranchConfig,
    branch_fn: F,
) -> Result<T, DurableError>
where
    T: Serialize + Send + 'static,
    F: FnOnce(DurableContext) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, DurableError>> + Send + 'static,
{
    // Send Context/START for this branch.
    let branch_start = OperationUpdate::builder()
        .id(config.branch_op_id.clone())
        .r#type(OperationType::Context)
        .action(OperationAction::Start)
        .sub_type("ParallelBranch")
        .name(&config.branch_name)
        .parent_id(config.parent_op_id.clone())
        .build()
        .map_err(|e| DurableError::checkpoint_failed(&config.branch_name, e))?;

    let _ = config
        .backend
        .checkpoint(&config.arn, &config.token, vec![branch_start], None)
        .await?;

    // Execute the branch closure with the child context.
    let result = branch_fn(child_ctx).await?;

    // Send Context/SUCCEED for this branch.
    let serialized = serde_json::to_string(&result)
        .map_err(|e| DurableError::serialization(&config.branch_name, e))?;

    let ctx_opts = aws_sdk_lambda::types::ContextOptions::builder()
        .replay_children(false)
        .build();

    let branch_succeed = OperationUpdate::builder()
        .id(config.branch_op_id.clone())
        .r#type(OperationType::Context)
        .action(OperationAction::Succeed)
        .sub_type("ParallelBranch")
        .name(&config.branch_name)
        .parent_id(config.parent_op_id.clone())
        .payload(serialized)
        .context_options(ctx_opts)
        .build()
        .map_err(|e| DurableError::checkpoint_failed(&config.branch_name, e))?;

    let _ = config
        .backend
        .checkpoint(&config.arn, &config.token, vec![branch_succeed], None)
        .await?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
    use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
    use aws_sdk_lambda::types::{
        ContextDetails, Operation, OperationAction, OperationStatus, OperationType, OperationUpdate,
    };
    use aws_smithy_types::DateTime;
    use tokio::sync::Mutex;

    use crate::backend::DurableBackend;
    use crate::context::DurableContext;
    use crate::error::DurableError;
    use crate::types::ParallelOptions;

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct CheckpointCall {
        arn: String,
        checkpoint_token: String,
        updates: Vec<OperationUpdate>,
    }

    /// MockBackend that records all checkpoint calls.
    struct ParallelMockBackend {
        calls: Arc<Mutex<Vec<CheckpointCall>>>,
    }

    impl ParallelMockBackend {
        fn new() -> (Self, Arc<Mutex<Vec<CheckpointCall>>>) {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let backend = Self {
                calls: calls.clone(),
            };
            (backend, calls)
        }
    }

    #[async_trait::async_trait]
    impl DurableBackend for ParallelMockBackend {
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

    // ─── parallel tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_parallel_executes_branches_concurrently() {
        let (backend, calls) = ParallelMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let branches: Vec<
            Box<
                dyn FnOnce(
                        DurableContext,
                    ) -> std::pin::Pin<
                        Box<dyn std::future::Future<Output = Result<i32, DurableError>> + Send>,
                    > + Send,
            >,
        > = vec![
            Box::new(|mut ctx: DurableContext| {
                Box::pin(async move {
                    let r: Result<i32, String> = ctx.step("validate", || async { Ok(10) }).await?;
                    Ok(r.unwrap())
                })
            }),
            Box::new(|mut ctx: DurableContext| {
                Box::pin(async move {
                    let r: Result<i32, String> = ctx.step("check", || async { Ok(20) }).await?;
                    Ok(r.unwrap())
                })
            }),
            Box::new(|mut ctx: DurableContext| {
                Box::pin(async move {
                    let r: Result<i32, String> = ctx.step("process", || async { Ok(30) }).await?;
                    Ok(r.unwrap())
                })
            }),
        ];

        let result = ctx
            .parallel("fan_out", branches, ParallelOptions::new())
            .await
            .unwrap();

        assert_eq!(result.results.len(), 3);
        // Results should be ordered by index
        assert_eq!(result.results[0].index, 0);
        assert_eq!(result.results[1].index, 1);
        assert_eq!(result.results[2].index, 2);
        assert_eq!(result.results[0].result, Some(10));
        assert_eq!(result.results[1].result, Some(20));
        assert_eq!(result.results[2].result, Some(30));

        // Verify checkpoints: outer START + 3*(branch START + step START+SUCCEED + branch SUCCEED) + outer SUCCEED
        let captured = calls.lock().await;
        assert!(
            captured.len() >= 2,
            "should have at least outer START and outer SUCCEED"
        );

        // First checkpoint should be outer START with type Context
        assert_eq!(captured[0].updates[0].r#type(), &OperationType::Context);
        assert_eq!(captured[0].updates[0].action(), &OperationAction::Start);
        assert_eq!(captured[0].updates[0].sub_type(), Some("Parallel"));

        // Last checkpoint should be outer SUCCEED
        let last = &captured[captured.len() - 1];
        assert_eq!(last.updates[0].r#type(), &OperationType::Context);
        assert_eq!(last.updates[0].action(), &OperationAction::Succeed);
        assert_eq!(last.updates[0].sub_type(), Some("Parallel"));
        assert!(
            last.updates[0].payload().is_some(),
            "should have BatchResult payload"
        );
    }

    #[tokio::test]
    async fn test_parallel_replays_from_cached_result() {
        let op_id = first_op_id();

        // Create a SUCCEEDED parallel operation with cached BatchResult in context_details
        let batch_json = r#"{"results":[{"index":0,"status":"Succeeded","result":42,"error":null},{"index":1,"status":"Succeeded","result":99,"error":null}],"completion_reason":"AllCompleted"}"#;

        let parallel_op = Operation::builder()
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

        let (backend, calls) = ParallelMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![parallel_op],
            None,
        )
        .await
        .unwrap();

        // These branches should NOT execute during replay
        let branches: Vec<
            Box<
                dyn FnOnce(
                        DurableContext,
                    ) -> std::pin::Pin<
                        Box<dyn std::future::Future<Output = Result<i32, DurableError>> + Send>,
                    > + Send,
            >,
        > = vec![Box::new(|_ctx: DurableContext| {
            Box::pin(async move { panic!("branch should not execute during replay") })
        })];

        let result: crate::types::BatchResult<i32> = ctx
            .parallel("fan_out", branches, ParallelOptions::new())
            .await
            .unwrap();

        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].result, Some(42));
        assert_eq!(result.results[1].result, Some(99));

        // No checkpoints during replay
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0, "no checkpoints during replay");
    }

    #[tokio::test]
    async fn test_parallel_branches_have_isolated_namespaces() {
        let (backend, _calls) = ParallelMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // Both branches use the same step name "work" — should NOT collide
        let branches: Vec<
            Box<
                dyn FnOnce(
                        DurableContext,
                    ) -> std::pin::Pin<
                        Box<dyn std::future::Future<Output = Result<String, DurableError>> + Send>,
                    > + Send,
            >,
        > = vec![
            Box::new(|mut ctx: DurableContext| {
                Box::pin(async move {
                    let r: Result<String, String> = ctx
                        .step("work", || async { Ok("branch-0".to_string()) })
                        .await?;
                    Ok(r.unwrap())
                })
            }),
            Box::new(|mut ctx: DurableContext| {
                Box::pin(async move {
                    let r: Result<String, String> = ctx
                        .step("work", || async { Ok("branch-1".to_string()) })
                        .await?;
                    Ok(r.unwrap())
                })
            }),
        ];

        let result = ctx
            .parallel("isolated_test", branches, ParallelOptions::new())
            .await
            .unwrap();

        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].result.as_deref(), Some("branch-0"));
        assert_eq!(result.results[1].result.as_deref(), Some("branch-1"));
    }

    #[tokio::test]
    async fn test_parallel_sends_correct_checkpoint_sequence() {
        let (backend, calls) = ParallelMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let branches: Vec<
            Box<
                dyn FnOnce(
                        DurableContext,
                    ) -> std::pin::Pin<
                        Box<dyn std::future::Future<Output = Result<i32, DurableError>> + Send>,
                    > + Send,
            >,
        > = vec![
            Box::new(|_ctx: DurableContext| Box::pin(async move { Ok(1) })),
            Box::new(|_ctx: DurableContext| Box::pin(async move { Ok(2) })),
        ];

        let _ = ctx
            .parallel("seq_test", branches, ParallelOptions::new())
            .await
            .unwrap();

        let captured = calls.lock().await;

        // Expected: outer START, branch0 START, branch0 SUCCEED, branch1 START, branch1 SUCCEED, outer SUCCEED
        // (branch order may vary due to tokio scheduling)
        assert!(
            captured.len() >= 6,
            "expected at least 6 checkpoints, got {}",
            captured.len()
        );

        // First: outer Context/START with sub_type "Parallel"
        assert_eq!(captured[0].updates[0].sub_type(), Some("Parallel"));
        assert_eq!(captured[0].updates[0].action(), &OperationAction::Start);

        // Last: outer Context/SUCCEED with sub_type "Parallel"
        let last_idx = captured.len() - 1;
        assert_eq!(captured[last_idx].updates[0].sub_type(), Some("Parallel"));
        assert_eq!(
            captured[last_idx].updates[0].action(),
            &OperationAction::Succeed
        );

        // Middle checkpoints should contain ParallelBranch START and SUCCEED pairs
        let branch_checkpoints: Vec<_> = captured[1..last_idx]
            .iter()
            .filter(|c| c.updates[0].sub_type() == Some("ParallelBranch"))
            .collect();
        assert_eq!(
            branch_checkpoints.len(),
            4,
            "expected 4 branch checkpoints (2 START + 2 SUCCEED)"
        );
    }

    #[tokio::test]
    async fn test_parallel_branch_failure_is_captured() {
        let (backend, _calls) = ParallelMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let branches: Vec<
            Box<
                dyn FnOnce(
                        DurableContext,
                    ) -> std::pin::Pin<
                        Box<dyn std::future::Future<Output = Result<i32, DurableError>> + Send>,
                    > + Send,
            >,
        > = vec![
            Box::new(|_ctx: DurableContext| Box::pin(async move { Ok(42) })),
            Box::new(|_ctx: DurableContext| {
                Box::pin(async move {
                    Err(DurableError::parallel_failed(
                        "branch",
                        "intentional failure",
                    ))
                })
            }),
        ];

        let result = ctx
            .parallel("fail_test", branches, ParallelOptions::new())
            .await
            .unwrap();

        assert_eq!(result.results.len(), 2);
        assert_eq!(
            result.results[0].status,
            crate::types::BatchItemStatus::Succeeded
        );
        assert_eq!(result.results[0].result, Some(42));
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
}
