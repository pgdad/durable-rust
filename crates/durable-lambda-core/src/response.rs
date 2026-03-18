//! Durable execution invocation output formatting.
//!
//! The AWS Lambda Durable Execution service requires Lambda handlers to return
//! a specific JSON envelope as their response, rather than the user's plain JSON
//! value. This module provides [`wrap_handler_result`] to convert the handler's
//! `Result<serde_json::Value, DurableError>` into the required format.
//!
//! # Protocol
//!
//! The durable execution service validates the Lambda response and expects one of:
//!
//! - `{"Status": "SUCCEEDED", "Result": "<serialized-user-result>"}` — execution completed
//! - `{"Status": "FAILED", "Error": {"ErrorMessage": "...", "ErrorType": "..."}}` — execution failed
//! - `{"Status": "PENDING"}` — execution suspended (wait/callback/retry/invoke)
//!
//! The `Result` field in the SUCCEEDED case is a **JSON string** (the user's result
//! value serialized to a JSON string, then included as a string field).

use serde_json::json;

use crate::error::DurableError;

/// Wrap a handler result into the durable execution invocation output format.
///
/// Converts `Result<serde_json::Value, DurableError>` into the `{"Status": ...}`
/// envelope that the AWS Lambda Durable Execution service expects as the Lambda
/// function response.
///
/// # Status Mapping
///
/// | Input | Output |
/// |-------|--------|
/// | `Ok(value)` | `{"Status": "SUCCEEDED", "Result": "<serialized value>"}` |
/// | `Err(StepRetryScheduled \| WaitSuspended \| CallbackSuspended \| InvokeSuspended)` | `{"Status": "PENDING"}` |
/// | `Err(other)` | `{"Status": "FAILED", "Error": {"ErrorMessage": "...", "ErrorType": "..."}}` |
///
/// # Returns
///
/// Always returns `Ok(serde_json::Value)` — the caller must never treat this as
/// an error, since the durable execution protocol uses the response body to signal
/// all outcomes including failures.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::response::wrap_handler_result;
/// use durable_lambda_core::error::DurableError;
///
/// // Success case
/// let result = Ok(serde_json::json!({"order_id": "123"}));
/// let output = wrap_handler_result(result).unwrap();
/// assert_eq!(output["Status"], "SUCCEEDED");
///
/// // Suspension case (PENDING)
/// let result: Result<serde_json::Value, DurableError> =
///     Err(DurableError::wait_suspended("cooldown"));
/// let output = wrap_handler_result(result).unwrap();
/// assert_eq!(output["Status"], "PENDING");
///
/// // Failure case
/// let result: Result<serde_json::Value, DurableError> =
///     Err(DurableError::step_timeout("slow_op"));
/// let output = wrap_handler_result(result).unwrap();
/// assert_eq!(output["Status"], "FAILED");
/// ```
pub fn wrap_handler_result(
    result: Result<serde_json::Value, DurableError>,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    match result {
        Ok(value) => {
            // Serialize the user result as a JSON string (double-encoded).
            // The durable execution service expects Result to be a JSON string,
            // not a nested JSON object.
            let result_str = serde_json::to_string(&value)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            Ok(json!({
                "Status": "SUCCEEDED",
                "Result": result_str,
            }))
        }

        Err(ref err) if is_suspension(err) => {
            // Suspension signals: function should exit cleanly so the durable
            // execution service can re-invoke after the timer/callback/retry.
            Ok(json!({ "Status": "PENDING" }))
        }

        Err(err) => {
            // All other errors: execution failed.
            let error_type = err.code().to_string();
            let error_message = err.to_string();
            Ok(json!({
                "Status": "FAILED",
                "Error": {
                    "ErrorType": error_type,
                    "ErrorMessage": error_message,
                },
            }))
        }
    }
}

/// Returns `true` for error variants that indicate the function should suspend
/// rather than fail. Suspended executions return `{"Status": "PENDING"}` so the
/// durable execution service knows to re-invoke after the triggering condition
/// (timer, callback signal, retry delay, or async invoke completion).
fn is_suspension(err: &DurableError) -> bool {
    matches!(
        err,
        DurableError::StepRetryScheduled { .. }
            | DurableError::WaitSuspended { .. }
            | DurableError::CallbackSuspended { .. }
            | DurableError::InvokeSuspended { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DurableError;

    #[test]
    fn success_wraps_in_succeeded_envelope() {
        let value = serde_json::json!({"order_id": "123", "status": "ok"});
        let output = wrap_handler_result(Ok(value.clone())).unwrap();

        assert_eq!(output["Status"], "SUCCEEDED");
        // Result is a JSON string
        let result_str = output["Result"]
            .as_str()
            .expect("Result should be a string");
        let parsed: serde_json::Value = serde_json::from_str(result_str).unwrap();
        assert_eq!(parsed["order_id"], "123");
    }

    #[test]
    fn step_retry_scheduled_returns_pending() {
        let err = DurableError::step_retry_scheduled("charge_payment");
        let output = wrap_handler_result(Err(err)).unwrap();
        assert_eq!(output["Status"], "PENDING");
        assert!(output.get("Error").is_none());
        assert!(output.get("Result").is_none());
    }

    #[test]
    fn wait_suspended_returns_pending() {
        let err = DurableError::wait_suspended("cooldown");
        let output = wrap_handler_result(Err(err)).unwrap();
        assert_eq!(output["Status"], "PENDING");
    }

    #[test]
    fn callback_suspended_returns_pending() {
        let err = DurableError::callback_suspended("approval", "cb-123");
        let output = wrap_handler_result(Err(err)).unwrap();
        assert_eq!(output["Status"], "PENDING");
    }

    #[test]
    fn invoke_suspended_returns_pending() {
        let err = DurableError::invoke_suspended("call_processor");
        let output = wrap_handler_result(Err(err)).unwrap();
        assert_eq!(output["Status"], "PENDING");
    }

    #[test]
    fn step_timeout_returns_failed() {
        let err = DurableError::step_timeout("slow_op");
        let output = wrap_handler_result(Err(err)).unwrap();
        assert_eq!(output["Status"], "FAILED");
        assert!(output.get("Error").is_some());
        let error_obj = &output["Error"];
        assert_eq!(error_obj["ErrorType"], "STEP_TIMEOUT");
        assert!(
            error_obj["ErrorMessage"]
                .as_str()
                .unwrap()
                .contains("timed out"),
            "ErrorMessage should contain 'timed out'"
        );
    }

    #[test]
    fn checkpoint_failed_returns_failed() {
        let err = DurableError::checkpoint_failed(
            "op",
            std::io::Error::new(std::io::ErrorKind::Other, "network error"),
        );
        let output = wrap_handler_result(Err(err)).unwrap();
        assert_eq!(output["Status"], "FAILED");
        assert_eq!(output["Error"]["ErrorType"], "CHECKPOINT_FAILED");
    }

    #[test]
    fn replay_mismatch_returns_failed() {
        let err = DurableError::replay_mismatch("Step", "Wait", 3);
        let output = wrap_handler_result(Err(err)).unwrap();
        assert_eq!(output["Status"], "FAILED");
        assert_eq!(output["Error"]["ErrorType"], "REPLAY_MISMATCH");
    }

    #[test]
    fn failed_status_has_no_result_field() {
        let err = DurableError::step_timeout("op");
        let output = wrap_handler_result(Err(err)).unwrap();
        assert!(output.get("Result").is_none());
    }

    #[test]
    fn succeeded_status_result_is_json_string() {
        // The Result field must be a JSON string, not a nested object.
        let output = wrap_handler_result(Ok(serde_json::json!({"key": "value"}))).unwrap();
        assert_eq!(output["Status"], "SUCCEEDED");
        assert!(
            output["Result"].is_string(),
            "Result must be a JSON string, not an object"
        );
    }

    #[test]
    fn null_value_wraps_correctly() {
        let output = wrap_handler_result(Ok(serde_json::Value::Null)).unwrap();
        assert_eq!(output["Status"], "SUCCEEDED");
        assert_eq!(output["Result"].as_str().unwrap(), "null");
    }
}
