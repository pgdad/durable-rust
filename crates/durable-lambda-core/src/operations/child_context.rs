//! Child context operation — isolated subflows.
//!
//! Implements FR26-FR28: isolated checkpoint namespace, independent operations,
//! fully owned child contexts sharing only `Arc<dyn DurableBackend>`.
//!
//! The child context operation uses `OperationType::Context` on the wire with
//! sub_type "Context". Unlike parallel, there is only a single closure that
//! runs inline (no `tokio::spawn`), and the result is returned directly as `T`
//! rather than wrapped in `BatchResult`.

use std::future::Future;

use aws_sdk_lambda::types::{OperationAction, OperationStatus, OperationType, OperationUpdate};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::context::DurableContext;
use crate::error::DurableError;

impl DurableContext {
    /// Execute an isolated subflow with its own checkpoint namespace.
    ///
    /// The closure receives an owned child [`DurableContext`] whose operations
    /// are namespaced under this child context's operation ID, preventing
    /// collisions with the parent or sibling child contexts.
    ///
    /// During replay mode, returns the cached result without re-executing
    /// the closure.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the child context operation
    /// * `f` — Closure receiving an owned `DurableContext` for the subflow
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::ChildContextFailed`] if the child context
    /// is found in a failed state during replay.
    /// Returns [`DurableError::CheckpointFailed`] if checkpoint API calls fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// let result: i32 = ctx.child_context("sub_workflow", |mut child_ctx| async move {
    ///     let r: Result<i32, String> = child_ctx.step("inner_step", || async { Ok(42) }).await?;
    ///     Ok(r.unwrap())
    /// }).await?;
    /// assert_eq!(result, 42);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::await_holding_lock)]
    pub async fn child_context<T, F, Fut>(&mut self, name: &str, f: F) -> Result<T, DurableError>
    where
        T: Serialize + DeserializeOwned + Send,
        F: FnOnce(DurableContext) -> Fut + Send,
        Fut: Future<Output = Result<T, DurableError>> + Send,
    {
        let op_id = self.replay_engine_mut().generate_operation_id();

        let span = tracing::info_span!(
            "durable_operation",
            op.name = name,
            op.type = "child_context",
            op.id = %op_id,
        );
        let _guard = span.enter();
        tracing::trace!("durable_operation");

        // Replay path: check for completed outer child context operation.
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
                                    "child context succeeded but no result in context_details",
                                ),
                            )
                        })?;

                let result: T = serde_json::from_str(result_str)
                    .map_err(|e| DurableError::deserialization(std::any::type_name::<T>(), e))?;

                self.replay_engine_mut().track_replay(&op_id);
                return Ok(result);
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
                    .unwrap_or_else(|| "child context failed".to_string());
                return Err(DurableError::child_context_failed(name, error_message));
            }
        }

        // Execute path: send Context/START for the child context block.
        let start_update = OperationUpdate::builder()
            .id(op_id.clone())
            .r#type(OperationType::Context)
            .action(OperationAction::Start)
            .sub_type("Context")
            .name(name)
            .build()
            .map_err(|e| DurableError::checkpoint_failed(name, e))?;

        let start_response = self
            .backend()
            .checkpoint(
                self.arn(),
                self.checkpoint_token(),
                vec![start_update],
                None,
            )
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

        // Create child context with isolated namespace.
        let child_ctx = self.create_child_context(&op_id);

        // Execute closure inline (no tokio::spawn).
        let result = f(child_ctx).await?;

        // Send Context/SUCCEED with serialized result as payload.
        let serialized_result = serde_json::to_string(&result)
            .map_err(|e| DurableError::serialization(std::any::type_name::<T>(), e))?;

        let ctx_opts = aws_sdk_lambda::types::ContextOptions::builder()
            .replay_children(false)
            .build();

        let succeed_update = OperationUpdate::builder()
            .id(op_id.clone())
            .r#type(OperationType::Context)
            .action(OperationAction::Succeed)
            .sub_type("Context")
            .payload(serialized_result)
            .context_options(ctx_opts)
            .build()
            .map_err(|e| DurableError::checkpoint_failed(name, e))?;

        let succeed_response = self
            .backend()
            .checkpoint(
                self.arn(),
                self.checkpoint_token(),
                vec![succeed_update],
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
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
    use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
    use aws_sdk_lambda::types::{
        ContextDetails, ErrorObject, Operation, OperationAction, OperationStatus, OperationType,
        OperationUpdate,
    };
    use aws_smithy_types::DateTime;
    use tokio::sync::Mutex;
    use tracing_test::traced_test;

    use crate::backend::DurableBackend;
    use crate::context::DurableContext;
    use crate::error::DurableError;

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct CheckpointCall {
        arn: String,
        checkpoint_token: String,
        updates: Vec<OperationUpdate>,
    }

    /// MockBackend that records all checkpoint calls.
    struct ChildContextMockBackend {
        calls: Arc<Mutex<Vec<CheckpointCall>>>,
    }

    impl ChildContextMockBackend {
        fn new() -> (Self, Arc<Mutex<Vec<CheckpointCall>>>) {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let backend = Self {
                calls: calls.clone(),
            };
            (backend, calls)
        }
    }

    #[async_trait::async_trait]
    impl DurableBackend for ChildContextMockBackend {
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

    // ─── child_context tests ────────────────────────────────────────────

    #[tokio::test]
    async fn test_child_context_executes_closure() {
        let (backend, calls) = ChildContextMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let result: i32 = ctx
            .child_context("sub_workflow", |mut child_ctx| async move {
                let r: Result<i32, String> =
                    child_ctx.step("inner_step", || async { Ok(42) }).await?;
                Ok(r.unwrap())
            })
            .await
            .unwrap();

        assert_eq!(result, 42);

        // Verify checkpoints: Context/START + inner step (START+SUCCEED) + Context/SUCCEED
        let captured = calls.lock().await;
        assert!(
            captured.len() >= 2,
            "should have at least Context/START and Context/SUCCEED, got {}",
            captured.len()
        );

        // First: Context/START with sub_type "Context"
        assert_eq!(captured[0].updates[0].r#type(), &OperationType::Context);
        assert_eq!(captured[0].updates[0].action(), &OperationAction::Start);
        assert_eq!(captured[0].updates[0].sub_type(), Some("Context"));

        // Last: Context/SUCCEED with sub_type "Context" and payload
        let last = &captured[captured.len() - 1];
        assert_eq!(last.updates[0].r#type(), &OperationType::Context);
        assert_eq!(last.updates[0].action(), &OperationAction::Succeed);
        assert_eq!(last.updates[0].sub_type(), Some("Context"));
        assert!(
            last.updates[0].payload().is_some(),
            "should have serialized result payload"
        );
    }

    #[tokio::test]
    async fn test_child_context_replays_from_cached_result() {
        let op_id = first_op_id();

        // Create a SUCCEEDED child context operation with cached result
        let child_op = Operation::builder()
            .id(&op_id)
            .r#type(OperationType::Context)
            .status(OperationStatus::Succeeded)
            .start_timestamp(DateTime::from_secs(0))
            .context_details(
                ContextDetails::builder()
                    .replay_children(false)
                    .result("42")
                    .build(),
            )
            .build()
            .unwrap();

        let (backend, calls) = ChildContextMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![child_op],
            None,
        )
        .await
        .unwrap();

        // Closure should NOT execute during replay
        let result: i32 = ctx
            .child_context("sub_workflow", |_child_ctx| async move {
                panic!("closure should not execute during replay")
            })
            .await
            .unwrap();

        assert_eq!(result, 42);

        // No checkpoints during replay
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0, "no checkpoints during replay");
    }

    #[tokio::test]
    async fn test_child_context_has_isolated_namespace() {
        let (backend, _calls) = ChildContextMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // Parent step with name "work"
        let parent_result: Result<String, String> = ctx
            .step("work", || async { Ok("parent".to_string()) })
            .await
            .unwrap();
        assert_eq!(parent_result.unwrap(), "parent");

        // Child context with step also named "work" — should NOT collide
        let child_result: String = ctx
            .child_context("sub_workflow", |mut child_ctx| async move {
                let r: Result<String, String> = child_ctx
                    .step("work", || async { Ok("child".to_string()) })
                    .await?;
                Ok(r.unwrap())
            })
            .await
            .unwrap();

        assert_eq!(child_result, "child");
    }

    #[tokio::test]
    async fn test_child_context_sends_correct_checkpoint_sequence() {
        let (backend, calls) = ChildContextMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let _result: i32 = ctx
            .child_context("seq_test", |_child_ctx| async move { Ok(99) })
            .await
            .unwrap();

        let captured = calls.lock().await;

        // Expected: Context/START + Context/SUCCEED (closure does no durable ops)
        assert_eq!(
            captured.len(),
            2,
            "expected exactly 2 checkpoints (START + SUCCEED), got {}",
            captured.len()
        );

        // First: Context/START with sub_type "Context"
        assert_eq!(captured[0].updates[0].r#type(), &OperationType::Context);
        assert_eq!(captured[0].updates[0].action(), &OperationAction::Start);
        assert_eq!(captured[0].updates[0].sub_type(), Some("Context"));
        assert_eq!(captured[0].updates[0].name(), Some("seq_test"));

        // Second: Context/SUCCEED with sub_type "Context"
        assert_eq!(captured[1].updates[0].r#type(), &OperationType::Context);
        assert_eq!(captured[1].updates[0].action(), &OperationAction::Succeed);
        assert_eq!(captured[1].updates[0].sub_type(), Some("Context"));
        assert_eq!(captured[1].updates[0].payload(), Some("99"));
    }

    #[tokio::test]
    async fn test_child_context_closure_failure_propagates() {
        let (backend, _calls) = ChildContextMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let result = ctx
            .child_context("failing_sub", |_child_ctx| async move {
                Err::<i32, _>(DurableError::child_context_failed(
                    "failing_sub",
                    "intentional failure",
                ))
            })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("intentional failure"),
            "error should contain failure message, got: {msg}"
        );
    }

    #[tokio::test]
    async fn test_child_context_nested() {
        let (backend, calls) = ChildContextMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let result: i32 = ctx
            .child_context("outer", |mut outer_child| async move {
                let inner_result: i32 = outer_child
                    .child_context("inner", |mut inner_child| async move {
                        let r: Result<i32, String> =
                            inner_child.step("deep_step", || async { Ok(7) }).await?;
                        Ok(r.unwrap())
                    })
                    .await?;
                Ok(inner_result * 6)
            })
            .await
            .unwrap();

        assert_eq!(result, 42);

        // Verify nested checkpoint structure:
        // outer START, inner START, step START+SUCCEED, inner SUCCEED, outer SUCCEED
        let captured = calls.lock().await;
        assert!(
            captured.len() >= 4,
            "expected at least 4 checkpoints for nested child contexts, got {}",
            captured.len()
        );

        // First: outer Context/START
        assert_eq!(captured[0].updates[0].sub_type(), Some("Context"));
        assert_eq!(captured[0].updates[0].action(), &OperationAction::Start);

        // Last: outer Context/SUCCEED
        let last = &captured[captured.len() - 1];
        assert_eq!(last.updates[0].sub_type(), Some("Context"));
        assert_eq!(last.updates[0].action(), &OperationAction::Succeed);
    }

    #[tokio::test]
    async fn test_child_context_replay_failed_status() {
        let op_id = first_op_id();

        // Create a FAILED child context operation
        let child_op = Operation::builder()
            .id(&op_id)
            .r#type(OperationType::Context)
            .status(OperationStatus::Failed)
            .start_timestamp(DateTime::from_secs(0))
            .context_details(
                ContextDetails::builder()
                    .replay_children(false)
                    .error(
                        ErrorObject::builder()
                            .error_type("RuntimeError")
                            .error_data("something went wrong")
                            .build(),
                    )
                    .build(),
            )
            .build()
            .unwrap();

        let (backend, calls) = ChildContextMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![child_op],
            None,
        )
        .await
        .unwrap();

        let result: Result<i32, DurableError> = ctx
            .child_context("sub_workflow", |_child_ctx| async move {
                panic!("closure should not execute during replay of failed context")
            })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("child context failed"),
            "error should mention child context failed, got: {err}"
        );
        assert!(
            err.contains("RuntimeError"),
            "error should contain error type, got: {err}"
        );
        assert!(
            err.contains("something went wrong"),
            "error should contain error data, got: {err}"
        );

        // No checkpoints during replay
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0);
    }

    // ─── span tests (FEAT-17, FEAT-18) ────────────────────────────────────

    #[traced_test]
    #[tokio::test]
    async fn test_child_context_emits_span() {
        let (backend, _calls) = ChildContextMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();
        let _ = ctx
            .child_context("sub", |_child| async move { Ok::<i32, DurableError>(1) })
            .await;
        assert!(logs_contain("durable_operation"));
        assert!(logs_contain("sub"));
        assert!(logs_contain("child_context"));
    }

    #[traced_test]
    #[tokio::test]
    async fn test_child_context_span_hierarchy() {
        let (backend, _calls) = ChildContextMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();
        let _ = ctx
            .child_context("parent_flow", |mut child| async move {
                let _: Result<i32, String> = child.step("inner_step", || async { Ok(42) }).await?;
                Ok::<_, DurableError>(1)
            })
            .await;
        assert!(logs_contain("child_context"));
        assert!(logs_contain("parent_flow"));
        assert!(logs_contain("inner_step"));
        assert!(logs_contain("step"));
    }
}
