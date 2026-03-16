//! SDK error types for the durable-lambda framework.
//!
//! Provide a typed [`DurableError`] enum covering all failure modes:
//! replay mismatches, checkpoint failures, serialization errors, and
//! AWS SDK errors. All variants are constructed via static methods,
//! never raw struct syntax.

use std::error::Error as StdError;

/// Represent all errors that can occur within the durable-lambda SDK.
///
/// Each variant carries rich context for diagnosing failures. Variants
/// are constructed via static methods (e.g., [`DurableError::replay_mismatch`])
/// to keep internal fields private.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::error::DurableError;
///
/// // Create a replay mismatch error.
/// let err = DurableError::replay_mismatch("Step", "Wait", 3);
/// assert!(err.to_string().contains("position 3"));
/// assert!(err.to_string().contains("expected Step"));
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DurableError {
    /// A replay operation encountered a different operation type or name
    /// than what was recorded in the execution history.
    #[error("replay mismatch at position {position}: expected {expected}, got {actual}")]
    #[non_exhaustive]
    ReplayMismatch {
        expected: String,
        actual: String,
        position: usize,
    },

    /// A checkpoint write or read operation failed.
    #[error("checkpoint failed for operation '{operation_name}': {source}")]
    #[non_exhaustive]
    CheckpointFailed {
        operation_name: String,
        source: Box<dyn StdError + Send + Sync>,
    },

    /// Serialization of a value to JSON failed.
    #[error("failed to serialize type '{type_name}': {source}")]
    #[non_exhaustive]
    Serialization {
        type_name: String,
        #[source]
        source: serde_json::Error,
    },

    /// Deserialization of a value from JSON failed.
    #[error("failed to deserialize type '{type_name}': {source}")]
    #[non_exhaustive]
    Deserialization {
        type_name: String,
        #[source]
        source: serde_json::Error,
    },

    /// A general AWS SDK error occurred.
    #[error("AWS SDK error: {0}")]
    AwsSdk(Box<aws_sdk_lambda::Error>),

    /// A specific AWS API operation error occurred.
    #[error("AWS operation error: {0}")]
    AwsSdkOperation(#[source] Box<dyn StdError + Send + Sync>),

    /// A step retry has been scheduled — the function should exit.
    ///
    /// The SDK has checkpointed a RETRY action. The durable execution server
    /// will re-invoke the Lambda after the configured delay. The handler must
    /// propagate this error to exit cleanly.
    #[error("step retry scheduled for operation '{operation_name}' — function should exit")]
    #[non_exhaustive]
    StepRetryScheduled { operation_name: String },

    /// A wait operation has been checkpointed — the function should exit.
    ///
    /// The SDK has sent a START checkpoint with the wait duration. The durable
    /// execution server will re-invoke the Lambda after the timer expires.
    /// The handler must propagate this error to exit cleanly.
    #[error("wait suspended for operation '{operation_name}' — function should exit")]
    #[non_exhaustive]
    WaitSuspended { operation_name: String },

    /// A callback is pending — the function should exit and wait for an
    /// external signal.
    ///
    /// The callback has been registered but no success/failure signal has
    /// been received yet. The handler must propagate this error to exit.
    /// The server will re-invoke the Lambda when the callback is signaled.
    #[error("callback suspended for operation '{operation_name}' (callback_id: {callback_id}) — function should exit")]
    #[non_exhaustive]
    CallbackSuspended {
        operation_name: String,
        callback_id: String,
    },

    /// A callback failed, was cancelled, or timed out.
    ///
    /// The external system signaled failure, or the callback exceeded its
    /// configured timeout. The error message contains details from the
    /// callback's error object.
    #[error("callback failed for operation '{operation_name}' (callback_id: {callback_id}): {error_message}")]
    #[non_exhaustive]
    CallbackFailed {
        operation_name: String,
        callback_id: String,
        error_message: String,
    },

    /// An invoke operation is pending — the function should exit while
    /// the target Lambda executes.
    ///
    /// The invoke START checkpoint has been sent. The server will invoke
    /// the target function asynchronously and re-invoke this Lambda when
    /// the target completes.
    #[error("invoke suspended for operation '{operation_name}' — function should exit")]
    #[non_exhaustive]
    InvokeSuspended { operation_name: String },

    /// An invoke operation failed, timed out, or was stopped.
    ///
    /// The target Lambda function returned an error, or the invoke
    /// exceeded its configured timeout.
    #[error("invoke failed for operation '{operation_name}': {error_message}")]
    #[non_exhaustive]
    InvokeFailed {
        operation_name: String,
        error_message: String,
    },

    /// A parallel operation failed.
    ///
    /// One or more branches encountered an unrecoverable error during
    /// concurrent execution.
    #[error("parallel failed for operation '{operation_name}': {error_message}")]
    #[non_exhaustive]
    ParallelFailed {
        operation_name: String,
        error_message: String,
    },

    /// A map operation failed.
    ///
    /// An unrecoverable error occurred during map collection processing.
    /// Individual item failures are captured in the [`BatchResult`](crate::types::BatchResult)
    /// rather than propagated as this error.
    #[error("map failed for operation '{operation_name}': {error_message}")]
    #[non_exhaustive]
    MapFailed {
        operation_name: String,
        error_message: String,
    },

    /// A child context operation failed.
    ///
    /// The child context's closure returned an error, or the child context
    /// was found in a failed/cancelled/timed-out state during replay.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::child_context_failed("sub_workflow", "closure returned error");
    /// assert!(err.to_string().contains("sub_workflow"));
    /// assert!(err.to_string().contains("closure returned error"));
    /// ```
    #[error("child context failed for operation '{operation_name}': {error_message}")]
    #[non_exhaustive]
    ChildContextFailed {
        operation_name: String,
        error_message: String,
    },

    /// A step exceeded its configured timeout.
    ///
    /// The step closure did not complete within the duration configured via
    /// [`StepOptions::timeout_seconds`](crate::types::StepOptions::timeout_seconds).
    /// The spawned task is aborted and this error is returned immediately.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::step_timeout("my_op");
    /// assert!(err.to_string().contains("my_op"));
    /// assert!(err.to_string().contains("timed out"));
    /// assert_eq!(err.code(), "STEP_TIMEOUT");
    /// ```
    #[error("step timed out for operation '{operation_name}'")]
    #[non_exhaustive]
    StepTimeout { operation_name: String },
}

impl DurableError {
    /// Create a replay mismatch error.
    ///
    /// Use when the replay engine encounters an operation at a history position
    /// that doesn't match the expected operation type or name.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::replay_mismatch("Step", "Wait", 5);
    /// assert!(err.to_string().contains("expected Step"));
    /// assert!(err.to_string().contains("got Wait"));
    /// assert!(err.to_string().contains("position 5"));
    /// ```
    pub fn replay_mismatch(
        expected: impl Into<String>,
        actual: impl Into<String>,
        position: usize,
    ) -> Self {
        Self::ReplayMismatch {
            expected: expected.into(),
            actual: actual.into(),
            position,
        }
    }

    /// Create a checkpoint failure error.
    ///
    /// Use when writing or reading a checkpoint to/from the durable
    /// execution backend fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    /// use std::io;
    ///
    /// let source = io::Error::new(io::ErrorKind::TimedOut, "connection timed out");
    /// let err = DurableError::checkpoint_failed("charge_payment", source);
    /// assert!(err.to_string().contains("charge_payment"));
    /// ```
    pub fn checkpoint_failed(
        operation_name: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::CheckpointFailed {
            operation_name: operation_name.into(),
            source: Box::new(source),
        }
    }

    /// Create a serialization error.
    ///
    /// Use when `serde_json::to_value` or `serde_json::to_string` fails
    /// for a checkpoint value.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// // Simulate a serde error by deserializing invalid JSON.
    /// let serde_err = serde_json::from_str::<i32>("not a number").unwrap_err();
    /// let err = DurableError::serialization("OrderPayload", serde_err);
    /// assert!(err.to_string().contains("OrderPayload"));
    /// ```
    pub fn serialization(type_name: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Serialization {
            type_name: type_name.into(),
            source,
        }
    }

    /// Create a deserialization error.
    ///
    /// Use when `serde_json::from_value` or `serde_json::from_str` fails
    /// for a cached checkpoint value during replay.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// // Simulate a serde error by deserializing invalid JSON.
    /// let serde_err = serde_json::from_str::<i32>("not a number").unwrap_err();
    /// let err = DurableError::deserialization("OrderResult", serde_err);
    /// assert!(err.to_string().contains("OrderResult"));
    /// assert!(err.to_string().contains("deserialize"));
    /// ```
    pub fn deserialization(type_name: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Deserialization {
            type_name: type_name.into(),
            source,
        }
    }

    /// Create an AWS API operation error.
    ///
    /// Use for specific AWS API call failures (e.g., `SdkError` from
    /// individual operations) that are distinct from the general
    /// `aws_sdk_lambda::Error`.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    /// use std::io;
    ///
    /// let source = io::Error::new(io::ErrorKind::Other, "service unavailable");
    /// let err = DurableError::aws_sdk_operation(source);
    /// assert!(err.to_string().contains("service unavailable"));
    /// ```
    pub fn aws_sdk_operation(source: impl StdError + Send + Sync + 'static) -> Self {
        Self::AwsSdkOperation(Box::new(source))
    }

    /// Create a step retry scheduled signal.
    ///
    /// Use when a step has been checkpointed with RETRY and the function
    /// should exit so the server can re-invoke after the configured delay.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::step_retry_scheduled("charge_payment");
    /// assert!(err.to_string().contains("charge_payment"));
    /// ```
    pub fn step_retry_scheduled(operation_name: impl Into<String>) -> Self {
        Self::StepRetryScheduled {
            operation_name: operation_name.into(),
        }
    }

    /// Create a wait suspended signal.
    ///
    /// Use when a wait operation has been checkpointed with START and the
    /// function should exit so the server can re-invoke after the timer.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::wait_suspended("cooldown_delay");
    /// assert!(err.to_string().contains("cooldown_delay"));
    /// ```
    pub fn wait_suspended(operation_name: impl Into<String>) -> Self {
        Self::WaitSuspended {
            operation_name: operation_name.into(),
        }
    }

    /// Create a callback suspended signal.
    ///
    /// Use when a callback has been registered but not yet signaled.
    /// The handler must propagate this to exit so the server can wait
    /// for the external signal.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::callback_suspended("approval", "cb-123");
    /// assert!(err.to_string().contains("approval"));
    /// assert!(err.to_string().contains("cb-123"));
    /// ```
    pub fn callback_suspended(
        operation_name: impl Into<String>,
        callback_id: impl Into<String>,
    ) -> Self {
        Self::CallbackSuspended {
            operation_name: operation_name.into(),
            callback_id: callback_id.into(),
        }
    }

    /// Create a callback failed error.
    ///
    /// Use when a callback was signaled with failure, cancelled, or timed out.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::callback_failed("approval", "cb-123", "rejected by reviewer");
    /// assert!(err.to_string().contains("approval"));
    /// assert!(err.to_string().contains("cb-123"));
    /// assert!(err.to_string().contains("rejected by reviewer"));
    /// ```
    pub fn callback_failed(
        operation_name: impl Into<String>,
        callback_id: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Self {
        Self::CallbackFailed {
            operation_name: operation_name.into(),
            callback_id: callback_id.into(),
            error_message: error_message.into(),
        }
    }

    /// Create an invoke suspended signal.
    ///
    /// Use when an invoke START checkpoint has been sent and the function
    /// should exit while the target Lambda executes.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::invoke_suspended("call_processor");
    /// assert!(err.to_string().contains("call_processor"));
    /// ```
    pub fn invoke_suspended(operation_name: impl Into<String>) -> Self {
        Self::InvokeSuspended {
            operation_name: operation_name.into(),
        }
    }

    /// Create an invoke failed error.
    ///
    /// Use when the target Lambda returned an error, timed out, or was stopped.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::invoke_failed("call_processor", "target function timed out");
    /// assert!(err.to_string().contains("call_processor"));
    /// assert!(err.to_string().contains("timed out"));
    /// ```
    pub fn invoke_failed(
        operation_name: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Self {
        Self::InvokeFailed {
            operation_name: operation_name.into(),
            error_message: error_message.into(),
        }
    }

    /// Create a parallel failed error.
    ///
    /// Use when a parallel operation encounters an unrecoverable error.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::parallel_failed("fan_out", "branch 2 panicked");
    /// assert!(err.to_string().contains("fan_out"));
    /// ```
    pub fn parallel_failed(
        operation_name: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Self {
        Self::ParallelFailed {
            operation_name: operation_name.into(),
            error_message: error_message.into(),
        }
    }

    /// Create a map failed error.
    ///
    /// Use when a map operation encounters an unrecoverable error (e.g.,
    /// checkpoint failure, task panic). Individual item failures are captured
    /// in [`BatchResult`](crate::types::BatchResult), not as this error.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::map_failed("process_orders", "item 3 panicked");
    /// assert!(err.to_string().contains("process_orders"));
    /// assert!(err.to_string().contains("item 3 panicked"));
    /// ```
    pub fn map_failed(operation_name: impl Into<String>, error_message: impl Into<String>) -> Self {
        Self::MapFailed {
            operation_name: operation_name.into(),
            error_message: error_message.into(),
        }
    }

    /// Create a child context failed error.
    ///
    /// Use when a child context operation fails during execution or is
    /// found in a failed state during replay.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::child_context_failed("sub_workflow", "closure returned error");
    /// assert!(err.to_string().contains("sub_workflow"));
    /// assert!(err.to_string().contains("closure returned error"));
    /// ```
    pub fn child_context_failed(
        operation_name: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Self {
        Self::ChildContextFailed {
            operation_name: operation_name.into(),
            error_message: error_message.into(),
        }
    }

    /// Create a step timeout error.
    ///
    /// Use when a step closure exceeds the configured `timeout_seconds` duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::step_timeout("my_op");
    /// assert!(err.to_string().contains("my_op"));
    /// assert_eq!(err.code(), "STEP_TIMEOUT");
    /// ```
    pub fn step_timeout(operation_name: impl Into<String>) -> Self {
        Self::StepTimeout {
            operation_name: operation_name.into(),
        }
    }

    /// Return a stable, programmatic error code for this error variant.
    ///
    /// Codes are SCREAMING_SNAKE_CASE and stable across versions.
    /// Use these for programmatic error matching instead of parsing
    /// display messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::replay_mismatch("Step", "Wait", 0);
    /// assert_eq!(err.code(), "REPLAY_MISMATCH");
    /// ```
    pub fn code(&self) -> &'static str {
        match self {
            Self::ReplayMismatch { .. } => "REPLAY_MISMATCH",
            Self::CheckpointFailed { .. } => "CHECKPOINT_FAILED",
            Self::Serialization { .. } => "SERIALIZATION",
            Self::Deserialization { .. } => "DESERIALIZATION",
            Self::AwsSdk(_) => "AWS_SDK",
            Self::AwsSdkOperation(_) => "AWS_SDK_OPERATION",
            Self::StepRetryScheduled { .. } => "STEP_RETRY_SCHEDULED",
            Self::WaitSuspended { .. } => "WAIT_SUSPENDED",
            Self::CallbackSuspended { .. } => "CALLBACK_SUSPENDED",
            Self::CallbackFailed { .. } => "CALLBACK_FAILED",
            Self::InvokeSuspended { .. } => "INVOKE_SUSPENDED",
            Self::InvokeFailed { .. } => "INVOKE_FAILED",
            Self::ParallelFailed { .. } => "PARALLEL_FAILED",
            Self::MapFailed { .. } => "MAP_FAILED",
            Self::ChildContextFailed { .. } => "CHILD_CONTEXT_FAILED",
            Self::StepTimeout { .. } => "STEP_TIMEOUT",
        }
    }
}

impl From<aws_sdk_lambda::Error> for DurableError {
    fn from(err: aws_sdk_lambda::Error) -> Self {
        Self::AwsSdk(Box::new(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::error::Error;

    // --- TDD RED: .code() method tests ---

    #[test]
    fn error_code_replay_mismatch() {
        let err = DurableError::replay_mismatch("A", "B", 0);
        assert_eq!(err.code(), "REPLAY_MISMATCH");
    }

    #[test]
    fn all_error_variants_have_unique_codes() {
        let serde_err = || serde_json::from_str::<i32>("bad").unwrap_err();
        let io_err = || std::io::Error::new(std::io::ErrorKind::Other, "test");

        // Construct all 15 testable variants (AwsSdk excluded — no public constructor).
        let variants: &[(DurableError, &str)] = &[
            (
                DurableError::replay_mismatch("A", "B", 0),
                "REPLAY_MISMATCH",
            ),
            (
                DurableError::checkpoint_failed("op", io_err()),
                "CHECKPOINT_FAILED",
            ),
            (
                DurableError::serialization("T", serde_err()),
                "SERIALIZATION",
            ),
            (
                DurableError::deserialization("T", serde_err()),
                "DESERIALIZATION",
            ),
            (
                DurableError::aws_sdk_operation(io_err()),
                "AWS_SDK_OPERATION",
            ),
            (
                DurableError::step_retry_scheduled("op"),
                "STEP_RETRY_SCHEDULED",
            ),
            (DurableError::wait_suspended("op"), "WAIT_SUSPENDED"),
            (
                DurableError::callback_suspended("op", "cb-1"),
                "CALLBACK_SUSPENDED",
            ),
            (
                DurableError::callback_failed("op", "cb-1", "msg"),
                "CALLBACK_FAILED",
            ),
            (DurableError::invoke_suspended("op"), "INVOKE_SUSPENDED"),
            (DurableError::invoke_failed("op", "msg"), "INVOKE_FAILED"),
            (
                DurableError::parallel_failed("op", "msg"),
                "PARALLEL_FAILED",
            ),
            (DurableError::map_failed("op", "msg"), "MAP_FAILED"),
            (
                DurableError::child_context_failed("op", "msg"),
                "CHILD_CONTEXT_FAILED",
            ),
            (DurableError::step_timeout("op"), "STEP_TIMEOUT"),
        ];

        let mut codes = HashSet::new();
        for (err, expected_code) in variants {
            let actual = err.code();
            assert_eq!(
                actual, *expected_code,
                "Expected code {:?} for variant but got {:?}",
                expected_code, actual
            );
            let inserted = codes.insert(actual);
            assert!(inserted, "Duplicate error code found: {:?}", actual);
        }

        // Verify AWS_SDK code is also unique (compile-time exhaustive match guarantees it exists).
        // We add it manually to the uniqueness check.
        assert!(
            !codes.contains("AWS_SDK"),
            "AWS_SDK code must be unique among all codes"
        );
    }

    // --- StepTimeout tests (TDD RED) ---

    #[test]
    fn step_timeout_error_code() {
        let err = DurableError::step_timeout("my_op");
        assert_eq!(err.code(), "STEP_TIMEOUT");
    }

    #[test]
    fn step_timeout_display_contains_op_and_timed_out() {
        let err = DurableError::step_timeout("my_op");
        let msg = err.to_string();
        assert!(
            msg.contains("my_op"),
            "display should contain operation name, got: {msg}"
        );
        assert!(
            msg.contains("timed out"),
            "display should contain 'timed out', got: {msg}"
        );
    }

    // --- existing tests ---

    #[test]
    fn replay_mismatch_display() {
        let err = DurableError::replay_mismatch("Step", "Wait", 3);
        let msg = err.to_string();
        assert!(msg.contains("position 3"));
        assert!(msg.contains("expected Step"));
        assert!(msg.contains("got Wait"));
    }

    #[test]
    fn replay_mismatch_accepts_string_types() {
        let err = DurableError::replay_mismatch(String::from("Step"), "Wait".to_string(), 0);
        assert!(err.to_string().contains("expected Step"));
    }

    #[test]
    fn checkpoint_failed_display_and_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");
        let err = DurableError::checkpoint_failed("charge_payment", io_err);
        let msg = err.to_string();
        assert!(msg.contains("charge_payment"));
        assert!(msg.contains("timed out"));
        assert!(err.source().is_some());
    }

    #[test]
    fn serialization_display_and_source() {
        let serde_err = serde_json::from_str::<i32>("bad").unwrap_err();
        let err = DurableError::serialization("MyType", serde_err);
        let msg = err.to_string();
        assert!(msg.contains("serialize"));
        assert!(msg.contains("MyType"));
        assert!(err.source().is_some());
    }

    #[test]
    fn deserialization_display_and_source() {
        let serde_err = serde_json::from_str::<i32>("bad").unwrap_err();
        let err = DurableError::deserialization("MyType", serde_err);
        let msg = err.to_string();
        assert!(msg.contains("deserialize"));
        assert!(msg.contains("MyType"));
        assert!(err.source().is_some());
    }

    #[test]
    fn aws_sdk_operation_display_and_source() {
        let source = std::io::Error::new(std::io::ErrorKind::Other, "service unavailable");
        let err = DurableError::aws_sdk_operation(source);
        let msg = err.to_string();
        assert!(msg.contains("service unavailable"));
        assert!(err.source().is_some());
    }

    #[test]
    fn aws_sdk_error_from_conversion() {
        // Verify that aws_sdk_lambda::Error can be converted via From.
        // We can't easily construct a real aws_sdk_lambda::Error, but we can
        // verify the From impl exists at compile time by checking the type.
        fn _assert_from_impl<T: From<aws_sdk_lambda::Error>>() {}
        _assert_from_impl::<DurableError>();
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DurableError>();
    }
}
