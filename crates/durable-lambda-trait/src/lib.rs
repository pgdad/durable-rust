//! Trait-based API approach for durable Lambda handlers.
//!
//! This crate provides a trait-based API for writing durable Lambda functions
//! with a structured, object-oriented approach. Implement [`DurableHandler`] on
//! your struct, then use [`run`] as the single entry point — it handles all
//! `lambda_runtime` and `DurableContext` wiring internally.
//!
//! # Quick Start
//!
//! ```no_run
//! use durable_lambda_trait::prelude::*;
//! use async_trait::async_trait;
//!
//! struct OrderProcessor;
//!
//! #[async_trait]
//! impl DurableHandler for OrderProcessor {
//!     async fn handle(
//!         &self,
//!         event: serde_json::Value,
//!         mut ctx: TraitContext,
//!     ) -> Result<serde_json::Value, DurableError> {
//!         let order = ctx.step("validate_order", || async {
//!             Ok::<_, String>(serde_json::json!({"id": 123, "valid": true}))
//!         }).await?;
//!         Ok(serde_json::json!({"order": order}))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), lambda_runtime::Error> {
//!     durable_lambda_trait::run(OrderProcessor).await
//! }
//! ```

pub mod context;
pub mod handler;
pub mod prelude;

pub use context::TraitContext;
pub use handler::{run, DurableHandler};
