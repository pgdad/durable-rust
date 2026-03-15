# Story 4.2: Trait-Based API Approach

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to define durable Lambda handlers by implementing a DurableHandler trait,
So that I can use a structured, object-oriented approach with explicit method signatures for my handlers.

## Acceptance Criteria

1. **Given** the durable-lambda-trait crate **When** I implement the DurableHandler trait on my struct **Then** I can define my handler logic in the trait's method (FR33) **And** I receive a context with access to all 8 core operations

2. **Given** the trait-based approach **When** I call `durable_lambda_trait::run(MyHandler).await` **Then** lambda_runtime is wired up internally with DurableContext creation **And** I never need to interact with lambda_runtime directly

3. **Given** the durable-lambda-trait crate **When** I import `use durable_lambda_trait::prelude::*` **Then** I have access to the DurableHandler trait, context wrapper, and all core types

4. **Given** the crate structure **When** I examine durable-lambda-trait/src/ **Then** it contains lib.rs (re-exports only), handler.rs (DurableHandler trait definition + run function), context.rs (trait-specific context wrapper), and prelude.rs **And** it depends only on durable-lambda-core (NFR8)

5. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --workspace` **Then** all tests pass including new trait approach tests **And** all doc tests compile

## Tasks / Subtasks

- [x] Task 1: Define the `DurableHandler` trait in `handler.rs` (AC: #1, #2)
  - [x] 1.1: `#[async_trait] pub trait DurableHandler: Send + Sync + 'static` with `async fn handle(&self, event: serde_json::Value, ctx: TraitContext) -> Result<serde_json::Value, DurableError>`
  - [x] 1.2: `pub async fn run<H: DurableHandler>(handler: H) -> Result<(), lambda_runtime::Error>` — wires up AWS config, Lambda client, RealBackend, lambda_runtime, creates TraitContext from DurableContext, calls handler.handle()
  - [x] 1.3: Reuse event parsing helpers from `durable_lambda_core::event` module (if created by story 4-1) or duplicate from closure crate's handler.rs
  - [x] 1.4: Rustdoc with `# Examples` showing trait implementation and `run()` usage

- [x] Task 2: Implement `TraitContext` wrapper in `context.rs` (AC: #1, #3)
  - [x] 2.1: `pub struct TraitContext { inner: DurableContext }` — same thin wrapper pattern as ClosureContext
  - [x] 2.2: All 8 durable operation methods delegating to `self.inner.*()` (step, step_with_options, wait, create_callback, callback_result, invoke, parallel, child_context, map)
  - [x] 2.3: All 8 log methods delegating to `self.inner.log*()`
  - [x] 2.4: Query methods: `execution_mode()`, `is_replaying()`, `arn()`, `checkpoint_token()`
  - [x] 2.5: `pub(crate) fn new(ctx: DurableContext) -> Self` constructor
  - [x] 2.6: Rustdoc with `# Examples` on all public methods

- [x] Task 3: Set up `lib.rs` and `prelude.rs` re-exports (AC: #3, #4)
  - [x] 3.1: lib.rs re-exports: `TraitContext`, `DurableHandler`, `run`
  - [x] 3.2: prelude.rs re-exports: `TraitContext`, `DurableHandler`, `run`, `DurableError`, and all core types (BatchItem, BatchItemStatus, BatchResult, CallbackHandle, CallbackOptions, CheckpointResult, CompletionReason, ExecutionMode, MapOptions, ParallelOptions, StepOptions)

- [x] Task 4: Write tests (AC: #1, #2, #5)
  - [x] 4.1: Test that a struct implementing DurableHandler compiles and handler is callable
  - [x] 4.2: Test TraitContext delegation — step, log, execution_mode, arn, checkpoint_token
  - [x] 4.3: Test `run()` function type signature accepts DurableHandler implementors
  - [x] 4.4: All doc tests compile via `cargo test --doc`

- [x] Task 5: Verify all checks pass (AC: #5)
  - [x] 5.1: `cargo test --workspace` — all tests pass
  - [x] 5.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [x] 5.3: `cargo fmt --check` — formatting passes

### Review Follow-ups (AI)

- [ ] [AI-Review][MEDIUM] Extract duplicated event parsing code (`parse_operations`, `parse_operation_type`, `parse_operation_status`, `extract_user_event` ~112 lines) into a shared module in `durable-lambda-core` (e.g., `durable_lambda_core::event`). Currently duplicated identically across `durable-lambda-closure/src/handler.rs` and `durable-lambda-trait/src/handler.rs`, and likely also `durable-lambda-builder/src/handler.rs`.
- [ ] [AI-Review][LOW] Consider adding a doc comment to `prelude.rs` listing `CheckpointResult` in the module-level rustdoc (it is re-exported but not mentioned in the summary list).

## Dev Notes

### Pattern: Follow the Closure Crate Exactly

The trait crate mirrors the closure crate's structure with one difference — the user API style:

| Component | Closure Crate | Trait Crate |
|-----------|--------------|-------------|
| Context wrapper | `ClosureContext` | `TraitContext` |
| Handler style | `Fn(Value, ClosureContext) -> Fut` | `DurableHandler` trait with `handle()` method |
| Entry point | `run(closure).await` | `run(handler_impl).await` |
| User code | Closure/function | Struct implementing trait |

The `run()` function and context wrapper are essentially identical to the closure crate. The only difference is how the user provides their handler logic.

### DurableHandler Trait Design

```rust
#[async_trait::async_trait]
pub trait DurableHandler: Send + Sync + 'static {
    async fn handle(
        &self,
        event: serde_json::Value,
        ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError>;
}
```

User usage:
```rust
use durable_lambda_trait::prelude::*;

struct OrderProcessor;

#[async_trait::async_trait]
impl DurableHandler for OrderProcessor {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        let result: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await?;
        Ok(serde_json::json!({"result": result.unwrap()}))
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(OrderProcessor).await
}
```

### TraitContext Delegation — Copy ClosureContext

`TraitContext` is structurally identical to `ClosureContext`:
- Same `inner: DurableContext` field
- Same `pub(crate) fn new(ctx: DurableContext)` constructor
- Same delegation methods for all operations
- Same rustdoc patterns

Copy `ClosureContext` from `crates/durable-lambda-closure/src/context.rs`, rename to `TraitContext`, update doc examples to use trait-style imports.

### run() Function — Copy closure crate's handler.rs

The `run()` function is nearly identical to the closure crate's `run()`, except:
- Takes `H: DurableHandler` instead of `F: Fn(...)`
- Creates `TraitContext::new(durable_ctx)` instead of `ClosureContext::new(durable_ctx)`
- Calls `handler.handle(user_event, trait_ctx).await` instead of `handler(user_event, closure_ctx).await`

Copy the closure crate's `handler.rs`, rename context type, update handler invocation.

### Cargo.toml Dependencies

The trait crate needs the same dependencies as the closure crate:
```toml
[dependencies]
durable-lambda-core = { path = "../durable-lambda-core" }
lambda_runtime = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
aws-config = { workspace = true }
aws-sdk-lambda = { workspace = true }
aws-smithy-types = { workspace = true }
async-trait = { workspace = true }
```

Note the `async-trait` dependency — needed for the `#[async_trait]` attribute on `DurableHandler`.

### What Exists vs What Needs to Be Added

**Already exists:**
- `crates/durable-lambda-trait/src/lib.rs` — stub with `pub mod context; pub mod handler; pub mod prelude;`
- `crates/durable-lambda-trait/src/handler.rs` — stub with header comment
- `crates/durable-lambda-trait/src/context.rs` — stub with header comment
- `crates/durable-lambda-trait/src/prelude.rs` — stub with header comment
- Closure crate — complete reference implementation to copy from

**Needs to be added:**
- `DurableHandler` trait definition in handler.rs
- `run()` function in handler.rs (copy from closure, adapt)
- `TraitContext` wrapper in context.rs (copy from closure, rename)
- Re-exports in lib.rs and prelude.rs
- Cargo.toml dependencies (add lambda_runtime, aws-config, aws-sdk-lambda, etc.)
- Tests

### File Structure Notes

- `crates/durable-lambda-trait/src/lib.rs` — re-exports
- `crates/durable-lambda-trait/src/handler.rs` — DurableHandler trait + run()
- `crates/durable-lambda-trait/src/context.rs` — TraitContext wrapper
- `crates/durable-lambda-trait/src/prelude.rs` — user-facing re-exports
- `crates/durable-lambda-trait/Cargo.toml` — add dependencies

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.2 — acceptance criteria, FR33]
- [Source: _bmad-output/planning-artifacts/architecture.md — trait-based approach, handler.rs, context.rs, prelude.rs]
- [Source: crates/durable-lambda-closure/src/ — complete reference implementation]
- [Source: crates/durable-lambda-trait/src/ — stubs to fill]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6 (1M context)

### Debug Log References

No debug issues encountered.

### Completion Notes List

- Implemented `DurableHandler` trait with `#[async_trait]` attribute, `Send + Sync + 'static` bounds, and `handle()` method signature matching the story spec exactly.
- Implemented `run()` function that wraps handler in `Arc` for shared access across Lambda invocations, wires up AWS config, Lambda client, RealBackend, and creates TraitContext from DurableContext.
- Event parsing helpers duplicated from closure crate's handler.rs (no shared `durable_lambda_core::event` module exists from story 4-1).
- `TraitContext` wrapper mirrors `ClosureContext` exactly: same `inner: DurableContext` field, same 9 durable operation methods (step, step_with_options, wait, create_callback, callback_result, invoke, parallel, child_context, map), 8 log methods, 4 query methods.
- lib.rs re-exports `TraitContext`, `DurableHandler`, `run`. prelude.rs re-exports all user-facing types.
- Added `async-trait`, `serde_json`, `aws-config`, `aws-sdk-lambda`, `aws-smithy-types` dependencies to Cargo.toml.
- 10 unit tests: DurableHandler trait compilation/callability, TraitContext delegation (step, step_with_options, child_context, execution_mode/replaying, arn, checkpoint_token, log methods), run() type signature acceptance.
- 26 doc tests all compile successfully.
- Full workspace: `cargo test --workspace` passes, `cargo clippy --workspace -- -D warnings` clean, `cargo fmt --check` clean.

### Change Log

- 2026-03-14: Implemented Story 4.2 — DurableHandler trait, TraitContext wrapper, run() function, re-exports, and tests.

### File List

- crates/durable-lambda-trait/Cargo.toml (modified — added dependencies)
- crates/durable-lambda-trait/src/handler.rs (modified — DurableHandler trait + run() + event parsing)
- crates/durable-lambda-trait/src/context.rs (modified — TraitContext wrapper + tests)
- crates/durable-lambda-trait/src/lib.rs (modified — re-exports + crate-level rustdoc)
- crates/durable-lambda-trait/src/prelude.rs (modified — user-facing re-exports)
