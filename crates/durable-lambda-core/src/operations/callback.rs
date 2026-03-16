//! Callback operation — external signal coordination.
//!
//! Implement FR14-FR16: register callback, suspend until signal,
//! handle success/failure/heartbeat signals.
//!
//! The callback operation is a **two-phase** operation:
//! 1. [`DurableContext::create_callback`] — sends a START checkpoint and
//!    returns a [`CallbackHandle`] with the server-generated `callback_id`.
//! 2. [`DurableContext::callback_result`] — checks if the callback has been
//!    signaled and returns the result or suspends.

use aws_sdk_lambda::types::{OperationAction, OperationStatus, OperationType, OperationUpdate};
use serde::de::DeserializeOwned;

use crate::context::DurableContext;
use crate::error::DurableError;
use crate::types::{CallbackHandle, CallbackOptions};

impl DurableContext {
    /// Register a callback and return a handle with the server-generated callback ID.
    ///
    /// During execution mode, sends a START checkpoint with callback configuration
    /// and returns a [`CallbackHandle`] containing the `callback_id` that external
    /// systems use to signal completion via `SendDurableExecutionCallbackSuccess`,
    /// `SendDurableExecutionCallbackFailure`, or `SendDurableExecutionCallbackHeartbeat`.
    ///
    /// During replay mode, extracts the cached `callback_id` from history without
    /// sending any checkpoint.
    ///
    /// **Important:** This method NEVER suspends. Suspension happens in
    /// [`callback_result`](Self::callback_result) when the callback hasn't
    /// been signaled yet.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the callback operation
    /// * `options` — Timeout configuration (see [`CallbackOptions`])
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::CheckpointFailed`] if the AWS checkpoint API
    /// call fails or if the callback_id cannot be extracted from the response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_core::types::CallbackOptions;
    ///
    /// let handle = ctx.create_callback("approval", CallbackOptions::new()).await?;
    /// println!("Callback ID for external system: {}", handle.callback_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_callback(
        &mut self,
        name: &str,
        options: CallbackOptions,
    ) -> Result<CallbackHandle, DurableError> {
        let op_id = self.replay_engine_mut().generate_operation_id();

        // Check if operation exists in history (any status — not just completed).
        if let Some(op) = self.replay_engine().get_operation(&op_id) {
            let callback_id = op
                .callback_details()
                .and_then(|d| d.callback_id())
                .ok_or_else(|| {
                    DurableError::checkpoint_failed(
                        name,
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "callback_details missing callback_id in history",
                        ),
                    )
                })?
                .to_string();

            self.replay_engine_mut().track_replay(&op_id);
            return Ok(CallbackHandle {
                callback_id,
                operation_id: op_id,
            });
        }

        // Execute path — send START checkpoint with CallbackOptions.
        let callback_opts = aws_sdk_lambda::types::CallbackOptions::builder()
            .timeout_seconds(options.get_timeout_seconds())
            .heartbeat_timeout_seconds(options.get_heartbeat_timeout_seconds())
            .build();

        let start_update = OperationUpdate::builder()
            .id(op_id.clone())
            .r#type(OperationType::Callback)
            .action(OperationAction::Start)
            .sub_type("Callback")
            .name(name)
            .callback_options(callback_opts)
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

        // Extract callback_id from the merged operation's callback_details.
        let callback_id = self
            .replay_engine()
            .get_operation(&op_id)
            .and_then(|op| op.callback_details())
            .and_then(|d| d.callback_id())
            .ok_or_else(|| {
                DurableError::checkpoint_failed(
                    name,
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "no callback_id in checkpoint response",
                    ),
                )
            })?
            .to_string();

        self.replay_engine_mut().track_replay(&op_id);

        Ok(CallbackHandle {
            callback_id,
            operation_id: op_id,
        })
    }

    /// Check the result of a previously created callback.
    ///
    /// Return the deserialized success payload if the callback has been
    /// signaled with success. Return an error if the callback failed,
    /// timed out, or hasn't been signaled yet.
    ///
    /// **Important:** This is NOT an async/durable operation — it only reads
    /// the current operation state. It does NOT generate an operation ID or
    /// create checkpoints.
    ///
    /// # Arguments
    ///
    /// * `handle` — The [`CallbackHandle`] returned by [`create_callback`](Self::create_callback)
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::CallbackSuspended`] if the callback has not
    /// been signaled yet (the handler should propagate this to exit).
    /// Returns [`DurableError::CallbackFailed`] if the callback was signaled
    /// with failure, was cancelled, or timed out.
    /// Returns [`DurableError::Deserialization`] if the callback result
    /// cannot be deserialized as type `T`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_core::types::CallbackOptions;
    ///
    /// let handle = ctx.create_callback("approval", CallbackOptions::new()).await?;
    /// // ... pass handle.callback_id to external system ...
    /// let result: String = ctx.callback_result(&handle)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn callback_result<T: DeserializeOwned>(
        &self,
        handle: &CallbackHandle,
    ) -> Result<T, DurableError> {
        let Some(op) = self.replay_engine().get_operation(&handle.operation_id) else {
            // Operation not found — shouldn't happen if create_callback was called,
            // but treat as suspended to be safe.
            return Err(DurableError::callback_suspended(
                "unknown",
                &handle.callback_id,
            ));
        };

        match &op.status {
            OperationStatus::Succeeded => {
                let result_str =
                    op.callback_details()
                        .and_then(|d| d.result())
                        .ok_or_else(|| {
                            DurableError::checkpoint_failed(
                                op.name().unwrap_or("callback"),
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "callback succeeded but no result in callback_details",
                                ),
                            )
                        })?;

                serde_json::from_str(result_str)
                    .map_err(|e| DurableError::deserialization(std::any::type_name::<T>(), e))
            }
            OperationStatus::Failed
            | OperationStatus::Cancelled
            | OperationStatus::TimedOut
            | OperationStatus::Stopped => {
                let error_message = op
                    .callback_details()
                    .and_then(|d| d.error())
                    .map(|e| {
                        format!(
                            "{}: {}",
                            e.error_type().unwrap_or("Unknown"),
                            e.error_data().unwrap_or("")
                        )
                    })
                    .unwrap_or_else(|| "callback failed".to_string());

                Err(DurableError::callback_failed(
                    op.name().unwrap_or("callback"),
                    &handle.callback_id,
                    error_message,
                ))
            }
            // Started, Pending, Ready, or any other status — not yet signaled.
            _ => Err(DurableError::callback_suspended(
                op.name().unwrap_or("callback"),
                &handle.callback_id,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
    use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
    use aws_sdk_lambda::types::{
        CallbackDetails, ErrorObject, Operation, OperationAction, OperationStatus, OperationType,
        OperationUpdate,
    };
    use aws_smithy_types::DateTime;
    use tokio::sync::Mutex;

    use crate::backend::DurableBackend;
    use crate::context::DurableContext;
    use crate::error::DurableError;
    use crate::types::CallbackOptions;

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct CheckpointCall {
        arn: String,
        checkpoint_token: String,
        updates: Vec<OperationUpdate>,
    }

    /// MockBackend that returns an operation with callback_details in new_execution_state.
    struct CallbackMockBackend {
        calls: Arc<Mutex<Vec<CheckpointCall>>>,
        checkpoint_token: String,
        /// The operation to return in new_execution_state after checkpoint.
        response_operation: Option<Operation>,
    }

    impl CallbackMockBackend {
        fn new(
            checkpoint_token: &str,
            response_op: Operation,
        ) -> (Self, Arc<Mutex<Vec<CheckpointCall>>>) {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let backend = Self {
                calls: calls.clone(),
                checkpoint_token: checkpoint_token.to_string(),
                response_operation: Some(response_op),
            };
            (backend, calls)
        }
    }

    #[async_trait::async_trait]
    impl DurableBackend for CallbackMockBackend {
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

            let mut builder = CheckpointDurableExecutionOutput::builder()
                .checkpoint_token(&self.checkpoint_token);

            if let Some(ref op) = self.response_operation {
                let new_state = aws_sdk_lambda::types::CheckpointUpdatedExecutionState::builder()
                    .operations(op.clone())
                    .build();
                builder = builder.new_execution_state(new_state);
            }

            Ok(builder.build())
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

    /// Pre-compute the first operation ID that will be generated.
    fn first_op_id() -> String {
        let mut gen = crate::operation_id::OperationIdGenerator::new(None);
        gen.next_id()
    }

    fn make_callback_op(
        id: &str,
        status: OperationStatus,
        callback_id: &str,
        result: Option<&str>,
        error: Option<ErrorObject>,
    ) -> Operation {
        let mut cb_builder = CallbackDetails::builder().callback_id(callback_id);
        if let Some(r) = result {
            cb_builder = cb_builder.result(r);
        }
        if let Some(e) = error {
            cb_builder = cb_builder.error(e);
        }

        Operation::builder()
            .id(id)
            .r#type(OperationType::Callback)
            .status(status)
            .name("test_callback")
            .start_timestamp(DateTime::from_secs(0))
            .callback_details(cb_builder.build())
            .build()
            .unwrap()
    }

    // ─── create_callback tests ───────────────────────────────────────────

    #[tokio::test]
    async fn test_create_callback_sends_start_checkpoint_and_returns_handle() {
        let op_id = first_op_id();

        // Mock returns an operation with callback_details containing the callback_id.
        let response_op = make_callback_op(
            &op_id,
            OperationStatus::Started,
            "cb-server-123",
            None,
            None,
        );

        let (backend, calls) = CallbackMockBackend::new("new-token", response_op);
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let handle = ctx
            .create_callback("approval", CallbackOptions::new().timeout_seconds(300))
            .await
            .unwrap();

        // Verify the handle contains the server-generated callback_id.
        assert_eq!(handle.callback_id, "cb-server-123");

        // Verify START checkpoint was sent.
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 1, "expected exactly 1 checkpoint (START)");

        let update = &captured[0].updates[0];
        assert_eq!(update.r#type(), &OperationType::Callback);
        assert_eq!(update.action(), &OperationAction::Start);
        assert_eq!(update.name(), Some("approval"));
        assert_eq!(update.sub_type(), Some("Callback"));

        // Verify CallbackOptions with timeout.
        let callback_opts = update
            .callback_options()
            .expect("should have callback_options");
        assert_eq!(callback_opts.timeout_seconds(), 300);
        assert_eq!(callback_opts.heartbeat_timeout_seconds(), 0);
    }

    #[tokio::test]
    async fn test_create_callback_replays_from_history() {
        let op_id = first_op_id();

        // Operation in history with SUCCEEDED status and callback_details.
        let callback_op = make_callback_op(
            &op_id,
            OperationStatus::Succeeded,
            "cb-cached-456",
            Some(r#""approved""#),
            None,
        );

        // Use a backend that should NOT be called for checkpoints.
        let response_op = make_callback_op(&op_id, OperationStatus::Started, "unused", None, None);
        let (backend, calls) = CallbackMockBackend::new("token", response_op);

        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![callback_op],
            None,
        )
        .await
        .unwrap();

        let handle = ctx
            .create_callback("approval", CallbackOptions::new())
            .await
            .unwrap();

        // Should return the cached callback_id.
        assert_eq!(handle.callback_id, "cb-cached-456");

        // No checkpoints during replay.
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0, "no checkpoints during replay");
    }

    // ─── callback_result tests ───────────────────────────────────────────

    #[tokio::test]
    async fn test_callback_result_returns_deserialized_value_on_succeeded() {
        let op_id = first_op_id();

        let callback_op = make_callback_op(
            &op_id,
            OperationStatus::Succeeded,
            "cb-789",
            Some(r#"{"status":"approved","approver":"alice"}"#),
            None,
        );

        let response_op = make_callback_op(&op_id, OperationStatus::Started, "unused", None, None);
        let (backend, _) = CallbackMockBackend::new("token", response_op);

        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![callback_op],
            None,
        )
        .await
        .unwrap();

        // First create_callback to replay and get the handle.
        let handle = ctx
            .create_callback("approval", CallbackOptions::new())
            .await
            .unwrap();

        // Now check the result.
        let result: serde_json::Value = ctx.callback_result(&handle).unwrap();
        assert_eq!(result["status"], "approved");
        assert_eq!(result["approver"], "alice");
    }

    #[tokio::test]
    async fn test_callback_result_returns_error_on_failed() {
        let op_id = first_op_id();

        let error_obj = ErrorObject::builder()
            .error_type("RejectionError")
            .error_data("reviewer declined the request")
            .build();

        let callback_op = make_callback_op(
            &op_id,
            OperationStatus::Failed,
            "cb-fail-1",
            None,
            Some(error_obj),
        );

        let response_op = make_callback_op(&op_id, OperationStatus::Started, "unused", None, None);
        let (backend, _) = CallbackMockBackend::new("token", response_op);

        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![callback_op],
            None,
        )
        .await
        .unwrap();

        let handle = ctx
            .create_callback("approval", CallbackOptions::new())
            .await
            .unwrap();

        let err = ctx.callback_result::<String>(&handle).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("callback failed"), "error: {msg}");
        assert!(
            msg.contains("cb-fail-1"),
            "should contain callback_id: {msg}"
        );
        assert!(
            msg.contains("RejectionError"),
            "should contain error type: {msg}"
        );
    }

    #[tokio::test]
    async fn test_callback_result_returns_error_on_timed_out() {
        let op_id = first_op_id();

        let callback_op = make_callback_op(
            &op_id,
            OperationStatus::TimedOut,
            "cb-timeout-1",
            None,
            None,
        );

        let response_op = make_callback_op(&op_id, OperationStatus::Started, "unused", None, None);
        let (backend, _) = CallbackMockBackend::new("token", response_op);

        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![callback_op],
            None,
        )
        .await
        .unwrap();

        let handle = ctx
            .create_callback("approval", CallbackOptions::new())
            .await
            .unwrap();

        let err = ctx.callback_result::<String>(&handle).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("callback failed"), "error: {msg}");
        assert!(
            msg.contains("cb-timeout-1"),
            "should contain callback_id: {msg}"
        );
    }

    #[tokio::test]
    async fn test_callback_result_suspends_on_started() {
        let op_id = first_op_id();

        // Operation in STARTED status — callback not yet signaled.
        let callback_op =
            make_callback_op(&op_id, OperationStatus::Started, "cb-pending-1", None, None);

        let response_op = make_callback_op(&op_id, OperationStatus::Started, "unused", None, None);
        let (backend, _) = CallbackMockBackend::new("token", response_op);

        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![callback_op],
            None,
        )
        .await
        .unwrap();

        let handle = ctx
            .create_callback("approval", CallbackOptions::new())
            .await
            .unwrap();

        let err = ctx.callback_result::<String>(&handle).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("callback suspended"), "error: {msg}");
        assert!(
            msg.contains("cb-pending-1"),
            "should contain callback_id: {msg}"
        );
    }
}
