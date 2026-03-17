# Phase 7: Saga / Compensation Pattern - Research

**Researched:** 2026-03-17
**Domain:** Rust async type system, durable execution checkpoint protocol, saga pattern
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**API surface**
- `ctx.step_with_compensation(name, forward_fn, compensate_fn)` — follows existing parameter convention (name, options, closure)
- Forward function signature identical to `step()` — `FnOnce() -> impl Future<Output = Result<T, E>>`
- Compensation function receives the forward result as input — `FnOnce(T) -> impl Future<Output = Result<(), CompensationError>>`
- Compensation closures must be `Send + 'static` (consistent with step/parallel closure requirements from CLAUDE.md)
- Return type: `Result<Result<T, E>, DurableError>` — same two-level pattern as `step()`

**Compensation registration and storage**
- Compensations stored in a `Vec<CompensationRecord>` on `DurableContext`
- Each record contains: operation name, serialized forward result, and the compensation closure (type-erased via `Box<dyn FnOnce>`)
- Registration happens after forward step succeeds — failed forward steps have nothing to compensate
- `CompensationRecord` must be serializable for checkpoint persistence (closure stored as operation reference, not the closure itself)

**Execution semantics**
- Compensations fire on explicit `ctx.run_compensations()` call, not automatically on any error
- Reverse order: last-registered compensation runs first (stack semantics)
- Each compensation is checkpointed as its own operation (START + SUCCEED/FAIL) — durable rollback
- Compensation operations use `OperationType::Context` with `sub_type: "Compensation"` — consistent with parallel/map/child_context pattern
- Child context isolation: each compensation runs in its own child context for operation ID namespacing

**Failure handling**
- Compensation failure is captured per-item, not abort-on-first — all compensations attempt to run
- Results returned as `CompensationResult` with per-item success/failure status (like `BatchResult` pattern)
- A `DurableError::CompensationFailed` variant for infrastructure failures during compensation execution
- Compensation of a compensation is NOT supported (no recursive saga) — keep it simple

**Replay behavior**
- During replay, compensation operations replay from history like any other operation (no special handling)
- Forward step results cached; compensation closures NOT re-executed during replay (same as step closures)
- Partial compensation (3 of 5 complete, re-invocation) resumes from checkpoint — compensations already completed are skipped

### Claude's Discretion
- Internal data structure for `CompensationRecord` (Vec vs VecDeque)
- Whether `step_with_compensation` also accepts `StepOptions` (timeout, retries) for the forward step
- Exact naming of the `CompensationResult` struct fields
- Whether to add `step_with_compensation` to `DurableContextOps` trait (recommended: yes)

### Deferred Ideas (OUT OF SCOPE)
- Nested sagas (compensation within compensation) — too complex for v1, could be a future enhancement
- Automatic compensation on any `DurableError` (instead of explicit `run_compensations()`) — potential future sugar
- Timeout on entire saga (all compensations must complete within N seconds) — could compose with step timeout
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FEAT-25 | `ctx.step_with_compensation(name, forward_fn, compensate_fn)` registers compensation and executes forward | Type erasure pattern from parallel.rs; `Box<dyn FnOnce>` for single-use closures; forward delegates to `step_with_options` |
| FEAT-26 | Compensation closures execute in reverse order on workflow failure | `Vec<CompensationRecord>` iterated with `.rev()`; explicit `run_compensations()` call |
| FEAT-27 | Compensation execution is itself checkpointed (durable rollback) | `OperationType::Context` + `sub_type: "Compensation"` follows child_context.rs checkpoint protocol exactly |
| FEAT-28 | Tests for compensation order, compensation failure, partial rollback | MockDurableContext pattern; pre-loaded history in Operation vec; new test file in `tests/e2e/tests/` |
</phase_requirements>

---

## Summary

Phase 7 adds the saga / compensation pattern to the SDK. The implementation is a composition of existing primitives rather than a new engine feature: `step_with_compensation` wraps a regular `step()` call and, on success, registers a type-erased compensation closure. `run_compensations()` iterates the registered compensations in reverse order, checkpointing each one using the established `OperationType::Context` + `sub_type: "Compensation"` protocol from child_context/parallel.

The main technical challenge is type erasure. The compensation closure receives the forward step's result (`T`), but `CompensationRecord` must be stored in a homogeneous `Vec` on `DurableContext`. The solution is to serialize `T` immediately (at registration time) and store the serialized JSON alongside a `Box<dyn FnOnce(serde_json::Value) -> BoxFuture<Result<(), DurableError>> + Send + 'static>` type-erased closure that deserializes and calls the original compensation.

Replay durability works identically to how parallel branches replay: the outer compensation block checkpoints Context/START then Context/SUCCEED (with sub_type "Compensation"), and each individual compensation checkpoints as a child. On partial rollback resume, compensations already checkpointed as Succeeded are skipped via the replay engine's `check_result` path.

**Primary recommendation:** Build `step_with_compensation` by composing `step_with_options`, then store `(serialized_forward_result, type_erased_compensate_fn)`. Implement `run_compensations` following the child_context pattern: Context/START + inner step + Context/SUCCEED for each compensation in reverse order. Keep the API surface minimal and defer all complexity (nested sagas, auto-compensation) to v2.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde_json` | workspace | Forward result serialization at registration time; stored in `CompensationRecord` | Already used throughout; all checkpointed values go through serde_json |
| `aws-sdk-lambda` | workspace | `OperationType::Context`, `OperationAction::Start/Succeed/Fail`, `OperationUpdate::builder()` | Same types used by parallel, child_context, map |
| `tokio` | workspace | Async execution of compensation closures (inline, not spawned) | Compensations run sequentially inline like child_context, not concurrently like parallel |
| `thiserror` | workspace | `DurableError::CompensationFailed` variant follows established pattern | All existing error variants use thiserror derive |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tracing` | workspace | Span per compensation operation (`op.type = "compensation"`) | Follows FEAT-17 pattern; every operation emits a span |
| `std::future::Future` | std | Boxed future type for type-erased closure return | Used in parallel.rs for boxed branch futures |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Type-erased `Box<dyn FnOnce>` with serialized result | Generic `CompensationRecord<T>` | Generic version cannot be stored in `Vec` on `DurableContext` without enum proliferation |
| Sequential inline execution (child_context pattern) | `tokio::spawn` concurrent execution | Compensations must run in strict reverse order; concurrent execution breaks ordering guarantee |
| Per-item `OperationType::Context` checkpoint | Plain `OperationType::Step` checkpoint | Context sub_type allows grouping under the parent saga operation for history readability |

**No additional dependencies needed.** Everything required is already in the workspace.

---

## Architecture Patterns

### Recommended File Structure
```
crates/durable-lambda-core/src/
├── operations/
│   ├── compensation.rs    # NEW: step_with_compensation + run_compensations impl
│   └── mod.rs             # ADD: pub mod compensation;
├── context.rs             # ADD: compensations: Vec<CompensationRecord> field
├── error.rs               # ADD: CompensationFailed variant + constructor
├── types.rs               # ADD: CompensationRecord, CompensationResult, CompensationItem structs
└── ops_trait.rs           # ADD: step_with_compensation + run_compensations methods

crates/durable-lambda-{closure,trait,builder}/src/context.rs
                           # ADD: delegation for step_with_compensation + run_compensations

crates/durable-lambda-testing/src/mock_context.rs
                           # ADD: with_compensation_result builder method

tests/e2e/tests/
└── compensation.rs        # NEW: FEAT-28 tests
```

### Pattern 1: Type Erasure for Compensation Closures

**What:** Store heterogeneous compensation closures in a homogeneous `Vec` by serializing `T` at registration and storing a type-erased closure that receives `serde_json::Value`.

**When to use:** Any time a closure captures a generic type that must be stored without knowing `T` at the storage site.

```rust
// Source: modeled after parallel.rs branch_fn type erasure pattern

/// Type alias for a type-erased async compensation closure.
pub type CompensateFn = Box<
    dyn FnOnce(serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), DurableError>> + Send>>
    + Send + 'static
>;

/// A single registered compensation with its serialized forward result.
pub struct CompensationRecord {
    /// The operation name (also used as the checkpoint name for the compensation).
    pub name: String,
    /// The serialized forward result passed to the compensation closure.
    pub forward_result_json: serde_json::Value,
    /// The type-erased compensation closure.
    pub compensate_fn: CompensateFn,
}
```

**Registration in `step_with_compensation`:**
```rust
// After forward step succeeds with Ok(T):
// 1. Serialize T to JSON for storage
// 2. Wrap the user's compensate_fn to accept serde_json::Value
let serialized = serde_json::to_value(&forward_value)?;
let wrapped: CompensateFn = Box::new(move |json_val| {
    let typed_val: T = serde_json::from_value(json_val).unwrap(); // safe: we just serialized T
    Box::pin(async move { compensate_fn(typed_val).await.map_err(|e| DurableError::compensation_failed(..., e)) })
});
self.compensations.push(CompensationRecord {
    name: name.to_string(),
    forward_result_json: serialized,
    compensate_fn: wrapped,
});
```

### Pattern 2: Compensation Checkpoint Protocol

**What:** Each compensation runs under a Context operation (START + SUCCEED/FAIL) with sub_type "Compensation", following the same protocol as child_context operations.

**When to use:** Every compensation in `run_compensations`.

```rust
// Source: modeled after child_context.rs checkpoint sequence

// For each compensation in reverse order:
let comp_op_id = self.replay_engine_mut().generate_operation_id();

// Replay path: check if compensation already completed
if let Some(op) = self.replay_engine().check_result(&comp_op_id) {
    if op.status == OperationStatus::Succeeded {
        self.replay_engine_mut().track_replay(&comp_op_id);
        results.push(CompensationItem { name, status: CompensationStatus::Succeeded, error: None });
        continue;
    }
    // Failed: record failure and continue (don't abort)
    results.push(CompensationItem { name, status: CompensationStatus::Failed, error: ... });
    continue;
}

// Execute path: Context/START
let start_update = OperationUpdate::builder()
    .id(comp_op_id.clone())
    .r#type(OperationType::Context)
    .action(OperationAction::Start)
    .sub_type("Compensation")
    .name(&record.name)
    .build()?;
self.backend().checkpoint(..., vec![start_update], None).await?;

// Execute the compensation closure
let comp_result = (record.compensate_fn)(record.forward_result_json).await;

// Context/SUCCEED or Context/FAIL based on result
match comp_result {
    Ok(()) => { /* send SUCCEED, push success item */ }
    Err(e) => { /* send FAIL with error object, push failure item */ }
}
```

### Pattern 3: DurableContext Field Addition

**What:** Add `compensations: Vec<CompensationRecord>` to `DurableContext` struct. Child contexts must NOT inherit parent compensations.

**When to use:** `DurableContext` struct initialization.

```rust
// In context.rs DurableContext struct:
pub struct DurableContext {
    // ... existing fields ...
    compensations: Vec<CompensationRecord>,  // NEW
}

// In DurableContext::new():
Ok(Self {
    // ... existing fields ...
    compensations: Vec::new(),  // NEW
})

// In create_child_context(): child starts with empty compensations
DurableContext {
    // ... existing fields ...
    compensations: Vec::new(),  // NOT copied from parent
}
```

### Pattern 4: CompensationResult Return Type

**What:** `run_compensations()` returns `Result<CompensationResult, DurableError>`. `CompensationResult` contains per-item outcomes. This mirrors `BatchResult<T>` from parallel/map.

```rust
// In types.rs — mirrors BatchResult<T> pattern

pub struct CompensationResult {
    pub items: Vec<CompensationItem>,
    /// True if all compensations succeeded; false if any failed.
    pub all_succeeded: bool,
}

pub struct CompensationItem {
    pub name: String,
    pub status: CompensationStatus,
    pub error: Option<String>,
}

pub enum CompensationStatus {
    Succeeded,
    Failed,
    Skipped,  // future: if needed for registration-only (no compensate_fn path)
}
```

### Anti-Patterns to Avoid

- **Storing the closure before forward step succeeds:** Only register compensation after `step_with_options` returns `Ok(Ok(T))`. A failed forward step has nothing to compensate.
- **Re-executing compensation closures during replay:** Same rule as step closures — if the operation is already checkpointed as Succeeded in history, skip execution entirely.
- **Aborting run_compensations on first failure:** Must run all compensations regardless of individual failures. Collect failures, return them in `CompensationResult`.
- **Inheriting compensations in child contexts:** `create_child_context()` always produces an empty `compensations` field. Compensation registration is scoped to the context it was registered on.
- **Using `tokio::spawn` for compensation execution:** Compensations run in strict reverse order (stack semantics). Concurrent execution via spawn would break this ordering guarantee.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Operation ID generation for compensation ops | Custom ID scheme | `self.replay_engine_mut().generate_operation_id()` | Already deterministic; must match Python SDK; breaks replay if diverged |
| Serialization of forward result for storage | Custom encoding | `serde_json::to_value(&T)` | Already used everywhere; consistent with checkpoint wire format |
| Checkpoint START/SUCCEED/FAIL protocol | New checkpoint logic | Copy child_context.rs pattern verbatim | Edge cases: missing checkpoint_token, new_execution_state pagination — all handled by existing code |
| Type-erased async closure | `dyn Fn + async` workaround | `Box<dyn FnOnce(...) -> Pin<Box<dyn Future<...> + Send>> + Send>` | Established pattern from parallel.rs BranchConfig; compiler-verified |
| Error variant for infrastructure failures | Ad-hoc string errors | `DurableError::CompensationFailed` variant with `.code() = "COMPENSATION_FAILED"` | Consistent with all other DurableError variants; exhaustive match in `.code()` enforces completeness |

**Key insight:** The compensation pattern is a composition of existing checkpoint primitives. Do not build a new checkpoint mechanism; reuse the exact protocol from `child_context.rs`.

---

## Common Pitfalls

### Pitfall 1: Forward Result Not Serializable
**What goes wrong:** `step_with_compensation` requires `T: Serialize` both for the step checkpoint AND for compensation registration. If `T` doesn't implement `Serialize`, compilation fails with a confusing error about `serde_json::to_value`.
**Why it happens:** The compensation stores a `serde_json::Value` of the forward result. This requires `T: Serialize` at registration time.
**How to avoid:** Add `T: Serialize + DeserializeOwned + Send + 'static` bounds to `step_with_compensation` signature — same as `step_with_options`. The `DeserializeOwned` bound is needed to deserialize from JSON inside the wrapped closure.
**Warning signs:** Compile error mentioning `serde_json::to_value`, `Serialize`, or `DeserializeOwned` in the context of `step_with_compensation`.

### Pitfall 2: CompensateFn Captured State Lifetime
**What goes wrong:** User compensation closures capture references or non-`'static` data, causing "borrowed value does not live long enough" errors.
**Why it happens:** The `CompensateFn` type alias requires `'static` because it's stored in `Vec<CompensationRecord>` on `DurableContext` which may outlive the user's lambda scope.
**How to avoid:** Document that compensation closures follow the same ownership rules as step closures: clone data before capturing into async move blocks. The `Send + 'static` bound on `CompensateFn` is the compiler enforcement.
**Warning signs:** Lifetime errors referencing `CompensationRecord` or `CompensateFn`.

### Pitfall 3: Compensation Operation ID Namespace
**What goes wrong:** If `run_compensations` generates operation IDs from the parent context directly, subsequent steps after `run_compensations` returns will get IDs that collide with compensation IDs.
**Why it happens:** The replay engine counter is stateful — each `generate_operation_id()` call increments it. Compensation IDs consume counter slots.
**How to avoid:** Use `create_child_context` for each compensation's operation ID namespace, exactly as child_context.rs does for its own operations. The compensation's child context generates sub-IDs scoped under the compensation op_id.
**Warning signs:** Replay mismatches after `run_compensations` in workflows that continue after rollback.

### Pitfall 4: Partial Rollback Resume Order
**What goes wrong:** On resume after partial compensation, compensations replay in the wrong order because the Vec iteration doesn't account for which compensations already succeeded.
**Why it happens:** `run_compensations` iterates in reverse. On resume, some early-in-reverse compensations (late registration) are already checkpointed as Succeeded. If iteration order is wrong, the resume might try to re-execute an already-succeeded compensation.
**How to avoid:** During resume, check each compensation's op_id against the replay engine before executing. Already-Succeeded ops are skipped via the replay path (same as step replay). The reverse iteration order is preserved — already-done ops just skip quickly through the replay path.
**Warning signs:** Double-execution of compensations on partial rollback, or compensations out of order in checkpoint history.

### Pitfall 5: Missing `.code()` Match Arm
**What goes wrong:** Adding `DurableError::CompensationFailed` without adding it to the exhaustive match in `.code()` causes a compile error, but the error points to the match arm, not to the missing variant.
**Why it happens:** Per STATE.md decision [04-02], `.code()` has no wildcard arm — compiler enforces exhaustive coverage.
**How to avoid:** When adding the variant to `DurableError` enum, immediately add the match arm in `.code()` returning `"COMPENSATION_FAILED"`. Add the test to the `all_error_variants_have_unique_codes` test in `error.rs`.
**Warning signs:** Compiler error "non-exhaustive patterns" in `error.rs::code()` match statement.

---

## Code Examples

Verified patterns from existing codebase:

### Type-Erased Async Closure (from parallel.rs)
```rust
// Source: crates/durable-lambda-core/src/operations/parallel.rs
// The BranchFn type alias pattern for type-erasing async closures into a Vec:
type BranchFn = Box<
    dyn FnOnce(DurableContext) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<i32, DurableError>> + Send>
    > + Send
>;
```

The compensation pattern adapts this: instead of `FnOnce(DurableContext)`, use `FnOnce(serde_json::Value)`.

### Context/START + Context/SUCCEED Checkpoint Sequence (from child_context.rs)
```rust
// Source: crates/durable-lambda-core/src/operations/child_context.rs lines 110-200

// START
let start_update = OperationUpdate::builder()
    .id(op_id.clone())
    .r#type(OperationType::Context)
    .action(OperationAction::Start)
    .sub_type("Context")  // change to "Compensation" for compensation ops
    .name(name)
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;
// ... send checkpoint, update token ...

// SUCCEED with payload
let ctx_opts = aws_sdk_lambda::types::ContextOptions::builder()
    .replay_children(false)
    .build();
let succeed_update = OperationUpdate::builder()
    .id(op_id.clone())
    .r#type(OperationType::Context)
    .action(OperationAction::Succeed)
    .sub_type("Context")  // change to "Compensation"
    .payload(serialized_result)
    .context_options(ctx_opts)
    .build()
    .map_err(|e| DurableError::checkpoint_failed(name, e))?;
```

### Replay Check Pattern (from child_context.rs)
```rust
// Source: crates/durable-lambda-core/src/operations/child_context.rs lines 72-107

if let Some(op) = self.replay_engine().check_result(&op_id) {
    if op.status == OperationStatus::Succeeded {
        // extract result from context_details().result()
        self.replay_engine_mut().track_replay(&op_id);
        return Ok(result);
    } else {
        // Failed/Cancelled/TimedOut/Stopped
        return Err(DurableError::child_context_failed(name, error_message));
    }
}
```

### DurableError New Variant Pattern (from error.rs)
```rust
// Source: crates/durable-lambda-core/src/error.rs

// 1. Add variant to enum (with #[non_exhaustive]):
#[error("compensation failed for operation '{operation_name}': {error_message}")]
#[non_exhaustive]
CompensationFailed {
    operation_name: String,
    error_message: String,
},

// 2. Add constructor:
pub fn compensation_failed(
    operation_name: impl Into<String>,
    error_message: impl Into<String>,
) -> Self {
    Self::CompensationFailed {
        operation_name: operation_name.into(),
        error_message: error_message.into(),
    }
}

// 3. Add to exhaustive .code() match:
Self::CompensationFailed { .. } => "COMPENSATION_FAILED",
```

### DurableContextOps Trait Extension Pattern (from ops_trait.rs)
```rust
// Source: crates/durable-lambda-core/src/ops_trait.rs

// In trait definition:
fn step_with_compensation<T, E, F, Fut, G, GFut>(
    &mut self,
    name: &str,
    forward_fn: F,
    compensate_fn: G,
) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
where
    T: Serialize + DeserializeOwned + Send + 'static,
    E: Serialize + DeserializeOwned + Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    G: FnOnce(T) -> GFut + Send + 'static,
    GFut: Future<Output = Result<(), DurableError>> + Send + 'static;

fn run_compensations(
    &mut self,
) -> impl Future<Output = Result<CompensationResult, DurableError>> + Send;

// In impl DurableContextOps for DurableContext:
fn step_with_compensation<...>(&mut self, name, forward_fn, compensate_fn) -> ... {
    DurableContext::step_with_compensation(self, name, forward_fn, compensate_fn)
}

fn run_compensations(&mut self) -> ... {
    DurableContext::run_compensations(self)
}
```

### Test Pattern for Compensation (from e2e_workflows.rs structure)
```rust
// Source: tests/e2e/tests/e2e_workflows.rs pattern

#[tokio::test]
async fn test_compensation_runs_in_reverse_order() {
    let (mut ctx, calls, _ops) = MockDurableContext::new().build().await;

    let mut order: Vec<String> = Vec::new();
    let order1 = Arc::new(Mutex::new(Vec::new()));  // shared capture for verification

    // Register compensations in order A, B, C
    let _: Result<i32, String> = ctx.step_with_compensation(
        "step_a",
        || async { Ok(1) },
        |_val| async move {
            // append "a" to order
            Ok(())
        },
    ).await.unwrap();

    // ... more steps ...

    // run_compensations should fire C, B, A
    let result = ctx.run_compensations().await.unwrap();
    assert!(result.all_succeeded);
    assert_eq!(result.items.len(), 3);
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual rollback logic in user code | `step_with_compensation` registers rollback alongside forward step | Phase 7 (new) | Users declare compensations at the point of the action, not in a separate error handler |
| No durable rollback guarantees | Compensation checkpointing via Context/START+SUCCEED | Phase 7 (new) | Partial rollbacks resume correctly on re-invocation, same as forward execution |

**Nothing is deprecated by this phase.** All existing operations continue to work unchanged.

---

## Open Questions

1. **`step_with_compensation` StepOptions support**
   - What we know: The CONTEXT.md marks this as "Claude's Discretion"
   - What's unclear: Whether the forward step in `step_with_compensation` should accept `StepOptions` (timeout, retries)
   - Recommendation: YES — add an overload `step_with_compensation_opts(name, options, forward_fn, compensate_fn)` to follow the `step` / `step_with_options` pair pattern. The simpler `step_with_compensation(name, forward_fn, compensate_fn)` uses `StepOptions::default()`. This keeps the API consistent and avoids feature gaps.

2. **`CompensationResult` field naming**
   - What we know: Should mirror `BatchResult<T>` from types.rs but for compensations
   - What's unclear: `items` vs `results`; `all_succeeded` computed field vs client-side check
   - Recommendation: Use `items: Vec<CompensationItem>` (more accurate than `results`), include `all_succeeded: bool` as a convenience field computed at construction. This mirrors `BatchResult.completion_reason` providing a quick status check without iterating.

3. **Wrapper crate delegation**
   - What we know: Per CLAUDE.md, all 3 wrapper crates (closure, trait, builder) must be updated; per ops_trait.rs, `DurableContextOps` is the single delegation point
   - What's unclear: Whether the `DurableContext`-native methods in compensation.rs need `pub(crate)` or `pub` visibility
   - Recommendation: Make `step_with_compensation` and `run_compensations` on `DurableContext` `pub` (same as `step`, `parallel`, etc.). The trait impl delegates to them.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + tokio-test |
| Config file | Cargo.toml per-crate `[dev-dependencies]` |
| Quick run command | `cargo test -p e2e-tests compensation` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FEAT-25 | `step_with_compensation` registers compensation and executes forward | unit | `cargo test -p durable-lambda-core step_with_compensation` | ❌ Wave 0 |
| FEAT-25 | Forward step result returned to caller (two-level Result) | unit | `cargo test -p durable-lambda-core step_with_compensation_returns_result` | ❌ Wave 0 |
| FEAT-25 | Failed forward step does NOT register compensation | unit | `cargo test -p durable-lambda-core step_with_compensation_no_register_on_fail` | ❌ Wave 0 |
| FEAT-26 | Compensations execute in reverse registration order | e2e | `cargo test -p e2e-tests compensation_reverse_order` | ❌ Wave 0 |
| FEAT-26 | `run_compensations` with 0 compensations is a no-op | unit | `cargo test -p durable-lambda-core run_compensations_empty` | ❌ Wave 0 |
| FEAT-27 | Each compensation sends Context/START + Context/SUCCEED | unit | `cargo test -p durable-lambda-core compensation_checkpoint_sequence` | ❌ Wave 0 |
| FEAT-27 | Partial rollback resumes from checkpoint on re-invocation | unit | `cargo test -p durable-lambda-core compensation_partial_rollback_resume` | ❌ Wave 0 |
| FEAT-27 | Replay of completed compensation skips re-execution | unit | `cargo test -p durable-lambda-core compensation_replay_skips` | ❌ Wave 0 |
| FEAT-28 | Compensation failure captured per-item, all compensations attempt | unit | `cargo test -p durable-lambda-core compensation_failure_captured` | ❌ Wave 0 |
| FEAT-28 | `CompensationResult.all_succeeded` is false when any item fails | unit | `cargo test -p durable-lambda-core compensation_result_not_all_succeeded` | ❌ Wave 0 |
| FEAT-28 | `DurableError::CompensationFailed` has code `"COMPENSATION_FAILED"` | unit | `cargo test -p durable-lambda-core compensation_error_code` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p durable-lambda-core && cargo test -p e2e-tests compensation`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green (`cargo test --workspace` + `cargo clippy --workspace -- -D warnings` + `cargo fmt --all --check`) before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/e2e/tests/compensation.rs` — covers FEAT-26, FEAT-28 e2e tests
- [ ] `crates/durable-lambda-core/src/operations/compensation.rs` — unit tests embedded in module (established pattern)
- [ ] `crates/durable-lambda-testing/src/mock_context.rs` — `.with_compensation_result()` builder if needed for replay tests

*(All unit tests for FEAT-25 and FEAT-27 live inside `compensation.rs` following the existing pattern where operations tests are co-located with implementation.)*

---

## Sources

### Primary (HIGH confidence)
- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/operations/child_context.rs` — Checkpoint protocol (Context/START + Context/SUCCEED), replay path, create_child_context usage
- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/operations/parallel.rs` — Type-erased async closure pattern (BranchFn), tokio::spawn vs inline execution
- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/context.rs` — DurableContext struct fields, create_child_context isolation, batch_mode precedent for new fields
- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/error.rs` — DurableError variant pattern, `.code()` exhaustive match requirement
- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/types.rs` — BatchResult/BatchItem/BatchItemStatus pattern for CompensationResult design
- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/ops_trait.rs` — DurableContextOps trait extension pattern, RPITIT async fn in traits

### Secondary (MEDIUM confidence)
- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/operations/step.rs` — step_with_options implementation for forward step delegation
- `/home/esa/git/durable-rust/.planning/phases/07-saga-compensation-pattern/07-CONTEXT.md` — All locked decisions, Claude's discretion items, deferred scope
- `/home/esa/git/durable-rust/CLAUDE.md` — Code style requirements (rustdoc, no unwrap, module docs, workspace deps)
- `/home/esa/git/durable-rust/.planning/STATE.md` — Accumulated decisions from previous phases (esp. [04-02] exhaustive code() match, [03-01] RPITIT async in traits)

### Tertiary (LOW confidence)
- None — all findings verified against actual codebase.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already in workspace, no new deps needed
- Architecture patterns: HIGH — directly derived from child_context.rs and parallel.rs which are proven implementations
- Type erasure approach: HIGH — exact pattern already used in parallel.rs
- Pitfalls: HIGH — derived from actual code reading and STATE.md accumulated decisions
- Test patterns: HIGH — derived from existing test files in the same repository

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (stable codebase, no external dependencies changing)
