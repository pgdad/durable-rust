//! Replay engine — operation-keyed state with visited tracking.
//!
//! Implement FR1-FR5: history loading, replay/execute mode detection,
//! cached result return, checkpoint execution, and replay status transitions.
//!
//! The replay engine uses a `HashMap<String, Operation>` keyed by operation ID
//! (matching the Python SDK's approach) and tracks which operations have been
//! visited. The replay status transitions from `Replaying` to `Executing` when
//! all completed operations in history have been visited.

use std::collections::{HashMap, HashSet};

use aws_sdk_lambda::types::{Operation, OperationStatus};

use crate::operation_id::OperationIdGenerator;
use crate::types::ExecutionMode;

/// Manage replay state for a durable execution.
///
/// The engine holds the complete operation state loaded from AWS, tracks which
/// operations have been visited during the current invocation, and determines
/// whether the execution is replaying cached results or executing new work.
///
/// # Replay Status Transitions
///
/// - Starts in [`ExecutionMode::Replaying`] if completed operations exist in history.
/// - Starts in [`ExecutionMode::Executing`] if history is empty or has no completed operations.
/// - Transitions from `Replaying` to `Executing` when all completed operations
///   have been visited via [`track_replay`](Self::track_replay).
///
/// # Examples
///
/// ```
/// use durable_lambda_core::replay::ReplayEngine;
/// use durable_lambda_core::types::ExecutionMode;
/// use std::collections::HashMap;
///
/// // Empty history → starts in Executing mode.
/// let engine = ReplayEngine::new(HashMap::new(), None);
/// assert_eq!(engine.execution_mode(), ExecutionMode::Executing);
/// ```
pub struct ReplayEngine {
    /// All operations from the durable execution state, keyed by operation ID.
    operations: HashMap<String, Operation>,
    /// Operation IDs that have been visited during the current invocation.
    visited: HashSet<String>,
    /// IDs of operations with completed statuses (cached at init for perf).
    completed_ids: HashSet<String>,
    /// Current replay/execute mode.
    mode: ExecutionMode,
    /// Deterministic operation ID generator.
    id_generator: OperationIdGenerator,
}

/// Check whether an operation status represents a completed state.
///
/// Completed statuses: `Succeeded`, `Failed`, `Cancelled`, `TimedOut`, `Stopped`.
fn is_completed_status(status: &OperationStatus) -> bool {
    matches!(
        status,
        OperationStatus::Succeeded
            | OperationStatus::Failed
            | OperationStatus::Cancelled
            | OperationStatus::TimedOut
            | OperationStatus::Stopped
    )
}

impl ReplayEngine {
    /// Create a new replay engine from loaded operations.
    ///
    /// Sets the initial [`ExecutionMode`] based on whether completed operations
    /// exist in the history. Operations with type `Execution` are excluded from
    /// replay tracking (they represent the root invocation, not user operations).
    ///
    /// # Arguments
    ///
    /// * `operations` — All operations from the durable execution state, keyed by ID.
    /// * `parent_id` — Parent operation ID for child context scoping (`None` for root).
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::replay::ReplayEngine;
    /// use durable_lambda_core::types::ExecutionMode;
    /// use std::collections::HashMap;
    ///
    /// let engine = ReplayEngine::new(HashMap::new(), None);
    /// assert_eq!(engine.execution_mode(), ExecutionMode::Executing);
    /// ```
    pub fn new(operations: HashMap<String, Operation>, parent_id: Option<String>) -> Self {
        let completed_ids: HashSet<String> = operations
            .iter()
            .filter(|(_, op)| {
                is_completed_status(&op.status)
                    && op.r#type != aws_sdk_lambda::types::OperationType::Execution
            })
            .map(|(id, _)| id.clone())
            .collect();

        let mode = if completed_ids.is_empty() {
            ExecutionMode::Executing
        } else {
            ExecutionMode::Replaying
        };

        Self {
            operations,
            visited: HashSet::new(),
            completed_ids,
            mode,
            id_generator: OperationIdGenerator::new(parent_id),
        }
    }

    /// Look up an operation by ID, returning it if it exists with a completed status.
    ///
    /// Returns `None` if the operation doesn't exist or is not in a completed state.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::replay::ReplayEngine;
    /// use std::collections::HashMap;
    ///
    /// let engine = ReplayEngine::new(HashMap::new(), None);
    /// assert!(engine.check_result("nonexistent").is_none());
    /// ```
    pub fn check_result(&self, operation_id: &str) -> Option<&Operation> {
        self.operations
            .get(operation_id)
            .filter(|op| is_completed_status(&op.status))
    }

    /// Mark an operation as visited and update replay status.
    ///
    /// After visiting, checks whether all completed operations have been visited.
    /// If so, transitions the mode from [`ExecutionMode::Replaying`] to
    /// [`ExecutionMode::Executing`].
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::replay::ReplayEngine;
    /// use std::collections::HashMap;
    ///
    /// let mut engine = ReplayEngine::new(HashMap::new(), None);
    /// engine.track_replay("some-op-id");
    /// ```
    pub fn track_replay(&mut self, operation_id: &str) {
        self.visited.insert(operation_id.to_string());

        if self.mode == ExecutionMode::Replaying && self.completed_ids.is_subset(&self.visited) {
            self.mode = ExecutionMode::Executing;
        }
    }

    /// Return whether the engine is currently in replay mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::replay::ReplayEngine;
    /// use std::collections::HashMap;
    ///
    /// let engine = ReplayEngine::new(HashMap::new(), None);
    /// assert!(!engine.is_replaying());
    /// ```
    pub fn is_replaying(&self) -> bool {
        self.mode == ExecutionMode::Replaying
    }

    /// Return the current execution mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::replay::ReplayEngine;
    /// use durable_lambda_core::types::ExecutionMode;
    /// use std::collections::HashMap;
    ///
    /// let engine = ReplayEngine::new(HashMap::new(), None);
    /// assert_eq!(engine.execution_mode(), ExecutionMode::Executing);
    /// ```
    pub fn execution_mode(&self) -> ExecutionMode {
        self.mode.clone()
    }

    /// Generate the next deterministic operation ID.
    ///
    /// Delegates to the internal [`OperationIdGenerator`].
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::replay::ReplayEngine;
    /// use std::collections::HashMap;
    ///
    /// let mut engine = ReplayEngine::new(HashMap::new(), None);
    /// let id = engine.generate_operation_id();
    /// assert_eq!(id.len(), 64);
    /// ```
    pub fn generate_operation_id(&mut self) -> String {
        self.id_generator.next_id()
    }

    /// Look up an operation by ID, returning it regardless of status.
    ///
    /// Unlike [`check_result`](Self::check_result) which only returns
    /// operations in a completed status, this returns the operation in
    /// any status (Started, Pending, Succeeded, etc.). Used by callback
    /// operations that need to extract the server-generated `callback_id`
    /// from operations that may still be in a non-completed state.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::replay::ReplayEngine;
    /// use std::collections::HashMap;
    ///
    /// let engine = ReplayEngine::new(HashMap::new(), None);
    /// assert!(engine.get_operation("nonexistent").is_none());
    /// ```
    pub fn get_operation(&self, operation_id: &str) -> Option<&Operation> {
        self.operations.get(operation_id)
    }

    /// Return a reference to the operations map.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::replay::ReplayEngine;
    /// use std::collections::HashMap;
    ///
    /// let engine = ReplayEngine::new(HashMap::new(), None);
    /// assert!(engine.operations().is_empty());
    /// ```
    pub fn operations(&self) -> &HashMap<String, Operation> {
        &self.operations
    }

    /// Insert or update an operation in the state.
    ///
    /// If the operation has a completed status (and is not the root `Execution`
    /// type), it is added to the completed set for replay tracking.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::replay::ReplayEngine;
    /// use aws_sdk_lambda::types::{Operation, OperationType, OperationStatus};
    /// use std::collections::HashMap;
    ///
    /// let mut engine = ReplayEngine::new(HashMap::new(), None);
    /// assert!(engine.operations().is_empty());
    ///
    /// let op = Operation::builder()
    ///     .id("op-1")
    ///     .r#type(OperationType::Step)
    ///     .status(OperationStatus::Succeeded)
    ///     .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
    ///     .build()
    ///     .unwrap();
    /// engine.insert_operation("op-1".to_string(), op);
    /// assert_eq!(engine.operations().len(), 1);
    /// ```
    pub fn insert_operation(&mut self, id: String, operation: Operation) {
        if is_completed_status(&operation.status)
            && operation.r#type != aws_sdk_lambda::types::OperationType::Execution
        {
            self.completed_ids.insert(id.clone());
        }
        self.operations.insert(id, operation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_lambda::types::{Operation, OperationStatus, OperationType};
    fn make_operation(id: &str, status: OperationStatus, op_type: OperationType) -> Operation {
        Operation::builder()
            .id(id)
            .r#type(op_type)
            .status(status)
            .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
            .build()
            .unwrap()
    }

    #[test]
    fn empty_history_starts_executing() {
        let engine = ReplayEngine::new(HashMap::new(), None);
        assert_eq!(engine.execution_mode(), ExecutionMode::Executing);
        assert!(!engine.is_replaying());
    }

    #[test]
    fn completed_operations_start_replaying() {
        let mut ops = HashMap::new();
        ops.insert(
            "op1".to_string(),
            make_operation("op1", OperationStatus::Succeeded, OperationType::Step),
        );

        let engine = ReplayEngine::new(ops, None);
        assert_eq!(engine.execution_mode(), ExecutionMode::Replaying);
        assert!(engine.is_replaying());
    }

    #[test]
    fn only_pending_operations_start_executing() {
        let mut ops = HashMap::new();
        ops.insert(
            "op1".to_string(),
            make_operation("op1", OperationStatus::Pending, OperationType::Step),
        );

        let engine = ReplayEngine::new(ops, None);
        assert_eq!(engine.execution_mode(), ExecutionMode::Executing);
    }

    #[test]
    fn execution_type_excluded_from_replay_tracking() {
        let mut ops = HashMap::new();
        // Only an EXECUTION-type completed op — should NOT count for replay.
        ops.insert(
            "exec".to_string(),
            make_operation("exec", OperationStatus::Succeeded, OperationType::Execution),
        );

        let engine = ReplayEngine::new(ops, None);
        assert_eq!(engine.execution_mode(), ExecutionMode::Executing);
    }

    #[test]
    fn transitions_to_executing_after_all_visited() {
        let mut ops = HashMap::new();
        ops.insert(
            "op1".to_string(),
            make_operation("op1", OperationStatus::Succeeded, OperationType::Step),
        );
        ops.insert(
            "op2".to_string(),
            make_operation("op2", OperationStatus::Failed, OperationType::Step),
        );

        let mut engine = ReplayEngine::new(ops, None);
        assert!(engine.is_replaying());

        engine.track_replay("op1");
        assert!(engine.is_replaying()); // Still replaying — op2 not visited.

        engine.track_replay("op2");
        assert!(!engine.is_replaying()); // All completed ops visited → Executing.
        assert_eq!(engine.execution_mode(), ExecutionMode::Executing);
    }

    #[test]
    fn check_result_returns_completed_operations() {
        let mut ops = HashMap::new();
        ops.insert(
            "op1".to_string(),
            make_operation("op1", OperationStatus::Succeeded, OperationType::Step),
        );
        ops.insert(
            "op2".to_string(),
            make_operation("op2", OperationStatus::Pending, OperationType::Step),
        );

        let engine = ReplayEngine::new(ops, None);
        assert!(engine.check_result("op1").is_some());
        assert!(engine.check_result("op2").is_none()); // Pending, not completed.
        assert!(engine.check_result("op3").is_none()); // Doesn't exist.
    }

    #[test]
    fn generate_operation_id_is_deterministic() {
        let mut engine1 = ReplayEngine::new(HashMap::new(), None);
        let mut engine2 = ReplayEngine::new(HashMap::new(), None);

        let id1a = engine1.generate_operation_id();
        let id1b = engine2.generate_operation_id();
        assert_eq!(id1a, id1b);

        let id2a = engine1.generate_operation_id();
        let id2b = engine2.generate_operation_id();
        assert_eq!(id2a, id2b);
        assert_ne!(id1a, id2a);
    }

    #[test]
    fn mixed_statuses_only_track_completed() {
        let mut ops = HashMap::new();
        ops.insert(
            "done".to_string(),
            make_operation("done", OperationStatus::Succeeded, OperationType::Step),
        );
        ops.insert(
            "pending".to_string(),
            make_operation("pending", OperationStatus::Pending, OperationType::Wait),
        );
        ops.insert(
            "started".to_string(),
            make_operation("started", OperationStatus::Started, OperationType::Step),
        );

        let mut engine = ReplayEngine::new(ops, None);
        assert!(engine.is_replaying());

        // Only need to visit the one completed op to transition.
        engine.track_replay("done");
        assert!(!engine.is_replaying());
    }

    #[test]
    fn all_completed_statuses_are_tracked() {
        for status in [
            OperationStatus::Succeeded,
            OperationStatus::Failed,
            OperationStatus::Cancelled,
            OperationStatus::TimedOut,
            OperationStatus::Stopped,
        ] {
            let mut ops = HashMap::new();
            ops.insert(
                "op".to_string(),
                make_operation("op", status, OperationType::Step),
            );
            let engine = ReplayEngine::new(ops, None);
            assert!(engine.is_replaying(), "Should replay for completed status");
        }
    }

    #[test]
    fn insert_operation_updates_state() {
        let mut engine = ReplayEngine::new(HashMap::new(), None);
        assert!(!engine.is_replaying());

        let op = make_operation("new_op", OperationStatus::Succeeded, OperationType::Step);
        engine.insert_operation("new_op".to_string(), op);

        assert!(engine.check_result("new_op").is_some());
    }
}
