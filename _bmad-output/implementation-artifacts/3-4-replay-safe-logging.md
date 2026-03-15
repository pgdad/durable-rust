# Story 3.4: Replay-Safe Logging

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to emit structured log messages that are automatically deduplicated across replays,
So that I can debug durable functions without log noise from replayed operations.

## Acceptance Criteria

1. **Given** a DurableContext in Executing mode **When** I call `ctx.log("order processed", structured_data)` **Then** the structured log message is emitted via the tracing crate (FR29, FR30) **And** the log entry includes execution context enrichment (execution ARN, operation name)

2. **Given** a DurableContext in Replaying mode **When** the replay engine is replaying completed operations **Then** log calls are suppressed — no duplicate log output appears (FR31) **And** previously emitted log messages are not re-emitted

3. **Given** the logging integration **When** I examine the implementation **Then** it uses the `tracing` crate ecosystem — no custom logging framework (NFR20) **And** the log operation integrates with the user's existing tracing subscriber configuration

4. **Given** a durable function with multiple steps and log calls **When** it replays after a suspension **Then** only log messages from newly executing operations appear **And** previously emitted log messages are not duplicated

5. **Given** the log operation **When** I examine its behavior **Then** it is NOT a checkpoint-based durable operation — it does NOT send checkpoints to AWS **And** replay suppression is purely client-side based on `is_replaying()` state

6. **Given** multiple log levels **When** I call `ctx.log_info()`, `ctx.log_debug()`, `ctx.log_warn()`, `ctx.log_error()` **Then** each emits at the corresponding tracing level **And** all are suppressed during replay

7. **Given** child contexts with logging **When** a child context emits logs **Then** the logs include parent context information for hierarchical tracing

8. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --workspace` **Then** all tests pass including new log operation tests **And** all doc tests compile

## Tasks / Subtasks

- [x] Task 1: Implement `log` methods on `DurableContext` in `operations/log.rs` (AC: #1, #2, #3, #5, #6)
  - [x] 1.1: `pub fn log(&self, message: &str)` — emit `tracing::info!` if NOT replaying, no-op if replaying
  - [x] 1.2: `pub fn log_with_data(&self, message: &str, data: &serde_json::Value)` — emit `tracing::info!` with structured data field if NOT replaying
  - [x] 1.3: `pub fn log_debug(&self, message: &str)` — emit `tracing::debug!` if NOT replaying
  - [x] 1.4: `pub fn log_warn(&self, message: &str)` — emit `tracing::warn!` if NOT replaying
  - [x] 1.5: `pub fn log_error(&self, message: &str)` — emit `tracing::error!` if NOT replaying
  - [x] 1.6: `pub fn log_debug_with_data(&self, message: &str, data: &serde_json::Value)` — debug with structured data
  - [x] 1.7: `pub fn log_warn_with_data(&self, message: &str, data: &serde_json::Value)` — warn with structured data
  - [x] 1.8: `pub fn log_error_with_data(&self, message: &str, data: &serde_json::Value)` — error with structured data
  - [x] 1.9: Include `execution_arn` and optional `operation_name` as tracing span/event fields for context enrichment (AC: #1, #7)
  - [x] 1.10: Rustdoc on all methods with `# Examples` documenting replay suppression behavior

- [x] Task 2: Add log delegation methods to `ClosureContext` (AC: #1, #6)
  - [x] 2.1: `log()`, `log_with_data()`, `log_debug()`, `log_warn()`, `log_error()` and `*_with_data` variants delegating to `self.inner.log*()`
  - [x] 2.2: Rustdoc with `# Examples` on each method

- [x] Task 3: Write tests (AC: #1, #2, #4, #5, #6, #8)
  - [x] 3.1: `test_log_emits_during_execution` — verify log is emitted when NOT replaying (use tracing-test or tracing-subscriber with capture)
  - [x] 3.2: `test_log_suppressed_during_replay` — verify log is NOT emitted when replaying
  - [x] 3.3: `test_log_with_structured_data` — verify structured data appears in log event
  - [x] 3.4: `test_log_all_levels` — verify debug, info, warn, error all work
  - [x] 3.5: `test_log_is_not_durable_operation` — verify no checkpoints sent, no operation ID generated
  - [x] 3.6: All doc tests compile via `cargo test --doc`

- [x] Task 4: Verify all checks pass (AC: #8)
  - [x] 4.1: `cargo test --workspace` — all tests pass (119 unit/integration + 120 doc tests)
  - [x] 4.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [x] 4.3: `cargo fmt --check` — formatting passes

### Review Follow-ups (AI)

- [x] [AI-Review][MED] AC#7 partial: child context logs lack `parent_id` enrichment — Added `parent_op_id: Option<String>` field to DurableContext, populated in `create_child_context()`, exposed via `parent_op_id()` accessor. All log methods now emit `parent_id` field (empty for root, operation ID for child contexts). Added 2 tests: `test_log_root_context_has_empty_parent_id`, `test_log_child_context_includes_parent_id`.
- [x] [AI-Review][LOW] Task 1.9 claims "optional `operation_name`" field but none is implemented — The Python SDK's `name` field comes from the current operation scope, not a user parameter on log calls. Since log methods are standalone (not tied to a specific operation), `operation_name` is not applicable. The `parent_id` field now provides the hierarchical context that was actually needed. Task 1.9 description was inaccurate; the real requirement (context enrichment) is satisfied by `execution_arn` + `parent_id`.

## Dev Notes

### CRITICAL: This Is NOT a Durable Operation

Unlike step, wait, callback, invoke, parallel, map, and child_context — the log operation does **NOT checkpoint to AWS**. It does **NOT use the operation ID generator**. It does **NOT interact with the replay engine's operations HashMap**.

The Python SDK's `context.logger` is purely a client-side replay-aware logging wrapper:
- During execution mode: logs are emitted normally via the logging framework
- During replay mode: logs are suppressed entirely
- The replay/execute mode detection uses the same `is_replaying()` check already on DurableContext

This means `operations/log.rs` will be fundamentally simpler than all other operation files — no async, no AWS types, no checkpoint lifecycle.

### Method Signatures — Synchronous, Not Async

All log methods are **synchronous** (`fn`, not `async fn`) because they don't make AWS API calls:

```rust
impl DurableContext {
    pub fn log(&self, message: &str) { ... }
    pub fn log_with_data(&self, message: &str, data: &serde_json::Value) { ... }
    pub fn log_debug(&self, message: &str) { ... }
    pub fn log_warn(&self, message: &str) { ... }
    pub fn log_error(&self, message: &str) { ... }
    // ... _with_data variants for each level
}
```

Note `&self` not `&mut self` — logging doesn't mutate context state.

### Replay Suppression Implementation

The core logic is trivially simple:

```rust
pub fn log(&self, message: &str) {
    if !self.is_replaying() {
        tracing::info!(
            execution_arn = %self.arn(),
            message = message,
            "durable_log"
        );
    }
}
```

### Python SDK Behavioral Reference

The Python SDK's `context.logger`:
- Provides methods: `debug()`, `info()`, `warning()`, `error()`, `exception()`
- Suppresses all logs during replay phase
- Enriches logs with execution context: `execution_arn`, `parent_id`, `name`, `attempt`
- Supports `extra` dict parameter for structured metadata
- Is NOT a checkpoint operation — purely client-side

Our Rust mapping:
- Python `context.logger.info(msg, extra={...})` → `ctx.log_with_data(msg, &json!({...}))`
- Python `context.logger.debug(msg)` → `ctx.log_debug(msg)`
- Python `context.logger.warning(msg)` → `ctx.log_warn(msg)`
- Python `context.logger.error(msg)` → `ctx.log_error(msg)`
- Python `context.logger.exception(msg)` → no direct equivalent (Rust doesn't have exceptions); `ctx.log_error(msg)` covers this

### Context Enrichment via Tracing Fields

Use tracing event fields for execution context:

```rust
tracing::info!(
    execution_arn = %self.arn(),
    message = message,
    "durable_log"
);
```

For structured data:
```rust
tracing::info!(
    execution_arn = %self.arn(),
    data = %data,  // serde_json::Value implements Display
    message = message,
    "durable_log"
);
```

Users configure their own `tracing_subscriber` (e.g., `tracing_subscriber::fmt()` or `tracing_subscriber::fmt().json()` for JSON output). The SDK does NOT set up a subscriber — it only emits events.

### Testing Approach

Since log operations use tracing (not stdout), testing requires capturing tracing events:

**Option 1 — `tracing-test` crate (simplest):**
Add `tracing-test` as a dev-dependency. It provides `#[traced_test]` attribute and `logs_contain()` assertion.

**Option 2 — Manual subscriber with capture:**
Create a test subscriber that records events, then assert on captured events.

**Recommended: Option 1** — `tracing-test` is purpose-built for this. Add to workspace dev-dependencies:
```toml
[workspace.dependencies]
tracing-test = "0.2"
```

Example test:
```rust
#[traced_test]
#[tokio::test]
async fn test_log_emits_during_execution() {
    let ctx = /* create DurableContext in Executing mode */;
    ctx.log("test message");
    assert!(logs_contain("test message"));
}

#[traced_test]
#[tokio::test]
async fn test_log_suppressed_during_replay() {
    let ctx = /* create DurableContext in Replaying mode */;
    ctx.log("should not appear");
    assert!(!logs_contain("should not appear"));
}
```

### What Exists vs What Needs to Be Added

**Already exists:**
- `operations/log.rs` — stub file with only doc comments (5 lines)
- `OperationType::Log` — defined in types.rs (but NOT used by this implementation since logging doesn't checkpoint)
- `tracing` in workspace dependencies
- `DurableContext::is_replaying()` — method to check replay mode
- `DurableContext::arn()` — method to get execution ARN
- `ClosureContext` delegation pattern (context.rs in closure crate)

**Needs to be added:**
- Log method implementations in `operations/log.rs`
- `ClosureContext` log delegation methods in `crates/durable-lambda-closure/src/context.rs`
- `tracing-test` dev-dependency (for testing only)
- Unit tests in `operations/log.rs`

**Does NOT need:**
- New error variants (logging doesn't fail with DurableError)
- New types in `types.rs`
- New re-exports in `lib.rs` or `prelude.rs`
- Any changes to `replay.rs`, `backend.rs`, or `context.rs`

### Architecture Doc Discrepancies

The architecture doc lists `operations/log.rs` alongside other checkpoint-based operations (FR29-FR31). However, studying the Python SDK reveals logging is NOT a checkpoint operation — it's client-side dedup. The `OperationType::Log` enum variant exists in types.rs but will NOT be used by the log operation implementation. It may be useful later if AWS adds server-side log operations.

### ClosureContext Delegation Pattern

```rust
// In crates/durable-lambda-closure/src/context.rs

pub fn log(&self, message: &str) {
    self.inner.log(message);
}

pub fn log_with_data(&self, message: &str, data: &serde_json::Value) {
    self.inner.log_with_data(message, data);
}

pub fn log_debug(&self, message: &str) {
    self.inner.log_debug(message);
}

// ... etc for all variants
```

### Previous Story Intelligence (Story 3.3 Learnings)

- Story 3.3 (child_context) was the last operation in Epic 3 before this story
- All 7 checkpoint-based operations are now complete: step, wait, callback, invoke, parallel, map, child_context
- The ClosureContext delegation pattern is well-established — follow exact same pattern (add methods, add rustdoc)
- All existing operations use `&mut self` because they mutate replay state. Log operations use `&self` since they don't mutate state.
- `cargo clippy -- -D warnings` is strict — ensure no unused imports or dead code

### File Structure Notes

- Implementation: `crates/durable-lambda-core/src/operations/log.rs` (rewrite the 5-line stub)
- Delegation: `crates/durable-lambda-closure/src/context.rs` (add log methods)
- Dev dependency: `Cargo.toml` (workspace level — add `tracing-test`)
- No changes to: `error.rs`, `types.rs`, `lib.rs`, `prelude.rs`, `replay.rs`, `backend.rs`, `context.rs`

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.4 — acceptance criteria, FR29-FR31]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR29, FR30, FR31]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements — NFR20]
- [Source: _bmad-output/planning-artifacts/architecture.md — tracing integration, operations/log.rs location]
- [Source: _bmad-output/implementation-artifacts/3-3-child-context-operation.md — previous story learnings, ClosureContext delegation pattern]
- [Source: Python SDK docs/core/logger.md — context.logger is NOT a checkpoint operation, purely client-side replay dedup]
- [Source: crates/durable-lambda-core/src/operations/log.rs — current stub, 5 lines]
- [Source: crates/durable-lambda-core/src/context.rs — is_replaying(), arn() methods]
- [Source: crates/durable-lambda-closure/src/context.rs — ClosureContext delegation pattern]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6 (1M context)

### Debug Log References

### Completion Notes List

- Implemented 8 replay-safe log methods on DurableContext: `log`, `log_with_data`, `log_debug`, `log_warn`, `log_error`, `log_debug_with_data`, `log_warn_with_data`, `log_error_with_data`
- All methods are synchronous (`fn`, not `async fn`) and take `&self` (not `&mut self`) since logging doesn't mutate state or call AWS
- Replay suppression via `is_replaying()` check — no-op during replay, emit via tracing macros during execution
- Each log event includes `execution_arn` and `parent_id` fields for context enrichment
- Added 8 matching delegation methods on ClosureContext following established pattern
- Added `tracing-test = "0.2"` as workspace dev-dependency for test assertions
- Wrote 10 comprehensive tests covering: execution emit, replay suppression, structured data, all levels, all levels suppressed, non-durable operation verification, data variants, ARN enrichment, root context parent_id, child context parent_id
- Added 1 ClosureContext delegation test
- Resolved review finding [MED]: Added `parent_op_id` field to DurableContext + `parent_op_id()` accessor + `parent_id` tracing field in all log methods
- Resolved review finding [LOW]: Clarified task 1.9 — `operation_name` not applicable to standalone log methods; `parent_id` provides the needed hierarchical context
- All 121 workspace tests pass, all doc tests pass, zero clippy warnings, formatting clean

### File List

- `Cargo.toml` — added `tracing-test = "0.2"` to workspace dependencies
- `crates/durable-lambda-core/Cargo.toml` — added `tracing-test` dev-dependency
- `crates/durable-lambda-core/src/context.rs` — added `parent_op_id: Option<String>` field, populated in `create_child_context()`, added `parent_op_id()` accessor
- `crates/durable-lambda-core/src/operations/log.rs` — implemented 8 log methods with `parent_id` enrichment + `log_parent_id()` helper + 10 unit tests
- `crates/durable-lambda-closure/src/context.rs` — added 8 log delegation methods + 1 delegation test
