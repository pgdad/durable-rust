# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

An idiomatic Rust SDK for AWS Lambda Durable Execution, providing full feature parity with the official AWS Python Durable Lambda SDK. Supports 8 core operations: Step, Wait, Callback, Invoke, Parallel, Map, Child Context, and Logging.

## Build & Test Commands

```bash
cargo build --workspace              # Build everything
cargo test --workspace               # Run all tests
cargo fmt --all --check              # Check formatting (CI enforces)
cargo clippy --workspace -- -D warnings  # Lint (warnings are errors in CI)

# Specific test suites
cargo test -p durable-lambda-core    # Core library tests
cargo test -p durable-lambda-testing # Testing utilities
cargo test -p e2e-tests              # End-to-end workflow tests
cargo test -p parity-tests           # Cross-approach behavioral parity
cargo test -p durable-lambda-compliance  # Python-Rust compliance

# Single test
cargo test -p e2e-tests test_name
```

No AWS credentials needed for any tests — all use `MockDurableContext`.

## Architecture

### Workspace Layout

Six crates in a strict one-way dependency graph:

```
durable-lambda-closure ─┐
durable-lambda-macro  ──┤
durable-lambda-trait  ──┼── durable-lambda-core
durable-lambda-builder ─┤
durable-lambda-testing ─┘
```

- **`durable-lambda-core`** — All operation logic (`src/operations/*.rs`), replay engine, types, errors, `DurableBackend` trait. This is where all behavior lives.
- **`durable-lambda-{closure,trait,builder}`** — Thin wrappers providing ergonomic API surfaces. Each has a `prelude` module re-exporting identical type sets. Never duplicate operation logic here.
- **`durable-lambda-macro`** — `#[durable_execution]` proc-macro for zero-boilerplate handler registration.
- **`durable-lambda-testing`** — `MockDurableContext` builder + assertion helpers for credential-free testing.

### Key Internals

- **Replay Engine** (`core/src/replay.rs`): HashMap<String, Operation> keyed by operation ID. Starts in Replaying mode if completed ops exist; transitions to Executing once all completed ops are visited.
- **Operation ID Generation** (`core/src/operation_id.rs`): `blake2b("{counter}")` for root, `blake2b("{parent_id}-{counter}")` for children. 64 hex chars. Must match Python SDK exactly — divergence breaks replay.
- **`DurableBackend` trait** (`core/src/backend.rs`): The sole I/O boundary. `RealBackend` calls AWS; `MockBackend` records calls. Never call AWS APIs outside this trait.
- **Checkpoint protocol**: Every operation sends START then SUCCEED/FAIL. Parallel/map/child_context use `OperationType::Context` with `sub_type` discriminator.
- **`DurableContextOps` trait** (`core/src/ops_trait.rs`): Single trait defining all context methods. Implemented by `DurableContext`. Wrapper contexts (`ClosureContext`, `TraitContext`, `BuilderContext`) delegate to `DurableContext`. To add or change a context method, edit `ops_trait.rs` + `context.rs` only.

## Critical Rules

### Type System Requirements
- Types flowing through `ctx.step()`, `ctx.parallel()`, `ctx.map()`, `ctx.child_context()` must implement `Serialize + DeserializeOwned` — including user error types in `Result<T, E>`.
- Step results need explicit type annotations: `let result: Result<T, E> = ctx.step(...)` — compiler can't infer T/E from closure alone.
- `?` on step results has two levels: outer unwraps `DurableError`, inner unwraps the user `Result<T, E>`.

### Closure Patterns
- Clone event data before moving into step closures: `ctx.step("name", || { let e = event.clone(); async move { ... } })`.
- Step closures must be `FnOnce`. Parallel/map closures must be `Send + 'static` (they use `tokio::spawn`).
- Parallel/map branches receive owned `DurableContext` via `create_child_context()`, not references.

### Determinism
- **Never** put non-deterministic code (`Utc::now()`, `rand::random()`, `Uuid::new_v4()`) outside durable operations. Must be inside `ctx.step()` so results are checkpointed.
- Operation sequence is position-based, not name-based. Reordering operations between deployments breaks in-flight replay.

### Testing Patterns
- `MockDurableContext::build()` returns `(DurableContext, CheckpointRecorder, OperationRecorder)`.
- Replay tests: `assert_no_checkpoints(&calls).await;` — verifies no AWS calls.
- Execute tests: `assert_operations(&ops, &["step:validate", "step:charge"]).await;` — verifies operation sequence.
- Replay mode produces no operation records in `OperationRecorder`.

### Code Style
- All public items need rustdoc with `# Examples` section. Use `no_run` for examples needing AWS context.
- No `unwrap()` in library code; acceptable in tests/examples only.
- Module doc comments (`//!`) at top of every file.
- All dependency versions in workspace `[workspace.dependencies]`, never in individual crate Cargo.toml.
- `#[non_exhaustive]` on public enums.
- Commit format: `type: description` (feat, fix, test, docs, refactor, chore).

### New Features (Phases 5-8)
- **Step timeout**: `StepOptions::new().timeout_seconds(u64)` — wraps closure in tokio::time::timeout
- **Conditional retry**: `StepOptions::new().retry_if(|e: &E| ...)` — predicate checked before consuming retry budget
- **Batch checkpoint**: `ctx.enable_batch_mode()` — multiple sequential steps share a single checkpoint call
- **Saga / compensation**: `ctx.step_with_compensation("name", forward_fn, compensate_fn)` — registers durable rollback
- **Proc-macro validation**: `#[durable_execution]` validates second param is `DurableContext` and return is `Result<_, DurableError>` at compile time
- **Builder configuration**: `.with_tracing(subscriber)` and `.with_error_handler(fn)` on `DurableHandlerBuilder`
