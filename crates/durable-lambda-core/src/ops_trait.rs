//! Shared trait for all durable context types.
//!
//! [`DurableContextOps`] is the single interface satisfied by every context
//! type in the SDK: [`DurableContext`](crate::context::DurableContext),
//! `ClosureContext`, `TraitContext`, and `BuilderContext`.
//!
//! This trait exists for **static dispatch only** — never use it as `dyn
//! DurableContextOps`. It enables generic handler functions that work with any
//! context flavour without code duplication.

use std::future::Future;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::context::DurableContext;
use crate::error::DurableError;
use crate::types::{
    BatchResult, CallbackHandle, CallbackOptions, CompensationResult, ExecutionMode, MapOptions,
    ParallelOptions, StepOptions,
};

/// Shared interface for all durable context types.
///
/// Every context type in the SDK implements this trait by delegating to the
/// underlying [`DurableContext`] or its inherent methods. Use this trait as a
/// generic bound when writing handler logic that should work across all context
/// approaches (closure, trait, builder, or core directly).
///
/// # Static dispatch only
///
/// This trait uses native async (`-> impl Future<...>`) and is designed for
/// static dispatch. Do **not** use `dyn DurableContextOps` — there is no
/// object-safe version.
///
/// # Examples
///
/// ```no_run
/// use durable_lambda_core::DurableContextOps;
/// use durable_lambda_core::error::DurableError;
///
/// async fn process_order<C: DurableContextOps>(ctx: &mut C, order_id: u64) -> Result<(), DurableError> {
///     let _result: Result<String, String> = ctx.step("validate", move || async move {
///         Ok(format!("validated:{order_id}"))
///     }).await?;
///     ctx.log("order processed");
///     Ok(())
/// }
/// ```
pub trait DurableContextOps {
    // -------------------------------------------------------------------------
    // Async operation methods
    // -------------------------------------------------------------------------

    /// Execute a named step with checkpointing.
    ///
    /// See [`DurableContext::step`](crate::context::DurableContext) for full
    /// documentation.
    fn step<T, E, F, Fut>(
        &mut self,
        name: &str,
        f: F,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static;

    /// Execute a named step with checkpointing and retry configuration.
    ///
    /// See [`DurableContext::step_with_options`](crate::context::DurableContext) for full
    /// documentation.
    fn step_with_options<T, E, F, Fut>(
        &mut self,
        name: &str,
        options: StepOptions,
        f: F,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static;

    /// Suspend execution for the specified duration.
    ///
    /// See [`DurableContext::wait`](crate::context::DurableContext) for full
    /// documentation.
    fn wait(
        &mut self,
        name: &str,
        duration_secs: i32,
    ) -> impl Future<Output = Result<(), DurableError>> + Send;

    /// Register a callback and return a handle with the server-generated callback ID.
    ///
    /// See [`DurableContext::create_callback`](crate::context::DurableContext) for full
    /// documentation.
    fn create_callback(
        &mut self,
        name: &str,
        options: CallbackOptions,
    ) -> impl Future<Output = Result<CallbackHandle, DurableError>> + Send;

    /// Durably invoke another Lambda function and return its result.
    ///
    /// See [`DurableContext::invoke`](crate::context::DurableContext) for full
    /// documentation.
    fn invoke<T, P>(
        &mut self,
        name: &str,
        function_name: &str,
        payload: &P,
    ) -> impl Future<Output = Result<T, DurableError>> + Send
    where
        T: DeserializeOwned + Send,
        P: Serialize + Sync;

    /// Execute multiple branches concurrently and return their results.
    ///
    /// See [`DurableContext::parallel`](crate::context::DurableContext) for full
    /// documentation.
    fn parallel<T, F, Fut>(
        &mut self,
        name: &str,
        branches: Vec<F>,
        options: ParallelOptions,
    ) -> impl Future<Output = Result<BatchResult<T>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce(DurableContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, DurableError>> + Send + 'static;

    /// Execute an isolated subflow with its own checkpoint namespace.
    ///
    /// See [`DurableContext::child_context`](crate::context::DurableContext) for full
    /// documentation.
    fn child_context<T, F, Fut>(
        &mut self,
        name: &str,
        f: F,
    ) -> impl Future<Output = Result<T, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send,
        F: FnOnce(DurableContext) -> Fut + Send,
        Fut: Future<Output = Result<T, DurableError>> + Send;

    /// Process a collection of items in parallel and return their results.
    ///
    /// See [`DurableContext::map`](crate::context::DurableContext) for full
    /// documentation.
    fn map<T, I, F, Fut>(
        &mut self,
        name: &str,
        items: Vec<I>,
        options: MapOptions,
        f: F,
    ) -> impl Future<Output = Result<BatchResult<T>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        I: Send + 'static,
        F: FnOnce(I, DurableContext) -> Fut + Send + 'static + Clone,
        Fut: Future<Output = Result<T, DurableError>> + Send + 'static;

    // -------------------------------------------------------------------------
    // Compensation (saga pattern) methods
    // -------------------------------------------------------------------------

    /// Execute a forward step and register a compensation closure on success.
    ///
    /// See [`DurableContext::step_with_compensation`](crate::context::DurableContext) for full
    /// documentation.
    fn step_with_compensation<T, E, F, Fut, G, GFut>(
        &mut self,
        name: &str,
        forward_fn: F,
        compensate_fn: G,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        G: FnOnce(T) -> GFut + Send + 'static,
        GFut: Future<Output = Result<(), DurableError>> + Send + 'static;

    /// Execute a forward step (with options) and register a compensation closure on success.
    ///
    /// See [`DurableContext::step_with_compensation_opts`](crate::context::DurableContext) for full
    /// documentation.
    fn step_with_compensation_opts<T, E, F, Fut, G, GFut>(
        &mut self,
        name: &str,
        options: StepOptions,
        forward_fn: F,
        compensate_fn: G,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        G: FnOnce(T) -> GFut + Send + 'static,
        GFut: Future<Output = Result<(), DurableError>> + Send + 'static;

    /// Execute all registered compensations in reverse registration order.
    ///
    /// See [`DurableContext::run_compensations`](crate::context::DurableContext) for full
    /// documentation.
    fn run_compensations(
        &mut self,
    ) -> impl Future<Output = Result<CompensationResult, DurableError>> + Send;

    // -------------------------------------------------------------------------
    // Sync operation method
    // -------------------------------------------------------------------------

    /// Check the result of a previously created callback.
    ///
    /// See [`DurableContext::callback_result`](crate::context::DurableContext) for full
    /// documentation.
    fn callback_result<T: DeserializeOwned>(
        &self,
        handle: &CallbackHandle,
    ) -> Result<T, DurableError>;

    // -------------------------------------------------------------------------
    // State query methods
    // -------------------------------------------------------------------------

    /// Return the current execution mode (Replaying or Executing).
    fn execution_mode(&self) -> ExecutionMode;

    /// Return whether the context is currently replaying from history.
    fn is_replaying(&self) -> bool;

    /// Return a reference to the durable execution ARN.
    fn arn(&self) -> &str;

    /// Return the current checkpoint token.
    fn checkpoint_token(&self) -> &str;

    // -------------------------------------------------------------------------
    // Log methods
    // -------------------------------------------------------------------------

    /// Emit a replay-safe info-level log message.
    fn log(&self, message: &str);

    /// Emit a replay-safe info-level log message with structured data.
    fn log_with_data(&self, message: &str, data: &serde_json::Value);

    /// Emit a replay-safe debug-level log message.
    fn log_debug(&self, message: &str);

    /// Emit a replay-safe warn-level log message.
    fn log_warn(&self, message: &str);

    /// Emit a replay-safe error-level log message.
    fn log_error(&self, message: &str);

    /// Emit a replay-safe debug-level log message with structured data.
    fn log_debug_with_data(&self, message: &str, data: &serde_json::Value);

    /// Emit a replay-safe warn-level log message with structured data.
    fn log_warn_with_data(&self, message: &str, data: &serde_json::Value);

    /// Emit a replay-safe error-level log message with structured data.
    fn log_error_with_data(&self, message: &str, data: &serde_json::Value);

    // -------------------------------------------------------------------------
    // Batch checkpoint methods
    // -------------------------------------------------------------------------

    /// Enable batch checkpoint mode.
    ///
    /// See [`DurableContext::enable_batch_mode`](crate::context::DurableContext) for full
    /// documentation.
    fn enable_batch_mode(&mut self);

    /// Flush accumulated batch checkpoint updates.
    ///
    /// See [`DurableContext::flush_batch`](crate::context::DurableContext) for full
    /// documentation.
    fn flush_batch(&mut self) -> impl Future<Output = Result<(), DurableError>> + Send;
}

impl DurableContextOps for DurableContext {
    fn step<T, E, F, Fut>(
        &mut self,
        name: &str,
        f: F,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        DurableContext::step(self, name, f)
    }

    fn step_with_options<T, E, F, Fut>(
        &mut self,
        name: &str,
        options: StepOptions,
        f: F,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        DurableContext::step_with_options(self, name, options, f)
    }

    fn wait(
        &mut self,
        name: &str,
        duration_secs: i32,
    ) -> impl Future<Output = Result<(), DurableError>> + Send {
        DurableContext::wait(self, name, duration_secs)
    }

    fn create_callback(
        &mut self,
        name: &str,
        options: CallbackOptions,
    ) -> impl Future<Output = Result<CallbackHandle, DurableError>> + Send {
        DurableContext::create_callback(self, name, options)
    }

    fn invoke<T, P>(
        &mut self,
        name: &str,
        function_name: &str,
        payload: &P,
    ) -> impl Future<Output = Result<T, DurableError>> + Send
    where
        T: DeserializeOwned + Send,
        P: Serialize + Sync,
    {
        DurableContext::invoke(self, name, function_name, payload)
    }

    fn parallel<T, F, Fut>(
        &mut self,
        name: &str,
        branches: Vec<F>,
        options: ParallelOptions,
    ) -> impl Future<Output = Result<BatchResult<T>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce(DurableContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, DurableError>> + Send + 'static,
    {
        DurableContext::parallel(self, name, branches, options)
    }

    fn child_context<T, F, Fut>(
        &mut self,
        name: &str,
        f: F,
    ) -> impl Future<Output = Result<T, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send,
        F: FnOnce(DurableContext) -> Fut + Send,
        Fut: Future<Output = Result<T, DurableError>> + Send,
    {
        DurableContext::child_context(self, name, f)
    }

    fn map<T, I, F, Fut>(
        &mut self,
        name: &str,
        items: Vec<I>,
        options: MapOptions,
        f: F,
    ) -> impl Future<Output = Result<BatchResult<T>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        I: Send + 'static,
        F: FnOnce(I, DurableContext) -> Fut + Send + 'static + Clone,
        Fut: Future<Output = Result<T, DurableError>> + Send + 'static,
    {
        DurableContext::map(self, name, items, options, f)
    }

    fn step_with_compensation<T, E, F, Fut, G, GFut>(
        &mut self,
        name: &str,
        forward_fn: F,
        compensate_fn: G,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        G: FnOnce(T) -> GFut + Send + 'static,
        GFut: Future<Output = Result<(), DurableError>> + Send + 'static,
    {
        DurableContext::step_with_compensation(self, name, forward_fn, compensate_fn)
    }

    fn step_with_compensation_opts<T, E, F, Fut, G, GFut>(
        &mut self,
        name: &str,
        options: StepOptions,
        forward_fn: F,
        compensate_fn: G,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        G: FnOnce(T) -> GFut + Send + 'static,
        GFut: Future<Output = Result<(), DurableError>> + Send + 'static,
    {
        DurableContext::step_with_compensation_opts(self, name, options, forward_fn, compensate_fn)
    }

    fn run_compensations(
        &mut self,
    ) -> impl Future<Output = Result<CompensationResult, DurableError>> + Send {
        DurableContext::run_compensations(self)
    }

    fn callback_result<T: DeserializeOwned>(
        &self,
        handle: &CallbackHandle,
    ) -> Result<T, DurableError> {
        DurableContext::callback_result(self, handle)
    }

    fn execution_mode(&self) -> ExecutionMode {
        DurableContext::execution_mode(self)
    }

    fn is_replaying(&self) -> bool {
        DurableContext::is_replaying(self)
    }

    fn arn(&self) -> &str {
        DurableContext::arn(self)
    }

    fn checkpoint_token(&self) -> &str {
        DurableContext::checkpoint_token(self)
    }

    fn log(&self, message: &str) {
        DurableContext::log(self, message);
    }

    fn log_with_data(&self, message: &str, data: &serde_json::Value) {
        DurableContext::log_with_data(self, message, data);
    }

    fn log_debug(&self, message: &str) {
        DurableContext::log_debug(self, message);
    }

    fn log_warn(&self, message: &str) {
        DurableContext::log_warn(self, message);
    }

    fn log_error(&self, message: &str) {
        DurableContext::log_error(self, message);
    }

    fn log_debug_with_data(&self, message: &str, data: &serde_json::Value) {
        DurableContext::log_debug_with_data(self, message, data);
    }

    fn log_warn_with_data(&self, message: &str, data: &serde_json::Value) {
        DurableContext::log_warn_with_data(self, message, data);
    }

    fn log_error_with_data(&self, message: &str, data: &serde_json::Value) {
        DurableContext::log_error_with_data(self, message, data);
    }

    fn enable_batch_mode(&mut self) {
        DurableContext::enable_batch_mode(self);
    }

    fn flush_batch(&mut self) -> impl Future<Output = Result<(), DurableError>> + Send {
        DurableContext::flush_batch(self)
    }
}
