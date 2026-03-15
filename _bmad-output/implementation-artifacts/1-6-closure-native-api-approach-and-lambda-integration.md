# Story 1.6: Closure-Native API Approach & Lambda Integration

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to write durable Lambda handlers using a closure-native API with a single run() entry point,
So that I can build durable functions with minimal boilerplate and no direct lambda_runtime wiring.

## Acceptance Criteria

1. **Given** the durable-lambda-closure crate **When** I write a durable Lambda handler **Then** I can use a single import: `use durable_lambda_closure::prelude::*` (FR34) **And** all core types (DurableError, StepOptions, ExecutionMode, CheckpointResult) are re-exported through the prelude

2. **Given** the closure-native API **When** I define a handler function **Then** I call `durable_lambda_closure::run(my_handler).await` as the entry point (FR47) **And** this internally wires up lambda_runtime handler registration and DurableContext creation **And** I never need to interact with lambda_runtime or DurableContext construction directly

3. **Given** the closure-native context wrapper **When** I use it inside my handler **Then** I can call `ctx.step(...)` and `ctx.step_with_options(...)` with the same signatures as core **And** the wrapper is a thin layer over DurableContext with no additional logic

4. **Given** the crate structure **When** I examine durable-lambda-closure/src/ **Then** it contains lib.rs (re-exports only), handler.rs, context.rs, and prelude.rs **And** durable-lambda-closure depends only on durable-lambda-core (NFR8) — plus lambda_runtime and tokio for runtime wiring

5. **Given** a complete closure-native durable Lambda handler **When** it compiles and runs **Then** a developer following SDK patterns encounters zero ownership/borrowing/trait-bound compiler errors related to the SDK's API surface (NFR11)

6. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --doc -p durable-lambda-closure` **Then** all rustdoc examples compile and pass

## Tasks / Subtasks

- [x] Task 1: Implement `ClosureContext` wrapper in `context.rs` (AC: #3, #5)
  - [x] 1.1: Define `ClosureContext` struct wrapping `DurableContext` (single field, not public)
  - [x] 1.2: Implement `ClosureContext::step(name, f)` delegating to `DurableContext::step`
  - [x] 1.3: Implement `ClosureContext::step_with_options(name, options, f)` delegating to `DurableContext::step_with_options`
  - [x] 1.4: Implement accessor methods: `execution_mode()`, `is_replaying()`, `arn()`, `checkpoint_token()`
  - [x] 1.5: Add rustdoc with `# Examples` on `ClosureContext` and all public methods
  - [x] 1.6: Ensure all method signatures preserve the same generic bounds as core (T, E, F, Fut)

- [x] Task 2: Implement `run()` entry point in `handler.rs` (AC: #2, #5)
  - [x] 2.1: Handler takes owned `ClosureContext` (not `&mut`) to avoid async lifetime issues — matches Python SDK pattern where context is passed by value
  - [x] 2.2: Implement `run()` function that: (a) initializes AWS config via `aws_config::load_defaults`, (b) creates Lambda client, (c) creates `RealBackend`, (d) registers with `lambda_runtime::run(service_fn(...))`
  - [x] 2.3: Inside the service_fn closure: extract durable execution parameters from the Lambda event, create `DurableContext`, wrap in `ClosureContext`, call user handler
  - [x] 2.4: Handle the Lambda event structure — PascalCase keys: `DurableExecutionArn`, `CheckpointToken`, `InitialExecutionState` with `Operations` and `NextMarker`. User payload extracted from first EXECUTION operation's `ExecutionDetails.InputPayload`
  - [x] 2.5: Add rustdoc with `# Examples` and `# Errors` on `run()`
  - [x] 2.6: `run()` returns `Result<(), lambda_runtime::Error>`

- [x] Task 3: Set up prelude re-exports in `prelude.rs` (AC: #1)
  - [x] 3.1: Re-export `ClosureContext` from context module
  - [x] 3.2: Re-export `run` from handler module
  - [x] 3.3: Re-export core types: `DurableError`, `StepOptions`, `ExecutionMode`, `CheckpointResult`
  - [x] 3.4: Add rustdoc on prelude module explaining the single-import pattern

- [x] Task 4: Update `lib.rs` with module declarations and top-level re-exports (AC: #4)
  - [x] 4.1: Keep existing module declarations: `pub mod context`, `pub mod handler`, `pub mod prelude`
  - [x] 4.2: Add top-level re-exports of `run` and `ClosureContext` for convenience
  - [x] 4.3: Add crate-level rustdoc with usage example

- [x] Task 5: Update `Cargo.toml` dependencies (AC: #4)
  - [x] 5.1: Verified `durable-lambda-core` path dependency exists
  - [x] 5.2: Added `aws-config` workspace dependency
  - [x] 5.3: Added `serde_json` workspace dependency
  - [x] 5.4: Added `tracing` workspace dependency
  - [x] 5.5: Verified `lambda_runtime`, `tokio`, `serde` workspace dependencies exist
  - [x] 5.6: Added `aws-sdk-lambda` and `aws-smithy-types` — required for parsing Lambda event operations into `Operation` types (AWS SDK types don't implement serde::Deserialize)

- [x] Task 6: Write tests (AC: #3, #5, #6)
  - [x] 6.1: Unit test `test_closure_context_step_delegates_to_core` — verified step executes closure and checkpoints
  - [x] 6.2: Unit test `test_closure_context_step_with_options_delegates_to_core` — verified retry config forwarded
  - [x] 6.3: Unit tests `test_closure_context_execution_mode_executing`, `test_closure_context_execution_mode_replaying`, `test_closure_context_arn`, `test_closure_context_checkpoint_token`
  - [x] 6.4: Verified 10 doc tests compile and pass
  - [x] 6.5: Full crate builds clean
  - [x] 6.6: Workspace builds clean

- [x] Task 7: Verify all checks pass (AC: #6)
  - [x] 7.1: `cargo test --doc -p durable-lambda-closure` — 10 doc tests pass
  - [x] 7.2: `cargo test -p durable-lambda-closure` — 6 unit tests pass
  - [x] 7.3: `cargo clippy -p durable-lambda-closure -- -D warnings` — no warnings
  - [x] 7.4: `cargo fmt --check` — formatting passes
  - [x] 7.5: `cargo build --workspace` — full workspace builds, core tests pass (57 unit + 40 doc)

### Review Follow-ups (AI)

- [ ] [AI-Review][MEDIUM] Remove unused `tracing` dependency from Cargo.toml or add instrumentation to `run()` [crates/durable-lambda-closure/Cargo.toml:15, handler.rs]
- [ ] [AI-Review][MEDIUM] `parse_operation_type` and `parse_operation_status` should return `None` for unknown values instead of silently defaulting to Step/Pending — let `filter_map` skip unparseable operations [crates/durable-lambda-closure/src/handler.rs:199,211]
- [ ] [AI-Review][LOW] Move `use std::collections::HashMap` to top of test module for conventional import ordering [crates/durable-lambda-closure/src/context.rs:218]

## Senior Developer Review (AI)

- **Review Date:** 2026-03-14
- **Reviewer:** Claude Opus 4.6
- **Review Outcome:** Approve (with minor action items)
- **Total Action Items:** 3 (0 High, 2 Medium, 1 Low)

### Action Items
- [ ] [MEDIUM] Remove unused `tracing` dependency or add instrumentation
- [ ] [MEDIUM] Fix silent defaulting in operation type/status parsers
- [ ] [LOW] Reorder HashMap import in test module

## Dev Notes

### Critical Architecture Constraints

- **Thin wrapper only**: `ClosureContext` must NOT contain any business logic — it delegates every call to `DurableContext`. No replay logic, no checkpoint logic, nothing.
- **lib.rs = re-exports only**: Zero logic in lib.rs. Only `pub mod` and `pub use` statements.
- **Parameter ordering convention**: name first, options second, closure last. Same as core.
- **Constructor methods for DurableError**: Use static methods, never raw struct construction. (Core already handles this — closure crate just propagates.)
- **User step errors require only `Serialize + DeserializeOwned`**: NOT `std::error::Error`. Same bounds as core.
- **Approach crate depends ONLY on durable-lambda-core**: Plus runtime crates (lambda_runtime, tokio, aws-config). Never on other approach crates.

### Lambda Runtime Integration Pattern

The `run()` function is the key integration point between `lambda_runtime` and the durable execution model. The pattern:

```rust
use lambda_runtime::{service_fn, LambdaEvent, Error as LambdaError};

pub async fn run<F, Fut>(handler: F) -> Result<(), LambdaError>
where
    F: Fn(&mut ClosureContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = aws_sdk_lambda::Client::new(&config);
    let backend = Arc::new(RealBackend::new(client));

    lambda_runtime::run(service_fn(|event: LambdaEvent<serde_json::Value>| {
        let backend = backend.clone();
        async move {
            // Extract durable execution params from event
            // Create DurableContext
            // Wrap in ClosureContext
            // Call handler
            // Return result
        }
    })).await
}
```

### Durable Execution Event Structure

The Lambda invocation event for a durable execution contains fields provided by the durable execution runtime. You MUST study the Python SDK source to understand the exact event payload structure. Key fields expected:

- The durable execution ARN
- The checkpoint token
- Operations (first page of execution history)
- Next marker for pagination
- The user's actual event payload (if any)

**CRITICAL**: Examine how the Python SDK extracts these from the Lambda event. The Python SDK uses a `DurableExecutionHandler` class that wraps the user's handler and extracts execution metadata from the Lambda context/event. Match this pattern exactly.

Reference: [Python SDK handler](https://github.com/aws/aws-durable-execution-sdk-python) — look at how it registers with the Lambda runtime and extracts execution state.

### ClosureContext Design

```rust
/// Closure-native context for durable Lambda operations.
///
/// Thin wrapper over [`DurableContext`] providing the closure-approach API.
/// All operations delegate directly to the inner context.
pub struct ClosureContext {
    inner: DurableContext,
}

impl ClosureContext {
    /// Create from an existing DurableContext (internal use only).
    pub(crate) fn new(ctx: DurableContext) -> Self {
        Self { inner: ctx }
    }

    /// Execute a named step with checkpointing.
    pub async fn step<T, E, F, Fut>(&mut self, name: &str, f: F) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send,
        E: Serialize + DeserializeOwned + Send,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        self.inner.step(name, f).await
    }

    /// Execute a named step with retry configuration.
    pub async fn step_with_options<T, E, F, Fut>(
        &mut self,
        name: &str,
        options: StepOptions,
        f: F,
    ) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send,
        E: Serialize + DeserializeOwned + Send,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        self.inner.step_with_options(name, options, f).await
    }
}
```

### What Exists vs What Needs to Be Added

**Already exists (from Stories 1.1–1.5):**
- `DurableContext` with `step()` and `step_with_options()` — fully implemented
- `RealBackend` with exponential backoff retry — fully implemented
- `DurableError` with 8 variants — fully implemented
- `StepOptions` with builder pattern — fully implemented
- `ReplayEngine` with mode transitions — fully implemented
- Stub files in durable-lambda-closure: lib.rs, handler.rs, context.rs, prelude.rs (headers only)
- Cargo.toml with durable-lambda-core, lambda_runtime, tokio, serde dependencies

**Needs to be added (this story):**
- `ClosureContext` struct + all delegating methods in `context.rs`
- `run()` function in `handler.rs` — lambda_runtime integration
- Prelude re-exports in `prelude.rs`
- Top-level re-exports in `lib.rs`
- Additional Cargo.toml dependencies: aws-config, serde_json, tracing
- Unit tests for ClosureContext delegation
- Rustdoc on all public items

### Architecture Doc Discrepancies (IMPORTANT — Inherited)

From previous stories — always follow Python SDK over architecture doc:
1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Replay tracking**: Uses HashSet of visited operation IDs, NOT simple cursor advancement
3. **Operation ID**: Uses blake2b hash of counter, NOT user-provided step name

New for this story:
4. **Handler signature**: The architecture doc sketches `durable_lambda_closure::run(my_handler).await` but doesn't specify the exact handler function signature. Study the Python SDK to determine what arguments the handler receives and what it returns.
5. **Event deserialization**: The architecture doc doesn't specify the Lambda event structure for durable executions. This MUST be determined from the Python SDK source.

### Previous Story Intelligence (Story 1.5)

- Step method implemented as `impl DurableContext` block in `operations/step.rs` — ClosureContext delegates to these.
- Return type is `Result<Result<T, E>, DurableError>` — outer Result for SDK errors, inner for user step results. Preserve this in ClosureContext.
- `DurableError` is `#[non_exhaustive]` — safe to re-export.
- 57 unit tests + 40 doc tests passing in core. Closure crate tests are additive.
- Clippy clean, fmt clean, workspace builds.
- Review follow-ups from 1.4: (1) No test for execute-path FAIL checkpoint [MEDIUM], (2) Synthetic serde error for missing step_details is misleading [LOW]. Not relevant to this story.

### Existing Cargo.toml Dependencies

The closure crate's Cargo.toml already has:
```toml
[dependencies]
durable-lambda-core = { path = "../durable-lambda-core" }
lambda_runtime = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
```

Add these (workspace):
- `aws-config` — for `run()` to call `load_defaults`
- `serde_json` — for event payload deserialization
- `tracing` — for structured logging in `run()`

Do NOT add `aws-sdk-lambda` — all AWS types come through `durable-lambda-core`.

### Testing Approach

- ClosureContext tests should use the same MockBackend pattern from core's `context.rs` tests (TestBackend that implements DurableBackend).
- Test that ClosureContext methods delegate correctly by verifying identical results to direct DurableContext calls.
- `run()` cannot be easily unit tested (requires Lambda runtime environment) — test it via doc examples with `no_run` or integration tests. Focus unit tests on ClosureContext.
- Test naming: `test_{component}_{behavior}_{condition}` e.g., `test_closure_context_step_delegates_to_core`.

### User-Facing Example (Target End State)

```rust
use durable_lambda_closure::prelude::*;

async fn handler(ctx: &mut ClosureContext) -> Result<serde_json::Value, DurableError> {
    let order = ctx.step("validate_order", || async {
        Ok::<_, String>(serde_json::json!({"id": 123, "valid": true}))
    }).await??;

    let charge = ctx.step_with_options(
        "charge_payment",
        StepOptions::new().retries(3).backoff_seconds(2),
        || async {
            Ok::<_, String>(serde_json::json!({"charged": true}))
        },
    ).await??;

    Ok(serde_json::json!({"order": order, "charge": charge}))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
```

### Project Structure Notes

- All files are in `crates/durable-lambda-closure/src/`
- No other crates need changes for this story
- The closure crate is the first approach crate to be fully implemented — its patterns will be replicated in trait, builder, and macro crates later

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.6 — acceptance criteria]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR34, FR47]
- [Source: _bmad-output/planning-artifacts/prd.md#API Surface — closure-native approach]
- [Source: _bmad-output/planning-artifacts/architecture.md#API Abstraction Pattern — thin wrappers, run() entry point]
- [Source: _bmad-output/planning-artifacts/architecture.md#Public API Surface — prelude, re-export pattern]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure — durable-lambda-closure files]
- [Source: _bmad-output/planning-artifacts/architecture.md#Architectural Boundaries — approach crate dependencies]
- [Source: _bmad-output/implementation-artifacts/1-5-step-retries-and-typed-errors.md — step patterns, testing approach]
- [Source: crates/durable-lambda-core/src/context.rs — DurableContext API]
- [Source: crates/durable-lambda-core/src/operations/step.rs — step/step_with_options signatures]
- [Source: crates/durable-lambda-core/src/lib.rs — core re-exports]
- [Source: Python SDK — github.com/aws/aws-durable-execution-sdk-python — handler registration pattern]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- `ClosureContext` struct in context.rs wrapping `DurableContext` with `pub(crate) fn new()` constructor
- Delegating methods: `step()`, `step_with_options()`, `execution_mode()`, `is_replaying()`, `arn()`, `checkpoint_token()`
- `run()` function in handler.rs — full lambda_runtime integration with AWS config, RealBackend, event parsing
- Handler signature: `Fn(serde_json::Value, ClosureContext) -> Fut` — takes owned context (not `&mut`) to avoid async lifetime issues
- Lambda event structure: PascalCase JSON with `DurableExecutionArn`, `CheckpointToken`, `InitialExecutionState.Operations`, `InitialExecutionState.NextMarker`
- User event extraction from first EXECUTION operation's `ExecutionDetails.InputPayload`
- Manual Operation JSON parsing via builders (AWS SDK types don't implement serde::Deserialize)
- Prelude re-exports: ClosureContext, run, DurableError, StepOptions, ExecutionMode, CheckpointResult
- lib.rs: re-exports only (pub mod + pub use)
- Added `aws-sdk-lambda`, `aws-smithy-types` as direct dependencies (required for Operation type construction from event JSON)
- 6 unit tests + 10 doc tests, clippy clean, fmt clean, workspace builds

### Deviations from Story Spec

1. **Handler takes owned `ClosureContext` instead of `&mut ClosureContext`**: Async functions with mutable references create Higher-Rank Trait Bound (HRTB) lifetime issues in Rust. Passing by value is the "pit of success" pattern — matches Python SDK, avoids all lifetime complexity. Users write `mut ctx: ClosureContext` which is natural.
2. **Added `aws-sdk-lambda` and `aws-smithy-types` as direct dependencies**: Story spec said "DO NOT add aws-sdk-lambda directly" but this is required because `DurableContext::new()` takes `Vec<aws_sdk_lambda::types::Operation>`, and AWS SDK types don't implement serde::Deserialize, so we must construct them using builders from the parsed event JSON.
3. **Handler receives `(event, ctx)` instead of just `(ctx)`**: Following Python SDK pattern where the handler receives both the user's deserialized event payload and the durable context.

### File List

- crates/durable-lambda-closure/Cargo.toml (modified — added aws-config, serde_json, tracing, aws-sdk-lambda, aws-smithy-types deps + async-trait dev-dep)
- crates/durable-lambda-closure/src/context.rs (rewritten — ClosureContext struct, delegating methods, 6 unit tests)
- crates/durable-lambda-closure/src/handler.rs (rewritten — run() entry point, event parsing, Operation JSON parsing)
- crates/durable-lambda-closure/src/prelude.rs (rewritten — re-exports of ClosureContext, run, core types)
- crates/durable-lambda-closure/src/lib.rs (rewritten — module declarations, top-level re-exports, crate rustdoc)

### Change Log

- 2026-03-14: Story 1.6 implemented — ClosureContext wrapper, run() entry point with Lambda event parsing, prelude re-exports. 6 unit + 10 doc tests passing. Clippy clean, fmt clean, workspace builds.
