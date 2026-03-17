//! Saga / compensation pattern operations.
//!
//! Implement the saga pattern for distributed transaction rollback:
//!
//! 1. [`DurableContext::step_with_compensation`] — execute a forward step and,
//!    on success, register a type-erased compensation closure that can reverse it.
//! 2. [`DurableContext::run_compensations`] — execute all registered compensations
//!    in **reverse registration order** (LIFO — last registered, first executed),
//!    checkpointing each one with `Context/START + Context/SUCCEED|FAIL` using
//!    `sub_type = "Compensation"`. All compensations are attempted regardless of
//!    earlier failures (continue-on-error semantics).
//!
//! # Checkpoint Protocol
//!
//! Each compensation checkpoint mirrors the child_context pattern:
//! - `OperationType::Context` + `OperationAction::Start` + `sub_type = "Compensation"`
//! - `OperationType::Context` + `OperationAction::Succeed` + `sub_type = "Compensation"`
//! - or `OperationType::Context` + `OperationAction::Fail` on error
//!
//! # Replay / Partial Rollback Resume
//!
//! During replay, completed compensations (Succeeded or Failed in history) are
//! skipped — their outcome is read from history. This enables partial rollback
//! resume: if a Lambda times out mid-compensation, the next invocation replays
//! the completed ones and continues from the first incomplete one.

use std::future::Future;

use aws_sdk_lambda::types::{OperationAction, OperationStatus, OperationType, OperationUpdate};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::context::DurableContext;
use crate::error::DurableError;
use crate::types::{
    CompensateFn, CompensationItem, CompensationRecord, CompensationResult, CompensationStatus,
    StepOptions,
};

impl DurableContext {
    /// Execute a forward step and register a compensation closure on success.
    ///
    /// Delegates the forward execution to [`step`](Self::step). If the step
    /// succeeds (returns `Ok(Ok(value))`), the `compensate_fn` closure is
    /// registered and will be executed by [`run_compensations`](Self::run_compensations).
    ///
    /// If the forward step fails (returns `Ok(Err(e))`), no compensation is
    /// registered — only successful steps have compensations that need undoing.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the forward step operation.
    /// * `forward_fn` — Closure to execute the forward step.
    /// * `compensate_fn` — Closure to execute when rolling back; receives the
    ///   forward step's success value.
    ///
    /// # Returns
    ///
    /// * `Ok(Ok(T))` — Forward step succeeded; compensation registered.
    /// * `Ok(Err(E))` — Forward step returned a user error; no compensation registered.
    /// * `Err(DurableError)` — SDK-level failure (checkpoint, serialization).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// // Book a hotel room and register its cancellation as compensation
    /// let booking_result: Result<String, String> = ctx.step_with_compensation(
    ///     "book_hotel",
    ///     || async { Ok("BOOKING-123".to_string()) },
    ///     |booking_id| async move {
    ///         // Cancel the hotel booking
    ///         println!("Cancelling booking: {booking_id}");
    ///         Ok(())
    ///     },
    /// ).await?;
    ///
    /// // Later, roll back all registered compensations
    /// let comp_result = ctx.run_compensations().await?;
    /// assert!(comp_result.all_succeeded);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn step_with_compensation<T, E, F, Fut, G, GFut>(
        &mut self,
        name: &str,
        forward_fn: F,
        compensate_fn: G,
    ) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        G: FnOnce(T) -> GFut + Send + 'static,
        GFut: Future<Output = Result<(), DurableError>> + Send + 'static,
    {
        let step_result = self.step(name, forward_fn).await?;

        match step_result {
            Ok(value) => {
                // Serialize the forward result so we can store it alongside the type-erased fn.
                let forward_result_json = serde_json::to_value(&value)
                    .map_err(|e| DurableError::serialization(std::any::type_name::<T>(), e))?;

                // Wrap the typed compensation fn into a type-erased CompensateFn that
                // deserializes the JSON back to T before calling the original closure.
                let wrapped: CompensateFn = Box::new(move |json_value: serde_json::Value| {
                    Box::pin(async move {
                        let deserialized: T = serde_json::from_value(json_value).map_err(|e| {
                            DurableError::deserialization(std::any::type_name::<T>(), e)
                        })?;
                        compensate_fn(deserialized).await
                    })
                });

                self.push_compensation(CompensationRecord {
                    name: name.to_string(),
                    forward_result_json,
                    compensate_fn: wrapped,
                });

                Ok(Ok(value))
            }
            Err(e) => {
                // Forward step returned a user error — no compensation needed.
                Ok(Err(e))
            }
        }
    }

    /// Execute a forward step (with options) and register a compensation closure on success.
    ///
    /// Like [`step_with_compensation`](Self::step_with_compensation) but accepts
    /// [`StepOptions`] for configuring retries, backoff, and timeouts on the
    /// forward step.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the forward step operation.
    /// * `options` — Step configuration (retries, backoff, timeout).
    /// * `forward_fn` — Closure to execute the forward step.
    /// * `compensate_fn` — Closure to execute when rolling back.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_core::types::StepOptions;
    ///
    /// let result: Result<String, String> = ctx.step_with_compensation_opts(
    ///     "book_hotel",
    ///     StepOptions::new().retries(3),
    ///     || async { Ok("BOOKING-123".to_string()) },
    ///     |booking_id| async move {
    ///         println!("Cancelling: {booking_id}");
    ///         Ok(())
    ///     },
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn step_with_compensation_opts<T, E, F, Fut, G, GFut>(
        &mut self,
        name: &str,
        options: StepOptions,
        forward_fn: F,
        compensate_fn: G,
    ) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        G: FnOnce(T) -> GFut + Send + 'static,
        GFut: Future<Output = Result<(), DurableError>> + Send + 'static,
    {
        let step_result = self.step_with_options(name, options, forward_fn).await?;

        match step_result {
            Ok(value) => {
                let forward_result_json = serde_json::to_value(&value)
                    .map_err(|e| DurableError::serialization(std::any::type_name::<T>(), e))?;

                let wrapped: CompensateFn = Box::new(move |json_value: serde_json::Value| {
                    Box::pin(async move {
                        let deserialized: T = serde_json::from_value(json_value).map_err(|e| {
                            DurableError::deserialization(std::any::type_name::<T>(), e)
                        })?;
                        compensate_fn(deserialized).await
                    })
                });

                self.push_compensation(CompensationRecord {
                    name: name.to_string(),
                    forward_result_json,
                    compensate_fn: wrapped,
                });

                Ok(Ok(value))
            }
            Err(e) => Ok(Err(e)),
        }
    }

    /// Execute all registered compensations in reverse registration order.
    ///
    /// Drains the registered compensations and executes them in LIFO order
    /// (last registered runs first — stack semantics). Each compensation is
    /// checkpointed with `Context/START + Context/SUCCEED|FAIL` using
    /// `sub_type = "Compensation"`.
    ///
    /// All compensations are attempted even if earlier ones fail. The returned
    /// [`CompensationResult`] captures the per-item outcomes.
    ///
    /// During replay, completed compensations are skipped — their status is
    /// read from the execution history to support partial rollback resume.
    ///
    /// # Returns
    ///
    /// Returns `Ok(CompensationResult)` always (individual failures are captured
    /// in the result items, not propagated as errors). Returns `Err(DurableError)`
    /// only on AWS checkpoint failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// // After some compensable steps fail:
    /// let result = ctx.run_compensations().await?;
    ///
    /// if !result.all_succeeded {
    ///     for item in &result.items {
    ///         if let Some(err) = &item.error {
    ///             eprintln!("Compensation {} failed: {}", item.name, err);
    ///         }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_compensations(&mut self) -> Result<CompensationResult, DurableError> {
        let mut compensations = self.take_compensations();

        // LIFO: reverse so last registered runs first.
        compensations.reverse();

        if compensations.is_empty() {
            return Ok(CompensationResult {
                items: vec![],
                all_succeeded: true,
            });
        }

        let mut items: Vec<CompensationItem> = Vec::with_capacity(compensations.len());

        for record in compensations {
            let comp_op_id = self.replay_engine_mut().generate_operation_id();
            let name = record.name.clone();

            let span = tracing::info_span!(
                "durable_operation",
                op.name = %name,
                op.type = "compensation",
                op.id = %comp_op_id,
            );
            let _guard = span.enter();
            tracing::trace!("durable_operation");

            // Replay path: check if this compensation already completed.
            // Extract all needed data BEFORE taking mutable borrow.
            let replay_outcome = self.replay_engine().check_result(&comp_op_id).map(|op| {
                let succeeded = op.status == OperationStatus::Succeeded;
                let error_msg = if !succeeded {
                    op.context_details()
                        .and_then(|d| d.error())
                        .map(|e| {
                            format!(
                                "{}: {}",
                                e.error_type().unwrap_or("Unknown"),
                                e.error_data().unwrap_or("")
                            )
                        })
                        .or_else(|| Some("compensation failed during replay".to_string()))
                } else {
                    None
                };
                (succeeded, error_msg)
            });

            if let Some((succeeded, error_msg)) = replay_outcome {
                self.replay_engine_mut().track_replay(&comp_op_id);
                let status = if succeeded {
                    CompensationStatus::Succeeded
                } else {
                    CompensationStatus::Failed
                };
                items.push(CompensationItem {
                    name,
                    status,
                    error: error_msg,
                });
                continue;
            }

            // Execute path: send Context/START for this compensation.
            let start_update = OperationUpdate::builder()
                .id(comp_op_id.clone())
                .r#type(OperationType::Context)
                .action(OperationAction::Start)
                .sub_type("Compensation")
                .name(&name)
                .build()
                .map_err(|e| DurableError::checkpoint_failed(&name, e))?;

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
                    &name,
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "compensation start checkpoint response missing checkpoint_token",
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

            // Execute the compensation closure inline (no tokio::spawn — strict LIFO order).
            let comp_result = (record.compensate_fn)(record.forward_result_json).await;

            match comp_result {
                Ok(()) => {
                    // Send Context/SUCCEED.
                    let succeed_update = OperationUpdate::builder()
                        .id(comp_op_id.clone())
                        .r#type(OperationType::Context)
                        .action(OperationAction::Succeed)
                        .sub_type("Compensation")
                        .build()
                        .map_err(|e| DurableError::checkpoint_failed(&name, e))?;

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
                            &name,
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "compensation succeed checkpoint response missing checkpoint_token",
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

                    self.replay_engine_mut().track_replay(&comp_op_id);
                    items.push(CompensationItem {
                        name,
                        status: CompensationStatus::Succeeded,
                        error: None,
                    });
                }
                Err(comp_err) => {
                    let error_msg = comp_err.to_string();

                    // Send Context/FAIL — continue-on-error: do NOT return early.
                    let fail_update = OperationUpdate::builder()
                        .id(comp_op_id.clone())
                        .r#type(OperationType::Context)
                        .action(OperationAction::Fail)
                        .sub_type("Compensation")
                        .build()
                        .map_err(|e| DurableError::checkpoint_failed(&name, e))?;

                    let fail_response = self
                        .backend()
                        .checkpoint(self.arn(), self.checkpoint_token(), vec![fail_update], None)
                        .await?;

                    let new_token = fail_response.checkpoint_token().ok_or_else(|| {
                        DurableError::checkpoint_failed(
                            &name,
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "compensation fail checkpoint response missing checkpoint_token",
                            ),
                        )
                    })?;
                    self.set_checkpoint_token(new_token.to_string());

                    if let Some(new_state) = fail_response.new_execution_state() {
                        for op in new_state.operations() {
                            self.replay_engine_mut()
                                .insert_operation(op.id().to_string(), op.clone());
                        }
                    }

                    self.replay_engine_mut().track_replay(&comp_op_id);
                    items.push(CompensationItem {
                        name,
                        status: CompensationStatus::Failed,
                        error: Some(error_msg),
                    });
                    // Continue to next compensation — do NOT abort.
                }
            }
        }

        let all_succeeded = items
            .iter()
            .all(|i| i.status == CompensationStatus::Succeeded);

        Ok(CompensationResult {
            items,
            all_succeeded,
        })
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
    use crate::types::{CompensationRecord, CompensationStatus};

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct CheckpointCall {
        arn: String,
        checkpoint_token: String,
        updates: Vec<OperationUpdate>,
    }

    /// MockBackend that records all checkpoint calls.
    struct CompensationMockBackend {
        calls: Arc<Mutex<Vec<CheckpointCall>>>,
    }

    impl CompensationMockBackend {
        fn new() -> (Self, Arc<Mutex<Vec<CheckpointCall>>>) {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let backend = Self {
                calls: calls.clone(),
            };
            (backend, calls)
        }
    }

    #[async_trait::async_trait]
    impl DurableBackend for CompensationMockBackend {
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

    fn second_op_id() -> String {
        let mut gen = crate::operation_id::OperationIdGenerator::new(None);
        let _ = gen.next_id(); // skip first
        gen.next_id()
    }

    async fn make_empty_ctx(backend: CompensationMockBackend) -> DurableContext {
        DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap()
    }

    fn make_context_op(id: &str, status: OperationStatus) -> Operation {
        Operation::builder()
            .id(id)
            .r#type(OperationType::Context)
            .status(status)
            .start_timestamp(DateTime::from_secs(0))
            .build()
            .unwrap()
    }

    // ─── step_with_compensation tests ────────────────────────────────────

    #[tokio::test]
    async fn test_step_with_compensation_returns_ok_ok_on_success() {
        let (backend, _calls) = CompensationMockBackend::new();
        let mut ctx = make_empty_ctx(backend).await;

        let result: Result<Result<i32, String>, DurableError> = ctx
            .step_with_compensation(
                "charge",
                || async { Ok::<i32, String>(42) },
                |_value| async move { Ok(()) },
            )
            .await;

        let inner = result.unwrap();
        assert!(inner.is_ok(), "expected Ok(42), got {inner:?}");
        assert_eq!(inner.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_step_with_compensation_returns_ok_err_on_forward_failure() {
        let (backend, _calls) = CompensationMockBackend::new();
        let mut ctx = make_empty_ctx(backend).await;

        let result: Result<Result<i32, String>, DurableError> = ctx
            .step_with_compensation(
                "charge",
                || async { Err::<i32, String>("payment declined".to_string()) },
                |_value| async move { Ok(()) },
            )
            .await;

        let inner = result.unwrap();
        assert!(inner.is_err(), "expected Err, got {inner:?}");
        assert_eq!(inner.unwrap_err(), "payment declined");
    }

    #[tokio::test]
    async fn test_step_with_compensation_registers_compensation_on_success() {
        let (backend, _calls) = CompensationMockBackend::new();
        let mut ctx = make_empty_ctx(backend).await;

        assert_eq!(ctx.compensation_count(), 0);

        let _: Result<Result<i32, String>, DurableError> = ctx
            .step_with_compensation(
                "charge",
                || async { Ok::<i32, String>(42) },
                |_value| async move { Ok(()) },
            )
            .await;

        assert_eq!(
            ctx.compensation_count(),
            1,
            "compensation should be registered"
        );
    }

    #[tokio::test]
    async fn test_step_with_compensation_does_not_register_on_forward_failure() {
        let (backend, _calls) = CompensationMockBackend::new();
        let mut ctx = make_empty_ctx(backend).await;

        let _: Result<Result<i32, String>, DurableError> = ctx
            .step_with_compensation(
                "charge",
                || async { Err::<i32, String>("declined".to_string()) },
                |_value| async move { Ok(()) },
            )
            .await;

        assert_eq!(
            ctx.compensation_count(),
            0,
            "no compensation should be registered when forward step fails"
        );
    }

    // ─── run_compensations tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_run_compensations_with_zero_returns_empty_all_succeeded() {
        let (backend, _calls) = CompensationMockBackend::new();
        let mut ctx = make_empty_ctx(backend).await;

        let result = ctx.run_compensations().await.unwrap();

        assert!(
            result.all_succeeded,
            "empty run should be all_succeeded=true"
        );
        assert!(result.items.is_empty(), "items should be empty");
    }

    #[tokio::test]
    async fn test_run_compensations_executes_in_reverse_order() {
        let execution_order: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        let (backend, _calls) = CompensationMockBackend::new();
        let mut ctx = make_empty_ctx(backend).await;

        // Register 3 compensations
        for i in 1..=3_i32 {
            let order_clone = execution_order.clone();
            let label = format!("step{i}");
            let _: Result<Result<i32, String>, DurableError> = ctx
                .step_with_compensation(
                    &label.clone(),
                    move || async move { Ok::<i32, String>(i) },
                    move |_value| {
                        let order = order_clone.clone();
                        let label = label.clone();
                        async move {
                            order.lock().await.push(label);
                            Ok(())
                        }
                    },
                )
                .await;
        }

        assert_eq!(ctx.compensation_count(), 3);
        let result = ctx.run_compensations().await.unwrap();
        assert!(result.all_succeeded);

        // Registered: step1, step2, step3 → executes: step3, step2, step1
        let order = execution_order.lock().await;
        assert_eq!(
            order.as_slice(),
            &["step3", "step2", "step1"],
            "compensations must run in reverse registration order, got: {order:?}"
        );
    }

    #[tokio::test]
    async fn test_run_compensations_sends_context_start_and_succeed() {
        let (backend, calls) = CompensationMockBackend::new();
        let mut ctx = make_empty_ctx(backend).await;

        let _: Result<Result<i32, String>, DurableError> = ctx
            .step_with_compensation(
                "refund",
                || async { Ok::<i32, String>(99) },
                |_value| async move { Ok(()) },
            )
            .await;

        // Clear the step checkpoints by getting their count
        let step_calls_count = calls.lock().await.len();

        let result = ctx.run_compensations().await.unwrap();
        assert!(result.all_succeeded);

        let all_calls = calls.lock().await;
        let comp_calls = &all_calls[step_calls_count..]; // only compensation checkpoints

        assert_eq!(
            comp_calls.len(),
            2,
            "expected Context/START + Context/SUCCEED for compensation, got {}",
            comp_calls.len()
        );

        // First: Context/START with sub_type "Compensation"
        assert_eq!(comp_calls[0].updates[0].r#type(), &OperationType::Context);
        assert_eq!(comp_calls[0].updates[0].action(), &OperationAction::Start);
        assert_eq!(comp_calls[0].updates[0].sub_type(), Some("Compensation"));
        assert_eq!(comp_calls[0].updates[0].name(), Some("refund"));

        // Second: Context/SUCCEED with sub_type "Compensation"
        assert_eq!(comp_calls[1].updates[0].r#type(), &OperationType::Context);
        assert_eq!(comp_calls[1].updates[0].action(), &OperationAction::Succeed);
        assert_eq!(comp_calls[1].updates[0].sub_type(), Some("Compensation"));
    }

    #[tokio::test]
    async fn test_run_compensations_captures_failure_per_item() {
        let (backend, _calls) = CompensationMockBackend::new();
        let mut ctx = make_empty_ctx(backend).await;

        let _: Result<Result<i32, String>, DurableError> = ctx
            .step_with_compensation(
                "charge",
                || async { Ok::<i32, String>(10) },
                |_value| async move {
                    Err(DurableError::checkpoint_failed(
                        "charge",
                        std::io::Error::new(std::io::ErrorKind::Other, "reversal failed"),
                    ))
                },
            )
            .await;

        let result = ctx.run_compensations().await.unwrap();

        assert!(
            !result.all_succeeded,
            "should not be all_succeeded when a compensation fails"
        );
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].status, CompensationStatus::Failed);
        assert!(
            result.items[0].error.is_some(),
            "failed compensation should have error message"
        );
    }

    #[tokio::test]
    async fn test_run_compensations_continues_after_failure() {
        let execution_order: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        let (backend, _calls) = CompensationMockBackend::new();
        let mut ctx = make_empty_ctx(backend).await;

        // Register step1 with a FAILING compensation
        let order1 = execution_order.clone();
        let _: Result<Result<i32, String>, DurableError> = ctx
            .step_with_compensation(
                "step1",
                || async { Ok::<i32, String>(1) },
                move |_| {
                    let order = order1.clone();
                    async move {
                        order.lock().await.push("step1".to_string());
                        Err(DurableError::checkpoint_failed(
                            "step1",
                            std::io::Error::new(std::io::ErrorKind::Other, "fail"),
                        ))
                    }
                },
            )
            .await;

        // Register step2 with a SUCCEEDING compensation
        let order2 = execution_order.clone();
        let _: Result<Result<i32, String>, DurableError> = ctx
            .step_with_compensation(
                "step2",
                || async { Ok::<i32, String>(2) },
                move |_| {
                    let order = order2.clone();
                    async move {
                        order.lock().await.push("step2".to_string());
                        Ok(())
                    }
                },
            )
            .await;

        let result = ctx.run_compensations().await.unwrap();

        // Both should have been attempted: step2 first (LIFO), then step1
        let order = execution_order.lock().await;
        assert_eq!(
            order.as_slice(),
            &["step2", "step1"],
            "both compensations must run regardless of step1 failure"
        );

        assert!(!result.all_succeeded);
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].status, CompensationStatus::Succeeded); // step2 ran first
        assert_eq!(result.items[1].status, CompensationStatus::Failed); // step1 ran second
    }

    #[tokio::test]
    async fn test_run_compensations_all_succeeded_false_when_any_fails() {
        let (backend, _calls) = CompensationMockBackend::new();
        let mut ctx = make_empty_ctx(backend).await;

        let _: Result<Result<i32, String>, DurableError> = ctx
            .step_with_compensation(
                "step",
                || async { Ok::<i32, String>(1) },
                |_| async move {
                    Err(DurableError::checkpoint_failed(
                        "step",
                        std::io::Error::new(std::io::ErrorKind::Other, "fail"),
                    ))
                },
            )
            .await;

        let result = ctx.run_compensations().await.unwrap();
        assert!(!result.all_succeeded);
    }

    #[tokio::test]
    async fn test_run_compensations_replay_skips_completed() {
        // Pre-load a context_op at the FIRST op_id (since no other ops have consumed it).
        // When run_compensations() is called, it generates op_ids starting from 0.
        // The compensation registered here will get first_op_id() as its op_id.
        // Since first_op_id() is pre-loaded as Succeeded, the closure must NOT execute.

        let first_op = first_op_id();
        let comp_op_replay = make_context_op(&first_op, OperationStatus::Succeeded);

        let (backend, calls) = CompensationMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![comp_op_replay],
            None,
        )
        .await
        .unwrap();

        let compensation_ran = Arc::new(Mutex::new(false));
        let ran_clone = compensation_ran.clone();

        let record = CompensationRecord {
            name: "refund".to_string(),
            forward_result_json: serde_json::json!(42),
            compensate_fn: Box::new(move |_| {
                let flag = ran_clone.clone();
                Box::pin(async move {
                    *flag.lock().await = true;
                    Ok(())
                })
            }),
        };
        ctx.push_compensation(record);

        let result = ctx.run_compensations().await.unwrap();

        // Compensation should be replayed (skipped from execution)
        let ran = *compensation_ran.lock().await;
        assert!(
            !ran,
            "compensation closure should NOT execute during replay"
        );

        // No checkpoint calls during replay
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0, "no checkpoints during replay");

        // Result should reflect the replayed status
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].status, CompensationStatus::Succeeded);
        assert!(result.all_succeeded);
    }

    #[tokio::test]
    async fn test_run_compensations_partial_rollback_resume() {
        // Simulate partial rollback: 3 compensations registered, first 2 already
        // completed in history. Only the 3rd should execute.
        //
        // Compensation op_ids are generated FIRST (before any step op_ids in this
        // fresh context). So:
        // - comp3 (last registered, runs first LIFO) → op_id = first_op_id()
        // - comp2 → op_id = second_op_id()
        // - comp1 (first registered, runs last LIFO) → op_id = third_op_id()

        let comp3_op_id = first_op_id();
        let comp2_op_id = second_op_id();

        // Pre-load history: comp3 and comp2 already completed (Succeeded)
        let comp3_op = make_context_op(&comp3_op_id, OperationStatus::Succeeded);
        let comp2_op = make_context_op(&comp2_op_id, OperationStatus::Succeeded);

        let (backend, calls) = CompensationMockBackend::new();
        let mut ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![comp3_op, comp2_op],
            None,
        )
        .await
        .unwrap();

        let execution_order: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));

        // Register 3 compensations (step1, step2, step3)
        for i in [1_i32, 2, 3] {
            let order = execution_order.clone();
            let record = CompensationRecord {
                name: format!("step{i}"),
                forward_result_json: serde_json::json!(i),
                compensate_fn: Box::new(move |_| {
                    let o = order.clone();
                    Box::pin(async move {
                        o.lock().await.push(i);
                        Ok(())
                    })
                }),
            };
            ctx.push_compensation(record);
        }

        let result = ctx.run_compensations().await.unwrap();

        // comp3 (i=3) and comp2 (i=2) replayed → only comp1 (i=1) actually executed
        let order = execution_order.lock().await;
        assert_eq!(
            order.as_slice(),
            &[1],
            "only comp1 should execute; comp3 and comp2 are already done in history"
        );

        assert!(result.all_succeeded);
        assert_eq!(result.items.len(), 3);

        // Check that we only sent checkpoints for comp1 (the one that actually executed)
        let captured = calls.lock().await;
        assert_eq!(
            captured.len(),
            2,
            "only 2 checkpoints (START+SUCCEED) for the one unfinished compensation, got {}",
            captured.len()
        );
    }
}
