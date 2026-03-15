//! User-facing re-exports for the builder-pattern approach.
//!
//! Single import for everything needed to write a durable Lambda handler:
//!
//! ```no_run
//! use durable_lambda_builder::prelude::*;
//! ```
//!
//! # Complete Minimal Handler
//!
//! ```no_run
//! use durable_lambda_builder::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     handler(|event: serde_json::Value, mut ctx: BuilderContext| async move {
//!         let result: Result<String, String> = ctx.step("greet", || async {
//!             Ok("Hello from durable Lambda!".to_string())
//!         }).await?;
//!         Ok(serde_json::json!({ "greeting": result.unwrap() }))
//!     })
//!     .run()
//!     .await
//! }
//! ```
//!
//! This re-exports:
//! - [`BuilderContext`] — the context wrapper for durable operations
//! - [`DurableHandlerBuilder`] — the builder type
//! - [`handler`](crate::handler) — the constructor function
//! - Core types: [`DurableError`], [`StepOptions`], [`CallbackOptions`], [`CallbackHandle`],
//!   [`ExecutionMode`], [`CheckpointResult`]

pub use crate::context::BuilderContext;
pub use crate::handler::{handler, DurableHandlerBuilder};
pub use durable_lambda_core::error::DurableError;
pub use durable_lambda_core::types::{
    BatchItem, BatchItemStatus, BatchResult, CallbackHandle, CallbackOptions, CheckpointResult,
    CompletionReason, ExecutionMode, MapOptions, ParallelOptions, StepOptions,
};
