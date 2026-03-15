# Story 6.2: Standalone Examples Across All API Styles

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want standalone examples demonstrating every core feature in all 4 API styles,
So that I can copy-paste a working pattern for any operation in my preferred API approach.

## Acceptance Criteria

1. **Given** the examples/ directory **When** I examine its structure **Then** it contains subdirectories closure-style/, macro-style/, trait-style/, builder-style/ **And** each contains the same set of example files (FR43)

2. **Given** each API style's example directory **When** I examine the example files **Then** they cover: basic steps, step retries, typed errors, waits, callbacks, invoke, parallel, map, child contexts, replay-safe logging, and a combined end-to-end workflow

3. **Given** all standalone examples **When** I run `cargo build --examples` or the examples compile as workspace members **Then** all examples compile successfully

4. **Given** a developer new to the SDK **When** they open any example file **Then** the example is self-contained with clear comments and can be used as a starting point for a new durable Lambda handler

## Tasks / Subtasks

- [x] Task 1: Set up example directory structure (AC: #1)
  - [x] 1.1: Create `examples/macro-style/` with `Cargo.toml` and `src/` directory
  - [x] 1.2: Create `examples/trait-style/` with `Cargo.toml` and `src/` directory
  - [x] 1.3: Create `examples/builder-style/` with `Cargo.toml` and `src/` directory
  - [x] 1.4: Add all 3 new example crates to workspace members
  - [x] 1.5: Restructure existing `examples/closure-style/` to have individual example binaries (currently only has main.rs)

- [x] Task 2: Create closure-style examples (AC: #2, #4)
  - [x] 2.1: `basic_steps.rs` — simple step with checkpoint
  - [x] 2.2: `step_retries.rs` — retry configuration with StepOptions
  - [x] 2.3: `typed_errors.rs` — Result<T, E> with serializable error type
  - [x] 2.4: `waits.rs` — time-based suspension
  - [x] 2.5: `callbacks.rs` — create_callback + callback_result
  - [x] 2.6: `invoke.rs` — durable Lambda-to-Lambda invocation
  - [x] 2.7: `parallel.rs` — fan-out with multiple branches
  - [x] 2.8: `map.rs` — parallel collection processing
  - [x] 2.9: `child_contexts.rs` — isolated subflows
  - [x] 2.10: `replay_safe_logging.rs` — deduplicated structured logging
  - [x] 2.11: `combined_workflow.rs` — end-to-end multi-operation workflow

- [x] Task 3: Port examples to macro-style (AC: #2)
  - [x] 3.1: Port all 11 examples using `#[durable_execution]` attribute macro pattern
  - [x] 3.2: Each uses `DurableContext` directly (no wrapper)
  - [x] 3.3: Verify all compile

- [x] Task 4: Port examples to trait-style (AC: #2)
  - [x] 4.1: Port all 11 examples using `DurableHandler` trait implementation pattern
  - [x] 4.2: Each uses `TraitContext` wrapper
  - [x] 4.3: Verify all compile

- [x] Task 5: Port examples to builder-style (AC: #2)
  - [x] 5.1: Port all 11 examples using `handler(f).run().await` builder pattern
  - [x] 5.2: Each uses `BuilderContext` wrapper
  - [x] 5.3: Verify all compile

- [x] Task 6: Verify all checks pass (AC: #3)
  - [x] 6.1: All example crates compile as workspace members
  - [x] 6.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [x] 6.3: `cargo fmt --check` — formatting passes

## Dev Notes

### Example Binary Pattern

Each example should be a separate binary in the crate:
```toml
# examples/closure-style/Cargo.toml
[[bin]]
name = "basic-steps"
path = "src/basic_steps.rs"

[[bin]]
name = "step-retries"
path = "src/step_retries.rs"
# ... etc
```

### Handler Patterns Per API Style

**Closure-native:**
```rust
use durable_lambda_closure::prelude::*;
async fn handler(event: serde_json::Value, mut ctx: ClosureContext) -> Result<serde_json::Value, DurableError> { ... }
#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> { durable_lambda_closure::run(handler).await }
```

**Macro:**
```rust
use durable_lambda_macro::durable_execution;
use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;
#[durable_execution]
async fn handler(event: serde_json::Value, mut ctx: DurableContext) -> Result<serde_json::Value, DurableError> { ... }
```

**Trait:**
```rust
use durable_lambda_trait::prelude::*;
use async_trait::async_trait;
struct MyHandler;
#[async_trait]
impl DurableHandler for MyHandler { async fn handle(&self, event: serde_json::Value, mut ctx: TraitContext) -> Result<serde_json::Value, DurableError> { ... } }
#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> { durable_lambda_trait::run(MyHandler).await }
```

**Builder:**
```rust
use durable_lambda_builder::prelude::*;
#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(|event, mut ctx| async move { ... }).run().await
}
```

### What Exists

- `examples/closure-style/src/main.rs` — single combined example (validate → charge → confirm)
- `examples/closure-style/Cargo.toml` — closure crate dependency
- `examples/Dockerfile` — container build template

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.2 — acceptance criteria, FR43]
- [Source: _bmad-output/planning-artifacts/architecture.md — examples/ directory structure]
- [Source: _bmad-output/planning-artifacts/prd.md — example coverage table]
- [Source: examples/closure-style/src/main.rs — existing reference]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6 (1M context)

### Debug Log References

None — no debugging required.

### Completion Notes List

- Created 44 standalone example files (11 per API style x 4 styles) covering all core operations
- Restructured closure-style from single main.rs to 11 individual binary examples
- Each example is self-contained with clear doc comments explaining the demonstrated feature
- All examples compile, pass clippy with -D warnings, and pass cargo fmt
- Key pattern: parallel/map/child_context closures require `DurableContext` type annotation and boxed closures for parallel branches due to Rust's unique closure types
- Full workspace test suite passes (64 suites, 0 failures)

### Change Log

- 2026-03-15: Implemented all 6 tasks — directory structure, 44 example files across 4 API styles, all verification checks pass

### File List

- Cargo.toml (modified — added 3 new workspace members)
- examples/closure-style/Cargo.toml (modified — restructured to 11 individual binaries)
- examples/closure-style/src/main.rs (deleted — replaced by individual examples)
- examples/closure-style/src/basic_steps.rs (new)
- examples/closure-style/src/step_retries.rs (new)
- examples/closure-style/src/typed_errors.rs (new)
- examples/closure-style/src/waits.rs (new)
- examples/closure-style/src/callbacks.rs (new)
- examples/closure-style/src/invoke.rs (new)
- examples/closure-style/src/parallel.rs (new)
- examples/closure-style/src/map.rs (new)
- examples/closure-style/src/child_contexts.rs (new)
- examples/closure-style/src/replay_safe_logging.rs (new)
- examples/closure-style/src/combined_workflow.rs (new)
- examples/macro-style/Cargo.toml (new)
- examples/macro-style/src/basic_steps.rs (new)
- examples/macro-style/src/step_retries.rs (new)
- examples/macro-style/src/typed_errors.rs (new)
- examples/macro-style/src/waits.rs (new)
- examples/macro-style/src/callbacks.rs (new)
- examples/macro-style/src/invoke.rs (new)
- examples/macro-style/src/parallel.rs (new)
- examples/macro-style/src/map.rs (new)
- examples/macro-style/src/child_contexts.rs (new)
- examples/macro-style/src/replay_safe_logging.rs (new)
- examples/macro-style/src/combined_workflow.rs (new)
- examples/trait-style/Cargo.toml (new)
- examples/trait-style/src/basic_steps.rs (new)
- examples/trait-style/src/step_retries.rs (new)
- examples/trait-style/src/typed_errors.rs (new)
- examples/trait-style/src/waits.rs (new)
- examples/trait-style/src/callbacks.rs (new)
- examples/trait-style/src/invoke.rs (new)
- examples/trait-style/src/parallel.rs (new)
- examples/trait-style/src/map.rs (new)
- examples/trait-style/src/child_contexts.rs (new)
- examples/trait-style/src/replay_safe_logging.rs (new)
- examples/trait-style/src/combined_workflow.rs (new)
- examples/builder-style/Cargo.toml (new)
- examples/builder-style/src/basic_steps.rs (new)
- examples/builder-style/src/step_retries.rs (new)
- examples/builder-style/src/typed_errors.rs (new)
- examples/builder-style/src/waits.rs (new)
- examples/builder-style/src/callbacks.rs (new)
- examples/builder-style/src/invoke.rs (new)
- examples/builder-style/src/parallel.rs (new)
- examples/builder-style/src/map.rs (new)
- examples/builder-style/src/child_contexts.rs (new)
- examples/builder-style/src/replay_safe_logging.rs (new)
- examples/builder-style/src/combined_workflow.rs (new)
