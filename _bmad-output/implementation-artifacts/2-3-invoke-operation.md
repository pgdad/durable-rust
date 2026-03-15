# Story 2.3: Invoke Operation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to durably invoke another Lambda function and receive its result,
So that I can compose durable workflows across multiple Lambda functions with guaranteed exactly-once invocation semantics.

## Acceptance Criteria

1. **Given** a DurableContext in Executing mode **When** I call `ctx.invoke("target-function", payload)` **Then** a START checkpoint is sent with OperationType::ChainedInvoke and ChainedInvokeOptions containing the function_name **And** the serialized payload is included in the checkpoint **And** `Err(DurableError::InvokeSuspended)` is returned to signal the function should exit (FR17)

2. **Given** a DurableContext in Replaying mode with a completed invoke in history (status SUCCEEDED) **When** the replay engine encounters the invoke entry **Then** the cached result is deserialized from `chained_invoke_details.result` and returned as `Ok(T)` without sending any checkpoint (FR18) **And** the target Lambda is NOT called again

3. **Given** an invoke operation in history with FAILED/TIMED_OUT/STOPPED status **When** the replay engine encounters it **Then** `Err(DurableError::InvokeFailed)` is returned with error details from `chained_invoke_details.error`

4. **Given** an invoke operation in history with STARTED status (target still executing) **When** the replay engine encounters it **Then** `Err(DurableError::InvokeSuspended)` is returned to re-suspend

5. **Given** the invoke checkpoint response contains the completed result in `new_execution_state` (immediate completion) **When** the double-check detects this **Then** the result is returned directly as `Ok(T)` without suspending

6. **Given** the invoke operation **When** I examine the implementation **Then** the logic lives in `crates/durable-lambda-core/src/operations/invoke.rs` **And** the closure-native approach crate exposes `ctx.invoke()` through its ClosureContext wrapper

7. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --workspace` **Then** all tests pass including new invoke operation tests **And** all doc tests compile

## Tasks / Subtasks

- [x] Task 1: Add `DurableError::InvokeSuspended` and `DurableError::InvokeFailed` variants (AC: #1, #3, #4)
  - [x]1.1: `InvokeSuspended { operation_name: String }` variant with `#[non_exhaustive]`
  - [x]1.2: `InvokeFailed { operation_name: String, error_message: String }` variant with `#[non_exhaustive]`
  - [x]1.3: `DurableError::invoke_suspended(operation_name)` constructor
  - [x]1.4: `DurableError::invoke_failed(operation_name, error_message)` constructor
  - [x]1.5: Rustdoc with `# Examples` on variants and constructors

- [x] Task 2: Implement `invoke` on `DurableContext` in `operations/invoke.rs` (AC: #1, #2, #3, #4, #5)
  - [x]2.1: `pub async fn invoke<T, P>(&mut self, name: &str, function_name: &str, payload: &P) -> Result<T, DurableError>` where T: DeserializeOwned, P: Serialize
  - [x]2.2: Operation ID via `generate_operation_id()`
  - [x]2.3: Replay path: `check_result(op_id)` → if SUCCEEDED, extract `chained_invoke_details.result`, deserialize as T, track_replay, return Ok(T)
  - [x]2.4: Replay path: `get_operation(op_id)` → if FAILED/TIMED_OUT/STOPPED, return `Err(InvokeFailed)` with error details
  - [x]2.5: Replay path: `get_operation(op_id)` → if STARTED/PENDING, return `Err(InvokeSuspended)`
  - [x]2.6: Execute path: serialize payload via `serde_json::to_string`
  - [x]2.7: Execute path: build OperationUpdate with ChainedInvoke type, Start action, "ChainedInvoke" sub_type, payload, ChainedInvokeOptions with function_name
  - [x]2.8: START checkpoint via backend, token update
  - [x]2.9: Merge `new_execution_state` from response
  - [x]2.10: Double-check: `check_result(op_id)` → if SUCCEEDED (immediate completion), deserialize, track_replay, return Ok(T)
  - [x]2.11: Double-check: `get_operation(op_id)` → if FAILED, return `Err(InvokeFailed)`
  - [x]2.12: Otherwise return `Err(InvokeSuspended)` — target function still executing
  - [x]2.13: Rustdoc with `# Examples` and `# Errors`

- [x] Task 3: Add `invoke` delegation to `ClosureContext` (AC: #6)
  - [x]3.1: `invoke()` method delegating to `self.inner.invoke()`
  - [x]3.2: Updated `parse_operation_type` in handler.rs to recognize "ChainedInvoke"/"CHAINED_INVOKE"
  - [x]3.3: Rustdoc with `# Examples` and `# Errors`

- [x] Task 4: Write tests (AC: #1, #2, #3, #4, #5, #7)
  - [x]4.1: `test_invoke_sends_start_checkpoint_and_suspends` — verifies ChainedInvoke type, Start action, ChainedInvokeOptions with function_name, serialized payload, returns InvokeSuspended
  - [x]4.2: `test_invoke_replays_succeeded_result` — SUCCEEDED invoke in history with chained_invoke_details.result, returns deserialized T, zero checkpoints
  - [x]4.3: `test_invoke_returns_error_on_failed` — FAILED invoke, returns InvokeFailed with error details
  - [x]4.4: `test_invoke_suspends_on_started` — STARTED invoke (target still running), returns InvokeSuspended
  - [x]4.5: `test_invoke_double_check_immediate_completion` — MockBackend returns SUCCEEDED in new_execution_state after START, returns Ok(T) without suspending
  - [x]4.6: All doc tests compile
  - [x]4.7: ClosureContext delegation tests (via doc tests)

- [x] Task 5: Verify all checks pass (AC: #7)
  - [x]5.1: `cargo test --workspace` — all tests pass
  - [x]5.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [x]5.3: `cargo fmt --check` — formatting passes

## Dev Notes

### Critical Architecture: Invoke is a Single-Phase Suspend Operation

The invoke operation is similar to wait — it sends a **single START checkpoint** and then suspends. The server handles the actual Lambda invocation and status transition. However, unlike wait, invoke carries a **payload** and returns a **typed result**.

| Aspect | Step | Wait | Callback | Invoke |
|--------|------|------|----------|--------|
| Checkpoints | START → SUCCEED/FAIL | START only | START only | **START only** |
| Who completes? | SDK (runs closure) | Server (timer) | External system | **Server (invokes target)** |
| Payload? | No (closure captures) | No | No | **Yes (serialized in checkpoint)** |
| Result? | `Result<T, E>` | `()` | `T` (from external) | **`T` (from target function)** |
| Suspend? | Never (except retry) | Always | Two-phase | **Always (except immediate)** |
| Double-check? | Yes | Yes | No | **Yes (detects immediate completion)** |

### Python SDK Invoke Flow (Exact Wire Protocol)

The Python SDK calls this `invoke()` but the wire protocol uses `CHAINED_INVOKE` as the operation type.

```
FIRST EXECUTION (operation not in history):
1. generate_operation_id() → op_id
2. check_result(op_id) → NOT FOUND
3. Serialize payload to JSON string
4. Build OperationUpdate:
   - id: op_id
   - type: CHAINED_INVOKE
   - action: START
   - sub_type: "ChainedInvoke"
   - name: user-provided name
   - payload: serialized payload string
   - chained_invoke_options: ChainedInvokeOptions { function_name }
5. Checkpoint START (sync, blocking)
6. Update token
7. Merge new_execution_state
8. Double-check: re-check operation state
   - If SUCCEEDED (immediate completion): deserialize result, track_replay, return Ok(T)
   - If FAILED: return Err(InvokeFailed)
   - If STARTED/PENDING: raise SuspendExecution → function exits
   → Server invokes target function asynchronously
   → When target completes, server marks operation SUCCEEDED, re-invokes Lambda

RE-INVOCATION (operation in history with SUCCEEDED):
1. generate_operation_id() → same op_id (deterministic)
2. check_result(op_id) → SUCCEEDED
3. Extract chained_invoke_details.result (serialized payload string)
4. Deserialize as T
5. track_replay(op_id)
6. Return Ok(T)

RE-INVOCATION (operation still STARTED — target not done yet):
1. generate_operation_id() → same op_id
2. get_operation(op_id) → STARTED
3. raise SuspendExecution → re-suspends

RE-INVOCATION (operation FAILED/TIMED_OUT/STOPPED):
1. check_result(op_id) → FAILED (is completed status)
2. Extract chained_invoke_details.error
3. Raise error to caller
```

### CRITICAL Design Decisions

1. **Wire type is `ChainedInvoke`, not `Invoke`** — The Python SDK exposes `invoke()` to users but uses `OperationType::ChainedInvoke` on the wire. Follow this pattern.

2. **Payload is serialized and sent in the checkpoint** — Unlike step (which runs a closure locally), invoke sends the serialized payload to the server in the `payload` field of the OperationUpdate.

3. **Always suspends (unless immediate completion)** — The invoke operation always sends START and returns `InvokeSuspended`, except when the double-check detects the target function completed immediately (edge case).

4. **Result comes from `chained_invoke_details.result`** — Not from `step_details`. The result is a JSON string that needs deserialization.

5. **Three-way status check needed** — Unlike wait (check_result or suspend), invoke must handle: SUCCEEDED (return result), FAILED/TIMED_OUT/STOPPED (return error), STARTED/PENDING (suspend).

### AWS SDK Types for Invoke

```rust
// OperationType::ChainedInvoke — the wire type
// OperationAction::Start — the only action sent

// ChainedInvokeOptions — target function configuration
// NOTE: .build() returns Result (function_name is required!)
let invoke_opts = aws_sdk_lambda::types::ChainedInvokeOptions::builder()
    .function_name("target-function-name")
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;

// OperationUpdate with payload
let update = OperationUpdate::builder()
    .id(op_id.clone())
    .r#type(OperationType::ChainedInvoke)
    .action(OperationAction::Start)
    .sub_type("ChainedInvoke")
    .name(name)
    .payload(serialized_payload)
    .chained_invoke_options(invoke_opts)
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;

// ChainedInvokeDetails — on completed Operation
// Fields:
//   result: Option<String>       — serialized result payload
//   error: Option<ErrorObject>   — error details on failure
// NOTE: .build() returns direct type (not Result)
```

### Builder Return Types (IMPORTANT)

- `ChainedInvokeOptions::builder().build()` → **Result** (function_name is required)
- `ChainedInvokeDetails::builder().build()` → direct type
- `OperationUpdate::builder().build()` → **Result** (confirmed from wait.rs/callback.rs)

### Method Signature

```rust
/// Durably invoke another Lambda function and return its result.
///
/// During execution mode, serializes the payload, sends a START checkpoint
/// with the target function name, and returns [`DurableError::InvokeSuspended`]
/// to signal the function should exit. The server invokes the target function
/// asynchronously and re-invokes this Lambda when complete.
///
/// During replay mode, returns the cached result without re-invoking.
///
/// # Arguments
///
/// * `name` — Human-readable name for the invoke operation
/// * `function_name` — Name or ARN of the target Lambda function
/// * `payload` — Input payload to send to the target function
pub async fn invoke<T, P>(
    &mut self,
    name: &str,
    function_name: &str,
    payload: &P,
) -> Result<T, DurableError>
where
    T: DeserializeOwned,
    P: Serialize,
```

### What Exists vs What Needs to Be Added

**Already exists:**
- `DurableContext` with operation ID generation, checkpoint, replay engine
- `ReplayEngine::check_result()` (completed ops) and `get_operation()` (any status)
- `DurableBackend` trait and MockBackend pattern
- `ClosureContext` with delegation pattern
- `DurableError` with `#[non_exhaustive]` and constructor methods
- `operations/invoke.rs` stub (header comment only)
- Wait + Callback operations as reference for START checkpoint + double-check pattern
- `WaitSuspended`, `CallbackSuspended`, `CallbackFailed` variants as precedent
- `parse_operation_type` handles Step, Execution, Wait, Callback

**Needs to be added:**
- `DurableError::InvokeSuspended` variant + constructor in `error.rs`
- `DurableError::InvokeFailed` variant + constructor in `error.rs`
- `DurableContext::invoke()` method in `operations/invoke.rs`
- `ClosureContext::invoke()` delegation in `durable-lambda-closure/src/context.rs`
- "ChainedInvoke"/"CHAINED_INVOKE" added to `parse_operation_type` in handler.rs
- Unit tests for all paths
- Rustdoc on all new public items

### Architecture Doc Discrepancies (IMPORTANT — Inherited)

From Epic 1, Story 2.1, and Story 2.2 — always follow Python SDK over architecture doc:
1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Operation ID**: Uses blake2b hash of counter, NOT user-provided name
3. **Handler signature**: Takes owned `ClosureContext`, receives `(event, ctx)`
4. Architecture says `ctx.invoke("function_name", payload)` with 2 params, but we add a `name` param first for consistency with all other operations (name, then config/options, then payload). So: `ctx.invoke("op_name", "target-function", &payload)`.
5. **Wire type is `ChainedInvoke`**, not `Invoke` — follow Python SDK exactly.

### Previous Story Intelligence (Story 2.2 — Callback Operation)

- Two-phase callback pattern showed how `get_operation()` handles non-completed statuses
- `CallbackSuspended`/`CallbackFailed` error patterns are precedent for `InvokeSuspended`/`InvokeFailed`
- `CheckpointUpdatedExecutionState` is the correct type for double-check mock
- Token update and new_execution_state merging patterns established
- `CallbackOptions::builder().build()` returned direct; `ChainedInvokeOptions::builder().build()` returns **Result** — different!

### Epic 1 Retrospective Intelligence

- AWS SDK builder `.build()` quirks: inconsistent return types. Test each builder
- Use `checkpoint_failed` for descriptive errors
- Follow parameter ordering: name first, then config, then payload
- `DurableError` is `#[non_exhaustive]` — safe to add new variants
- Python SDK is source of truth — architecture doc is guidance only

### Testing Approach

- Use same MockBackend pattern from callback.rs tests
- For execute test: MockBackend returns checkpoint response WITHOUT completed op in new_execution_state → InvokeSuspended
- For double-check immediate test: MockBackend returns SUCCEEDED op in new_execution_state → Ok(T)
- For replay tests: pre-load Operation with OperationType::ChainedInvoke, appropriate status, and chained_invoke_details
- Verify `payload` field is set on the OperationUpdate checkpoint
- Test naming: `test_invoke_{behavior}_{condition}`

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.3 — acceptance criteria]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR17, FR18]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Engine Architecture — DurableBackend trait, invoke.rs]
- [Source: _bmad-output/implementation-artifacts/2-2-callback-operation.md — previous story learnings]
- [Source: crates/durable-lambda-core/src/operations/callback.rs — START checkpoint + get_operation pattern]
- [Source: crates/durable-lambda-core/src/operations/wait.rs — START checkpoint + double-check pattern]
- [Source: crates/durable-lambda-core/src/error.rs — CallbackSuspended/CallbackFailed variant patterns]
- [Source: Python SDK — github.com/aws/aws-durable-execution-sdk-python — chained invoke wire protocol]
- [Source: aws_sdk_lambda::types::ChainedInvokeOptions — function_name (required), tenant_id]
- [Source: aws_sdk_lambda::types::ChainedInvokeDetails — result, error]
- [Source: aws_sdk_lambda::types::OperationType::ChainedInvoke — invoke operation type]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- `DurableError::InvokeSuspended` variant + `invoke_suspended()` constructor — signals handler should exit while target Lambda executes
- `DurableError::InvokeFailed` variant + `invoke_failed()` constructor — target failed, timed out, or stopped
- `DurableContext::invoke(name, function_name, &payload)` — sends START checkpoint with OperationType::ChainedInvoke, serialized payload in `payload` field, ChainedInvokeOptions with function_name. Always suspends unless double-check detects immediate completion.
- Three-way status handling: SUCCEEDED→deserialize from chained_invoke_details.result, FAILED/TIMED_OUT→InvokeFailed, STARTED/PENDING→InvokeSuspended
- `deserialize_invoke_result<T>` and `extract_invoke_error` helper methods (avoid generic-on-impl issue)
- `ClosureContext::invoke()` delegation method
- Updated `parse_operation_type` in handler.rs to recognize "ChainedInvoke"/"CHAINED_INVOKE"
- `ChainedInvokeOptions::builder().build()` returns Result (function_name required), confirmed
- `ChainedInvokeDetails::builder().build()` returns direct type, confirmed
- 5 invoke unit tests: execute+suspend, replay-succeeded, failed, started-suspend, double-check-immediate
- 165 total workspace tests pass (72 core unit + 55 core doc + 14 closure doc + 6 closure unit + 6 testing unit + 12 testing doc)

### File List

- crates/durable-lambda-core/src/error.rs (modified — added InvokeSuspended, InvokeFailed variants + constructors)
- crates/durable-lambda-core/src/operations/invoke.rs (rewritten — invoke(), deserialize_invoke_result(), extract_invoke_error(), 5 unit tests)
- crates/durable-lambda-closure/src/context.rs (modified — added invoke() delegation)
- crates/durable-lambda-closure/src/handler.rs (modified — added "ChainedInvoke"/"CHAINED_INVOKE" to parse_operation_type)

### Change Log

- 2026-03-14: Story 2.3 implemented — invoke operation with single START checkpoint, payload serialization, three-way status handling, double-check for immediate completion. 5 invoke unit tests + doc tests passing. Clippy clean, fmt clean, workspace builds. 165 total tests pass.
