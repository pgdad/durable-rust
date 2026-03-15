//! User-facing re-exports for the closure-native approach.
//!
//! Single import for everything needed to write a durable Lambda handler:
//!
//! ```no_run
//! use durable_lambda_closure::prelude::*;
//! ```
//!
//! # Complete Minimal Handler
//!
//! ```no_run
//! use durable_lambda_closure::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     run(|event: serde_json::Value, mut ctx: ClosureContext| async move {
//!         let result: Result<String, String> = ctx.step("greet", || async {
//!             Ok("Hello from durable Lambda!".to_string())
//!         }).await?;
//!         Ok(serde_json::json!({ "greeting": result.unwrap() }))
//!     }).await
//! }
//! ```
//!
//! This re-exports:
//! - [`ClosureContext`] — the context wrapper for durable operations
//! - [`run`](crate::run) — the entry point for closure-native handlers
//! - Core types: [`DurableError`], [`StepOptions`], [`CallbackOptions`], [`CallbackHandle`],
//!   [`ExecutionMode`], [`CheckpointResult`]

pub use crate::context::ClosureContext;
pub use crate::handler::run;
pub use durable_lambda_core::error::DurableError;
pub use durable_lambda_core::types::{
    BatchItem, BatchItemStatus, BatchResult, CallbackHandle, CallbackOptions, CheckpointResult,
    CompletionReason, ExecutionMode, MapOptions, ParallelOptions, StepOptions,
};
