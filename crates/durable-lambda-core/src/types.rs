//! Shared types used across the durable-lambda SDK.
//!
//! Export the core data types that all SDK components share:
//! [`HistoryEntry`], [`OperationType`], [`ExecutionMode`], and [`CheckpointResult`].

use serde::{Deserialize, Serialize};

/// Represent a single entry from the durable execution history log.
///
/// Each entry records the name, result, and type of a durable operation
/// that was previously executed and checkpointed. During replay, the
/// replay engine reads these entries to return cached results without
/// re-executing operations.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::{HistoryEntry, OperationType};
/// use serde_json::json;
///
/// let entry = HistoryEntry {
///     name: "validate_order".to_string(),
///     result: json!({"order_id": 42, "valid": true}),
///     operation_type: OperationType::Step,
/// };
///
/// assert_eq!(entry.name, "validate_order");
/// assert_eq!(entry.operation_type, OperationType::Step);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// The name/key identifying this operation (e.g., step name).
    pub name: String,
    /// The serialized result stored as a JSON value.
    pub result: serde_json::Value,
    /// The type of durable operation that produced this entry.
    pub operation_type: OperationType,
}

/// Identify the type of durable operation.
///
/// Each variant corresponds to one of the 8 core durable operations
/// supported by the SDK.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::OperationType;
///
/// let op = OperationType::Step;
/// assert_eq!(op, OperationType::Step);
///
/// // OperationType is serializable for checkpoint storage.
/// let json = serde_json::to_string(&op).unwrap();
/// let deserialized: OperationType = serde_json::from_str(&json).unwrap();
/// assert_eq!(deserialized, OperationType::Step);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationType {
    Step,
    Wait,
    Callback,
    Invoke,
    Parallel,
    Map,
    ChildContext,
    Log,
}

/// Signal whether the replay engine is replaying from history or executing new operations.
///
/// During replay, durable operations return cached results from the history log
/// without re-executing. Once the cursor advances past all history entries,
/// the mode transitions to `Executing` and new operations run and checkpoint
/// their results.
///
/// The history data and cursor position are owned by the replay engine
/// (`replay.rs`), not by this enum.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::ExecutionMode;
///
/// let mode = ExecutionMode::Replaying;
/// assert_eq!(mode, ExecutionMode::Replaying);
///
/// let mode = ExecutionMode::Executing;
/// assert_eq!(mode, ExecutionMode::Executing);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// The engine is replaying operations from cached history.
    Replaying,
    /// The engine is executing new operations and checkpointing results.
    Executing,
}

/// Represent the checkpointed outcome of a durable step operation.
///
/// Unlike [`std::result::Result`], both the success and error values in a
/// `CheckpointResult` are valid, serialized checkpoint data. A step that
/// returns an error still has that error checkpointed and replayed
/// identically on subsequent invocations.
///
/// # Type Parameters
///
/// * `T` — The success value type. Must implement `Serialize + DeserializeOwned`.
/// * `E` — The error value type. Must implement `Serialize + DeserializeOwned`.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::CheckpointResult;
///
/// // A successful checkpoint result.
/// let ok: CheckpointResult<i32, String> = CheckpointResult::Ok(42);
/// assert_eq!(ok, CheckpointResult::Ok(42));
///
/// // An error checkpoint result — the error is also checkpointed.
/// let err: CheckpointResult<i32, String> = CheckpointResult::Err("validation failed".into());
///
/// // Both variants serialize to JSON for checkpoint storage.
/// let json = serde_json::to_string(&ok).unwrap();
/// let deserialized: CheckpointResult<i32, String> = serde_json::from_str(&json).unwrap();
/// assert_eq!(deserialized, CheckpointResult::Ok(42));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CheckpointResult<T, E> {
    /// The step completed successfully with this value.
    Ok(T),
    /// The step returned an error, which is also checkpointed.
    Err(E),
}

/// Configure retry behavior for step operations.
///
/// Provide a builder-style API for configuring how step operations handle
/// failures. By default, no retries are configured — failures are checkpointed
/// immediately. When retries are configured, the SDK sends a RETRY checkpoint
/// to the server, the function exits, and the server re-invokes the Lambda
/// after the configured delay.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::StepOptions;
///
/// // No retries (default).
/// let opts = StepOptions::new();
///
/// // Retry up to 3 times with 5-second backoff.
/// let opts = StepOptions::new().retries(3).backoff_seconds(5);
/// ```
#[derive(Debug, Clone, Default)]
pub struct StepOptions {
    retries: Option<u32>,
    backoff_seconds: Option<i32>,
}

impl StepOptions {
    /// Create a new `StepOptions` with no retries configured.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::types::StepOptions;
    ///
    /// let opts = StepOptions::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of retry attempts on failure.
    ///
    /// When a step fails and retries remain, the SDK sends a RETRY checkpoint
    /// and the server re-invokes the function after the backoff delay.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::types::StepOptions;
    ///
    /// let opts = StepOptions::new().retries(3);
    /// ```
    pub fn retries(mut self, count: u32) -> Self {
        self.retries = Some(count);
        self
    }

    /// Set the delay in seconds between retry attempts.
    ///
    /// If not set, the server uses its default delay (typically 0 for
    /// immediate retry).
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::types::StepOptions;
    ///
    /// let opts = StepOptions::new().retries(3).backoff_seconds(5);
    /// ```
    pub fn backoff_seconds(mut self, seconds: i32) -> Self {
        self.backoff_seconds = Some(seconds);
        self
    }

    /// Return the configured retry count, if any.
    pub fn get_retries(&self) -> Option<u32> {
        self.retries
    }

    /// Return the configured backoff delay in seconds, if any.
    pub fn get_backoff_seconds(&self) -> Option<i32> {
        self.backoff_seconds
    }
}

/// Configure callback timeout behavior.
///
/// Provide a builder-style API for configuring callback operations.
/// By default, no timeouts are set — the callback remains active indefinitely
/// until an external system signals success or failure.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::CallbackOptions;
///
/// // No timeouts (default).
/// let opts = CallbackOptions::new();
///
/// // 5-minute overall timeout, 30-second heartbeat timeout.
/// let opts = CallbackOptions::new()
///     .timeout_seconds(300)
///     .heartbeat_timeout_seconds(30);
/// ```
#[derive(Debug, Clone, Default)]
pub struct CallbackOptions {
    timeout_seconds: i32,
    heartbeat_timeout_seconds: i32,
}

impl CallbackOptions {
    /// Create a new `CallbackOptions` with no timeouts configured.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::types::CallbackOptions;
    ///
    /// let opts = CallbackOptions::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the overall timeout in seconds for the callback.
    ///
    /// If no success/failure signal arrives before this deadline, the server
    /// marks the callback as timed out. `0` means no timeout (default).
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::types::CallbackOptions;
    ///
    /// let opts = CallbackOptions::new().timeout_seconds(300);
    /// ```
    pub fn timeout_seconds(mut self, seconds: i32) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set the heartbeat timeout in seconds.
    ///
    /// External systems must send periodic heartbeat signals within this
    /// interval to keep the callback alive. `0` means no heartbeat
    /// requirement (default).
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::types::CallbackOptions;
    ///
    /// let opts = CallbackOptions::new().heartbeat_timeout_seconds(30);
    /// ```
    pub fn heartbeat_timeout_seconds(mut self, seconds: i32) -> Self {
        self.heartbeat_timeout_seconds = seconds;
        self
    }

    /// Return the configured timeout in seconds.
    pub fn get_timeout_seconds(&self) -> i32 {
        self.timeout_seconds
    }

    /// Return the configured heartbeat timeout in seconds.
    pub fn get_heartbeat_timeout_seconds(&self) -> i32 {
        self.heartbeat_timeout_seconds
    }
}

/// Handle returned by [`DurableContext::create_callback`] containing the
/// server-generated callback ID.
///
/// The `callback_id` is used by external systems to signal the callback
/// via `SendDurableExecutionCallbackSuccess`, `SendDurableExecutionCallbackFailure`,
/// or `SendDurableExecutionCallbackHeartbeat` APIs.
///
/// Pass this handle to [`DurableContext::callback_result`] to retrieve the
/// callback result or suspend if the callback hasn't been signaled yet.
///
/// # Examples
///
/// ```no_run
/// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) -> Result<(), durable_lambda_core::error::DurableError> {
/// use durable_lambda_core::types::CallbackOptions;
///
/// let handle = ctx.create_callback("approval", CallbackOptions::new()).await?;
/// println!("Send this to the external system: {}", handle.callback_id);
///
/// // Later, check the result (suspends if not yet signaled).
/// let result: String = ctx.callback_result(&handle)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct CallbackHandle {
    /// The server-generated callback ID for external systems.
    pub callback_id: String,
    /// The deterministic operation ID (internal use).
    pub(crate) operation_id: String,
}

/// Configure parallel execution behavior.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::ParallelOptions;
///
/// // Default: all branches must succeed.
/// let opts = ParallelOptions::new();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ParallelOptions {
    _private: (),
}

impl ParallelOptions {
    /// Create a new `ParallelOptions` with default settings (all successful).
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::types::ParallelOptions;
    ///
    /// let opts = ParallelOptions::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

/// Configure map operation behavior including batching.
///
/// Control how items in a collection are processed. By default, all items
/// execute concurrently in a single batch. Set `batch_size` to limit
/// concurrency — each batch completes before the next begins.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::MapOptions;
///
/// // Default: all items concurrent.
/// let opts = MapOptions::new();
///
/// // Process in batches of 10.
/// let opts = MapOptions::new().batch_size(10);
/// ```
#[derive(Debug, Clone, Default)]
pub struct MapOptions {
    batch_size: Option<usize>,
}

impl MapOptions {
    /// Create a new `MapOptions` with default settings (all items concurrent).
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::types::MapOptions;
    ///
    /// let opts = MapOptions::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of items to process concurrently per batch.
    ///
    /// Each batch of items executes concurrently, but batches themselves
    /// run sequentially — the next batch starts only after the previous
    /// batch completes.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::types::MapOptions;
    ///
    /// let opts = MapOptions::new().batch_size(5);
    /// ```
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = Some(size);
        self
    }

    /// Return the configured batch size, if any.
    pub fn get_batch_size(&self) -> Option<usize> {
        self.batch_size
    }
}

/// Result of a parallel or map operation containing all branch outcomes.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::{BatchResult, BatchItem, BatchItemStatus, CompletionReason};
///
/// let result = BatchResult {
///     results: vec![
///         BatchItem { index: 0, status: BatchItemStatus::Succeeded, result: Some(42), error: None },
///         BatchItem { index: 1, status: BatchItemStatus::Succeeded, result: Some(99), error: None },
///     ],
///     completion_reason: CompletionReason::AllCompleted,
/// };
/// assert_eq!(result.results.len(), 2);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult<T> {
    /// Individual branch results, ordered by index.
    pub results: Vec<BatchItem<T>>,
    /// Why the parallel block completed.
    pub completion_reason: CompletionReason,
}

/// A single branch outcome within a [`BatchResult`].
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::{BatchItem, BatchItemStatus};
///
/// let item = BatchItem {
///     index: 0,
///     status: BatchItemStatus::Succeeded,
///     result: Some("done".to_string()),
///     error: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem<T> {
    /// The branch index (0-based).
    pub index: usize,
    /// The branch's final status.
    pub status: BatchItemStatus,
    /// The branch result (present if succeeded).
    pub result: Option<T>,
    /// Error message (present if failed).
    pub error: Option<String>,
}

/// Status of an individual branch in a parallel operation.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::BatchItemStatus;
///
/// let status = BatchItemStatus::Succeeded;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BatchItemStatus {
    /// Branch completed successfully.
    Succeeded,
    /// Branch failed with an error.
    Failed,
    /// Branch was still running when completion criteria were met.
    Started,
}

/// Reason the parallel block completed.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::types::CompletionReason;
///
/// let reason = CompletionReason::AllCompleted;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompletionReason {
    /// All branches finished (success or failure).
    AllCompleted,
    /// Minimum successful threshold was reached.
    MinSuccessfulReached,
    /// Too many failures exceeded tolerance.
    FailureToleranceExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn history_entry_serde_round_trip() {
        let entry = HistoryEntry {
            name: "charge_payment".to_string(),
            result: json!({"amount": 99.99, "currency": "USD"}),
            operation_type: OperationType::Step,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn operation_type_serde_round_trip() {
        let variants = [
            OperationType::Step,
            OperationType::Wait,
            OperationType::Callback,
            OperationType::Invoke,
            OperationType::Parallel,
            OperationType::Map,
            OperationType::ChildContext,
            OperationType::Log,
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: OperationType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn execution_mode_equality() {
        assert_eq!(ExecutionMode::Replaying, ExecutionMode::Replaying);
        assert_eq!(ExecutionMode::Executing, ExecutionMode::Executing);
        assert_ne!(ExecutionMode::Replaying, ExecutionMode::Executing);
    }

    #[test]
    fn execution_mode_serde_round_trip() {
        for mode in [ExecutionMode::Replaying, ExecutionMode::Executing] {
            let json = serde_json::to_string(&mode).unwrap();
            let deserialized: ExecutionMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, deserialized);
        }
    }

    #[test]
    fn checkpoint_result_ok_serde_round_trip() {
        let result: CheckpointResult<i32, String> = CheckpointResult::Ok(42);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: CheckpointResult<i32, String> = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }

    #[test]
    fn checkpoint_result_err_serde_round_trip() {
        let result: CheckpointResult<i32, String> =
            CheckpointResult::Err("validation failed".to_string());
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: CheckpointResult<i32, String> = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }

    #[test]
    fn checkpoint_result_with_complex_types() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Order {
            id: u64,
            total: f64,
        }

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct OrderError {
            code: String,
            message: String,
        }

        let ok_result: CheckpointResult<Order, OrderError> = CheckpointResult::Ok(Order {
            id: 1,
            total: 99.99,
        });
        let json = serde_json::to_string(&ok_result).unwrap();
        let deserialized: CheckpointResult<Order, OrderError> =
            serde_json::from_str(&json).unwrap();
        assert_eq!(ok_result, deserialized);

        let err_result: CheckpointResult<Order, OrderError> = CheckpointResult::Err(OrderError {
            code: "INVALID".to_string(),
            message: "Order invalid".to_string(),
        });
        let json = serde_json::to_string(&err_result).unwrap();
        let deserialized: CheckpointResult<Order, OrderError> =
            serde_json::from_str(&json).unwrap();
        assert_eq!(err_result, deserialized);
    }

    #[test]
    fn history_entry_with_all_operation_types() {
        let operation_types = [
            OperationType::Step,
            OperationType::Wait,
            OperationType::Callback,
            OperationType::Invoke,
            OperationType::Parallel,
            OperationType::Map,
            OperationType::ChildContext,
            OperationType::Log,
        ];

        for op_type in operation_types {
            let entry = HistoryEntry {
                name: "test_op".to_string(),
                result: json!(null),
                operation_type: op_type.clone(),
            };

            let json = serde_json::to_string(&entry).unwrap();
            let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();
            assert_eq!(entry, deserialized);
        }
    }
}
