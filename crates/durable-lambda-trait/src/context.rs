//! Trait-specific context wrapper.
//!
//! Thin wrapper over [`DurableContext`](durable_lambda_core::DurableContext)
//! providing trait-approach ergonomics. All operations delegate directly
//! to the inner context with no additional logic.

use std::future::Future;

use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
use durable_lambda_core::ops_trait::DurableContextOps;
use durable_lambda_core::types::{
    BatchResult, CallbackHandle, CallbackOptions, CompensationResult, ExecutionMode, MapOptions,
    ParallelOptions, StepOptions,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Trait-native context for durable Lambda operations.
///
/// Thin wrapper over [`DurableContext`] providing the trait-approach API.
/// All operations delegate directly to the inner context — no replay logic,
/// no checkpoint logic, just delegation.
///
/// Constructed internally by [`run`](crate::run) — users never create this
/// directly.
///
/// # Examples
///
/// ```no_run
/// use durable_lambda_trait::prelude::*;
/// use async_trait::async_trait;
///
/// struct MyHandler;
///
/// #[async_trait]
/// impl DurableHandler for MyHandler {
///     async fn handle(
///         &self,
///         event: serde_json::Value,
///         mut ctx: TraitContext,
///     ) -> Result<serde_json::Value, DurableError> {
///         let result: Result<i32, String> = ctx.step("validate", || async {
///             Ok(42)
///         }).await?;
///         Ok(serde_json::json!({"validated": result.unwrap()}))
///     }
/// }
/// ```
pub struct TraitContext {
    inner: DurableContext,
}

impl TraitContext {
    /// Create from an existing `DurableContext`.
    ///
    /// This constructor is `pub(crate)` — only the [`run`](crate::run)
    /// function creates `TraitContext` instances.
    pub(crate) fn new(ctx: DurableContext) -> Self {
        Self { inner: ctx }
    }

    /// Execute a named step with checkpointing.
    ///
    /// During execution mode, runs the closure and checkpoints the result to AWS.
    /// During replay mode, returns the previously checkpointed result without
    /// executing the closure.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable step name, used as checkpoint metadata
    /// * `f` — Closure to execute (skipped during replay)
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::Serialization`] if the result cannot be serialized.
    /// Returns [`DurableError::Deserialization`] if a cached result cannot be deserialized.
    /// Returns [`DurableError::CheckpointFailed`] if the AWS checkpoint API call fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// let result: Result<i32, String> = ctx.step("validate_order", || async {
    ///     Ok(42)
    /// }).await?;
    ///
    /// match result {
    ///     Ok(value) => println!("Step succeeded: {value}"),
    ///     Err(e) => println!("Step failed: {e}"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn step<T, E, F, Fut>(
        &mut self,
        name: &str,
        f: F,
    ) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        self.inner.step(name, f).await
    }

    /// Execute a named step with checkpointing and retry configuration.
    ///
    /// If the closure fails and retries are configured, sends a RETRY checkpoint
    /// and returns [`DurableError::StepRetryScheduled`] to signal the function
    /// should exit.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable step name, used as checkpoint metadata
    /// * `options` — Retry configuration (see [`StepOptions`])
    /// * `f` — Closure to execute (skipped during replay)
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::StepRetryScheduled`] when a retry has been scheduled.
    /// Returns [`DurableError::Serialization`] if the result cannot be serialized.
    /// Returns [`DurableError::Deserialization`] if a cached result cannot be deserialized.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_trait::prelude::*;
    ///
    /// let result: Result<i32, String> = ctx.step_with_options(
    ///     "charge_payment",
    ///     StepOptions::new().retries(3).backoff_seconds(5),
    ///     || async { Ok(100) },
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn step_with_options<T, E, F, Fut>(
        &mut self,
        name: &str,
        options: StepOptions,
        f: F,
    ) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        self.inner.step_with_options(name, options, f).await
    }

    /// Suspend execution for the specified duration.
    ///
    /// During execution mode, sends a START checkpoint and returns
    /// [`DurableError::WaitSuspended`] to signal the function should exit.
    /// The server re-invokes after the duration.
    ///
    /// During replay mode, returns `Ok(())` immediately if the wait has
    /// already completed.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the wait operation
    /// * `duration_secs` — Duration to wait in seconds
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::WaitSuspended`] when the wait has been checkpointed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// ctx.wait("cooldown", 30).await?;
    /// println!("Wait completed!");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait(&mut self, name: &str, duration_secs: i32) -> Result<(), DurableError> {
        self.inner.wait(name, duration_secs).await
    }

    /// Register a callback and return a handle with the server-generated callback ID.
    ///
    /// During execution mode, sends a START checkpoint and returns a
    /// [`CallbackHandle`] containing the `callback_id` for external systems.
    /// During replay mode, extracts the cached callback_id from history.
    ///
    /// This method NEVER suspends. Use [`callback_result`](Self::callback_result)
    /// to check the callback outcome (which suspends if not yet signaled).
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the callback operation
    /// * `options` — Timeout configuration (see [`CallbackOptions`])
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::CheckpointFailed`] if the AWS checkpoint API call fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_trait::prelude::*;
    ///
    /// let handle = ctx.create_callback("approval", CallbackOptions::new()).await?;
    /// println!("Callback ID: {}", handle.callback_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_callback(
        &mut self,
        name: &str,
        options: CallbackOptions,
    ) -> Result<CallbackHandle, DurableError> {
        self.inner.create_callback(name, options).await
    }

    /// Check the result of a previously created callback.
    ///
    /// Return the deserialized success payload if the callback has been
    /// signaled. Return an error if the callback failed, timed out, or
    /// hasn't been signaled yet.
    ///
    /// # Arguments
    ///
    /// * `handle` — The [`CallbackHandle`] returned by [`create_callback`](Self::create_callback)
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::CallbackSuspended`] if not yet signaled.
    /// Returns [`DurableError::CallbackFailed`] if the callback failed or timed out.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_trait::prelude::*;
    ///
    /// let handle = ctx.create_callback("approval", CallbackOptions::new()).await?;
    /// let result: String = ctx.callback_result(&handle)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn callback_result<T: DeserializeOwned>(
        &self,
        handle: &CallbackHandle,
    ) -> Result<T, DurableError> {
        self.inner.callback_result(handle)
    }

    /// Durably invoke another Lambda function and return its result.
    ///
    /// During execution mode, serializes the payload, sends a START checkpoint,
    /// and returns [`DurableError::InvokeSuspended`] to signal exit. The server
    /// invokes the target asynchronously and re-invokes this Lambda when done.
    ///
    /// During replay, returns the cached result without re-invoking.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the invoke operation
    /// * `function_name` — Name or ARN of the target Lambda function
    /// * `payload` — Input payload to send to the target function
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::InvokeSuspended`] when the target is still executing.
    /// Returns [`DurableError::InvokeFailed`] if the target failed or timed out.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// let result: String = ctx.invoke(
    ///     "call_processor",
    ///     "payment-processor-lambda",
    ///     &serde_json::json!({"order_id": 123}),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invoke<T, P>(
        &mut self,
        name: &str,
        function_name: &str,
        payload: &P,
    ) -> Result<T, DurableError>
    where
        T: DeserializeOwned,
        P: Serialize,
    {
        self.inner.invoke(name, function_name, payload).await
    }

    /// Execute multiple branches concurrently and return their results.
    ///
    /// Each branch receives an owned child context with an isolated checkpoint
    /// namespace. Branches satisfy `Send + 'static` via `tokio::spawn`.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the parallel operation
    /// * `branches` — Collection of branch closures
    /// * `options` — Parallel configuration
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::ParallelFailed`] if the operation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_trait::prelude::*;
    /// use durable_lambda_core::context::DurableContext;
    /// use std::pin::Pin;
    /// use std::future::Future;
    ///
    /// type BranchFn = Box<dyn FnOnce(DurableContext) -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>> + Send>;
    ///
    /// let branches: Vec<BranchFn> = vec![
    ///     Box::new(|_ctx| Box::pin(async move { Ok(1) })),
    ///     Box::new(|_ctx| Box::pin(async move { Ok(2) })),
    /// ];
    /// let result = ctx.parallel("fan_out", branches, ParallelOptions::new()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn parallel<T, F, Fut>(
        &mut self,
        name: &str,
        branches: Vec<F>,
        options: ParallelOptions,
    ) -> Result<BatchResult<T>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce(DurableContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, DurableError>> + Send + 'static,
    {
        self.inner.parallel(name, branches, options).await
    }

    /// Execute an isolated subflow with its own checkpoint namespace.
    ///
    /// The closure receives an owned child [`DurableContext`] whose operations
    /// are namespaced under this child context's operation ID, preventing
    /// collisions with the parent or sibling child contexts.
    ///
    /// During replay mode, returns the cached result without re-executing
    /// the closure.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the child context operation
    /// * `f` — Closure receiving an owned `DurableContext` for the subflow
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::ChildContextFailed`] if the child context
    /// is found in a failed state during replay.
    /// Returns [`DurableError::CheckpointFailed`] if checkpoint API calls fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_core::context::DurableContext;
    ///
    /// let result: i32 = ctx.child_context("sub_workflow", |mut child_ctx: DurableContext| async move {
    ///     let r: Result<i32, String> = child_ctx.step("inner_step", || async { Ok(42) }).await?;
    ///     Ok(r.unwrap())
    /// }).await?;
    /// assert_eq!(result, 42);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn child_context<T, F, Fut>(&mut self, name: &str, f: F) -> Result<T, DurableError>
    where
        T: Serialize + DeserializeOwned + Send,
        F: FnOnce(DurableContext) -> Fut + Send,
        Fut: Future<Output = Result<T, DurableError>> + Send,
    {
        self.inner.child_context(name, f).await
    }

    /// Process a collection of items in parallel and return their results.
    ///
    /// Apply the closure `f` to each item concurrently. Each item receives an
    /// owned child context with an isolated checkpoint namespace. Items satisfy
    /// `Send + 'static` via `tokio::spawn`. The closure must be `Clone` since
    /// it is applied to each item independently.
    ///
    /// When `batch_size` is configured, items process in sequential batches.
    ///
    /// # Arguments
    ///
    /// * `name` — Human-readable name for the map operation
    /// * `items` — Collection of items to process
    /// * `options` — Map configuration (batching)
    /// * `f` — Closure applied to each item with an owned child context
    ///
    /// # Errors
    ///
    /// Returns [`DurableError::MapFailed`] if the operation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_trait::prelude::*;
    /// use durable_lambda_core::context::DurableContext;
    ///
    /// let items = vec![1, 2, 3];
    /// let result = ctx.map(
    ///     "process_items",
    ///     items,
    ///     MapOptions::new().batch_size(2),
    ///     |item: i32, mut child_ctx: DurableContext| async move {
    ///         let r: Result<i32, String> = child_ctx.step("double", move || async move { Ok(item * 2) }).await?;
    ///         Ok(r.unwrap())
    ///     },
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn map<T, I, F, Fut>(
        &mut self,
        name: &str,
        items: Vec<I>,
        options: MapOptions,
        f: F,
    ) -> Result<BatchResult<T>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        I: Send + 'static,
        F: FnOnce(I, DurableContext) -> Fut + Send + 'static + Clone,
        Fut: Future<Output = Result<T, DurableError>> + Send + 'static,
    {
        self.inner.map(name, items, options, f).await
    }

    /// Register a compensatable step.
    ///
    /// Executes the forward step and, on success, registers the compensation
    /// closure for later rollback via [`run_compensations`](Self::run_compensations).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// let result: Result<i32, String> = ctx.step_with_compensation(
    ///     "charge",
    ///     || async { Ok(100) },
    ///     |amount| async move { println!("Refunding {amount}"); Ok(()) },
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn step_with_compensation<T, E, F, Fut, G, GFut>(
        &mut self,
        name: &str,
        forward_fn: F,
        compensate_fn: G,
    ) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        G: FnOnce(T) -> GFut + Send + 'static,
        GFut: Future<Output = Result<(), DurableError>> + Send + 'static,
    {
        self.inner
            .step_with_compensation(name, forward_fn, compensate_fn)
            .await
    }

    /// Register a compensatable step with options.
    ///
    /// Like [`step_with_compensation`](Self::step_with_compensation) but accepts
    /// [`StepOptions`] for configuring retries, backoff, and timeouts on the forward step.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// use durable_lambda_trait::prelude::*;
    ///
    /// let result: Result<String, String> = ctx.step_with_compensation_opts(
    ///     "book_hotel",
    ///     StepOptions::new().retries(3),
    ///     || async { Ok("BOOKING-123".to_string()) },
    ///     |booking_id| async move { println!("Cancelling: {booking_id}"); Ok(()) },
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn step_with_compensation_opts<T, E, F, Fut, G, GFut>(
        &mut self,
        name: &str,
        options: StepOptions,
        forward_fn: F,
        compensate_fn: G,
    ) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        E: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        G: FnOnce(T) -> GFut + Send + 'static,
        GFut: Future<Output = Result<(), DurableError>> + Send + 'static,
    {
        self.inner
            .step_with_compensation_opts(name, options, forward_fn, compensate_fn)
            .await
    }

    /// Execute all registered compensations in reverse registration order.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &mut durable_lambda_trait::context::TraitContext) -> Result<(), durable_lambda_core::error::DurableError> {
    /// let result = ctx.run_compensations().await?;
    /// if !result.all_succeeded {
    ///     eprintln!("Some compensations failed");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_compensations(&mut self) -> Result<CompensationResult, DurableError> {
        self.inner.run_compensations().await
    }

    /// Return the current execution mode (Replaying or Executing).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// use durable_lambda_core::types::ExecutionMode;
    /// match ctx.execution_mode() {
    ///     ExecutionMode::Replaying => { /* returning cached results */ }
    ///     ExecutionMode::Executing => { /* running new operations */ }
    /// }
    /// # }
    /// ```
    pub fn execution_mode(&self) -> ExecutionMode {
        self.inner.execution_mode()
    }

    /// Return whether the context is currently replaying from history.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// if ctx.is_replaying() {
    ///     println!("Replaying cached operations");
    /// }
    /// # }
    /// ```
    pub fn is_replaying(&self) -> bool {
        self.inner.is_replaying()
    }

    /// Return a reference to the durable execution ARN.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// println!("Execution ARN: {}", ctx.arn());
    /// # }
    /// ```
    pub fn arn(&self) -> &str {
        self.inner.arn()
    }

    /// Return the current checkpoint token.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// let token = ctx.checkpoint_token();
    /// # }
    /// ```
    pub fn checkpoint_token(&self) -> &str {
        self.inner.checkpoint_token()
    }

    /// Emit a replay-safe info-level log message.
    ///
    /// During execution mode, emits via `tracing::info!` with execution
    /// context enrichment. During replay mode, the call is a no-op.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// ctx.log("Order processing started");
    /// # }
    /// ```
    pub fn log(&self, message: &str) {
        self.inner.log(message);
    }

    /// Emit a replay-safe info-level log message with structured data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// ctx.log_with_data("Order processed", &serde_json::json!({"order_id": 42}));
    /// # }
    /// ```
    pub fn log_with_data(&self, message: &str, data: &serde_json::Value) {
        self.inner.log_with_data(message, data);
    }

    /// Emit a replay-safe debug-level log message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// ctx.log_debug("Validating order fields");
    /// # }
    /// ```
    pub fn log_debug(&self, message: &str) {
        self.inner.log_debug(message);
    }

    /// Emit a replay-safe warn-level log message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// ctx.log_warn("Inventory below threshold");
    /// # }
    /// ```
    pub fn log_warn(&self, message: &str) {
        self.inner.log_warn(message);
    }

    /// Emit a replay-safe error-level log message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// ctx.log_error("Payment gateway timeout");
    /// # }
    /// ```
    pub fn log_error(&self, message: &str) {
        self.inner.log_error(message);
    }

    /// Emit a replay-safe debug-level log message with structured data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// ctx.log_debug_with_data("Request details", &serde_json::json!({"method": "POST"}));
    /// # }
    /// ```
    pub fn log_debug_with_data(&self, message: &str, data: &serde_json::Value) {
        self.inner.log_debug_with_data(message, data);
    }

    /// Emit a replay-safe warn-level log message with structured data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// ctx.log_warn_with_data("Retry attempt", &serde_json::json!({"attempt": 3}));
    /// # }
    /// ```
    pub fn log_warn_with_data(&self, message: &str, data: &serde_json::Value) {
        self.inner.log_warn_with_data(message, data);
    }

    /// Emit a replay-safe error-level log message with structured data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(ctx: &durable_lambda_trait::context::TraitContext) {
    /// ctx.log_error_with_data("Payment failed", &serde_json::json!({"error": "timeout"}));
    /// # }
    /// ```
    pub fn log_error_with_data(&self, message: &str, data: &serde_json::Value) {
        self.inner.log_error_with_data(message, data);
    }
}

impl DurableContextOps for TraitContext {
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
        self.inner.step(name, f)
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
        self.inner.step_with_options(name, options, f)
    }

    fn wait(
        &mut self,
        name: &str,
        duration_secs: i32,
    ) -> impl Future<Output = Result<(), DurableError>> + Send {
        self.inner.wait(name, duration_secs)
    }

    fn create_callback(
        &mut self,
        name: &str,
        options: CallbackOptions,
    ) -> impl Future<Output = Result<CallbackHandle, DurableError>> + Send {
        self.inner.create_callback(name, options)
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
        self.inner.invoke(name, function_name, payload)
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
        self.inner.parallel(name, branches, options)
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
        self.inner.child_context(name, f)
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
        self.inner.map(name, items, options, f)
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
        self.inner
            .step_with_compensation(name, forward_fn, compensate_fn)
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
        self.inner
            .step_with_compensation_opts(name, options, forward_fn, compensate_fn)
    }

    fn run_compensations(
        &mut self,
    ) -> impl Future<Output = Result<CompensationResult, DurableError>> + Send {
        self.inner.run_compensations()
    }

    fn callback_result<T: DeserializeOwned>(
        &self,
        handle: &CallbackHandle,
    ) -> Result<T, DurableError> {
        self.inner.callback_result(handle)
    }

    fn execution_mode(&self) -> ExecutionMode {
        self.inner.execution_mode()
    }

    fn is_replaying(&self) -> bool {
        self.inner.is_replaying()
    }

    fn arn(&self) -> &str {
        self.inner.arn()
    }

    fn checkpoint_token(&self) -> &str {
        self.inner.checkpoint_token()
    }

    fn log(&self, message: &str) {
        self.inner.log(message);
    }

    fn log_with_data(&self, message: &str, data: &serde_json::Value) {
        self.inner.log_with_data(message, data);
    }

    fn log_debug(&self, message: &str) {
        self.inner.log_debug(message);
    }

    fn log_warn(&self, message: &str) {
        self.inner.log_warn(message);
    }

    fn log_error(&self, message: &str) {
        self.inner.log_error(message);
    }

    fn log_debug_with_data(&self, message: &str, data: &serde_json::Value) {
        self.inner.log_debug_with_data(message, data);
    }

    fn log_warn_with_data(&self, message: &str, data: &serde_json::Value) {
        self.inner.log_warn_with_data(message, data);
    }

    fn log_error_with_data(&self, message: &str, data: &serde_json::Value) {
        self.inner.log_error_with_data(message, data);
    }

    fn enable_batch_mode(&mut self) {
        self.inner.enable_batch_mode();
    }

    fn flush_batch(&mut self) -> impl Future<Output = Result<(), DurableError>> + Send {
        self.inner.flush_batch()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
    use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
    use aws_sdk_lambda::types::{
        Operation, OperationStatus, OperationType, OperationUpdate, StepDetails,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::handler::DurableHandler;
    use durable_lambda_core::backend::DurableBackend;

    /// Mock backend for testing TraitContext delegation.
    struct MockBackend {
        operations: HashMap<String, Operation>,
        checkpoint_token: Mutex<String>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                operations: HashMap::new(),
                checkpoint_token: Mutex::new("mock-token".to_string()),
            }
        }

        fn with_operation(mut self, id: &str, operation: Operation) -> Self {
            self.operations.insert(id.to_string(), operation);
            self
        }
    }

    #[async_trait::async_trait]
    impl DurableBackend for MockBackend {
        async fn checkpoint(
            &self,
            _arn: &str,
            _checkpoint_token: &str,
            _updates: Vec<OperationUpdate>,
            _client_token: Option<&str>,
        ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
            let token = self.checkpoint_token.lock().unwrap().clone();
            Ok(CheckpointDurableExecutionOutput::builder()
                .checkpoint_token(token)
                .build())
        }

        async fn get_execution_state(
            &self,
            _arn: &str,
            _checkpoint_token: &str,
            _next_marker: &str,
            _max_items: i32,
        ) -> Result<GetDurableExecutionStateOutput, DurableError> {
            Ok(GetDurableExecutionStateOutput::builder()
                .build()
                .expect("test: empty execution state"))
        }
    }

    async fn make_trait_context(backend: MockBackend) -> TraitContext {
        let ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![],
            None,
        )
        .await
        .unwrap();
        TraitContext::new(ctx)
    }

    fn make_succeeded_op(id: &str, result_json: &str) -> Operation {
        Operation::builder()
            .id(id)
            .r#type(OperationType::Step)
            .status(OperationStatus::Succeeded)
            .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
            .step_details(StepDetails::builder().result(result_json).build())
            .build()
            .unwrap_or_else(|e| panic!("failed to build test Operation: {e}"))
    }

    // --- Task 4.1: Test DurableHandler trait compilation and callability ---

    struct TestHandler;

    #[async_trait::async_trait]
    impl crate::handler::DurableHandler for TestHandler {
        async fn handle(
            &self,
            _event: serde_json::Value,
            mut ctx: TraitContext,
        ) -> Result<serde_json::Value, DurableError> {
            let result: Result<i32, String> = ctx.step("test_step", || async { Ok(42) }).await?;
            Ok(serde_json::json!({"value": result.unwrap()}))
        }
    }

    #[tokio::test]
    async fn test_durable_handler_trait_compiles_and_is_callable() {
        let ctx = make_trait_context(MockBackend::new()).await;
        let handler = TestHandler;
        let result = handler.handle(serde_json::json!({}), ctx).await.unwrap();
        assert_eq!(result, serde_json::json!({"value": 42}));
    }

    // --- Task 4.2: Test TraitContext delegation ---

    #[tokio::test]
    async fn test_trait_context_step_delegates_to_core() {
        let mut ctx = make_trait_context(MockBackend::new()).await;

        let result: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await.unwrap();

        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_trait_context_step_with_options_delegates_to_core() {
        let mut ctx = make_trait_context(MockBackend::new()).await;

        let result: Result<i32, String> = ctx
            .step_with_options("charge", StepOptions::new().retries(3), || async {
                Ok(100)
            })
            .await
            .unwrap();

        assert_eq!(result, Ok(100));
    }

    #[tokio::test]
    async fn test_trait_context_execution_mode_executing() {
        let ctx = make_trait_context(MockBackend::new()).await;
        assert_eq!(ctx.execution_mode(), ExecutionMode::Executing);
        assert!(!ctx.is_replaying());
    }

    #[tokio::test]
    async fn test_trait_context_execution_mode_replaying() {
        let op = make_succeeded_op("op-1", "42");
        let backend = MockBackend::new().with_operation("op-1", op.clone());

        let durable_ctx = DurableContext::new(
            Arc::new(backend),
            "arn:test".to_string(),
            "tok".to_string(),
            vec![op],
            None,
        )
        .await
        .unwrap();

        let ctx = TraitContext::new(durable_ctx);
        assert_eq!(ctx.execution_mode(), ExecutionMode::Replaying);
        assert!(ctx.is_replaying());
    }

    #[tokio::test]
    async fn test_trait_context_arn() {
        let ctx = make_trait_context(MockBackend::new()).await;
        assert_eq!(ctx.arn(), "arn:test");
    }

    #[tokio::test]
    async fn test_trait_context_checkpoint_token() {
        let ctx = make_trait_context(MockBackend::new()).await;
        assert_eq!(ctx.checkpoint_token(), "tok");
    }

    #[tokio::test]
    async fn test_trait_context_child_context_delegates_to_core() {
        let mut ctx = make_trait_context(MockBackend::new()).await;

        let result: i32 = ctx
            .child_context("sub_workflow", |mut child_ctx: DurableContext| async move {
                let r: Result<i32, String> =
                    child_ctx.step("inner_step", || async { Ok(42) }).await?;
                Ok(r.unwrap())
            })
            .await
            .unwrap();

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_trait_context_log_delegates_to_core() {
        let ctx = make_trait_context(MockBackend::new()).await;
        // Verify log methods are callable and don't panic.
        ctx.log("test message");
        ctx.log_with_data("test data", &serde_json::json!({"key": "val"}));
        ctx.log_debug("test debug");
        ctx.log_warn("test warn");
        ctx.log_error("test error");
        ctx.log_debug_with_data("debug data", &serde_json::json!({"k": "v"}));
        ctx.log_warn_with_data("warn data", &serde_json::json!({"k": "v"}));
        ctx.log_error_with_data("error data", &serde_json::json!({"k": "v"}));
    }

    // --- Task 4.3: Test run() function type signature ---

    #[test]
    fn test_run_function_accepts_durable_handler_implementors() {
        // Verify the run() function signature compiles with a DurableHandler impl.
        // We cannot actually call run() since it starts the Lambda runtime,
        // but we can verify the type constraint is satisfied.
        fn assert_handler_accepted<H: crate::handler::DurableHandler>(_h: H) {}
        assert_handler_accepted(TestHandler);
    }
}
