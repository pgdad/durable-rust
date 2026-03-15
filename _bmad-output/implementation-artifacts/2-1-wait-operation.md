# Story 2.1: Wait Operation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to suspend a durable function for a specified duration without consuming compute,
So that I can implement time-based delays in my workflows (e.g., retry cooldowns, scheduled follow-ups).

## Acceptance Criteria

1. **Given** a DurableContext in Executing mode **When** I call `ctx.wait("my_wait", duration_secs)` **Then** the function sends a START checkpoint with OperationType::Wait and WaitOptions containing the duration **And** returns `Err(DurableError::WaitSuspended)` to signal the function should exit (FR12)

2. **Given** a DurableContext in Replaying mode with a completed wait in history (status SUCCEEDED) **When** the replay engine encounters the wait entry **Then** the wait is skipped immediately without suspension **And** execution continues to the next operation

3. **Given** the wait operation **When** I examine the implementation **Then** the logic lives in `crates/durable-lambda-core/src/operations/wait.rs` **And** the closure-native approach crate exposes `ctx.wait(name, duration_secs)` through its ClosureContext wrapper

4. **Given** the ClosureContext **When** I call `ctx.wait("delay", 30)` **Then** it delegates directly to `DurableContext::wait` with the same arguments

5. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --workspace` **Then** all tests pass including new wait operation tests **And** all doc tests compile

## Tasks / Subtasks

- [x] Task 1: Add `DurableError::WaitSuspended` variant (AC: #1)
  - [x] 1.1: Added `WaitSuspended { operation_name: String }` variant with `#[non_exhaustive]`
  - [x] 1.2: Added `DurableError::wait_suspended(operation_name)` constructor
  - [x] 1.3: Rustdoc with `# Examples` on variant and constructor
  - [x] 1.4: Re-export automatic via existing DurableError re-export

- [x] Task 2: Implement `wait` method on `DurableContext` in `operations/wait.rs` (AC: #1, #2)
  - [x] 2.1: `pub async fn wait(&mut self, name: &str, duration_secs: i32) -> Result<(), DurableError>` as `impl DurableContext` block
  - [x] 2.2: Operation ID via `generate_operation_id()`
  - [x] 2.3: Replay path: check_result → track_replay → Ok(())
  - [x] 2.4: Execute path: OperationUpdate with Wait type, Start action, "Wait" sub_type, WaitOptions
  - [x] 2.5: START checkpoint via backend, token update
  - [x] 2.6: Merge new_execution_state from response
  - [x] 2.7: Double-check after START
  - [x] 2.8: Returns Err(WaitSuspended) if not completed
  - [x] 2.9: Rustdoc with `# Examples` and `# Errors`

- [x] Task 3: Add `wait` delegation to `ClosureContext` in `durable-lambda-closure` (AC: #3, #4)
  - [x] 3.1: `wait()` method delegating to `self.inner.wait()`
  - [x] 3.2: Rustdoc with `# Examples` and `# Errors`
  - [x] 3.3: Updated `parse_operation_type` in handler.rs to recognize "Wait"/"WAIT"

- [x] Task 4: Write tests (AC: #1, #2, #5)
  - [x] 4.1: `test_wait_sends_start_checkpoint_and_suspends` — verifies Wait type, Start action, WaitOptions with wait_seconds=30, returns WaitSuspended
  - [x] 4.2: `test_wait_replays_completed_wait` — SUCCEEDED wait in history, returns Ok(()), zero checkpoints
  - [x] 4.3: `test_wait_double_check_after_start` — DoubleCheckBackend returns completed op in new_execution_state, returns Ok(())
  - [x] 4.4: ClosureContext::wait doc test compiles (line 162)
  - [x] 4.5: All doc tests compile (42 core doc tests, 11 closure doc tests)

- [x] Task 5: Verify all checks pass (AC: #5)
  - [x] 5.1: `cargo test --workspace` — 61 core unit + 42 core doc + 6 closure unit + 11 closure doc + 6 testing unit + 12 testing doc = all pass
  - [x] 5.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [x] 5.3: `cargo fmt --check` — formatting passes

## Dev Notes

### Critical Architecture: Wait is NOT Like Step

The wait operation is **fundamentally different** from step operations:

| Aspect | Step | Wait |
|--------|------|------|
| Checkpoints | START → Execute → SUCCEED/FAIL | START only (single checkpoint) |
| Who sends SUCCEED? | SDK sends SUCCEED/FAIL | **Server** transitions to SUCCEEDED after timer |
| User closure? | Yes — executes user code | No — just suspends |
| Return value? | `Result<Result<T, E>, DurableError>` | `Result<(), DurableError>` |
| On execute | Runs closure, checkpoints result | Sends START, returns WaitSuspended error |

**Key insight**: The SDK sends ONE checkpoint (START with WaitOptions), then EXITS. The server handles the timer and status transition. On re-invocation, the operation is SUCCEEDED in history.

### Python SDK Wait Flow (Exact Wire Protocol)

```
FIRST EXECUTION (operation not in history):
1. generate_operation_id() → op_id
2. check_result(op_id) → not found
3. Build OperationUpdate:
   - id: op_id
   - type: WAIT
   - action: START
   - sub_type: "Wait"
   - name: user-provided name
   - wait_options: WaitOptions { wait_seconds: N }
4. Checkpoint START (sync), update token
5. Double-check: re-check if operation completed (handles edge case)
6. If not completed: raise TimedSuspendExecution (→ function exits)
   → Server waits N seconds, transitions operation to SUCCEEDED, re-invokes Lambda

RE-INVOCATION (operation in history with SUCCEEDED):
1. generate_operation_id() → same op_id (deterministic)
2. check_result(op_id) → found, status=SUCCEEDED
3. track_replay(op_id)
4. Return Ok(()) — execution continues past the wait
```

### AWS SDK Types for Wait

The following Rust AWS SDK types are available and MUST be used:

- **`OperationType::Wait`** — the operation type for wait operations
- **`OperationAction::Start`** — the only action sent by the SDK
- **`WaitOptions`** — has `wait_seconds: Option<i32>`. Use on `OperationUpdate` via `.wait_options()` builder method
- **`OperationUpdate`** builder has `.wait_options(WaitOptions)` method
- **`WaitDetails`** — has `scheduled_end_timestamp: Option<DateTime>`. Appears on completed wait operations in history (informational only, not used by replay logic)

```rust
// Building the wait START checkpoint:
let wait_opts = aws_sdk_lambda::types::WaitOptions::builder()
    .wait_seconds(duration_secs)
    .build();

let update = OperationUpdate::builder()
    .id(op_id.clone())
    .r#type(OperationType::Wait)
    .action(OperationAction::Start)
    .sub_type("Wait")
    .name(name)
    .wait_options(wait_opts)
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;
```

### WaitSuspended Error Pattern

Follows the same pattern as `StepRetryScheduled` — signals the handler should exit:

```rust
// In DurableError:
#[error("wait suspended for operation '{operation_name}' — function should exit")]
#[non_exhaustive]
WaitSuspended { operation_name: String },

// Constructor:
pub fn wait_suspended(operation_name: impl Into<String>) -> Self {
    Self::WaitSuspended { operation_name: operation_name.into() }
}
```

When `wait()` returns `Err(DurableError::WaitSuspended { .. })`, the handler propagates via `?` and the Lambda exits. The server re-invokes after the timer.

### Method Signature

```rust
/// Suspend execution for the specified duration.
///
/// During execution mode, sends a START checkpoint with the wait duration
/// and returns [`DurableError::WaitSuspended`] to signal the function
/// should exit. The server re-invokes after the duration.
///
/// During replay mode, returns `Ok(())` immediately if the wait has
/// already completed.
pub async fn wait(
    &mut self,
    name: &str,
    duration_secs: i32,
) -> Result<(), DurableError>
```

### What Exists vs What Needs to Be Added

**Already exists:**
- `DurableContext` with operation ID generation, checkpoint, replay engine
- `DurableBackend` trait and MockBackend pattern
- `ClosureContext` with delegation pattern
- `DurableError` with `#[non_exhaustive]` and constructor methods
- `operations/wait.rs` stub (header comment only)
- Step operation pattern to follow (same START checkpoint + double-check)
- `StepRetryScheduled` variant as precedent for "function should exit" errors

**Needs to be added:**
- `DurableError::WaitSuspended` variant + constructor in `error.rs`
- `DurableContext::wait()` method in `operations/wait.rs`
- `ClosureContext::wait()` delegation in `durable-lambda-closure/src/context.rs`
- Unit tests for execute/replay/double-check paths
- Rustdoc on all new public items

### Architecture Doc Discrepancies (IMPORTANT — Inherited)

From Epic 1 — always follow Python SDK over architecture doc:
1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Operation ID**: Uses blake2b hash of counter, NOT user-provided name
3. **Handler signature**: Takes owned `ClosureContext`, receives `(event, ctx)`

New for this story:
4. **Architecture says `ctx.wait(Duration::from_secs(30))`** but the Python SDK takes seconds as an integer, not a `Duration`. Use `i32` for seconds to match the AWS SDK's `WaitOptions.wait_seconds` type directly. Also takes a `name` parameter for checkpoint metadata.

### Previous Story Intelligence (Epic 1 Retrospective)

- AWS SDK builder `.build()` quirks: check whether `WaitOptions::builder().build()` and `OperationUpdate::builder().build()` return Result or direct type — test each
- Use `checkpoint_failed` for descriptive errors (not synthetic serde errors)
- Follow parameter ordering: name first, then options/config
- `DurableError` is `#[non_exhaustive]` — safe to add new variants
- Parser functions in closure handler now return `None` for unknown types — but `OperationType::Wait` must be added to `parse_operation_type` in handler.rs

### IMPORTANT: Update Parsers in Closure Handler

The `parse_operation_type` function in `crates/durable-lambda-closure/src/handler.rs` must be updated to recognize `"Wait"` / `"WAIT"` and return `Some(OperationType::Wait)`. Without this, wait operations in the Lambda event will be silently skipped.

### Testing Approach

- Use the same MockBackend pattern from step.rs tests
- For replay tests: create Operation with `OperationType::Wait` and `OperationStatus::Succeeded`
- For execute tests: verify checkpoint has `OperationType::Wait`, `OperationAction::Start`, and `WaitOptions`
- For the double-check test: MockBackend returns completed operation in new_execution_state after START
- Test naming: `test_wait_{behavior}_{condition}`

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.1 — acceptance criteria]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR12, FR13]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Engine Architecture — DurableBackend trait]
- [Source: _bmad-output/implementation-artifacts/epic-1-retro-2026-03-14.md — process improvements for Epic 2]
- [Source: crates/durable-lambda-core/src/operations/step.rs — START checkpoint + double-check pattern]
- [Source: crates/durable-lambda-core/src/error.rs — StepRetryScheduled variant pattern]
- [Source: Python SDK — github.com/aws/aws-durable-execution-sdk-python — wait wire protocol]
- [Source: aws_sdk_lambda::types::WaitOptions — wait_seconds field]
- [Source: aws_sdk_lambda::types::OperationType::Wait — wait operation type]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- `DurableError::WaitSuspended` variant + `wait_suspended()` constructor in error.rs
- `DurableContext::wait(name, duration_secs)` in operations/wait.rs — single START checkpoint with WaitOptions, returns WaitSuspended on execute, Ok(()) on replay
- Double-check pattern after START (same as step) — handles edge case where server completes wait synchronously
- `ClosureContext::wait()` delegation in closure crate context.rs
- Updated `parse_operation_type` in handler.rs to recognize "Wait"/"WAIT"
- 3 unit tests: execute+suspend, replay, double-check
- Uses `CheckpointUpdatedExecutionState` (not `NewDurableExecutionState`) for double-check test mock

### File List

- crates/durable-lambda-core/src/error.rs (modified — added WaitSuspended variant + constructor)
- crates/durable-lambda-core/src/operations/wait.rs (rewritten — wait() method, 3 unit tests)
- crates/durable-lambda-closure/src/context.rs (modified — added wait() delegation method)
- crates/durable-lambda-closure/src/handler.rs (modified — added "Wait"/"WAIT" to parse_operation_type)

### Change Log

- 2026-03-14: Story 2.1 implemented — wait operation with single START checkpoint, WaitSuspended error, replay support, double-check pattern. 3 wait unit tests + doc tests passing. Clippy clean, fmt clean, workspace builds.
