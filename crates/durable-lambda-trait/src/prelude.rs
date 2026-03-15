//! User-facing re-exports for the trait-based approach.
//!
//! Single import for everything needed to write a durable Lambda handler:
//!
//! ```no_run
//! use durable_lambda_trait::prelude::*;
//! ```
//!
//! # Complete Minimal Handler
//!
//! ```no_run
//! use durable_lambda_trait::prelude::*;
//!
//! struct MyHandler;
//!
//! #[async_trait::async_trait]
//! impl DurableHandler for MyHandler {
//!     async fn handle(
//!         &self,
//!         event: serde_json::Value,
//!         mut ctx: TraitContext,
//!     ) -> Result<serde_json::Value, DurableError> {
//!         let result: Result<String, String> = ctx.step("greet", || async {
//!             Ok("Hello from durable Lambda!".to_string())
//!         }).await?;
//!         Ok(serde_json::json!({ "greeting": result.unwrap() }))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     run(MyHandler).await
//! }
//! ```
//!
//! This re-exports:
//! - [`DurableHandler`] — the trait for defining durable handlers
//! - [`TraitContext`] — the context wrapper for durable operations
//! - [`run`](crate::run) — the entry point for trait-based handlers
//! - Core types: [`DurableError`], [`StepOptions`], [`CallbackOptions`], [`CallbackHandle`],
//!   [`ExecutionMode`], [`CheckpointResult`]

pub use crate::context::TraitContext;
pub use crate::handler::{run, DurableHandler};
pub use durable_lambda_core::error::DurableError;
pub use durable_lambda_core::types::{
    BatchItem, BatchItemStatus, BatchResult, CallbackHandle, CallbackOptions, CheckpointResult,
    CompletionReason, ExecutionMode, MapOptions, ParallelOptions, StepOptions,
};
