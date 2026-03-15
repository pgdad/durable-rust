# Story 1.3: DurableBackend Trait & Replay Engine

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want a replay engine that loads execution history and distinguishes replay from execution mode,
So that durable operations can correctly return cached results or execute new work.

## Acceptance Criteria

1. **Given** the durable-lambda-core crate **When** I examine the backend module **Then** it exports a `DurableBackend` async trait covering the 2 SDK-internal AWS durable execution API operations (checkpoint + get_execution_state) **And** it exports a `RealBackend` struct that implements `DurableBackend` using `aws-sdk-lambda`

2. **Given** a `RealBackend` connected to AWS **When** a durable function is invoked **Then** the complete execution state (all operations) is loaded via pagination of `get_durable_execution_state` (FR1, NFR2) **And** operations are stored as a keyed collection in the replay engine

3. **Given** a loaded execution history **When** the replay engine initializes **Then** it creates a deterministic operation ID generator (ordered counter + blake2b hash) **And** it sets `ReplayStatus` to `Replaying` when completed operations exist, or `Executing` when history is empty (FR2)

4. **Given** the replay engine is in `Replaying` mode **When** a durable operation is encountered with a matching operation ID in history **Then** the cached result from history is returned without re-executing the operation (FR3) **And** the operation is marked as visited

5. **Given** the replay engine has visited all completed operations in history **When** the next durable operation is encountered **Then** the replay status transitions to `Executing` (FR2) **And** the operation executes and checkpoints its result to AWS (FR4)

6. **Given** any checkpoint value **When** it is serialized for storage or deserialized from history **Then** `serde_json` is used, matching the Python SDK's JSON format exactly (FR6)

7. **Given** all public types, traits, and methods **When** I run `cargo test --doc -p durable-lambda-core` **Then** all rustdoc examples compile and pass

8. **Given** AWS API transient failures (throttling, timeouts) **When** the `RealBackend` encounters them **Then** appropriate retries are performed before surfacing errors via `DurableError` (NFR7)

## Tasks / Subtasks

- [x] Task 1: Implement `DurableBackend` trait in `backend.rs` (AC: #1)
  - [x]1.1: Define the `DurableBackend` async trait with 2 methods:
    - `checkpoint(&self, arn: &str, checkpoint_token: &str, updates: Vec<OperationUpdate>, client_token: Option<String>) -> Result<CheckpointOutput, DurableError>` — wraps `checkpoint_durable_execution`
    - `get_execution_state(&self, arn: &str, checkpoint_token: &str, next_marker: &str, max_items: u32) -> Result<StateOutput, DurableError>` — wraps `get_durable_execution_state`
  - [x]1.2: Define supporting types: `OperationUpdate`, `CheckpointOutput`, `StateOutput`, `Operation`, `OperationStatus`, `OperationType` (AWS-level, distinct from the SDK-level `types::OperationType`), `OperationAction`
  - [x]1.3: Add rustdoc with `# Examples` and `# Errors` on the trait and every method
  - [x]1.4: Add unit tests verifying trait is object-safe (`dyn DurableBackend`)

- [x]Task 2: Implement `RealBackend` struct (AC: #1, #2, #8)
  - [x]2.1: Define `RealBackend` struct holding an `aws_sdk_lambda::Client`
  - [x]2.2: Implement `DurableBackend` for `RealBackend`, mapping to `aws-sdk-lambda` client calls
  - [x]2.3: Add retry logic for AWS API transient failures (throttling, timeouts) — use exponential backoff with jitter, max 3 retries
  - [x]2.4: Add rustdoc with `# Examples` and `# Errors` sections
  - [x]2.5: Add constructor `RealBackend::new(client: aws_sdk_lambda::Client) -> Self`

### Review Follow-ups (AI)

- [x] [AI-Review][HIGH] Backoff jitter is a no-op: `capped / 2 + capped / 2 == capped` produces no randomness. Add actual jitter (e.g., `rand::thread_rng().gen_range(0..=capped)` or similar) to prevent thundering herd on concurrent Lambda retries. [backend.rs:116]
- [x] [AI-Review][MEDIUM] Task 6 (6.1–6.4) marked [x] but custom types were not defined — AWS SDK types used directly instead. This is a better approach, but task descriptions should be updated to reflect what was actually done, or a deviation note added to each subtask.
- [x] [AI-Review][LOW] Missing `# Examples` rustdoc on public methods in replay.rs (track_replay, is_replaying, execution_mode, generate_operation_id, operations, insert_operation) and context.rs (execution_mode, is_replaying, arn, checkpoint_token, set_checkpoint_token, backend, replay_engine_mut, replay_engine). Architecture mandates examples on every public item.
- [x] [AI-Review][LOW] `OperationIdGenerator` not re-exported at crate root in lib.rs — add `pub use operation_id::OperationIdGenerator;` for consistency with other key types.
- [ ] [AI-Review][LOW] `insert_operation` doc example in replay.rs doesn't demonstrate insertion — shows empty engine assertion identical to `operations()` example. Update to show actual insert call. [replay.rs:229]

- [x]Task 3: Implement operation ID generation in a new `operation_id.rs` module (AC: #3)
  - [x]3.1: Implement deterministic operation ID generation: `blake2b(f"{parent_id}-{counter}")` truncated to 64 hex chars for child operations, `blake2b(f"{counter}")` for root-level operations — must match Python SDK's `OrderedCounter` + `blake2b` pattern exactly
  - [x]3.2: Add `blake2b` dependency to `Cargo.toml` (use the `blake2` crate)
  - [x]3.3: Implement `OperationIdGenerator` struct with `next_id(&mut self) -> String` method
  - [x]3.4: Add unit tests verifying ID generation produces the same IDs as the Python SDK for identical counter sequences
  - [x]3.5: Add rustdoc documenting the determinism invariant

- [x]Task 4: Implement replay engine in `replay.rs` (AC: #3, #4, #5)
  - [x]4.1: Define `ReplayEngine` struct holding: `operations: HashMap<String, Operation>` (keyed by operation ID), `visited: HashSet<String>`, `replay_status: ReplayStatus`, `id_generator: OperationIdGenerator`
  - [x]4.2: Implement `ReplayEngine::new(operations: HashMap<String, Operation>) -> Self` — sets initial `ReplayStatus` based on whether completed operations exist
  - [x]4.3: Implement `check_result(&mut self, operation_id: &str) -> Option<&Operation>` — looks up operation in the operations map, returns it if it exists with a completed status
  - [x]4.4: Implement `track_replay(&mut self, operation_id: &str)` — adds to visited set, transitions replay_status from `Replaying` to `Executing` when all completed operations have been visited
  - [x]4.5: Implement `is_replaying(&self) -> bool` — returns whether currently in replay mode
  - [x]4.6: Implement `generate_operation_id(&mut self) -> String` — delegates to id_generator
  - [x]4.7: Add comprehensive unit tests for replay status transitions
  - [x]4.8: Add rustdoc with `# Examples` on all public items

- [x]Task 5: Implement `DurableContext` in `context.rs` (AC: #2, #3, #4, #5, #6)
  - [x]5.1: Define `DurableContext` struct holding: `backend: Arc<dyn DurableBackend + Send + Sync>`, `replay_engine: ReplayEngine`, `durable_execution_arn: String`, `checkpoint_token: String` (mutable — updated after each checkpoint response)
  - [x]5.2: Implement `DurableContext::new(backend, arn, checkpoint_token, initial_operations, next_marker) -> Result<Self, DurableError>` — paginates through all remaining operations via `get_execution_state`, builds the full operations map, initializes `ReplayEngine`
  - [x]5.3: Implement the pagination loop: while `next_marker` is not empty, call `backend.get_execution_state()` and merge operations
  - [x]5.4: Implement `execution_mode(&self) -> ExecutionMode` — delegates to replay_engine.is_replaying()
  - [x]5.5: Add rustdoc with `# Examples` on the struct and all public methods

- [x]Task 6: Define checkpoint batching types (AC: #6)
  - [x]6.1: ~~Define `OperationUpdate` struct~~ **Deviation:** Used `aws_sdk_lambda::types::OperationUpdate` directly instead of redefining — avoids duplication, ensures wire-format compatibility
  - [x]6.2: ~~Derive serde renames~~ **Deviation:** AWS SDK types handle their own serialization — no custom serde needed
  - [x]6.3: ~~Define `CheckpointOutput` and `StateOutput`~~ **Deviation:** Used `CheckpointDurableExecutionOutput` and `GetDurableExecutionStateOutput` from AWS SDK directly
  - [x]6.4: ~~Add rustdoc on all types~~ **Deviation:** AWS SDK types have their own docs; trait methods document usage of these types

- [x]Task 7: Update `lib.rs` re-exports (AC: #1)
  - [x]7.1: Add `pub mod operation_id` to lib.rs
  - [x]7.2: Add `pub use` statements for `DurableBackend`, `RealBackend`, `DurableContext`, `ReplayEngine`, and key backend types
  - [x]7.3: Ensure lib.rs remains re-exports only — zero logic

- [x]Task 8: Verify all checks pass (AC: #7)
  - [x]8.1: Run `cargo test --doc -p durable-lambda-core` — all doc examples compile and pass
  - [x]8.2: Run `cargo test -p durable-lambda-core` — all unit tests pass
  - [x]8.3: Run `cargo clippy -p durable-lambda-core -- -D warnings` — no warnings
  - [x]8.4: Run `cargo fmt --check` — formatting passes
  - [x]8.5: Run `cargo build --workspace` — full workspace still builds

## Dev Notes

### Critical Architecture Constraints

- **DurableBackend has only 2 methods**: The Python SDK only calls 2 AWS APIs internally: `checkpoint_durable_execution` and `get_durable_execution_state`. The other 7 durable execution APIs are for external consumers (callback senders, management tools). Do NOT add methods for APIs the SDK doesn't call.
- **NOT `get_durable_execution_history`**: The architecture doc mentions `get_durable_execution_history` but the Python SDK actually uses `get_durable_execution_state`. These are different APIs. Follow the Python SDK — use `get_durable_execution_state`.
- **Operation ID determinism is critical**: IDs are generated via `blake2b("{parent_id}-{counter}")` (child) or `blake2b("{counter}")` (root), truncated to 64 hex chars. Same code path must produce same IDs across replays. This is the key invariant for replay correctness.
- **Operation-keyed, not position-keyed**: The Python SDK uses a `dict[str, Operation]` keyed by operation ID, NOT a positional `Vec` with cursor index. The architecture doc says "positional Vec with cursor" — this is wrong. Follow the Python SDK's hash-map approach for behavioral compliance.
- **ReplayStatus tracking**: Status transitions from `Replaying` to `Executing` when all completed operations have been visited (not when cursor passes end). Each operation's visit is tracked independently.
- **DurableBackend must be object-safe**: It's used as `Arc<dyn DurableBackend + Send + Sync>` for both `RealBackend` and `MockBackend`.
- **lib.rs = re-exports only**: Zero logic in lib.rs.
- **Checkpoint batching is deferred**: The Python SDK uses a background thread for checkpoint batching. For this story, implement synchronous checkpointing. Batching optimization can come in a later story if needed.
- **Suspension mechanism deferred**: The Python SDK uses `SuspendExecution` exceptions. The Rust equivalent (custom error variant or control flow mechanism) is not needed in this story — it will be addressed when implementing wait/callback operations.

### Python SDK Architecture Reference

The Python SDK's key components that map to this story:

| Python SDK | Rust Implementation |
|---|---|
| `DurableServiceClient` protocol | `DurableBackend` trait |
| `LambdaClient` | `RealBackend` |
| `ExecutionState` | `ReplayEngine` |
| `DurableContext` | `DurableContext` |
| `OrderedCounter` + blake2b | `OperationIdGenerator` |
| `ReplayStatus` enum | `ReplayStatus` enum (reuse `ExecutionMode` or new type) |
| `Operation` dataclass | `Operation` struct |
| `OperationUpdate` | `OperationUpdate` struct |

### Operation Types & Statuses (match Python SDK exactly)

**6 Operation Types** (AWS-level `Type` field):
- `EXECUTION` — the root invocation (contains original input payload)
- `CONTEXT` — child context
- `STEP` — checkpointed step
- `WAIT` — time-based suspension
- `CALLBACK` — external signal wait
- `CHAINED_INVOKE` — Lambda-to-Lambda invocation

**5 Operation Actions** (sent in `OperationUpdate.Action`):
- `START` — operation initiated
- `SUCCEED` — operation completed successfully
- `FAIL` — operation failed
- `RETRY` — operation scheduled for retry
- `CANCEL` — operation cancelled

**8 Operation Statuses** (returned in `Operation.Status`):
- `STARTED` — operation has been initiated
- `PENDING` — waiting for external event (wait/callback)
- `READY` — ready to execute (after retry delay)
- `SUCCEEDED` — completed successfully
- `FAILED` — completed with failure
- `CANCELLED` — cancelled
- `TIMED_OUT` — exceeded timeout
- `STOPPED` — execution stopped

**Completed statuses** (for replay tracking): `SUCCEEDED`, `FAILED`, `CANCELLED`, `TIMED_OUT`, `STOPPED`

### Operation Data Model

```rust
/// A single operation from the durable execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Operation {
    pub id: String,
    #[serde(rename = "Type")]
    pub operation_type: AwsOperationType,
    pub status: OperationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_details: Option<StepDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_details: Option<ExecutionDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_details: Option<ContextDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_details: Option<WaitDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_details: Option<CallbackDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chained_invoke_details: Option<ChainedInvokeDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StepDetails {
    pub attempt: u32,
    pub next_attempt_timestamp: Option<u64>,
    pub result: Option<String>,  // JSON string
    pub error: Option<OperationError>,
}
```

### DurableBackend Trait Shape

```rust
#[async_trait::async_trait]
pub trait DurableBackend: Send + Sync {
    /// Persist checkpoint updates for a durable execution.
    async fn checkpoint(
        &self,
        arn: &str,
        checkpoint_token: &str,
        updates: Vec<OperationUpdate>,
        client_token: Option<&str>,
    ) -> Result<CheckpointOutput, DurableError>;

    /// Get the current operation state of a durable execution (paginated).
    async fn get_execution_state(
        &self,
        arn: &str,
        checkpoint_token: &str,
        next_marker: &str,
        max_items: u32,
    ) -> Result<StateOutput, DurableError>;
}
```

**Note on async_trait**: Consider whether to use the `async_trait` crate or Rust's native async trait support (stabilized in Rust 1.75). Since the project targets latest stable Rust, native async traits should work. However, native async traits don't support `dyn` dispatch by default — you need `#[trait_variant::make(SendDurableBackend: Send)]` or the `async_trait` crate for `Arc<dyn DurableBackend>`. Evaluate which approach is simpler. The `async_trait` crate is battle-tested and more straightforward for `dyn` dispatch.

### blake2b Dependency

Add to `[workspace.dependencies]` in root Cargo.toml:
```toml
blake2 = "0.10"
hex = "0.4"     # for encoding hash to hex string
```

Add to `crates/durable-lambda-core/Cargo.toml`:
```toml
blake2 = { workspace = true }
hex = { workspace = true }
```

### Retry Strategy for RealBackend

Simple exponential backoff with jitter for transient AWS failures:
- Max retries: 3
- Base delay: 100ms
- Max delay: 2s
- Jitter: randomized within [0, base_delay * 2^attempt]
- Retryable conditions: throttling (429), server errors (5xx), timeouts

### Relationship to Existing Types

- `types::OperationType` (Step, Wait, Callback, etc.) is the SDK-level operation type enum for `HistoryEntry`. Keep it as-is.
- The new `AwsOperationType` (EXECUTION, CONTEXT, STEP, WAIT, CALLBACK, CHAINED_INVOKE) is the AWS API-level type field. These are related but distinct — the AWS level has EXECUTION and CONTEXT which don't map to user-facing operations.
- `types::ExecutionMode` (Replaying/Executing) can be reused or a new `ReplayStatus` can be created. Prefer reusing `ExecutionMode` for API consistency.

### Testing Approach

- Unit tests in `#[cfg(test)] mod tests` at bottom of each file
- Test operation ID generation determinism (same counter → same ID)
- Test replay status transitions (Replaying → Executing when all completed ops visited)
- Test pagination loop (mock multiple pages)
- Test `DurableBackend` trait is object-safe
- Test `Operation` serde round-trip
- Doc tests must compile standalone (include necessary `use` statements)

### New Dependencies Required

```toml
# Root Cargo.toml [workspace.dependencies]
blake2 = "0.10"
hex = "0.4"
async-trait = "0.1"  # if using async_trait for dyn dispatch
```

### Project Structure Notes

Files to create/modify:
```
crates/durable-lambda-core/src/
  lib.rs              — ADD pub mod operation_id, updated pub use re-exports
  backend.rs          — REPLACE stub with DurableBackend trait, RealBackend, supporting types
  replay.rs           — REPLACE stub with ReplayEngine implementation
  context.rs          — REPLACE stub with DurableContext implementation
  operation_id.rs     — NEW: deterministic operation ID generation
crates/durable-lambda-core/Cargo.toml — ADD blake2, hex, async-trait dependencies
Cargo.toml            — ADD blake2, hex, async-trait to [workspace.dependencies]
```

No other crates need changes. The testing crate's `MockBackend` will implement `DurableBackend` in Story 1.7.

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.3]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements - Core Replay Engine (FR1-FR7)]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements (NFR2, NFR5, NFR7)]
- [Source: _bmad-output/implementation-artifacts/1-2-core-types-and-error-foundation.md — existing types and error patterns]
- [Source: Python SDK — github.com/aws/aws-durable-execution-sdk-python — behavioral reference for replay engine]

### Previous Story Intelligence (Story 1.2)

- `types.rs` exports `HistoryEntry`, `OperationType`, `ExecutionMode`, `CheckpointResult` — all serde-serializable
- `error.rs` exports `DurableError` with 6 variants, all constructed via static methods, `#[non_exhaustive]`
- `DurableError` is `Send + Sync` (verified by test)
- `lib.rs` has `pub mod` for all modules and `pub use` for types and errors
- `backend.rs`, `context.rs`, `replay.rs` are currently doc-comment-only stubs ready to be replaced
- thiserror `2.0.18`, serde `1.0.228`, aws-sdk-lambda `1.118.0` are pinned in workspace deps
- Code review added `#[non_exhaustive]` to `DurableError` and struct variants — follow same pattern for new public enums
- `ExecutionMode` has `Serialize + Deserialize` derives (added during code review)

### Architecture Doc Discrepancies (IMPORTANT)

The architecture document contains assumptions that don't match the Python SDK implementation. **Always follow the Python SDK over the architecture doc for behavioral compliance (NFR5):**

1. **API used**: Architecture says `get_durable_execution_history`. Python SDK uses `get_durable_execution_state`. Use `get_durable_execution_state`.
2. **Data structure**: Architecture says `Vec<HistoryEntry>` with positional cursor. Python SDK uses `dict[str, Operation]` keyed by operation ID. Use HashMap keyed by operation ID.
3. **Replay tracking**: Architecture implies simple cursor advancement. Python SDK tracks visited operations via a HashSet and transitions when all completed ops are visited. Use the HashSet approach.
4. **Operation ID**: Architecture doesn't mention deterministic ID generation. Python SDK uses blake2b hashing of counter values. Implement blake2b-based ID generation.
5. **DurableBackend method count**: Architecture says "all 9 AWS durable execution API operations". Python SDK only uses 2 internally (checkpoint + get_execution_state). Implement only the 2 the SDK needs.

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- Used AWS SDK types directly (`aws_sdk_lambda::types::Operation`, `OperationUpdate`, `OperationStatus`, etc.) instead of redefining — avoids duplication, ensures type compatibility
- `DurableBackend` trait has 2 async methods (checkpoint + get_execution_state) using `async_trait` for dyn dispatch
- `RealBackend` implements retry with exponential backoff (max 3 retries, 100ms base, 2s cap)
- `OperationIdGenerator` uses blake2b hashing matching Python SDK pattern: `blake2b("{counter}")` for root, `blake2b("{parent_id}-{counter}")` for children, truncated to 64 hex chars
- `ReplayEngine` uses `HashMap<String, Operation>` keyed by operation ID (matches Python SDK), tracks visited ops via HashSet, transitions Replaying→Executing when all completed ops visited
- `DurableContext` paginates all operations on construction, owns backend + replay engine
- Added `blake2`, `hex`, `async-trait`, `aws-smithy-types` dependencies
- 44 unit tests + 19 doc tests all passing, clippy clean, fmt clean, full workspace builds
- ✅ Resolved review finding [HIGH]: Replaced no-op jitter with full jitter using SystemTime nanoseconds as entropy source
- ✅ Resolved review finding [MEDIUM]: Added deviation notes to Task 6 subtasks documenting AWS SDK type reuse decision
- ✅ Resolved review finding [LOW]: Added `# Examples` rustdoc to all public methods in replay.rs (6 methods) and context.rs (8 methods) — doc tests grew from 19 to 33
- ✅ Resolved review finding [LOW]: Added `pub use operation_id::OperationIdGenerator` to lib.rs crate root re-exports

### File List

- Cargo.toml (modified — added blake2, hex, async-trait, aws-smithy-types to workspace deps)
- crates/durable-lambda-core/Cargo.toml (modified — added blake2, hex, async-trait, aws-smithy-types)
- crates/durable-lambda-core/src/lib.rs (modified — added pub mod operation_id, pub use for new types)
- crates/durable-lambda-core/src/backend.rs (modified — replaced stub with DurableBackend trait + RealBackend)
- crates/durable-lambda-core/src/replay.rs (modified — replaced stub with ReplayEngine)
- crates/durable-lambda-core/src/context.rs (modified — replaced stub with DurableContext)
- crates/durable-lambda-core/src/operation_id.rs (new — deterministic operation ID generation)

### Change Log

- 2026-03-14: Story 1.3 implemented — DurableBackend trait, RealBackend with retries, ReplayEngine with operation-keyed HashMap, DurableContext with pagination, OperationIdGenerator with blake2b, 63 tests passing
- 2026-03-14: Code review — 1 HIGH (jitter no-op), 1 MEDIUM (Task 6 deviation), 2 LOW (missing rustdoc examples, missing re-export). 4 action items created. Status → in-progress.
- 2026-03-14: Addressed code review findings — 4 items resolved (1 HIGH, 1 MEDIUM, 2 LOW). Jitter fixed with SystemTime entropy, Task 6 deviation documented, 14 rustdoc examples added, OperationIdGenerator re-exported. 44 unit + 33 doc tests passing.
- 2026-03-14: Second code review — all previous findings verified resolved. 1 new LOW (insert_operation doc example). Status → done.
