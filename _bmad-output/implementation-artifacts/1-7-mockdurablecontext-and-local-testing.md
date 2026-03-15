# Story 1.7: MockDurableContext & Local Testing

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want a MockDurableContext with pre-loaded step results for local testing,
So that I can write and run tests without AWS credentials or deployment.

## Acceptance Criteria

1. **Given** the durable-lambda-testing crate **When** I create a MockDurableContext **Then** I can pre-load step results as a sequence of JSON values (FR37) **And** the mock uses a MockBackend that implements DurableBackend without any AWS dependency at runtime

2. **Given** a MockDurableContext with pre-loaded results for steps "validate" and "charge" **When** my handler calls `ctx.step("validate", ...)` and `ctx.step("charge", ...)` **Then** the pre-loaded results are returned in order without executing the closures (FR37) **And** no AWS credentials or network access are required (FR38)

3. **Given** a test using MockDurableContext **When** I run `cargo test` **Then** the test executes locally in milliseconds **And** it verifies the handler's logic with deterministic, pre-loaded data

4. **Given** the durable-lambda-testing crate **When** I examine its structure **Then** it contains mock_backend.rs (MockBackend implementing DurableBackend), mock_context.rs (MockDurableContext), assertions.rs (test helpers), and prelude.rs **And** it depends only on durable-lambda-core, not on aws-sdk-lambda at runtime

5. **Given** the testing prelude **When** I import `use durable_lambda_testing::prelude::*` **Then** I have access to MockDurableContext and all test assertion helpers

6. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --doc -p durable-lambda-testing` **Then** all rustdoc examples compile and pass

## Tasks / Subtasks

- [x] Task 1: Implement `MockBackend` in `mock_backend.rs` (AC: #1, #4)
  - [x] 1.1: Define `MockBackend` struct that implements `DurableBackend` trait
  - [x] 1.2: Operations are passed via DurableContext::new initial_operations, not stored on MockBackend
  - [x] 1.3: `checkpoint()` method records all checkpoint calls in `Arc<Mutex<Vec<CheckpointCall>>>` (tokio Mutex) and returns configurable checkpoint token
  - [x] 1.4: `get_execution_state()` returns empty results
  - [x] 1.5: `CheckpointCall` struct with public fields: arn, checkpoint_token, updates
  - [x] 1.6: `MockBackend::new(token) -> (Self, Arc<Mutex<Vec<CheckpointCall>>>)` factory
  - [x] 1.7: Rustdoc with `# Examples` on MockBackend, CheckpointCall, and new()

- [x] Task 2: Implement `MockDurableContext` builder in `mock_context.rs` (AC: #1, #2, #3)
  - [x] 2.1: `MockDurableContext` builder that creates pre-configured `DurableContext` with `MockBackend`
  - [x] 2.2: `MockDurableContext::new()` and `Default` impl
  - [x] 2.3: `with_step_result(name, result_json)` — generates deterministic operation IDs via `OperationIdGenerator`, creates SUCCEEDED Operation
  - [x] 2.4: `with_step_error(name, error_type, error_json)` — creates FAILED Operation with ErrorObject
  - [x] 2.5: `build()` async — creates DurableContext with MockBackend, returns `(DurableContext, Arc<Mutex<Vec<CheckpointCall>>>)`
  - [x] 2.6: Rustdoc with `# Examples` on all methods showing complete test flow

- [x] Task 3: Implement assertion helpers in `assertions.rs` (AC: #5)
  - [x] 3.1: `assert_checkpoint_count(calls, expected)` — asserts checkpoint call count
  - [x] 3.2: `assert_no_checkpoints(calls)` — asserts zero checkpoints (delegates to count)
  - [x] 3.3: Rustdoc with `# Examples` on both helpers

- [x] Task 4: Set up prelude re-exports in `prelude.rs` (AC: #5)
  - [x] 4.1: Re-export `MockDurableContext`
  - [x] 4.2: Re-export `MockBackend` and `CheckpointCall`
  - [x] 4.3: Re-export assertion helpers
  - [x] 4.4: Re-export core types: `DurableContext`, `DurableError`, `StepOptions`, `ExecutionMode`
  - [x] 4.5: Rustdoc on prelude module

- [x] Task 5: Update `lib.rs` with module declarations and top-level re-exports (AC: #4)
  - [x] 5.1: Module declarations for all 4 modules
  - [x] 5.2: Top-level re-exports of MockBackend, CheckpointCall, MockDurableContext
  - [x] 5.3: Crate-level rustdoc with usage example

- [x] Task 6: Update `Cargo.toml` dependencies (AC: #4)
  - [x] 6.1: Verified `durable-lambda-core` path dependency
  - [x] 6.2: Added `aws-sdk-lambda` workspace dependency
  - [x] 6.3: Added `aws-smithy-types` workspace dependency
  - [x] 6.4: Added `async-trait` workspace dependency
  - [x] 6.5: Verified `serde`, `serde_json`, `tokio` workspace dependencies

- [x] Task 7: Write tests (AC: #2, #3, #6)
  - [x] 7.1: `test_mock_context_replays_step_result` — verifies cached result returned, closure NOT executed (AtomicBool flag)
  - [x] 7.2: `test_mock_context_replays_multiple_steps` — two steps in sequence, both replayed
  - [x] 7.3: `test_mock_context_replays_step_error` — typed error replayed correctly
  - [x] 7.4: Assertion helpers tested implicitly via all tests using assert_no_checkpoints pattern
  - [x] 7.5: `test_mock_context_no_aws_credentials_needed` — proves no AWS needed
  - [x] 7.6: 12 doc tests compile and pass
  - [x] 7.7: Workspace builds clean

- [x] Task 8: Verify all checks pass (AC: #6)
  - [x] 8.1: `cargo test --doc -p durable-lambda-testing` — 12 doc tests pass
  - [x] 8.2: `cargo test -p durable-lambda-testing` — 6 unit tests pass
  - [x] 8.3: `cargo clippy -p durable-lambda-testing -- -D warnings` — no warnings
  - [x] 8.4: `cargo fmt --check` — formatting passes
  - [x] 8.5: `cargo build --workspace` — full workspace builds

## Dev Notes

### Critical Architecture Constraints

- **MockBackend implements DurableBackend**: The `DurableBackend` trait is the I/O boundary. `MockBackend` replaces `RealBackend` — no AWS calls, no credentials needed.
- **lib.rs = re-exports only**: Zero logic in lib.rs. Only `pub mod` and `pub use` statements.
- **Testing crate depends ONLY on durable-lambda-core**: Plus serde/tokio for types. The architecture doc says "does not depend on aws-sdk-lambda at runtime" but practically `aws-sdk-lambda` IS needed for the `Operation`, `OperationUpdate`, `StepDetails` etc. types used to construct mock history. This is the same pattern as the closure crate — the types come through the AWS SDK but MockBackend never makes any AWS API calls.
- **Deterministic operation IDs**: Core uses blake2b hashing with a counter. MockDurableContext MUST generate the same operation IDs as DurableContext would, so pre-loaded operations are found during replay. Use `OperationIdGenerator` from core to generate IDs when building mock history.

### How MockDurableContext Works — The Key Pattern

The mock creates a DurableContext in **Replaying** mode by pre-loading completed operations:

```
1. User calls MockDurableContext::new()
       .with_step_result("validate", r#"{"valid": true}"#)
       .with_step_result("charge", r#"{"charged": true}"#)
       .build()

2. Build internally:
   a. Create OperationIdGenerator (no parent_id)
   b. For each with_step_result call:
      - Generate operation ID using generator.next_id() (blake2b hash of counter)
      - Create an Operation with that ID, type=Step, status=Succeeded, step_details.result=json
   c. Create MockBackend with empty checkpoint_calls
   d. Create DurableContext::new(mock_backend, arn, token, operations, None)
   e. DurableContext sees completed operations → starts in Replaying mode

3. When the handler calls ctx.step("validate", || async { ... }):
   a. DurableContext generates operation ID (same blake2b hash — deterministic!)
   b. Finds the pre-loaded operation with matching ID → replay path
   c. Returns cached result WITHOUT executing the closure
   d. Tracks replay, advances to next operation

4. After all pre-loaded operations are replayed, mode transitions to Executing
   - Any additional ctx.step() calls will execute their closures and checkpoint
```

### DurableBackend Trait (What MockBackend Must Implement)

```rust
#[async_trait::async_trait]
pub trait DurableBackend: Send + Sync {
    async fn checkpoint(
        &self,
        arn: &str,
        checkpoint_token: &str,
        updates: Vec<OperationUpdate>,
        client_token: Option<&str>,
    ) -> Result<CheckpointDurableExecutionOutput, DurableError>;

    async fn get_execution_state(
        &self,
        arn: &str,
        checkpoint_token: &str,
        next_marker: &str,
        max_items: i32,
    ) -> Result<GetDurableExecutionStateOutput, DurableError>;
}
```

### Existing MockBackend Patterns (From Core Tests)

Two MockBackend implementations already exist in core's test modules:

**1. step.rs tests (operations/step.rs:389-434):**
- Uses `Arc<Mutex<Vec<CheckpointCall>>>` (tokio Mutex) for recording checkpoint calls
- Returns configurable checkpoint_token
- `get_execution_state` returns empty
- Factory: `MockBackend::new(token) -> (Self, Arc<Mutex<Vec<CheckpointCall>>>)`

**2. context.rs tests (context.rs:242-279):**
- Configurable pages for pagination testing
- Stores pages as `Vec<(Vec<Operation>, Option<String>)>`
- Simpler — doesn't record checkpoint calls

The testing crate's `MockBackend` should combine the best of both:
- Record checkpoint calls (like step.rs pattern)
- Support pre-loaded operations (for replay mode)
- Use `tokio::sync::Mutex` (not `std::sync::Mutex`) since DurableBackend methods are async

### OperationIdGenerator — Critical for Mock Correctness

The `OperationIdGenerator` in `crates/durable-lambda-core/src/operation_id.rs`:
- Generates deterministic IDs using blake2b hash of `"{counter}"` string
- Counter starts at 0, increments on each `next_id()` call
- Returns 64-character hex string
- `OperationIdGenerator` is re-exported from `durable-lambda-core::OperationIdGenerator`

For `MockDurableContext::with_step_result()` to work, the mock MUST generate operation IDs using the same algorithm. Since core re-exports `OperationIdGenerator`, use it directly:

```rust
let mut id_gen = OperationIdGenerator::new(None); // no parent
let op_id_1 = id_gen.next_id(); // corresponds to first ctx.step() call
let op_id_2 = id_gen.next_id(); // corresponds to second ctx.step() call
```

### What Exists vs What Needs to Be Added

**Already exists (from Stories 1.1–1.6):**
- `DurableBackend` trait — fully defined
- `DurableContext::new()` — accepts Vec<Operation> and MockBackend
- `OperationIdGenerator` — deterministic blake2b ID generation
- `ReplayEngine` — mode transitions based on completed operations
- Stub files in durable-lambda-testing: lib.rs, mock_backend.rs, mock_context.rs, assertions.rs, prelude.rs (headers only)
- Cargo.toml with durable-lambda-core, serde, serde_json, tokio dependencies
- Multiple test MockBackend implementations in core proving the pattern works

**Needs to be added (this story):**
- `MockBackend` struct in `mock_backend.rs` — public, reusable implementation
- `CheckpointCall` struct — public, for test assertions
- `MockDurableContext` builder in `mock_context.rs` — ergonomic API for pre-loading results
- Assertion helpers in `assertions.rs`
- Prelude re-exports in `prelude.rs`
- Top-level re-exports in `lib.rs`
- Additional Cargo.toml deps: aws-sdk-lambda, aws-smithy-types, async-trait
- Unit tests and rustdoc

### Architecture Doc Discrepancies (IMPORTANT — Inherited)

From previous stories — always follow Python SDK / actual implementation over architecture doc:
1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Replay tracking**: Uses HashSet of visited operation IDs, NOT simple cursor advancement
3. **Operation ID**: Uses blake2b hash of counter, NOT user-provided step name

New for this story:
4. **Architecture says "does not depend on aws-sdk-lambda at runtime"**: Practically, `aws-sdk-lambda` IS needed at compile time for types (Operation, StepDetails, etc.). The distinction is that MockBackend never makes AWS API calls — it's a pure in-memory mock.
5. **FR39 (operation sequence verification)** is mapped to Epic 5, not this story. This story covers FR37 and FR38 only. Do NOT implement full operation sequence verification — just provide basic checkpoint-count assertions.

### Previous Story Intelligence (Story 1.6)

- Handler takes owned `ClosureContext` (not `&mut`) to avoid async lifetime issues
- `aws-sdk-lambda` and `aws-smithy-types` needed as direct dependencies for Operation type construction
- AWS SDK builder `.build()` sometimes returns Result, sometimes direct type — check each
- `CheckpointDurableExecutionOutput::builder().build()` returns the type directly (no unwrap)
- `GetDurableExecutionStateOutput::builder().build()` returns Result (needs unwrap/expect)
- `Operation::builder().build()` returns Result (needs unwrap)
- `StepDetails::builder().build()` returns the type directly (no unwrap)
- Clippy enforces: no redundant closures, proper formatting
- Review flagged: unused dependencies should be removed, imports should be at top of module

### Testing Approach

- Tests should verify MockDurableContext creates contexts that replay correctly
- Test the full flow: build mock → call step → verify cached result returned → verify closure NOT executed
- Test error replay: build mock with error → call step → verify error returned
- Use a side-effect flag (e.g., `Arc<AtomicBool>`) in test closures to verify they're NOT executed during replay
- Test naming: `test_{component}_{behavior}_{condition}` e.g., `test_mock_context_replays_step_results`

### User-Facing Example (Target End State)

```rust
use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_order_workflow() {
    let (mut ctx, calls) = MockDurableContext::new()
        .with_step_result("validate_order", r#"{"valid": true}"#)
        .with_step_result("charge_payment", r#"{"charged": true}"#)
        .build()
        .await;

    // Handler logic — closures are NOT executed, cached results returned
    let validate: Result<serde_json::Value, String> = ctx.step("validate_order", || async {
        panic!("should not execute during replay");
    }).await.unwrap();

    assert_eq!(validate.unwrap(), serde_json::json!({"valid": true}));

    let charge: Result<serde_json::Value, String> = ctx.step("charge_payment", || async {
        panic!("should not execute during replay");
    }).await.unwrap();

    assert_eq!(charge.unwrap(), serde_json::json!({"charged": true}));

    // Verify no checkpoints made (pure replay)
    assert_no_checkpoints(&calls).await;
}
```

### Project Structure Notes

- All files are in `crates/durable-lambda-testing/src/`
- No other crates need changes for this story
- The testing crate is consumed by user tests via `[dev-dependencies]`

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.7 — acceptance criteria]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR37, FR38]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Engine Architecture — DurableBackend trait, MockBackend]
- [Source: _bmad-output/planning-artifacts/architecture.md#Architectural Boundaries — testing crate dependencies]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure — durable-lambda-testing files]
- [Source: _bmad-output/implementation-artifacts/1-6-closure-native-api-approach-and-lambda-integration.md — AWS SDK type patterns, builder quirks]
- [Source: crates/durable-lambda-core/src/backend.rs — DurableBackend trait definition]
- [Source: crates/durable-lambda-core/src/context.rs — DurableContext::new, TestBackend pattern]
- [Source: crates/durable-lambda-core/src/operations/step.rs:389-434 — MockBackend with checkpoint recording]
- [Source: crates/durable-lambda-core/src/operation_id.rs — OperationIdGenerator, blake2b deterministic IDs]
- [Source: crates/durable-lambda-core/src/replay.rs — ReplayEngine mode transitions]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- `MockBackend` in mock_backend.rs — implements DurableBackend with checkpoint call recording via `Arc<tokio::sync::Mutex<Vec<CheckpointCall>>>`
- `CheckpointCall` struct with public fields (arn, checkpoint_token, updates) for test assertions
- `MockDurableContext` builder in mock_context.rs — `new()`, `with_step_result()`, `with_step_error()`, `build()` async
- Deterministic operation IDs via `OperationIdGenerator::new(None)` — nth `with_step_result` maps to nth `ctx.step()` call
- `with_step_error()` takes error_type and error_json, constructs ErrorObject for FAILED operations
- `build()` creates DurableContext with mock ARN/token, returns `(DurableContext, Arc<Mutex<Vec<CheckpointCall>>>)`
- Assertion helpers: `assert_checkpoint_count()` and `assert_no_checkpoints()` (async, takes `&Arc<Mutex<...>>`)
- Prelude re-exports: MockDurableContext, MockBackend, CheckpointCall, assertion helpers, core types
- 6 unit tests proving replay, error replay, mode detection, no-AWS-credentials
- 12 doc tests, clippy clean, fmt clean, workspace builds

### File List

- crates/durable-lambda-testing/Cargo.toml (modified — added aws-sdk-lambda, aws-smithy-types, async-trait deps)
- crates/durable-lambda-testing/src/mock_backend.rs (rewritten — MockBackend, CheckpointCall, DurableBackend impl)
- crates/durable-lambda-testing/src/mock_context.rs (rewritten — MockDurableContext builder, 6 unit tests)
- crates/durable-lambda-testing/src/assertions.rs (rewritten — assert_checkpoint_count, assert_no_checkpoints)
- crates/durable-lambda-testing/src/prelude.rs (rewritten — re-exports of testing types and core types)
- crates/durable-lambda-testing/src/lib.rs (rewritten — module declarations, top-level re-exports, crate rustdoc)

### Change Log

- 2026-03-14: Story 1.7 implemented — MockBackend, MockDurableContext builder, assertion helpers, prelude re-exports. 6 unit + 12 doc tests passing. Clippy clean, fmt clean, workspace builds.
