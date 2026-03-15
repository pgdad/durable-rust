# Story 2.2: Callback Operation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to register a callback and suspend execution until an external system signals completion,
So that I can coordinate with external workflows (e.g., human approvals, third-party webhooks).

## Acceptance Criteria

1. **Given** a DurableContext in Executing mode **When** I call `ctx.create_callback("approval", options)` **Then** a START checkpoint is sent with OperationType::Callback and CallbackOptions **And** a `CallbackHandle` is returned containing the server-generated `callback_id` (FR14)

2. **Given** a `CallbackHandle` from `create_callback` **When** I call `ctx.callback_result::<T>(&handle)` and the callback has NOT been signaled (status STARTED/PENDING) **Then** `Err(DurableError::CallbackSuspended)` is returned to signal the function should exit (FR15)

3. **Given** a `CallbackHandle` **When** an external system sends a success signal with payload using the `callback_id` (FR16) **And** the function is re-invoked **And** I call `ctx.callback_result::<T>(&handle)` **Then** the deserialized success payload is returned as `Ok(T)`

4. **Given** a `CallbackHandle` **When** an external system sends a failure signal or the callback times out **And** the function is re-invoked **And** I call `ctx.callback_result::<T>(&handle)` **Then** `Err(DurableError::CallbackFailed)` is returned with error details

5. **Given** a DurableContext in Replaying mode with a completed callback in history **When** `create_callback` is called **Then** the cached `callback_id` is extracted from `callback_details` and returned in a `CallbackHandle` without sending any checkpoint **And** `callback_result` returns the cached result

6. **Given** the callback operation **When** I examine the implementation **Then** the logic lives in `crates/durable-lambda-core/src/operations/callback.rs` **And** the closure-native approach crate exposes `ctx.create_callback()` and `ctx.callback_result()` through its ClosureContext wrapper

7. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --workspace` **Then** all tests pass including new callback operation tests **And** all doc tests compile

## Tasks / Subtasks

- [x] Task 1: Add callback types to `types.rs` (AC: #1)
  - [x] 1.1: `CallbackOptions` struct with builder pattern (timeout_seconds, heartbeat_timeout_seconds)
  - [x] 1.2: `CallbackHandle` struct with `pub callback_id: String` and `pub(crate) operation_id: String`
  - [x] 1.3: Rustdoc with `# Examples` on both types
  - [x] 1.4: Re-export from `lib.rs`

- [x] Task 2: Add `DurableError::CallbackSuspended` and `DurableError::CallbackFailed` variants (AC: #2, #4)
  - [x] 2.1: `CallbackSuspended { operation_name: String, callback_id: String }` variant with `#[non_exhaustive]`
  - [x] 2.2: `CallbackFailed { operation_name: String, callback_id: String, error_message: String }` variant with `#[non_exhaustive]`
  - [x] 2.3: `DurableError::callback_suspended(operation_name, callback_id)` constructor
  - [x] 2.4: `DurableError::callback_failed(operation_name, callback_id, error_message)` constructor
  - [x] 2.5: Rustdoc with `# Examples` on variants and constructors

- [x] Task 3: Add `get_operation` method to `ReplayEngine` (AC: #1, #5)
  - [x] 3.1: `pub fn get_operation(&self, operation_id: &str) -> Option<&Operation>` — returns any operation regardless of status (unlike `check_result` which only returns completed)
  - [x] 3.2: Rustdoc with `# Examples`

- [x] Task 4: Implement `create_callback` on `DurableContext` in `operations/callback.rs` (AC: #1, #5)
  - [x] 4.1: `pub async fn create_callback(&mut self, name: &str, options: CallbackOptions) -> Result<CallbackHandle, DurableError>`
  - [x] 4.2: Operation ID via `generate_operation_id()`
  - [x] 4.3: Replay path: `get_operation(op_id)` → if found, extract `callback_details().callback_id()` → `track_replay` → return `CallbackHandle`
  - [x] 4.4: Execute path: build OperationUpdate with Callback type, Start action, "Callback" sub_type, CallbackOptions
  - [x] 4.5: START checkpoint via backend, token update
  - [x] 4.6: Merge `new_execution_state` from response
  - [x] 4.7: Extract `callback_id` from the merged operation's `callback_details`
  - [x] 4.8: `track_replay(op_id)` — always, both paths
  - [x] 4.9: Return `CallbackHandle { callback_id, operation_id }`
  - [x] 4.10: Rustdoc with `# Examples` and `# Errors`

- [x] Task 5: Implement `callback_result` on `DurableContext` in `operations/callback.rs` (AC: #2, #3, #4, #5)
  - [x] 5.1: `pub fn callback_result<T: DeserializeOwned>(&self, handle: &CallbackHandle) -> Result<T, DurableError>`
  - [x] 5.2: Look up operation by `handle.operation_id` via `get_operation`
  - [x] 5.3: Succeeded → extract `callback_details.result`, deserialize as T, return Ok(T)
  - [x] 5.4: Failed/Cancelled/TimedOut/Stopped → return `Err(CallbackFailed)` with error details
  - [x] 5.5: Started/Pending/Ready/not found → return `Err(CallbackSuspended)`
  - [x] 5.6: Do NOT call `track_replay` — this is not a durable operation, just a state query
  - [x] 5.7: Rustdoc with `# Examples` and `# Errors`

- [x] Task 6: Add `create_callback` and `callback_result` delegation to `ClosureContext` (AC: #6)
  - [x] 6.1: `create_callback()` method delegating to `self.inner.create_callback()`
  - [x] 6.2: `callback_result()` method delegating to `self.inner.callback_result()`
  - [x] 6.3: Updated `parse_operation_type` in handler.rs to recognize "Callback"/"CALLBACK"
  - [x] 6.4: Rustdoc with `# Examples` and `# Errors` on both methods

- [x] Task 7: Write tests (AC: #1, #2, #3, #4, #5, #7)
  - [x] 7.1: `test_create_callback_sends_start_checkpoint_and_returns_handle` — verifies Callback type, Start action, CallbackOptions, returns handle with callback_id
  - [x] 7.2: `test_create_callback_replays_from_history` — SUCCEEDED callback in history, returns CallbackHandle with cached callback_id, zero checkpoints
  - [x] 7.3: `test_callback_result_returns_deserialized_value_on_succeeded` — callback with result in callback_details, returns deserialized T
  - [x] 7.4: `test_callback_result_returns_error_on_failed` — FAILED callback, returns CallbackFailed error
  - [x] 7.5: `test_callback_result_returns_error_on_timed_out` — TIMED_OUT callback, returns CallbackFailed error
  - [x] 7.6: `test_callback_result_suspends_on_started` — STARTED callback (not yet signaled), returns CallbackSuspended error
  - [x] 7.7: All doc tests compile (52 core doc tests, 13 closure doc tests)
  - [x] 7.8: ClosureContext delegation tests (via doc tests on create_callback/callback_result)

- [x] Task 8: Verify all checks pass (AC: #7)
  - [x] 8.1: `cargo test --workspace` — 156 tests pass, 0 failures
  - [x] 8.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [x] 8.3: `cargo fmt --check` — formatting passes

### Review Follow-ups (AI)

- [x] [AI-Review][Medium] Fix duplicate doc comment line in `callback_result` rustdoc — lines 150-152 in `crates/durable-lambda-core/src/operations/callback.rs` have "Return the deserialized success payload if the callback has been" repeated with an empty `///` line between them
- [x] [AI-Review][Low] Update prelude.rs module-level doc comment to mention CallbackHandle and CallbackOptions in the re-exports list [crates/durable-lambda-closure/src/prelude.rs:12]

## Dev Notes

### Critical Architecture: Callback is a TWO-PHASE Operation

The callback operation is **fundamentally different** from step and wait because it has TWO distinct phases within a single invocation:

| Aspect | Step | Wait | Callback |
|--------|------|------|----------|
| Phases | Single (execute + checkpoint) | Single (START then exit) | TWO: create_callback + callback_result |
| Checkpoints | START → Execute → SUCCEED/FAIL | START only | START only (in create_callback) |
| Who sends SUCCEED? | SDK | Server (after timer) | **External system** (via SendDurableExecutionCallbackSuccess API) |
| Return value? | `Result<Result<T, E>, DurableError>` | `Result<(), DurableError>` | Phase 1: `CallbackHandle` / Phase 2: `Result<T, DurableError>` |
| Suspend signal | StepRetryScheduled | WaitSuspended | CallbackSuspended (from callback_result) |
| callback_id | N/A | N/A | **Server-generated**, returned in checkpoint response |

### Python SDK Callback Flow (Exact Wire Protocol)

```
FIRST EXECUTION (operation not in history):

create_callback(name, config):
1. generate_operation_id() → op_id
2. get_checkpoint_result(op_id) → NOT EXISTENT
3. Build OperationUpdate:
   - id: op_id
   - type: CALLBACK
   - action: START
   - sub_type: "Callback"
   - name: user-provided name
   - callback_options: CallbackOptions { timeout_seconds, heartbeat_timeout_seconds }
4. Checkpoint START (sync) — BLOCKS until API responds with callback_id
5. Double-check: re-check operation state
6. Extract callback_id from operation's callback_details
7. track_replay(op_id)
8. Return Callback object with callback_id

callback.result():
9. get_checkpoint_result(op_id) → EXISTENT (STARTED/PENDING)
10. Status is NOT completed → raise SuspendExecution
    → Function exits. Server keeps callback alive.
    → External system calls SendDurableExecutionCallbackSuccess(callback_id, result)
    → Server marks operation SUCCEEDED, re-invokes Lambda


RE-INVOCATION (after callback signaled — operation SUCCEEDED):

create_callback(name, config):
1. generate_operation_id() → same op_id (deterministic)
2. get_checkpoint_result(op_id) → EXISTENT (SUCCEEDED)
3. Extract callback_id from callback_details
4. track_replay(op_id)
5. Return Callback object with same callback_id

callback.result():
6. get_checkpoint_result(op_id) → SUCCEEDED
7. Extract callback_details.result (serialized payload string)
8. Deserialize → return value to developer
```

### CRITICAL Design Decisions

1. **`create_callback` NEVER suspends** — it always returns a `CallbackHandle`. Errors are deferred to `callback_result` to ensure deterministic replay (code between create and result must always execute).

2. **`callback_result` is NOT async and NOT a durable operation** — it only reads state. It does NOT call `track_replay`. It does NOT generate an operation ID. It just checks the operation status and returns/errors.

3. **callback_id is SERVER-GENERATED** — unlike operation_id (which is blake2b deterministic), the callback_id comes from the AWS Lambda Durable Execution service in the checkpoint response's `new_execution_state`.

4. **`get_operation` vs `check_result`** — `create_callback` must use a new `get_operation` method (returns any status) instead of `check_result` (returns only completed). A STARTED callback must still return the callback_id.

### AWS SDK Types for Callback

```rust
// OperationType::Callback — the operation type
// OperationAction::Start — the only action sent by the SDK

// CallbackOptions — timeout configuration
let callback_opts = aws_sdk_lambda::types::CallbackOptions::builder()
    .timeout_seconds(0)           // 0 = no timeout
    .heartbeat_timeout_seconds(0) // 0 = no heartbeat timeout
    .build();

// OperationUpdate — the START checkpoint
let update = OperationUpdate::builder()
    .id(op_id.clone())
    .r#type(OperationType::Callback)
    .action(OperationAction::Start)
    .sub_type("Callback")
    .name(name)
    .callback_options(callback_opts)
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;

// CallbackDetails — available on Operation after checkpoint response
// Fields:
//   callback_id: Option<String>  — the server-generated callback ID
//   result: Option<String>       — serialized success payload (set by external system)
//   error: Option<ErrorObject>   — error details (set by failure/timeout)
```

### Verify Builder Return Types

From Epic 1 learnings — AWS SDK builders have inconsistent return types:
- `CallbackOptions::builder().build()` — check if returns `Result` or direct type
- `OperationUpdate::builder().build()` — returns `Result` (confirmed from wait.rs)
- Test each `.build()` call in your tests

### New Types to Add

```rust
// In types.rs:

/// Configure callback timeout behavior.
pub struct CallbackOptions {
    timeout_seconds: i32,
    heartbeat_timeout_seconds: i32,
}

impl CallbackOptions {
    pub fn new() -> Self { Self { timeout_seconds: 0, heartbeat_timeout_seconds: 0 } }
    pub fn timeout_seconds(mut self, seconds: i32) -> Self { self.timeout_seconds = seconds; self }
    pub fn heartbeat_timeout_seconds(mut self, seconds: i32) -> Self { self.heartbeat_timeout_seconds = seconds; self }
    pub fn get_timeout_seconds(&self) -> i32 { self.timeout_seconds }
    pub fn get_heartbeat_timeout_seconds(&self) -> i32 { self.heartbeat_timeout_seconds }
}

/// Handle returned by `create_callback` containing the server-generated callback ID.
pub struct CallbackHandle {
    pub callback_id: String,
    pub(crate) operation_id: String,
}
```

### New DurableError Variants

```rust
// In error.rs:

/// A callback is pending — the function should exit and wait for external signal.
#[error("callback suspended for operation '{operation_name}' (callback_id: {callback_id}) — function should exit")]
#[non_exhaustive]
CallbackSuspended { operation_name: String, callback_id: String },

/// A callback failed or timed out.
#[error("callback failed for operation '{operation_name}' (callback_id: {callback_id}): {error_message}")]
#[non_exhaustive]
CallbackFailed { operation_name: String, callback_id: String, error_message: String },
```

### New ReplayEngine Method

```rust
// In replay.rs:

/// Look up an operation by ID, returning it regardless of status.
///
/// Unlike `check_result` which only returns completed operations,
/// this returns the operation in any status (Started, Pending, etc.).
/// Used by callback operations which need to extract callback_id
/// from STARTED operations.
pub fn get_operation(&self, operation_id: &str) -> Option<&Operation> {
    self.operations.get(operation_id)
}
```

### Method Signatures

```rust
// In operations/callback.rs (impl DurableContext):

/// Register a callback and return a handle with the server-generated callback ID.
///
/// During execution mode, sends a START checkpoint with callback configuration
/// and returns a [`CallbackHandle`] containing the `callback_id` that external
/// systems use to signal completion.
///
/// During replay mode, extracts the cached callback_id from history.
///
/// **Important:** This method NEVER suspends. Suspension happens in
/// [`callback_result`](Self::callback_result) when the callback hasn't
/// been signaled yet.
pub async fn create_callback(
    &mut self,
    name: &str,
    options: CallbackOptions,
) -> Result<CallbackHandle, DurableError>

/// Check the result of a previously created callback.
///
/// Returns the deserialized success payload if the callback has been
/// signaled with success. Returns an error if the callback failed,
/// timed out, or hasn't been signaled yet.
///
/// **Important:** This is NOT an async/durable operation — it only reads
/// the current operation state. It does NOT generate an operation ID or
/// create checkpoints.
pub fn callback_result<T: DeserializeOwned>(
    &self,
    handle: &CallbackHandle,
) -> Result<T, DurableError>
```

### What Exists vs What Needs to Be Added

**Already exists:**
- `DurableContext` with operation ID generation, checkpoint, replay engine
- `DurableBackend` trait and MockBackend pattern
- `ClosureContext` with delegation pattern
- `DurableError` with `#[non_exhaustive]` and constructor methods
- `operations/callback.rs` stub (header comment only)
- Wait operation as reference for START checkpoint + double-check pattern
- `StepRetryScheduled` and `WaitSuspended` variants as precedent for "function should exit" errors
- `StepOptions` as precedent for options builder pattern
- `parse_operation_type` already handles Step, Execution, Wait

**Needs to be added:**
- `CallbackOptions` struct in `types.rs` (builder pattern like StepOptions)
- `CallbackHandle` struct in `types.rs`
- `DurableError::CallbackSuspended` variant + constructor in `error.rs`
- `DurableError::CallbackFailed` variant + constructor in `error.rs`
- `ReplayEngine::get_operation()` method in `replay.rs`
- `DurableContext::create_callback()` method in `operations/callback.rs`
- `DurableContext::callback_result()` method in `operations/callback.rs`
- `ClosureContext::create_callback()` delegation in `durable-lambda-closure/src/context.rs`
- `ClosureContext::callback_result()` delegation in `durable-lambda-closure/src/context.rs`
- "Callback"/"CALLBACK" added to `parse_operation_type` in handler.rs
- Unit tests for all paths
- Rustdoc on all new public items

### Architecture Doc Discrepancies (IMPORTANT — Inherited)

From Epic 1 and Story 2.1 — always follow Python SDK over architecture doc:
1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Operation ID**: Uses blake2b hash of counter, NOT user-provided name
3. **Handler signature**: Takes owned `ClosureContext`, receives `(event, ctx)`
4. Architecture says `ctx.callback()` but Python SDK uses `create_callback(name, config)` returning a `Callback` object with separate `.result()` call. **Follow Python SDK's two-phase pattern.**

### Previous Story Intelligence (Story 2.1 — Wait Operation)

- Wait operation used the same START checkpoint + double-check pattern that callback will use
- `WaitSuspended` error pattern is the precedent for `CallbackSuspended`
- `CheckpointUpdatedExecutionState` (not `NewDurableExecutionState`) is the correct type for the double-check mock
- `parse_operation_type` was updated to handle "Wait"/"WAIT" — extend for "Callback"/"CALLBACK"
- AWS SDK `WaitOptions::builder().build()` returned direct (not Result) — check `CallbackOptions::builder().build()` similarly
- Token update from checkpoint response: `response.checkpoint_token()` then `set_checkpoint_token`
- `new_execution_state` merging: `response.new_execution_state()` → iterate `.operations()` → `insert_operation`

### Epic 1 Retrospective Intelligence

- AWS SDK builder `.build()` quirks: inconsistent return types. Test each builder
- Use `checkpoint_failed` for descriptive errors
- Follow parameter ordering: name first, then options
- `DurableError` is `#[non_exhaustive]` — safe to add new variants
- Python SDK is source of truth — architecture doc is guidance only

### Testing Approach

- Use the same MockBackend pattern from wait.rs tests
- For `create_callback` execute test: MockBackend must return a checkpoint response with `new_execution_state` containing an Operation with `callback_details` that has a `callback_id`
- For replay tests: create Operation with `OperationType::Callback`, appropriate status, and `callback_details`
- For `callback_result` tests: create DurableContext with pre-loaded operations at various statuses (Succeeded, Failed, TimedOut, Started)
- Key mock detail: the `callback_id` must be set on the Operation's `callback_details`, not generated by the SDK
- Test naming: `test_callback_{behavior}_{condition}`

### External System APIs (Context Only — Not Implemented by SDK)

The SDK does NOT implement these — they are called by external systems using the `callback_id`:
- `SendDurableExecutionCallbackSuccess(callback_id, result)` — marks operation SUCCEEDED
- `SendDurableExecutionCallbackFailure(callback_id, error)` — marks operation FAILED
- `SendDurableExecutionCallbackHeartbeat(callback_id)` — resets heartbeat timer

### Re-exports

- `CallbackOptions` and `CallbackHandle` must be re-exported from:
  - `durable-lambda-core/src/lib.rs` (via types module)
  - `durable-lambda-closure/src/prelude.rs` (for user imports)

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.2 — acceptance criteria]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR14, FR15, FR16]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Engine Architecture — DurableBackend trait, callback.rs]
- [Source: _bmad-output/implementation-artifacts/epic-1-retro-2026-03-14.md — process improvements]
- [Source: _bmad-output/implementation-artifacts/2-1-wait-operation.md — previous story learnings]
- [Source: crates/durable-lambda-core/src/operations/wait.rs — START checkpoint + double-check pattern]
- [Source: crates/durable-lambda-core/src/error.rs — WaitSuspended/StepRetryScheduled variant patterns]
- [Source: crates/durable-lambda-core/src/replay.rs — check_result only returns completed ops]
- [Source: Python SDK — github.com/aws/aws-durable-execution-sdk-python — callback wire protocol]
- [Source: aws_sdk_lambda::types::CallbackOptions — timeout_seconds, heartbeat_timeout_seconds]
- [Source: aws_sdk_lambda::types::CallbackDetails — callback_id, result, error]
- [Source: aws_sdk_lambda::types::OperationType::Callback — callback operation type]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- `CallbackOptions` struct with builder pattern (timeout_seconds, heartbeat_timeout_seconds) and `CallbackHandle` struct in types.rs
- `DurableError::CallbackSuspended` variant + `callback_suspended()` constructor — signals handler should exit while waiting for external signal
- `DurableError::CallbackFailed` variant + `callback_failed()` constructor — callback signaled failure, cancelled, or timed out
- `ReplayEngine::get_operation()` method — returns operation regardless of status (unlike `check_result` which only returns completed)
- `DurableContext::create_callback(name, options)` — two-phase callback: sends START checkpoint with CallbackOptions, extracts server-generated callback_id from checkpoint response's new_execution_state, returns CallbackHandle. NEVER suspends.
- `DurableContext::callback_result::<T>(handle)` — sync state query: Succeeded→deserialize result, Failed/TimedOut→CallbackFailed error, Started/Pending→CallbackSuspended error. NOT a durable operation (no track_replay, no checkpoint).
- `ClosureContext::create_callback()` and `callback_result()` delegation methods
- Updated `parse_operation_type` in handler.rs to recognize "Callback"/"CALLBACK"
- Re-exports in lib.rs (CallbackHandle, CallbackOptions) and prelude.rs
- 6 callback unit tests: execute+handle, replay, result-success, result-failed, result-timed-out, result-suspended
- `CallbackOptions::builder().build()` returns direct type (not Result), confirmed
- `CallbackDetails::builder().build()` returns direct type (not Result), confirmed
- Resolved review finding [Medium]: Fixed duplicate doc comment line in callback_result rustdoc
- Resolved review finding [Low]: Updated prelude.rs module doc to list CallbackHandle, CallbackOptions

### File List

- crates/durable-lambda-core/src/types.rs (modified — added CallbackOptions, CallbackHandle)
- crates/durable-lambda-core/src/lib.rs (modified — added CallbackHandle, CallbackOptions re-exports)
- crates/durable-lambda-core/src/error.rs (modified — added CallbackSuspended, CallbackFailed variants + constructors)
- crates/durable-lambda-core/src/replay.rs (modified — added get_operation() method)
- crates/durable-lambda-core/src/operations/callback.rs (rewritten — create_callback(), callback_result(), 6 unit tests)
- crates/durable-lambda-closure/src/context.rs (modified — added create_callback(), callback_result() delegation)
- crates/durable-lambda-closure/src/handler.rs (modified — added "Callback"/"CALLBACK" to parse_operation_type)
- crates/durable-lambda-closure/src/prelude.rs (modified — added CallbackHandle, CallbackOptions re-exports)

### Senior Developer Review (AI)

**Review Date:** 2026-03-14
**Reviewer:** Claude Opus 4.6
**Outcome:** Changes Requested (minor)

**Summary:** Clean implementation. All 7 ACs verified against code. All 34 subtasks genuinely completed. 6 real unit tests with meaningful assertions. No security issues, no architecture violations, no false claims. Two minor cosmetic findings.

**Action Items:**
- [x] [Medium] Fix duplicate doc comment line in callback_result rustdoc (callback.rs:150-152)
- [x] [Low] Update prelude.rs module doc to list CallbackHandle, CallbackOptions

### Change Log

- 2026-03-14: Story 2.2 implemented — callback operation with two-phase design (create_callback + callback_result). 6 callback unit tests + doc tests passing. Clippy clean, fmt clean, workspace builds. 156 total tests pass.
- 2026-03-14: Addressed code review findings — 2 items resolved (1 Medium: duplicate doc comment, 1 Low: prelude doc update)
