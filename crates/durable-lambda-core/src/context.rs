//! DurableContext — the main context struct passed to handler functions.
//!
//! Own the replay state machine, backend connection, and execution metadata.
//! Provide methods for all durable operations to interact with the replay engine.

use std::collections::HashMap;
use std::sync::Arc;

use aws_sdk_lambda::types::Operation;

use crate::backend::DurableBackend;
use crate::error::DurableError;
use crate::replay::ReplayEngine;
use crate::types::ExecutionMode;

/// Main context for a durable execution invocation.
///
/// `DurableContext` is created at the start of each Lambda invocation. It loads
/// the complete operation state from AWS (paginating if necessary), initializes
/// the replay engine, and provides the interface for durable operations.
///
/// # Construction
///
/// Use [`DurableContext::new`] to create a context from the invocation payload.
/// The constructor paginates through all remaining operations automatically.
///
/// # Examples
///
/// ```no_run
/// use durable_lambda_core::context::DurableContext;
/// use durable_lambda_core::backend::RealBackend;
/// use durable_lambda_core::types::ExecutionMode;
/// use std::sync::Arc;
/// use std::collections::HashMap;
///
/// # async fn example() -> Result<(), durable_lambda_core::error::DurableError> {
/// let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
/// let client = aws_sdk_lambda::Client::new(&config);
/// let backend = Arc::new(RealBackend::new(client));
///
/// let ctx = DurableContext::new(
///     backend,
///     "arn:aws:lambda:us-east-1:123456789:durable-execution/my-exec".to_string(),
///     "initial-token".to_string(),
///     vec![],       // initial operations from invocation payload
///     None,         // no more pages
/// ).await?;
///
/// match ctx.execution_mode() {
///     ExecutionMode::Replaying => println!("Replaying from history"),
///     ExecutionMode::Executing => println!("Executing new operations"),
/// }
/// # Ok(())
/// # }
/// ```
pub struct DurableContext {
    backend: Arc<dyn DurableBackend>,
    replay_engine: ReplayEngine,
    durable_execution_arn: String,
    checkpoint_token: String,
    parent_op_id: Option<String>,
}

/// Maximum items per page when paginating execution state.
const PAGE_SIZE: i32 = 1000;

impl DurableContext {
    /// Create a new `DurableContext` from invocation parameters.
    ///
    /// Loads the complete operation state by paginating through
    /// `get_execution_state` until all pages are fetched. Initializes the
    /// replay engine with the full operations map.
    ///
    /// # Arguments
    ///
    /// * `backend` — The durable execution backend (real or mock).
    /// * `arn` — The durable execution ARN.
    /// * `checkpoint_token` — The initial checkpoint token from the invocation payload.
    /// * `initial_operations` — First page of operations from the invocation payload.
    /// * `next_marker` — Pagination marker for additional pages (`None` if complete).
    ///
    /// # Errors
    ///
    /// Returns [`DurableError`] if paginating the execution state fails.
    pub async fn new(
        backend: Arc<dyn DurableBackend>,
        arn: String,
        checkpoint_token: String,
        initial_operations: Vec<Operation>,
        next_marker: Option<String>,
    ) -> Result<Self, DurableError> {
        let mut operations: HashMap<String, Operation> = initial_operations
            .into_iter()
            .map(|op| (op.id().to_string(), op))
            .collect();

        // Paginate remaining operations.
        let mut marker = next_marker;
        while let Some(ref m) = marker {
            if m.is_empty() {
                break;
            }
            let response = backend
                .get_execution_state(&arn, &checkpoint_token, m, PAGE_SIZE)
                .await?;

            for op in response.operations() {
                operations.insert(op.id().to_string(), op.clone());
            }

            marker = response.next_marker().map(|s| s.to_string());
        }

        let replay_engine = ReplayEngine::new(operations, None);

        Ok(Self {
            backend,
            replay_engine,
            durable_execution_arn: arn,
            checkpoint_token,
            parent_op_id: None,
        })
    }

    /// Return the current execution mode (Replaying or Executing).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: durable_lambda_core::context::DurableContext) {
    /// use durable_lambda_core::types::ExecutionMode;
    /// match ctx.execution_mode() {
    ///     ExecutionMode::Replaying => { /* returning cached results */ }
    ///     ExecutionMode::Executing => { /* running new operations */ }
    /// }
    /// # }
    /// ```
    pub fn execution_mode(&self) -> ExecutionMode {
        self.replay_engine.execution_mode()
    }

    /// Return whether the context is currently replaying from history.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: durable_lambda_core::context::DurableContext) {
    /// if ctx.is_replaying() {
    ///     println!("Replaying cached operations");
    /// }
    /// # }
    /// ```
    pub fn is_replaying(&self) -> bool {
        self.replay_engine.is_replaying()
    }

    /// Return a reference to the durable execution ARN.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: durable_lambda_core::context::DurableContext) {
    /// println!("Execution ARN: {}", ctx.arn());
    /// # }
    /// ```
    pub fn arn(&self) -> &str {
        &self.durable_execution_arn
    }

    /// Return the current checkpoint token.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: durable_lambda_core::context::DurableContext) {
    /// let token = ctx.checkpoint_token();
    /// # }
    /// ```
    pub fn checkpoint_token(&self) -> &str {
        &self.checkpoint_token
    }

    /// Update the checkpoint token (called after a successful checkpoint).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) {
    /// ctx.set_checkpoint_token("new-token-from-aws".to_string());
    /// # }
    /// ```
    pub fn set_checkpoint_token(&mut self, token: String) {
        self.checkpoint_token = token;
    }

    /// Return a reference to the backend.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: durable_lambda_core::context::DurableContext) {
    /// let _backend = ctx.backend();
    /// # }
    /// ```
    pub fn backend(&self) -> &Arc<dyn DurableBackend> {
        &self.backend
    }

    /// Return a mutable reference to the replay engine.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(mut ctx: durable_lambda_core::context::DurableContext) {
    /// let engine = ctx.replay_engine_mut();
    /// # }
    /// ```
    pub fn replay_engine_mut(&mut self) -> &mut ReplayEngine {
        &mut self.replay_engine
    }

    /// Create a child context for isolated operation ID namespacing.
    ///
    /// The child context shares the same backend and ARN but gets its own
    /// `ReplayEngine` with a parent-scoped `OperationIdGenerator`. Operations
    /// within the child context produce deterministic IDs scoped under
    /// `parent_op_id`, preventing collisions with the parent or sibling contexts.
    ///
    /// Used internally by parallel and child_context operations.
    ///
    /// # Arguments
    ///
    /// * `parent_op_id` — The operation ID that scopes this child context
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_core::context::DurableContext) {
    /// let child = ctx.create_child_context("branch-op-id");
    /// // child operations will have IDs scoped under "branch-op-id"
    /// # }
    /// ```
    pub fn create_child_context(&self, parent_op_id: &str) -> DurableContext {
        let operations = self.replay_engine.operations().clone();
        let replay_engine = ReplayEngine::new(operations, Some(parent_op_id.to_string()));

        DurableContext {
            backend: self.backend.clone(),
            replay_engine,
            durable_execution_arn: self.durable_execution_arn.clone(),
            checkpoint_token: self.checkpoint_token.clone(),
            parent_op_id: Some(parent_op_id.to_string()),
        }
    }

    /// Return a reference to the replay engine.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: durable_lambda_core::context::DurableContext) {
    /// let engine = ctx.replay_engine();
    /// assert!(!engine.operations().is_empty() || true);
    /// # }
    /// ```
    pub fn replay_engine(&self) -> &ReplayEngine {
        &self.replay_engine
    }

    /// Return the parent operation ID, if this is a child context.
    ///
    /// Returns `None` for the root context. Returns the parent's operation ID
    /// for child contexts created via [`create_child_context`](Self::create_child_context).
    /// Used by replay-safe logging for hierarchical tracing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_core::context::DurableContext) {
    /// if let Some(parent_id) = ctx.parent_op_id() {
    ///     println!("Child context under parent: {parent_id}");
    /// }
    /// # }
    /// ```
    pub fn parent_op_id(&self) -> Option<&str> {
        self.parent_op_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
    use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
    use aws_sdk_lambda::types::{OperationStatus, OperationType, OperationUpdate};
    /// A simple mock backend for testing context construction.
    struct TestBackend {
        pages: Vec<(Vec<Operation>, Option<String>)>,
    }

    #[async_trait::async_trait]
    impl DurableBackend for TestBackend {
        async fn checkpoint(
            &self,
            _arn: &str,
            _checkpoint_token: &str,
            _updates: Vec<OperationUpdate>,
            _client_token: Option<&str>,
        ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
            unimplemented!("not needed for context tests")
        }

        async fn get_execution_state(
            &self,
            _arn: &str,
            _checkpoint_token: &str,
            next_marker: &str,
            _max_items: i32,
        ) -> Result<GetDurableExecutionStateOutput, DurableError> {
            let page_idx: usize = next_marker.parse().unwrap_or(0);
            if page_idx >= self.pages.len() {
                return Ok(GetDurableExecutionStateOutput::builder().build().unwrap());
            }
            let (ops, marker) = &self.pages[page_idx];
            let mut builder = GetDurableExecutionStateOutput::builder();
            for op in ops {
                builder = builder.operations(op.clone());
            }
            if let Some(m) = marker {
                builder = builder.next_marker(m);
            }
            Ok(builder.build().unwrap())
        }
    }

    fn make_op(id: &str, status: OperationStatus) -> Operation {
        Operation::builder()
            .id(id)
            .r#type(OperationType::Step)
            .status(status)
            .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn empty_history_creates_executing_context() {
        let backend = Arc::new(TestBackend { pages: vec![] });
        let ctx = DurableContext::new(backend, "arn:test".into(), "tok".into(), vec![], None)
            .await
            .unwrap();

        assert_eq!(ctx.execution_mode(), ExecutionMode::Executing);
        assert!(!ctx.is_replaying());
        assert_eq!(ctx.arn(), "arn:test");
        assert_eq!(ctx.checkpoint_token(), "tok");
    }

    #[tokio::test]
    async fn initial_operations_loaded() {
        let backend = Arc::new(TestBackend { pages: vec![] });
        let ops = vec![make_op("op1", OperationStatus::Succeeded)];
        let ctx = DurableContext::new(backend, "arn:test".into(), "tok".into(), ops, None)
            .await
            .unwrap();

        assert!(ctx.is_replaying());
        assert!(ctx.replay_engine().check_result("op1").is_some());
    }

    #[tokio::test]
    async fn pagination_loads_all_pages() {
        let backend = Arc::new(TestBackend {
            pages: vec![
                (
                    vec![make_op("op2", OperationStatus::Succeeded)],
                    Some("1".to_string()),
                ),
                (vec![make_op("op3", OperationStatus::Succeeded)], None),
            ],
        });

        let initial = vec![make_op("op1", OperationStatus::Succeeded)];
        let ctx = DurableContext::new(
            backend,
            "arn:test".into(),
            "tok".into(),
            initial,
            Some("0".to_string()),
        )
        .await
        .unwrap();

        assert!(ctx.replay_engine().check_result("op1").is_some());
        assert!(ctx.replay_engine().check_result("op2").is_some());
        assert!(ctx.replay_engine().check_result("op3").is_some());
    }

    #[tokio::test]
    async fn set_checkpoint_token_updates() {
        let backend = Arc::new(TestBackend { pages: vec![] });
        let mut ctx = DurableContext::new(backend, "arn:test".into(), "tok1".into(), vec![], None)
            .await
            .unwrap();

        assert_eq!(ctx.checkpoint_token(), "tok1");
        ctx.set_checkpoint_token("tok2".to_string());
        assert_eq!(ctx.checkpoint_token(), "tok2");
    }
}
