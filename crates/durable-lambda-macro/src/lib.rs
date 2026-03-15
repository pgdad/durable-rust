//! Proc-macro approach for durable Lambda handlers.
//!
//! Provides the `#[durable_execution]` attribute macro (FR32).
//!
//! # Usage
//!
//! Annotate an async handler function with `#[durable_execution]` to generate
//! all Lambda runtime boilerplate. The macro creates a `main()` function that
//! wires up AWS config, Lambda client, `RealBackend`, and `lambda_runtime`.
//!
//! ```ignore
//! use durable_lambda_macro::durable_execution;
//! use durable_lambda_core::context::DurableContext;
//! use durable_lambda_core::error::DurableError;
//!
//! #[durable_execution]
//! async fn handler(event: serde_json::Value, mut ctx: DurableContext) -> Result<serde_json::Value, DurableError> {
//!     Ok(event)
//! }
//! ```

mod expand;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Attribute macro that transforms an async handler into a complete durable Lambda binary.
///
/// The annotated function must:
/// - Be `async`
/// - Have exactly 2 parameters: `(event: serde_json::Value, ctx: DurableContext)`
/// - Return `Result<serde_json::Value, DurableError>`
///
/// The macro preserves the original function and generates a `#[tokio::main] async fn main()`
/// that sets up the Lambda runtime, AWS backend, and event parsing — mirroring the
/// `#[tokio::main]` ergonomic pattern.
#[proc_macro_attribute]
pub fn durable_execution(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as syn::ItemFn);

    match expand::expand_durable_execution(func) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
