//! MockDurableContext — pre-loaded step results for local testing.
//!
//! Implements FR37-FR38: create mock context with pre-loaded results,
//! run tests without AWS credentials.

use std::sync::Arc;

use aws_sdk_lambda::types::{
    CallbackDetails, ChainedInvokeDetails, Operation, OperationStatus, OperationType, StepDetails,
};
use durable_lambda_core::context::DurableContext;
use durable_lambda_core::operation_id::OperationIdGenerator;

use crate::mock_backend::{CheckpointRecorder, MockBackend, OperationRecorder};

/// Builder for creating a [`DurableContext`] with pre-loaded step results.
///
/// `MockDurableContext` generates a `DurableContext` in **Replaying** mode
/// by pre-loading completed operations. When the handler calls `ctx.step()`,
/// the pre-loaded results are returned without executing the closure.
///
/// Operation IDs are generated deterministically using the same blake2b
/// algorithm as the core engine, ensuring the nth `with_step_result` call
/// corresponds to the nth `ctx.step()` call.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::prelude::*;
///
/// let (mut ctx, calls, _ops) = MockDurableContext::new()
///     .with_step_result("validate", r#"{"valid": true}"#)
///     .with_step_result("charge", r#"100"#)
///     .build()
///     .await;
///
/// // Steps replay cached results — closures are NOT executed
/// let result: Result<serde_json::Value, String> = ctx.step("validate", || async {
///     panic!("not executed during replay");
/// }).await.unwrap();
///
/// assert_eq!(result.unwrap(), serde_json::json!({"valid": true}));
///
/// // Verify no checkpoints were made (pure replay)
/// assert_no_checkpoints(&calls).await;
/// # }
/// ```
pub struct MockDurableContext {
    id_gen: OperationIdGenerator,
    operations: Vec<Operation>,
}

impl MockDurableContext {
    /// Create a new `MockDurableContext` builder.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use durable_lambda_testing::prelude::*;
    ///
    /// let (mut ctx, calls, _ops) = MockDurableContext::new()
    ///     .with_step_result("my_step", r#""hello""#)
    ///     .build()
    ///     .await;
    /// # }
    /// ```
    pub fn new() -> Self {
        Self {
            id_gen: OperationIdGenerator::new(None),
            operations: Vec::new(),
        }
    }

    /// Add a successful step result to the mock history.
    ///
    /// The `result_json` is a JSON string representing the step's return value.
    /// It will be returned by `ctx.step()` during replay without executing
    /// the closure.
    ///
    /// # Arguments
    ///
    /// * `_name` — Step name (for documentation; the operation ID is position-based)
    /// * `result_json` — JSON string of the step result (e.g., `r#"{"valid": true}"#`)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use durable_lambda_testing::prelude::*;
    ///
    /// let (mut ctx, _, _ops) = MockDurableContext::new()
    ///     .with_step_result("validate", r#"42"#)
    ///     .build()
    ///     .await;
    ///
    /// let result: Result<i32, String> = ctx.step("validate", || async {
    ///     panic!("not executed");
    /// }).await.unwrap();
    ///
    /// assert_eq!(result.unwrap(), 42);
    /// # }
    /// ```
    pub fn with_step_result(mut self, _name: &str, result_json: &str) -> Self {
        let op_id = self.id_gen.next_id();
        let op = Operation::builder()
            .id(&op_id)
            .r#type(OperationType::Step)
            .status(OperationStatus::Succeeded)
            .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
            .step_details(StepDetails::builder().result(result_json).build())
            .build()
            .unwrap_or_else(|e| panic!("failed to build mock Operation: {e}"));
        self.operations.push(op);
        self
    }

    /// Add a failed step result to the mock history.
    ///
    /// The step will replay as a typed error. The `error_type` is the type
    /// name and `error_json` is the serialized error data.
    ///
    /// # Arguments
    ///
    /// * `_name` — Step name (for documentation; the operation ID is position-based)
    /// * `error_type` — The error type name (e.g., `"my_crate::MyError"`)
    /// * `error_json` — JSON string of the error data
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use durable_lambda_testing::prelude::*;
    ///
    /// let (mut ctx, _, _ops) = MockDurableContext::new()
    ///     .with_step_error("charge", "PaymentError", r#""insufficient_funds""#)
    ///     .build()
    ///     .await;
    ///
    /// let result: Result<i32, String> = ctx.step("charge", || async {
    ///     panic!("not executed");
    /// }).await.unwrap();
    ///
    /// assert_eq!(result.unwrap_err(), "insufficient_funds");
    /// # }
    /// ```
    pub fn with_step_error(mut self, _name: &str, error_type: &str, error_json: &str) -> Self {
        let op_id = self.id_gen.next_id();
        let error_obj = aws_sdk_lambda::types::ErrorObject::builder()
            .error_type(error_type)
            .error_data(error_json)
            .build();
        let op = Operation::builder()
            .id(&op_id)
            .r#type(OperationType::Step)
            .status(OperationStatus::Failed)
            .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
            .step_details(StepDetails::builder().error(error_obj).build())
            .build()
            .unwrap_or_else(|e| panic!("failed to build mock Operation: {e}"));
        self.operations.push(op);
        self
    }

    /// Add a completed wait to the mock history.
    ///
    /// Simulates a wait that has already completed (SUCCEEDED). During replay,
    /// `ctx.wait()` will return `Ok(())` immediately.
    ///
    /// # Arguments
    ///
    /// * `_name` — Wait name (for documentation; the operation ID is position-based)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use durable_lambda_testing::prelude::*;
    ///
    /// let (mut ctx, _, _ops) = MockDurableContext::new()
    ///     .with_step_result("validate", r#"42"#)
    ///     .with_wait("cooldown")
    ///     .with_step_result("charge", r#"100"#)
    ///     .build()
    ///     .await;
    /// # }
    /// ```
    pub fn with_wait(mut self, _name: &str) -> Self {
        let op_id = self.id_gen.next_id();
        let op = Operation::builder()
            .id(&op_id)
            .r#type(OperationType::Wait)
            .status(OperationStatus::Succeeded)
            .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
            .build()
            .unwrap_or_else(|e| panic!("failed to build mock Wait Operation: {e}"));
        self.operations.push(op);
        self
    }

    /// Add a completed callback to the mock history.
    ///
    /// Simulates a callback that has been signaled with success. During replay,
    /// `ctx.create_callback()` will return a `CallbackHandle` with the given
    /// `callback_id`, and `ctx.callback_result()` will return the deserialized
    /// result.
    ///
    /// # Arguments
    ///
    /// * `_name` — Callback name (for documentation; the operation ID is position-based)
    /// * `callback_id` — The server-generated callback ID
    /// * `result_json` — JSON string of the callback result
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use durable_lambda_testing::prelude::*;
    ///
    /// let (mut ctx, _, _ops) = MockDurableContext::new()
    ///     .with_callback("approval", "cb-123", r#""approved""#)
    ///     .build()
    ///     .await;
    /// # }
    /// ```
    pub fn with_callback(mut self, _name: &str, callback_id: &str, result_json: &str) -> Self {
        let op_id = self.id_gen.next_id();
        let cb_details = CallbackDetails::builder()
            .callback_id(callback_id)
            .result(result_json)
            .build();
        let op = Operation::builder()
            .id(&op_id)
            .r#type(OperationType::Callback)
            .status(OperationStatus::Succeeded)
            .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
            .callback_details(cb_details)
            .build()
            .unwrap_or_else(|e| panic!("failed to build mock Callback Operation: {e}"));
        self.operations.push(op);
        self
    }

    /// Add a completed invoke to the mock history.
    ///
    /// Simulates a chained invoke that has completed successfully. During replay,
    /// `ctx.invoke()` will return the deserialized result.
    ///
    /// # Arguments
    ///
    /// * `_name` — Invoke name (for documentation; the operation ID is position-based)
    /// * `result_json` — JSON string of the invoke result
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use durable_lambda_testing::prelude::*;
    ///
    /// let (mut ctx, _, _ops) = MockDurableContext::new()
    ///     .with_invoke("call_processor", r#"{"status":"ok"}"#)
    ///     .build()
    ///     .await;
    /// # }
    /// ```
    pub fn with_invoke(mut self, _name: &str, result_json: &str) -> Self {
        let op_id = self.id_gen.next_id();
        let details = ChainedInvokeDetails::builder().result(result_json).build();
        let op = Operation::builder()
            .id(&op_id)
            .r#type(OperationType::ChainedInvoke)
            .status(OperationStatus::Succeeded)
            .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
            .chained_invoke_details(details)
            .build()
            .unwrap_or_else(|e| panic!("failed to build mock ChainedInvoke Operation: {e}"));
        self.operations.push(op);
        self
    }

    /// Build the mock context, returning a `DurableContext` and checkpoint call recorder.
    ///
    /// The returned `DurableContext` starts in **Replaying** mode if any
    /// operations were pre-loaded, or **Executing** mode if none were added.
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// - `DurableContext` — ready for use with `ctx.step(...)` etc.
    /// - `Arc<Mutex<Vec<CheckpointCall>>>` — inspect checkpoint calls after test
    ///
    /// # Errors
    ///
    /// Returns [`DurableError`] if context construction fails (should not happen
    /// with mock data).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use durable_lambda_testing::prelude::*;
    ///
    /// let (mut ctx, calls, _ops) = MockDurableContext::new()
    ///     .with_step_result("step1", r#"true"#)
    ///     .build()
    ///     .await;
    /// # }
    /// ```
    pub async fn build(self) -> (DurableContext, CheckpointRecorder, OperationRecorder) {
        let (backend, calls, operations) = MockBackend::new("mock-token");

        let ctx = DurableContext::new(
            Arc::new(backend),
            "arn:aws:lambda:us-east-1:000000000000:durable-execution/mock".to_string(),
            "mock-checkpoint-token".to_string(),
            self.operations,
            None,
        )
        .await
        .expect("MockDurableContext::build should not fail");

        (ctx, calls, operations)
    }
}

impl Default for MockDurableContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn test_mock_context_replays_step_result() {
        let (mut ctx, calls, _ops) = MockDurableContext::new()
            .with_step_result("validate", r#"42"#)
            .build()
            .await;

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let result: Result<i32, String> = ctx
            .step("validate", || {
                let executed = executed_clone.clone();
                async move {
                    executed.store(true, Ordering::SeqCst);
                    Ok(999) // should NOT be returned
                }
            })
            .await
            .unwrap();

        assert_eq!(result.unwrap(), 42);
        assert!(
            !executed.load(Ordering::SeqCst),
            "closure should not execute during replay"
        );

        // No checkpoints should be made during pure replay
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0);
    }

    #[tokio::test]
    async fn test_mock_context_replays_multiple_steps() {
        let (mut ctx, calls, _ops) = MockDurableContext::new()
            .with_step_result("step1", r#""hello""#)
            .with_step_result("step2", r#""world""#)
            .build()
            .await;

        let r1: Result<String, String> = ctx
            .step("step1", || async { panic!("not executed") })
            .await
            .unwrap();
        assert_eq!(r1.unwrap(), "hello");

        let r2: Result<String, String> = ctx
            .step("step2", || async { panic!("not executed") })
            .await
            .unwrap();
        assert_eq!(r2.unwrap(), "world");

        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0);
    }

    #[tokio::test]
    async fn test_mock_context_replays_step_error() {
        let (mut ctx, _calls, _ops) = MockDurableContext::new()
            .with_step_error("charge", "PaymentError", r#""insufficient_funds""#)
            .build()
            .await;

        let result: Result<i32, String> = ctx
            .step("charge", || async { panic!("not executed") })
            .await
            .unwrap();

        assert_eq!(result.unwrap_err(), "insufficient_funds");
    }

    #[tokio::test]
    async fn test_mock_context_executing_mode_when_empty() {
        let (ctx, _calls, _ops) = MockDurableContext::new().build().await;

        assert!(!ctx.is_replaying());
        assert_eq!(
            ctx.execution_mode(),
            durable_lambda_core::types::ExecutionMode::Executing
        );
    }

    #[tokio::test]
    async fn test_mock_context_replaying_mode_with_operations() {
        let (ctx, _calls, _ops) = MockDurableContext::new()
            .with_step_result("step1", r#"1"#)
            .build()
            .await;

        assert!(ctx.is_replaying());
        assert_eq!(
            ctx.execution_mode(),
            durable_lambda_core::types::ExecutionMode::Replaying
        );
    }

    #[tokio::test]
    async fn test_mock_context_no_aws_credentials_needed() {
        // This test proves the mock works without any AWS env vars
        // by simply running successfully
        let (mut ctx, _calls, _ops) = MockDurableContext::new()
            .with_step_result("test", r#"true"#)
            .build()
            .await;

        let result: Result<bool, String> = ctx
            .step("test", || async { panic!("not executed") })
            .await
            .unwrap();

        assert!(result.unwrap());
    }
}
