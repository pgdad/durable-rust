//! Log operation — replay-safe structured logging.
//!
//! Implement FR29-FR31: deduplicated log messages, tracing integration,
//! suppress duplicate output during replay phase.
//!
//! Unlike all other operations in this module, logging is NOT a checkpoint-based
//! durable operation. It does not send checkpoints to AWS, does not generate
//! operation IDs, and does not interact with the replay engine's operations map.
//!
//! Replay suppression is purely client-side: if the context is replaying,
//! log calls are no-ops. This matches the Python SDK's `context.logger` behavior.

use crate::context::DurableContext;

impl DurableContext {
    /// Return the parent operation ID for log enrichment, or empty string for root context.
    fn log_parent_id(&self) -> &str {
        self.parent_op_id().unwrap_or("")
    }

    /// Emit a replay-safe info-level log message.
    ///
    /// During execution mode, emits the message via `tracing::info!` with
    /// execution context enrichment (execution ARN, parent ID for child
    /// contexts). During replay mode, the call is a no-op — no log output
    /// is produced.
    ///
    /// # Arguments
    ///
    /// * `message` — The log message to emit
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_core::context::DurableContext) {
    /// ctx.log("Order processing started");
    /// // During replay, this produces no output.
    /// # }
    /// ```
    pub fn log(&self, message: &str) {
        if !self.is_replaying() {
            tracing::info!(
                execution_arn = %self.arn(),
                parent_id = %self.log_parent_id(),
                message = message,
                "durable_log"
            );
        }
    }

    /// Emit a replay-safe info-level log message with structured data.
    ///
    /// During execution mode, emits the message and structured data via
    /// `tracing::info!`. During replay mode, the call is a no-op.
    ///
    /// # Arguments
    ///
    /// * `message` — The log message to emit
    /// * `data` — Structured data to include in the log event
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_core::context::DurableContext) {
    /// ctx.log_with_data("Order processed", &serde_json::json!({"order_id": 42}));
    /// # }
    /// ```
    pub fn log_with_data(&self, message: &str, data: &serde_json::Value) {
        if !self.is_replaying() {
            tracing::info!(
                execution_arn = %self.arn(),
                parent_id = %self.log_parent_id(),
                data = %data,
                message = message,
                "durable_log"
            );
        }
    }

    /// Emit a replay-safe debug-level log message.
    ///
    /// During execution mode, emits via `tracing::debug!`. During replay
    /// mode, the call is a no-op.
    ///
    /// # Arguments
    ///
    /// * `message` — The log message to emit
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_core::context::DurableContext) {
    /// ctx.log_debug("Validating order fields");
    /// # }
    /// ```
    pub fn log_debug(&self, message: &str) {
        if !self.is_replaying() {
            tracing::debug!(
                execution_arn = %self.arn(),
                parent_id = %self.log_parent_id(),
                message = message,
                "durable_log"
            );
        }
    }

    /// Emit a replay-safe warn-level log message.
    ///
    /// During execution mode, emits via `tracing::warn!`. During replay
    /// mode, the call is a no-op.
    ///
    /// # Arguments
    ///
    /// * `message` — The log message to emit
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_core::context::DurableContext) {
    /// ctx.log_warn("Inventory below threshold");
    /// # }
    /// ```
    pub fn log_warn(&self, message: &str) {
        if !self.is_replaying() {
            tracing::warn!(
                execution_arn = %self.arn(),
                parent_id = %self.log_parent_id(),
                message = message,
                "durable_log"
            );
        }
    }

    /// Emit a replay-safe error-level log message.
    ///
    /// During execution mode, emits via `tracing::error!`. During replay
    /// mode, the call is a no-op.
    ///
    /// # Arguments
    ///
    /// * `message` — The log message to emit
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_core::context::DurableContext) {
    /// ctx.log_error("Payment gateway timeout");
    /// # }
    /// ```
    pub fn log_error(&self, message: &str) {
        if !self.is_replaying() {
            tracing::error!(
                execution_arn = %self.arn(),
                parent_id = %self.log_parent_id(),
                message = message,
                "durable_log"
            );
        }
    }

    /// Emit a replay-safe debug-level log message with structured data.
    ///
    /// During execution mode, emits via `tracing::debug!` with data field.
    /// During replay mode, the call is a no-op.
    ///
    /// # Arguments
    ///
    /// * `message` — The log message to emit
    /// * `data` — Structured data to include in the log event
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_core::context::DurableContext) {
    /// ctx.log_debug_with_data("Request details", &serde_json::json!({"method": "POST"}));
    /// # }
    /// ```
    pub fn log_debug_with_data(&self, message: &str, data: &serde_json::Value) {
        if !self.is_replaying() {
            tracing::debug!(
                execution_arn = %self.arn(),
                parent_id = %self.log_parent_id(),
                data = %data,
                message = message,
                "durable_log"
            );
        }
    }

    /// Emit a replay-safe warn-level log message with structured data.
    ///
    /// During execution mode, emits via `tracing::warn!` with data field.
    /// During replay mode, the call is a no-op.
    ///
    /// # Arguments
    ///
    /// * `message` — The log message to emit
    /// * `data` — Structured data to include in the log event
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_core::context::DurableContext) {
    /// ctx.log_warn_with_data("Retry attempt", &serde_json::json!({"attempt": 3}));
    /// # }
    /// ```
    pub fn log_warn_with_data(&self, message: &str, data: &serde_json::Value) {
        if !self.is_replaying() {
            tracing::warn!(
                execution_arn = %self.arn(),
                parent_id = %self.log_parent_id(),
                data = %data,
                message = message,
                "durable_log"
            );
        }
    }

    /// Emit a replay-safe error-level log message with structured data.
    ///
    /// During execution mode, emits via `tracing::error!` with data field.
    /// During replay mode, the call is a no-op.
    ///
    /// # Arguments
    ///
    /// * `message` — The log message to emit
    /// * `data` — Structured data to include in the log event
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_core::context::DurableContext) {
    /// ctx.log_error_with_data("Payment failed", &serde_json::json!({"error": "timeout"}));
    /// # }
    /// ```
    pub fn log_error_with_data(&self, message: &str, data: &serde_json::Value) {
        if !self.is_replaying() {
            tracing::error!(
                execution_arn = %self.arn(),
                parent_id = %self.log_parent_id(),
                data = %data,
                message = message,
                "durable_log"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
    use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
    use aws_sdk_lambda::types::{Operation, OperationStatus, OperationType, OperationUpdate};
    use tracing_test::traced_test;

    use crate::backend::DurableBackend;
    use crate::context::DurableContext;
    use crate::error::DurableError;

    /// Minimal mock backend for log tests — logging never calls the backend.
    struct LogTestBackend;

    #[async_trait::async_trait]
    impl DurableBackend for LogTestBackend {
        async fn checkpoint(
            &self,
            _arn: &str,
            _checkpoint_token: &str,
            _updates: Vec<OperationUpdate>,
            _client_token: Option<&str>,
        ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
            unimplemented!("logging does not checkpoint")
        }

        async fn get_execution_state(
            &self,
            _arn: &str,
            _checkpoint_token: &str,
            _next_marker: &str,
            _max_items: i32,
        ) -> Result<GetDurableExecutionStateOutput, DurableError> {
            Ok(GetDurableExecutionStateOutput::builder().build().unwrap())
        }
    }

    async fn make_executing_context() -> DurableContext {
        DurableContext::new(
            Arc::new(LogTestBackend),
            "arn:aws:lambda:us-east-1:123456789:durable-execution/test-exec".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap()
    }

    async fn make_replaying_context() -> DurableContext {
        let op = Operation::builder()
            .id("op-1")
            .r#type(OperationType::Step)
            .status(OperationStatus::Succeeded)
            .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
            .build()
            .unwrap();

        DurableContext::new(
            Arc::new(LogTestBackend),
            "arn:aws:lambda:us-east-1:123456789:durable-execution/test-exec".to_string(),
            "tok".to_string(),
            vec![op],
            None,
        )
        .await
        .unwrap()
    }

    #[traced_test]
    #[tokio::test]
    async fn test_log_emits_during_execution() {
        let ctx = make_executing_context().await;
        assert!(!ctx.is_replaying());

        ctx.log("order processing started");
        assert!(logs_contain("order processing started"));
        assert!(logs_contain("execution_arn"));
    }

    #[traced_test]
    #[tokio::test]
    async fn test_log_suppressed_during_replay() {
        let ctx = make_replaying_context().await;
        assert!(ctx.is_replaying());

        ctx.log("should not appear in logs");
        assert!(!logs_contain("should not appear in logs"));
    }

    #[traced_test]
    #[tokio::test]
    async fn test_log_with_structured_data() {
        let ctx = make_executing_context().await;
        let data = serde_json::json!({"order_id": 42, "amount": 99.99});

        ctx.log_with_data("order processed", &data);
        assert!(logs_contain("order processed"));
        assert!(logs_contain("order_id"));
    }

    #[traced_test]
    #[tokio::test]
    async fn test_log_all_levels() {
        let ctx = make_executing_context().await;

        ctx.log_debug("debug message");
        ctx.log("info message");
        ctx.log_warn("warn message");
        ctx.log_error("error message");

        assert!(logs_contain("debug message"));
        assert!(logs_contain("info message"));
        assert!(logs_contain("warn message"));
        assert!(logs_contain("error message"));
    }

    #[traced_test]
    #[tokio::test]
    async fn test_log_all_levels_suppressed_during_replay() {
        let ctx = make_replaying_context().await;

        ctx.log_debug("replay debug");
        ctx.log("replay info");
        ctx.log_warn("replay warn");
        ctx.log_error("replay error");
        ctx.log_with_data("replay data", &serde_json::json!({"key": "val"}));
        ctx.log_debug_with_data("replay debug data", &serde_json::json!({"k": "v"}));
        ctx.log_warn_with_data("replay warn data", &serde_json::json!({"k": "v"}));
        ctx.log_error_with_data("replay error data", &serde_json::json!({"k": "v"}));

        assert!(!logs_contain("replay debug"));
        assert!(!logs_contain("replay info"));
        assert!(!logs_contain("replay warn"));
        assert!(!logs_contain("replay error"));
        assert!(!logs_contain("replay data"));
        assert!(!logs_contain("replay debug data"));
        assert!(!logs_contain("replay warn data"));
        assert!(!logs_contain("replay error data"));
    }

    #[traced_test]
    #[tokio::test]
    async fn test_log_is_not_durable_operation() {
        let ctx = make_executing_context().await;

        // Logging should NOT generate operation IDs or interact with the replay engine.
        let ops_before = ctx.replay_engine().operations().len();

        ctx.log("test message");
        ctx.log_with_data("test data", &serde_json::json!({"k": "v"}));
        ctx.log_debug("test debug");
        ctx.log_warn("test warn");
        ctx.log_error("test error");

        let ops_after = ctx.replay_engine().operations().len();
        assert_eq!(
            ops_before, ops_after,
            "logging must not add operations to replay engine"
        );
    }

    #[traced_test]
    #[tokio::test]
    async fn test_log_with_data_variants() {
        let ctx = make_executing_context().await;

        ctx.log_debug_with_data("debug details", &serde_json::json!({"step": "validate"}));
        ctx.log_warn_with_data("warn details", &serde_json::json!({"retries": 2}));
        ctx.log_error_with_data("error details", &serde_json::json!({"code": 500}));

        assert!(logs_contain("debug details"));
        assert!(logs_contain("warn details"));
        assert!(logs_contain("error details"));
    }

    #[traced_test]
    #[tokio::test]
    async fn test_log_includes_execution_arn() {
        let ctx = make_executing_context().await;

        ctx.log("arn check");
        assert!(logs_contain("durable-execution/test-exec"));
    }

    #[traced_test]
    #[tokio::test]
    async fn test_log_root_context_has_empty_parent_id() {
        let ctx = make_executing_context().await;
        assert!(ctx.parent_op_id().is_none());

        ctx.log("root log");
        assert!(logs_contain("root log"));
        assert!(logs_contain("parent_id"));
    }

    #[traced_test]
    #[tokio::test]
    async fn test_log_child_context_includes_parent_id() {
        let ctx = make_executing_context().await;
        let child = ctx.create_child_context("child-op-123");

        assert_eq!(child.parent_op_id(), Some("child-op-123"));

        child.log("child log message");
        assert!(logs_contain("child log message"));
        assert!(logs_contain("child-op-123"));
    }
}
