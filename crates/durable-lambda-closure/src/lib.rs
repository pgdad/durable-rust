//! Closure-native API approach for durable Lambda handlers.
//!
//! This crate provides a closure-based API for writing durable Lambda functions
//! with minimal boilerplate. Use [`run`] as the single entry point — it handles
//! all `lambda_runtime` and `DurableContext` wiring internally.
//!
//! # Quick Start
//!
//! ```no_run
//! use durable_lambda_closure::prelude::*;
//!
//! async fn handler(
//!     event: serde_json::Value,
//!     mut ctx: ClosureContext,
//! ) -> Result<serde_json::Value, DurableError> {
//!     let order = ctx.step("validate_order", || async {
//!         Ok::<_, String>(serde_json::json!({"id": 123, "valid": true}))
//!     }).await?;
//!     Ok(serde_json::json!({"order": order}))
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), lambda_runtime::Error> {
//!     durable_lambda_closure::run(handler).await
//! }
//! ```

pub mod context;
pub mod handler;
pub mod prelude;

pub use context::ClosureContext;
pub use handler::run;
