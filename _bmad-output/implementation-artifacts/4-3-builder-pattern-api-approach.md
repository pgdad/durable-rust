# Story 4.3: Builder-Pattern API Approach

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to construct durable Lambda handlers using a builder pattern,
So that I can configure complex handlers with explicit, step-by-step construction and rich configuration options.

## Acceptance Criteria

1. **Given** the durable-lambda-builder crate **When** I construct a handler using the builder pattern **Then** I can configure the handler step-by-step before building and running it (FR35) **And** the built handler has access to all 8 core operations

2. **Given** the builder-pattern approach **When** I call the builder's run method **Then** lambda_runtime is wired up internally with DurableContext creation **And** I never need to interact with lambda_runtime directly

3. **Given** the durable-lambda-builder crate **When** I import `use durable_lambda_builder::prelude::*` **Then** I have access to the builder types, context wrapper, and all core types

4. **Given** the crate structure **When** I examine durable-lambda-builder/src/ **Then** it contains lib.rs (re-exports only), handler.rs (builder-pattern handler construction), context.rs (builder-specific context wrapper), and prelude.rs **And** it depends only on durable-lambda-core (NFR8)

5. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --workspace` **Then** all tests pass including new builder approach tests **And** all doc tests compile

## Tasks / Subtasks

- [ ] Task 1: Implement `DurableHandlerBuilder` in `handler.rs` (AC: #1, #2)
  - [ ] 1.1: `pub struct DurableHandlerBuilder<F, Fut>` with `handler: F` field — stores the user's handler closure
  - [ ] 1.2: `pub fn handler<F, Fut>(f: F) -> DurableHandlerBuilder<F, Fut>` — constructor function that starts the builder
  - [ ] 1.3: `pub async fn run(self) -> Result<(), lambda_runtime::Error>` — consumes builder, wires up AWS config, Lambda client, RealBackend, lambda_runtime
  - [ ] 1.4: Reuse event parsing helpers from `durable_lambda_core::event` module (if created by story 4-1) or duplicate from closure crate's handler.rs
  - [ ] 1.5: Rustdoc with `# Examples` showing builder construction and `run()` usage

- [ ] Task 2: Implement `BuilderContext` wrapper in `context.rs` (AC: #1, #3)
  - [ ] 2.1: `pub struct BuilderContext { inner: DurableContext }` — same thin wrapper pattern as ClosureContext
  - [ ] 2.2: All 8 durable operation methods delegating to `self.inner.*()` (step, step_with_options, wait, create_callback, callback_result, invoke, parallel, child_context, map)
  - [ ] 2.3: All 8 log methods delegating to `self.inner.log*()`
  - [ ] 2.4: Query methods: `execution_mode()`, `is_replaying()`, `arn()`, `checkpoint_token()`
  - [ ] 2.5: `pub(crate) fn new(ctx: DurableContext) -> Self` constructor
  - [ ] 2.6: Rustdoc with `# Examples` on all public methods

- [ ] Task 3: Set up `lib.rs` and `prelude.rs` re-exports (AC: #3, #4)
  - [ ] 3.1: lib.rs re-exports: `BuilderContext`, `DurableHandlerBuilder`, `handler` (constructor function)
  - [ ] 3.2: prelude.rs re-exports: `BuilderContext`, `DurableHandlerBuilder`, `handler`, `DurableError`, and all core types (BatchItem, BatchItemStatus, BatchResult, CallbackHandle, CallbackOptions, CheckpointResult, CompletionReason, ExecutionMode, MapOptions, ParallelOptions, StepOptions)

- [ ] Task 4: Write tests (AC: #1, #2, #5)
  - [ ] 4.1: Test builder construction and type correctness
  - [ ] 4.2: Test BuilderContext delegation — step, log, execution_mode, arn, checkpoint_token
  - [ ] 4.3: Test builder `run()` function type signature
  - [ ] 4.4: All doc tests compile via `cargo test --doc`

- [ ] Task 5: Verify all checks pass (AC: #5)
  - [ ] 5.1: `cargo test --workspace` — all tests pass
  - [ ] 5.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [ ] 5.3: `cargo fmt --check` — formatting passes

## Dev Notes

### Pattern: Follow the Closure Crate Exactly

The builder crate mirrors the closure crate's structure with one difference — the user API style:

| Component | Closure Crate | Builder Crate |
|-----------|--------------|---------------|
| Context wrapper | `ClosureContext` | `BuilderContext` |
| Handler style | `run(closure).await` | `handler(closure).run().await` |
| Entry point | `run()` function | `handler()` → builder → `.run()` |
| User code | Direct function/closure | Same, but via builder |

### Builder API Design

```rust
use durable_lambda_builder::prelude::*;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(|event: serde_json::Value, mut ctx: BuilderContext| async move {
        let result: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await?;
        Ok(serde_json::json!({"result": result.unwrap()}))
    })
    .run()
    .await
}
```

The builder pattern here is intentionally minimal for MVP — `handler()` creates the builder, `.run()` executes it. Future enhancements could add configuration methods like `.with_tracing()`, `.with_middleware()`, etc.

### DurableHandlerBuilder Design

```rust
pub struct DurableHandlerBuilder<F, Fut>
where
    F: Fn(serde_json::Value, BuilderContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    handler: F,
    _phantom: std::marker::PhantomData<Fut>,
}

pub fn handler<F, Fut>(f: F) -> DurableHandlerBuilder<F, Fut>
where
    F: Fn(serde_json::Value, BuilderContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    DurableHandlerBuilder {
        handler: f,
        _phantom: std::marker::PhantomData,
    }
}

impl<F, Fut> DurableHandlerBuilder<F, Fut>
where
    F: Fn(serde_json::Value, BuilderContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    pub async fn run(self) -> Result<(), lambda_runtime::Error> {
        // Same as closure crate's run() but with BuilderContext
    }
}
```

### BuilderContext Delegation — Copy ClosureContext

Same pattern as trait crate: copy `ClosureContext`, rename to `BuilderContext`, update imports.

### run() Implementation — Copy closure crate

The `run()` method on `DurableHandlerBuilder` is nearly identical to the closure crate's `run()`:
- Same AWS config + Lambda client + RealBackend setup
- Same lambda_runtime registration
- Creates `BuilderContext::new(durable_ctx)` instead of `ClosureContext::new(durable_ctx)`
- Calls `(self.handler)(user_event, builder_ctx).await`

### Cargo.toml Dependencies

Same as closure crate (minus async-trait since no trait involved):
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
```

### What Exists vs What Needs to Be Added

**Already exists:**
- `crates/durable-lambda-builder/src/lib.rs` — stub with module declarations
- `crates/durable-lambda-builder/src/handler.rs` — stub
- `crates/durable-lambda-builder/src/context.rs` — stub
- `crates/durable-lambda-builder/src/prelude.rs` — stub
- Closure crate — complete reference implementation

**Needs to be added:**
- `DurableHandlerBuilder` struct + `handler()` constructor + `.run()` method in handler.rs
- `BuilderContext` wrapper in context.rs (copy from closure)
- Re-exports in lib.rs and prelude.rs
- Cargo.toml dependencies
- Tests

### File Structure Notes

- `crates/durable-lambda-builder/src/lib.rs` — re-exports
- `crates/durable-lambda-builder/src/handler.rs` — builder struct + handler() + run()
- `crates/durable-lambda-builder/src/context.rs` — BuilderContext wrapper
- `crates/durable-lambda-builder/src/prelude.rs` — user-facing re-exports
- `crates/durable-lambda-builder/Cargo.toml` — add dependencies

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.3 — acceptance criteria, FR35]
- [Source: _bmad-output/planning-artifacts/architecture.md — builder-pattern approach]
- [Source: crates/durable-lambda-closure/src/ — complete reference implementation]
- [Source: crates/durable-lambda-builder/src/ — stubs to fill]

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
