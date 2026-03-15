# Story 1.1: Project Workspace Initialization

Status: done

## Story

As a developer,
I want a properly structured cargo workspace with all 6 crate skeletons and CI pipeline,
So that I have the foundation to build and test SDK components incrementally.

## Acceptance Criteria

1. **Given** a fresh clone of the durable-rust repository **When** I run `cargo build --workspace` **Then** the workspace compiles successfully with all 6 crates: durable-lambda-core, durable-lambda-macro, durable-lambda-trait, durable-lambda-closure, durable-lambda-builder, durable-lambda-testing **And** the root Cargo.toml is a virtual manifest with `[workspace.dependencies]` for shared dependency versions

2. **Given** the workspace is initialized **When** I examine the crate dependency graph **Then** each approach crate (macro, trait, closure, builder) depends only on durable-lambda-core **And** durable-lambda-testing depends only on durable-lambda-core **And** no circular dependencies exist

3. **Given** the workspace is initialized **When** I examine each crate's src/ directory **Then** each crate has a lib.rs containing only `pub use` and `pub mod` statements (no logic) **And** the core crate has the canonical module structure: context.rs, backend.rs, replay.rs, operations/, types.rs, error.rs

4. **Given** the workspace is initialized **When** I push a commit **Then** GitHub Actions CI runs: cargo fmt --check, cargo clippy -- -D warnings, cargo build --workspace, cargo test --workspace

## Tasks / Subtasks

- [x] Task 1: Create root virtual manifest Cargo.toml (AC: #1)
  - [x] 1.1: Create root Cargo.toml with `[workspace]` definition listing all 6 crate members under `crates/`
  - [x] 1.2: Add `[workspace.dependencies]` with all shared dependency versions (see Dev Notes for exact versions)
  - [x] 1.3: Create .gitignore with Rust defaults (target/, Cargo.lock for libraries — BUT keep Cargo.lock since this is a workspace with binaries in examples)
- [x] Task 2: Create durable-lambda-core crate skeleton (AC: #1, #3)
  - [x] 2.1: Create `crates/durable-lambda-core/Cargo.toml` with dependencies from workspace
  - [x] 2.2: Create `crates/durable-lambda-core/src/lib.rs` with `pub mod` declarations only
  - [x] 2.3: Create stub module files: `context.rs`, `backend.rs`, `replay.rs`, `types.rs`, `error.rs`
  - [x] 2.4: Create `crates/durable-lambda-core/src/operations/mod.rs` with `pub mod` stubs for: step, wait, callback, invoke, parallel, map, child_context, log
  - [x] 2.5: Create stub files for each operation: `step.rs`, `wait.rs`, `callback.rs`, `invoke.rs`, `parallel.rs`, `map.rs`, `child_context.rs`, `log.rs`
- [x] Task 3: Create durable-lambda-macro crate skeleton (AC: #1, #2, #3)
  - [x] 3.1: Create `crates/durable-lambda-macro/Cargo.toml` with `proc-macro = true`, depending on core + syn/quote/proc-macro2
  - [x] 3.2: Create `crates/durable-lambda-macro/src/lib.rs` with proc-macro entry point stub
  - [x] 3.3: Create `crates/durable-lambda-macro/src/expand.rs` stub
- [x] Task 4: Create durable-lambda-trait crate skeleton (AC: #1, #2, #3)
  - [x] 4.1: Create `crates/durable-lambda-trait/Cargo.toml` depending only on durable-lambda-core
  - [x] 4.2: Create `crates/durable-lambda-trait/src/lib.rs` with pub mod declarations
  - [x] 4.3: Create stub files: `handler.rs`, `context.rs`, `prelude.rs`
- [x] Task 5: Create durable-lambda-closure crate skeleton (AC: #1, #2, #3)
  - [x] 5.1: Create `crates/durable-lambda-closure/Cargo.toml` depending only on durable-lambda-core
  - [x] 5.2: Create `crates/durable-lambda-closure/src/lib.rs` with pub mod declarations
  - [x] 5.3: Create stub files: `handler.rs`, `context.rs`, `prelude.rs`
- [x] Task 6: Create durable-lambda-builder crate skeleton (AC: #1, #2, #3)
  - [x] 6.1: Create `crates/durable-lambda-builder/Cargo.toml` depending only on durable-lambda-core
  - [x] 6.2: Create `crates/durable-lambda-builder/src/lib.rs` with pub mod declarations
  - [x] 6.3: Create stub files: `handler.rs`, `context.rs`, `prelude.rs`
- [x] Task 7: Create durable-lambda-testing crate skeleton (AC: #1, #2, #3)
  - [x] 7.1: Create `crates/durable-lambda-testing/Cargo.toml` depending only on durable-lambda-core
  - [x] 7.2: Create `crates/durable-lambda-testing/src/lib.rs` with pub mod declarations
  - [x] 7.3: Create stub files: `mock_backend.rs`, `mock_context.rs`, `assertions.rs`, `prelude.rs`
- [x] Task 8: Create GitHub Actions CI pipeline (AC: #4)
  - [x] 8.1: Create `.github/workflows/ci.yml` with PR trigger
  - [x] 8.2: Add jobs: cargo fmt --check, cargo clippy -- -D warnings, cargo build --workspace, cargo test --workspace
  - [x] 8.3: Use latest stable Rust toolchain, cache cargo registry/target
- [x] Task 9: Verify workspace builds and passes all checks (AC: #1, #2, #3, #4)
  - [x] 9.1: Run `cargo build --workspace` — must succeed
  - [x] 9.2: Run `cargo test --workspace` — must succeed (no tests yet, but no failures)
  - [x] 9.3: Run `cargo fmt --check` — must pass
  - [x] 9.4: Run `cargo clippy -- -D warnings` — must pass
  - [x] 9.5: Verify no circular dependencies via `cargo tree`

### Review Follow-ups (AI)

- [ ] [AI-Review][Med] CI cache key uses only `Cargo.lock` hash — add `Cargo.toml` as fallback hash input for robustness on clean runs [.github/workflows/ci.yml:29]
- [ ] [AI-Review][Low] Approach crate lib.rs files (trait, closure, builder) missing `//!` crate-level doc comments for discoverability [crates/durable-lambda-*/src/lib.rs]
- [ ] [AI-Review][Med] No git commits — all workspace files are untracked. Commit initial workspace to enable CI and git history auditing
- [ ] [AI-Review][Med] `backoff_delay` jitter is a no-op: `capped / 2 + capped / 2 == capped`. Introduce actual randomization or remove jitter comment [crates/durable-lambda-core/src/backend.rs:116-117]
- [ ] [AI-Review][Low] `operation_id.rs` not in Story 1.1 File List — update File List or move to Story 1.2/1.3 ownership [crates/durable-lambda-core/src/operation_id.rs]
- [ ] [AI-Review][Low] Extra workspace deps (`blake2`, `hex`, `async-trait`, `aws-smithy-types`) not documented in Story 1.1 — track in the story that added them [Cargo.toml]

## Dev Notes

### Dependency Versions (as of 2026-03-13)

Use these exact versions in `[workspace.dependencies]`:

```toml
[workspace.dependencies]
aws-sdk-lambda = "1.118.0"
aws-config = { version = "1.8.15", features = ["behavior-version-latest"] }
lambda_runtime = "1.1.1"
tokio = { version = "1.50.0", features = ["full"] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
thiserror = "2.0.18"
tracing = "0.1.44"
tracing-subscriber = "0.3.23"
syn = { version = "2.0.117", features = ["full"] }
quote = "1.0.45"
proc-macro2 = "1.0.106"
```

### Critical Architecture Constraints

- **Virtual manifest**: Root Cargo.toml has NO `[package]` section — only `[workspace]` and `[workspace.dependencies]`
- **lib.rs = re-exports only**: Every crate's lib.rs contains ONLY `pub mod` and `pub use` statements. Zero logic.
- **Crate dependency boundary**: Approach crates depend ONLY on durable-lambda-core. Testing crate depends ONLY on durable-lambda-core. No cross-approach dependencies.
- **proc-macro crate**: durable-lambda-macro Cargo.toml must have `[lib] proc-macro = true`. It uses syn/quote/proc-macro2 but does NOT depend on tokio/aws-sdk at build time.
- **Module organization**: Core crate uses `operations/` subdirectory with `mod.rs` re-exporting all 8 operation modules.

### Stub File Content Pattern

Each stub module file should contain a minimal comment explaining its future purpose. Do NOT add placeholder structs/traits/functions — these will be added in subsequent stories. Example:

```rust
//! Step operation — checkpointed work with retries.
//!
//! Implements FR8-FR11: named steps, retry configuration, typed errors, replay skip.
```

For lib.rs files, use `pub mod` declarations:

```rust
pub mod context;
pub mod backend;
// etc.
```

### Core Crate Dependencies

The core crate needs: aws-sdk-lambda, aws-config, tokio, serde, serde_json, thiserror, tracing.

Approach crates (trait, closure, builder) need: durable-lambda-core (path dependency), lambda_runtime, tokio, serde.

Macro crate needs: durable-lambda-core (NOT as proc-macro dependency — it needs syn, quote, proc-macro2 only in the proc-macro crate itself. The core dependency is for re-exports in the generated code, referenced via `::durable_lambda_core::` paths).

Testing crate needs: durable-lambda-core (path dependency), serde, serde_json, tokio.

### CI Pipeline Requirements

- Trigger on: push to main, pull_request to main
- Rust toolchain: stable (latest)
- Cache: actions/cache with ~/.cargo/registry, ~/.cargo/git, target/
- Steps run sequentially: fmt check -> clippy -> build -> test
- Use `actions/checkout@v4` and `dtolnay/rust-toolchain@stable`

### thiserror 2.x Note

thiserror 2.0 is a major version bump from 1.x. Key differences:
- No separate `thiserror-impl` crate needed
- `#[error(...)]` attribute syntax is the same
- `#[from]` still works for automatic From implementations
- The API is largely compatible but ensure you use `thiserror = "2"` not `thiserror = "1"`

### Project Structure Notes

Target directory layout after this story:

```
durable-rust/
├── Cargo.toml                          # virtual manifest
├── Cargo.lock
├── .gitignore
├── .github/
│   └── workflows/
│       └── ci.yml
└── crates/
    ├── durable-lambda-core/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── context.rs
    │       ├── backend.rs
    │       ├── replay.rs
    │       ├── types.rs
    │       ├── error.rs
    │       └── operations/
    │           ├── mod.rs
    │           ├── step.rs
    │           ├── wait.rs
    │           ├── callback.rs
    │           ├── invoke.rs
    │           ├── parallel.rs
    │           ├── map.rs
    │           ├── child_context.rs
    │           └── log.rs
    ├── durable-lambda-macro/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       └── expand.rs
    ├── durable-lambda-trait/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── handler.rs
    │       ├── context.rs
    │       └── prelude.rs
    ├── durable-lambda-closure/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── handler.rs
    │       ├── context.rs
    │       └── prelude.rs
    ├── durable-lambda-builder/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── handler.rs
    │       ├── context.rs
    │       └── prelude.rs
    └── durable-lambda-testing/
        ├── Cargo.toml
        └── src/
            ├── lib.rs
            ├── mock_backend.rs
            ├── mock_context.rs
            ├── assertions.rs
            └── prelude.rs
```

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.1]
- [Source: _bmad-output/planning-artifacts/prd.md#Technical Architecture]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- All 6 crate skeletons created with correct dependency boundaries
- Root Cargo.toml is a virtual manifest with [workspace.dependencies] for all shared deps
- Core crate has canonical module structure: context.rs, backend.rs, replay.rs, types.rs, error.rs, operations/ with 8 operation files
- Macro crate correctly configured as proc-macro with syn/quote/proc-macro2 only (no AWS deps)
- Approach crates (trait, closure, builder) depend only on durable-lambda-core + lambda_runtime + tokio + serde
- Testing crate depends only on durable-lambda-core + serde + serde_json + tokio
- All stub files contain doc comments explaining future purpose, no placeholder code
- GitHub Actions CI pipeline created with fmt, clippy, build, test steps
- All verification checks passed: cargo build, cargo test, cargo fmt --check, cargo clippy, cargo tree

### File List

- Cargo.toml (new)
- Cargo.lock (new, auto-generated)
- .gitignore (new)
- .github/workflows/ci.yml (new)
- crates/durable-lambda-core/Cargo.toml (new)
- crates/durable-lambda-core/src/lib.rs (new)
- crates/durable-lambda-core/src/context.rs (new)
- crates/durable-lambda-core/src/backend.rs (new)
- crates/durable-lambda-core/src/replay.rs (new)
- crates/durable-lambda-core/src/types.rs (new)
- crates/durable-lambda-core/src/error.rs (new)
- crates/durable-lambda-core/src/operations/mod.rs (new)
- crates/durable-lambda-core/src/operations/step.rs (new)
- crates/durable-lambda-core/src/operations/wait.rs (new)
- crates/durable-lambda-core/src/operations/callback.rs (new)
- crates/durable-lambda-core/src/operations/invoke.rs (new)
- crates/durable-lambda-core/src/operations/parallel.rs (new)
- crates/durable-lambda-core/src/operations/map.rs (new)
- crates/durable-lambda-core/src/operations/child_context.rs (new)
- crates/durable-lambda-core/src/operations/log.rs (new)
- crates/durable-lambda-macro/Cargo.toml (new)
- crates/durable-lambda-macro/src/lib.rs (new)
- crates/durable-lambda-macro/src/expand.rs (new)
- crates/durable-lambda-trait/Cargo.toml (new)
- crates/durable-lambda-trait/src/lib.rs (new)
- crates/durable-lambda-trait/src/handler.rs (new)
- crates/durable-lambda-trait/src/context.rs (new)
- crates/durable-lambda-trait/src/prelude.rs (new)
- crates/durable-lambda-closure/Cargo.toml (new)
- crates/durable-lambda-closure/src/lib.rs (new)
- crates/durable-lambda-closure/src/handler.rs (new)
- crates/durable-lambda-closure/src/context.rs (new)
- crates/durable-lambda-closure/src/prelude.rs (new)
- crates/durable-lambda-builder/Cargo.toml (new)
- crates/durable-lambda-builder/src/lib.rs (new)
- crates/durable-lambda-builder/src/handler.rs (new)
- crates/durable-lambda-builder/src/context.rs (new)
- crates/durable-lambda-builder/src/prelude.rs (new)
- crates/durable-lambda-testing/Cargo.toml (new)
- crates/durable-lambda-testing/src/lib.rs (new)
- crates/durable-lambda-testing/src/mock_backend.rs (new)
- crates/durable-lambda-testing/src/mock_context.rs (new)
- crates/durable-lambda-testing/src/assertions.rs (new)
- crates/durable-lambda-testing/src/prelude.rs (new)

### Change Log

- 2026-03-13: Story 1.1 implemented — cargo workspace with 6 crate skeletons, CI pipeline, all checks passing
- 2026-03-14: Code review — 0 Critical, 2 Medium, 2 Low issues found. 4 action items created. All ACs verified implemented. Status remains done.
