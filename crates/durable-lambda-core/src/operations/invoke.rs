//! Invoke operation — durable Lambda-to-Lambda invocation.
//!
//! Implement FR17-FR18: invoke target function, checkpoint result for replay.
//!
//! The invoke operation sends a **single START checkpoint** with the serialized
//! payload and target function name. The server invokes the target Lambda
//! asynchronously and transitions the operation to SUCCEEDED/FAILED when done.
//! The wire type is `ChainedInvoke` (matching the Python SDK).

use aws_sdk_lambda::types::{OperationAction, OperationStatus, OperationType, OperationUpdate};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::context::DurableContext;
use crate::error::DurableError;

impl DurableContext {
    /// Durably invoke another Lambda function and return its result.
    ///
    /// During execution mode, serializes the payload, sends a START checkpoint
    /// with the target function name, and returns [`DurableError::InvokeSuspended`]
    /// to signal the function should exit. The server invokes the target function
    /// asynchronously and re-invokes this Lambda when complete.
    ///
    /// During replay mode, returns the cached result without re-invoking the
    /// target function.
    ///
    /// If the target function completes immediately (detected via the
    /// double-check pattern), the result is returned directly without
    /// suspending.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the invoke operation
    /// * `function_name` — Name or ARN of the target Lambda function
    /// * `payload` — Input payload to send to the target function
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::InvokeSuspended`] when the invoke has been
    /// checkpointed and the target is still executing — the handler must
    /// propagate this to exit.
    /// Returns [`DurableError::InvokeFailed`] if the target function failed,
    /// timed out, or was stopped.
    /// Returns [`DurableError::Serialization`] if the payload cannot be
    /// serialized.
    /// Returns [`DurableError::Deserialization`] if the result cannot be
    /// deserialized as type `T`.
    /// Returns [`DurableError::CheckpointFailed`] if the AWS checkpoint API
    /// call fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// let result: String = ctx.invoke(
    ///     "call_processor",
    ///     "payment-processor-lambda",
    ///     &serde_json::json!({"order_id": 123}),
    /// ).await?;
    /// println!("Target returned: {result}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invoke<T, P>(
        &mut self,
        name: &str,
        function_name: &str,
        payload: &P,
    ) -> Result<T, DurableError>
    where
        T: DeserializeOwned,
        P: Serialize,
    {
        let op_id = self.replay_engine_mut().generate_operation_id();

        // Replay path: check for completed result (SUCCEEDED/FAILED/TIMED_OUT/etc).
        if let Some(op) = self.replay_engine().check_result(&op_id) {
            match &op.status {
                OperationStatus::Succeeded => {
                    let result = Self::deserialize_invoke_result::<T>(op, name)?;
                    self.replay_engine_mut().track_replay(&op_id);
                    return Ok(result);
                }
                _ => {
                    // Failed/Cancelled/TimedOut/Stopped — completed but not successful.
                    let error_message = Self::extract_invoke_error(op);
                    return Err(DurableError::invoke_failed(name, error_message));
                }
            }
        }

        // Check for non-completed status (STARTED/PENDING — target still running).
        if self.replay_engine().get_operation(&op_id).is_some() {
            return Err(DurableError::invoke_suspended(name));
        }

        // Execute path — serialize payload and send START checkpoint.
        let serialized_payload = serde_json::to_string(payload)
            .map_err(|e| DurableError::serialization(std::any::type_name::<P>(), e))?;

        let invoke_opts = aws_sdk_lambda::types::ChainedInvokeOptions::builder()
            .function_name(function_name)
            .build()
            .map_err(|e| DurableError::checkpoint_failed(name, e))?;

        let start_update = OperationUpdate::builder()
            .id(op_id.clone())
            .r#type(OperationType::ChainedInvoke)
            .action(OperationAction::Start)
            .sub_type("ChainedInvoke")
            .name(name)
            .payload(serialized_payload)
            .chained_invoke_options(invoke_opts)
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

        // Double-check: detect immediate completion.
        if let Some(op) = self.replay_engine().check_result(&op_id) {
            match &op.status {
                OperationStatus::Succeeded => {
                    let result = Self::deserialize_invoke_result::<T>(op, name)?;
                    self.replay_engine_mut().track_replay(&op_id);
                    return Ok(result);
                }
                _ => {
                    let error_message = Self::extract_invoke_error(op);
                    return Err(DurableError::invoke_failed(name, error_message));
                }
            }
        }

        // Target still executing — suspend.
        Err(DurableError::invoke_suspended(name))
    }

    /// Deserialize the result from a succeeded invoke operation.
    fn deserialize_invoke_result<T: DeserializeOwned>(
        op: &aws_sdk_lambda::types::Operation,
        name: &str,
    ) -> Result<T, DurableError> {
        let result_str = op
            .chained_invoke_details()
            .and_then(|d| d.result())
            .ok_or_else(|| {
                DurableError::checkpoint_failed(
                    name,
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invoke succeeded but no result in chained_invoke_details",
                    ),
                )
            })?;

        serde_json::from_str(result_str)
            .map_err(|e| DurableError::deserialization(std::any::type_name::<T>(), e))
    }

    /// Extract error message from an invoke operation's chained_invoke_details.
    fn extract_invoke_error(op: &aws_sdk_lambda::types::Operation) -> String {
        op.chained_invoke_details()
            .and_then(|d| d.error())
            .map(|e| {
                format!(
                    "{}: {}",
                    e.error_type().unwrap_or("Unknown"),
                    e.error_data().unwrap_or("")
                )
            })
            .unwrap_or_else(|| "invoke failed".to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
    use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
    use aws_sdk_lambda::types::{
        ChainedInvokeDetails, ErrorObject, Operation, OperationAction, OperationStatus,
        OperationType, OperationUpdate,
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

    /// MockBackend for invoke tests. Optionally returns an operation in new_execution_state.
    struct InvokeMockBackend {
        calls: Arc<Mutex<Vec<CheckpointCall>>>,
        checkpoint_token: String,
        response_operation: Option<Operation>,
    }

    impl InvokeMockBackend {
        fn new(
            checkpoint_token: &str,
            response_op: Option<Operation>,
        ) -> (Self, Arc<Mutex<Vec<CheckpointCall>>>) {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let backend = Self {
                calls: calls.clone(),
                checkpoint_token: checkpoint_token.to_string(),
                response_operation: response_op,
            };
            (backend, calls)
        }
    }

    #[async_trait::async_trait]
    impl DurableBackend for InvokeMockBackend {
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

    fn first_op_id() -> String {
        let mut gen = crate::operation_id::OperationIdGenerator::new(None);
        gen.next_id()
    }

    fn make_invoke_op(
        id: &str,
        status: OperationStatus,
        result: Option<&str>,
        error: Option<ErrorObject>,
    ) -> Operation {
        let mut details_builder = ChainedInvokeDetails::builder();
        if let Some(r) = result {
            details_builder = details_builder.result(r);
        }
        if let Some(e) = error {
            details_builder = details_builder.error(e);
        }

        Operation::builder()
            .id(id)
            .r#type(OperationType::ChainedInvoke)
            .status(status)
            .name("test_invoke")
            .start_timestamp(DateTime::from_secs(0))
            .chained_invoke_details(details_builder.build())
            .build()
            .unwrap()
    }

    // ─── invoke tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_invoke_sends_start_checkpoint_and_suspends() {
        // No response operation → target still executing → should suspend.
        let (backend, calls) = InvokeMockBackend::new("new-token", None);
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let result = ctx
            .invoke::<String, _>(
                "call_processor",
                "target-lambda",
                &serde_json::json!({"id": 42}),
            )
            .await;

        // Should return InvokeSuspended.
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invoke suspended"), "error: {msg}");
        assert!(msg.contains("call_processor"), "error: {msg}");

        // Verify START checkpoint was sent.
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 1, "expected exactly 1 checkpoint (START)");

        let update = &captured[0].updates[0];
        assert_eq!(update.r#type(), &OperationType::ChainedInvoke);
        assert_eq!(update.action(), &OperationAction::Start);
        assert_eq!(update.name(), Some("call_processor"));
        assert_eq!(update.sub_type(), Some("ChainedInvoke"));

        // Verify payload is set.
        let payload = update.payload().expect("should have payload");
        assert!(
            payload.contains("42"),
            "payload should contain id: {payload}"
        );

        // Verify ChainedInvokeOptions with function_name.
        let invoke_opts = update
            .chained_invoke_options()
            .expect("should have chained_invoke_options");
        assert_eq!(invoke_opts.function_name(), "target-lambda");
    }

    #[tokio::test]
    async fn test_invoke_replays_succeeded_result() {
        let op_id = first_op_id();

        let invoke_op = make_invoke_op(
            &op_id,
            OperationStatus::Succeeded,
            Some(r#"{"status":"processed","amount":100}"#),
            None,
        );

        let (backend, calls) = InvokeMockBackend::new("token", None);
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![invoke_op],
            None,
        )
        .await
        .unwrap();

        let result: serde_json::Value = ctx
            .invoke("call_processor", "target-lambda", &serde_json::json!({}))
            .await
            .unwrap();

        assert_eq!(result["status"], "processed");
        assert_eq!(result["amount"], 100);

        // No checkpoints during replay.
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0, "no checkpoints during replay");
    }

    #[tokio::test]
    async fn test_invoke_returns_error_on_failed() {
        let op_id = first_op_id();

        let error_obj = ErrorObject::builder()
            .error_type("TargetError")
            .error_data("target function crashed")
            .build();

        let invoke_op = make_invoke_op(&op_id, OperationStatus::Failed, None, Some(error_obj));

        let (backend, _) = InvokeMockBackend::new("token", None);
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![invoke_op],
            None,
        )
        .await
        .unwrap();

        let err = ctx
            .invoke::<String, _>("call_processor", "target-lambda", &serde_json::json!({}))
            .await
            .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("invoke failed"), "error: {msg}");
        assert!(msg.contains("TargetError"), "error: {msg}");
        assert!(msg.contains("target function crashed"), "error: {msg}");
    }

    #[tokio::test]
    async fn test_invoke_suspends_on_started() {
        let op_id = first_op_id();

        // Operation in STARTED status — target still running.
        let invoke_op = make_invoke_op(&op_id, OperationStatus::Started, None, None);

        let (backend, _) = InvokeMockBackend::new("token", None);
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![invoke_op],
            None,
        )
        .await
        .unwrap();

        let err = ctx
            .invoke::<String, _>("call_processor", "target-lambda", &serde_json::json!({}))
            .await
            .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("invoke suspended"), "error: {msg}");
    }

    #[tokio::test]
    async fn test_invoke_double_check_immediate_completion() {
        let op_id = first_op_id();

        // MockBackend returns SUCCEEDED operation in new_execution_state.
        let completed_op = make_invoke_op(
            &op_id,
            OperationStatus::Succeeded,
            Some(r#""instant-result""#),
            None,
        );

        let (backend, calls) = InvokeMockBackend::new("new-token", Some(completed_op));
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // Should return Ok because double-check detects immediate completion.
        let result: String = ctx
            .invoke("call_processor", "target-lambda", &serde_json::json!({}))
            .await
            .unwrap();

        assert_eq!(result, "instant-result");

        // START checkpoint was still sent.
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 1, "START checkpoint sent");
    }
}
