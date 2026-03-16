# Phase 2: Boundary & Replay Engine Tests - Research

**Researched:** 2026-03-16
**Domain:** Rust async testing — boundary conditions and replay engine robustness for a durable execution SDK
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TEST-12 | Zero-duration wait — `wait("name", 0)` behavior | `wait()` signature is `duration_secs: i32`; AWS SDK `WaitOptions::wait_seconds(0)` is likely valid; zero-duration wait will execute the START checkpoint path and return `WaitSuspended` since no completed op exists in history |
| TEST-13 | Map with batch_size edge cases — 0, 1, greater than collection size | `MapOptions::batch_size(0)` already panics via `assert!(size > 0)` in `types.rs`; batch_size=1 creates one task per batch sequentially; batch_size > collection size is handled via `max(1)` in `map.rs` line 157 |
| TEST-14 | Parallel with 0 branches and 1 branch | `parallel()` accepts `Vec<F>` so 0 branches is valid Rust; engine spawns 0 tasks, `results` is empty, outer START+SUCCEED is still sent; 1 branch exercised normally |
| TEST-15 | Operation names — empty string, unicode characters, 255+ characters | Operation name flows to `OperationUpdate::builder().name(name)`; AWS SDK may impose string constraints; needs test to discover runtime behavior (panic vs error vs success) |
| TEST-16 | Negative option values — retries(-1), backoff_seconds(-1), timeout_seconds(0) | Already validated in `types.rs` — all three panic with clear messages; tests confirm the panic messages match expected format |
| TEST-17 | Deeply nested child contexts — 5+ levels | `create_child_context()` chains `OperationIdGenerator::new(Some(parent_id))`; 5-level nesting means child-of-child-of-child-of-child-of-child; IDs use `blake2b("{parent_id}-{counter}")`; deeply nested IDs must be verified for correct structure |
| TEST-18 | Nested parallel inside child context inside parallel (3-level nesting) | Each context level gets isolated `ReplayEngine` with its own `OperationIdGenerator`; operations HashMap is shared via clone; 3-level nesting requires careful verification that ID namespaces don't collide |
| TEST-19 | Deterministic replay — same history produces identical results across 100 runs | `OperationIdGenerator` is counter-based; `blake2b` is deterministic; 100 runs with the same pre-loaded history should produce identical `check_result` lookups and identical operation IDs |
| TEST-20 | Duplicate operation IDs in history — behavior is defined | `HashMap<String, Operation>` in `ReplayEngine`; if two ops share the same ID, last-writer-wins in the `HashMap`; this is defined behavior by HashMap semantics |
| TEST-21 | History gap — missing operation IDs between existing ones | The engine uses a `HashSet` of completed IDs; gaps (IDs 1, 3 present but not 2) cause `check_result` for the gap to return None, triggering execute path despite later ops being in replay state |
| TEST-22 | Checkpoint token evolution — token changes after each checkpoint verified | `MockBackend` returns `checkpoint_token` from constructor; context calls `set_checkpoint_token(new_token)` after each checkpoint; `CheckpointCall.checkpoint_token` in `CheckpointRecorder` records the token used per call |
</phase_requirements>

---

## Summary

Phase 2 adds tests that prove boundary conditions and edge cases behave predictably across all options, operation name formats, nesting depths, and replay engine semantics. The codebase is already well-structured for this: the `ReplayEngine` is a simple `HashMap<String, Operation>` + `HashSet<String>` pair, and `OperationIdGenerator` is a pure deterministic counter-based hasher.

**Key discoveries from code inspection:**

1. **Option validation is already done at construction** (Phase 4 work completed). `StepOptions::retries(-1)`, `backoff_seconds(-1)`, `MapOptions::batch_size(0)`, and `CallbackOptions::timeout_seconds(0)` all panic with descriptive messages already in `types.rs`. TEST-16 tests confirm these panic messages.

2. **Zero-duration wait** is an interesting boundary: `wait("name", 0)` uses `i32` for `duration_secs`, and zero is a valid `i32`. The `WaitOptions::builder().wait_seconds(0)` call will succeed. The wait will checkpoint a START with 0 seconds, server will handle it (immediately complete), but in test context the mock will return `WaitSuspended`. This is expected behavior — the test documents what happens.

3. **Parallel with 0 branches** is fully valid Rust: `Vec<F>` can be empty. The code path sends outer START, creates 0 tasks, builds a `BatchResult { results: vec![], completion_reason: AllCompleted }`, sends outer SUCCEED. This is valid, defined, tested behavior.

4. **Map batch_size > collection size**: Line 157 in `map.rs` — `let batch_size = options.get_batch_size().unwrap_or(item_count).max(1)` — handles this gracefully. All items go into one batch (the `take(batch_size)` just takes all).

5. **Duplicate IDs in history**: HashMap last-writer-wins. If history is pre-populated with two operations sharing the same ID, the second overrides the first. This is deterministic and documented.

6. **History gaps**: The `completed_ids` HashSet only tracks operations present at initialization. If operation at counter=2 is missing from history but counter=1 and counter=3 are present, the engine stays in `Replaying` mode until counter=1 and counter=3 are both visited (counter=2 is never in `completed_ids`). When counter=2 is encountered during execution, `check_result` returns None, triggering new execution — this is correct replay behavior for incomplete history.

7. **Deterministic replay**: `OperationIdGenerator::next_id()` is fully deterministic — counter increments from 0, blake2b is a pure function. Running the same handler against the same pre-loaded history 100 times will produce identical IDs and identical result lookups every time. A test looping 100 iterations with identical mock state confirms this invariant holds.

**Primary recommendation:** Write all Phase 2 tests as a new test file `tests/e2e/tests/boundary_conditions.rs`, following the same pattern as `error_paths.rs`. Keep tests self-contained with local mock backends where needed, or use `MockDurableContext` builder for replay tests.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `durable-lambda-core` | workspace (local) | `DurableContext`, `ReplayEngine`, `OperationIdGenerator`, all operations | System under test |
| `durable-lambda-testing` | workspace (local) | `MockDurableContext`, `MockBackend`, `CheckpointRecorder`, `OperationRecorder` | Designed for credential-free testing |
| `tokio` | workspace (1.x) | `#[tokio::test]` async test runtime | All async tests require this |
| `aws_sdk_lambda::types::*` | workspace | `Operation`, `OperationStatus`, `OperationType`, `StepDetails`, `ContextDetails` | Constructing mock history entries |
| `aws_smithy_types::DateTime` | workspace | Required field for `Operation::builder().start_timestamp()` | Every Operation needs a timestamp |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `std::sync::Arc` | std | Share backends across async contexts | MockBackend always wrapped in Arc |
| `async_trait` | workspace | `#[async_trait]` for `DurableBackend` impl on test backends | Custom mock backends need this |

**Installation:** No new dependencies needed. All required crates are already in the workspace.

---

## Architecture Patterns

### Recommended Project Structure

New test file location:
```
tests/e2e/tests/
├── e2e_workflows.rs           # existing — happy path workflows
├── error_paths.rs             # existing — Phase 1 error path tests
└── boundary_conditions.rs     # NEW — Phase 2 boundary & replay tests
```

All option validation tests (TEST-16) can alternatively live as unit tests within `crates/durable-lambda-core/src/types.rs` alongside the existing `step_options_rejects_negative_retries` tests — but since those already exist from Phase 4 work, TEST-16 simply confirms the existing panics.

### Pattern 1: Pre-populated MockDurableContext for Replay Tests

**What:** Use `MockDurableContext` builder for replay-path boundary tests (TEST-19, TEST-22). Pre-load history, run the handler, assert behavior.

**When to use:** TEST-19 (deterministic replay — same history → identical results 100x), TEST-22 (checkpoint token evolution during execute path).

**Example:**
```rust
// Source: mock_context.rs — MockDurableContext builder pattern
#[tokio::test]
async fn test_deterministic_replay_100_runs() {
    for _ in 0..100 {
        let (mut ctx, calls, _ops) = MockDurableContext::new()
            .with_step_result("step_a", r#"42"#)
            .with_step_result("step_b", r#""hello""#)
            .build()
            .await;

        let r1: Result<i32, String> = ctx.step("step_a", || async { panic!("not executed") }).await.unwrap();
        let r2: Result<String, String> = ctx.step("step_b", || async { panic!("not executed") }).await.unwrap();

        assert_eq!(r1.unwrap(), 42);
        assert_eq!(r2.unwrap(), "hello");

        // No checkpoints in replay
        let captured = calls.lock().await;
        assert_eq!(captured.len(), 0);
    }
}
```

### Pattern 2: Direct DurableContext::new() with Custom HashMap for Engine Tests

**What:** Construct `DurableContext` directly with a pre-built operations `HashMap` for precise control over duplicate IDs, gaps, and replay state. Used for TEST-20, TEST-21.

**When to use:** When you need non-standard operation ID patterns that `MockDurableContext` builder doesn't support (e.g., two ops with the same ID, or a gap in the ID sequence).

**Example:**
```rust
// Source: context.rs unit test pattern + operation_id.rs
use durable_lambda_core::operation_id::OperationIdGenerator;
use std::collections::HashMap;

#[tokio::test]
async fn test_duplicate_ids_last_writer_wins() {
    let mut gen = OperationIdGenerator::new(None);
    let op_id = gen.next_id();  // counter=1 → first ID

    // Build initial_operations with TWO operations sharing the same ID.
    // The DurableContext::new() loads them into a HashMap, second wins.
    let op_first = Operation::builder()
        .id(&op_id)
        .r#type(OperationType::Step)
        .status(OperationStatus::Succeeded)
        .start_timestamp(DateTime::from_secs(0))
        .step_details(StepDetails::builder().result(r#"999"#).build())
        .build().unwrap();

    let op_second = Operation::builder()
        .id(&op_id)  // same ID
        .r#type(OperationType::Step)
        .status(OperationStatus::Succeeded)
        .start_timestamp(DateTime::from_secs(0))
        .step_details(StepDetails::builder().result(r#"42"#).build())
        .build().unwrap();

    // DurableContext::new() takes initial_operations as Vec<Operation>.
    // The HashMap construction is `op.id().to_string() → op`, so op_second
    // overrides op_first.
    let (backend, _, _) = MockBackend::new("mock-token");
    let mut ctx = DurableContext::new(
        Arc::new(backend),
        "arn:test".into(),
        "tok".into(),
        vec![op_first, op_second],  // second overwrites first in HashMap
        None,
    ).await.unwrap();

    let result: Result<i32, String> = ctx.step("any_name", || async { panic!("not executed") }).await.unwrap();
    assert_eq!(result.unwrap(), 42);  // second op (value 42) wins
}
```

### Pattern 3: Direct Operation ID Computation for Nesting Tests

**What:** Pre-compute expected operation IDs using `OperationIdGenerator` to verify that deeply nested child context operations produce the correct IDs at each level.

**When to use:** TEST-17 (5-level nesting), TEST-18 (3-level parallel-in-child-in-parallel).

**Example — computing expected IDs for 5-level nesting:**
```rust
// Source: operation_id.rs — OperationIdGenerator chain logic
// Level 0 (root): id = blake2b("1")
// Level 1 (child of L0): id = blake2b("{L0_id}-1")
// Level 2 (child of L1): id = blake2b("{L1_id}-1")
// ...
let mut root_gen = OperationIdGenerator::new(None);
let l0_id = root_gen.next_id();

let mut l1_gen = OperationIdGenerator::new(Some(l0_id.clone()));
let l1_id = l1_gen.next_id();

let mut l2_gen = OperationIdGenerator::new(Some(l1_id.clone()));
let l2_id = l2_gen.next_id();
// ... etc. for 5 levels
```

### Pattern 4: Loop + Verify for Determinism Test (TEST-19)

**What:** Run the same operation sequence 100 times in a loop, capturing checkpoint calls each run, asserting identical token sequences and operation IDs.

**When to use:** TEST-19 only. Also useful for verifying that parallel/map determinism holds across multiple runs.

### Anti-Patterns to Avoid

- **Do not use `HashMap::new()` directly for operation history**: `DurableContext::new()` takes `Vec<Operation>` and converts internally. Construct operations as a Vec and let the context build the HashMap.
- **Do not pre-compute operation IDs manually as hex strings**: Always use `OperationIdGenerator::next_id()` to compute expected IDs — hardcoding blake2b output will break if the algorithm changes.
- **Do not assume parallel branch order is deterministic for assertions**: `tokio::spawn` order is non-deterministic. Assert on `results[i].index` not `results[i]` directly.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Operation ID computation | Custom hex hash | `OperationIdGenerator::next_id()` | Must match Python SDK — divergence breaks replay |
| Mock checkpoint backend | New backend structs per test | `MockBackend::new()` from `durable-lambda-testing` | Already handles recording, token returns, OperationRecorder |
| Mock replay context | Manual `HashMap` + `ReplayEngine` construction | `MockDurableContext::builder()` for standard cases | Automatically computes correct operation IDs for pre-loaded ops |
| Unicode string assertions | Manual byte-counting | Standard `str.len()` vs `str.chars().count()` | Rust `String` len() is bytes; chars().count() is code points |

---

## Common Pitfalls

### Pitfall 1: Operation Name vs Operation ID Confusion
**What goes wrong:** Tests assume operation name determines replay matching, but replay uses operation ID (position-based blake2b hash). `wait("name", 0)` with name "anything" still produces the same ID — counter-based.
**Why it happens:** Natural assumption that names are keys.
**How to avoid:** Always use `OperationIdGenerator::next_id()` to compute expected IDs; never use the operation name as a lookup key.
**Warning signs:** Test that works for one name fails for another name identically placed.

### Pitfall 2: Parallel Branch Order Non-Determinism
**What goes wrong:** Test asserts `result.results[0].result == Some("branch-a")` but sometimes gets `Some("branch-b")` because tokio schedules tasks non-deterministically.
**Why it happens:** `tokio::spawn` ordering is not guaranteed.
**How to avoid:** Sort by `results[i].index` (already done by `map.rs`) before asserting, or assert that the set of values is correct rather than the ordered sequence.
**Warning signs:** Flaky tests that pass sometimes and fail sometimes.

### Pitfall 3: Empty Parallel Sends Checkpoints Anyway
**What goes wrong:** Test for `parallel("name", vec![], opts)` asserts no checkpoints, but outer START + SUCCEED are still sent even with 0 branches.
**Why it happens:** The outer Context/START is always sent before branches are spawned.
**How to avoid:** Expect exactly 2 checkpoints (START + SUCCEED) even for 0 branches.
**Warning signs:** `assert_no_checkpoints` assertion fails on empty parallel test.

### Pitfall 4: `wait_seconds(0)` May Not Be Rejected by AWS SDK Builder
**What goes wrong:** Expecting `wait("name", 0)` to panic, but it doesn't — `WaitOptions::builder().wait_seconds(0)` is valid and sends a 0-second wait. The operation suspends via `WaitSuspended` error.
**Why it happens:** The AWS SDK doesn't validate the duration value at the Rust type level.
**How to avoid:** Test that `wait("name", 0)` returns `Err(WaitSuspended)` on execute path, and `Ok(())` on replay path. Document this as "defined behavior: zero duration is valid".

### Pitfall 5: Duplicate IDs in DurableContext::new() Vec Input
**What goes wrong:** Two operations with the same ID in the `initial_operations` Vec — HashMap overwrites the first. Test must account for this by knowing HashMap insert order (second wins) when creating test state.
**Why it happens:** `initial_operations.into_iter().map(|op| (op.id().to_string(), op)).collect()` is the construction in `context.rs` line 92-95. HashMap `.collect()` for duplicate keys takes last-writer.
**How to avoid:** For TEST-20, deliberately put the "wrong" op first and the "right" op second to confirm last-writer-wins semantics.
**Warning signs:** Assertion gets the first-op value, not the second.

### Pitfall 6: MockDurableContext Builder ID Generation Alignment
**What goes wrong:** `MockDurableContext` generates op IDs with its own `OperationIdGenerator::new(None)`, starting from counter=1. When you call `ctx.step()` N times, it also advances the counter. For replay tests to work, the number of `with_step_result()` calls must match the number of `ctx.step()` calls exactly, in the same order.
**Why it happens:** IDs are position-based. Mismatch in count causes execute-path execution instead of replay.
**How to avoid:** One-to-one correspondence between `with_step_result/with_wait/etc.` calls and actual operation calls in the test.
**Warning signs:** Step closure actually executes during what should be replay.

---

## Code Examples

Verified patterns from source inspection:

### Creating a Completed Operation for History Pre-loading
```rust
// Source: error_paths.rs + context.rs test pattern
use aws_sdk_lambda::types::{Operation, OperationStatus, OperationType, StepDetails};
use aws_smithy_types::DateTime;

fn make_step_op(id: &str, result_json: &str) -> Operation {
    Operation::builder()
        .id(id)
        .r#type(OperationType::Step)
        .status(OperationStatus::Succeeded)
        .start_timestamp(DateTime::from_secs(0))
        .step_details(StepDetails::builder().result(result_json).build())
        .build()
        .unwrap()
}
```

### Computing Correct Operation IDs for Nested Contexts
```rust
// Source: operation_id.rs — exact algorithm for child ID generation
use durable_lambda_core::operation_id::OperationIdGenerator;

// Root context: first op ID = blake2b("1")
let mut root = OperationIdGenerator::new(None);
let root_op1_id = root.next_id();  // counter=1

// Child context under root_op1: first child op ID = blake2b("{root_op1_id}-1")
let mut child = OperationIdGenerator::new(Some(root_op1_id.clone()));
let child_op1_id = child.next_id();  // counter=1 under root_op1

// Grandchild context: first grandchild op ID = blake2b("{child_op1_id}-1")
let mut grandchild = OperationIdGenerator::new(Some(child_op1_id.clone()));
let grandchild_op1_id = grandchild.next_id();
```

### Verifying Checkpoint Token Evolution (TEST-22)
```rust
// Source: mock_backend.rs CheckpointCall + context.rs set_checkpoint_token
// Each checkpoint call records the token used at the time of the call.
// The context calls set_checkpoint_token() after each successful checkpoint.

#[tokio::test]
async fn test_checkpoint_token_evolves() {
    // Use MockDurableContext in execute mode (no pre-loaded ops)
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;
    // MockBackend always returns "mock-token" for every checkpoint

    let _: Result<i32, String> = ctx.step("s1", || async { Ok(1) }).await.unwrap();
    let _: Result<i32, String> = ctx.step("s2", || async { Ok(2) }).await.unwrap();

    let captured = calls.lock().await;
    // s1 START uses "mock-checkpoint-token" (initial), s1 SUCCEED uses "mock-token" (returned from START)
    // s2 START uses "mock-token" (updated after s1 SUCCEED), s2 SUCCEED uses "mock-token"
    // First call uses initial token; subsequent calls use token returned by previous checkpoint
    assert_eq!(captured[0].checkpoint_token, "mock-checkpoint-token");
    assert_eq!(captured[1].checkpoint_token, "mock-token"); // mock returns same token always
    assert_eq!(captured[2].checkpoint_token, "mock-token");
}
```

### Operation Name Edge Cases (TEST-15)
```rust
// Source: wait.rs — name flows directly to OperationUpdate::builder().name(name)
// The AWS SDK builder accepts &str. Edge cases to test:
// 1. Empty string: name("") — likely valid, name field is optional in AWS SDK
// 2. Unicode: name("こんにちは世界") — UTF-8 string, should work
// 3. 255+ chars: name(&"a".repeat(300)) — may be truncated or rejected by AWS
// In test context with MockBackend, the name is just recorded. Behavior is:
// - With MockBackend: all names accepted (no AWS validation)
// - Document expected behavior (SDK does not validate locally)

#[tokio::test]
async fn test_operation_names_edge_cases() {
    // Empty string name — executes normally with MockBackend
    let (mut ctx, _, ops) = MockDurableContext::new().build().await;
    let _: Result<i32, String> = ctx.step("", || async { Ok(1) }).await.unwrap();
    let recorded = ops.lock().await;
    assert_eq!(recorded[0].name, "");

    // Unicode name — executes normally
    let (mut ctx, _, ops) = MockDurableContext::new().build().await;
    let _: Result<i32, String> = ctx.step("こんにちは", || async { Ok(1) }).await.unwrap();
    let recorded = ops.lock().await;
    assert_eq!(recorded[0].name, "こんにちは");

    // 255+ character name — executes normally with MockBackend
    let long_name = "a".repeat(300);
    let (mut ctx, _, ops) = MockDurableContext::new().build().await;
    let _: Result<i32, String> = ctx.step(&long_name, || async { Ok(1) }).await.unwrap();
    let recorded = ops.lock().await;
    assert_eq!(recorded[0].name, long_name);
}
```

### Parallel with Zero Branches (TEST-14)
```rust
// Source: parallel.rs — branches.len() == 0 → empty handles Vec → empty results
#[tokio::test]
async fn test_parallel_zero_branches() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;

    type BranchFn = Box<dyn FnOnce(DurableContext)
        -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>>
        + Send>;
    let branches: Vec<BranchFn> = vec![];

    let result: BatchResult<i32> = ctx.parallel("empty", branches, ParallelOptions::new()).await.unwrap();
    assert_eq!(result.results.len(), 0);

    // 2 checkpoints: outer Context/START + outer Context/SUCCEED
    let captured = calls.lock().await;
    assert_eq!(captured.len(), 2);
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `StepOptions::retries` was `u32` (no negative rejection) | Changed to `i32` with `assert!(count >= 0)` guard | Phase 4 (2026-03-16) | TEST-16 tests confirm panics at construction, not at use |
| No structured error codes | `DurableError::code()` returns stable SCREAMING_SNAKE_CASE | Phase 4 (2026-03-16) | Tests can now match on `err.code()` instead of parsing display strings |
| Silent `if let Some(token)` on checkpoint response | `ok_or_else` error propagation for missing tokens | Phase 4 (2026-03-16) | `None` checkpoint token now surfaces as `CHECKPOINT_FAILED` |

---

## Open Questions

1. **Zero-duration wait server behavior**
   - What we know: `wait("name", 0)` sends `WaitOptions { wait_seconds: 0 }` to the AWS SDK builder; with `MockBackend` it will always return `WaitSuspended` since no completed op exists in history.
   - What's unclear: In production with the real backend, does AWS immediately complete a 0-second wait, or does it treat it as "wait forever"? This is irrelevant for Phase 2 (pure unit tests with mocks) but worth noting in documentation.
   - Recommendation: Document that `wait("name", 0)` is defined to send a 0-second wait START checkpoint and return `WaitSuspended` on execute path. Replay path returns `Ok(())` as with any completed wait.

2. **Operation name length limits**
   - What we know: The AWS SDK `OperationUpdate::builder().name()` accepts `&str`. MockBackend records it verbatim. No Rust-level validation exists.
   - What's unclear: Does the AWS Lambda Durable Execution API impose a maximum name length? If so, what's the limit?
   - Recommendation: For Phase 2, test with MockBackend only — all lengths are accepted locally. Document that AWS-side validation is not tested and may reject very long names in production.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `tokio::test` (from workspace tokio dep) |
| Config file | `Cargo.toml` at workspace root — `[profile.test]` |
| Quick run command | `cargo test -p e2e-tests boundary -- --test-output immediate` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TEST-12 | `wait("x", 0)` on execute path returns `WaitSuspended`; on replay path returns `Ok(())` | unit | `cargo test -p e2e-tests test_zero_duration_wait` | ❌ Wave 0 |
| TEST-13 | `MapOptions::batch_size(0)` panics; `batch_size(1)` sequential; `batch_size > collection` all-concurrent | unit | `cargo test -p durable-lambda-core map_options` | Partial (panic test exists in types.rs) |
| TEST-14 | `parallel("x", vec![], opts)` returns empty `BatchResult`; `parallel` with 1 branch works | unit | `cargo test -p e2e-tests test_parallel_zero_branches` | ❌ Wave 0 |
| TEST-15 | `step("")`, `step("unicode")`, `step("255+chars")` record names correctly; no panic | unit | `cargo test -p e2e-tests test_operation_name_edge_cases` | ❌ Wave 0 |
| TEST-16 | `StepOptions::retries(-1)` panics; `backoff_seconds(-1)` panics; `CallbackOptions::timeout_seconds(0)` panics | unit | `cargo test -p durable-lambda-core step_options_rejects` | Partial (already in types.rs) |
| TEST-17 | 5-level `child_context` nesting produces correct operation IDs at each level | unit | `cargo test -p e2e-tests test_five_level_nested_child_contexts` | ❌ Wave 0 |
| TEST-18 | `parallel` inside `child_context` inside `parallel` (3-level) — IDs don't collide, results correct | unit | `cargo test -p e2e-tests test_parallel_in_child_in_parallel` | ❌ Wave 0 |
| TEST-19 | Same history replayed 100 times produces identical `BatchResult` and zero checkpoints every time | unit | `cargo test -p e2e-tests test_deterministic_replay_100_runs` | ❌ Wave 0 |
| TEST-20 | Duplicate op IDs in history — last-writer wins in HashMap; first op value is discarded | unit | `cargo test -p e2e-tests test_duplicate_operation_ids` | ❌ Wave 0 |
| TEST-21 | History gap — missing op ID causes execute path for that position even during replay mode | unit | `cargo test -p e2e-tests test_history_gap_behavior` | ❌ Wave 0 |
| TEST-22 | Checkpoint token in each `CheckpointCall` equals token returned by previous checkpoint response | unit | `cargo test -p e2e-tests test_checkpoint_token_evolution` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p e2e-tests boundary`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/e2e/tests/boundary_conditions.rs` — covers TEST-12, TEST-13 (execute path), TEST-14, TEST-15, TEST-17, TEST-18, TEST-19, TEST-20, TEST-21, TEST-22
- [ ] TEST-16 confirmations: `tests/e2e/tests/boundary_conditions.rs` (the panic tests already exist in `types.rs` unit tests from Phase 4 — new tests in boundary_conditions.rs confirm via `#[should_panic]` at the integration level)
- [ ] TEST-13 batch_size=1 sequential and batch_size > collection size: new test cases in `boundary_conditions.rs` (complementing the panic test that already exists in `types.rs`)

*(No new test infrastructure is needed — the existing `e2e-tests` crate already has all required dependencies configured.)*

---

## Sources

### Primary (HIGH confidence)
- Direct source inspection of `/home/esa/git/durable-rust/crates/durable-lambda-core/src/` — `replay.rs`, `operation_id.rs`, `context.rs`, `types.rs`, `error.rs`
- Direct source inspection of `/home/esa/git/durable-rust/crates/durable-lambda-core/src/operations/` — `wait.rs`, `parallel.rs`, `map.rs`, `child_context.rs`, `step.rs`
- Direct source inspection of `/home/esa/git/durable-rust/crates/durable-lambda-testing/src/` — `mock_context.rs`, `mock_backend.rs`
- Direct source inspection of existing tests — `tests/e2e/tests/error_paths.rs`, `tests/e2e/tests/e2e_workflows.rs`
- `/home/esa/git/durable-rust/CLAUDE.md` — project-specific conventions and architecture

### Secondary (MEDIUM confidence)
- `.planning/REQUIREMENTS.md` — requirement IDs and descriptions
- `.planning/STATE.md` — accumulated decisions from Phases 1-4
- `.planning/ROADMAP.md` — phase dependencies and success criteria
- `.planning/phases/01-error-path-test-coverage/01-RESEARCH.md` — established testing patterns from Phase 1

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already in workspace; direct code inspection
- Architecture: HIGH — all patterns derived from existing source code and Phase 1 patterns
- Operation behavior at boundaries: HIGH — code paths traced directly from source
- AWS-side behavior (e.g., real `wait_seconds(0)` outcome): LOW — not tested in this SDK; documented as out-of-scope for Phase 2

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (stable codebase; valid until Phase 2 production changes are made)
