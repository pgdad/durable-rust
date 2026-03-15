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
/// let (mut ctx, calls) = MockDurableContext::new()
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
/// let (backend, calls) = MockBackend::new("mock-token");
/// let backend = Arc::new(backend);
/// // Use with DurableContext::new(backend, ...)
/// # }
/// ```
pub struct MockBackend {
    calls: Arc<Mutex<Vec<CheckpointCall>>>,
    checkpoint_token: String,
}

impl MockBackend {
    /// Create a new `MockBackend` with the given checkpoint token.
    ///
    /// Returns the backend and a shared reference to the captured checkpoint
    /// calls for test assertions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() {
    /// use durable_lambda_testing::mock_backend::MockBackend;
    ///
    /// let (backend, calls) = MockBackend::new("token-123");
    /// // calls can be inspected after running the handler
    /// # }
    /// ```
    pub fn new(checkpoint_token: &str) -> (Self, Arc<Mutex<Vec<CheckpointCall>>>) {
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
        Ok(GetDurableExecutionStateOutput::builder()
            .build()
            .expect("empty execution state"))
    }
}
