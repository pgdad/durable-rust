//! MockBackend — implements DurableBackend without AWS dependency.
//!
//! Returns pre-loaded data for testing. No network calls, no credentials needed.
//! Records all checkpoint calls for test assertions.

use std::sync::Arc;

use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
use aws_sdk_lambda::types::OperationUpdate;
use durable_lambda_core::backend::DurableBackend;
use durable_lambda_core::error::DurableError;
use tokio::sync::Mutex;

/// A recorded durable operation for sequence verification (FR39).
///
/// Each time the handler starts a new operation (step, wait, callback, etc.),
/// an `OperationRecord` is captured with the operation name and type.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::prelude::*;
///
/// let (mut ctx, calls, ops) = MockDurableContext::new()
///     .build()
///     .await;
///
/// let _: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await.unwrap();
///
/// let recorded = ops.lock().await;
/// assert_eq!(recorded[0].name, "validate");
/// assert_eq!(recorded[0].operation_type, "step");
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRecord {
    /// The user-provided operation name (e.g., "validate", "cooldown").
    pub name: String,
    /// The operation type as a lowercase string (e.g., "step", "wait", "callback").
    pub operation_type: String,
}

impl OperationRecord {
    /// Format as `"type:name"` for use with assertion helpers.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_testing::mock_backend::OperationRecord;
    ///
    /// let record = OperationRecord {
    ///     name: "validate".to_string(),
    ///     operation_type: "step".to_string(),
    /// };
    /// assert_eq!(record.to_type_name(), "step:validate");
    /// ```
    pub fn to_type_name(&self) -> String {
        format!("{}:{}", self.operation_type, self.name)
    }
}

impl std::fmt::Display for OperationRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.operation_type, self.name)
    }
}

/// A captured checkpoint call for test assertions.
///
/// Each time the handler checkpoints an operation (START, SUCCEED, FAIL, RETRY),
/// a `CheckpointCall` is recorded with the full details.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::prelude::*;
///
/// let (mut ctx, calls, _ops) = MockDurableContext::new()
///     .build()
///     .await;
///
/// // ... run handler ...
///
/// let captured = calls.lock().await;
/// assert_eq!(captured.len(), 2); // START + SUCCEED
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct CheckpointCall {
    /// The durable execution ARN passed to checkpoint.
    pub arn: String,
    /// The checkpoint token passed to checkpoint.
    pub checkpoint_token: String,
    /// The operation updates sent in this checkpoint.
    pub updates: Vec<OperationUpdate>,
}

/// Mock implementation of [`DurableBackend`] for testing.
///
/// Records all checkpoint calls and returns configurable responses.
/// Never makes AWS API calls — pure in-memory mock.
///
/// Typically created via [`MockDurableContext`](crate::mock_context::MockDurableContext)
/// rather than directly.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::mock_backend::MockBackend;
/// use std::sync::Arc;
///
/// let (backend, calls, _ops) = MockBackend::new("mock-token");
/// let backend = Arc::new(backend);
/// // Use with DurableContext::new(backend, ...)
/// # }
/// ```
/// Shared recorder for checkpoint calls.
///
/// Use this handle to inspect all checkpoint API calls made during a test.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::prelude::*;
///
/// let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;
/// // ... run handler operations ...
/// let captured: Vec<_> = calls.lock().await.clone();
/// assert!(!captured.is_empty());
/// # }
/// ```
pub type CheckpointRecorder = Arc<Mutex<Vec<CheckpointCall>>>;

/// Shared recorder for operation sequence tracking.
///
/// Use this handle to inspect the sequence of durable operations
/// started during a test.
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use durable_lambda_testing::prelude::*;
///
/// let (mut ctx, _calls, ops) = MockDurableContext::new().build().await;
/// // ... run handler operations ...
/// let recorded: Vec<_> = ops.lock().await.clone();
/// assert!(!recorded.is_empty());
/// # }
/// ```
pub type OperationRecorder = Arc<Mutex<Vec<OperationRecord>>>;

/// Shared counter for batch checkpoint calls.
pub type BatchCallCounter = Arc<Mutex<usize>>;

pub struct MockBackend {
    calls: CheckpointRecorder,
    operations: OperationRecorder,
    checkpoint_token: String,
    batch_call_count: BatchCallCounter,
}

impl MockBackend {
    /// Create a new `MockBackend` with the given checkpoint token.
    ///
    /// Returns the backend, a checkpoint call recorder, and an operation
    /// sequence recorder for test assertions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use durable_lambda_testing::mock_backend::MockBackend;
    ///
    /// let (backend, calls, _ops) = MockBackend::new("token-123");
    /// // calls can be inspected after running the handler
    /// # }
    /// ```
    pub fn new(checkpoint_token: &str) -> (Self, CheckpointRecorder, OperationRecorder) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let operations = Arc::new(Mutex::new(Vec::new()));
        let backend = Self {
            calls: calls.clone(),
            operations: operations.clone(),
            checkpoint_token: checkpoint_token.to_string(),
            batch_call_count: Arc::new(Mutex::new(0)),
        };
        (backend, calls, operations)
    }

    /// Return the batch checkpoint call counter for test assertions.
    pub fn batch_call_counter(&self) -> BatchCallCounter {
        self.batch_call_count.clone()
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
        // Record operation sequence from START actions (one per logical operation).
        for update in &updates {
            if update.action() == &aws_sdk_lambda::types::OperationAction::Start {
                let op_type = match update.r#type() {
                    aws_sdk_lambda::types::OperationType::Step => "step",
                    aws_sdk_lambda::types::OperationType::Wait => "wait",
                    aws_sdk_lambda::types::OperationType::Callback => "callback",
                    aws_sdk_lambda::types::OperationType::ChainedInvoke => "invoke",
                    _ => "unknown",
                };
                let name = update.name().unwrap_or("").to_string();
                self.operations.lock().await.push(OperationRecord {
                    name,
                    operation_type: op_type.to_string(),
                });
            }
        }

        self.calls.lock().await.push(CheckpointCall {
            arn: arn.to_string(),
            checkpoint_token: checkpoint_token.to_string(),
            updates,
        });
        Ok(CheckpointDurableExecutionOutput::builder()
            .checkpoint_token(&self.checkpoint_token)
            .build())
    }

    async fn batch_checkpoint(
        &self,
        arn: &str,
        checkpoint_token: &str,
        updates: Vec<OperationUpdate>,
        _client_token: Option<&str>,
    ) -> Result<
        aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput,
        DurableError,
    > {
        *self.batch_call_count.lock().await += 1;
        // Record individual operations for sequence tracking (same as checkpoint).
        for update in &updates {
            if update.action() == &aws_sdk_lambda::types::OperationAction::Start {
                let op_type = match update.r#type() {
                    aws_sdk_lambda::types::OperationType::Step => "step",
                    aws_sdk_lambda::types::OperationType::Wait => "wait",
                    aws_sdk_lambda::types::OperationType::Callback => "callback",
                    aws_sdk_lambda::types::OperationType::ChainedInvoke => "invoke",
                    _ => "unknown",
                };
                let name = update.name().unwrap_or("").to_string();
                self.operations.lock().await.push(OperationRecord {
                    name,
                    operation_type: op_type.to_string(),
                });
            }
        }
        self.calls.lock().await.push(CheckpointCall {
            arn: arn.to_string(),
            checkpoint_token: checkpoint_token.to_string(),
            updates,
        });
        Ok(
            aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput::builder()
                .checkpoint_token(&self.checkpoint_token)
                .build(),
        )
    }

    async fn get_execution_state(
        &self,
        _arn: &str,
        _checkpoint_token: &str,
        _next_marker: &str,
        _max_items: i32,
    ) -> Result<GetDurableExecutionStateOutput, DurableError> {
        Ok(GetDurableExecutionStateOutput::builder()
            .build()
            .expect("empty execution state"))
    }
}
