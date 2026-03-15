//! DurableBackend trait and RealBackend implementation.
//!
//! The [`DurableBackend`] trait is the I/O boundary between the replay engine
//! and AWS. It covers the 2 AWS durable execution API operations used internally
//! by the SDK: `checkpoint_durable_execution` and `get_durable_execution_state`.
//!
//! [`RealBackend`] calls AWS via `aws-sdk-lambda`; `MockBackend` (in
//! `durable-lambda-testing`) returns pre-loaded data for credential-free testing.

use std::time::Duration;

use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
use aws_sdk_lambda::types::OperationUpdate;

use crate::error::DurableError;

/// Define the I/O boundary between the replay engine and the durable execution backend.
///
/// This trait abstracts the 2 AWS Lambda durable execution API operations that
/// the SDK uses internally. Implement this trait for real AWS calls
/// ([`RealBackend`]) or for testing ([`MockBackend`] in `durable-lambda-testing`).
///
/// # Object Safety
///
/// This trait is object-safe and designed to be used as
/// `Arc<dyn DurableBackend + Send + Sync>`.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::backend::{DurableBackend, RealBackend};
///
/// // RealBackend implements DurableBackend.
/// fn accepts_backend(_b: &dyn DurableBackend) {}
/// ```
#[async_trait::async_trait]
pub trait DurableBackend: Send + Sync {
    /// Persist checkpoint updates for a durable execution.
    ///
    /// Wraps the `checkpoint_durable_execution` AWS API. Sends a batch of
    /// [`OperationUpdate`] items and receives a new checkpoint token plus
    /// any updated execution state.
    ///
    /// # Errors
    ///
    /// Returns [`DurableError`] if the AWS API call fails after retries.
    async fn checkpoint(
        &self,
        arn: &str,
        checkpoint_token: &str,
        updates: Vec<OperationUpdate>,
        client_token: Option<&str>,
    ) -> Result<CheckpointDurableExecutionOutput, DurableError>;

    /// Get the current operation state of a durable execution (paginated).
    ///
    /// Wraps the `get_durable_execution_state` AWS API. Returns a page of
    /// [`Operation`](aws_sdk_lambda::types::Operation) items and an optional
    /// `next_marker` for pagination.
    ///
    /// # Errors
    ///
    /// Returns [`DurableError`] if the AWS API call fails after retries.
    async fn get_execution_state(
        &self,
        arn: &str,
        checkpoint_token: &str,
        next_marker: &str,
        max_items: i32,
    ) -> Result<GetDurableExecutionStateOutput, DurableError>;
}

/// Real AWS backend that calls `aws-sdk-lambda` durable execution APIs.
///
/// Implements [`DurableBackend`] with exponential backoff retry for transient
/// AWS failures (throttling, server errors, timeouts).
///
/// # Examples
///
/// ```no_run
/// use aws_sdk_lambda::Client;
/// use durable_lambda_core::backend::RealBackend;
///
/// # async fn example() {
/// let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
/// let client = Client::new(&config);
/// let backend = RealBackend::new(client);
/// # }
/// ```
pub struct RealBackend {
    client: aws_sdk_lambda::Client,
}

impl RealBackend {
    /// Create a new `RealBackend` wrapping an `aws-sdk-lambda` client.
    pub fn new(client: aws_sdk_lambda::Client) -> Self {
        Self { client }
    }
}

/// Maximum number of retry attempts for transient AWS failures.
const MAX_RETRIES: u32 = 3;
/// Base delay for exponential backoff (100ms).
const BASE_DELAY_MS: u64 = 100;
/// Maximum delay cap for backoff (2s).
const MAX_DELAY_MS: u64 = 2000;

/// Compute backoff delay with jitter for a given attempt (0-indexed).
///
/// Uses "full jitter" strategy: uniform random delay in `[0, min(cap, base * 2^attempt)]`.
/// Entropy comes from `SystemTime` nanoseconds — sufficient for retry decorrelation,
/// not cryptographic.
fn backoff_delay(attempt: u32) -> Duration {
    let base = BASE_DELAY_MS.saturating_mul(1u64 << attempt);
    let capped = base.min(MAX_DELAY_MS);
    // Use system time nanoseconds as cheap entropy source for jitter.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let jittered = if capped > 0 { nanos % capped } else { 0 };
    Duration::from_millis(jittered)
}

/// Check if an AWS SDK error is retryable (throttling or server error).
fn is_retryable_error(err: &DurableError) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("throttl")
        || msg.contains("rate exceeded")
        || msg.contains("too many requests")
        || msg.contains("service unavailable")
        || msg.contains("internal server error")
        || msg.contains("timed out")
        || msg.contains("timeout")
}

#[async_trait::async_trait]
impl DurableBackend for RealBackend {
    async fn checkpoint(
        &self,
        arn: &str,
        checkpoint_token: &str,
        updates: Vec<OperationUpdate>,
        client_token: Option<&str>,
    ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
        let mut last_err = None;

        for attempt in 0..=MAX_RETRIES {
            let mut builder = self
                .client
                .checkpoint_durable_execution()
                .durable_execution_arn(arn)
                .checkpoint_token(checkpoint_token)
                .set_updates(Some(updates.clone()));

            if let Some(token) = client_token {
                builder = builder.client_token(token);
            }

            match builder.send().await {
                Ok(output) => return Ok(output),
                Err(e) => {
                    let durable_err = DurableError::aws_sdk_operation(e);
                    if attempt < MAX_RETRIES && is_retryable_error(&durable_err) {
                        tokio::time::sleep(backoff_delay(attempt)).await;
                        last_err = Some(durable_err);
                        continue;
                    }
                    return Err(durable_err);
                }
            }
        }

        Err(last_err.unwrap())
    }

    async fn get_execution_state(
        &self,
        arn: &str,
        checkpoint_token: &str,
        next_marker: &str,
        max_items: i32,
    ) -> Result<GetDurableExecutionStateOutput, DurableError> {
        let mut last_err = None;

        for attempt in 0..=MAX_RETRIES {
            let result = self
                .client
                .get_durable_execution_state()
                .durable_execution_arn(arn)
                .checkpoint_token(checkpoint_token)
                .marker(next_marker)
                .max_items(max_items)
                .send()
                .await;

            match result {
                Ok(output) => return Ok(output),
                Err(e) => {
                    let durable_err = DurableError::aws_sdk_operation(e);
                    if attempt < MAX_RETRIES && is_retryable_error(&durable_err) {
                        tokio::time::sleep(backoff_delay(attempt)).await;
                        last_err = Some(durable_err);
                        continue;
                    }
                    return Err(durable_err);
                }
            }
        }

        Err(last_err.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn durable_backend_is_object_safe() {
        // Verify DurableBackend can be used as a trait object.
        fn _accepts_dyn(_b: Arc<dyn DurableBackend>) {}
    }

    #[test]
    fn real_backend_is_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<RealBackend>();
    }

    #[test]
    fn backoff_delay_within_bounds() {
        // Each attempt's delay must be in [0, min(cap, base * 2^attempt)].
        for attempt in 0..=MAX_RETRIES {
            let d = backoff_delay(attempt);
            let base = BASE_DELAY_MS.saturating_mul(1u64 << attempt);
            let capped = base.min(MAX_DELAY_MS);
            assert!(
                d.as_millis() <= capped as u128,
                "attempt {attempt}: delay {}ms exceeds cap {capped}ms",
                d.as_millis()
            );
        }
    }

    #[test]
    fn is_retryable_detects_throttling() {
        let err = DurableError::checkpoint_failed(
            "test",
            std::io::Error::new(std::io::ErrorKind::Other, "Throttling: Rate exceeded"),
        );
        assert!(is_retryable_error(&err));
    }

    #[test]
    fn is_retryable_detects_timeout() {
        let err = DurableError::checkpoint_failed(
            "test",
            std::io::Error::new(std::io::ErrorKind::TimedOut, "connection timed out"),
        );
        assert!(is_retryable_error(&err));
    }

    #[test]
    fn is_retryable_rejects_non_transient() {
        let err = DurableError::replay_mismatch("Step", "Wait", 0);
        assert!(!is_retryable_error(&err));
    }
}
