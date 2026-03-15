# Story 1.4: Step Operation Implementation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to define named steps that checkpoint results and replay from cache,
So that my durable Lambda functions can safely resume after interruption.

## Acceptance Criteria

1. **Given** a `DurableContext` in Executing mode **When** I call `ctx.step("validate_order", || async { Ok(validated_order) })` **Then** the closure executes and its result is checkpointed to AWS via `DurableBackend` (FR8, FR4) **And** the step name "validate_order" is used as the checkpoint key

2. **Given** a `DurableContext` in Replaying mode with a cached result for step "validate_order" **When** I call `ctx.step("validate_order", || async { ... })` **Then** the closure is NOT executed (FR11) **And** the previously checkpointed result is deserialized and returned (FR3) **And** the operation is tracked as visited in the replay engine (FR5)

3. **Given** a step closure that returns `Result<T, E>` where T and E implement `Serialize + DeserializeOwned` **When** the step executes successfully **Then** `Ok(T)` is serialized as JSON and checkpointed in the `Payload` field of a SUCCEED `OperationUpdate`

4. **Given** the step operation is implemented in core **When** I examine the file structure **Then** step logic lives in `crates/durable-lambda-core/src/operations/step.rs` **And** it follows the parameter ordering convention: name first, closure last

5. **Given** multiple sequential steps **When** they execute in order **Then** each step generates a unique deterministic operation ID via `OperationIdGenerator` **And** SDK overhead per step adds < 1ms latency beyond the AWS API call (NFR1)

6. **Given** all public types, traits, and methods **When** I run `cargo test --doc -p durable-lambda-core` **Then** all rustdoc examples compile and pass

## Tasks / Subtasks

- [x] Task 1: Implement the step operation in `operations/step.rs` (AC: #1, #2, #3, #4, #5)
  - [x] 1.1: Define a `step` async method on `DurableContext` that accepts `name: &str` and a closure `F: FnOnce() -> Fut + Send + 'static` where `Fut: Future<Output = Result<T, E>> + Send`, `T: Serialize + DeserializeOwned + Send`, `E: Serialize + DeserializeOwned + Send`
  - [x] 1.2: Implement the replay path — generate operation ID, call `replay_engine.check_result(op_id)`, if found with SUCCEEDED status: deserialize result from `StepDetails.result` (JSON string), track_replay, return `Ok(deserialized_value)`
  - [x] 1.3: Implement the replay path for FAILED status — deserialize error from `StepDetails.error.error_data`, track_replay, return `Err(deserialized_error)`
  - [x] 1.4: Implement the execute path — generate operation ID, checkpoint START OperationUpdate (sync), execute closure, checkpoint SUCCEED or FAIL OperationUpdate (sync), update checkpoint token from response, return result
  - [x] 1.5: Handle the "double-check" pattern from Python SDK: after START checkpoint, re-check if the operation now has a result (handles race conditions with concurrent checkpoints)
  - [x] 1.6: Ensure step method follows parameter ordering: name first, closure last

- [x] Task 2: Define step-related checkpoint types (AC: #1, #3)
  - [x] 2.1: ~~Add constructor methods~~ **Deviation:** Used AWS SDK's `OperationUpdate::builder()` and `ErrorObject::builder()` directly — no custom constructors needed
  - [x] 2.2: ~~Define `OperationError` struct~~ **Deviation:** Used `aws_sdk_lambda::types::ErrorObject` directly, which has `error_message`, `error_type`, `error_data`, `stack_trace` fields
  - [x] 2.3: Use `aws_sdk_lambda::types::OperationUpdate` directly (as done in backend.rs) rather than redefining — build updates using the SDK's builder pattern

- [x] Task 3: Add `step` method to `DurableContext` public API (AC: #1, #4)
  - [x] 3.1: ~~Implement in context.rs~~ Implemented as `impl DurableContext` block in `operations/step.rs` — keeps context.rs thin while making step a direct method on DurableContext
  - [x] 3.2: Chose approach: step logic directly as method on DurableContext via impl block in operations/step.rs
  - [x] 3.3: Ensure the public API signature matches the architecture convention: `ctx.step("name", || async { ... }).await`

- [x] Task 4: Update `lib.rs` and `operations/mod.rs` re-exports (AC: #4)
  - [x] 4.1: Ensure `operations/step.rs` exports are accessible through `operations/mod.rs` — `pub mod step` already in mod.rs
  - [x] 4.2: No new public types needed — step method is on DurableContext, return type is `Result<Result<T, E>, DurableError>`
  - [x] 4.3: Ensure lib.rs remains re-exports only — zero logic

- [x] Task 5: Write comprehensive tests (AC: #1, #2, #3, #5, #6)
  - [x] 5.1: Test step in Executing mode — verify START + SUCCEED checkpoints are sent, result returned
  - [x] 5.2: Test step in Replaying mode with SUCCEEDED operation — verify closure NOT called, cached result returned
  - [x] 5.3: Test step in Replaying mode with FAILED operation — verify error deserialized and returned
  - [x] 5.4: Test step serialization round-trip — complex types (structs with nested fields) serialize/deserialize correctly
  - [x] 5.5: Test multiple sequential steps — verify each gets unique deterministic operation ID
  - [x] 5.6: Test that `track_replay` is called after replaying — verify replay status transitions correctly
  - [x] 5.7: Add rustdoc examples on step method and any new public types

### Review Follow-ups (AI)

- [ ] [AI-Review][MEDIUM] No test for execute-path FAIL checkpoint — closure returning `Err(E)` should trigger START + FAIL checkpoints with correct `ErrorObject` fields (error_type, error_data). Only the replay FAIL path is tested. [step.rs tests]
- [ ] [AI-Review][LOW] Synthetic serde error for missing step_details is misleading — `serde_json::from_str::<serde_json::Value>("").unwrap_err()` produces "EOF while parsing" instead of describing the actual problem (missing step_details/result/error field). [step.rs:209-212, 225-228]

- [x] Task 6: Verify all checks pass (AC: #6)
  - [x] 6.1: Run `cargo test --doc -p durable-lambda-core` — all doc examples compile and pass (34 passing)
  - [x] 6.2: Run `cargo test -p durable-lambda-core` — all unit tests pass (50 passing)
  - [x] 6.3: Run `cargo clippy -p durable-lambda-core -- -D warnings` — no warnings
  - [x] 6.4: Run `cargo fmt --check` — formatting passes
  - [x] 6.5: Run `cargo build --workspace` — full workspace still builds

## Dev Notes

### Critical Architecture Constraints

- **This story implements basic step WITHOUT retries**: Retry logic (StepOptions, retry strategies) is Story 1.5. Do NOT implement retries here. Implement only the execute-and-checkpoint and replay-from-cache paths.
- **Two-phase checkpoint pattern**: The Python SDK sends START as one checkpoint, then SUCCEED/FAIL as a separate checkpoint. Both are synchronous (blocking) checkpoint calls for steps. Follow this pattern exactly.
- **Double-check pattern**: After sending START checkpoint (sync), re-read the operation status. If the operation already has a result (e.g., from a prior interrupted execution), use that cached result instead of re-executing. This handles the case where a previous invocation checkpointed START but crashed before the SDK saw the SUCCEED/FAIL response.
- **Operation IDs, not names**: Steps are identified by their deterministic operation ID (from `OperationIdGenerator`), NOT by the user-provided step name. The name is metadata sent in the `OperationUpdate` for debugging, but the operation ID is the lookup key.
- **Payload is JSON string**: The step result is serialized to a JSON string via `serde_json::to_string()` and placed in the `Payload` field of the SUCCEED `OperationUpdate`. On replay, it comes back in `Operation.step_details.result` as a string to be deserialized.
- **Error format**: Step errors use `OperationError` with fields: `ErrorMessage`, `ErrorType`, `ErrorData` (serialized error value), `StackTrace`. In Rust, `ErrorType` is the type name, `ErrorData` is the serde-serialized error value.
- **lib.rs = re-exports only**: Zero logic in lib.rs.
- **Constructor methods for errors**: Continue using `DurableError` constructor methods, never raw struct construction.

### Python SDK Step Flow (Reference Implementation)

```
EXECUTING MODE:
1. generate_operation_id() → op_id
2. check_result_status(op_id) → not found
3. checkpoint(START update, sync=true) → blocks until persisted
4. check_result_status(op_id) again → should be STARTED (double-check)
5. execute closure → result
6. if Ok(result):
   checkpoint(SUCCEED update with Payload=serialize(result), sync=true)
   return Ok(result)
7. if Err(error):
   checkpoint(FAIL update with Error=error_info, sync=true)
   return Err(error)

REPLAYING MODE:
1. generate_operation_id() → op_id (same ID due to determinism)
2. check_result_status(op_id) → found in operations map
3. if SUCCEEDED: deserialize(operation.step_details.result) → return Ok(value)
4. if FAILED: deserialize(operation.step_details.error.error_data) → return Err(error)
5. track_replay(op_id) → marks as visited, may transition to Executing
```

### OperationUpdate Wire Format for Steps

Use `aws_sdk_lambda::types::OperationUpdate` builder. Key fields:

```rust
// START:
OperationUpdate::builder()
    .id(&op_id)
    .r#type(OperationType::Step)
    .action(OperationAction::Start)
    .name(&step_name)           // optional user-provided name
    .parent_id(parent_id)       // optional, for child contexts
    .sub_type("Step")
    .build()

// SUCCEED:
OperationUpdate::builder()
    .id(&op_id)
    .r#type(OperationType::Step)
    .action(OperationAction::Succeed)
    .name(&step_name)
    .parent_id(parent_id)
    .payload(serialized_result)  // JSON string
    .sub_type("Step")
    .build()

// FAIL:
OperationUpdate::builder()
    .id(&op_id)
    .r#type(OperationType::Step)
    .action(OperationAction::Fail)
    .name(&step_name)
    .parent_id(parent_id)
    .error(operation_error)
    .sub_type("Step")
    .build()
```

**IMPORTANT**: Check if `aws_sdk_lambda::types::OperationUpdate`, `OperationAction`, and related types exist in the AWS SDK. If they do, use them directly. If not (the durable execution API may be too new), you may need to define equivalent types. The backend.rs already uses `aws_sdk_lambda::types::OperationUpdate` in the `checkpoint` method signature — follow that pattern.

### Step Result Extraction from Operation

When replaying, the result is extracted from the `aws_sdk_lambda::types::Operation` object:
- For SUCCEEDED: `operation.step_details().result()` returns `Option<&str>` (the JSON string)
- For FAILED: `operation.step_details().error()` contains the error info

Check the actual AWS SDK types for `StepDetails` — use whatever fields are available. The Python SDK reads `StepDetails.result` for the payload and `StepDetails.error` for the error.

### Step Method Signature Design

```rust
impl DurableContext {
    /// Execute a named step with checkpointing.
    ///
    /// During execution mode, runs the closure and checkpoints the result.
    /// During replay mode, returns the previously checkpointed result
    /// without executing the closure.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable step name, used as checkpoint metadata
    /// * `f` - Closure to execute (skipped during replay)
    ///
    /// # Errors
    ///
    /// Returns [`DurableError`] if checkpointing or deserialization fails.
    /// Returns `Err(E)` if the step closure returns an error (also checkpointed).
    pub async fn step<T, E, F, Fut>(
        &mut self,
        name: &str,
        f: F,
    ) -> Result<Result<T, E>, DurableError>
    where
        T: Serialize + DeserializeOwned + Send,
        E: Serialize + DeserializeOwned + Send,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        // Implementation in operations/step.rs
    }
}
```

**Return type consideration**: The step returns `Result<Result<T, E>, DurableError>` — the outer `Result` is for SDK errors (checkpoint failure, deserialization), the inner `Result<T, E>` is the user's step result. This matches the Python SDK's separation between SDK-level exceptions and user-level step outcomes. Alternatively, consider `Result<T, StepError<E>>` where `StepError` wraps both — evaluate which is more ergonomic.

### Existing Code to Build On

From Story 1.3, the following are already implemented and available:

- **`DurableContext`** (`context.rs`): Holds `backend: Arc<dyn DurableBackend>`, `replay_engine: ReplayEngine`, `durable_execution_arn`, `checkpoint_token`. Has `replay_engine_mut()`, `backend()`, `arn()`, `checkpoint_token()`, `set_checkpoint_token()`, `is_replaying()`, `execution_mode()`.
- **`ReplayEngine`** (`replay.rs`): Has `check_result(op_id) -> Option<&Operation>`, `track_replay(op_id)`, `generate_operation_id() -> String`, `is_replaying()`, `insert_operation()`.
- **`DurableBackend`** (`backend.rs`): Has `checkpoint(arn, checkpoint_token, updates, client_token) -> Result<CheckpointDurableExecutionOutput, DurableError>`.
- **`DurableError`** (`error.rs`): Has constructors `checkpoint_failed()`, `serialization()`, `deserialization()`, `replay_mismatch()`, `aws_sdk_operation()`.
- **`OperationIdGenerator`** (`operation_id.rs`): Deterministic blake2b-based ID generation.
- **AWS SDK types used**: `aws_sdk_lambda::types::Operation`, `OperationUpdate`, `OperationStatus`, `OperationType`.

### Testing Approach

- Create a test-only `MockBackend` implementing `DurableBackend` within `#[cfg(test)]` module (similar to `TestBackend` in `context.rs` tests). This is NOT the production MockBackend from `durable-lambda-testing` (Story 1.7).
- Test with pre-populated operations map for replay scenarios.
- Verify checkpoint calls contain correct OperationUpdate fields.
- Use `Arc<Mutex<Vec<OperationUpdate>>>` in mock to capture checkpoint calls for assertion.
- Test naming: `test_step_{behavior}_{condition}` e.g., `test_step_executes_closure_in_executing_mode`

### Rustdoc Convention

Every public item must have:
- Summary line in imperative mood ("Execute a named step", not "Executes")
- Document replay vs execution behavior
- `# Examples` section with compilable example
- `# Errors` section on anything returning `Result`
- No `# Panics` in public API — SDK should never panic

### Project Structure Notes

Files to modify/create:
```
crates/durable-lambda-core/src/
  context.rs          — ADD step method (or delegate to operations/step.rs)
  operations/step.rs  — REPLACE stub with step operation implementation
  operations/mod.rs   — may need additional re-exports
  lib.rs              — ADD any new public type re-exports
```

No other crates need changes for this story.

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules — parameter ordering]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions — checkpoint serialization JSON]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.4]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR3, FR4, FR5, FR8, FR11]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements — NFR1]
- [Source: _bmad-output/implementation-artifacts/1-3-durablebackend-trait-and-replay-engine.md — DurableContext, ReplayEngine, DurableBackend APIs]
- [Source: _bmad-output/implementation-artifacts/1-2-core-types-and-error-foundation.md — DurableError constructors, types]
- [Source: Python SDK — github.com/aws/aws-durable-execution-sdk-python — step operation behavioral reference]

### Previous Story Intelligence (Story 1.3)

- `DurableContext` owns `backend: Arc<dyn DurableBackend>`, `replay_engine: ReplayEngine`, `durable_execution_arn`, `checkpoint_token`
- `ReplayEngine` uses `HashMap<String, Operation>` keyed by operation ID — NOT positional cursor
- `check_result(op_id)` returns `Option<&Operation>` — check operation status to determine if SUCCEEDED or FAILED
- `track_replay(op_id)` marks as visited, transitions to Executing when all completed ops visited
- `generate_operation_id()` returns deterministic blake2b-based IDs
- `DurableBackend::checkpoint()` accepts `Vec<OperationUpdate>` and returns `CheckpointDurableExecutionOutput`
- `CheckpointDurableExecutionOutput` contains updated checkpoint token — must update `DurableContext.checkpoint_token` after each checkpoint call
- Used AWS SDK types directly (`aws_sdk_lambda::types::*`) rather than redefining — follow this pattern
- `backoff_delay` jitter is a no-op bug (`capped/2 + capped/2 == capped`) — noted but not in scope for this story
- 63 tests passing, clippy clean, fmt clean

### Architecture Doc Discrepancies (IMPORTANT)

Inherited from Story 1.3 — always follow Python SDK over architecture doc:
1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Replay tracking**: Uses HashSet of visited operation IDs, NOT simple cursor advancement
3. **Operation ID**: Uses blake2b hash of counter, NOT user-provided step name

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- Step method implemented as `impl DurableContext` block in `operations/step.rs` — keeps context.rs thin
- Return type is `Result<Result<T, E>, DurableError>` — outer Result for SDK errors, inner for user step results
- Two-phase checkpoint: START then SUCCEED/FAIL, matching Python SDK flow
- Double-check pattern implemented: after START checkpoint, merges new_execution_state and re-checks for existing result
- Used AWS SDK types directly: `OperationUpdate::builder()`, `ErrorObject::builder()`, `OperationAction`, `OperationType`
- Checkpoint token updated after every checkpoint response
- Fixed clippy `result_large_err` by boxing `AwsSdk` variant in `DurableError` — `aws_sdk_lambda::Error` was 192+ bytes
- 6 new step tests + MockBackend with checkpoint call capture (Arc<Mutex<Vec>>)
- 50 unit tests + 34 doc tests passing, clippy clean, fmt clean, full workspace builds

### File List

- crates/durable-lambda-core/src/operations/step.rs (modified — replaced stub with step operation + 6 tests)
- crates/durable-lambda-core/src/error.rs (modified — boxed AwsSdk variant to fix clippy result_large_err, added manual From impl)

### Change Log

- 2026-03-14: Story 1.4 implemented — step operation with replay/execute paths, two-phase checkpoint, double-check pattern, 6 tests. Boxed DurableError::AwsSdk for clippy. 50 unit + 34 doc tests passing.
- 2026-03-14: Code review — 0 HIGH, 1 MEDIUM (missing execute-path FAIL test), 1 LOW (misleading synthetic error). 2 action items created. Status → done.
