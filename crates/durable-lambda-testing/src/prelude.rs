//! User-facing test re-exports for durable Lambda testing.
//!
//! Single import for everything needed to write durable Lambda tests:
//!
//! ```no_run
//! use durable_lambda_testing::prelude::*;
//! ```
//!
//! This re-exports:
//! - [`MockDurableContext`] — builder for creating mock contexts with pre-loaded results
//! - [`MockBackend`] and [`CheckpointCall`] — mock backend and checkpoint recording
//! - Assertion helpers: [`assert_checkpoint_count`], [`assert_no_checkpoints`]
//! - Core types: [`DurableContext`], [`DurableError`], [`StepOptions`], [`ExecutionMode`]

pub use crate::assertions::{
    assert_checkpoint_count, assert_no_checkpoints, assert_operation_count, assert_operation_names,
    assert_operations,
};
pub use crate::mock_backend::{
    CheckpointCall, CheckpointRecorder, MockBackend, OperationRecord, OperationRecorder,
};
pub use crate::mock_context::MockDurableContext;
pub use durable_lambda_core::context::DurableContext;
pub use durable_lambda_core::error::DurableError;
pub use durable_lambda_core::types::{ExecutionMode, StepOptions};
