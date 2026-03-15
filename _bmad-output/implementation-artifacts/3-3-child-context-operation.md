# Story 3.3: Child Context Operation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to create isolated subflows with their own checkpoint namespace,
So that I can organize complex workflows into logically independent sections that don't interfere with each other's checkpoint state.

## Acceptance Criteria

1. **Given** a DurableContext in Executing mode **When** I call `ctx.child_context("sub_workflow", closure)` with a name and closure **Then** an isolated child DurableContext is created with its own checkpoint namespace (FR26) **And** the child context can execute any durable operation independently from the parent (FR27)

2. **Given** a child context **When** it executes durable operations (steps, waits, parallel, etc.) **Then** its checkpoint keys are namespaced under the child context's operation ID **And** do not collide with the parent or sibling child contexts

3. **Given** a child context **When** it is created **Then** it is a fully owned context sharing only `Arc<dyn DurableBackend>` with the parent (FR28) **And** no shared mutable state exists between parent and child

4. **Given** a child context namespacing strategy **When** implemented **Then** it matches the Python SDK's namespacing approach for behavioral compliance (NFR5) — specifically, child context uses `OperationType::Context` with sub_type "Context" and the child's operation ID scopes all nested operations

5. **Given** a DurableContext in Replaying mode with a completed child context in history (SUCCEEDED with payload) **When** the replay engine encounters the child context entry **Then** the cached result is deserialized from `context_details().result()` **And** the closure is NOT re-executed **And** `track_replay` is called for the outer child_context operation ID

6. **Given** the child context operation **When** I examine the checkpoints sent **Then** one Context/START is sent for the child context block (sub_type: "Context") **And** one Context/SUCCEED for the child context block with the serialized result as payload **And** `ContextOptions { replay_children: false }` is set on SUCCEED

7. **Given** the child context closure **When** it returns a value of type `T: Serialize + DeserializeOwned` **Then** that value is serialized as the Context/SUCCEED payload **And** returned directly (NOT wrapped in BatchResult)

8. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --workspace` **Then** all tests pass including new child context operation tests **And** all doc tests compile

## Tasks / Subtasks

- [x] Task 1: Add `DurableError::ChildContextFailed` variant (AC: #5)
  - [x] 1.1: `ChildContextFailed { operation_name: String, error_message: String }` variant (mirrors `ParallelFailed`)
  - [x] 1.2: `DurableError::child_context_failed(operation_name, error_message)` constructor
  - [x] 1.3: Rustdoc with `# Examples`

- [x] Task 2: Implement `child_context` on `DurableContext` in `operations/child_context.rs` (AC: #1, #2, #3, #4, #5, #6, #7)
  - [x] 2.1: `pub async fn child_context<T, F, Fut>(&mut self, name: &str, f: F) -> Result<T, DurableError>` where T: Serialize + DeserializeOwned + Send, F: FnOnce(DurableContext) -> Fut + Send, Fut: Future<Output = Result<T, DurableError>> + Send
  - [x] 2.2: Operation ID via `generate_operation_id()` for the child context block
  - [x] 2.3: Replay path: `check_result(op_id)` → if SUCCEEDED, deserialize T from `context_details().result()`, track_replay, return
  - [x] 2.4: Replay path: if status is Failed/Cancelled/TimedOut → return `ChildContextFailed` with error from `context_details().error()`
  - [x] 2.5: Execute path: send Context/START (sub_type: "Context", non-blocking)
  - [x] 2.6: Update checkpoint_token and merge new_execution_state from START response
  - [x] 2.7: Create child context via `self.create_child_context(&child_op_id)` — reuse existing method from Story 3.1
  - [x] 2.8: Execute closure with child context: `f(child_ctx).await`
  - [x] 2.9: Send Context/SUCCEED with serialized result as payload, ContextOptions { replay_children: false }
  - [x] 2.10: Update checkpoint_token and merge new_execution_state from SUCCEED response
  - [x] 2.11: track_replay(op_id), return result

- [x] Task 3: Add `child_context` delegation to `ClosureContext` (AC: #1)
  - [x] 3.1: `child_context()` method delegating to `self.inner.child_context()`
  - [x] 3.2: Rustdoc with `# Examples` and `# Errors`

- [x] Task 4: Write tests (AC: #1, #2, #3, #4, #5, #6, #7, #8)
  - [x] 4.1: `test_child_context_executes_closure` — closure executes, returns result, correct checkpoints sent
  - [x] 4.2: `test_child_context_replays_from_cached_result` — SUCCEEDED child context in history, returns deserialized result, zero checkpoints
  - [x] 4.3: `test_child_context_has_isolated_namespace` — child can call ctx.step with same name as parent without collision
  - [x] 4.4: `test_child_context_sends_correct_checkpoint_sequence` — verify Context/START + Context/SUCCEED with sub_type "Context"
  - [x] 4.5: `test_child_context_closure_failure_propagates` — closure returns Err, error propagates to caller
  - [x] 4.6: `test_child_context_nested` — child context within child context works correctly
  - [x] 4.7: All doc tests compile
  - [x] 4.8: ClosureContext delegation test

- [x] Task 5: Verify all checks pass (AC: #8)
  - [x] 5.1: `cargo test --workspace` — all tests pass
  - [x] 5.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [x] 5.3: `cargo fmt --check` — formatting passes

## Dev Notes

### Pattern Category: Single-Phase Context Operation (Simplified Parallel)

Child context follows a **simplified version** of the parallel pattern from Story 3.1. Key differences: single closure instead of multiple branches, runs inline (no `tokio::spawn`), returns `T` directly instead of `BatchResult<T>`.

### Relationship to Parallel (Story 3.1) and Map (Story 3.2)

Child context is the simplest of the three Context-type operations. Reuse infrastructure from 3.1:
- `create_child_context()` — **already exists** on DurableContext (context.rs:241)
- `OperationType::Context` as wire type — same as parallel/map
- Replay via outer operation `context_details().result()` — same pattern
- `ContextOptions::builder().replay_children(false).build()` — same pattern
- Checkpoint token update + new_execution_state merge — same pattern

**DO NOT use `tokio::spawn`.** The child context closure runs inline in the parent's async context. This means:
- No `Send + 'static` bounds needed on the closure or return type (only `Send` for async compatibility)
- No `BatchResult` wrapping — return `T` directly
- No concurrency — sequential execution within the parent flow

### Child Context vs Parallel: Key Differences

| Aspect | Parallel | Child Context |
|--------|----------|---------------|
| Input | `Vec<F>` (multiple closures) | Single `F` (one closure) |
| Concurrency | `tokio::spawn` per branch | Inline execution (no spawn) |
| Closure bounds | `FnOnce(DurableContext) -> Fut + Send + 'static` | `FnOnce(DurableContext) -> Fut + Send` |
| Return type | `BatchResult<T>` | `T` directly |
| Sub-types | "Parallel" / "ParallelBranch" | "Context" only |
| Error variant | `ParallelFailed` | `ChildContextFailed` |

### Wire Protocol (Based on Python SDK Context Pattern)

```
FIRST EXECUTION:
1. generate_operation_id() → child_op_id (context block ID)
2. check_result(child_op_id) → NOT FOUND
3. Send Context/START for child context block:
   { Id: child_op_id, Type: CONTEXT, Action: START, SubType: "Context", Name: user_name }
4. Update checkpoint_token from response
5. Merge new_execution_state operations into replay engine
6. Create child context via create_child_context(&child_op_id)
7. Execute f(child_ctx).await → result
8. Serialize result to JSON
9. Send Context/SUCCEED for child context block:
   { Id: child_op_id, Type: CONTEXT, Action: SUCCEED, SubType: "Context",
     Payload: serialized_result, ContextOptions: { ReplayChildren: false } }
10. Update checkpoint_token from response
11. Merge new_execution_state operations into replay engine
12. track_replay(child_op_id)
13. Return result

RE-INVOCATION (child context SUCCEEDED in history):
1. generate_operation_id() → same child_op_id
2. check_result(child_op_id) → SUCCEEDED
3. Deserialize T from context_details().result()
4. track_replay(child_op_id)
5. Return T — closure NOT re-executed
```

### Method Signature

```rust
pub async fn child_context<T, F, Fut>(
    &mut self,
    name: &str,
    f: F,
) -> Result<T, DurableError>
where
    T: Serialize + DeserializeOwned + Send,
    F: FnOnce(DurableContext) -> Fut + Send,
    Fut: Future<Output = Result<T, DurableError>> + Send,
```

**Note on bounds:** No `'static` bound needed since the closure is NOT spawned — it runs inline via `.await`. This is simpler than parallel/map and makes the API more ergonomic (closures can borrow from the enclosing scope as long as the borrow is `Send`).

### AWS SDK Types for Context Operations

```rust
// Reuse exact same types as parallel — OperationType::Context
// Sub_type is "Context" (not "Parallel" or "ParallelBranch")

// Context START for child context block
let start_update = OperationUpdate::builder()
    .id(op_id.clone())
    .r#type(OperationType::Context)
    .action(OperationAction::Start)
    .sub_type("Context")
    .name(name)
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;

// Context SUCCEED for child context (with payload)
let ctx_opts = ContextOptions::builder()
    .replay_children(false)
    .build();

let succeed_update = OperationUpdate::builder()
    .id(op_id.clone())
    .r#type(OperationType::Context)
    .action(OperationAction::Succeed)
    .sub_type("Context")
    .payload(serialized_result)
    .context_options(ctx_opts)
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;
```

### Builder Return Types (IMPORTANT — from Story 3.1)

- `ContextOptions::builder().build()` → **direct type** (not Result)
- `OperationUpdate::builder().build()` → **Result**
- `ContextDetails::builder().build()` → **direct type** (not Result)

### track_replay Behavior

- `child_context()` calls `track_replay` for the outer child_op_id only
- Operations within the child context are tracked by the child's ReplayEngine during execution
- On replay, only the outer operation is checked — the closure is not re-executed

### Architecture Doc Discrepancies (INHERITED from Story 3.1)

1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Operation ID**: Uses blake2b hash of counter, NOT user-provided name
3. **Handler signature**: Takes owned `ClosureContext`, receives `(event, ctx)`
4. **Wire type**: Child context uses `OperationType::Context` with sub_type "Context" — NOT a new `OperationType::ChildContext`

### What Exists vs What Needs to Be Added

**Already exists (from Story 3.1 and earlier):**
- `DurableContext` with backend, ARN, checkpoint_token, ReplayEngine
- `DurableContext::create_child_context()` — creates child with isolated namespace (context.rs:241)
- `OperationIdGenerator` with parent_id support
- `ReplayEngine` with operations HashMap, check_result, get_operation, track_replay
- `DurableBackend` trait and MockBackend pattern
- `ClosureContext` with delegation pattern
- `DurableError` with `#[non_exhaustive]` and constructor methods
- `operations/child_context.rs` stub (doc comment only, 5 lines)
- Context checkpoint lifecycle pattern from parallel.rs (START + execute + SUCCEED)
- `context_details().result()` for reading Context SUCCEED payloads
- `context_details().error()` for reading Context failure details

**Needs to be added:**
- `DurableError::ChildContextFailed` variant + `child_context_failed()` constructor in error.rs
- `DurableContext::child_context()` method in `operations/child_context.rs`
- `ClosureContext::child_context()` delegation in closure crate context.rs
- Re-exports: no new types to export (no ChildContextOptions — method takes only name and closure)
- Unit tests for all paths (execution, replay, namespace isolation, checkpoint sequence, failure, nesting)

### Previous Story Intelligence (Story 3.1 + 3.2 Learnings)

- Context operations use `OperationType::Context`, differentiated by sub_type string
- `context_details().result()` is where SUCCEED payloads are stored for Context operations
- Child context IDs via `generate_operation_id()` (no parent-scoped generator needed since there's only one context, not multiple branches)
- `ContextOptions::builder().build()` returns direct type, `OperationUpdate::builder().build()` returns Result
- Checkpoint token must be updated after BOTH START and SUCCEED responses
- `new_execution_state` operations must be merged into replay engine after each checkpoint
- Story 3.1 had unused `branch_op_ids` vector — avoid dead code
- Story 3.2 added `MapOptions` but child_context needs NO options struct (simpler API)
- `execute_branch()` helper in parallel.rs is a useful reference but DO NOT call it — child_context should implement inline since it's simpler (no spawn, no branch config)

### Testing Approach

- Mirror parallel test suite structure but simpler (no concurrency, no BatchResult)
- MockBackend must handle checkpoint calls and return tokens
- Test naming: `test_child_context_{behavior}_{condition}`
- Nesting test: child context within child context — verify operation IDs are properly scoped
- The MockBackend from parallel tests (`ParallelMockBackend`) is a good template — copy and rename

### Parameter Ordering Convention

Following architecture doc convention: name, closure (no options needed)
```rust
ctx.child_context("sub_workflow", |mut child_ctx| async move {
    let r: Result<i32, String> = child_ctx.step("inner_step", || async { Ok(42) }).await?;
    Ok(r.unwrap())
}).await?
```

### ClosureContext Delegation Pattern

```rust
pub async fn child_context<T, F, Fut>(
    &mut self,
    name: &str,
    f: F,
) -> Result<T, DurableError>
where
    T: Serialize + DeserializeOwned + Send,
    F: FnOnce(DurableContext) -> Fut + Send,
    Fut: Future<Output = Result<T, DurableError>> + Send,
{
    self.inner.child_context(name, f).await
}
```

Note: The closure receives a `DurableContext` (core type), NOT a `ClosureContext`. This is consistent with parallel and map — child/branch closures always receive the core context.

### Project Structure Notes

- Implementation: `crates/durable-lambda-core/src/operations/child_context.rs` (rewrite stub)
- Error variant: `crates/durable-lambda-core/src/error.rs` (add variant + constructor)
- Delegation: `crates/durable-lambda-closure/src/context.rs` (add method)
- No new types to add to `types.rs`, `lib.rs`, or `prelude.rs`

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.3 — acceptance criteria, FR26-FR28]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR26, FR27, FR28]
- [Source: _bmad-output/planning-artifacts/architecture.md — child context namespacing, Context wire type, parameter ordering]
- [Source: _bmad-output/implementation-artifacts/3-1-parallel-operation.md — parallel patterns, child context design, Context wire protocol, builder return types]
- [Source: _bmad-output/implementation-artifacts/3-2-map-operation.md — map patterns, confirms Context wire protocol, builder return types]
- [Source: crates/durable-lambda-core/src/operations/parallel.rs — execute_branch(), checkpoint lifecycle reference]
- [Source: crates/durable-lambda-core/src/context.rs:241 — create_child_context() already exists]
- [Source: crates/durable-lambda-core/src/error.rs — DurableError enum, constructor pattern]
- [Source: crates/durable-lambda-closure/src/context.rs — ClosureContext delegation pattern]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6 (1M context)

### Debug Log References

### Completion Notes List

- Implemented `child_context` on DurableContext in `operations/child_context.rs` with full execute + replay paths
- Execute path: Context/START → create_child_context → execute closure inline → Context/SUCCEED with serialized result and ContextOptions { replay_children: false }
- Replay path: check_result → Succeeded: deserialize from context_details().result() → Failed: return ChildContextFailed with error details
- Added `DurableError::ChildContextFailed` variant + `child_context_failed()` constructor in error.rs
- Added `ClosureContext::child_context()` delegation in closure crate
- Wrote 7 unit tests + 1 delegation test covering: execution, replay, namespace isolation, checkpoint sequence, failure propagation, nesting, failed replay status
- All workspace tests pass, clippy clean, fmt clean

### File List

- `crates/durable-lambda-core/src/operations/child_context.rs` — full child_context implementation + 7 unit tests
- `crates/durable-lambda-core/src/error.rs` — added ChildContextFailed variant + constructor
- `crates/durable-lambda-closure/src/context.rs` — added child_context delegation method + test
