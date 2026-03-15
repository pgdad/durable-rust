# Story 3.1: Parallel Operation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to execute multiple branches concurrently within a single Lambda invocation with configurable completion criteria,
So that I can fan out work (e.g., notify multiple reviewers, check multiple services) and control when the parallel block completes.

## Acceptance Criteria

1. **Given** a DurableContext in Executing mode **When** I call `ctx.parallel("fan_out", branches, options)` with a collection of branch closures **Then** each branch gets its own child DurableContext with an isolated checkpoint namespace **And** all branches execute concurrently as `tokio::spawn` tasks (FR19, FR22)

2. **Given** a parallel operation **When** I use default completion criteria (all successful) **Then** the parallel block waits for all branches to complete **And** returns a `BatchResult<T>` containing each branch's result (FR20)

3. **Given** parallel branches executing concurrently **When** each branch executes durable operations (steps, waits, etc.) **Then** each branch operates in its own checkpoint namespace via child context with `parent_id` = branch operation ID (FR21) **And** branch checkpoint keys do not collide with each other or the parent context

4. **Given** branch closures passed to parallel **When** they are compiled **Then** they satisfy `Send + 'static` bounds required by `tokio::spawn` (FR22) **And** the API patterns make satisfying these bounds natural (owned child context, no shared mutable state)

5. **Given** a DurableContext in Replaying mode with a completed parallel operation in history (SUCCEEDED with payload) **When** the replay engine encounters the parallel entry **Then** the cached `BatchResult` is deserialized from the outer context operation's payload **And** no branches are re-executed **And** the cursor advances past all parallel-related entries

6. **Given** the parallel operation **When** I examine the checkpoints sent **Then** one Context/START is sent for the outer parallel block (sub_type: "Parallel") **And** one Context/START + Context/SUCCEED per branch (sub_type: "ParallelBranch") **And** one Context/SUCCEED for the outer block with the serialized BatchResult

7. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --workspace` **Then** all tests pass including new parallel operation tests **And** all doc tests compile

## Tasks / Subtasks

- [x] Task 1: Add parallel types to `types.rs` (AC: #2, #4)
  - [x] 1.1: `ParallelOptions` struct with builder pattern (default: all successful completion)
  - [x] 1.2: `CompletionCriteria` enum — `AllSuccessful` (default), for future extensibility
  - [x] 1.3: `BatchResult<T>` struct with `results: Vec<BatchItem<T>>` and `completion_reason: CompletionReason`
  - [x] 1.4: `BatchItem<T>` struct with `index: usize`, `status: BatchItemStatus`, `result: Option<T>`, `error: Option<String>`
  - [x] 1.5: `BatchItemStatus` enum — `Succeeded`, `Failed`, `Started`
  - [x] 1.6: `CompletionReason` enum — `AllCompleted`, `MinSuccessfulReached`, `FailureToleranceExceeded`
  - [x] 1.7: Derive `Serialize + Deserialize` on BatchResult, BatchItem, BatchItemStatus, CompletionReason (needed for checkpoint payload)
  - [x] 1.8: Rustdoc with `# Examples` on all types
  - [x] 1.9: Re-export from `lib.rs` and closure prelude

- [x] Task 2: Add child context creation to `DurableContext` (AC: #3)
  - [x] 2.1: `pub fn create_child_context(&self, parent_op_id: &str) -> DurableContext` — creates a new DurableContext sharing the same backend and ARN but with a child OperationIdGenerator scoped by parent_op_id
  - [x] 2.2: Child context shares `Arc<dyn DurableBackend>`, same ARN and checkpoint_token
  - [x] 2.3: Child context gets a fresh `ReplayEngine` initialized with the SAME operations HashMap (child operations are in the same state) but with `parent_id = Some(parent_op_id)` for ID generation
  - [x] 2.4: Rustdoc with `# Examples`

- [x] Task 3: Add `DurableError::ParallelFailed` variant (AC: #2)
  - [x] 3.1: `ParallelFailed { operation_name: String, error_message: String }` variant with `#[non_exhaustive]`
  - [x] 3.2: `DurableError::parallel_failed(operation_name, error_message)` constructor
  - [x] 3.3: Rustdoc with `# Examples`

- [x] Task 4: Implement `child_handler` helper for Context checkpoint lifecycle (AC: #6)
  - [x] 4.1: Private async fn that sends Context/START, executes a closure, then sends Context/SUCCEED or Context/FAIL
  - [x] 4.2: START checkpoint: OperationType::Context, OperationAction::Start, sub_type parameter, name, ContextOptions (replay_children: false)
  - [x] 4.3: SUCCEED checkpoint: OperationType::Context, OperationAction::Succeed, sub_type, payload = serialized result, ContextOptions (replay_children: false)
  - [x] 4.4: FAIL checkpoint: OperationType::Context, OperationAction::Fail, sub_type, error details
  - [x] 4.5: Token update and new_execution_state merge after each checkpoint

- [x] Task 5: Implement `parallel` on `DurableContext` in `operations/parallel.rs` (AC: #1, #2, #3, #4, #5, #6)
  - [x] 5.1: `pub async fn parallel<T, F, Fut>(&mut self, name: &str, branches: Vec<F>, options: ParallelOptions) -> Result<BatchResult<T>, DurableError>` where T: Serialize + DeserializeOwned + Send + 'static, F: FnOnce(DurableContext) -> Fut + Send + 'static, Fut: Future<Output = Result<T, DurableError>> + Send + 'static
  - [x] 5.2: Operation ID via `generate_operation_id()` for outer parallel block
  - [x] 5.3: Replay path: `check_result(op_id)` → if SUCCEEDED, deserialize BatchResult from step_details or payload, track_replay, return
  - [x] 5.4: Execute path: send outer Context/START (sub_type: "Parallel", non-blocking)
  - [x] 5.5: For each branch i: compute branch_op_id = blake2b("{parallel_op_id}-{i}"), create child context with parent_id = branch_op_id
  - [x] 5.6: Spawn each branch via `tokio::spawn` with child_handler wrapping (Context/START + execute + Context/SUCCEED for each branch, sub_type: "ParallelBranch")
  - [x] 5.7: Collect results from all spawned tasks via `tokio::JoinSet` or `futures::join_all`
  - [x] 5.8: Build `BatchResult<T>` from branch outcomes
  - [x] 5.9: Send outer Context/SUCCEED with serialized BatchResult as payload
  - [x] 5.10: track_replay, return BatchResult

- [x] Task 6: Add `parallel` delegation to `ClosureContext` (AC: #1)
  - [x] 6.1: `parallel()` method delegating to `self.inner.parallel()`
  - [x] 6.2: Rustdoc with `# Examples` and `# Errors`

- [x] Task 7: Write tests (AC: #1, #2, #3, #4, #5, #6, #7)
  - [x] 7.1: `test_parallel_executes_branches_concurrently` — 3 branches, all succeed, returns BatchResult with 3 items
  - [x] 7.2: `test_parallel_replays_from_cached_result` — SUCCEEDED parallel in history, returns deserialized BatchResult, zero checkpoints
  - [x] 7.3: `test_parallel_branches_have_isolated_namespaces` — branches can call ctx.step with same name without collision
  - [x] 7.4: `test_parallel_sends_correct_checkpoint_sequence` — verify outer START + per-branch START/SUCCEED + outer SUCCEED
  - [x] 7.5: `test_parallel_branch_failure_is_captured` — one branch fails, BatchResult contains error for that branch
  - [x] 7.6: All doc tests compile
  - [x] 7.7: ClosureContext delegation tests

- [x] Task 8: Verify all checks pass (AC: #7)
  - [x] 8.1: `cargo test --workspace` — all tests pass
  - [x] 8.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [x] 8.3: `cargo fmt --check` — formatting passes

### Review Follow-ups (AI)

- [x] [AI-Review][Medium] Remove unused `branch_op_ids` vector — allocated and pushed to but never read after the loop [crates/durable-lambda-core/src/operations/parallel.rs:152,159]

## Dev Notes

### Pattern Category: Multi-Phase Context Operation (NEW PATTERN)

This is fundamentally different from all previous operations. Parallel is the first operation that:
- Sends **multiple checkpoint pairs** (START+SUCCEED for each branch + outer block)
- Creates **child contexts** with isolated operation ID namespaces
- Uses **`tokio::spawn`** for actual concurrency (Send + 'static bounds)
- Uses **OperationType::Context** (not Step, Wait, Callback, or ChainedInvoke)
- Has a **hierarchical checkpoint structure** (outer block → branches → branch operations)

### Python SDK Parallel Flow (Exact Wire Protocol)

```
FIRST EXECUTION:
1. generate_operation_id() → parallel_op_id (outer block ID)
2. check_result(parallel_op_id) → NOT FOUND
3. Send Context/START for outer block:
   { Id: parallel_op_id, Type: CONTEXT, Action: START, SubType: "Parallel", Name: user_name }
4. For each branch i = 0, 1, 2, ...:
   a. branch_op_id = blake2b("{parallel_op_id}-{i}")[:64]
   b. Create child context with parent_id = branch_op_id
   c. Send Context/START for branch:
      { Id: branch_op_id, Type: CONTEXT, Action: START, SubType: "ParallelBranch", Name: "parallel-branch-{i}", ParentId: parallel_op_id }
   d. Execute branch closure with child context
   e. Send Context/SUCCEED for branch:
      { Id: branch_op_id, Type: CONTEXT, Action: SUCCEED, SubType: "ParallelBranch", Payload: serialized_result, ContextOptions: { ReplayChildren: false } }
5. Collect all branch results into BatchResult
6. Send Context/SUCCEED for outer block:
   { Id: parallel_op_id, Type: CONTEXT, Action: SUCCEED, SubType: "Parallel", Payload: serialized_batch_result, ContextOptions: { ReplayChildren: false } }
7. track_replay(parallel_op_id)
8. Return BatchResult

RE-INVOCATION (parallel SUCCEEDED in history):
1. generate_operation_id() → same parallel_op_id
2. check_result(parallel_op_id) → SUCCEEDED
3. Deserialize BatchResult from operation's payload (step_details.result or payload field)
4. track_replay(parallel_op_id)
5. Return BatchResult — NO branches re-executed
```

### CRITICAL Design Decisions

1. **Wire type is `Context` (OperationType::Context)** — NOT a new operation type. Both parallel and child_context operations use Context as the wire type, differentiated by sub_type ("Parallel", "ParallelBranch", "Context").

2. **Branch operation IDs are index-based, NOT counter-based** — In Python SDK, branch IDs are `blake2b("{parallel_op_id}-{branch_index}")`. This ensures branches can execute in any order and produce deterministic IDs. Our existing `OperationIdGenerator::new(Some(parent_id))` uses counter-based IDs (`blake2b("{parent_id}-{counter}")`), which works since branches are spawned in order with indices 0, 1, 2...

3. **Child contexts share the SAME operations HashMap** — Branches don't get separate state stores. They share the parent's `ReplayEngine` operations map but have isolated operation ID generation via parent_id scoping. However, for `tokio::spawn`, each branch needs its own `DurableContext` to avoid shared mutable state. **The child context must clone or share the operations map appropriately.**

4. **Send + 'static bounds** — Branch closures must be `FnOnce(DurableContext) -> Fut + Send + 'static` where `Fut: Future<Output = Result<T, DurableError>> + Send + 'static`. The child DurableContext is **owned** by each branch — no shared mutable references.

5. **Backend is `Arc<dyn DurableBackend>`** — Already `Arc`-wrapped, so cloning for child contexts is cheap. The checkpoint_token is trickier — branches may each get different tokens from their checkpoints. For simplicity, each child context tracks its own token.

6. **Replay: outer block only** — On replay, we check the OUTER parallel operation's result. If SUCCEEDED with `replay_children: false`, we deserialize the BatchResult directly without re-executing branches. We do NOT need to replay individual branches.

### AWS SDK Types for Context Operations

```rust
// OperationType::Context — the wire type for parallel, child context, etc.
// OperationAction::Start — opening a context
// OperationAction::Succeed — closing a context successfully
// OperationAction::Fail — closing a context with error

// ContextOptions — replay configuration
// NOTE: .build() returns direct type (not Result)
let ctx_opts = aws_sdk_lambda::types::ContextOptions::builder()
    .replay_children(false)
    .build();

// Context START checkpoint
let start_update = OperationUpdate::builder()
    .id(op_id.clone())
    .r#type(OperationType::Context)
    .action(OperationAction::Start)
    .sub_type("Parallel")  // or "ParallelBranch"
    .name(name)
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;

// Context SUCCEED checkpoint (with payload)
let succeed_update = OperationUpdate::builder()
    .id(op_id.clone())
    .r#type(OperationType::Context)
    .action(OperationAction::Succeed)
    .sub_type("Parallel")
    .payload(serialized_result)
    .context_options(ctx_opts)
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;
```

### Builder Return Types (IMPORTANT)

- `ContextOptions::builder().build()` → **direct type** (not Result)
- `OperationUpdate::builder().build()` → **Result**

### Branch Operation ID Generation

**CRITICAL:** Branch operation IDs must match the Python SDK's approach for deterministic replay:

```rust
// For branch i within a parallel block:
// Python: blake2b(f"{parallel_op_id}-{i}")[:64]
// Rust equivalent:
fn branch_op_id(parallel_op_id: &str, branch_index: usize) -> String {
    let input = format!("{}-{}", parallel_op_id, branch_index);
    blake2b_hash_64(&input)  // reuse existing function from operation_id.rs
}
```

Note: The Python SDK uses `_create_step_id_for_logical_step(index)` which hashes `"{parent_id}-{index}"`. This matches our `OperationIdGenerator` pattern if we treat the index as the counter. However, the Python SDK starts indices at 0 and hashes `"{parent_id}-{0}"`, while our counter starts at 1 and hashes `"{parent_id}-{1}"`. **Verify which is correct and match Python SDK exactly.**

Actually, looking more carefully: Python's `_create_step_id_for_logical_step(index)` sets `_step_counter = index` and calls `_create_step_id()` which does `self._step_counter += 1` then hashes `"{parent_id}-{self._step_counter}"`. So for index=0, it hashes `"{parent_id}-1"`. For index=1, it hashes `"{parent_id}-2"`. **This matches our counter-based OperationIdGenerator which starts at counter=0 and increments to 1 before hashing.** So using `OperationIdGenerator::new(Some(parallel_op_id))` and calling `next_id()` for each branch should produce correct IDs.

### Child Context Design

```rust
// Child DurableContext for each branch:
// - Shares: Arc<dyn DurableBackend> (cloned Arc), durable_execution_arn (cloned)
// - Own: ReplayEngine with parent_id = branch_op_id, checkpoint_token (cloned from parent)
// - The ReplayEngine needs access to the same operations HashMap for replay lookups
//   BUT must have its own OperationIdGenerator with the branch's parent_id

// Problem: ReplayEngine owns the operations HashMap. We can't share it across
// tokio::spawn boundaries without Arc<Mutex<>> or similar.
//
// Solution for first pass: Clone the operations HashMap for each child context.
// This is acceptable because:
// 1. Operations map is read-only during branch execution (branches only add, parent reads)
// 2. The map is typically small (tens to low hundreds of entries)
// 3. Avoids complex shared-state synchronization

impl DurableContext {
    pub fn create_child_context(&self, parent_op_id: &str) -> DurableContext {
        DurableContext::new_child(
            self.backend.clone(),
            self.durable_execution_arn.clone(),
            self.checkpoint_token.clone(),
            self.replay_engine.operations().clone(),  // Clone operations map
            parent_op_id.to_string(),
        )
    }
}
```

### Storing Results: Where Does the BatchResult Go?

On SUCCEED, the serialized BatchResult is stored in the `payload` field of the outer Context operation's SUCCEED checkpoint. On replay, we need to find this payload. The `Operation` struct has a `payload` field (`Option<String>`) but it's accessed via `step_details().result()` in some cases.

**Check how the Python SDK stores it:** The `create_context_succeed` factory sets `payload=serialized_result`. On replay, `child_handler` checks `checkpointed_result.result` which comes from `CheckpointedResult.create_from_operation()` for CONTEXT type: `result = operation.payload`.

So the result is in `operation.payload`, NOT in `step_details.result`. **However**, the Rust AWS SDK `Operation` struct doesn't have a top-level `payload` accessor — it has `step_details`, `callback_details`, `chained_invoke_details`. Let me check...

Actually, looking at the Operation builder, there's no `payload` field on Operation directly. The payload is on `OperationUpdate` (what we SEND), not on `Operation` (what we READ). The checkpoint API response puts the result in operation-specific details.

**For Context operations**, the result may be stored differently. We need to check if there's a context_details field or if the step_details.result is reused. Since this is a CONTEXT type (not Step), we should check the actual Operation structure.

**IMPORTANT: This needs investigation during implementation.** Check:
1. Does `Operation` have any context-specific details field?
2. Or does the server store Context payloads in `step_details.result`?
3. Or is there another mechanism?

If no context-specific details exist, we may need to store the BatchResult in `step_details.result` as a convention, or explore `OperationUpdate.payload` mapping to `Operation.step_details.result`.

### track_replay Behavior

- `parallel()` calls `track_replay` for the outer parallel_op_id only
- Individual branch operations are tracked by the child context's operations during execution
- On replay, only the outer operation is checked — branches are not re-executed

### What Exists vs What Needs to Be Added

**Already exists:**
- `DurableContext` with backend, ARN, checkpoint_token, ReplayEngine
- `OperationIdGenerator` with parent_id support (child context scoping)
- `ReplayEngine` with operations HashMap, check_result, get_operation, track_replay, insert_operation
- `DurableBackend` trait and MockBackend pattern
- `ClosureContext` with delegation pattern
- `DurableError` with `#[non_exhaustive]` and constructor methods
- `operations/parallel.rs` stub (header comment only)
- All previous operations (step, wait, callback, invoke) as reference

**Needs to be added:**
- `ParallelOptions`, `CompletionCriteria`, `BatchResult<T>`, `BatchItem<T>`, `BatchItemStatus`, `CompletionReason` in `types.rs`
- `DurableContext::create_child_context()` method
- `DurableContext::new_child()` constructor (or similar)
- `DurableError::ParallelFailed` variant + constructor
- `child_handler` helper for Context checkpoint lifecycle
- `DurableContext::parallel()` method in `operations/parallel.rs`
- `ClosureContext::parallel()` delegation
- Unit tests for all paths

### Architecture Doc Discrepancies (IMPORTANT — Inherited)

1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Operation ID**: Uses blake2b hash of counter, NOT user-provided name
3. **Handler signature**: Takes owned `ClosureContext`, receives `(event, ctx)`
4. **Wire type**: Parallel uses `OperationType::Context` with sub_type "Parallel"/"ParallelBranch" — NOT a new `OperationType::Parallel`

### Previous Story Intelligence (Epic 2 Retro)

- Operation taxonomy: Step (SDK-completed), Wait (server-completed, no result), Callback (external, two-phase), Invoke (server-completed, with result). Parallel adds a new category: **multi-checkpoint, child-context, concurrent**
- `get_operation()` already available for any-status lookups
- Rust generics-on-impl constraint: use static methods or turbofish for generic helpers
- Story should classify pattern category (done above: "Multi-Phase Context Operation")
- Story should explicitly state track_replay behavior (done above)

### Testing Approach

- This is the most complex operation yet — tests need to verify:
  1. Concurrent execution (branches actually run in parallel via tokio)
  2. Checkpoint sequence (outer START, branch STARTs, branch SUCCEEDs, outer SUCCEED)
  3. Namespace isolation (branches with same step names don't collide)
  4. Replay (outer SUCCEEDED → deserialize BatchResult, skip branches)
  5. Branch failure handling (one branch fails → captured in BatchResult)
- MockBackend must handle MULTIPLE checkpoint calls in sequence and return appropriate responses
- Test naming: `test_parallel_{behavior}_{condition}`

### IMPORTANT: Investigate During Implementation

1. **How does the server store Context operation results?** Check if `Operation` has context-specific details or if step_details.result is reused for Context type
2. **Branch ID generation** — Verify counter-based matches Python SDK's index-based approach (counter starts at 1, index starts at 0, but Python's `_create_step_id_for_logical_step(0)` also produces counter=1)
3. **Checkpoint token management across branches** — Each branch checkpoint returns a new token. Do branches need to coordinate tokens, or can each track independently?

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.1 — acceptance criteria]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR19, FR20, FR21, FR22]
- [Source: _bmad-output/planning-artifacts/architecture.md — Send + 'static bounds, parallel operation pattern]
- [Source: _bmad-output/implementation-artifacts/epic-2-retro-2026-03-14.md — process improvements for Epic 3]
- [Source: crates/durable-lambda-core/src/operation_id.rs — OperationIdGenerator with parent_id support]
- [Source: crates/durable-lambda-core/src/replay.rs — ReplayEngine with operations HashMap]
- [Source: crates/durable-lambda-core/src/context.rs — DurableContext structure]
- [Source: Python SDK — github.com/aws/aws-durable-execution-sdk-python — parallel/ConcurrentExecutor]
- [Source: aws_sdk_lambda::types::OperationType::Context — context operation type]
- [Source: aws_sdk_lambda::types::ContextOptions — replay_children field]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- `ParallelOptions` struct, `BatchResult<T>`, `BatchItem<T>`, `BatchItemStatus`, `CompletionReason` types in types.rs — all Serialize+Deserialize for checkpoint payload
- `DurableContext::create_child_context(parent_op_id)` — creates child context with cloned operations map, shared Arc backend, isolated OperationIdGenerator scoped by parent_op_id
- `DurableError::ParallelFailed` variant + `parallel_failed()` constructor
- `DurableContext::parallel(name, branches, options)` — full parallel execution: outer Context/START, per-branch tokio::spawn with Context/START+SUCCEED (sub_type "ParallelBranch"), collect BatchResult, outer Context/SUCCEED with serialized payload
- `execute_branch()` helper with `BranchConfig` struct (avoids clippy too_many_arguments)
- Replay path: outer SUCCEEDED → deserialize BatchResult from `context_details().result()` — investigation item #1 resolved: ContextDetails has `result` field
- Branch ID generation uses `OperationIdGenerator::new(Some(parallel_op_id))` — produces counter-based IDs matching Python SDK's `_create_step_id_for_logical_step` behavior (investigation item #2 resolved)
- Branch checkpoints use independent token from parent (investigation item #3 resolved: each branch tracks its own token)
- `ClosureContext::parallel()` delegation method
- 5 parallel unit tests: concurrent execution, replay, namespace isolation, checkpoint sequence, branch failure capture
- `ContextOptions::builder().build()` returns direct type (not Result), confirmed
- `ContextDetails::builder().build()` returns direct type (not Result), confirmed
- 189 total workspace tests pass
- Resolved review finding [Medium]: Removed unused `branch_op_ids` vector (parallel.rs) — dead code that was allocated and pushed to but never read

### File List

- crates/durable-lambda-core/src/types.rs (modified — added ParallelOptions, BatchResult, BatchItem, BatchItemStatus, CompletionReason)
- crates/durable-lambda-core/src/lib.rs (modified — added BatchItem, BatchItemStatus, BatchResult, CompletionReason, ParallelOptions re-exports)
- crates/durable-lambda-core/src/context.rs (modified — added create_child_context() method)
- crates/durable-lambda-core/src/error.rs (modified — added ParallelFailed variant + constructor)
- crates/durable-lambda-core/src/operations/parallel.rs (rewritten — parallel(), execute_branch(), BranchConfig, 5 unit tests)
- crates/durable-lambda-closure/src/context.rs (modified — added parallel() delegation)
- crates/durable-lambda-closure/src/prelude.rs (modified — added BatchItem, BatchItemStatus, BatchResult, CompletionReason, ParallelOptions re-exports)

### Senior Developer Review (AI)

**Review Date:** 2026-03-14
**Reviewer:** Claude Opus 4.6
**Outcome:** Changes Requested (minor)

**Summary:** Clean implementation of the most complex operation in the SDK. All 7 ACs verified. All tasks genuinely complete. 5 real unit tests with meaningful assertions. Correct use of OperationType::Context, child context namespacing, tokio::spawn with Send + 'static, and BatchResult serialization via context_details. One minor dead-code finding.

**Action Items:**
- [x] [Medium] Remove unused `branch_op_ids` vector (parallel.rs:152,159)

### Change Log

- 2026-03-14: Story 3.1 implemented — parallel operation with child contexts, tokio::spawn concurrency, multi-checkpoint Context lifecycle, BatchResult serialization. 5 parallel unit tests + doc tests passing. Clippy clean, fmt clean. 189 total workspace tests pass.
- 2026-03-14: Addressed code review findings — 1 item resolved: removed unused `branch_op_ids` vector from parallel.rs. 202 total workspace tests pass.
