//! Testing utilities for durable Lambda handlers.
//!
//! This crate provides [`MockDurableContext`](mock_context::MockDurableContext) for
//! writing tests without AWS credentials. Pre-load step results and verify
//! handler logic with deterministic, in-memory data.
//!
//! # Quick Start
//!
//! ```no_run
//! use durable_lambda_testing::prelude::*;
//!
//! #[tokio::test]
//! async fn test_my_handler() {
//!     let (mut ctx, calls, _ops) = MockDurableContext::new()
//!         .with_step_result("validate", r#"true"#)
//!         .build()
//!         .await;
//!
//!     let result: Result<bool, String> = ctx.step("validate", || async {
//!         panic!("not executed during replay");
//!     }).await.unwrap();
//!
//!     assert!(result.unwrap());
//!     assert_no_checkpoints(&calls).await;
//! }
//! ```

pub mod assertions;
pub mod mock_backend;
pub mod mock_context;
pub mod prelude;

pub use mock_backend::{
    CheckpointCall, CheckpointRecorder, MockBackend, OperationRecord, OperationRecorder,
};
pub use mock_context::MockDurableContext;
