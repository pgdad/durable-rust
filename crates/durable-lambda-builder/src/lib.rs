//! Builder-pattern API approach for durable Lambda handlers.
//!
//! This crate provides a builder-pattern API for writing durable Lambda functions
//! with step-by-step configuration. Use [`handler`] to create a
//! [`DurableHandlerBuilder`], then call [`.run()`](DurableHandlerBuilder::run)
//! to start the Lambda runtime.
//!
//! # Quick Start
//!
//! ```no_run
//! use durable_lambda_builder::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), lambda_runtime::Error> {
//!     durable_lambda_builder::handler(|event: serde_json::Value, mut ctx: BuilderContext| async move {
//!         let order = ctx.step("validate_order", || async {
//!             Ok::<_, String>(serde_json::json!({"id": 123, "valid": true}))
//!         }).await?;
//!         Ok(serde_json::json!({"order": order}))
//!     })
//!     .run()
//!     .await
//! }
//! ```

pub mod context;
pub mod handler;
pub mod prelude;

pub use context::BuilderContext;
pub use handler::{handler, DurableHandlerBuilder};
