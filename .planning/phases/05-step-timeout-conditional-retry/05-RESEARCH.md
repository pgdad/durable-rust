# Phase 5: Step Timeout & Conditional Retry - Research

**Researched:** 2026-03-16
**Domain:** Rust async timeouts, closure predicate types, DurableContext extension
**Confidence:** HIGH

## Summary

Phase 5 adds two independent capabilities to `StepOptions`: a per-step execution timeout
(`timeout_seconds`) that wraps the closure in `tokio::time::timeout`, and a conditional
retry predicate (`retry_if`) that gates retry scheduling on whether the user error
satisfies a closure. Both features are pure extensions of existing
`StepOptions`/`step_with_options` code; no new checkpointing protocol entries or replay
engine changes are required.

The timeout feature is straightforward: `tokio::time::timeout` (already a transitive
dependency via the `tokio = { features = ["full"] }` workspace dep) wraps the spawned
task and, on expiry, maps the error to a new `DurableError::StepTimeout` variant. The
conditional-retry feature requires a new field in `StepOptions` that stores `Option<Box<dyn
Fn(&E) -> bool + Send + Sync>>`. Because the error type `E` is generic and not known at
`StepOptions` construction time, the predicate must be stored in type-erased form and
threaded through the retry decision point in `step_with_options`.

The cross-approach parity tests (TEST-23, TEST-24, TEST-25) follow the established pattern
in `tests/parity/tests/parity.rs`: all wrapper contexts delegate to `DurableContext`, so
tests against `DurableContext` directly cover parity for the closure/trait/builder crates.

**Primary recommendation:** Add `timeout_seconds: Option<u64>` and
`retry_if: Option<Box<dyn Fn(&dyn std::any::Any) -> bool + Send + Sync>>` to
`StepOptions`; integrate both into `step_with_options` in
`crates/durable-lambda-core/src/operations/step.rs`; add `DurableError::StepTimeout`
variant.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FEAT-09 | `StepOptions` gains `.timeout_seconds(u64)` field | Add `timeout_seconds: Option<u64>` with builder method and getter; `u64` chosen because negative timeout is nonsensical — validation requires > 0 (consistent with `CallbackOptions::timeout_seconds`) |
| FEAT-10 | Step closure wrapped in `tokio::time::timeout` when timeout is set | `tokio::time::timeout(Duration::from_secs(secs), handle).await` inside `step_with_options` after `tokio::spawn`; on `Err(Elapsed)` map to `DurableError::StepTimeout` |
| FEAT-11 | Step exceeding timeout returns `DurableError::step_timeout` with operation name | New `StepTimeout { operation_name }` variant in `DurableError`, constructor `step_timeout(name)`, error code `"STEP_TIMEOUT"` |
| FEAT-12 | Tests for step timeout (exceeds, completes within, zero timeout) | Unit tests in `step.rs` + e2e tests in `tests/e2e/tests/`; zero timeout panics at construction (consistent with validation pattern) |
| FEAT-13 | `StepOptions` gains `.retry_if(Fn(&E) -> bool)` predicate | Type-erased storage as `Option<Arc<dyn Fn(&dyn Any) -> bool + Send + Sync>>`; builder method accepts `impl Fn(&E) -> bool + Send + Sync + 'static` |
| FEAT-14 | Retry only when predicate returns true; non-matching errors fail immediately | At the retry decision point in `step_with_options`, if `retry_if` is `Some(pred)`, downcast `&error` via `Any` and call predicate; false → skip to FAIL checkpoint |
| FEAT-15 | Default predicate (no `retry_if`) retries all errors (backward compatible) | `retry_if` field defaults to `None`; existing code path `(current_attempt as u32) <= max_retries` continues to apply; predicate only consulted when `Some` |
| FEAT-16 | Tests for conditional retry (transient retries, non-transient fails fast) | Tests covering predicate-true path (retry scheduled), predicate-false path (immediate FAIL), and no-predicate path (unchanged behavior) |
| TEST-23 | Same workflow through all 4 API styles produces identical operation sequences | Parity test in `tests/parity/tests/parity.rs`; uses `DurableContext` directly (all wrappers delegate); tests step with timeout and retry_if |
| TEST-24 | Complex workflow parity — parallel + map + child_context across all approaches | Extend parity test file with a multi-operation workflow that includes a step with timeout in a parallel branch |
| TEST-25 | BatchItemStatus verification — per-item success/failure status in parallel/map | Assertions on `BatchResult.results[i].status` using `BatchItemStatus::Succeeded` / `BatchItemStatus::Failed` |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tokio::time::timeout` | 1.50.0 (workspace) | Wraps async futures with a deadline | Already a `tokio = { features = ["full"] }` dep; zero new dependencies |
| `std::any::Any` | stdlib | Type-erased downcast for predicate storage | Enables storing `Fn(&E) -> bool` without making `StepOptions` generic |
| `Arc<dyn Fn>` | stdlib | Shared ownership for the predicate closure | Allows `StepOptions` to be `Clone`; `Fn` (not `FnOnce`) because predicate is called once per retry attempt but stored for multiple uses |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thiserror` | 2.0.18 (workspace) | New `StepTimeout` variant derivation | Always — matches existing `DurableError` pattern |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `Box<dyn Fn(&dyn Any)>` | `Box<dyn Fn(&E)>` (generic `StepOptions<E>`) | Generic `StepOptions` would infect `DurableContextOps` trait signatures and every caller; type-erased approach preserves the existing `StepOptions::default()` ergonomics |
| `Arc<dyn Fn>` for predicate | `Box<dyn Fn>` | `Arc` required because `StepOptions` must be `Clone` (it implements `Clone` and is passed by value); `Box` is not `Clone` |
| `tokio::time::timeout` wrapping the spawn handle | `tokio::time::timeout` wrapping the closure future directly | Spawning first then timing the handle means the panic-catch path still works; wrapping the future directly would bypass the `tokio::spawn` panic safety |

**Installation:**
```bash
# No new dependencies — tokio::time is already in scope via features = ["full"]
```

---

## Architecture Patterns

### Recommended Project Structure

Changes are confined to:
```
crates/durable-lambda-core/src/
├── error.rs          # Add StepTimeout variant + constructor + code
├── types.rs          # Add timeout_seconds, retry_if to StepOptions
└── operations/
    └── step.rs       # Integrate timeout and retry_if into step_with_options

tests/e2e/tests/
└── step_timeout_retry.rs   # New test file: FEAT-12, FEAT-16 tests

tests/parity/tests/
└── parity.rs          # Extend: TEST-23, TEST-24, TEST-25
```

### Pattern 1: Timeout Integration in step_with_options

**What:** Wrap the `tokio::spawn` handle in `tokio::time::timeout` when `options.get_timeout_seconds()` is `Some`.

**When to use:** Always inside `step_with_options`, after `tokio::spawn(f())`.

**Example:**
```rust
// Source: tokio docs (https://docs.rs/tokio/latest/tokio/time/fn.timeout.html)
let name_owned = name.to_string();
let handle = tokio::spawn(async move { f().await });

let user_result = if let Some(secs) = options.get_timeout_seconds() {
    match tokio::time::timeout(
        std::time::Duration::from_secs(secs),
        handle,
    ).await {
        Ok(join_result) => join_result.map_err(|join_err| {
            DurableError::checkpoint_failed(
                &name_owned,
                std::io::Error::other(format!("step closure panicked: {join_err}")),
            )
        })?,
        Err(_elapsed) => {
            return Err(DurableError::step_timeout(&name_owned));
        }
    }
} else {
    handle.await.map_err(|join_err| {
        DurableError::checkpoint_failed(
            &name_owned,
            std::io::Error::other(format!("step closure panicked: {join_err}")),
        )
    })?
};
```

### Pattern 2: Type-Erased Predicate Storage

**What:** Store the retry predicate in `StepOptions` as `Option<Arc<dyn Fn(&dyn std::any::Any) -> bool + Send + Sync>>`. The builder method captures the concrete `&E` into an `Any`-based closure at construction time.

**When to use:** In `StepOptions` for `retry_if` field.

**Example:**
```rust
// In types.rs — StepOptions field:
retry_if: Option<Arc<dyn Fn(&dyn std::any::Any) -> bool + Send + Sync>>,

// Builder method:
pub fn retry_if<E, P>(mut self, predicate: P) -> Self
where
    E: 'static,
    P: Fn(&E) -> bool + Send + Sync + 'static,
{
    self.retry_if = Some(Arc::new(move |any_err: &dyn std::any::Any| {
        if let Some(e) = any_err.downcast_ref::<E>() {
            predicate(e)
        } else {
            false  // wrong type — treat as non-retryable
        }
    }));
    self
}

// Getter:
pub fn get_retry_if(&self) -> Option<&Arc<dyn Fn(&dyn std::any::Any) -> bool + Send + Sync>> {
    self.retry_if.as_ref()
}
```

### Pattern 3: Predicate Evaluation at Retry Decision Point

**What:** In `step_with_options`, when the user closure returns `Err`, evaluate `retry_if` before deciding to RETRY or FAIL.

**When to use:** At the existing `if (current_attempt as u32) <= max_retries` branch.

**Example:**
```rust
// In step.rs — inside Err(error) match arm:
let should_retry = if let Some(pred) = options.get_retry_if() {
    pred(error as &dyn std::any::Any)
} else {
    true  // no predicate — retry all errors (backward compatible)
};

if (current_attempt as u32) <= max_retries && should_retry {
    // checkpoint RETRY (existing code)
} else {
    // checkpoint FAIL (existing code)
}
```

**CRITICAL:** `error` here is a `&E` from the `Err(error)` arm. Passing it as `&dyn Any` works when `E: 'static` — which is already required by `step_with_options`'s `E: DeserializeOwned + Send + 'static` bound.

### Pattern 4: New DurableError Variant

**What:** Add `StepTimeout` variant following the exact style of existing variants.

**When to use:** Returned from `step_with_options` when `tokio::time::timeout` elapses.

**Example:**
```rust
// In error.rs:
/// A step exceeded its configured timeout.
#[error("step timed out for operation '{operation_name}'")]
#[non_exhaustive]
StepTimeout { operation_name: String },

// Constructor:
pub fn step_timeout(operation_name: impl Into<String>) -> Self {
    Self::StepTimeout { operation_name: operation_name.into() }
}

// In code() match:
Self::StepTimeout { .. } => "STEP_TIMEOUT",
```

### Pattern 5: Cross-Approach Parity Test Structure

**What:** Parity tests in `tests/parity/tests/parity.rs` that verify `DurableContext` produces correct results for the new features. Since all 4 approaches delegate to `DurableContext`, this covers all styles.

**When to use:** TEST-23, TEST-24, TEST-25 — follows existing parity test pattern.

**Example:**
```rust
#[tokio::test]
async fn step_timeout_parity_all_approaches() {
    // Timeout fires — DurableContext returns DurableError::StepTimeout
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;
    let result: Result<Result<i32, String>, DurableError> = ctx
        .step_with_options(
            "slow_step",
            StepOptions::new().timeout_seconds(1),
            || async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(42)
            },
        )
        .await;
    matches!(result.unwrap_err(), DurableError::StepTimeout { .. });
}
```

### Anti-Patterns to Avoid

- **Making `StepOptions` generic over `E`:** This would add a type parameter to every call site, break `Default`, and infect `DurableContextOps` trait signatures.
- **Storing `Box<dyn Fn>` instead of `Arc<dyn Fn>`:** `StepOptions` implements `Clone`; `Box<dyn Fn>` is not `Clone`. Use `Arc`.
- **Timing the closure future directly instead of the spawn handle:** Bypasses the panic-catch mechanism. Always spawn first, then timeout the join handle.
- **Using `u32` for `timeout_seconds`:** Use `u64` — timeouts can be large (hours); `u32` overflows at ~49 days. Use validation consistent with `CallbackOptions`: `timeout_seconds > 0` panics on 0.
- **Checkpointing on timeout:** The step did not complete — do NOT emit a FAIL checkpoint for a timeout. Return `DurableError::step_timeout` directly, allowing the caller to decide whether to propagate or retry.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async timeout | Manual `select!` with `tokio::time::sleep` | `tokio::time::timeout` | `timeout` is the canonical API; `select!` requires more code and is error-prone |
| Type erasure for predicate | Custom trait object protocol | `std::any::Any + downcast_ref` | Idiomatic Rust stdlib pattern for this exact use case |
| Panic safety in timed tasks | Manual catch_unwind | `tokio::spawn` + JoinError | Existing pattern already in the codebase; spawn catches panics reliably |

**Key insight:** `tokio::time::timeout` is a drop-in wrapper — it does not require any changes to the spawned task or the backend protocol.

---

## Common Pitfalls

### Pitfall 1: Timeout Does Not Abort the Spawned Task

**What goes wrong:** `tokio::time::timeout` returns `Err(Elapsed)` but the spawned task continues running in the background, consuming resources.

**Why it happens:** `timeout` does not cancel the future; it just stops waiting for it.

**How to avoid:** After `Err(Elapsed)`, call `handle.abort()` on the `JoinHandle` to send a cancellation signal. The task will be dropped at the next yield point.

**Warning signs:** Long-running tests that hang or consume memory after a timeout test.

```rust
// Correct pattern:
let handle = tokio::spawn(async move { f().await });
match tokio::time::timeout(duration, &mut handle).await {
    Ok(join_result) => { /* ... */ }
    Err(_elapsed) => {
        handle.abort();  // cancel the background task
        return Err(DurableError::step_timeout(name));
    }
}
```

Note: `tokio::time::timeout` on a `JoinHandle` ref works because `JoinHandle: Future`.

### Pitfall 2: `StepOptions` Clone Breaks With `Box<dyn Fn>`

**What goes wrong:** Adding `retry_if: Option<Box<dyn Fn(&dyn Any) -> bool + Send + Sync>>` causes a compile error because `Box<dyn Fn>` is not `Clone`, but `StepOptions` derives `Clone`.

**Why it happens:** `StepOptions` has `#[derive(Clone)]` and is currently all-`Clone` fields.

**How to avoid:** Use `Arc<dyn Fn>` — `Arc<T>` is always `Clone` by incrementing the refcount. Remove `#[derive(Clone)]` and implement `Clone` manually only if `Arc` is insufficient (it won't be).

**Warning signs:** Compiler error: "the trait `Clone` is not implemented for `Box<dyn Fn...>`".

### Pitfall 3: Predicate Receives Wrong Type — Silent False

**What goes wrong:** `downcast_ref::<E>()` returns `None` when the actual error type doesn't match what the predicate was built with (e.g., due to type inference selecting the wrong `E`).

**Why it happens:** The type-erased predicate uses `downcast_ref::<E>()` where `E` is captured at `retry_if` call time. If `E` at that call site is inferred differently from `E` in `step_with_options`, the downcast silently fails and the predicate returns `false`.

**How to avoid:** The `E` in both calls must match. Since `step_with_options` is generic over `E`, and `retry_if` captures `E` at construction, users must use explicit type annotations when building `StepOptions` if inference is ambiguous.

**Warning signs:** Retry predicate never fires; step always fails without retrying even when predicate should return `true`.

### Pitfall 4: Exhaustive Match on `DurableError::code()` — Compiler Enforced

**What goes wrong:** Adding `StepTimeout` without updating the `code()` match causes a compile error.

**Why it happens:** `code()` uses an exhaustive `match self` with no wildcard arm (design decision from Phase 4 — [04-02]).

**How to avoid:** Add `Self::StepTimeout { .. } => "STEP_TIMEOUT"` to the `code()` match. Compiler will catch the omission.

**Warning signs:** `cargo build` fails with "non-exhaustive patterns: `StepTimeout { .. }` not covered".

### Pitfall 5: Zero Timeout Validation Consistency

**What goes wrong:** Accepting `timeout_seconds(0)` silently creates a timeout that fires immediately, making the step always fail.

**Why it happens:** Without a guard, zero is a valid `u64`.

**How to avoid:** Add `assert!(seconds > 0, "StepOptions::timeout_seconds: seconds must be > 0, got {}", seconds)` — consistent with `CallbackOptions::timeout_seconds` validation pattern.

**Warning signs:** Tests that pass `timeout_seconds(0)` should use `#[should_panic]`.

---

## Code Examples

Verified patterns from official sources:

### tokio::time::timeout Usage

```rust
// Source: https://docs.rs/tokio/latest/tokio/time/fn.timeout.html
use tokio::time::{timeout, Duration};

// Timeout on a JoinHandle (Future):
let handle = tokio::spawn(some_async_fn());
match timeout(Duration::from_secs(5), handle).await {
    Ok(Ok(value)) => { /* task completed, inner Ok */ }
    Ok(Err(join_err)) => { /* task panicked */ }
    Err(_elapsed) => { /* timed out */ }
}
```

### Arc<dyn Fn> for Clone-able Type-Erased Closures

```rust
// Source: Rust stdlib Arc docs — Arc<T>: Clone when T: ?Sized
use std::sync::Arc;

let pred: Arc<dyn Fn(&dyn std::any::Any) -> bool + Send + Sync> =
    Arc::new(|any_val: &dyn std::any::Any| {
        any_val.downcast_ref::<MyError>()
            .map(|e| e.is_transient())
            .unwrap_or(false)
    });
let cloned = Arc::clone(&pred);  // O(1) clone
```

### Complete StepOptions with timeout_seconds and retry_if

```rust
// After Phase 5 — StepOptions in types.rs:
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct StepOptions {
    retries: Option<u32>,
    backoff_seconds: Option<i32>,
    timeout_seconds: Option<u64>,
    retry_if: Option<Arc<dyn Fn(&dyn std::any::Any) -> bool + Send + Sync>>,
}

// Note: #[derive(Debug)] must be removed or replaced with manual Debug impl
// because `dyn Fn` does not implement Debug.
```

### Manual Debug impl for StepOptions (required)

```rust
// StepOptions currently has #[derive(Debug, Clone, Default)]
// After adding the Fn field, derive(Debug) no longer compiles.
// Replace with:
impl std::fmt::Debug for StepOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepOptions")
            .field("retries", &self.retries)
            .field("backoff_seconds", &self.backoff_seconds)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("retry_if", &self.retry_if.as_ref().map(|_| "<predicate>"))
            .finish()
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| String-matching for retry detection | Structured error codes via `.code()` | Phase 4 | `retry_if` must use structured types, not message parsing |
| `StepOptions` stores only `retries` + `backoff_seconds` | Add `timeout_seconds` + `retry_if` | Phase 5 (this phase) | Builders must remain backward compatible — all new fields `Option`-defaulting to `None` |
| `DurableError` has no timeout variant | Add `StepTimeout` variant | Phase 5 (this phase) | Exhaustive `code()` match enforces addition |

**Deprecated/outdated:**
- None in this phase.

---

## Open Questions

1. **Should timeout trigger a FAIL checkpoint before returning?**
   - What we know: A timeout means the closure did not produce a result. There is no value or error to serialize. Checkpointing FAIL with no payload would be malformed.
   - What's unclear: Does the Python SDK checkpoint on timeout, or does it let the Lambda exit and rely on server-side timeout tracking?
   - Recommendation: Do NOT checkpoint on timeout. Return `DurableError::StepTimeout` directly and let the handler propagate. The AWS server will handle the operation via server-side step timeout if configured; local enforcement is client-side only.

2. **Should `retry_if` gate apply before or after checking `max_retries`?**
   - What we know: Per FEAT-14, "Non-matching errors fail immediately without consuming retry budget."
   - What's unclear: Does this mean the predicate check comes before the `current_attempt <= max_retries` check, so a false predicate always goes to FAIL regardless of remaining retries?
   - Recommendation: Yes — check the predicate first. If predicate returns `false`, jump to FAIL checkpoint regardless of retry budget. This matches the requirement "fail immediately without consuming retry budget."

3. **TEST-23/24/25 placement: e2e or parity crate?**
   - What we know: TEST-23 says "all 4 API styles produce identical operation sequences." TEST-24 says "complex workflow parity." The existing parity tests live in `tests/parity/tests/parity.rs`.
   - What's unclear: Whether TEST-25 (BatchItemStatus) requires new parity tests or is satisfied by existing e2e tests.
   - Recommendation: Put TEST-23 and TEST-24 in `tests/parity/tests/parity.rs` (existing parity crate). For TEST-25, add assertions on `BatchItemStatus` fields to an existing or new parity test — it only verifies existing batch result structure, not new features.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + `tokio::test` (via `tokio = { features = ["full"] }`) |
| Config file | None — `cargo test` convention |
| Quick run command | `cargo test -p durable-lambda-core step_timeout` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FEAT-09 | `StepOptions::new().timeout_seconds(5)` stores field | unit | `cargo test -p durable-lambda-core step_options_timeout` | ❌ Wave 0 |
| FEAT-09 | `timeout_seconds(0)` panics | unit | `cargo test -p durable-lambda-core step_options_timeout_zero_panics` | ❌ Wave 0 |
| FEAT-10 | Slow closure exceeds timeout, returns `StepTimeout` | unit | `cargo test -p durable-lambda-core step_timeout_fires` | ❌ Wave 0 |
| FEAT-10 | Fast closure completes within timeout, returns value | unit | `cargo test -p durable-lambda-core step_within_timeout_succeeds` | ❌ Wave 0 |
| FEAT-11 | `DurableError::step_timeout("op").code() == "STEP_TIMEOUT"` | unit | `cargo test -p durable-lambda-core step_timeout_error_code` | ❌ Wave 0 |
| FEAT-12 | Timeout exceeds | e2e | `cargo test -p e2e-tests step_timeout` | ❌ Wave 0 |
| FEAT-12 | Completes within timeout | e2e | `cargo test -p e2e-tests step_within_timeout` | ❌ Wave 0 |
| FEAT-12 | Zero timeout panics | e2e | `cargo test -p e2e-tests step_timeout_zero_panics` | ❌ Wave 0 |
| FEAT-13 | `StepOptions::retry_if` stores predicate | unit | `cargo test -p durable-lambda-core step_options_retry_if` | ❌ Wave 0 |
| FEAT-13 | `StepOptions` still `Clone` with predicate | unit | `cargo test -p durable-lambda-core step_options_clone_with_predicate` | ❌ Wave 0 |
| FEAT-14 | Predicate returns true → RETRY scheduled | unit | `cargo test -p durable-lambda-core step_conditional_retry_transient` | ❌ Wave 0 |
| FEAT-14 | Predicate returns false → immediate FAIL, no retry | unit | `cargo test -p durable-lambda-core step_conditional_retry_non_transient` | ❌ Wave 0 |
| FEAT-15 | No predicate → all errors retried (existing behavior) | unit | `cargo test -p durable-lambda-core step_no_predicate_retries_all` | ❌ Wave 0 |
| FEAT-16 | Transient error retried, non-transient fails fast | e2e | `cargo test -p e2e-tests step_conditional_retry` | ❌ Wave 0 |
| TEST-23 | All 4 styles: step with timeout produces identical op sequence | parity | `cargo test -p parity-tests step_timeout_parity` | ❌ Wave 0 |
| TEST-24 | Complex workflow parity: parallel + map + child + timeout step | parity | `cargo test -p parity-tests complex_workflow_parity` | ❌ Wave 0 |
| TEST-25 | BatchItemStatus: succeeded/failed status asserted per item | parity | `cargo test -p parity-tests batch_item_status_verification` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p durable-lambda-core`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/e2e/tests/step_timeout_retry.rs` — covers FEAT-12, FEAT-16 (new test file)
- [ ] `crates/durable-lambda-core/src/operations/step.rs` — unit test functions for FEAT-09 through FEAT-15 (added inline in existing `#[cfg(test)]` module)
- [ ] `tests/parity/tests/parity.rs` — extended with TEST-23, TEST-24, TEST-25

No new test infrastructure needed — all existing `MockBackend`, `MockDurableContext`, and `CheckpointRecorder` patterns apply directly.

---

## Sources

### Primary (HIGH confidence)
- `crates/durable-lambda-core/src/operations/step.rs` — current `step_with_options` implementation; retry loop at lines 263–306; panic safety via `tokio::spawn` at lines 218–226
- `crates/durable-lambda-core/src/types.rs` — `StepOptions` struct; existing builder validation pattern; `#[derive(Debug, Clone, Default)]` note
- `crates/durable-lambda-core/src/error.rs` — `DurableError` variants; `.code()` exhaustive match; existing constructor pattern
- `Cargo.toml` workspace — `tokio = { version = "1.50.0", features = ["full"] }` confirms `tokio::time::timeout` is available
- `tests/parity/tests/parity.rs` — established cross-approach parity test pattern
- `crates/durable-lambda-testing/src/mock_backend.rs` — `CheckpointCall` struct (no `client_token` field, per STATE.md [02-03] note)

### Secondary (MEDIUM confidence)
- tokio docs for `tokio::time::timeout` — drop-in future wrapper, returns `Err(Elapsed)` on expiry; `JoinHandle::abort()` for task cancellation
- Rust stdlib `std::any::Any` — `downcast_ref::<T>()` pattern for type-erased closures

### Tertiary (LOW confidence)
- None — all critical claims verified against codebase directly.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all deps already in workspace; no new crates needed
- Architecture: HIGH — extension of existing `step_with_options` pattern; code reviewed in detail
- Pitfalls: HIGH — derived from actual codebase structure (Clone requirement, exhaustive match, type erasure)

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (stable codebase; no upstream dependency changes expected)
