# Story 1.5: Step Retries & Typed Errors

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to configure retry behavior and return typed errors from steps,
So that my steps handle transient failures and propagate domain-specific errors correctly.

## Acceptance Criteria

1. **Given** a step with retry configuration **When** I call `ctx.step_with_options("charge", StepOptions::new().retries(3), || async { ... })` **Then** the parameter ordering is name, options, closure **And** if the closure fails, it retries up to the configured count before checkpointing the final error (FR9)

2. **Given** a step closure that returns `Err(MyDomainError)` where `MyDomainError` implements `Serialize + DeserializeOwned` **When** the step executes and fails with the domain error **Then** the typed error is serialized and checkpointed as JSON (FR7, FR10)

3. **Given** a `DurableContext` in Replaying mode with a cached typed error for a step **When** the step is replayed **Then** the exact same typed error is deserialized and returned (FR10) **And** the error type `E` requires only `Serialize + DeserializeOwned` bounds, not `std::error::Error`

4. **Given** a step with retries configured **When** all retries are exhausted **Then** the final error is checkpointed **And** subsequent replays return that checkpointed error without re-executing

5. **Given** the `StepOptions` type **When** I examine its API **Then** it provides a builder-style API with `new()` and `retries(n)` methods **And** it is re-exported from `durable-lambda-core`

6. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --doc -p durable-lambda-core` **Then** all rustdoc examples compile and pass

## Tasks / Subtasks

- [x] Task 1: Define the SDK-level `StepOptions` struct (AC: #1, #5)
  - [x] 1.1: Create the `StepOptions` struct in `types.rs` with a `retries: Option<u32>` field and a `backoff_seconds: Option<i32>` field
  - [x] 1.2: Implement `StepOptions::new()` returning default (no retries configured)
  - [x] 1.3: Implement `StepOptions::retries(self, count: u32) -> Self` builder method
  - [x] 1.4: Implement `StepOptions::backoff_seconds(self, seconds: i32) -> Self` builder method
  - [x] 1.5: Implement `Default` for `StepOptions` (all fields `None`) — via `#[derive(Default)]`
  - [x] 1.6: Add rustdoc with `# Examples` on `StepOptions` and all public methods
  - [x] 1.7: Re-export `StepOptions` from `lib.rs`

- [x] Task 2: Implement `step_with_options` method on `DurableContext` (AC: #1, #2, #4)
  - [x] 2.1: Add `step_with_options` method in `operations/step.rs` with signature matching spec
  - [x] 2.2: Retry flow: compare `current_attempt` against `options.retries`, RETRY if remaining, FAIL if exhausted
  - [x] 2.3: RETRY path: `OperationUpdate` with `OperationAction::Retry` + `aws_sdk_lambda::types::StepOptions` via `.step_options()`
  - [x] 2.4: Return `DurableError::StepRetryScheduled` after RETRY checkpoint
  - [x] 2.5: Handle PENDING/READY/STARTED status as re-execution (skip START, re-run closure)
  - [x] 2.6: Read `StepDetails.attempt()` for current attempt tracking
  - [x] 2.7: Refactored `step` to delegate to `step_with_options(name, StepOptions::default(), f)`

- [x] Task 3: Handle the retry lifecycle in the execute path (AC: #1, #4)
  - [x] 3.1: PENDING/READY/STARTED detected via `replay_engine().operations().get(&op_id)`, skip START checkpoint on re-execution
  - [x] 3.2: ~~Send START on re-execution~~ **Deviation:** Do NOT send START on re-execution per Python SDK pattern — START was already sent on prior invocation
  - [x] 3.3: `next_attempt_delay_seconds` from `options.get_backoff_seconds()`, defaults to 0

- [x] Task 4: Add `DurableError` variant for retry scheduling (AC: #1)
  - [x] 4.1: Added `StepRetryScheduled { operation_name: String }` variant with `#[non_exhaustive]`
  - [x] 4.2: Added `DurableError::step_retry_scheduled(operation_name)` constructor
  - [x] 4.3: Added rustdoc on variant and constructor with `# Examples`

- [x] Task 5: Verify typed error handling end-to-end (AC: #2, #3)
  - [x] 5.1: Confirmed `step` handles typed errors with `E: Serialize + DeserializeOwned + Send` (from Story 1.4)
  - [x] 5.2: Confirmed `step_with_options` has same typed error behavior (delegates to same logic)
  - [x] 5.3: Test `test_step_with_options_typed_error_roundtrip` with multi-variant `DomainError` enum

- [x] Task 6: Write comprehensive tests (AC: #1, #2, #3, #4, #5, #6)
  - [x] 6.1: `test_step_with_options_basic_success` — START + SUCCEED, correct result
  - [x] 6.2: `test_step_with_options_retry_on_failure` — START + RETRY, correct step_options, StepRetryScheduled returned
  - [x] 6.3: `test_step_with_options_retry_exhaustion` — PENDING at attempt 4, retries(3), FAIL checkpoint only
  - [x] 6.4: `test_step_with_options_replay_succeeded_with_retries` — cached SUCCEEDED, no closure, no checkpoints
  - [x] 6.5: `test_step_with_options_typed_error_roundtrip` — cached FAILED with domain error enum, correct deserialization
  - [x] 6.6: `test_step_options_builder` — new(), retries(), backoff_seconds(), Default, chaining
  - [x] 6.7: `test_step_with_options_typed_error_roundtrip` — multi-variant DomainError enum
  - [x] 6.8: `test_step_backward_compatibility` — `step` still works after refactoring
  - [x] 6.9: Rustdoc examples on `step_with_options` and `StepOptions`

- [x] Task 7: Verify all checks pass (AC: #6)
  - [x] 7.1: Run `cargo test --doc -p durable-lambda-core` — 40 doc tests pass
  - [x] 7.2: Run `cargo test -p durable-lambda-core` — 57 unit tests pass
  - [x] 7.3: Run `cargo clippy -p durable-lambda-core -- -D warnings` — no warnings
  - [x] 7.4: Run `cargo fmt --check` — formatting passes
  - [x] 7.5: Run `cargo build --workspace` — full workspace builds

## Dev Notes

### Critical Architecture Constraints

- **Retries are SERVER-SIDE, not client-side**: This is the single most important thing to understand. The SDK does NOT loop and retry locally. Instead, the SDK sends a `RETRY` checkpoint to the server with a delay, the function exits, and the server re-invokes the Lambda after the delay. On the next invocation, the step is found in a PENDING/READY state, and the SDK re-executes the closure. This is fundamentally different from typical client-side retry patterns.
- **Parameter ordering convention**: name first, options second, closure last. `ctx.step_with_options("name", options, || async { ... })`.
- **lib.rs = re-exports only**: Zero logic in lib.rs. Add `StepOptions` to re-exports.
- **Constructor methods for DurableError**: Use static methods, never raw struct construction.
- **User step errors require only `Serialize + DeserializeOwned`**: NOT `std::error::Error`. This is an explicit architecture decision to minimize trait bounds. Simple enums and strings work without thiserror.

### Python SDK Retry Behavior (Reference Implementation)

The Python SDK step retry flow works as follows:

```
FIRST ATTEMPT (execute mode):
1. generate_operation_id() → op_id
2. check_result_status(op_id) → not found
3. checkpoint(START, sync=true)
4. double-check for existing result
5. execute closure → Err(e)
6. If retries configured and attempts remain:
   checkpoint(RETRY action, step_options={next_attempt_delay_seconds: N})
   → function returns/exits
   → server waits N seconds, then re-invokes the Lambda

SUBSEQUENT ATTEMPT (after server re-invocation):
1. Load execution state → operation exists with status=PENDING or READY
2. generate_operation_id() → same op_id (deterministic)
3. check_result_status(op_id) → found, but status is PENDING/READY (not completed)
4. Treat as "needs execution" — re-run the closure
5. If success: checkpoint(SUCCEED)
6. If fail and more retries: checkpoint(RETRY) → exit again
7. If fail and no more retries: checkpoint(FAIL)
```

### AWS SDK Types Available

The following AWS SDK types are directly available and MUST be used (do not redefine):

- **`aws_sdk_lambda::types::StepOptions`** — has `next_attempt_delay_seconds: Option<i32>`. Use this on `OperationUpdate` via `.step_options()` builder method. This is the AWS wire type for retry configuration sent during RETRY checkpoints.
- **`aws_sdk_lambda::types::OperationAction::Retry`** — the RETRY action variant exists in the enum.
- **`aws_sdk_lambda::types::OperationStatus::Pending`** — status for a step awaiting retry.
- **`aws_sdk_lambda::types::OperationStatus::Ready`** — status for a step ready to be re-executed.
- **`aws_sdk_lambda::types::StepDetails`** — has `attempt: i32` for tracking current attempt number.
- **`aws_sdk_lambda::types::OperationUpdate`** builder has `.step_options(StepOptions)` method.

### SDK-Level StepOptions vs AWS StepOptions

There are TWO different `StepOptions`:

1. **SDK-level `StepOptions`** (our type, in `types.rs`): User-facing builder with `retries(n)` and `backoff_seconds(n)`. This is what the user passes to `step_with_options()`.
2. **`aws_sdk_lambda::types::StepOptions`**: Wire-level type with `next_attempt_delay_seconds`. This is what gets attached to the `OperationUpdate` during a RETRY checkpoint.

The SDK-level `StepOptions` is translated to the AWS-level `StepOptions` internally. Users never construct `aws_sdk_lambda::types::StepOptions` directly.

```rust
/// SDK-level step configuration (in types.rs).
#[derive(Debug, Clone, Default)]
pub struct StepOptions {
    retries: Option<u32>,
    backoff_seconds: Option<i32>,
}

impl StepOptions {
    pub fn new() -> Self { Self::default() }
    pub fn retries(mut self, count: u32) -> Self { self.retries = Some(count); self }
    pub fn backoff_seconds(mut self, seconds: i32) -> Self { self.backoff_seconds = Some(seconds); self }
}
```

### How step_with_options Integrates with Existing step Method

The existing `step` method should be refactored to delegate to `step_with_options`:

```rust
pub async fn step<T, E, F, Fut>(&mut self, name: &str, f: F) -> Result<Result<T, E>, DurableError>
where
    T: Serialize + DeserializeOwned + Send,
    E: Serialize + DeserializeOwned + Send,
    F: FnOnce() -> Fut + Send,
    Fut: Future<Output = Result<T, E>> + Send,
{
    self.step_with_options(name, StepOptions::default(), f).await
}
```

This ensures `step` remains fully backward compatible while sharing all logic with `step_with_options`.

### step_with_options Method Signature

```rust
pub async fn step_with_options<T, E, F, Fut>(
    &mut self,
    name: &str,
    options: StepOptions,
    f: F,
) -> Result<Result<T, E>, DurableError>
where
    T: Serialize + DeserializeOwned + Send,
    E: Serialize + DeserializeOwned + Send,
    F: FnOnce() -> Fut + Send,
    Fut: Future<Output = Result<T, E>> + Send,
{
    // Same as step, but on Err path:
    // 1. Check if retries are configured and attempts remain
    // 2. If yes: send RETRY checkpoint with step_options, return StepRetryScheduled error
    // 3. If no: send FAIL checkpoint as before
}
```

### Retry Execute Path — Detailed Flow

```
step_with_options("charge", StepOptions::new().retries(3), || async { ... }):

1. generate_operation_id() → op_id
2. check_result(op_id):
   a. If found with SUCCEEDED → return Ok(cached_value), track_replay
   b. If found with FAILED → return Err(cached_error), track_replay
   c. If found with PENDING/READY → step needs re-execution (see step 4b below)
   d. If not found → first attempt (see step 3)

3. FIRST ATTEMPT (not found):
   a. Checkpoint START (sync), update token, merge state, double-check
   b. Execute closure
   c. If Ok → checkpoint SUCCEED, return Ok(value)
   d. If Err and retries configured and attempt < max_retries:
      - Build OperationUpdate with Action=RETRY
      - Attach aws_sdk_lambda::types::StepOptions { next_attempt_delay_seconds }
      - Checkpoint RETRY (sync), update token
      - Return Err(DurableError::StepRetryScheduled { operation_name })
   e. If Err and no retries (or retries exhausted):
      - Checkpoint FAIL, return Ok(Err(user_error))

4. RE-EXECUTION (found with PENDING/READY status):
   a. The operation exists but is not completed — server re-invoked after delay
   b. Read current attempt from StepDetails.attempt()
   c. Execute closure
   d. Same branching as 3c/3d/3e, but with updated attempt count
```

**IMPORTANT**: When re-executing after retry, the START checkpoint was already sent on a previous invocation. Do NOT send another START. Just re-execute the closure and checkpoint SUCCEED/FAIL/RETRY.

### Handling the Retry Signal

When `step_with_options` returns `Err(DurableError::StepRetryScheduled { .. })`, the handler function should propagate this error upward, causing the Lambda to exit. The durable execution server will then re-invoke the function after the configured delay.

The handler code pattern would be:
```rust
// In the user's handler:
let result = ctx.step_with_options("charge", StepOptions::new().retries(3), || async {
    charge_payment().await
}).await?;   // ? propagates StepRetryScheduled, exiting the handler
```

### Typed Error Handling

Story 1.4 already implements typed error handling correctly:
- `E: Serialize + DeserializeOwned + Send` (NOT `E: std::error::Error`)
- Error serialized via `serde_json::to_string` into `ErrorObject.error_data`
- Error type name stored in `ErrorObject.error_type` via `std::any::type_name::<E>()`
- Error deserialized via `serde_json::from_str` from `StepDetails.error().error_data()`

This story confirms and tests this behavior but does NOT need to change the error handling logic — it already works. The only addition is that `step_with_options` has the same typed error behavior.

### What Exists vs What Needs to Be Added

**Already exists (from Story 1.4):**
- `DurableContext::step()` method with full execute/replay paths
- `extract_step_result()` helper function
- Typed error serialization/deserialization via `ErrorObject`
- Two-phase checkpoint (START → SUCCEED/FAIL)
- Double-check pattern after START
- MockBackend in test module with checkpoint call capture
- 6 step tests + test infrastructure

**Needs to be added (this story):**
- `StepOptions` struct in `types.rs` (SDK-level, user-facing)
- `DurableContext::step_with_options()` method in `operations/step.rs`
- RETRY checkpoint path using `OperationAction::Retry` + `aws_sdk_lambda::types::StepOptions`
- `DurableError::StepRetryScheduled` variant + constructor
- Handling of PENDING/READY operation status (re-execution path)
- Refactoring `step()` to delegate to `step_with_options()`
- Re-export of `StepOptions` in `lib.rs`
- New tests for retry scenarios
- Rustdoc on all new public items

### Testing Approach

- Extend the existing `MockBackend` in `operations/step.rs` tests — it already captures checkpoint calls via `Arc<Mutex<Vec<CheckpointCall>>>`.
- For retry tests, the MockBackend needs to return operations in PENDING/READY status to simulate the re-invocation scenario.
- Test naming: `test_step_{behavior}_{condition}` e.g., `test_step_with_options_retries_on_failure`.
- Verify checkpoint calls contain correct `OperationAction::Retry` and `step_options` fields.
- Test `StepOptions` builder independently (unit test in `types.rs` tests module).

### Previous Story Intelligence (Story 1.4)

- Step method implemented as `impl DurableContext` block in `operations/step.rs` — keeps context.rs thin. Follow this pattern for `step_with_options`.
- Return type is `Result<Result<T, E>, DurableError>` — outer Result for SDK errors, inner for user step results.
- Two-phase checkpoint: START then SUCCEED/FAIL, matching Python SDK flow.
- Double-check pattern: after START checkpoint, merges `new_execution_state` and re-checks.
- Used AWS SDK types directly: `OperationUpdate::builder()`, `ErrorObject::builder()`, `OperationAction`, `OperationType`.
- Checkpoint token updated after every checkpoint response.
- Fixed clippy `result_large_err` by boxing `AwsSdk` variant — keep new `DurableError` variants small.
- `DurableError` is `#[non_exhaustive]` — safe to add new variants.
- 50 unit tests + 34 doc tests passing, clippy clean, fmt clean.
- Review follow-ups from 1.4: (1) No test for execute-path FAIL checkpoint [MEDIUM], (2) Synthetic serde error for missing step_details is misleading [LOW]. Consider addressing these while working on this story.

### File Structure Notes

Files to modify:
```
crates/durable-lambda-core/src/
  types.rs            — ADD StepOptions struct with builder methods
  operations/step.rs  — ADD step_with_options method, REFACTOR step to delegate
  error.rs            — ADD StepRetryScheduled variant + constructor
  lib.rs              — ADD StepOptions to re-exports
```

No other crates need changes for this story.

### Architecture Doc Discrepancies (IMPORTANT)

Inherited from Story 1.3/1.4 — always follow Python SDK over architecture doc:
1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Replay tracking**: Uses HashSet of visited operation IDs, NOT simple cursor advancement
3. **Operation ID**: Uses blake2b hash of counter, NOT user-provided step name

New for this story:
4. **Architecture doc says `StepOptions::new().retries(3)`** — this is our SDK-level type, NOT the AWS SDK type. The AWS SDK has `aws_sdk_lambda::types::StepOptions` with only `next_attempt_delay_seconds`. Do not confuse the two. Our SDK-level `StepOptions` wraps user-friendly configuration and translates it to the AWS wire type internally.
5. **Architecture doc lists StepOptions as a gap** — "Three minor implementation-detail gaps (StepOptions struct, retry strategy specifics, BatchResult internals) — all resolved by studying the Python SDK source during implementation." This story resolves the StepOptions gap.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.5 — acceptance criteria]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR7, FR9, FR10]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns — parameter ordering, StepOptions pattern]
- [Source: _bmad-output/planning-artifacts/architecture.md#Error Strategy — serde-only bounds for user step errors]
- [Source: _bmad-output/planning-artifacts/architecture.md#Gap Analysis — StepOptions gap noted]
- [Source: _bmad-output/implementation-artifacts/1-4-step-operation-implementation.md — step method, MockBackend, test patterns]
- [Source: aws_sdk_lambda::types::StepOptions — next_attempt_delay_seconds field]
- [Source: aws_sdk_lambda::types::OperationAction::Retry — RETRY action variant]
- [Source: aws_sdk_lambda::types::OperationUpdate — step_options field on builder]
- [Source: aws_sdk_lambda::types::StepDetails — attempt field for tracking retry count]
- [Source: Python SDK — github.com/aws/aws-durable-execution-sdk-python — retry behavioral reference]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- `StepOptions` struct in types.rs with builder pattern: `new()`, `retries()`, `backoff_seconds()`, `Default` derive, getter methods
- `step_with_options` implemented in operations/step.rs as `impl DurableContext` block
- `step` refactored to delegate to `step_with_options(name, StepOptions::default(), f)` — full backward compatibility
- RETRY path: builds `OperationUpdate` with `OperationAction::Retry` + `aws_sdk_lambda::types::StepOptions` for delay config
- Re-execution path: detects PENDING/READY/STARTED operations, skips START checkpoint, re-executes closure
- `DurableError::StepRetryScheduled` variant added with `#[non_exhaustive]` and constructor method
- `StepOptions` re-exported from lib.rs
- 7 new tests (total 13 step tests, 57 unit tests overall + 40 doc tests)
- Clippy clean, fmt clean, workspace builds

### File List

- crates/durable-lambda-core/src/types.rs (modified — added StepOptions struct with builder)
- crates/durable-lambda-core/src/error.rs (modified — added StepRetryScheduled variant + constructor)
- crates/durable-lambda-core/src/operations/step.rs (modified — added step_with_options, refactored step to delegate, 7 new tests)
- crates/durable-lambda-core/src/lib.rs (modified — added StepOptions to re-exports)

### Change Log

- 2026-03-14: Story 1.5 implemented — StepOptions, step_with_options with server-side retry, StepRetryScheduled error, PENDING/READY re-execution. 57 unit + 40 doc tests passing.
- 2026-03-14: Code review — clean pass, 0 issues found. Status → done.
