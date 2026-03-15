---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
lastStep: 8
status: 'complete'
completedAt: '2026-03-13'
inputDocuments:
  - '_bmad-output/planning-artifacts/prd.md'
  - '_bmad-output/planning-artifacts/product-brief-durable-rust-2026-03-13.md'
workflowType: 'architecture'
project_name: 'durable-rust'
user_name: 'Esa'
date: '2026-03-13'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**
50 FRs across 10 categories:
- Core Replay Engine (FR1-FR7): History loading, replay/execute mode detection, cursor-based advancement, serde checkpointing — the foundational state machine that all operations build on
- Durable Operations (FR8-FR28): 8 operation types (steps, waits, callbacks, invoke, parallel, map, child contexts, logging) — each with distinct checkpoint semantics and replay behavior
- API Approaches (FR32-FR36): 4 independent crates exposing identical operation sets — architecture must ensure behavioral consistency without code duplication
- Testing (FR37-FR40): MockDurableContext and compliance suite — requires abstraction boundary between operation interface and AWS backend
- Documentation (FR41-FR44): Rustdoc with compilable examples, migration guide — doc examples are effectively integration tests
- Deployment & Error Handling (FR45-FR50): Container images, lambda_runtime integration, typed error enum

**Non-Functional Requirements:**
20 NFRs across 5 categories:
- Performance: <1ms per-operation overhead, single history API call, <32MB memory, <100ms cold start
- Reliability: Zero divergence from Python SDK, atomic checkpoints, transient failure retries
- Maintainability: Clean crate dependency boundaries (approach → core only), 100% test coverage
- Developer Experience: Zero SDK-caused borrow/ownership errors, AI-friendly patterns, actionable compiler messages
- Compatibility: Latest stable Rust + aws-sdk-lambda + lambda_runtime + provided.al2023 containers

**Scale & Complexity:**

- Primary domain: Rust SDK / Library
- Complexity level: Medium
- Estimated architectural components: 6 crates + compliance suite + examples + CI/CD

### Technical Constraints & Dependencies

- **tokio** — mandated by aws-sdk-lambda and lambda_runtime; single async runtime, no choice
- **serde** — sole serialization mechanism; all checkpointed types must implement Serialize + DeserializeOwned
- **thiserror** — error enum strategy; DurableError wraps AWS SDK and serde errors
- **tracing** — logging framework; replay-safe dedup layer built on top
- **aws-sdk-lambda** — 9 durable execution API operations are the only AWS interface; no direct HTTP
- **lambda_runtime** — handler registration; container deployment with provided.al2023
- **Send + 'static** — required for parallel/map branch closures via tokio::spawn; drives owned-data patterns
- **No MSRV** — latest stable only simplifies feature gate decisions

### Cross-Cutting Concerns Identified

- **Replay determinism** — every operation must behave identically in replay vs. execute mode; non-deterministic code outside operations is a user footgun that documentation/examples must address
- **Serialization boundaries** — serde bounds propagate through every public API; type constraints affect ergonomics across all 4 approaches
- **Error propagation** — DurableError must flow through core → approach crates → user handlers without losing context; step errors are also serialized/checkpointed
- **AWS API isolation** — core wraps the 9 AWS operations; approach crates never touch AWS directly; MockDurableContext replaces this layer in tests
- **Async throughout** — all operations are async; the replay engine, AWS calls, and user closures all run on tokio
- **Ownership model** — "pit of success" patterns must hide Send + 'static + owned data requirements behind ergonomic APIs; this is the primary DX challenge

## Starter Template Evaluation

### Primary Technology Domain

Rust SDK / Library — cargo workspace with 6 crates, targeting AWS Lambda durable execution APIs.

### Starter Options Considered

No pre-built starter template exists for this project type. AWS Lambda durable execution SDKs exist only for Python, JavaScript/TypeScript, and Java (preview). This is a greenfield Rust implementation against the same AWS APIs, using the [Python SDK](https://github.com/aws/aws-durable-execution-sdk-python) as the behavioral reference.

The initialization approach is a hand-crafted cargo workspace — the standard approach for multi-crate Rust libraries.

### Selected Approach: Custom Cargo Workspace

**Rationale:** No starter template applies. This is a library SDK, not an application. The workspace structure is already defined in the PRD, and the dependency set is fully determined by the AWS Lambda ecosystem.

**Initialization Command:**

```bash
cargo init --name durable-lambda durable-rust
# Then restructure into workspace with crates/ directory
```

**Architectural Decisions Established:**

**Language & Runtime:**
- Rust, latest stable toolchain (no MSRV)
- Async runtime: tokio (required by aws-sdk-lambda + lambda_runtime)
- Minimum Rust version: 1.82.0 (lambda_runtime 1.0 requirement)

**Workspace Layout:**
- Virtual manifest at root (no root src/)
- `crates/` directory with flat layout:
  - `crates/durable-lambda-core` — replay engine, AWS API wrapper, types, errors
  - `crates/durable-lambda-macro` — proc-macro approach (`proc-macro = true`, syn, quote, proc-macro2)
  - `crates/durable-lambda-trait` — trait-based approach
  - `crates/durable-lambda-closure` — closure-native approach
  - `crates/durable-lambda-builder` — builder-pattern approach
  - `crates/durable-lambda-testing` — MockDurableContext, test utilities

**Key Dependencies:**
- `aws-sdk-lambda` ~1.50+ — AWS Lambda API client (durable execution operations)
- `aws-config` with `behavior-version-latest` feature
- `lambda_runtime` 1.0 — Lambda handler registration
- `tokio` with `full` feature — async runtime
- `serde` + `serde_json` — serialization (all checkpointed types)
- `thiserror` — error enum derives
- `tracing` + `tracing-subscriber` — structured logging
- `syn` + `quote` + `proc-macro2` — proc-macro crate only

**Testing & Coverage:**
- `cargo-llvm-cov` for coverage (cross-platform, more accurate than tarpaulin — important since team develops on macOS)
- `tokio::test` for async test harness
- MockDurableContext in `durable-lambda-testing` for credential-free testing

**Code Organization:**
- Workspace-level `Cargo.toml` with shared dependency versions via `[workspace.dependencies]`
- Each approach crate depends only on `durable-lambda-core`
- Compliance suite in `tests/compliance/` or separate `compliance/` directory
- Examples in `examples/` organized by API approach

**Reference Implementation:**
- [aws/aws-durable-execution-sdk-python](https://github.com/aws/aws-durable-execution-sdk-python) — behavioral reference for all 8 operations
- [aws/aws-durable-execution-sdk-python-testing](https://github.com/aws/aws-durable-execution-sdk-python-testing) — testing patterns reference

**Note:** Project initialization using `cargo init` + workspace restructuring should be the first implementation story.

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
- Core engine abstraction boundary (DurableBackend trait)
- History replay strategy (positional Vec with cursor)
- Checkpoint serialization format (match Python SDK — JSON)
- API approach-to-core interface pattern (thin wrappers)
- Handler registration pattern (approach-specific entry point)
- Error handling strategy (serde-only bounds for user errors, flat enum with context for SDK errors)

**Important Decisions (Shape Architecture):**
- Child context namespacing (match Python SDK)
- Proc-macro design (attribute macro)
- CI pipeline stages (comprehensive)
- Container base image (official AWS Lambda al2023)

**Deferred Decisions (Post-MVP):**
- API approach consolidation (after team evaluation in Phase 2)
- crates.io publishing strategy (Phase 2)
- Performance benchmarking tooling (measured via AWS Cost Explorer)

### Core Engine Architecture

**Testability Abstraction: `DurableBackend` trait**
- Decision: Single trait covering all 9 AWS durable execution API operations
- Rationale: Core owns the replay state machine; the trait is the I/O boundary only. `RealBackend` calls AWS, `MockBackend` returns pre-loaded data. Replay logic is implemented once in core, never duplicated in mocks.
- Affects: `durable-lambda-core`, `durable-lambda-testing`

**History Replay: Positional `Vec<HistoryEntry>` with cursor**
- Decision: Eager-load all history into a `Vec`, advance a `usize` cursor as each durable operation is encountered
- Rationale: Matches Python SDK's positional/sequential replay model. O(1) per operation, cache-friendly, simple implementation.
- Affects: `durable-lambda-core` replay engine

**Checkpoint Serialization: Match Python SDK format (JSON)**
- Decision: Use `serde_json` for checkpoint serialization, matching the Python SDK's format exactly
- Rationale: Behavioral compliance with Python SDK is a hard requirement. JSON is human-readable and debuggable. Eliminates any checkpoint format incompatibility risk.
- Affects: All crates (serialization boundaries)

**Child Context Namespacing: Match Python SDK**
- Decision: Follow whatever namespacing strategy the Python SDK uses for child context checkpoint isolation
- Rationale: Behavioral compliance. Exact approach to be determined by studying Python SDK source during implementation.
- Affects: `durable-lambda-core` (child context implementation)

### API Abstraction Pattern

**Approach-to-Core Interface: Thin wrappers**
- Decision: Each approach crate provides ergonomic sugar over core's `DurableContext`. Users never touch `DurableContext` directly.
- Rationale: "Pit of success" — users only see the approach crate's clean API. Core internals stay hidden. All 4 approaches share identical core behavior.
- Affects: All 4 approach crates

**Handler Registration: Approach-specific entry point**
- Decision: Each approach crate provides a `run()` function that wires up `lambda_runtime` + `DurableContext` internally
- Rationale: One function call, everything wired correctly. Junior devs don't need to understand lambda_runtime plumbing. e.g., `durable_lambda_closure::run(my_handler).await`
- Affects: All 4 approach crates

**Proc-Macro Design: Attribute macro**
- Decision: `#[durable_execution]` attribute on an async fn, generates lambda_runtime registration + DurableContext setup
- Rationale: Most idiomatic Rust pattern (mirrors `#[tokio::main]`). AI coding tools generate attribute macros most reliably.
- Affects: `durable-lambda-macro`

### Error Strategy

**User Step Errors: Serde-only bounds**
- Decision: Step error type `E` requires only `Serialize + DeserializeOwned`, not `std::error::Error`
- Rationale: Fewest trait bounds for users to satisfy. Simple enums and strings work without additional derives. `thiserror` enums with serde derives are a natural pattern but not required.
- Affects: All approach crates (step operation signatures)

**SDK Error Type: Flat enum with context**
- Decision: `DurableError` is a flat enum where each variant carries rich context data (expected/actual values, positions, operation names)
- Rationale: Simple `match` arms for handling, rich diagnostic information for debugging replay issues — the hardest bugs in durable functions.
- Affects: `durable-lambda-core`, propagated through all crates

### Infrastructure & Deployment

**CI/CD Platform: GitHub Actions**
- Decision: GitHub Actions for all CI/CD
- Rationale: Standard for Rust open-source projects. Strong cargo toolchain support. Free for public repos (Phase 2 open-source).
- Affects: Repository configuration

**CI Pipeline: Comprehensive checks**
- Decision: Every PR runs: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo build --workspace`, `cargo test --workspace`, `cargo test --doc`, `cargo llvm-cov` with coverage threshold, example container builds
- Rationale: Matches 100% coverage target. Catches formatting, lint, build, test, doc, and coverage issues before merge.
- Affects: `.github/workflows/`

**Container Base Image: Official AWS Lambda al2023**
- Decision: Use `public.ecr.aws/lambda/provided:al2023` as base image
- Rationale: Official AWS-supported image, includes Lambda runtime interface. Simplest path, well-documented. Rust cold starts are already sub-100ms — no need to optimize image further.
- Affects: Dockerfile in examples

### Decision Impact Analysis

**Implementation Sequence:**
1. Workspace initialization + `DurableBackend` trait definition (foundation)
2. Core replay engine with `Vec<HistoryEntry>` cursor (central component)
3. `DurableError` enum with context variants (used by everything)
4. Core operations (step, wait, callback, invoke, parallel, map, child context, log)
5. `MockDurableContext` with `MockBackend` (enables testing all subsequent work)
6. Approach crates (thin wrappers + entry points, can be parallelized)
7. Proc-macro crate (attribute macro, depends on core API being stable)
8. CI pipeline + container build (can start early, refined as crates mature)

**Cross-Component Dependencies:**
- `DurableBackend` trait shape determines both `RealBackend` and `MockBackend` implementations
- Checkpoint JSON format must be consistent across core, all approach crates, and testing crate
- `DurableError` variants must cover all failure modes across core and approach crates
- Entry point functions in approach crates depend on core's `DurableContext` constructor and `DurableBackend` trait

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**Critical Conflict Points Identified:**
6 areas where AI agents could make different choices, all now resolved with explicit conventions.

### Naming & Signature Patterns

**Operation Method Signatures:**
All 8 durable operations follow a consistent parameter ordering across all approach crates:
1. Name (string) — always first
2. Options/config — second, when applicable
3. Closure/payload — always last

```rust
ctx.step("validate_order", || async { ... })
ctx.step_with_options("charge", StepOptions::new().retries(3), || async { ... })
ctx.parallel("fan_out", branches)
ctx.map("process_items", items, || async { ... })
```

**Rust Language Conventions (enforced by compiler/clippy):**
- Functions/methods: `snake_case`
- Types/traits/enums: `CamelCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Crate names: `kebab-case` in Cargo.toml, `snake_case` in code

### Structure Patterns

**Module Organization (all crates follow this pattern):**
- `lib.rs` is **re-exports only** — no logic, just `pub use` and `pub mod`
- One file per operation in an `operations/` module
- Types and errors get their own files — never inline in `lib.rs`
- Approach crates follow the same pattern but with fewer files

**Core crate canonical structure:**
```
crates/durable-lambda-core/src/
  lib.rs          — public re-exports only
  context.rs      — DurableContext struct
  backend.rs      — DurableBackend trait + RealBackend
  replay.rs       — replay engine (cursor, history)
  operations/     — one file per operation
    mod.rs
    step.rs
    wait.rs
    callback.rs
    invoke.rs
    parallel.rs
    map.rs
    child_context.rs
    log.rs
  types.rs        — shared types (HistoryEntry, ExecutionMode, CheckpointResult, BatchResult)
  error.rs        — DurableError enum
```

**Test Organization:**
- Unit tests: Co-located via `#[cfg(test)] mod tests { ... }` at bottom of each file
- Integration tests: `crates/<crate-name>/tests/` directory, one file per major feature
- Compliance tests: Top-level `compliance/` directory, tests Python-vs-Rust behavior
- Doc tests: Inline in rustdoc on every public item
- Test naming: `test_{operation}_{behavior}_{condition}` e.g., `test_step_returns_cached_result_during_replay`

### Error Patterns

**DurableError Construction:**
- Always use constructor methods, never raw struct construction
- Constructor methods keep internal field names as implementation details
- All variants wrapping underlying errors use `#[from]` or `source()` for error chain propagation

```rust
// Correct:
DurableError::replay_mismatch(expected, got, position)
DurableError::checkpoint_failed(operation_name, source_error)
DurableError::serialization(type_name, source_error)

// Incorrect — exposes internal fields:
DurableError::ReplayMismatch { expected: ..., got: ..., position: ... }
```

### Documentation Patterns

**Rustdoc Convention:**
- Summary line in imperative mood ("Execute a named step", not "Executes")
- Always document replay vs execution behavior for durable operations
- `# Examples` section is mandatory on every public item
- `# Errors` section on anything returning `Result`
- No `# Panics` in public API — SDK should never panic

```rust
/// Execute a named step with checkpointing.
///
/// During execution mode, runs the closure and checkpoints the result.
/// During replay mode, returns the previously checkpointed result
/// without executing the closure.
///
/// # Arguments
///
/// * `name` - Human-readable step name, used as checkpoint key
/// * `f` - Closure to execute (skipped during replay)
///
/// # Examples
///
/// ```rust
/// ctx.step("validate", || async {
///     Ok(validated_order)
/// }).await?;
/// ```
///
/// # Errors
///
/// Returns [`DurableError::CheckpointFailed`] if ...
```

### Public API Surface

**Re-export Pattern:**
- Approach crates re-export everything users need — users never add `durable-lambda-core` to their Cargo.toml
- Each approach crate provides a `prelude` module with all user-facing types
- Single import pattern: `use durable_lambda_closure::prelude::*;`
- Core types (DurableError, StepOptions, BatchResult, etc.) are re-exported through approach crates

```rust
// User's Cargo.toml — one dependency:
// durable-lambda-closure = "0.1"

// User's code — one import:
use durable_lambda_closure::prelude::*;
```

### Enforcement Guidelines

**All AI Agents MUST:**
- Follow the parameter ordering convention (name, options, closure) for all operation signatures
- Place logic in dedicated module files, never in `lib.rs`
- Use `DurableError` constructor methods, never raw struct construction
- Include `# Examples` and `# Errors` rustdoc sections on every public item
- Document replay vs execution behavior for every durable operation
- Re-export user-facing types through the approach crate's `prelude` module

**Pattern Enforcement:**
- `cargo clippy -- -D warnings` catches naming and style violations
- `cargo test --doc` verifies all doc examples compile and pass
- `cargo llvm-cov` with threshold enforces test coverage
- PR review checklist should verify pattern compliance

## Project Structure & Boundaries

### Complete Project Directory Structure

```
durable-rust/
├── Cargo.toml                          — virtual manifest (workspace definition + [workspace.dependencies])
├── Cargo.lock
├── README.md
├── LICENSE
├── .gitignore
├── .github/
│   └── workflows/
│       ├── ci.yml                      — PR checks (fmt, clippy, build, test, doc-test, coverage)
│       └── release.yml                 — container build + publish (Phase 2: crates.io)
├── crates/
│   ├── durable-lambda-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  — pub re-exports only
│   │       ├── context.rs              — DurableContext struct (replay state machine)
│   │       ├── backend.rs              — DurableBackend trait + RealBackend (AWS calls)
│   │       ├── replay.rs               — replay engine (Vec<HistoryEntry>, usize cursor)
│   │       ├── operations/
│   │       │   ├── mod.rs
│   │       │   ├── step.rs             — FR8-FR11: checkpointed work with retries
│   │       │   ├── wait.rs             — FR12-FR13: time-based suspension
│   │       │   ├── callback.rs         — FR14-FR16: external signal coordination
│   │       │   ├── invoke.rs           — FR17-FR18: durable Lambda-to-Lambda
│   │       │   ├── parallel.rs         — FR19-FR22: fan-out with completion criteria
│   │       │   ├── map.rs              — FR23-FR25: parallel collection processing
│   │       │   ├── child_context.rs    — FR26-FR28: isolated subflows
│   │       │   └── log.rs              — FR29-FR31: replay-safe structured logging
│   │       ├── types.rs                — HistoryEntry, ExecutionMode, CheckpointResult, BatchResult
│   │       └── error.rs                — DurableError enum with constructor methods
│   ├── durable-lambda-macro/
│   │   ├── Cargo.toml                  — proc-macro = true, depends on syn/quote/proc-macro2
│   │   └── src/
│   │       ├── lib.rs                  — #[durable_execution] attribute macro entry point
│   │       └── expand.rs               — macro expansion logic (code generation)
│   ├── durable-lambda-trait/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  — pub re-exports + prelude module
│   │       ├── handler.rs              — DurableHandler trait definition
│   │       ├── context.rs              — trait-specific context wrapper
│   │       └── prelude.rs              — user-facing re-exports
│   ├── durable-lambda-closure/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  — pub re-exports + prelude module
│   │       ├── handler.rs              — closure-based handler registration
│   │       ├── context.rs              — closure-specific context wrapper
│   │       └── prelude.rs              — user-facing re-exports
│   ├── durable-lambda-builder/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  — pub re-exports + prelude module
│   │       ├── handler.rs              — builder-pattern handler construction
│   │       ├── context.rs              — builder-specific context wrapper
│   │       └── prelude.rs              — user-facing re-exports
│   └── durable-lambda-testing/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  — pub re-exports
│           ├── mock_backend.rs         — MockBackend (implements DurableBackend)
│           ├── mock_context.rs         — MockDurableContext (pre-loaded step results)
│           ├── assertions.rs           — test assertion helpers (operation sequence verification)
│           └── prelude.rs              — user-facing test re-exports
├── compliance/
│   ├── README.md                       — compliance suite documentation
│   ├── python/                         — Python reference implementations
│   │   ├── requirements.txt
│   │   └── workflows/
│   │       ├── order_processing.py     — multi-step order workflow
│   │       ├── parallel_fanout.py      — parallel branch workflow
│   │       └── callback_approval.py    — callback-based approval workflow
│   ├── rust/                           — Rust implementations matching Python
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── order_processing.rs
│   │       ├── parallel_fanout.rs
│   │       └── callback_approval.rs
│   └── tests/
│       └── compare_outputs.rs          — FR40: compare Python vs Rust outputs
├── examples/
│   ├── Dockerfile                      — FR45-FR46: Lambda container build template
│   ├── closure-style/
│   │   └── src/
│   │       ├── basic_steps.rs          — simple step with checkpoint
│   │       ├── step_retries.rs         — retry configuration
│   │       ├── typed_errors.rs         — Result<T, E> with serializable error
│   │       ├── waits.rs                — time-based suspension
│   │       ├── callbacks.rs            — external signal coordination
│   │       ├── invoke.rs               — Lambda-to-Lambda invocation
│   │       ├── parallel.rs             — fan-out with branches
│   │       ├── map.rs                  — collection processing
│   │       ├── child_contexts.rs       — isolated subflows
│   │       ├── replay_safe_logging.rs  — deduplicated logging
│   │       └── combined_workflow.rs    — end-to-end multi-operation workflow
│   ├── macro-style/
│   │   └── src/
│   │       └── ...                     — same examples as closure-style
│   ├── trait-style/
│   │   └── src/
│   │       └── ...                     — same examples as closure-style
│   └── builder-style/
│       └── src/
│           └── ...                     — same examples as closure-style
└── docs/
    └── migration-guide.md              — FR44: Python-to-Rust conceptual mapping
```

### Architectural Boundaries

**Crate Dependency Boundary (strictly enforced):**
```
durable-lambda-macro    ─┐
durable-lambda-trait    ─┤
durable-lambda-closure  ─┼──► durable-lambda-core ──► aws-sdk-lambda
durable-lambda-builder  ─┘                         ──► lambda_runtime
                                                   ──► tokio

durable-lambda-testing  ──► durable-lambda-core (no AWS dependency at runtime)
```

- Approach crates depend **only** on `durable-lambda-core` — never on each other
- Testing crate depends on core but does **not** depend on `aws-sdk-lambda` at runtime (MockBackend replaces it)
- No circular dependencies — core knows nothing about approach crates

**I/O Boundary (DurableBackend trait):**
```
┌─────────────────────────────────┐
│  DurableContext (replay engine) │ ← owns state machine, cursor, history
├─────────────────────────────────┤
│  DurableBackend trait           │ ← I/O boundary
├────────────┬────────────────────┤
│ RealBackend│   MockBackend      │
│ (AWS calls)│ (pre-loaded data)  │
└────────────┴────────────────────┘
```

**User API Boundary (approach crates):**
```
User code ──► approach crate (prelude) ──► DurableContext (core)
             thin wrapper + run()          replay engine + operations
```

Users interact only with the approach crate's public API. Core types are re-exported through the prelude.

### Requirements to Structure Mapping

**FR Category Mapping:**

| FR Category | Primary Location | Related Files |
|---|---|---|
| Core Replay Engine (FR1-FR7) | `crates/durable-lambda-core/src/replay.rs`, `context.rs`, `backend.rs` | `types.rs` |
| Steps (FR8-FR11) | `crates/durable-lambda-core/src/operations/step.rs` | Each approach crate's `context.rs` |
| Waits (FR12-FR13) | `crates/durable-lambda-core/src/operations/wait.rs` | Each approach crate's `context.rs` |
| Callbacks (FR14-FR16) | `crates/durable-lambda-core/src/operations/callback.rs` | Each approach crate's `context.rs` |
| Invoke (FR17-FR18) | `crates/durable-lambda-core/src/operations/invoke.rs` | Each approach crate's `context.rs` |
| Parallel (FR19-FR22) | `crates/durable-lambda-core/src/operations/parallel.rs` | Each approach crate's `context.rs` |
| Map (FR23-FR25) | `crates/durable-lambda-core/src/operations/map.rs` | Each approach crate's `context.rs` |
| Child Contexts (FR26-FR28) | `crates/durable-lambda-core/src/operations/child_context.rs` | Each approach crate's `context.rs` |
| Replay-Safe Logging (FR29-FR31) | `crates/durable-lambda-core/src/operations/log.rs` | tracing integration |
| API Approaches (FR32-FR36) | `crates/durable-lambda-{macro,trait,closure,builder}/` | One crate per approach |
| Testing (FR37-FR39) | `crates/durable-lambda-testing/` | `mock_backend.rs`, `mock_context.rs` |
| Compliance (FR40) | `compliance/` | Python + Rust workflows + comparison tests |
| Documentation (FR41-FR44) | Inline rustdoc + `examples/` + `docs/migration-guide.md` | All public items |
| Deployment (FR45-FR47) | `examples/Dockerfile` | lambda_runtime integration in approach crates |
| Error Handling (FR48-FR50) | `crates/durable-lambda-core/src/error.rs` | Re-exported through all approach crates |

**Cross-Cutting Concerns Mapping:**

| Concern | Locations |
|---|---|
| Serialization (serde bounds) | `types.rs`, all operation files, approach crate wrappers |
| Error propagation | `error.rs` → all operation files → approach crates → user code |
| Async/tokio | All operation files, backend.rs, approach crate entry points |
| Replay determinism | `replay.rs`, `context.rs`, all operation files |
| Send + 'static bounds | `parallel.rs`, `map.rs`, approach crate context wrappers |

### Integration Points

**Internal Communication:**
- Approach crates call `DurableContext` methods via thin wrappers
- `DurableContext` calls `DurableBackend` trait methods for all AWS I/O
- Operations read/advance the shared cursor in `replay.rs`
- Child contexts create new `DurableContext` instances sharing `Arc<dyn DurableBackend>`

**External Integrations:**
- `aws-sdk-lambda`: 9 durable execution API operations (the only external integration)
- `lambda_runtime`: Handler registration and Lambda lifecycle management
- `tracing`: Structured logging output (user's tracing subscriber configuration)

**Data Flow:**
```
Lambda invocation
  → lambda_runtime receives event
  → approach crate run() creates DurableContext with RealBackend
  → RealBackend loads history (single API call)
  → DurableContext replays operations from cursor
  → New operations execute and checkpoint via RealBackend
  → Handler returns result to lambda_runtime
```

### Development Workflow Integration

**Local Development:**
- `cargo build --workspace` — builds all 6 crates
- `cargo test --workspace` — runs all unit + integration tests (no AWS needed)
- `cargo test --doc` — verifies all doc examples compile
- `cargo llvm-cov --workspace` — coverage report

**CI Pipeline:**
- PR: fmt → clippy → build → test → doc-test → coverage → container build
- Main: all PR checks + publish container images

**Deployment:**
- Build: `docker build -f examples/Dockerfile -t my-durable-lambda .`
- Push: `docker push` to ECR
- Deploy: Update Lambda function configuration to point at new container image

## Architecture Validation Results

### Coherence Validation

**Decision Compatibility:** All technology choices are mutually compatible. tokio is required by both aws-sdk-lambda and lambda_runtime — no async runtime conflict. serde_json for checkpoints aligns with Python SDK compliance. thiserror + flat enum with constructors work together cleanly. cargo-llvm-cov works on macOS (team's platform).

**Pattern Consistency:** No contradictions found. Parameter ordering (name, options, closure) is consistent across all operation signatures. Module organization (lib.rs = re-exports only) applies uniformly. Error construction (constructor methods) is consistent with flat enum decision. Documentation pattern is enforceable via doc tests.

**Structure Alignment:** Project tree supports all decisions. Virtual manifest + crates/ layout matches the 6-crate workspace. operations/ module with one file per operation matches the 8 operations requirement. compliance/ directory supports Python/Rust comparison. examples/ organized by API approach with identical example sets.

### Requirements Coverage Validation

**Functional Requirements:** All 50 FRs (FR1-FR50) have explicit architectural coverage mapped to specific files and directories in the project structure.

**Non-Functional Requirements:** All 20 NFRs (NFR1-NFR20) are architecturally supported:
- Performance (NFR1-4): Vec cursor O(1), eager loading, Rust baseline memory, container on al2023
- Reliability (NFR5-7): JSON format match + compliance suite, DurableBackend atomicity, RealBackend retries
- Maintainability (NFR8-10): Strict crate dependency boundaries, operations module pattern, cargo-llvm-cov in CI
- Developer Experience (NFR11-14): Thin wrappers + pit of success, flat enum with context, prelude pattern, small crates
- Compatibility (NFR15-20): Latest stable Rust, aws-sdk-lambda, lambda_runtime, al2023, AWS API only, tracing

### Implementation Readiness Validation

**Decision Completeness:** All critical and important decisions are documented with rationale and affected components. Implementation patterns are comprehensive with concrete code examples.

**Structure Completeness:** Complete directory tree with every file annotated. All crate boundaries defined with dependency diagram. Integration points specified.

**Pattern Completeness:** All 6 identified conflict points resolved. Naming, structure, error, documentation, and API surface patterns all specified with examples.

### Gap Analysis Results

**No critical gaps.** Three minor implementation-detail gaps (StepOptions struct, retry strategy specifics, BatchResult internals) — all resolved by studying the Python SDK source during implementation, consistent with the "match Python SDK" decisions.

### Architecture Completeness Checklist

- [x] Project context thoroughly analyzed (50 FRs, 20 NFRs)
- [x] Scale and complexity assessed (Medium, SDK/Library)
- [x] Technical constraints identified (tokio, serde, aws-sdk-lambda, etc.)
- [x] Cross-cutting concerns mapped (replay determinism, serialization, errors, async, ownership)
- [x] Critical decisions documented with versions
- [x] Technology stack fully specified
- [x] Integration patterns defined (DurableBackend trait, thin wrappers, entry points)
- [x] Naming conventions established
- [x] Structure patterns defined
- [x] Error patterns specified
- [x] Documentation patterns documented
- [x] Complete directory structure defined
- [x] Component boundaries established
- [x] Integration points mapped
- [x] Requirements to structure mapping complete

### Architecture Readiness Assessment

**Overall Status:** READY FOR IMPLEMENTATION

**Confidence Level:** High

**Key Strengths:**
- Every FR and NFR has explicit architectural coverage
- DurableBackend trait cleanly separates I/O from replay logic
- "Match Python SDK" strategy eliminates behavioral ambiguity
- Thin wrapper pattern ensures all 4 approaches share identical core behavior
- Comprehensive CI pipeline enforces quality standards automatically

**Areas for Future Enhancement:**
- StepOptions, BatchResult, and retry strategy details (resolve during implementation)
- Child context namespacing (resolve by studying Python SDK source)
- Phase 2: crates.io publishing metadata, API approach consolidation

**Implementation Handoff — First Priority:**
Initialize the cargo workspace with virtual manifest, create all 6 crate skeletons, define DurableBackend trait, and set up CI pipeline.
