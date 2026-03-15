# Story 4.2: Trait-Based API Approach

Status: ready-for-dev

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

- [ ] Task 1: Define the `DurableHandler` trait in `handler.rs` (AC: #1, #2)
  - [ ] 1.1: `#[async_trait] pub trait DurableHandler: Send + Sync + 'static` with `async fn handle(&self, event: serde_json::Value, ctx: TraitContext) -> Result<serde_json::Value, DurableError>`
  - [ ] 1.2: `pub async fn run<H: DurableHandler>(handler: H) -> Result<(), lambda_runtime::Error>` — wires up AWS config, Lambda client, RealBackend, lambda_runtime, creates TraitContext from DurableContext, calls handler.handle()
  - [ ] 1.3: Reuse event parsing helpers from `durable_lambda_core::event` module (if created by story 4-1) or duplicate from closure crate's handler.rs
  - [ ] 1.4: Rustdoc with `# Examples` showing trait implementation and `run()` usage

- [ ] Task 2: Implement `TraitContext` wrapper in `context.rs` (AC: #1, #3)
  - [ ] 2.1: `pub struct TraitContext { inner: DurableContext }` — same thin wrapper pattern as ClosureContext
  - [ ] 2.2: All 8 durable operation methods delegating to `self.inner.*()` (step, step_with_options, wait, create_callback, callback_result, invoke, parallel, child_context, map)
  - [ ] 2.3: All 8 log methods delegating to `self.inner.log*()`
  - [ ] 2.4: Query methods: `execution_mode()`, `is_replaying()`, `arn()`, `checkpoint_token()`
  - [ ] 2.5: `pub(crate) fn new(ctx: DurableContext) -> Self` constructor
  - [ ] 2.6: Rustdoc with `# Examples` on all public methods

- [ ] Task 3: Set up `lib.rs` and `prelude.rs` re-exports (AC: #3, #4)
  - [ ] 3.1: lib.rs re-exports: `TraitContext`, `DurableHandler`, `run`
  - [ ] 3.2: prelude.rs re-exports: `TraitContext`, `DurableHandler`, `run`, `DurableError`, and all core types (BatchItem, BatchItemStatus, BatchResult, CallbackHandle, CallbackOptions, CheckpointResult, CompletionReason, ExecutionMode, MapOptions, ParallelOptions, StepOptions)

- [ ] Task 4: Write tests (AC: #1, #2, #5)
  - [ ] 4.1: Test that a struct implementing DurableHandler compiles and handler is callable
  - [ ] 4.2: Test TraitContext delegation — step, log, execution_mode, arn, checkpoint_token
  - [ ] 4.3: Test `run()` function type signature accepts DurableHandler implementors
  - [ ] 4.4: All doc tests compile via `cargo test --doc`

- [ ] Task 5: Verify all checks pass (AC: #5)
  - [ ] 5.1: `cargo test --workspace` — all tests pass
  - [ ] 5.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [ ] 5.3: `cargo fmt --check` — formatting passes

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

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
