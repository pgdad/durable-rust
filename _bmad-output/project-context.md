---
project_name: 'durable-rust'
user_name: 'Esa'
date: '2026-03-16'
sections_completed:
  ['technology_stack', 'language_rules', 'framework_rules', 'testing_rules', 'quality_rules', 'workflow_rules', 'anti_patterns']
status: 'complete'
rule_count: 38
optimized_for_llm: true
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

---

## Technology Stack & Versions

- **Rust** edition 2021, stable toolchain (1.82.0+), no MSRV policy
- **aws-sdk-lambda** 1.118.0 — Lambda API client (durable execution operations)
- **aws-config** 1.8.15 (feature: `behavior-version-latest`)
- **lambda_runtime** 1.1.1 — handler registration
- **tokio** 1.50.0 (feature: `full`) — sole async runtime
- **serde** 1.0.228 + **serde_json** 1.0.149 — all checkpointed types must impl `Serialize + DeserializeOwned`
- **thiserror** 2.0.18 — error enum derives
- **tracing** 0.1.44 + **tracing-subscriber** 0.3.23 — structured logging
- **blake2** 0.10 + **hex** 0.4 — deterministic operation ID generation
- **async-trait** 0.1 — trait object async methods (`DurableBackend`, `DurableHandler`)
- **syn** 2.0.117 + **quote** 1.0.45 + **proc-macro2** 1.0.106 — proc-macro crate only
- **aws-smithy-types** 1.4.6 — AWS SDK type utilities

### Version Constraints
- All workspace members use `[workspace.dependencies]` for version alignment — never pin versions in individual crates
- Container deployment targets `provided.al2023` base image

## Critical Implementation Rules

### Rust Language Rules

- **Serde bounds propagate everywhere** — every type that flows through `ctx.step()`, `ctx.parallel()`, `ctx.map()`, or `ctx.child_context()` must implement `Serialize + DeserializeOwned`. User error types in `Result<T, E>` must also be serializable — they get checkpointed too.
- **`Send + 'static` required for parallel/map closures** — these use `tokio::spawn`, so closures and their captured data must be `Send + 'static`. Use owned data (clone before move), never borrow across spawn boundaries.
- **Clone event data before moving into step closures** — `ctx.step("name", || { let event = event.clone(); async move { ... } })` is the correct pattern. Moving `event` directly will fail because the closure must be `FnOnce` but may need the event after the step.
- **Type annotations on step results are mandatory** — `let result: Result<T, E> = ctx.step(...)` — the compiler cannot infer `T` and `E` from the closure alone due to serde deserialization bounds.
- **`?` operator on step/operation results has two levels** — `ctx.step(...).await?` returns `Result<T, E>` (the user result), not `T`. The outer `?` unwraps `DurableError`; the inner `.unwrap()` or match unwraps the user result.
- **Error enum uses static constructors** — `DurableError::replay_mismatch(...)`, never raw struct construction. All variants are `#[non_exhaustive]`.
- **All public items require rustdoc** — every `pub fn`, `pub struct`, `pub enum`, and `pub trait` must have `///` doc comments with at least one `# Examples` section containing compilable code.
- **No `unwrap()` in library code** — use `?` with proper error conversion. `unwrap()` is acceptable only in tests and examples.

### Architecture Rules

- **Approach crates are thin wrappers** — `durable-lambda-closure`, `durable-lambda-trait`, `durable-lambda-builder` each wrap `DurableContext` from core. All operation logic lives in `durable-lambda-core/src/operations/`. Never duplicate operation logic in approach crates.
- **Dependency direction is strictly one-way** — approach crates depend on `durable-lambda-core` only. No cross-approach dependencies. No core depending on approach crates (except `durable-lambda-testing` as dev-dependency for integration tests).
- **Every approach crate has a `prelude` module** — re-exports all types users need from a single `use crate::prelude::*;`. All prelude modules must export identical type sets (verified by parity tests).
- **Operation ID generation must match Python SDK** — `blake2b("{counter}")` for root ops, `blake2b("{parent_id}-{counter}")` for child ops, truncated to 64 hex chars. This is the replay determinism invariant — if IDs diverge, replay breaks.
- **Checkpoint protocol: START then SUCCEED/FAIL** — every operation sends a START checkpoint, then SUCCEED or FAIL. Parallel/map/child_context use `OperationType::Context` with `sub_type` discriminator ("Parallel", "ParallelBranch", "Map", "MapItem", "Context").
- **`DurableBackend` trait is the I/O boundary** — the only abstraction point between SDK and AWS. `RealBackend` calls AWS; `MockBackend` records calls for testing. Never call AWS APIs outside this trait.
- **Replay mode returns cached results, execute mode runs closures** — during replay, step closures are NOT invoked. The replay engine returns the cached result from history. After history is exhausted, mode transitions to Executing and closures run.
- **Child contexts get isolated operation ID namespaces** — child context, parallel branches, and map items each get a child `OperationIdGenerator` scoped under the parent op ID. Same-named steps in different branches do NOT collide.
- **Parameter ordering convention: `(name, options, closure)`** — `name` always first, configuration options second (when applicable), closure/payload always last. All approach crates follow this consistently.

### Testing Rules

- **All tests use `MockDurableContext`** — no AWS credentials needed. Build with `.with_step_result()`, `.with_wait()`, `.with_callback()`, `.with_invoke()` for replay mode, or empty `.build()` for execute mode.
- **`MockDurableContext::build()` returns a 3-tuple** — `(DurableContext, CheckpointRecorder, OperationRecorder)`. Always destructure all three even if unused: `let (mut ctx, calls, ops) = ...` or `let (mut ctx, _calls, _ops) = ...`.
- **Use `#[tokio::test]` for all async tests** — never `#[test]` with manual runtime construction.
- **Replay tests must assert no checkpoints** — `assert_no_checkpoints(&calls).await;` verifies pure replay (no AWS calls made). This is the fundamental replay correctness assertion.
- **Execute-mode tests verify operation recording** — use `assert_operations(&ops, &["step:validate", "step:charge"]).await;` to verify the exact operation sequence in `"type:name"` format.
- **Test file locations:**
  - Unit tests: `#[cfg(test)] mod tests` at bottom of source files
  - Integration tests: `crates/*/tests/*.rs`
  - Cross-crate E2E tests: `tests/e2e/tests/`
  - Parity tests: `tests/parity/tests/`
  - Compliance tests: `compliance/rust/tests/`
- **Replay mode produces no operation records** — `OperationRecorder` only captures execute-mode operations (via START checkpoints). Don't assert operation sequence in replay-only tests.
- **CI runs `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, then `cargo test --workspace`** — all three must pass. Clippy warnings are errors.

### Code Quality & Style Rules

- **`cargo fmt` is the sole formatter** — no rustfmt.toml overrides, use default settings. CI enforces with `--check`.
- **Clippy with `-D warnings`** — all clippy lints are treated as errors. Fix warnings, don't suppress with `#[allow(...)]` unless there's a documented reason.
- **File naming: `snake_case.rs`** — modules match file names. Operation implementations are one file per operation in `crates/durable-lambda-core/src/operations/`.
- **Crate naming: `durable-lambda-{name}`** — hyphenated in Cargo.toml, underscored in Rust code (`durable_lambda_core`).
- **Module doc comments (`//!`) at top of every file** — describe what the module implements and which functional requirements it covers (e.g., `//! Implements FR19-FR22: concurrent branches...`).
- **Rustdoc examples use `no_run`** — examples that need AWS context or a running Lambda use `/// ```no_run`. Pure type/logic examples use `/// ````.
- **`#[non_exhaustive]` on public enums and error variants** — allows adding variants without breaking downstream code.
- **Workspace dependency management** — all shared dependency versions declared in root `[workspace.dependencies]`. Crate Cargo.toml uses `{ workspace = true }`. Never pin versions in individual crates.
- **No `pub use` of internal implementation details** — only re-export types through `lib.rs` and `prelude.rs`. Keep internal modules (`replay.rs`, `operation_id.rs`, `event.rs`) accessible but not prominently surfaced.

### Development Workflow Rules

- **Commit message format: `type: description`** — types are `feat`, `fix`, `test`, `docs`, `refactor`, `chore`. Keep the first line under 72 characters.
- **Main branch is `main`** — direct pushes for this project (no PR workflow currently enforced).
- **CI pipeline: GitHub Actions** — runs on push to `main` and PRs. Steps: fmt check, clippy, build, test (all workspace).
- **Container deployment: multi-stage Dockerfile** — `rust:1-slim` build stage, `public.ecr.aws/lambda/provided:al2023` runtime stage. Binary goes to `${LAMBDA_RUNTIME_DIR}/bootstrap`.
- **Always commit planning docs and code together** — don't leave sprint status, story files, or implementation artifacts uncommitted.

### Critical Don't-Miss Rules

- **NEVER put non-deterministic code outside durable operations** — `Utc::now()`, `rand::random()`, `Uuid::new_v4()` etc. must be inside a `ctx.step()` so the result is checkpointed and replayed deterministically. Code outside steps re-executes on every replay.
- **NEVER call AWS APIs directly** — all AWS interaction goes through `DurableBackend`. Approach crates never touch AWS. This is the testability boundary.
- **NEVER add cross-approach crate dependencies** — `durable-lambda-closure` must not depend on `durable-lambda-trait` or vice versa. All shared logic lives in `durable-lambda-core`.
- **NEVER duplicate operation implementations** — if you need to change how `step()` works, change it in `durable-lambda-core/src/operations/step.rs`. The approach crate wrappers delegate to core; they don't reimplement.
- **Step closures must be `FnOnce`** — not `Fn` or `FnMut`. Each step closure executes at most once (either during execute mode or not at all during replay).
- **Parallel/map branch closures receive owned `DurableContext`** — not `&mut`. Each branch/item gets its own context via `create_child_context()`. Don't try to share a context reference across branches.
- **Callback has two phases** — `create_callback()` returns a `CallbackHandle`, then `callback_result(&handle)` retrieves the result. These are separate operations with separate operation IDs. Don't collapse them.
- **Wait and callback SUSPEND in execute mode** — they checkpoint START and the function exits. The server re-invokes after the condition is met. In replay mode they return immediately from cached history. Tests must pre-load these operations for replay or accept the suspension.
- **`BatchResult<T>` captures individual failures** — parallel/map operations succeed even if some branches/items fail. Check `BatchItemStatus` per result item. The outer `Result` only fails if the operation infrastructure itself fails.
- **Operation sequence is position-based, not name-based** — the nth operation in code corresponds to the nth operation ID. Reordering operations between deployments breaks replay of in-flight executions.

---

## Usage Guidelines

**For AI Agents:**

- Read this file before implementing any code
- Follow ALL rules exactly as documented
- When in doubt, prefer the more restrictive option
- Update this file if new patterns emerge

**For Humans:**

- Keep this file lean and focused on agent needs
- Update when technology stack changes
- Review quarterly for outdated rules
- Remove rules that become obvious over time

Last Updated: 2026-03-16
