//! Step operation — checkpointed work units with optional retries.
//!
//! Implement FR8, FR9, FR10, FR11: named steps that execute a closure,
//! checkpoint the result, support server-side retries, and handle typed errors.
//!
//! This module follows the Python SDK's two-phase checkpoint pattern:
//! 1. Checkpoint START (sync)
//! 2. Execute closure
//! 3. Checkpoint SUCCEED, FAIL, or RETRY (sync)

use std::future::Future;
use std::time::Duration;

use aws_sdk_lambda::types::{
    ErrorObject, OperationAction, OperationStatus, OperationType, OperationUpdate,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::context::DurableContext;
use crate::error::DurableError;
use crate::types::StepOptions;

impl DurableContext {
    /// Execute a named step with checkpointing.
    ///
    /// During execution mode, runs the closure and checkpoints the result to AWS.
    /// During replay mode, returns the previously checkpointed result without
    /// executing the closure.
    ///
    /// This is a convenience wrapper around [`step_with_options`](Self::step_with_options)
    /// with default options (no retries).
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable step name, used as checkpoint metadata
    /// * `f` — Closure to execute (skipped during replay)
    ///
    /// # Returns
    ///
    /// Returns `Ok(Ok(T))` on successful step execution or replay.
    /// Returns `Ok(Err(E))` when the step closure returned an error (also checkpointed).
    /// Returns `Err(DurableError)` on SDK-level failures (checkpoint, serialization).
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::Serialization`] if the result cannot be serialized to JSON.
    /// Returns [`DurableError::Deserialization`] if a cached result cannot be deserialized.
    /// Returns [`DurableError::CheckpointFailed`] or [`DurableError::AwsSdkOperation`]
    /// if the AWS checkpoint API call fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// let result: Result<i32, String> = ctx.step("validate_order", || async {
    ///     Ok(42)
    /// }).await?;
    ///
    /// match result {
    ///     Ok(value) => println!("Step succeeded: {value}"),
    ///     Err(e) => println!("Step failed: {e}"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn step<T, E, F, Fut>(
        &mut self,
        name: &str,
        f: F,
    ) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        self.step_with_options(name, StepOptions::default(), f)
            .await
    }

    /// Execute a named step with checkpointing and retry configuration.
    ///
    /// During execution mode, runs the closure and checkpoints the result.
    /// If the closure fails and retries are configured, sends a RETRY checkpoint
    /// and returns [`DurableError::StepRetryScheduled`] to signal the function
    /// should exit. The server re-invokes the Lambda after the backoff delay.
    ///
    /// During replay mode, returns the previously checkpointed result without
    /// executing the closure.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable step name, used as checkpoint metadata
    /// * `options` — Retry configuration (see [`StepOptions`])
    /// * `f` — Closure to execute (skipped during replay)
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::StepRetryScheduled`] when a retry has been
    /// scheduled — the handler must propagate this to exit the function.
    /// Returns [`DurableError::Serialization`] if the result cannot be serialized.
    /// Returns [`DurableError::Deserialization`] if a cached result cannot be deserialized.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_core::types::StepOptions;
    ///
    /// let result: Result<i32, String> = ctx.step_with_options(
    ///     "charge_payment",
    ///     StepOptions::new().retries(3).backoff_seconds(5),
    ///     || async { Ok(100) },
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn step_with_options<T, E, F, Fut>(
        &mut self,
        name: &str,
        options: StepOptions,
        f: F,
    ) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        let op_id = self.replay_engine_mut().generate_operation_id();

        // Check if we have a completed result (replay path).
        if let Some(operation) = self.replay_engine().check_result(&op_id) {
            let result = extract_step_result::<T, E>(operation)?;
            self.replay_engine_mut().track_replay(&op_id);
            return Ok(result);
        }

        // Check if operation exists with non-completed status (retry re-execution).
        // PENDING/READY/STARTED mean the step was previously attempted but not yet
        // completed — the server re-invoked us to retry.
        let is_retry_reexecution =
            self.replay_engine()
                .operations()
                .get(&op_id)
                .is_some_and(|op| {
                    matches!(
                        op.status,
                        OperationStatus::Pending
                            | OperationStatus::Ready
                            | OperationStatus::Started
                    )
                });

        let current_attempt = if is_retry_reexecution {
            // Read attempt from existing operation's step details.
            self.replay_engine()
                .operations()
                .get(&op_id)
                .and_then(|op| op.step_details())
                .map(|d| d.attempt())
                .unwrap_or(1)
        } else {
            // First attempt — send START checkpoint.
            let start_update = OperationUpdate::builder()
                .id(op_id.clone())
                .r#type(OperationType::Step)
                .action(OperationAction::Start)
                .name(name)
                .sub_type("Step")
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

            // Double-check: after START, re-check if operation already has result.
            if let Some(operation) = self.replay_engine().check_result(&op_id) {
                let result = extract_step_result::<T, E>(operation)?;
                self.replay_engine_mut().track_replay(&op_id);
                return Ok(result);
            }

            1 // first attempt
        };

        // Execute the closure in a spawned task to catch panics.
        // tokio::spawn catches panics as JoinError, converting them to
        // DurableError::CheckpointFailed rather than unwinding through the caller.
        // When timeout_seconds is configured, wrap execution in tokio::time::timeout
        // and abort the task if the deadline is exceeded.
        let name_owned = name.to_string();
        let mut handle = tokio::spawn(async move { f().await });
        let user_result = if let Some(secs) = options.get_timeout_seconds() {
            match tokio::time::timeout(Duration::from_secs(secs), &mut handle).await {
                Ok(join_result) => join_result.map_err(|join_err| {
                    DurableError::checkpoint_failed(
                        &name_owned,
                        std::io::Error::other(format!("step closure panicked: {join_err}")),
                    )
                })?,
                Err(_elapsed) => {
                    handle.abort();
                    return Err(DurableError::step_timeout(&name_owned));
                }
            }
        } else {
            handle.await.map_err(|join_err| {
                DurableError::checkpoint_failed(
                    &name_owned,
                    std::io::Error::other(format!("step closure panicked: {join_err}")),
                )
            })?
        };

        // Checkpoint the result.
        match &user_result {
            Ok(value) => {
                let payload = serde_json::to_string(value)
                    .map_err(|e| DurableError::serialization(std::any::type_name::<T>(), e))?;

                let succeed_update = OperationUpdate::builder()
                    .id(op_id.clone())
                    .r#type(OperationType::Step)
                    .action(OperationAction::Succeed)
                    .name(name)
                    .sub_type("Step")
                    .payload(payload)
                    .build()
                    .map_err(|e| DurableError::checkpoint_failed(name, e))?;

                let response = self
                    .backend()
                    .checkpoint(
                        self.arn(),
                        self.checkpoint_token(),
                        vec![succeed_update],
                        None,
                    )
                    .await?;

                let new_token = response.checkpoint_token().ok_or_else(|| {
                    DurableError::checkpoint_failed(
                        name,
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "checkpoint response missing checkpoint_token",
                        ),
                    )
                })?;
                self.set_checkpoint_token(new_token.to_string());
            }
            Err(error) => {
                let max_retries = options.get_retries().unwrap_or(0);

                // Check retry predicate first — false means fail immediately without
                // consuming the retry budget (FEAT-14).
                let should_retry = if let Some(pred) = options.get_retry_if() {
                    pred(error as &dyn std::any::Any)
                } else {
                    true // no predicate — retry all errors (backward compatible)
                };

                if should_retry && (current_attempt as u32) <= max_retries {
                    // Retries remain — checkpoint RETRY and signal exit.
                    let delay = options.get_backoff_seconds().unwrap_or(0);
                    let aws_step_options = aws_sdk_lambda::types::StepOptions::builder()
                        .next_attempt_delay_seconds(delay)
                        .build();

                    let retry_update = OperationUpdate::builder()
                        .id(op_id.clone())
                        .r#type(OperationType::Step)
                        .action(OperationAction::Retry)
                        .name(name)
                        .sub_type("Step")
                        .step_options(aws_step_options)
                        .build()
                        .map_err(|e| DurableError::checkpoint_failed(name, e))?;

                    let response = self
                        .backend()
                        .checkpoint(
                            self.arn(),
                            self.checkpoint_token(),
                            vec![retry_update],
                            None,
                        )
                        .await?;

                    let new_token = response.checkpoint_token().ok_or_else(|| {
                        DurableError::checkpoint_failed(
                            name,
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "checkpoint response missing checkpoint_token",
                            ),
                        )
                    })?;
                    self.set_checkpoint_token(new_token.to_string());

                    return Err(DurableError::step_retry_scheduled(name));
                }

                // No retries left — checkpoint FAIL.
                let error_data = serde_json::to_string(error)
                    .map_err(|e| DurableError::serialization(std::any::type_name::<E>(), e))?;

                let error_object = ErrorObject::builder()
                    .error_type(std::any::type_name::<E>())
                    .error_data(error_data)
                    .build();

                let fail_update = OperationUpdate::builder()
                    .id(op_id.clone())
                    .r#type(OperationType::Step)
                    .action(OperationAction::Fail)
                    .name(name)
                    .sub_type("Step")
                    .error(error_object)
                    .build()
                    .map_err(|e| DurableError::checkpoint_failed(name, e))?;

                let response = self
                    .backend()
                    .checkpoint(self.arn(), self.checkpoint_token(), vec![fail_update], None)
                    .await?;

                let new_token = response.checkpoint_token().ok_or_else(|| {
                    DurableError::checkpoint_failed(
                        name,
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "checkpoint response missing checkpoint_token",
                        ),
                    )
                })?;
                self.set_checkpoint_token(new_token.to_string());
            }
        }

        Ok(user_result)
    }
}

/// Extract a step result from a completed Operation.
///
/// For SUCCEEDED operations, deserializes the result from `step_details.result`.
/// For FAILED operations, deserializes the error from `step_details.error.error_data`.
fn extract_step_result<T, E>(
    operation: &aws_sdk_lambda::types::Operation,
) -> Result<Result<T, E>, DurableError>
where
    T: DeserializeOwned,
    E: DeserializeOwned,
{
    match &operation.status {
        OperationStatus::Succeeded => {
            let result_json = operation
                .step_details()
                .and_then(|d| d.result())
                .ok_or_else(|| {
                    DurableError::checkpoint_failed(
                        "step",
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "SUCCEEDED operation missing step_details.result",
                        ),
                    )
                })?;

            let value: T = serde_json::from_str(result_json)
                .map_err(|e| DurableError::deserialization(std::any::type_name::<T>(), e))?;
            Ok(Ok(value))
        }
        OperationStatus::Failed => {
            let error_data = operation
                .step_details()
                .and_then(|d| d.error())
                .and_then(|e| e.error_data())
                .ok_or_else(|| {
                    DurableError::checkpoint_failed(
                        "step",
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "FAILED operation missing step_details.error.error_data",
                        ),
                    )
                })?;

            let error: E = serde_json::from_str(error_data)
                .map_err(|e| DurableError::deserialization(std::any::type_name::<E>(), e))?;
            Ok(Err(error))
        }
        other => Err(DurableError::replay_mismatch(
            "Succeeded or Failed",
            format!("{other:?}"),
            0,
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
    use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
    use aws_sdk_lambda::types::{
        ErrorObject, Operation, OperationStatus, OperationType, OperationUpdate, StepDetails,
    };
    use aws_smithy_types::DateTime;
    use serde::{Deserialize, Serialize};
    use tokio::sync::Mutex;

    use crate::backend::DurableBackend;
    use crate::context::DurableContext;
    use crate::error::DurableError;
    use crate::operation_id::OperationIdGenerator;
    use crate::types::StepOptions;

    /// Captured checkpoint call for test assertions.
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct CheckpointCall {
        arn: String,
        checkpoint_token: String,
        updates: Vec<OperationUpdate>,
    }

    /// Mock backend that records checkpoint calls and returns configurable responses.
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
    async fn test_step_executes_closure_in_executing_mode() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let result: Result<i32, String> = ctx.step("my_step", || async { Ok(42) }).await.unwrap();

        assert_eq!(result.unwrap(), 42);

        let captured = calls.lock().await;
        assert_eq!(captured.len(), 2, "expected START + SUCCEED checkpoints");

        // First call is START.
        let start_call = &captured[0];
        assert_eq!(start_call.updates.len(), 1);
        let start_update = &start_call.updates[0];
        assert_eq!(start_update.r#type(), &OperationType::Step);
        assert_eq!(
            start_update.action(),
            &aws_sdk_lambda::types::OperationAction::Start
        );
        assert_eq!(start_update.name(), Some("my_step"));

        // Second call is SUCCEED.
        let succeed_call = &captured[1];
        assert_eq!(succeed_call.updates.len(), 1);
        let succeed_update = &succeed_call.updates[0];
        assert_eq!(succeed_update.r#type(), &OperationType::Step);
        assert_eq!(
            succeed_update.action(),
            &aws_sdk_lambda::types::OperationAction::Succeed
        );
        assert_eq!(succeed_update.payload().unwrap(), "42");

        // Verify checkpoint token was updated (second call should use the token from first response).
        assert_eq!(succeed_call.checkpoint_token, "new-token");
    }

    #[tokio::test]
    async fn test_step_returns_cached_result_in_replaying_mode() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        // Pre-compute the operation ID that step() will generate.
        let mut gen = OperationIdGenerator::new(None);
        let expected_op_id = gen.next_id();

        let cached_op = Operation::builder()
            .id(&expected_op_id)
            .r#type(OperationType::Step)
            .status(OperationStatus::Succeeded)
            .start_timestamp(DateTime::from_secs(0))
            .step_details(
                StepDetails::builder()
                    .attempt(1)
                    .result(r#"{"value":42}"#)
                    .build(),
            )
            .build()
            .unwrap();

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![cached_op],
            None,
        )
        .await
        .unwrap();

        // Track whether closure is called.
        let closure_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let closure_called_clone = closure_called.clone();

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct MyResult {
            value: i32,
        }

        let result: Result<MyResult, String> = ctx
            .step("my_step", move || {
                let flag = closure_called_clone.clone();
                async move {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(MyResult { value: 999 })
                }
            })
            .await
            .unwrap();

        assert_eq!(result.unwrap(), MyResult { value: 42 });
        assert!(
            !closure_called.load(std::sync::atomic::Ordering::SeqCst),
            "closure should NOT have been called during replay"
        );

        // No checkpoint calls should have been made for a replay.
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0, "no checkpoint calls during replay");
    }

    #[tokio::test]
    async fn test_step_returns_cached_error_in_replaying_mode() {
        let (backend, _calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut gen = OperationIdGenerator::new(None);
        let expected_op_id = gen.next_id();

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct MyError {
            code: i32,
            message: String,
        }

        let error_data = serde_json::to_string(&MyError {
            code: 404,
            message: "not found".to_string(),
        })
        .unwrap();

        let cached_op = Operation::builder()
            .id(&expected_op_id)
            .r#type(OperationType::Step)
            .status(OperationStatus::Failed)
            .start_timestamp(DateTime::from_secs(0))
            .step_details(
                StepDetails::builder()
                    .attempt(1)
                    .error(
                        ErrorObject::builder()
                            .error_type("MyError")
                            .error_data(&error_data)
                            .build(),
                    )
                    .build(),
            )
            .build()
            .unwrap();

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![cached_op],
            None,
        )
        .await
        .unwrap();

        let result: Result<String, MyError> = ctx
            .step("my_step", || async { Ok("nope".to_string()) })
            .await
            .unwrap();

        let err = result.unwrap_err();
        assert_eq!(err.code, 404);
        assert_eq!(err.message, "not found");
    }

    #[tokio::test]
    async fn test_step_serialization_roundtrip() {
        let (backend, _calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
        struct ComplexData {
            name: String,
            values: Vec<i32>,
            nested: NestedData,
            optional: Option<String>,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
        struct NestedData {
            flag: bool,
            score: f64,
        }

        let expected = ComplexData {
            name: "test".to_string(),
            values: vec![1, 2, 3],
            nested: NestedData {
                flag: true,
                score: 99.5,
            },
            optional: Some("present".to_string()),
        };

        // Pre-compute the operation ID and create a cached operation with serialized data.
        let mut gen = OperationIdGenerator::new(None);
        let expected_op_id = gen.next_id();

        let serialized = serde_json::to_string(&expected).unwrap();

        let cached_op = Operation::builder()
            .id(&expected_op_id)
            .r#type(OperationType::Step)
            .status(OperationStatus::Succeeded)
            .start_timestamp(DateTime::from_secs(0))
            .step_details(
                StepDetails::builder()
                    .attempt(1)
                    .result(&serialized)
                    .build(),
            )
            .build()
            .unwrap();

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![cached_op],
            None,
        )
        .await
        .unwrap();

        let result: Result<ComplexData, String> = ctx
            .step("complex_step", || async {
                panic!("should not execute during replay")
            })
            .await
            .unwrap();

        assert_eq!(result.unwrap(), expected);
    }

    #[tokio::test]
    async fn test_step_sequential_unique_ids() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let _r1: Result<i32, String> = ctx.step("step_1", || async { Ok(1) }).await.unwrap();
        let _r2: Result<i32, String> = ctx.step("step_2", || async { Ok(2) }).await.unwrap();

        let captured = calls.lock().await;
        // 2 steps x 2 checkpoints each = 4 calls.
        assert_eq!(captured.len(), 4);

        // Extract operation IDs from the START updates of each step.
        let step1_id = captured[0].updates[0].id().to_string();
        let step2_id = captured[2].updates[0].id().to_string();

        assert_ne!(
            step1_id, step2_id,
            "sequential steps must have different operation IDs"
        );

        // Verify the START and SUCCEED of each step use the same ID.
        assert_eq!(step1_id, captured[1].updates[0].id());
        assert_eq!(step2_id, captured[3].updates[0].id());
    }

    #[tokio::test]
    async fn test_step_tracks_replay() {
        let (backend, _calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        // Pre-compute the operation ID for the single operation.
        let mut gen = OperationIdGenerator::new(None);
        let expected_op_id = gen.next_id();

        let cached_op = Operation::builder()
            .id(&expected_op_id)
            .r#type(OperationType::Step)
            .status(OperationStatus::Succeeded)
            .start_timestamp(DateTime::from_secs(0))
            .step_details(StepDetails::builder().attempt(1).result("100").build())
            .build()
            .unwrap();

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![cached_op],
            None,
        )
        .await
        .unwrap();

        // Before replay, the context should be in replaying mode.
        assert!(
            ctx.is_replaying(),
            "should be replaying before visiting cached ops"
        );

        let result: Result<i32, String> =
            ctx.step("cached_step", || async { Ok(999) }).await.unwrap();
        assert_eq!(result.unwrap(), 100);

        // After replaying the only cached operation, mode should transition to executing.
        assert!(
            !ctx.is_replaying(),
            "should transition to executing after all cached ops replayed"
        );
    }

    #[tokio::test]
    async fn test_step_with_options_basic_success() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let result: Result<i32, String> = ctx
            .step_with_options("opts_step", StepOptions::default(), || async { Ok(42) })
            .await
            .unwrap();

        assert_eq!(result.unwrap(), 42);

        let captured = calls.lock().await;
        assert_eq!(captured.len(), 2, "expected START + SUCCEED checkpoints");

        let start_update = &captured[0].updates[0];
        assert_eq!(start_update.r#type(), &OperationType::Step);
        assert_eq!(
            start_update.action(),
            &aws_sdk_lambda::types::OperationAction::Start
        );
        assert_eq!(start_update.name(), Some("opts_step"));

        let succeed_update = &captured[1].updates[0];
        assert_eq!(succeed_update.r#type(), &OperationType::Step);
        assert_eq!(
            succeed_update.action(),
            &aws_sdk_lambda::types::OperationAction::Succeed
        );
        assert_eq!(succeed_update.payload().unwrap(), "42");
    }

    #[tokio::test]
    async fn test_step_with_options_retry_on_failure() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let options = StepOptions::new().retries(3).backoff_seconds(5);
        let result: Result<Result<i32, String>, DurableError> = ctx
            .step_with_options("retry_step", options, || async {
                Err("transient failure".to_string())
            })
            .await;

        // Should return StepRetryScheduled error.
        let err = result.unwrap_err();
        match err {
            DurableError::StepRetryScheduled { .. } => {}
            other => panic!("expected StepRetryScheduled, got {other:?}"),
        }

        let captured = calls.lock().await;
        assert_eq!(captured.len(), 2, "expected START + RETRY checkpoints");

        // First call is START.
        let start_update = &captured[0].updates[0];
        assert_eq!(
            start_update.action(),
            &aws_sdk_lambda::types::OperationAction::Start
        );

        // Second call is RETRY.
        let retry_update = &captured[1].updates[0];
        assert_eq!(
            retry_update.action(),
            &aws_sdk_lambda::types::OperationAction::Retry
        );
        let step_opts = retry_update
            .step_options()
            .expect("should have step_options");
        assert_eq!(step_opts.next_attempt_delay_seconds(), Some(5));
    }

    #[tokio::test]
    async fn test_step_with_options_retry_exhaustion() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        // Pre-compute the operation ID.
        let mut gen = OperationIdGenerator::new(None);
        let expected_op_id = gen.next_id();

        // Simulate an operation already at attempt 4 (retries exhausted with retries(3)).
        let cached_op = Operation::builder()
            .id(&expected_op_id)
            .r#type(OperationType::Step)
            .status(OperationStatus::Pending)
            .start_timestamp(DateTime::from_secs(0))
            .step_details(StepDetails::builder().attempt(4).build())
            .build()
            .unwrap();

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![cached_op],
            None,
        )
        .await
        .unwrap();

        let options = StepOptions::new().retries(3).backoff_seconds(5);
        let result: Result<Result<i32, String>, DurableError> = ctx
            .step_with_options("exhaust_step", options, || async {
                Err("final failure".to_string())
            })
            .await;

        // Should return Ok(Err(user_error)) since retries are exhausted.
        let inner = result.unwrap();
        let user_error = inner.unwrap_err();
        assert_eq!(user_error, "final failure");

        // Only FAIL checkpoint sent (no START since operation already exists).
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 1, "expected only FAIL checkpoint");

        let fail_update = &captured[0].updates[0];
        assert_eq!(
            fail_update.action(),
            &aws_sdk_lambda::types::OperationAction::Fail
        );
    }

    #[tokio::test]
    async fn test_step_with_options_replay_succeeded_with_retries() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut gen = OperationIdGenerator::new(None);
        let expected_op_id = gen.next_id();

        // Pre-populate a SUCCEEDED operation (as if it succeeded after retries).
        let cached_op = Operation::builder()
            .id(&expected_op_id)
            .r#type(OperationType::Step)
            .status(OperationStatus::Succeeded)
            .start_timestamp(DateTime::from_secs(0))
            .step_details(StepDetails::builder().attempt(3).result("99").build())
            .build()
            .unwrap();

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![cached_op],
            None,
        )
        .await
        .unwrap();

        let closure_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let closure_called_clone = closure_called.clone();

        let options = StepOptions::new().retries(3);
        let result: Result<i32, String> = ctx
            .step_with_options("replay_retry_step", options, move || {
                let flag = closure_called_clone.clone();
                async move {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(999)
                }
            })
            .await
            .unwrap();

        assert_eq!(result.unwrap(), 99);
        assert!(
            !closure_called.load(std::sync::atomic::Ordering::SeqCst),
            "closure should NOT have been called during replay"
        );

        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0, "no checkpoint calls during replay");
    }

    #[tokio::test]
    async fn test_step_backward_compatibility() {
        let (backend, calls) = MockBackend::new("compat-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // Call `step` (not step_with_options) to verify backward compatibility.
        let result: Result<String, String> = ctx
            .step("compat_step", || async { Ok("hello".to_string()) })
            .await
            .unwrap();

        assert_eq!(result.unwrap(), "hello");

        let captured = calls.lock().await;
        assert_eq!(captured.len(), 2, "expected START + SUCCEED checkpoints");

        let start_update = &captured[0].updates[0];
        assert_eq!(
            start_update.action(),
            &aws_sdk_lambda::types::OperationAction::Start
        );
        assert_eq!(start_update.name(), Some("compat_step"));

        let succeed_update = &captured[1].updates[0];
        assert_eq!(
            succeed_update.action(),
            &aws_sdk_lambda::types::OperationAction::Succeed
        );
        assert_eq!(succeed_update.payload().unwrap(), r#""hello""#);
    }

    #[test]
    fn test_step_options_builder() {
        // Default has no retries and no backoff.
        let default_opts = StepOptions::default();
        assert_eq!(default_opts.get_retries(), None);
        assert_eq!(default_opts.get_backoff_seconds(), None);

        // new() should be equivalent to default().
        let new_opts = StepOptions::new();
        assert_eq!(new_opts.get_retries(), None);
        assert_eq!(new_opts.get_backoff_seconds(), None);

        // Builder methods.
        let opts = StepOptions::new().retries(5).backoff_seconds(10);
        assert_eq!(opts.get_retries(), Some(5));
        assert_eq!(opts.get_backoff_seconds(), Some(10));

        // Chaining overwrites previous values.
        let opts2 = StepOptions::new().retries(1).retries(3);
        assert_eq!(opts2.get_retries(), Some(3));
    }

    #[tokio::test]
    async fn test_step_with_options_typed_error_roundtrip() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum DomainError {
            NotFound { resource: String },
            PermissionDenied { user: String, action: String },
            RateLimited { retry_after_secs: u64 },
        }

        // Pre-compute the operation ID.
        let mut gen = OperationIdGenerator::new(None);
        let expected_op_id = gen.next_id();

        let original_error = DomainError::PermissionDenied {
            user: "alice".to_string(),
            action: "delete".to_string(),
        };
        let error_data = serde_json::to_string(&original_error).unwrap();

        // Pre-populate a FAILED operation with the serialized domain error.
        let cached_op = Operation::builder()
            .id(&expected_op_id)
            .r#type(OperationType::Step)
            .status(OperationStatus::Failed)
            .start_timestamp(DateTime::from_secs(0))
            .step_details(
                StepDetails::builder()
                    .attempt(1)
                    .error(
                        ErrorObject::builder()
                            .error_type("DomainError")
                            .error_data(&error_data)
                            .build(),
                    )
                    .build(),
            )
            .build()
            .unwrap();

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![cached_op],
            None,
        )
        .await
        .unwrap();

        let result: Result<String, DomainError> = ctx
            .step_with_options("typed_err_step", StepOptions::default(), || async {
                Ok("should not run".to_string())
            })
            .await
            .unwrap();

        let err = result.unwrap_err();
        assert_eq!(
            err,
            DomainError::PermissionDenied {
                user: "alice".to_string(),
                action: "delete".to_string(),
            }
        );

        // No checkpoint calls for replay.
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0, "no checkpoint calls during replay");
    }

    #[tokio::test]
    async fn test_step_execute_fail_checkpoint() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // Step closure returns Err with no retries — should checkpoint FAIL.
        let result: Result<i32, String> = ctx
            .step("failing_step", || async {
                Err("something went wrong".to_string())
            })
            .await
            .unwrap();

        assert_eq!(result.unwrap_err(), "something went wrong");

        let captured = calls.lock().await;
        assert_eq!(captured.len(), 2, "expected START + FAIL checkpoints");

        // First call is START.
        assert_eq!(
            captured[0].updates[0].action(),
            &aws_sdk_lambda::types::OperationAction::Start
        );

        // Second call is FAIL.
        assert_eq!(
            captured[1].updates[0].action(),
            &aws_sdk_lambda::types::OperationAction::Fail
        );
    }

    /// Mock backend that returns checkpoint responses WITHOUT a checkpoint_token,
    /// simulating an AWS API contract violation.
    struct NoneTokenMockBackend;

    #[async_trait::async_trait]
    impl DurableBackend for NoneTokenMockBackend {
        async fn checkpoint(
            &self,
            _arn: &str,
            _checkpoint_token: &str,
            _updates: Vec<OperationUpdate>,
            _client_token: Option<&str>,
        ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
            // Return a checkpoint response with NO checkpoint_token set
            Ok(CheckpointDurableExecutionOutput::builder().build())
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

    // --- Task 2 TDD RED: timeout and conditional retry integration ---

    #[tokio::test]
    async fn test_step_timeout_aborts_slow_closure() {
        let (backend, _calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let options = StepOptions::new().timeout_seconds(1);
        let result: Result<Result<i32, String>, DurableError> = ctx
            .step_with_options("slow_step", options, || async {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                Ok::<i32, String>(42)
            })
            .await;

        let err = result.unwrap_err();
        match err {
            DurableError::StepTimeout { operation_name } => {
                assert_eq!(operation_name, "slow_step");
            }
            other => panic!("expected StepTimeout, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_step_timeout_does_not_fire_when_fast_enough() {
        let (backend, _calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        let options = StepOptions::new().timeout_seconds(5);
        let result: Result<i32, String> = ctx
            .step_with_options("fast_step", options, || async { Ok(99) })
            .await
            .unwrap();

        assert_eq!(result.unwrap(), 99);
    }

    #[tokio::test]
    async fn test_retry_if_false_causes_immediate_fail_no_retry_budget_consumed() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // retry_if returns false — should skip retry and send FAIL checkpoint.
        let options = StepOptions::new().retries(3).retry_if(|_e: &String| false);

        let result: Result<Result<i32, String>, DurableError> = ctx
            .step_with_options("no_retry_step", options, || async {
                Err("permanent error".to_string())
            })
            .await;

        // Should return Ok(Err(user_error)) — FAIL, not retry.
        let inner = result.unwrap();
        let user_error = inner.unwrap_err();
        assert_eq!(user_error, "permanent error");

        let captured = calls.lock().await;
        // START + FAIL — no RETRY despite having 3 retries configured.
        assert_eq!(
            captured.len(),
            2,
            "expected START + FAIL, got {}",
            captured.len()
        );
        assert_eq!(
            captured[1].updates[0].action(),
            &aws_sdk_lambda::types::OperationAction::Fail,
            "second checkpoint should be FAIL not RETRY"
        );
    }

    #[tokio::test]
    async fn test_retry_if_true_retries_normally() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // retry_if returns true — should retry (same as no predicate).
        let options = StepOptions::new().retries(3).retry_if(|_e: &String| true);

        let result: Result<Result<i32, String>, DurableError> = ctx
            .step_with_options("retry_true_step", options, || async {
                Err("transient error".to_string())
            })
            .await;

        let err = result.unwrap_err();
        match err {
            DurableError::StepRetryScheduled { .. } => {}
            other => panic!("expected StepRetryScheduled, got {other:?}"),
        }

        let captured = calls.lock().await;
        assert_eq!(captured.len(), 2, "expected START + RETRY");
        assert_eq!(
            captured[1].updates[0].action(),
            &aws_sdk_lambda::types::OperationAction::Retry,
        );
    }

    #[tokio::test]
    async fn test_no_retry_if_retries_all_errors_backward_compatible() {
        let (backend, calls) = MockBackend::new("new-token");
        let backend = Arc::new(backend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();

        // No retry_if — should retry all errors (backward compatible).
        let options = StepOptions::new().retries(2);

        let result: Result<Result<i32, String>, DurableError> = ctx
            .step_with_options("compat_retry_step", options, || async {
                Err("any error".to_string())
            })
            .await;

        let err = result.unwrap_err();
        match err {
            DurableError::StepRetryScheduled { .. } => {}
            other => panic!("expected StepRetryScheduled, got {other:?}"),
        }

        let captured = calls.lock().await;
        assert_eq!(captured.len(), 2, "expected START + RETRY");
    }

    #[tokio::test]
    async fn checkpoint_none_token_returns_error() {
        let backend = Arc::new(NoneTokenMockBackend);

        let mut ctx = DurableContext::new(
            backend,
            "arn:test".to_string(),
            "initial-token".to_string(),
            vec![], // empty history = executing mode
            None,
        )
        .await
        .unwrap();

        // Attempt a step -- the first checkpoint (START) will return None token
        // step() returns Result<Result<T, E>, DurableError>; when START checkpoint fails,
        // the outer Result is Err(DurableError::CheckpointFailed).
        let result: Result<Result<i32, String>, DurableError> =
            ctx.step("test_step", || async { Ok(42) }).await;

        // Must be an error, not a silent success with stale token
        let err = result
            .expect_err("step should fail when checkpoint response has None checkpoint_token");

        // Verify it's specifically a CheckpointFailed error
        match &err {
            DurableError::CheckpointFailed { operation_name, .. } => {
                assert!(
                    operation_name.contains("test_step"),
                    "error should reference the operation name, got: {}",
                    operation_name
                );
            }
            other => panic!("expected DurableError::CheckpointFailed, got: {:?}", other),
        }

        // Verify the error message mentions the missing token
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("checkpoint response missing checkpoint_token"),
            "error message should mention missing checkpoint_token, got: {}",
            err_msg
        );
    }
}
