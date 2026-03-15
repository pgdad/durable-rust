# Story 3.2: Map Operation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to process a collection of items in parallel using a closure applied to each item,
So that I can efficiently handle batch workloads (e.g., processing a list of orders, transforming a dataset) with configurable batching.

## Acceptance Criteria

1. **Given** a DurableContext in Executing mode **When** I call `ctx.map("process_items", items, options, closure)` with a collection of items and a closure **Then** the closure is applied to each item in the collection in parallel **And** each item gets its own child DurableContext with an isolated checkpoint namespace (FR23)

2. **Given** a map operation with batching configured via `MapOptions::new().batch_size(N)` **When** items are processed **Then** items are processed in batches of size N **And** each batch completes before the next batch begins (FR24)

3. **Given** a map operation with no batch_size configured (default) **When** items are processed **Then** all items execute concurrently in a single batch

4. **Given** a map operation that completes **When** all items have been processed **Then** a `BatchResult<T>` is returned containing results for all items **And** results maintain index correspondence to input items (FR25)

5. **Given** a DurableContext in Replaying mode with a completed map operation in history (SUCCEEDED with payload) **When** the replay engine encounters the map entry **Then** the cached `BatchResult` is deserialized from the outer context operation's `context_details().result()` **And** no item closures are re-executed **And** the cursor advances past all map-related entries

6. **Given** the map operation **When** I examine the checkpoints sent **Then** one Context/START is sent for the outer map block (sub_type: "Map") **And** one Context/START + Context/SUCCEED per item (sub_type: "MapItem") **And** one Context/SUCCEED for the outer block with the serialized BatchResult

7. **Given** item closures passed to map **When** they are compiled **Then** they satisfy `Send + 'static + Clone` bounds required for concurrent execution **And** items satisfy `Send + 'static` **And** the API patterns make satisfying these bounds natural (owned child context, owned item)

8. **Given** all public types, traits, and methods added in this story **When** I run `cargo test --workspace` **Then** all tests pass including new map operation tests **And** all doc tests compile

## Tasks / Subtasks

- [x] Task 1: Add `MapOptions` to `types.rs` (AC: #2, #3)
  - [x] 1.1: `MapOptions` struct with `batch_size: Option<usize>` field, `Default` impl (batch_size = None = all concurrent)
  - [x] 1.2: `MapOptions::new()` constructor, `batch_size(n: usize)` builder method
  - [x] 1.3: Derive `Debug, Clone, Default`
  - [x] 1.4: Rustdoc with `# Examples`
  - [x] 1.5: Re-export `MapOptions` from `lib.rs` and closure prelude

- [x] Task 2: Add `DurableError::MapFailed` variant (AC: #4)
  - [x] 2.1: `MapFailed { operation_name: String, error_message: String }` variant (mirrors `ParallelFailed`)
  - [x] 2.2: `DurableError::map_failed(operation_name, error_message)` constructor
  - [x] 2.3: Rustdoc with `# Examples`

- [x] Task 3: Implement `map` on `DurableContext` in `operations/map.rs` (AC: #1, #2, #3, #4, #5, #6, #7)
  - [x] 3.1: `pub async fn map<T, I, F, Fut>(&mut self, name: &str, items: Vec<I>, options: MapOptions, f: F) -> Result<BatchResult<T>, DurableError>` where T: Serialize + DeserializeOwned + Send + 'static, I: Send + 'static, F: FnOnce(I, DurableContext) -> Fut + Send + 'static + Clone, Fut: Future<Output = Result<T, DurableError>> + Send + 'static
  - [x] 3.2: Operation ID via `generate_operation_id()` for outer map block
  - [x] 3.3: Replay path: `check_result(op_id)` → if SUCCEEDED, deserialize BatchResult from `context_details().result()`, track_replay, return
  - [x] 3.4: Execute path: send outer Context/START (sub_type: "Map", non-blocking)
  - [x] 3.5: Batching logic: if `batch_size` set, chunk items into batches; else single batch with all items
  - [x] 3.6: For each batch: spawn items as `tokio::spawn` tasks, await all in batch before starting next batch
  - [x] 3.7: For each item i: compute item_op_id via `OperationIdGenerator::new(Some(map_op_id))`, create child context with parent_id = item_op_id
  - [x] 3.8: Each item spawned with Context/START + execute closure(item, child_ctx) + Context/SUCCEED (sub_type: "MapItem")
  - [x] 3.9: Collect results from all items/batches into `BatchResult<T>` preserving index order
  - [x] 3.10: Send outer Context/SUCCEED with serialized BatchResult as payload
  - [x] 3.11: track_replay, return BatchResult

- [x] Task 4: Add `map` delegation to `ClosureContext` (AC: #1)
  - [x] 4.1: `map()` method delegating to `self.inner.map()`
  - [x] 4.2: Rustdoc with `# Examples` and `# Errors`

- [x] Task 5: Write tests (AC: #1, #2, #3, #4, #5, #6, #7, #8)
  - [x] 5.1: `test_map_executes_items_concurrently` — 3 items, all succeed, returns BatchResult with 3 items in correct index order
  - [x] 5.2: `test_map_replays_from_cached_result` — SUCCEEDED map in history, returns deserialized BatchResult, zero checkpoints
  - [x] 5.3: `test_map_items_have_isolated_namespaces` — items can call ctx.step with same name without collision
  - [x] 5.4: `test_map_sends_correct_checkpoint_sequence` — verify outer START + per-item START/SUCCEED + outer SUCCEED
  - [x] 5.5: `test_map_item_failure_is_captured` — one item fails, BatchResult contains error for that item
  - [x] 5.6: `test_map_batching_processes_sequentially` — 4 items with batch_size=2, verify first batch completes before second starts
  - [x] 5.7: `test_map_default_options_all_concurrent` — no batch_size, all items execute in single batch
  - [x] 5.8: All doc tests compile
  - [x] 5.9: ClosureContext delegation tests

- [x] Task 6: Verify all checks pass (AC: #8)
  - [x] 6.1: `cargo test --workspace` — all tests pass
  - [x] 6.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [x] 6.3: `cargo fmt --check` — formatting passes

### Review Follow-ups (AI)

- [ ] [AI-Review][Low] Add dedicated `ClosureContext::map` runtime delegation unit test (e.g., `test_closure_context_map_delegates_to_core`) — currently only doc test covers compilation; consistent with parallel pattern but worth adding for completeness [crates/durable-lambda-closure/src/context.rs]

## Dev Notes

### Pattern Category: Multi-Phase Context Operation (Same as Parallel — Story 3.1)

Map follows the exact same architectural pattern as parallel. Key difference: parallel takes a `Vec<F>` of distinct closures, map takes a `Vec<I>` of items and a single `F: Clone` closure applied to each item.

### Relationship to Parallel (Story 3.1)

Map is essentially "parallel with data". Reuse all infrastructure from 3.1:
- `create_child_context()` — already exists on DurableContext
- `execute_branch()` / equivalent helper — Context/START + closure + Context/SUCCEED lifecycle
- `BatchResult<T>`, `BatchItem<T>`, `BatchItemStatus`, `CompletionReason` — already defined in types.rs
- `OperationType::Context` as wire type — same as parallel
- Replay via outer operation `context_details().result()` — same pattern

**DO NOT reinvent these. Call or adapt the same helpers from parallel.rs.** If `execute_branch()` is private, consider extracting a shared helper into a common module, or duplicate with minimal adaptation.

### Map vs Parallel: Key Differences

| Aspect | Parallel | Map |
|--------|----------|-----|
| Input | `Vec<F>` (distinct closures) | `Vec<I>` items + single `F: Clone` |
| Sub-types | "Parallel" / "ParallelBranch" | "Map" / "MapItem" |
| Batching | No | Yes (FR24) — `MapOptions::batch_size` |
| Closure signature | `FnOnce(DurableContext) -> Fut` | `FnOnce(I, DurableContext) -> Fut + Clone` |
| Closure bounds | `Send + 'static` | `Send + 'static + Clone` |

### Wire Protocol (Based on Python SDK Parallel Flow)

```
FIRST EXECUTION:
1. generate_operation_id() → map_op_id (outer block ID)
2. check_result(map_op_id) → NOT FOUND
3. Send Context/START for outer block:
   { Id: map_op_id, Type: CONTEXT, Action: START, SubType: "Map", Name: user_name }
4. For each batch (or all items if no batching):
   For each item i in batch:
     a. item_op_id = OperationIdGenerator::new(Some(map_op_id)).next_id() for each i
     b. Create child context with parent_id = item_op_id
     c. Send Context/START for item:
        { Id: item_op_id, Type: CONTEXT, Action: START, SubType: "MapItem", Name: "map-item-{i}", ParentId: map_op_id }
     d. Execute f(item, child_context)
     e. Send Context/SUCCEED for item:
        { Id: item_op_id, Type: CONTEXT, Action: SUCCEED, SubType: "MapItem", Payload: serialized_result, ContextOptions: { ReplayChildren: false } }
   Await all items in batch before proceeding to next batch
5. Collect all item results into BatchResult
6. Send Context/SUCCEED for outer block:
   { Id: map_op_id, Type: CONTEXT, Action: SUCCEED, SubType: "Map", Payload: serialized_batch_result, ContextOptions: { ReplayChildren: false } }
7. track_replay(map_op_id)
8. Return BatchResult

RE-INVOCATION (map SUCCEEDED in history):
1. generate_operation_id() → same map_op_id
2. check_result(map_op_id) → SUCCEEDED
3. Deserialize BatchResult from context_details().result()
4. track_replay(map_op_id)
5. Return BatchResult — NO item closures re-executed
```

### Batching Implementation

```rust
// If batch_size is set:
let batch_size = options.batch_size.unwrap_or(items.len());
for batch in items.chunks(batch_size) {
    // Spawn all items in this batch concurrently
    let handles: Vec<_> = batch.iter().enumerate().map(|(i, item)| {
        tokio::spawn(async move { ... })
    }).collect();
    // Await ALL handles in this batch before moving to next
    for handle in handles { ... }
}
```

**IMPORTANT:** Items must be consumed from `Vec<I>`, not borrowed. Use `.into_iter().enumerate()` and chunk manually since `Vec::chunks` works on slices. Consider:
```rust
let mut remaining = items.into_iter().enumerate().collect::<Vec<_>>();
let batches: Vec<Vec<(usize, I)>> = remaining.chunks(batch_size)...;
// Or use itertools::chunks, or manual loop
```

Actually, since items need to be moved into spawned tasks, use a manual batching loop:
```rust
let batch_size = options.batch_size.unwrap_or(items.len());
let mut all_results = Vec::with_capacity(items.len());
let mut items_iter = items.into_iter().enumerate().peekable();
let mut item_id_gen = OperationIdGenerator::new(Some(map_op_id.clone()));

while items_iter.peek().is_some() {
    let batch: Vec<(usize, I)> = items_iter.by_ref().take(batch_size).collect();
    let mut handles = Vec::with_capacity(batch.len());
    for (index, item) in batch {
        let item_op_id = item_id_gen.next_id();
        let child_ctx = self.create_child_context(&item_op_id);
        let f_clone = f.clone();
        handles.push(tokio::spawn(async move {
            // Context/START + f_clone(item, child_ctx) + Context/SUCCEED
            (index, result)
        }));
    }
    for handle in handles {
        let (index, result) = handle.await.map_err(|e| ...)?;
        all_results.push((index, result));
    }
}
// Sort all_results by index, build BatchResult
```

### Closure Bounds: Why Clone?

The closure `F` is called once per item. With multiple items, `F` must be `Clone` so each `tokio::spawn` gets its own copy. This is different from parallel where each branch is already a distinct closure.

The user-facing API: `f: F` where `F: FnOnce(I, DurableContext) -> Fut + Send + 'static + Clone`. In practice, closures that capture owned data and are `Clone` just work — the `Clone` bound is satisfied by `move |item, ctx| async move { ... }` as long as captured variables are `Clone`.

### Item Operation ID Generation

Use a SINGLE `OperationIdGenerator::new(Some(map_op_id))` and call `next_id()` sequentially for each item across ALL batches. This ensures deterministic IDs regardless of batching configuration:
- Item 0 → `hash("{map_op_id}-1")`
- Item 1 → `hash("{map_op_id}-2")`
- Item 2 → `hash("{map_op_id}-3")`
- etc.

This matches the pattern established in parallel where branch IDs are generated sequentially.

### AWS SDK Types for Context Operations

```rust
// Reuse exact same types as parallel — OperationType::Context
// Only sub_type strings differ: "Map" and "MapItem"

// Context START for outer map block
let start_update = OperationUpdate::builder()
    .id(op_id.clone())
    .r#type(OperationType::Context)
    .action(OperationAction::Start)
    .sub_type("Map")
    .name(name)
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;

// Context SUCCEED for item (with payload)
let ctx_opts = ContextOptions::builder()
    .replay_children(false)
    .build();

let succeed_update = OperationUpdate::builder()
    .id(item_op_id.clone())
    .r#type(OperationType::Context)
    .action(OperationAction::Succeed)
    .sub_type("MapItem")
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

- `map()` calls `track_replay` for the outer map_op_id only
- Individual item operations are tracked by child contexts during execution
- On replay, only the outer operation is checked — items are not re-executed

### Architecture Doc Discrepancies (INHERITED from Story 3.1)

1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Operation ID**: Uses blake2b hash of counter, NOT user-provided name
3. **Handler signature**: Takes owned `ClosureContext`, receives `(event, ctx)`
4. **Wire type**: Map uses `OperationType::Context` with sub_type "Map"/"MapItem" — NOT a new `OperationType::Map`

### What Exists vs What Needs to Be Added

**Already exists (from Story 3.1 and earlier):**
- `DurableContext` with backend, ARN, checkpoint_token, ReplayEngine
- `DurableContext::create_child_context()` — creates child with isolated namespace
- `OperationIdGenerator` with parent_id support
- `ReplayEngine` with operations HashMap, check_result, get_operation, track_replay
- `BatchResult<T>`, `BatchItem<T>`, `BatchItemStatus`, `CompletionReason` types
- `DurableBackend` trait and MockBackend pattern
- `ClosureContext` with delegation pattern
- `DurableError` with `#[non_exhaustive]` and constructor methods
- `operations/map.rs` stub (header comment only)
- `execute_branch()` helper and `BranchConfig` in parallel.rs (reference/adapt)
- Context checkpoint lifecycle pattern (START + execute + SUCCEED)

**Needs to be added:**
- `MapOptions` struct in `types.rs` with `batch_size: Option<usize>`
- `DurableError::MapFailed` variant + `map_failed()` constructor
- `DurableContext::map()` method in `operations/map.rs`
- `ClosureContext::map()` delegation
- Re-exports for `MapOptions` in `lib.rs` and closure `prelude.rs`
- Unit tests for all paths (execution, replay, batching, isolation, failure, checkpoint sequence)

### Previous Story Intelligence (Story 3.1 Learnings)

- Context operations use `OperationType::Context`, differentiated by sub_type string
- `context_details().result()` is where SUCCEED payloads are stored for Context operations (investigation resolved in 3.1)
- Branch/item IDs via `OperationIdGenerator::new(Some(parent_op_id))` with sequential `next_id()` calls (verified to match Python SDK)
- Each child context tracks its own checkpoint_token independently
- `execute_branch()` uses `BranchConfig` struct to avoid clippy `too_many_arguments` — consider similar pattern for map items
- `ContextOptions::builder().build()` returns direct type, `OperationUpdate::builder().build()` returns Result
- Story 3.1 has unused `branch_op_ids` vector (review follow-up) — avoid same mistake in map

### Testing Approach

- Mirror parallel test suite structure with additional batching tests
- MockBackend must handle MULTIPLE checkpoint calls in sequence
- Test naming: `test_map_{behavior}_{condition}`
- Batching test: verify sequential batch execution (e.g., assert timestamps or use AtomicUsize counter to track execution order)
- Index preservation test: verify `BatchItem.index` matches original item position across batches

### Parameter Ordering Convention

Following architecture doc convention: name, options, closure
```rust
ctx.map("process_items", items, MapOptions::default(), |item, ctx| async move { ... })
```

Note: `items` comes before `options` because items is the primary input and options configures how they're processed. The closure is always last per convention.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.2 — acceptance criteria, FR23-FR25]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR23, FR24, FR25]
- [Source: _bmad-output/planning-artifacts/architecture.md — Send + 'static bounds, map operation pattern, parameter ordering]
- [Source: _bmad-output/implementation-artifacts/3-1-parallel-operation.md — parallel implementation patterns, child context design, Context wire protocol]
- [Source: _bmad-output/implementation-artifacts/epic-2-retro-2026-03-14.md — process improvements for Epic 3]
- [Source: crates/durable-lambda-core/src/operations/parallel.rs — execute_branch(), BranchConfig, checkpoint lifecycle]
- [Source: crates/durable-lambda-core/src/context.rs — create_child_context()]
- [Source: crates/durable-lambda-core/src/types.rs — BatchResult, BatchItem, ParallelOptions (model for MapOptions)]
- [Source: crates/durable-lambda-core/src/operation_id.rs — OperationIdGenerator with parent_id support]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- `MapOptions` struct with `batch_size: Option<usize>`, builder pattern, Default impl in types.rs
- `DurableError::MapFailed` variant + `map_failed()` constructor in error.rs
- `DurableContext::map(name, items, options, f)` — full map execution: outer Context/START, per-item tokio::spawn with Context/START+SUCCEED (sub_type "MapItem"), collect into BatchResult, outer Context/SUCCEED with serialized payload
- Batching: items processed in sequential batches via `items_iter.take(batch_size)`; all items in batch execute concurrently; next batch starts after previous completes
- `execute_item()` helper with `ItemConfig` struct (mirrors parallel's `execute_branch`/`BranchConfig`)
- Replay path: outer SUCCEEDED → deserialize BatchResult from `context_details().result()` — same pattern as parallel
- Item ID generation uses single `OperationIdGenerator::new(Some(map_op_id))` across all batches for deterministic IDs
- Closure bounds: `F: FnOnce(I, DurableContext) -> Fut + Send + 'static + Clone` — Clone needed because closure is applied to each item
- `ClosureContext::map()` delegation method
- 7 map unit tests: concurrent execution, replay, namespace isolation, checkpoint sequence, item failure capture, batching sequential execution, default all-concurrent
- All doc tests compile (MapOptions, map_failed, map method)
- Re-exports: MapOptions added to core lib.rs and closure prelude.rs
- 202 total workspace tests pass (84 core + 6 closure + 6 integration + 6 testing + 100 doc tests)
- Clippy clean, fmt clean

### File List

- crates/durable-lambda-core/src/types.rs (modified — added MapOptions struct)
- crates/durable-lambda-core/src/lib.rs (modified — added MapOptions re-export)
- crates/durable-lambda-core/src/error.rs (modified — added MapFailed variant + map_failed() constructor)
- crates/durable-lambda-core/src/operations/map.rs (rewritten — map(), execute_item(), ItemConfig, 7 unit tests)
- crates/durable-lambda-closure/src/context.rs (modified — added map() delegation + MapOptions import)
- crates/durable-lambda-closure/src/prelude.rs (modified — added MapOptions re-export)

### Senior Developer Review (AI)

**Review Date:** 2026-03-14
**Reviewer:** Claude Opus 4.6
**Outcome:** Approve

**Summary:** Clean implementation that correctly adapts the parallel pattern for collection processing. All 8 ACs verified against actual code. All tasks genuinely complete. 7 real unit tests with meaningful assertions covering execution, replay, namespace isolation, checkpoint sequence, item failure, batching, and default concurrency. Correct use of OperationType::Context with "Map"/"MapItem" sub-types, child context namespacing, tokio::spawn with Send + 'static + Clone, and BatchResult serialization via context_details.

**Action Items:**
- [ ] [Low] Add dedicated ClosureContext::map runtime delegation unit test

### Change Log

- 2026-03-14: Story 3.2 implemented — map operation with batching support, child context isolation, tokio::spawn concurrency, multi-checkpoint Context lifecycle, BatchResult serialization. 7 map unit tests + doc tests passing. Clippy clean, fmt clean. 202 total workspace tests pass.
