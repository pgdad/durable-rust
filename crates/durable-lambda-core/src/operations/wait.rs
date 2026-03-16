//! Wait operation — time-based suspension.
//!
//! Implement FR12-FR13: suspend for specified duration, resume after elapsed.
//!
//! The wait operation uses a **single START checkpoint** with `WaitOptions`.
//! The server handles the timer and transitions the operation to SUCCEEDED.
//! On re-invocation, the replay engine finds the completed wait and skips it.

use aws_sdk_lambda::types::{OperationAction, OperationType, OperationUpdate};

use crate::context::DurableContext;
use crate::error::DurableError;

impl DurableContext {
    /// Suspend execution for the specified duration.
    ///
    /// During execution mode, sends a START checkpoint with the wait duration
    /// and returns [`DurableError::WaitSuspended`] to signal the function
    /// should exit. The durable execution server re-invokes the Lambda after
    /// the duration elapses.
    ///
    /// During replay mode, returns `Ok(())` immediately if the wait has
    /// already completed (status SUCCEEDED in history).
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the wait operation
    /// * `duration_secs` — Duration to wait in seconds (1 to 31,622,400)
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::WaitSuspended`] when the wait has been
    /// checkpointed — the handler must propagate this to exit the function.
    /// Returns [`DurableError::CheckpointFailed`] if the AWS checkpoint
    /// API call fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// // Wait 30 seconds before continuing.
    /// ctx.wait("cooldown", 30).await?;
    ///
    /// // Execution continues here after the wait completes.
    /// println!("Wait completed!");
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::await_holding_lock)]
    pub async fn wait(&mut self, name: &str, duration_secs: i32) -> Result<(), DurableError> {
        let op_id = self.replay_engine_mut().generate_operation_id();

        let span = tracing::info_span!(
            "durable_operation",
            op.name = name,
            op.type = "wait",
            op.id = %op_id,
        );
        let _guard = span.enter();

        // Check if we have a completed result (replay path).
        if self.replay_engine().check_result(&op_id).is_some() {
            self.replay_engine_mut().track_replay(&op_id);
            return Ok(());
        }

        // Execute path — send START checkpoint with WaitOptions.
        let wait_opts = aws_sdk_lambda::types::WaitOptions::builder()
            .wait_seconds(duration_secs)
            .build();

        let start_update = OperationUpdate::builder()
            .id(op_id.clone())
            .r#type(OperationType::Wait)
            .action(OperationAction::Start)
            .sub_type("Wait")
            .name(name)
            .wait_options(wait_opts)
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

        // Merge any new execution state from checkpoint response.
        if let Some(new_state) = start_response.new_execution_state() {
            for op in new_state.operations() {
                self.replay_engine_mut()
                    .insert_operation(op.id().to_string(), op.clone());
            }
        }

        // Double-check: after START, re-check if operation already completed.
        if self.replay_engine().check_result(&op_id).is_some() {
            self.replay_engine_mut().track_replay(&op_id);
            return Ok(());
        }

        // Wait not yet completed — signal the handler to exit.
        Err(DurableError::wait_suspended(name))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
    use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
    use aws_sdk_lambda::types::{
        Operation, OperationAction, OperationStatus, OperationType, OperationUpdate,
    };
    use aws_smithy_types::DateTime;
    use tokio::sync::Mutex;

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

    struct MockBackend {
        calls: Arc<Mutex<Vec<CheckpointCall>>>,
        checkpoint_token: String,
    }

    impl MockBackend {
        fn new(checkpoint_token: &str) -> (Self, Arc<Mutex<Vec<CheckpointCall>>>) {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let backend = Self {
                calls: calls.clone(),
                checkpoint_token: checkpoint_token.to_string(),
            };
            (backend, calls)
        }
    }

    #[async_trait::async_trait]
    impl DurableBackend for MockBackend {
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
                .checkpoint_token(&self.checkpoint_token)
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

    #[tokio::test]
    async fn test_wait_sends_start_checkpoint_and_suspends() {
        let (backend, calls) = MockBackend::new("new-token");
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let result = ctx.wait("cooldown", 30).await;

        // Should return WaitSuspended error.
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("cooldown"),
            "error should contain operation name"
        );
        assert!(
            err.to_string().contains("wait suspended"),
            "error should indicate wait suspension"
        );

        // Verify START checkpoint was sent.
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 1, "expected exactly 1 checkpoint (START)");

        let update = &captured[0].updates[0];
        assert_eq!(update.r#type(), &OperationType::Wait);
        assert_eq!(update.action(), &OperationAction::Start);
        assert_eq!(update.name(), Some("cooldown"));
        assert_eq!(update.sub_type(), Some("Wait"));

        // Verify WaitOptions with duration.
        let wait_opts = update.wait_options().expect("should have wait_options");
        assert_eq!(wait_opts.wait_seconds(), Some(30));
    }

    #[tokio::test]
    async fn test_wait_replays_completed_wait() {
        // Create a completed wait operation in history.
        let op_id = {
            let mut gen = crate::operation_id::OperationIdGenerator::new(None);
            gen.next_id()
        };

        let wait_op = Operation::builder()
            .id(&op_id)
            .r#type(OperationType::Wait)
            .status(OperationStatus::Succeeded)
            .start_timestamp(DateTime::from_secs(0))
            .build()
            .unwrap();

        let (backend, calls) = MockBackend::new("token");
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![wait_op],
            None,
        )
        .await
        .unwrap();

        // Should replay successfully — no suspension.
        let result = ctx.wait("cooldown", 30).await;
        assert!(result.is_ok(), "replay should return Ok(())");

        // No checkpoints during replay.
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0, "no checkpoints during replay");
    }

    #[tokio::test]
    async fn test_wait_double_check_after_start() {
        // MockBackend that returns a completed wait in new_execution_state after START.
        struct DoubleCheckBackend {
            calls: Arc<Mutex<Vec<CheckpointCall>>>,
            completed_op_id: String,
        }

        #[async_trait::async_trait]
        impl DurableBackend for DoubleCheckBackend {
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

                // Simulate server completing the wait synchronously.
                let completed_op = Operation::builder()
                    .id(&self.completed_op_id)
                    .r#type(OperationType::Wait)
                    .status(OperationStatus::Succeeded)
                    .start_timestamp(DateTime::from_secs(0))
                    .build()
                    .unwrap();

                let new_state = aws_sdk_lambda::types::CheckpointUpdatedExecutionState::builder()
                    .operations(completed_op)
                    .build();

                Ok(CheckpointDurableExecutionOutput::builder()
                    .checkpoint_token("new-token")
                    .new_execution_state(new_state)
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

        // Pre-compute the operation ID that will be generated.
        let op_id = {
            let mut gen = crate::operation_id::OperationIdGenerator::new(None);
            gen.next_id()
        };

        let calls = Arc::new(Mutex::new(Vec::new()));
        let backend = DoubleCheckBackend {
            calls: calls.clone(),
            completed_op_id: op_id,
        };

        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // Should return Ok(()) because the server completed the wait during START.
        let result = ctx.wait("fast_wait", 1).await;
        assert!(
            result.is_ok(),
            "double-check should detect completion and return Ok(())"
        );

        // START checkpoint was still sent.
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 1, "START checkpoint sent");
    }
}
