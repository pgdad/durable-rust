//! Event parsing helpers for durable execution Lambda payloads.
//!
//! Parse the `InitialExecutionState` envelope that AWS sends to durable Lambda
//! handlers. These helpers are shared across all API approach crates (closure,
//! macro, trait, builder) so event parsing logic stays in one place.

use aws_sdk_lambda::types::{Operation, OperationStatus, OperationType, StepDetails};

/// Parse operations from the `InitialExecutionState` JSON payload.
///
/// Constructs [`Operation`] objects from the JSON `"Operations"` array using
/// the builder pattern. Operations that cannot be fully parsed are silently
/// skipped.
///
/// # Examples
///
/// ```
/// let state = serde_json::json!({
///     "Operations": [{
///         "Id": "op-1",
///         "Type": "Step",
///         "Status": "Succeeded",
///         "StartTimestamp": 1700000000.0,
///         "StepDetails": { "Result": "{\"ok\":true}" }
///     }]
/// });
/// let ops = durable_lambda_core::event::parse_operations(&state);
/// assert_eq!(ops.len(), 1);
/// ```
pub fn parse_operations(initial_state: &serde_json::Value) -> Vec<Operation> {
    let Some(ops_array) = initial_state["Operations"].as_array() else {
        return vec![];
    };

    ops_array
        .iter()
        .filter_map(|op_json| {
            let id = op_json["Id"].as_str()?;
            let op_type = parse_operation_type(op_json["Type"].as_str()?)?;
            let status = parse_operation_status(op_json["Status"].as_str()?)?;

            let timestamp = op_json["StartTimestamp"]
                .as_f64()
                .map(aws_smithy_types::DateTime::from_secs_f64)
                .unwrap_or_else(|| aws_smithy_types::DateTime::from_secs(0));

            let mut builder = Operation::builder()
                .id(id)
                .r#type(op_type)
                .status(status)
                .start_timestamp(timestamp);

            // Parse step details if present.
            if let Some(step_details_json) = op_json.get("StepDetails") {
                let mut sd_builder = StepDetails::builder();

                if let Some(result) = step_details_json["Result"].as_str() {
                    sd_builder = sd_builder.result(result);
                }

                if let Some(error_json) = step_details_json.get("Error") {
                    if let (Some(error_type), Some(error_data)) = (
                        error_json["ErrorType"].as_str(),
                        error_json["ErrorData"].as_str(),
                    ) {
                        sd_builder = sd_builder.error(
                            aws_sdk_lambda::types::ErrorObject::builder()
                                .error_type(error_type)
                                .error_data(error_data)
                                .build(),
                        );
                    }
                }

                if let Some(attempt) = step_details_json["Attempt"].as_i64() {
                    sd_builder = sd_builder.attempt(attempt as i32);
                }

                builder = builder.step_details(sd_builder.build());
            }

            // Parse execution details if present.
            if let Some(exec_json) = op_json.get("ExecutionDetails") {
                let mut ed_builder = aws_sdk_lambda::types::ExecutionDetails::builder();
                if let Some(input) = exec_json["InputPayload"].as_str() {
                    ed_builder = ed_builder.input_payload(input);
                }
                builder = builder.execution_details(ed_builder.build());
            }

            builder.build().ok()
        })
        .collect()
}

/// Parse an operation type string into the AWS SDK enum.
///
/// Accepts both PascalCase (`"Step"`) and UPPER_CASE (`"STEP"`) variants.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::event::parse_operation_type;
/// use aws_sdk_lambda::types::OperationType;
///
/// assert_eq!(parse_operation_type("Step"), Some(OperationType::Step));
/// assert_eq!(parse_operation_type("EXECUTION"), Some(OperationType::Execution));
/// assert_eq!(parse_operation_type("unknown"), None);
/// ```
pub fn parse_operation_type(s: &str) -> Option<OperationType> {
    match s {
        "Step" | "STEP" => Some(OperationType::Step),
        "Execution" | "EXECUTION" => Some(OperationType::Execution),
        "Wait" | "WAIT" => Some(OperationType::Wait),
        "Callback" | "CALLBACK" => Some(OperationType::Callback),
        "ChainedInvoke" | "CHAINED_INVOKE" => Some(OperationType::ChainedInvoke),
        _ => None,
    }
}

/// Parse an operation status string into the AWS SDK enum.
///
/// Accepts both PascalCase (`"Succeeded"`) and UPPER_CASE (`"SUCCEEDED"`) variants.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::event::parse_operation_status;
/// use aws_sdk_lambda::types::OperationStatus;
///
/// assert_eq!(parse_operation_status("Succeeded"), Some(OperationStatus::Succeeded));
/// assert_eq!(parse_operation_status("PENDING"), Some(OperationStatus::Pending));
/// assert_eq!(parse_operation_status("unknown"), None);
/// ```
pub fn parse_operation_status(s: &str) -> Option<OperationStatus> {
    match s {
        "Succeeded" | "SUCCEEDED" => Some(OperationStatus::Succeeded),
        "Failed" | "FAILED" => Some(OperationStatus::Failed),
        "Pending" | "PENDING" => Some(OperationStatus::Pending),
        "Ready" | "READY" => Some(OperationStatus::Ready),
        "Started" | "STARTED" => Some(OperationStatus::Started),
        _ => None,
    }
}

/// Structured data extracted from a durable Lambda invocation payload.
///
/// Contains all fields needed to construct a [`DurableContext`](crate::context::DurableContext):
/// the execution ARN, checkpoint token, initial operations, pagination marker,
/// and the user's original event payload.
///
/// # Examples
///
/// ```
/// let payload = serde_json::json!({
///     "DurableExecutionArn": "arn:aws:lambda:us-east-1:123:durable-execution/test",
///     "CheckpointToken": "tok-1",
///     "InitialExecutionState": {
///         "Operations": [],
///         "NextMarker": ""
///     }
/// });
/// let data = durable_lambda_core::event::parse_invocation(&payload).unwrap();
/// assert_eq!(data.durable_execution_arn, "arn:aws:lambda:us-east-1:123:durable-execution/test");
/// ```
#[derive(Debug)]
pub struct InvocationData {
    /// The durable execution ARN from the event envelope.
    pub durable_execution_arn: String,
    /// The initial checkpoint token from the event envelope.
    pub checkpoint_token: String,
    /// Parsed operations from InitialExecutionState.
    pub operations: Vec<aws_sdk_lambda::types::Operation>,
    /// Pagination marker for additional operations pages (None if all loaded).
    pub next_marker: Option<String>,
    /// The user's original event payload extracted from the Execution operation.
    pub user_event: serde_json::Value,
}

/// Parse all durable execution fields from a Lambda event payload.
///
/// Extracts ARN, checkpoint token, initial operations, pagination marker,
/// and user event from the standard durable Lambda event envelope.
///
/// # Arguments
///
/// * `payload` - The raw Lambda event payload as JSON
///
/// # Errors
///
/// Returns `Err(&'static str)` if `DurableExecutionArn` or `CheckpointToken`
/// is missing from the payload.
///
/// # Examples
///
/// ```
/// let payload = serde_json::json!({
///     "DurableExecutionArn": "arn:aws:lambda:us-east-1:123:durable-execution/test",
///     "CheckpointToken": "tok-1",
///     "InitialExecutionState": {
///         "Operations": [{
///             "Id": "exec-1",
///             "Type": "Execution",
///             "Status": "Started",
///             "ExecutionDetails": { "InputPayload": "{\"order_id\": 42}" }
///         }]
///     }
/// });
/// let data = durable_lambda_core::event::parse_invocation(&payload).unwrap();
/// assert_eq!(data.checkpoint_token, "tok-1");
/// assert_eq!(data.user_event["order_id"], 42);
/// ```
pub fn parse_invocation(payload: &serde_json::Value) -> Result<InvocationData, &'static str> {
    let durable_execution_arn = payload["DurableExecutionArn"]
        .as_str()
        .ok_or("missing DurableExecutionArn in event")?
        .to_string();

    let checkpoint_token = payload["CheckpointToken"]
        .as_str()
        .ok_or("missing CheckpointToken in event")?
        .to_string();

    let initial_state = &payload["InitialExecutionState"];
    let operations = parse_operations(initial_state);

    let next_marker = initial_state["NextMarker"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let user_event = extract_user_event(initial_state);

    Ok(InvocationData {
        durable_execution_arn,
        checkpoint_token,
        operations,
        next_marker,
        user_event,
    })
}

/// Extract the user's original event payload from the `InitialExecutionState`.
///
/// The first operation with type `EXECUTION` contains the user's input payload
/// in its `ExecutionDetails.InputPayload` field. If not found or unparsable,
/// returns an empty JSON object.
///
/// # Examples
///
/// ```
/// let state = serde_json::json!({
///     "Operations": [{
///         "Id": "exec-1",
///         "Type": "Execution",
///         "Status": "Started",
///         "ExecutionDetails": {
///             "InputPayload": "{\"order_id\": 42}"
///         }
///     }]
/// });
/// let event = durable_lambda_core::event::extract_user_event(&state);
/// assert_eq!(event["order_id"], 42);
/// ```
pub fn extract_user_event(initial_state: &serde_json::Value) -> serde_json::Value {
    if let Some(ops) = initial_state["Operations"].as_array() {
        for op in ops {
            if op["Type"].as_str() == Some("Execution") || op["Type"].as_str() == Some("EXECUTION")
            {
                if let Some(input) = op
                    .get("ExecutionDetails")
                    .and_then(|ed| ed["InputPayload"].as_str())
                {
                    if let Ok(parsed) = serde_json::from_str(input) {
                        return parsed;
                    }
                }
            }
        }
    }
    serde_json::Value::Object(serde_json::Map::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_operations() {
        let state = serde_json::json!({});
        assert!(parse_operations(&state).is_empty());
    }

    #[test]
    fn parse_operations_with_step() {
        let state = serde_json::json!({
            "Operations": [{
                "Id": "step-1",
                "Type": "Step",
                "Status": "Succeeded",
                "StartTimestamp": 1700000000.0,
                "StepDetails": {
                    "Result": "{\"value\": 42}",
                    "Attempt": 1
                }
            }]
        });
        let ops = parse_operations(&state);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].id(), "step-1");
    }

    #[test]
    fn parse_operations_skips_invalid() {
        let state = serde_json::json!({
            "Operations": [
                { "Id": "good", "Type": "Step", "Status": "Succeeded" },
                { "Id": "bad", "Type": "Unknown", "Status": "Succeeded" },
            ]
        });
        let ops = parse_operations(&state);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].id(), "good");
    }

    #[test]
    fn parse_operation_type_all_variants() {
        assert_eq!(parse_operation_type("Step"), Some(OperationType::Step));
        assert_eq!(parse_operation_type("STEP"), Some(OperationType::Step));
        assert_eq!(
            parse_operation_type("Execution"),
            Some(OperationType::Execution)
        );
        assert_eq!(
            parse_operation_type("EXECUTION"),
            Some(OperationType::Execution)
        );
        assert_eq!(parse_operation_type("Wait"), Some(OperationType::Wait));
        assert_eq!(parse_operation_type("WAIT"), Some(OperationType::Wait));
        assert_eq!(
            parse_operation_type("Callback"),
            Some(OperationType::Callback)
        );
        assert_eq!(
            parse_operation_type("CALLBACK"),
            Some(OperationType::Callback)
        );
        assert_eq!(
            parse_operation_type("ChainedInvoke"),
            Some(OperationType::ChainedInvoke)
        );
        assert_eq!(
            parse_operation_type("CHAINED_INVOKE"),
            Some(OperationType::ChainedInvoke)
        );
        assert_eq!(parse_operation_type("bogus"), None);
    }

    #[test]
    fn parse_operation_status_all_variants() {
        assert_eq!(
            parse_operation_status("Succeeded"),
            Some(OperationStatus::Succeeded)
        );
        assert_eq!(
            parse_operation_status("SUCCEEDED"),
            Some(OperationStatus::Succeeded)
        );
        assert_eq!(
            parse_operation_status("Failed"),
            Some(OperationStatus::Failed)
        );
        assert_eq!(
            parse_operation_status("Pending"),
            Some(OperationStatus::Pending)
        );
        assert_eq!(
            parse_operation_status("Ready"),
            Some(OperationStatus::Ready)
        );
        assert_eq!(
            parse_operation_status("Started"),
            Some(OperationStatus::Started)
        );
        assert_eq!(parse_operation_status("bogus"), None);
    }

    #[test]
    fn extract_user_event_from_execution_op() {
        let state = serde_json::json!({
            "Operations": [{
                "Id": "exec-1",
                "Type": "Execution",
                "Status": "Started",
                "ExecutionDetails": {
                    "InputPayload": "{\"order_id\": 42}"
                }
            }]
        });
        let event = extract_user_event(&state);
        assert_eq!(event["order_id"], 42);
    }

    #[test]
    fn extract_user_event_returns_empty_when_missing() {
        let state = serde_json::json!({ "Operations": [] });
        let event = extract_user_event(&state);
        assert!(event.as_object().unwrap().is_empty());
    }

    #[test]
    fn extract_user_event_handles_uppercase_type() {
        let state = serde_json::json!({
            "Operations": [{
                "Id": "exec-1",
                "Type": "EXECUTION",
                "Status": "STARTED",
                "ExecutionDetails": {
                    "InputPayload": "{\"key\": \"value\"}"
                }
            }]
        });
        let event = extract_user_event(&state);
        assert_eq!(event["key"], "value");
    }

    #[test]
    fn parse_invocation_valid_complete_payload() {
        let payload = serde_json::json!({
            "DurableExecutionArn": "arn:aws:lambda:us-east-1:123:durable-execution/test",
            "CheckpointToken": "tok-abc",
            "InitialExecutionState": {
                "Operations": [{
                    "Id": "exec-1",
                    "Type": "Execution",
                    "Status": "Started",
                    "ExecutionDetails": { "InputPayload": "{\"order_id\": 99}" }
                }],
                "NextMarker": "page-2"
            }
        });
        let data = parse_invocation(&payload).unwrap();
        assert_eq!(
            data.durable_execution_arn,
            "arn:aws:lambda:us-east-1:123:durable-execution/test"
        );
        assert_eq!(data.checkpoint_token, "tok-abc");
        assert_eq!(data.operations.len(), 1);
        assert_eq!(data.next_marker, Some("page-2".to_string()));
        assert_eq!(data.user_event["order_id"], 99);
    }

    #[test]
    fn parse_invocation_missing_arn_returns_error() {
        let payload = serde_json::json!({
            "CheckpointToken": "tok-1",
            "InitialExecutionState": { "Operations": [] }
        });
        let result = parse_invocation(&payload);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "missing DurableExecutionArn in event");
    }

    #[test]
    fn parse_invocation_missing_token_returns_error() {
        let payload = serde_json::json!({
            "DurableExecutionArn": "arn:aws:lambda:us-east-1:123:durable-execution/test",
            "InitialExecutionState": { "Operations": [] }
        });
        let result = parse_invocation(&payload);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "missing CheckpointToken in event");
    }

    #[test]
    fn parse_invocation_empty_next_marker_produces_none() {
        let payload = serde_json::json!({
            "DurableExecutionArn": "arn:aws:lambda:us-east-1:123:durable-execution/test",
            "CheckpointToken": "tok-1",
            "InitialExecutionState": {
                "Operations": [],
                "NextMarker": ""
            }
        });
        let data = parse_invocation(&payload).unwrap();
        assert_eq!(data.next_marker, None);
    }

    #[test]
    fn parse_invocation_nonempty_next_marker_produces_some() {
        let payload = serde_json::json!({
            "DurableExecutionArn": "arn:aws:lambda:us-east-1:123:durable-execution/test",
            "CheckpointToken": "tok-1",
            "InitialExecutionState": {
                "Operations": [],
                "NextMarker": "cursor-xyz"
            }
        });
        let data = parse_invocation(&payload).unwrap();
        assert_eq!(data.next_marker, Some("cursor-xyz".to_string()));
    }
}
