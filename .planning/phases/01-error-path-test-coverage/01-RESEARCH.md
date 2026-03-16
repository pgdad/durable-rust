# Phase 1: Error Path Test Coverage - Research

**Researched:** 2026-03-16
**Domain:** Rust async testing — error path coverage for a durable execution SDK
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TEST-01 | Replay mismatch detection — step expects type A but history has type B returns DurableError::replay_mismatch | ReplayEngine.check_result + extract_step_result are the trigger points; mismatch arises from wrong OperationStatus or mismatched deserialization |
| TEST-02 | Serialization failure — step result type mismatch between closure return and history data | extract_step_result calls serde_json::from_str; feeding incompatible JSON triggers DurableError::Deserialization |
| TEST-03 | Checkpoint failure — AWS SDK error during checkpoint write (network timeout, invalid token) | MockBackend.checkpoint can be made to return Err(DurableError::checkpoint_failed(...)) to inject failures |
| TEST-04 | Retry exhaustion — step with retries(3) fails all 4 attempts and returns final error | step_with_options executes closure at attempt 4; max_retries check is `(current_attempt as u32) <= max_retries` |
| TEST-05 | Callback timeout expiration — callback exceeds timeout_seconds and returns error | callback_result matches OperationStatus::TimedOut and returns DurableError::CallbackFailed |
| TEST-06 | Callback explicit failure signal — callback receives failure from external system | callback_result matches OperationStatus::Failed and returns DurableError::CallbackFailed |
| TEST-07 | Invoke error — target Lambda returns error payload | invoke() checks non-Succeeded completed status and returns DurableError::InvokeFailed |
| TEST-08 | Parallel all-branches-fail — all parallel branches return errors | parallel() collects branch errors into BatchResult.results with BatchItemStatus::Failed |
| TEST-09 | Map item failures at different positions — first, middle, last item failures | map() captures per-item errors in BatchResult; all items complete regardless of individual failures |
| TEST-10 | Step closure panic — panic in user closure does not crash context | step() calls f().await directly; panic propagates; needs catch_unwind or tokio::spawn wrapper |
| TEST-11 | Parallel branch panic — panic in one branch doesn't affect others | parallel branch uses tokio::spawn; JoinError is caught via handle.await.map_err(...) |
</phase_requirements>

---

## Summary

Phase 1 adds tests that prove every explicit failure mode in the SDK surfaces the correct typed `DurableError` variant. The SDK is already well-structured for this: all error paths produce named constructors (`DurableError::replay_mismatch`, `DurableError::checkpoint_failed`, etc.), and the `MockBackend` / `MockDurableContext` infrastructure eliminates AWS dependency from all tests.

The most important gap is TEST-10 (step closure panic). The current `step_with_options` implementation calls `f().await` directly with no `catch_unwind` guard. A panic in a step closure will unwind through the calling code, not be caught and converted to `DurableError`. This is a real behavioral gap that the tests will expose — and the implementation will need a fix (see Architecture Patterns section). The parallel operation is already safe because `tokio::spawn` catches panics as `JoinError`.

All other tests (TEST-01 through TEST-09, TEST-11) are pure assertion tests: construct the right mock state, invoke the operation, match the error variant. No new production code is needed for these — only new test code.

**Primary recommendation:** Write tests in `tests/e2e/tests/` using the `MockDurableContext` builder and a custom `FailingMockBackend` that returns `Err(...)` on demand. Fix TEST-10 by wrapping the step closure in `tokio::task::spawn_blocking` or `AssertUnwindSafe` + `catch_unwind` during the execute path.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tokio` | workspace (1.x) | Async runtime; `tokio::spawn` for parallel branches | Already in workspace; all existing tests use it |
| `serde` / `serde_json` | workspace | Serialize/deserialize step results; inject bad JSON for TEST-02 | Already used in all operation modules |
| `durable-lambda-core` | workspace (local) | `DurableContext`, `DurableError`, all operations | The system under test |
| `durable-lambda-testing` | workspace (local) | `MockDurableContext`, `MockBackend`, assertion helpers | Designed for exactly this purpose |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `std::panic::catch_unwind` / `AssertUnwindSafe` | std | Wrap async closures to catch panics in TEST-10 fix | Required if implementing panic catching in step() |
| `aws_sdk_lambda::types::*` | workspace | `Operation`, `OperationStatus`, `StepDetails`, `ErrorObject`, etc. | Constructing mock history operations for error scenarios |
| `aws_smithy_types::DateTime` | workspace | Required field for `Operation::builder()` | All Operation construction needs `start_timestamp` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom `FailingMockBackend` per-test | Shared configurable backend | Custom per-test is simpler and already the pattern in `step.rs`/`parallel.rs` unit tests |
| Adding new test file in `tests/e2e/tests/` | Adding to existing `e2e_workflows.rs` | New file is cleaner; keeps error-path tests separate from happy-path workflow tests |

**Installation:** No new dependencies needed. All required crates are already in the workspace.

---

## Architecture Patterns

### Recommended Project Structure

New test file location:
```
tests/e2e/tests/
├── e2e_workflows.rs      # existing — happy path workflows
└── error_paths.rs        # NEW — Phase 1 error path tests
```

Optionally, unit tests for TEST-10 fix can live in `crates/durable-lambda-core/src/operations/step.rs` alongside existing unit tests.

### Pattern 1: FailingMockBackend for Checkpoint Failure (TEST-03)

**What:** A `MockBackend` variant that returns `Err(DurableError::checkpoint_failed(...))` on the Nth call or unconditionally.

**When to use:** TEST-03 — proving checkpoint write failures propagate as `DurableError::CheckpointFailed` to the caller.

**Example:**
```rust
// Source: derived from existing step.rs unit test MockBackend pattern
use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;
use durable_lambda_core::backend::DurableBackend;
use durable_lambda_core::error::DurableError;
use aws_sdk_lambda::operation::checkpoint_durable_execution::CheckpointDurableExecutionOutput;
use aws_sdk_lambda::operation::get_durable_execution_state::GetDurableExecutionStateOutput;
use aws_sdk_lambda::types::OperationUpdate;

struct FailingBackend {
    fail_on_call: usize,          // 0 = always fail, N = fail on Nth call (1-indexed)
    call_count: Arc<Mutex<usize>>,
}

#[async_trait]
impl DurableBackend for FailingBackend {
    async fn checkpoint(
        &self,
        _arn: &str,
        _checkpoint_token: &str,
        _updates: Vec<OperationUpdate>,
        _client_token: Option<&str>,
    ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
        let mut count = self.call_count.lock().await;
        *count += 1;
        if self.fail_on_call == 0 || *count == self.fail_on_call {
            return Err(DurableError::checkpoint_failed(
                "test_op",
                std::io::Error::new(std::io::ErrorKind::TimedOut, "network timeout"),
            ));
        }
        Ok(CheckpointDurableExecutionOutput::builder()
            .checkpoint_token("mock-token")
            .build())
    }

    async fn get_execution_state(
        &self, _: &str, _: &str, _: &str, _: i32,
    ) -> Result<GetDurableExecutionStateOutput, DurableError> {
        Ok(GetDurableExecutionStateOutput::builder().build().unwrap())
    }
}
```

### Pattern 2: Pre-loaded History with Wrong Type for Replay Mismatch (TEST-01, TEST-02)

**What:** Build an `Operation` with `OperationStatus::Succeeded` but with JSON that cannot be deserialized as the expected type. Pass it to `DurableContext::new()` as history.

**When to use:** TEST-01 (type mismatch between expected and actual), TEST-02 (serialization mismatch).

**Example:**
```rust
// Source: derived from step.rs test pattern for cached operations
use aws_sdk_lambda::types::{Operation, OperationStatus, OperationType, StepDetails};
use aws_smithy_types::DateTime;
use durable_lambda_core::operation_id::OperationIdGenerator;

// Generate the same op_id the context will generate for its first step
let mut gen = OperationIdGenerator::new(None);
let op_id = gen.next_id();

// History says the result is a boolean, but test expects i32
let wrong_type_op = Operation::builder()
    .id(&op_id)
    .r#type(OperationType::Step)
    .status(OperationStatus::Succeeded)
    .start_timestamp(DateTime::from_secs(0))
    .step_details(
        StepDetails::builder()
            .attempt(1)
            .result(r#"true"#)  // boolean, not i32
            .build(),
    )
    .build()
    .unwrap();

// ctx.step("name", ...) will attempt to deserialize "true" as i32
// and return DurableError::Deserialization
let result: Result<Result<i32, String>, DurableError> = ctx
    .step("step_name", || async { Ok(99) })
    .await;

assert!(matches!(result, Err(DurableError::Deserialization { .. })));
```

### Pattern 3: Retry Exhaustion via Pre-loaded Pending Operation (TEST-04)

**What:** Simulate a step already at its Nth attempt (matching `retries(N-1)`) by pre-loading a `Pending` operation with `StepDetails.attempt = N`. The step executes the closure and fails, then checks `current_attempt <= max_retries`. Since attempt == max_retries + 1, it checkpoints FAIL instead of RETRY.

**When to use:** TEST-04 — verifying retry exhaustion produces `Ok(Err(user_error))`, not `Err(StepRetryScheduled)`.

**Example:**
```rust
// Source: step.rs test_step_with_options_retry_exhaustion
// Simulate attempt 4 (retries(3) means max 3 retries = 4 total attempts)
let cached_op = Operation::builder()
    .id(&expected_op_id)
    .r#type(OperationType::Step)
    .status(OperationStatus::Pending)
    .start_timestamp(DateTime::from_secs(0))
    .step_details(StepDetails::builder().attempt(4).build())
    .build()
    .unwrap();

let options = StepOptions::new().retries(3);
let result: Result<Result<i32, String>, DurableError> = ctx
    .step_with_options("exhaust_step", options, || async {
        Err("final failure".to_string())
    })
    .await;

// Retries exhausted: inner Err, not Err(StepRetryScheduled)
let inner = result.unwrap();
assert_eq!(inner.unwrap_err(), "final failure");
```

### Pattern 4: Callback Error States via Pre-loaded Operations (TEST-05, TEST-06)

**What:** Pre-load a callback operation with `OperationStatus::TimedOut` (TEST-05) or `OperationStatus::Failed` with `ErrorObject` (TEST-06). Call `create_callback` to replay the handle, then `callback_result` to retrieve the error.

**When to use:** TEST-05, TEST-06.

**Example:**
```rust
// Source: callback.rs tests test_callback_result_returns_error_on_timed_out
let callback_op = Operation::builder()
    .id(&op_id)
    .r#type(OperationType::Callback)
    .status(OperationStatus::TimedOut)  // or Failed with ErrorObject
    .name("approval")
    .start_timestamp(DateTime::from_secs(0))
    .callback_details(
        CallbackDetails::builder()
            .callback_id("cb-timeout-1")
            .build()
    )
    .build()
    .unwrap();

// ctx.create_callback replays, ctx.callback_result returns CallbackFailed
let handle = ctx.create_callback("approval", CallbackOptions::new()).await.unwrap();
let err = ctx.callback_result::<String>(&handle).unwrap_err();
assert!(matches!(err, DurableError::CallbackFailed { .. }));
```

### Pattern 5: Parallel All-Branches-Fail (TEST-08)

**What:** Provide branches that all return `Err(DurableError::...)`. The parallel operation should return `Ok(BatchResult)` where every item has `BatchItemStatus::Failed`. The outer `parallel()` call itself returns `Ok(...)`, not `Err(...)`.

**When to use:** TEST-08 — confirms that branch-level failures are captured in `BatchResult`, not propagated as SDK errors.

**Example:**
```rust
let branches: Vec<BranchFn> = vec![
    Box::new(|_ctx| Box::pin(async move {
        Err(DurableError::parallel_failed("b0", "branch 0 failed"))
    })),
    Box::new(|_ctx| Box::pin(async move {
        Err(DurableError::parallel_failed("b1", "branch 1 failed"))
    })),
];

let result = ctx.parallel("all_fail", branches, ParallelOptions::new()).await.unwrap();
assert_eq!(result.results.len(), 2);
assert!(result.results.iter().all(|r| r.status == BatchItemStatus::Failed));
assert!(result.results.iter().all(|r| r.error.is_some()));
```

### Pattern 6: Panic Catch for Step Closure (TEST-10) — Implementation Gap

**What:** The current `step_with_options` calls `f().await` directly. A panic in the closure propagates as an unwinding stack unwind — not caught and not converted to `DurableError`. This is a behavioral gap.

**Fix approach:** Wrap the closure execution in `std::panic::catch_unwind` via `tokio::task::spawn_blocking` or use `AssertUnwindSafe`. Since `f()` is async, the correct approach is:

```rust
// In step_with_options, replace:
let user_result = f().await;

// With:
use std::panic::AssertUnwindSafe;
let user_result = match std::panic::catch_unwind(AssertUnwindSafe(|| f())) {
    Ok(fut) => {
        // catch_unwind on the future itself (not the output)
        // For async, use tokio::spawn and catch JoinError
        ...
    }
    Err(_panic_payload) => {
        return Err(DurableError::checkpoint_failed(
            name,
            std::io::Error::new(std::io::ErrorKind::Other, "step closure panicked"),
        ));
    }
};
```

**Practical approach:** Wrap in `tokio::spawn` + `JoinHandle.await`:
```rust
// Source: pattern from parallel.rs execute_branch which already catches panics this way
let handle = tokio::spawn(async move { f().await });
let user_result = handle.await.map_err(|join_err| {
    DurableError::checkpoint_failed(
        name,
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("step closure panicked: {join_err}"),
        ),
    )
})??; // outer ? for JoinError->DurableError, inner would be T or E
```

**Note:** Wrapping in `tokio::spawn` changes the closure bound from `FnOnce() -> Fut` to requiring `Send + 'static` on the future. This is a breaking change if users pass non-Send closures. The safer fix uses `catch_unwind` directly on the async block if the future implements `UnwindSafe`, or adds a new `step_catching_panics` method without changing the existing API.

**Recommendation:** Add a `DurableError::StepPanicked` variant and wrap step execution in `tokio::spawn` (parallel already requires `Send` for branches; this aligns `step` with the same requirement). Document as a non-breaking change since `step` closures are already `Send + FnOnce`.

Wait — checking step's current bound: `F: FnOnce() -> Fut + Send`, `Fut: Future<Output = ...> + Send`. So closures ARE already `Send`. Wrapping in `tokio::spawn` is valid. But it adds `'static` to `Fut`. This IS a breaking change: currently step closures can borrow from the outer scope (e.g., `event.clone()` pattern from CLAUDE.md). Users already have to clone, so `'static` is likely already satisfied in practice — but it changes the formal API.

**Safer alternative (no API change):** Use `AssertUnwindSafe` + `futures::FutureExt::catch_unwind` which works without `'static`:
```rust
// futures = already in ecosystem; check if in workspace
use futures::FutureExt;
let user_result = AssertUnwindSafe(f())
    .catch_unwind()
    .await
    .map_err(|_| DurableError::checkpoint_failed(
        name,
        std::io::Error::new(std::io::ErrorKind::Other, "step closure panicked"),
    ))?;
```

Check if `futures` is in the workspace before choosing this path (see Cargo.toml).

### Anti-Patterns to Avoid

- **Don't assert on `DurableError` display strings for correctness:** Use `matches!` macro or pattern destructuring to check variant identity. Display strings can change without being a bug.
- **Don't use `unwrap()` in error-path tests without intent:** Use `expect("test description")` so failures report clearly.
- **Don't share a single `MockBackend` across parallel branches in the same test:** Parallel branches use `Arc<backend>` concurrently; tests that check call counts must account for concurrent checkpoint ordering.
- **Don't mix replay-path and execute-path assertions in the same test:** Tests are cleaner when each tests exactly one mode.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Mock AWS backend | Custom per-file MockBackend struct | `durable-lambda-testing::MockBackend` | Already built, records calls, supports operation tracking |
| Operation ID generation for mock history | Hardcoded hex strings | `OperationIdGenerator::new(None).next_id()` | Must match exactly what `DurableContext` generates or replay will fail |
| Checkpoint response construction | Raw struct init | `CheckpointDurableExecutionOutput::builder().checkpoint_token(...).build()` | AWS SDK builder pattern is the only valid construction path |
| Operation construction | Raw struct init | `Operation::builder()...build().unwrap()` with all required fields | Missing required fields cause build to return Err |

**Key insight:** The `OperationIdGenerator` algorithm is blake2b-keyed and position-based. Any mock that pre-loads operations MUST use the same generator to produce matching IDs. The existing unit tests in `step.rs` show the pattern: `let mut gen = OperationIdGenerator::new(None); let op_id = gen.next_id();`.

---

## Common Pitfalls

### Pitfall 1: Operation ID Mismatch in Replay Tests
**What goes wrong:** Test pre-loads an Operation with a hardcoded ID, but the DurableContext generates a different ID at runtime. The replay path is never triggered; the test enters execute mode and checkpoints instead of replaying.
**Why it happens:** Operation IDs are `blake2b("{counter}")` — deterministic but not guessable without running the generator.
**How to avoid:** Always use `OperationIdGenerator::new(None).next_id()` (or `.next_id()` N times for the Nth operation) to generate IDs for mock history.
**Warning signs:** Test expects `assert_no_checkpoints` but sees checkpoints; or test expects a cached value but gets the live closure result.

### Pitfall 2: Retry Count Off-by-One
**What goes wrong:** Test sets `retries(3)` and expects 4 total attempts, but the test scenario only simulates attempt 3 (Pending) — so the step retries one more time instead of exhausting.
**Why it happens:** `current_attempt <= max_retries` means: if `max_retries = 3` and `current_attempt = 4`, retries are exhausted. Pre-loading `attempt(3)` is not exhausted (3 <= 3 is true), pre-loading `attempt(4)` is (4 <= 3 is false).
**How to avoid:** For retry exhaustion tests, pre-load `StepDetails::builder().attempt(max_retries + 1)`.
**Warning signs:** Test expects `Ok(Err(user_error))` but gets `Err(StepRetryScheduled)`.

### Pitfall 3: Step Closure Panic Does Not Return DurableError (Currently)
**What goes wrong:** TEST-10 — writing a test that asserts `step()` returns `Err(...)` after a closure panic will fail because the panic unwinds instead of being caught.
**Why it happens:** `f().await` has no panic boundary. Rust panics unwind through async await points in tokio by default.
**How to avoid:** The test must first implement the fix (wrap in tokio::spawn or catch_unwind), then write the assertion. The test file should be created, but TEST-10 requires a production code change before it can pass.
**Warning signs:** Test runner crashes with "thread panicked at..." instead of reporting a test assertion failure.

### Pitfall 4: Parallel Branch Panic Already Works
**What goes wrong:** Writing TEST-11 expecting it to fail before writing a fix — but it already works.
**Why it happens:** `tokio::spawn` in `parallel()` catches panics as `JoinError` with `is_panic() == true`. The `handle.await.map_err(...)` converts this to `DurableError::ParallelFailed`.
**How to avoid:** Write the test first to confirm the existing behavior — it should pass immediately.
**Warning signs:** None; this is a positive finding that means less implementation work.

### Pitfall 5: MockBackend Call Count Races in Parallel Tests
**What goes wrong:** Asserting exact checkpoint counts in parallel tests fails intermittently due to ordering.
**Why it happens:** Parallel branches use separate `tokio::spawn` tasks and make checkpoint calls concurrently. The order of checkpoint calls in the `Vec<CheckpointCall>` is non-deterministic.
**How to avoid:** Assert on call count (>= N) and action types, not on exact ordering of branch checkpoints. The pattern is already in `parallel.rs` tests: `assert!(captured.len() >= 6, ...)`.

---

## Code Examples

### Building a Context with Pre-loaded Error History
```rust
// Source: crates/durable-lambda-core/src/operations/step.rs test_step_returns_cached_error_in_replaying_mode
use durable_lambda_core::operation_id::OperationIdGenerator;
use aws_sdk_lambda::types::{ErrorObject, Operation, OperationStatus, OperationType, StepDetails};

let mut gen = OperationIdGenerator::new(None);
let op_id = gen.next_id();

let error_data = r#""payment_failed""#;
let cached_op = Operation::builder()
    .id(&op_id)
    .r#type(OperationType::Step)
    .status(OperationStatus::Failed)
    .start_timestamp(aws_smithy_types::DateTime::from_secs(0))
    .step_details(
        StepDetails::builder()
            .attempt(1)
            .error(
                ErrorObject::builder()
                    .error_type("PaymentError")
                    .error_data(error_data)
                    .build(),
            )
            .build(),
    )
    .build()
    .unwrap();
```

### Simulating Checkpoint Failure
```rust
// Source: pattern from crates/durable-lambda-core/src/operations/step.rs MockBackend
// FailingMockBackend always returns Err from checkpoint()
#[async_trait::async_trait]
impl DurableBackend for FailingMockBackend {
    async fn checkpoint(
        &self,
        _arn: &str,
        _token: &str,
        _updates: Vec<OperationUpdate>,
        _client_token: Option<&str>,
    ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
        Err(DurableError::checkpoint_failed(
            "test_step",
            std::io::Error::new(std::io::ErrorKind::TimedOut, "simulated network timeout"),
        ))
    }
    // get_execution_state returns Ok (only checkpoint fails)
    async fn get_execution_state(&self, ...) -> Result<..., DurableError> {
        Ok(GetDurableExecutionStateOutput::builder().build().unwrap())
    }
}
```

### Testing Parallel Branch Panic Capture
```rust
// Source: parallel.rs — handle.await catches JoinError (panic)
// TEST-11: branch panics but parallel() returns Ok(BatchResult) with Failed item
let branches: Vec<BranchFn> = vec![
    Box::new(|_ctx| Box::pin(async move { Ok(42i32) })),
    Box::new(|_ctx| Box::pin(async move {
        panic!("deliberate branch panic");
    })),
];

let result = ctx.parallel("panic_test", branches, ParallelOptions::new()).await;
// parallel() ITSELF should return Ok (not propagate the JoinError up)
// The JoinError for the panicking branch becomes a DurableError::ParallelFailed
// which surfaces as: result is Err(DurableError::ParallelFailed)
// because handle.await.map_err returns Err which then hits the ? operator
```

Wait — re-reading `parallel.rs`:
```rust
let branch_outcome = handle.await.map_err(|e| {
    DurableError::parallel_failed(name, format!("branch {i} panicked: {e}"))
})?;  // <-- The ? here propagates the DurableError up!
```

So when a branch PANICS, `parallel()` returns `Err(DurableError::ParallelFailed)` — NOT `Ok(BatchResult)`. This is different from when a branch returns `Err(DurableError::...)` (which gets captured in `BatchResult`).

TEST-11 should assert: `parallel()` returns `Err(DurableError::ParallelFailed { ... })` when a branch panics.

### Map Item Position Failures (TEST-09)
```rust
// Test failures at index 0 (first), middle, and last
let items = vec![0i32, 1, 2, 3, 4];
let result = ctx.map(
    "position_test",
    items,
    MapOptions::new(),
    |item: i32, _ctx: DurableContext| async move {
        if item == 0 || item == 2 || item == 4 {
            Err(DurableError::map_failed("item", format!("item {item} failed")))
        } else {
            Ok(item * 10)
        }
    },
).await.unwrap();

assert_eq!(result.results.len(), 5);
assert_eq!(result.results[0].status, BatchItemStatus::Failed);  // first
assert_eq!(result.results[1].status, BatchItemStatus::Succeeded);
assert_eq!(result.results[2].status, BatchItemStatus::Failed);  // middle
assert_eq!(result.results[3].status, BatchItemStatus::Succeeded);
assert_eq!(result.results[4].status, BatchItemStatus::Failed);  // last
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Direct `panic!` in tests for "should not reach" | `#[tokio::test]` with proper mock state | At project start | No change needed |
| Ad-hoc MockBackend per file | `MockDurableContext` builder | Already implemented | Use the builder for high-level tests; ad-hoc for low-level unit tests in operations/ |

**Deprecated/outdated:**
- Building DurableContext directly with raw `Arc<MockBackend>` in new tests: prefer `MockDurableContext::new().build()` when testing workflow-level behavior; use the direct approach only when testing checkpoint call details.

---

## Open Questions

1. **Does `futures` crate exist in the workspace?**
   - What we know: `tokio` is present; `serde`, `thiserror`, `async-trait`, `blake2` are in workspace deps.
   - What's unclear: Whether `futures::FutureExt::catch_unwind` is available without adding a new dep.
   - Recommendation: Check `Cargo.toml` before choosing the panic-catch strategy. If `futures` is absent, use `tokio::spawn` approach for TEST-10 fix.

2. **Should TEST-10 fix be in this phase or Phase 4?**
   - What we know: Phase 4 handles input validation and error code improvements. TEST-10 is in Phase 1 requirements.
   - What's unclear: Whether the team wants to fix the panic behavior in Phase 1 (writing the test exposes the gap) or defer.
   - Recommendation: Write the test in Phase 1 with `#[ignore]` if the fix is deferred; document that the test is blocked on a production code change.

3. **Exact behavior when all parallel branches panic (vs. all return Err)**
   - What we know: A single branch panic causes `parallel()` to return `Err(ParallelFailed)` via the `?` on `handle.await.map_err(...)`.
   - What's unclear: If branch 0 panics and branch 1 returns `Ok`, does parallel still return `Err`? Yes — because the panic abort propagates immediately via `?`.
   - Recommendation: TEST-11 should test: (a) one branch panics, one succeeds → `Err(ParallelFailed)`, (b) all branches return `Err(...)` → `Ok(BatchResult)` with all items failed. These are separate behaviors.

---

## Validation Architecture

> No `.planning/config.json` found — `nyquist_validation` key absent, treating as enabled.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test framework via `cargo test` |
| Config file | `Cargo.toml` workspace with `[dev-dependencies]` per crate |
| Quick run command | `cargo test -p e2e-tests error_paths` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TEST-01 | Replay mismatch → DurableError::Deserialization (or ReplayMismatch) | unit | `cargo test -p durable-lambda-core error_path_tests::replay_mismatch` | ❌ Wave 0 |
| TEST-02 | Type mismatch in history → DurableError::Deserialization, not panic | unit | `cargo test -p durable-lambda-core error_path_tests::serialization_mismatch` | ❌ Wave 0 |
| TEST-03 | Checkpoint API fails → DurableError::CheckpointFailed propagates | unit | `cargo test -p e2e-tests error_paths::checkpoint_failure` | ❌ Wave 0 |
| TEST-04 | Step retries(3) exhausts 4 attempts → Ok(Err(user_error)) | unit | `cargo test -p durable-lambda-core step::tests::retry_exhaustion` (exists) / `cargo test -p e2e-tests error_paths::retry_exhaustion_e2e` | Partial (unit test exists in step.rs) |
| TEST-05 | Callback TimedOut → DurableError::CallbackFailed | unit | `cargo test -p durable-lambda-core callback::tests::callback_timeout` (exists) / `cargo test -p e2e-tests error_paths::callback_timeout` | Partial (unit test exists) |
| TEST-06 | Callback Failed → DurableError::CallbackFailed | unit | `cargo test -p durable-lambda-core callback::tests::callback_failed` (exists) | Partial (unit test exists) |
| TEST-07 | Invoke Failed/TimedOut → DurableError::InvokeFailed | unit | `cargo test -p durable-lambda-core invoke::tests::invoke_failed` (exists) | Partial (unit test exists) |
| TEST-08 | All parallel branches fail → Ok(BatchResult with all Failed) | integration | `cargo test -p e2e-tests error_paths::parallel_all_branches_fail` | ❌ Wave 0 |
| TEST-09 | Map item failures at first/middle/last positions | integration | `cargo test -p e2e-tests error_paths::map_item_failures_positions` | ❌ Wave 0 |
| TEST-10 | Step closure panic → DurableError (not process abort) | unit+fix | `cargo test -p durable-lambda-core step::tests::step_closure_panic` | ❌ Wave 0 (requires production fix) |
| TEST-11 | Parallel branch panic → DurableError::ParallelFailed | integration | `cargo test -p e2e-tests error_paths::parallel_branch_panic` | ❌ Wave 0 |

**Note on existing coverage:** Several requirements (TEST-04 through TEST-07) have relevant unit tests in the operation source files, but those tests test the replay path (returning the cached error). The Phase 1 tests add:
- Execute-path verification that errors actually propagate through the full stack
- Integration-level tests through `MockDurableContext`
- Tests for error variants that have no existing tests (TEST-01, TEST-02, TEST-03, TEST-08, TEST-09, TEST-10, TEST-11)

### Sampling Rate
- **Per task commit:** `cargo test -p e2e-tests error_paths`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/e2e/tests/error_paths.rs` — main test file covering all TEST-01 through TEST-11
- [ ] `FailingMockBackend` struct (can be defined inline in `error_paths.rs` or in a shared `tests/e2e/src/lib.rs` helper)
- [ ] Production fix in `crates/durable-lambda-core/src/operations/step.rs` for TEST-10 (panic catching)
- [ ] Possibly: `DurableError::StepPanicked` variant addition in `error.rs` for TEST-10

*(Existing test infrastructure in `tests/e2e/tests/e2e_workflows.rs` covers happy paths; this phase adds the error path complement file.)*

---

## Sources

### Primary (HIGH confidence)
- Direct source code reading of `/home/esa/git/durable-rust/crates/durable-lambda-core/src/operations/step.rs` — step retry logic, current_attempt calculation, closure execution
- Direct source code reading of `/home/esa/git/durable-rust/crates/durable-lambda-core/src/operations/parallel.rs` — JoinError panic capture behavior
- Direct source code reading of `/home/esa/git/durable-rust/crates/durable-lambda-core/src/operations/callback.rs` — TimedOut/Failed status handling
- Direct source code reading of `/home/esa/git/durable-rust/crates/durable-lambda-core/src/operations/invoke.rs` — InvokeFailed propagation
- Direct source code reading of `/home/esa/git/durable-rust/crates/durable-lambda-core/src/operations/map.rs` — per-item error capture behavior
- Direct source code reading of `/home/esa/git/durable-rust/crates/durable-lambda-core/src/error.rs` — all DurableError variants and constructors
- Direct source code reading of `/home/esa/git/durable-rust/crates/durable-lambda-testing/src/mock_context.rs` — MockDurableContext builder API
- Direct source code reading of `/home/esa/git/durable-rust/crates/durable-lambda-testing/src/mock_backend.rs` — MockBackend structure
- Direct source code reading of `/home/esa/git/durable-rust/crates/durable-lambda-testing/src/assertions.rs` — available assertion helpers
- Direct source code reading of `/home/esa/git/durable-rust/tests/e2e/tests/e2e_workflows.rs` — existing test patterns

### Secondary (MEDIUM confidence)
- Rust standard library documentation on `std::panic::catch_unwind` and `AssertUnwindSafe` — behavior of panic catching in async contexts
- Tokio documentation on `spawn` panic behavior — `JoinError::is_panic()` confirmed from tokio docs pattern used in parallel.rs

### Tertiary (LOW confidence)
- `futures::FutureExt::catch_unwind` availability — not verified; workspace Cargo.toml not fully read; treat as needing verification before use

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies identified from workspace; no new deps needed
- Architecture: HIGH — test patterns derived directly from existing test code in the codebase
- Error variant behavior: HIGH — read directly from source code
- TEST-10 fix approach: MEDIUM — two viable approaches identified; `futures` crate availability uncertain
- Pitfalls: HIGH — derived from exact code analysis (retry off-by-one from line-by-line reading of step_with_options)

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (stable Rust codebase; no external dependencies changing)
