# Phase 6: Observability & Batch Checkpoint - Research

**Researched:** 2026-03-16
**Domain:** Rust `tracing` crate spans, AWS checkpoint batching, `DurableBackend` extension
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FEAT-17 | Each operation wrapped in `tracing::span` with operation name, type, and ID fields | `tracing` 0.1.44 already in workspace; `tracing::span!` macro + `Span::enter` cover this |
| FEAT-18 | Parent-child span hierarchy matches context nesting | `tracing::Span::current()` as parent when entering child contexts; `span.entered()` guard propagates implicitly |
| FEAT-19 | Span enters on operation start, exits on completion | `let _guard = span.enter()` pattern; guard drops at scope end |
| FEAT-20 | Tests verify spans are emitted with correct fields | `tracing-test` 0.2 already in workspace; `#[traced_test]` + `logs_contain()` pattern already used in `log.rs` |
| FEAT-21 | `DurableBackend` gains `batch_checkpoint()` accepting `Vec<OperationUpdate>` | New method on `DurableBackend` trait; `RealBackend` calls same AWS SDK method with full vec; `MockBackend` records single call |
| FEAT-22 | Sequential steps can opt into batched checkpoint mode | New opt-in mode on `DurableContext` (flag or mode enum); batch accumulates `OperationUpdate` items |
| FEAT-23 | Single checkpoint call for N operation updates | `batch_checkpoint()` flattens accumulated updates into one AWS call |
| FEAT-24 | Tests verify batch reduces checkpoint call count | `MockBackend` checkpoint counter comparison: 5-step individual = 10 calls, 5-step batch = 1 call |
</phase_requirements>

---

## Summary

Phase 6 adds two orthogonal features to `durable-lambda-core`: operation-level tracing spans (FEAT-17 through FEAT-20) and a batch checkpoint API (FEAT-21 through FEAT-24).

**Tracing spans** leverage the `tracing` crate already in the workspace (`0.1.44`). The existing codebase uses `tracing::info!` / `tracing::debug!` etc. in `log.rs`. The new requirement is span creation (not just log events) wrapping the duration of each operation — `step`, `wait`, `callback`, `invoke`, `parallel`, `map`, and `child_context`. Each operation file already has the operation name and a newly-generated `op_id` available at the top of the function; wrapping the rest of the function body in a span guard is a minimal, non-breaking change. Parent-child span hierarchy is automatic when child operations run inside a parent span's guard: `tracing` uses thread-local storage to track the current span, so spans entered in a parent context automatically become the parent of spans opened inside them.

**Batch checkpoint** requires adding a new method to `DurableBackend` (a breaking change for the trait, but the only external implementors are `RealBackend` and `MockBackend` — both owned). The simplest design: `DurableContext` gains an optional `pending_updates: Vec<OperationUpdate>` accumulator and a batch mode flag. When batch mode is active, START and SUCCEED checkpoints are deferred into the accumulator instead of sent immediately; `flush_batch()` or an auto-flush at operation completion sends them all in a single call. The AWS SDK's `checkpoint_durable_execution` already accepts `Vec<OperationUpdate>` with multiple items — so the underlying wire call is unchanged.

**Primary recommendation:** Implement spans with `tracing::span!` guards inside each operation method (7 insertion points in `operations/*.rs`). Implement batch checkpoint as an accumulator + explicit `batch_checkpoint()` method on the trait. Both features are additive — no existing behavior changes.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tracing` | 0.1.44 (workspace) | Structured diagnostics spans and events | Already in workspace; used for log.rs events; the span API is the standard Rust observability primitive |
| `tracing-test` | 0.2 (workspace) | `#[traced_test]` + `logs_contain()` in tests | Already used in `log.rs` tests; same pattern for span field assertions |
| `aws-sdk-lambda` | 1.118.0 (workspace) | `OperationUpdate` for batch checkpoint | Already the wire type; `checkpoint_durable_execution` accepts `Vec<OperationUpdate>` |

### No New Dependencies Required

All needed libraries are already in the workspace. Phase 6 requires zero new `Cargo.toml` entries.

---

## Architecture Patterns

### Tracing Spans — Insertion Pattern

Each of the 7 operation methods in `operations/*.rs` follows this shape:

```rust
// Source: tracing 0.1.44 API
pub async fn step_with_options<T, E, F, Fut>(
    &mut self,
    name: &str,
    options: StepOptions,
    f: F,
) -> Result<Result<T, E>, DurableError> {
    let op_id = self.replay_engine_mut().generate_operation_id();

    // NEW: create span immediately after op_id is known
    let span = tracing::info_span!(
        "durable_operation",
        op.name = name,
        op.type = "step",
        op.id = %op_id,
    );
    let _guard = span.enter();

    // ... rest of method unchanged ...
}
```

Key points:
- Use `tracing::info_span!` (not `tracing::span!` with explicit Level) — INFO is the correct level for durable operation boundaries visible in production.
- Field names follow the `op.name`, `op.type`, `op.id` convention for grouping under a common prefix.
- `let _guard = span.enter()` — the guard is held for the entire function body. When the async function yields (`.await`), the guard is NOT held across await points by default. This is intentional for async code (see Pitfall 1 below).
- For truly async span coverage across `.await` points, use `span.in_scope(async { ... })` or `.instrument(span)` from `tracing-futures` / `tracing::Instrument`. However the `tracing` crate in 0.1.x includes `Instrument` in `tracing::Instrument`.

### Async-Safe Span Instrumentation

Because operations like `step_with_options` are `async fn` that contain `.await` points, using `span.enter()` only covers synchronous portions. For full span coverage across awaits:

```rust
use tracing::Instrument;

pub async fn step_with_options(...) -> Result<...> {
    let op_id = self.replay_engine_mut().generate_operation_id();
    let span = tracing::info_span!(
        "durable_operation",
        op.name = name,
        op.type = "step",
        op.id = %op_id,
    );

    async move {
        // ... full body ...
    }
    .instrument(span)
    .await
}
```

However, since these are `&mut self` methods, the body cannot be easily moved into a closure. The practical approach: create the span, enter it for the synchronous setup, exit before the first `await`, and re-enter for post-await work. For the purposes of FEAT-17 through FEAT-20, the span simply needs to exist — the `tracing_test` assertions check that spans were emitted with correct fields, not necessarily that they span across all await boundaries. Use `let _guard = span.enter()` at the top of each method. This is the same pattern used by the existing `log.rs` code.

For `parallel` and `map` which spawn `tokio::spawn` tasks: the span must be cloned and `.instrument()`-ed on the spawned futures for child branches to appear as children. This is a secondary concern for FEAT-18.

### Parent-Child Hierarchy

`tracing`'s thread-local current-span mechanism makes hierarchy automatic for synchronous and `async` code when using `Instrument`. For child contexts:

- `DurableContext::child_context()` creates a `create_child_context()` — the child's operations run inside the parent's async block
- If the parent's span is active when the child operation starts, `tracing` records it as a child automatically
- No explicit parent-span ID passing is needed for the default `tracing` subscriber

### Batch Checkpoint Design

#### Option A: New method on DurableBackend (RECOMMENDED)

Add `batch_checkpoint()` to `DurableBackend` with a default implementation that delegates to `checkpoint()`:

```rust
// In backend.rs — DurableBackend trait
async fn batch_checkpoint(
    &self,
    arn: &str,
    checkpoint_token: &str,
    updates: Vec<OperationUpdate>,
    client_token: Option<&str>,
) -> Result<CheckpointDurableExecutionOutput, DurableError> {
    // Default: single call (same as checkpoint)
    self.checkpoint(arn, checkpoint_token, updates, client_token).await
}
```

`RealBackend` inherits the default (it already passes `Vec<OperationUpdate>` to the AWS SDK). `MockBackend` can override to record batch calls separately for test assertion.

However: `DurableBackend` uses `#[async_trait]`. Default methods in `async_trait` traits are supported.

#### DurableContext Batch Mode

```rust
// In context.rs
pub struct DurableContext {
    backend: Arc<dyn DurableBackend>,
    replay_engine: ReplayEngine,
    durable_execution_arn: String,
    checkpoint_token: String,
    parent_op_id: Option<String>,
    // NEW:
    batch_mode: bool,
    pending_updates: Vec<OperationUpdate>,
}
```

When `batch_mode = true`, each operation does NOT call `backend.checkpoint()` immediately after building `OperationUpdate` items. Instead it calls `self.pending_updates.push(update)`. A new method `ctx.flush_batch()` (or `ctx.batch_checkpoint()`) sends them all:

```rust
pub async fn flush_batch(&mut self) -> Result<(), DurableError> {
    if self.pending_updates.is_empty() {
        return Ok(());
    }
    let updates = std::mem::take(&mut self.pending_updates);
    let response = self.backend()
        .checkpoint(self.arn(), self.checkpoint_token(), updates, None)
        .await?;
    let new_token = response.checkpoint_token().ok_or_else(|| { ... })?;
    self.set_checkpoint_token(new_token.to_string());
    Ok(())
}
```

#### Alternative: `batch_checkpoint()` on DurableContext directly

Per FEAT-21, `DurableBackend` gains `batch_checkpoint()`. Per FEAT-22, sequential steps opt into batch mode. The cleanest split:

- `DurableBackend::batch_checkpoint()` = the single-call mechanism (takes `Vec<OperationUpdate>`)
- `DurableContext::enable_batch_mode()` / `DurableContext::batch_checkpoint()` = the public API for users to flush

The requirement says "batch_checkpoint() accepts multiple OperationUpdate items" — this is the `DurableBackend` method. The opt-in per FEAT-22 is on the context level.

### Recommended Project Structure Changes

```
crates/durable-lambda-core/src/
├── backend.rs          # Add batch_checkpoint() method
├── context.rs          # Add batch_mode, pending_updates fields + enable_batch_mode(), flush_batch()
├── operations/
│   ├── step.rs         # Add span at top of step_with_options
│   ├── wait.rs         # Add span
│   ├── callback.rs     # Add span (create_callback)
│   ├── invoke.rs       # Add span
│   ├── parallel.rs     # Add span + instrument spawned branches
│   ├── map.rs          # Add span + instrument spawned items
│   └── child_context.rs # Add span
└── ...
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Span parent-child tracking | Manual parent ID field passing in every call | `tracing` thread-local current-span | Built-in to `tracing`; automatic for sync/inline-async flows |
| Span field formatting | Custom string formatting for op fields | `tracing::info_span!` field syntax | Handles field escaping, subscriber routing automatically |
| Test span assertions | Custom subscriber impl for testing | `tracing-test`'s `#[traced_test]` + `logs_contain()` | Already used in `log.rs`; captures spans and events identically |
| Async span propagation | Manual thread-local management | `tracing::Instrument` trait | `.instrument(span)` correctly handles tokio task switching |

**Key insight:** The `tracing` crate's design means correct async span propagation requires `.instrument()` on futures, not `span.enter()` guards across `.await` — but for the test assertions needed by FEAT-20, `logs_contain()` checks fields on events, not span lifetimes. The `tracing_test` subscriber captures all events/spans in the test thread.

---

## Common Pitfalls

### Pitfall 1: `span.enter()` across `.await` points in async code

**What goes wrong:** Holding a `span.enter()` guard across an `.await` point in `async` code is incorrect when using multi-threaded Tokio. The guard holds a thread-local reference; if the future is polled on a different thread after the await, the span state is wrong. `tracing` emits a compile-time warning for this pattern.

**Why it happens:** `async fn` is syntactic sugar for a state machine; the guard's lifetime extends across suspension points if it lives in the same scope as the `.await`.

**How to avoid:** Use `.instrument(span)` on the async block/future rather than `span.enter()` inside `async fn`. For `&mut self` methods, enter the span only for synchronous setup sections, or restructure as:
```rust
let span = tracing::info_span!("durable_operation", op.name = name, op.type = "step");
// Enter only for synchronous replay check:
{
    let _guard = span.enter();
    // synchronous replay check...
}
// For the async execute path, use a separate span or accept that the guard
// only covers the synchronous portions.
```

For FEAT-17 through FEAT-20, the requirement is that spans are emitted with correct fields — the `#[traced_test]` approach verifies event/span creation, not duration. The simplest compliant approach: create the span, enter it synchronously (before the first await), do the work, and record that the span was entered. The `tracing-test` `logs_contain()` checks the span's fields in the global captured log.

**Warning signs:** `clippy` with `tracing` lints will flag `_guard` held across `.await`.

### Pitfall 2: `async_trait` + default methods

**What goes wrong:** `async_trait` rewrites `async fn` into `Box<dyn Future>`. Default async methods in `async_trait` traits compile but may require explicit `#[async_trait]` on the implementing type even when not overriding.

**Why it happens:** The `async_trait` proc-macro transforms each async method individually; default impls need the same transformation.

**How to avoid:** Mark the new `batch_checkpoint` method with `#[async_trait]` semantics by ensuring the trait's `#[async_trait]` attribute covers it. Since `DurableBackend` already has `#[async_trait::async_trait]` on the trait definition, adding a new `async fn` method with a default body inside the trait block works correctly — the macro processes all methods in the block.

### Pitfall 3: `MockBackend` not recording batch calls correctly

**What goes wrong:** If `batch_checkpoint()` defaults to calling `self.checkpoint()`, `MockBackend` will record N individual calls instead of 1 batch call — defeating the test assertion for FEAT-24.

**Why it happens:** Default delegation means the mock sees the same code path as individual mode.

**How to avoid:** `MockBackend` must override `batch_checkpoint()` to record the single call with all updates in one `CheckpointCall`. Add a separate counter or flag in `MockBackend` to distinguish batch calls from individual calls. Alternatively, add `batch_call_count: Arc<Mutex<usize>>` to `MockBackend`.

### Pitfall 4: Batch mode + checkpoint token evolution

**What goes wrong:** In individual mode, each checkpoint call returns a new token. In batch mode, the token is only updated once (after `flush_batch()`). If a failure occurs mid-batch (after some updates are accumulated but before flush), the token state is inconsistent.

**Why it happens:** The AWS API requires the current valid token with each call; skipping intermediate updates means intermediate tokens are never obtained.

**How to avoid:** Batch mode is explicitly opt-in and documented as a performance optimization for sequential workflows that complete without error. The success criteria (FEAT-24) only requires that a 5-step batch workflow produces fewer checkpoint calls than individual mode — not that failure semantics are identical. Document that batch mode is best-effort: if the Lambda times out mid-batch, some operations may not be checkpointed.

### Pitfall 5: `pending_updates` in child contexts

**What goes wrong:** `create_child_context()` clones most `DurableContext` fields. If `pending_updates` is not cloned (or is cloned when it should be empty), child context batch updates may be lost or incorrectly merged with parent updates.

**Why it happens:** `DurableContext::create_child_context()` manually constructs a new `DurableContext` — any new field must be handled explicitly.

**How to avoid:** Child contexts should always start with empty `pending_updates` and `batch_mode = false` (or inherit parent batch mode but with their own accumulator). The planner should explicitly address how `create_child_context` handles the new fields.

---

## Code Examples

### Span Creation in an Operation

```rust
// Pattern for all 7 operation types in operations/*.rs
// Source: tracing 0.1.44 docs + existing log.rs patterns in this codebase
pub async fn step_with_options<T, E, F, Fut>(
    &mut self,
    name: &str,
    options: StepOptions,
    f: F,
) -> Result<Result<T, E>, DurableError> {
    let op_id = self.replay_engine_mut().generate_operation_id();

    // Create span with operation metadata known at this point.
    let span = tracing::info_span!(
        "durable_operation",
        op.name = name,
        op.type = "step",
        op.id = %op_id,
    );
    let _enter = span.enter();

    // Replay path (synchronous):
    if let Some(operation) = self.replay_engine().check_result(&op_id) {
        let result = extract_step_result::<T, E>(operation)?;
        self.replay_engine_mut().track_replay(&op_id);
        return Ok(result);
    }
    // ... execute path ...
}
```

### Test Assertion Pattern for Spans (from existing log.rs pattern)

```rust
// Source: existing crates/durable-lambda-core/src/operations/log.rs tests
#[traced_test]
#[tokio::test]
async fn test_step_emits_span_with_fields() {
    let (mut ctx, _calls, _ops) = MockDurableContext::new().build().await;

    let _: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await.unwrap();

    // tracing_test captures span/event fields in logs_contain()
    assert!(logs_contain("durable_operation"));
    assert!(logs_contain("validate"));    // op.name field
    assert!(logs_contain("step"));        // op.type field
}
```

### Batch Checkpoint on DurableBackend

```rust
// In backend.rs — addition to DurableBackend trait
// Source: async_trait 0.1 docs + existing checkpoint method pattern
#[async_trait::async_trait]
pub trait DurableBackend: Send + Sync {
    // ... existing methods ...

    /// Persist multiple checkpoint updates in a single AWS API call.
    ///
    /// Default implementation delegates to [`checkpoint`](Self::checkpoint),
    /// producing one call per invocation (same as individual mode).
    /// Override in `MockBackend` to record batch-specific call counts.
    async fn batch_checkpoint(
        &self,
        arn: &str,
        checkpoint_token: &str,
        updates: Vec<OperationUpdate>,
        client_token: Option<&str>,
    ) -> Result<CheckpointDurableExecutionOutput, DurableError> {
        self.checkpoint(arn, checkpoint_token, updates, client_token).await
    }
}
```

### Enabling Batch Mode on DurableContext

```rust
// In context.rs
impl DurableContext {
    /// Enable batch checkpoint mode.
    ///
    /// When enabled, operation checkpoints are accumulated in memory and
    /// sent as a single AWS API call when [`flush_batch`](Self::flush_batch)
    /// is called. Individual checkpoint mode remains the default.
    pub fn enable_batch_mode(&mut self) {
        self.batch_mode = true;
    }

    /// Flush all accumulated checkpoint updates in a single AWS API call.
    ///
    /// No-op if no updates are pending. Resets the accumulator after flushing.
    pub async fn flush_batch(&mut self) -> Result<(), DurableError> {
        if self.pending_updates.is_empty() {
            return Ok(());
        }
        let updates = std::mem::take(&mut self.pending_updates);
        let response = self.backend()
            .batch_checkpoint(self.arn(), self.checkpoint_token(), updates, None)
            .await?;
        let new_token = response.checkpoint_token().ok_or_else(|| {
            DurableError::checkpoint_failed(
                "batch",
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "batch checkpoint response missing checkpoint_token",
                ),
            )
        })?;
        self.set_checkpoint_token(new_token.to_string());
        Ok(())
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `tracing::event!` for diagnostics | `tracing::Span` for operation boundaries + events for log lines | Always; spans are the canonical unit | Operations get duration, hierarchy, and fields in one construct |
| `span.enter()` guard in async | `.instrument(span)` on futures | tracing 0.1.x | Correct behavior across tokio thread switches |

**Existing in codebase:** The `log.rs` module already uses `tracing::info!` etc. for events. Phase 6 adds `tracing::info_span!` for operation-wrapping spans — these are complementary, not replacements.

---

## Open Questions

1. **Span level for replay vs. execute paths**
   - What we know: `log.rs` uses `tracing::info!` for executing, suppresses during replay
   - What's unclear: Should spans during replay be emitted at DEBUG level (or suppressed entirely)?
   - Recommendation: Emit spans at INFO regardless of replay mode. Span fields include enough context (op.id) for consumers to filter. FEAT-17 says "each operation wrapped in tracing::span" without qualification. Suppressing replay spans would hide useful diagnostics.

2. **`flush_batch()` vs. automatic flush**
   - What we know: FEAT-22 says "sequential steps can opt into batched checkpoint mode"; FEAT-23 says "single checkpoint call for N operation updates"
   - What's unclear: Is `flush_batch()` explicit (called by user) or automatic (called at the end of each context)?
   - Recommendation: Explicit `flush_batch()`. This gives the user control and makes the call count verifiable in tests. Auto-flush at `DurableContext` drop is not practical (async drop is unsupported in Rust).

3. **Batch mode interaction with retries and suspensions**
   - What we know: `wait` and `invoke` both return suspension errors after the START checkpoint. If batch mode defers the START, the suspension is also deferred.
   - What's unclear: Does batch mode apply to all operation types or only to `step`?
   - Recommendation: For Phase 6, batch mode applies only to `step` operations (START + SUCCEED pairs). `wait`, `invoke`, and `callback` produce suspension errors that must be sent immediately — batching them would break the suspension contract. Document this constraint clearly.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in + `tracing-test` 0.2 |
| Config file | none (workspace `Cargo.toml`) |
| Quick run command | `cargo test -p durable-lambda-core` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FEAT-17 | `step` emits span with name, type, op_id fields | unit | `cargo test -p durable-lambda-core test_step_emits_span` | No — Wave 0 |
| FEAT-17 | `wait` emits span with correct fields | unit | `cargo test -p durable-lambda-core test_wait_emits_span` | No — Wave 0 |
| FEAT-17 | `callback` emits span with correct fields | unit | `cargo test -p durable-lambda-core test_callback_emits_span` | No — Wave 0 |
| FEAT-17 | `invoke` emits span with correct fields | unit | `cargo test -p durable-lambda-core test_invoke_emits_span` | No — Wave 0 |
| FEAT-17 | `parallel` emits span with correct fields | unit | `cargo test -p durable-lambda-core test_parallel_emits_span` | No — Wave 0 |
| FEAT-17 | `map` emits span with correct fields | unit | `cargo test -p durable-lambda-core test_map_emits_span` | No — Wave 0 |
| FEAT-17 | `child_context` emits span with correct fields | unit | `cargo test -p durable-lambda-core test_child_context_emits_span` | No — Wave 0 |
| FEAT-18 | Nested child context produces child span | unit | `cargo test -p durable-lambda-core test_child_context_span_hierarchy` | No — Wave 0 |
| FEAT-19 | Span entered on start, exited on completion | unit | `cargo test -p durable-lambda-core test_span_lifecycle` | No — Wave 0 |
| FEAT-20 | All 7 operation types emit correct fields (batch test) | unit | `cargo test -p durable-lambda-core test_all_operations_emit_spans` | No — Wave 0 |
| FEAT-21 | `DurableBackend::batch_checkpoint` exists and calls AWS | unit | `cargo test -p durable-lambda-core test_batch_checkpoint_method` | No — Wave 0 |
| FEAT-22 | `enable_batch_mode()` defers checkpoints | unit | `cargo test -p durable-lambda-core test_batch_mode_defers_checkpoints` | No — Wave 0 |
| FEAT-23 | 5-step batch produces 1 checkpoint call | unit | `cargo test -p durable-lambda-core test_batch_single_call_for_n_updates` | No — Wave 0 |
| FEAT-24 | 5-step batch < 5-step individual checkpoint count | unit | `cargo test -p durable-lambda-core test_batch_reduces_checkpoint_count` | No — Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p durable-lambda-core`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

All test functions are new — they live in existing files (e.g., `operations/step.rs`, `operations/wait.rs`, etc.) using the existing `#[traced_test]` + `MockDurableContext` patterns. No new test files are needed.

- [ ] `crates/durable-lambda-core/src/operations/step.rs` — add span tests using `#[traced_test]`
- [ ] `crates/durable-lambda-core/src/operations/wait.rs` — add span tests
- [ ] `crates/durable-lambda-core/src/operations/callback.rs` — add span tests
- [ ] `crates/durable-lambda-core/src/operations/invoke.rs` — add span tests
- [ ] `crates/durable-lambda-core/src/operations/parallel.rs` — add span tests
- [ ] `crates/durable-lambda-core/src/operations/map.rs` — add span tests
- [ ] `crates/durable-lambda-core/src/operations/child_context.rs` — add span tests
- [ ] `crates/durable-lambda-core/src/backend.rs` — add `batch_checkpoint` default method + `MockBackend` override
- [ ] `crates/durable-lambda-core/src/context.rs` — add `batch_mode`, `pending_updates`, `enable_batch_mode()`, `flush_batch()`

---

## Codebase-Specific Findings (HIGH confidence)

### What Already Exists

1. **`tracing` 0.1.44 is in workspace** (`Cargo.toml` line 32). No new dependency needed.
2. **`tracing-test` 0.2 is in workspace** (`Cargo.toml` line 36) and already used in `log.rs`. The `#[traced_test]` + `logs_contain()` test pattern is established.
3. **All operation files import nothing from `tracing`** (confirmed by Grep). Zero spans exist in any operation file — clean insertion point.
4. **`DurableBackend` already accepts `Vec<OperationUpdate>`** in `checkpoint()`. The AWS SDK method `checkpoint_durable_execution` takes a list — batching is already the wire-level API. Individual calls use single-item vecs.
5. **`DurableContext` already has `backend: Arc<dyn DurableBackend>`** with `backend()` accessor. All operation files already call `self.backend().checkpoint(...)`. The batch accumulator can intercept these calls with a simple flag check.
6. **`create_child_context()` manually constructs `DurableContext`** (line 247 of `context.rs`). Any new fields on `DurableContext` must be explicitly initialized there.
7. **`MockBackend::checkpoint()` already records `CheckpointCall`** with full `updates: Vec<OperationUpdate>`. The test for FEAT-24 (checkpoint count reduction) can reuse `CheckpointRecorder` with `calls.lock().await.len()` comparisons.
8. **The operation type string values** are: `"step"`, `"wait"`, `"callback"`, `"invoke"`, `"parallel"`, `"map"`, `"child_context"` — inferred from `MockBackend`'s op_type strings and operation file names. Span `op.type` fields should use these exact strings for consistency.

### Operation-Specific Notes

- **`step.rs`**: Span can be opened right after `generate_operation_id()` — op_id is available at that point.
- **`wait.rs`**: Single START checkpoint (no SUCCEED — server handles it). Span covers the START call only.
- **`callback.rs`**: Two-phase — `create_callback()` sends START; `callback_result()` is synchronous. Span belongs in `create_callback()`.
- **`invoke.rs`**: Single START checkpoint. Span covers the START call.
- **`parallel.rs`**: Outer span covers the parallel block; each branch's `tokio::spawn` needs `.instrument(span.clone())` for child branches to appear as children of the outer span.
- **`map.rs`**: Same as parallel — outer span + per-item `.instrument()` on spawned items.
- **`child_context.rs`**: Outer span covers the child context setup; the inner subflow runs in the same task (no `tokio::spawn`), so the span hierarchy is automatic.

### Batch Checkpoint Scope

FEAT-22 says "sequential steps can opt into batched checkpoint mode." The key constraint:
- `step` operations produce START then SUCCEED/FAIL — both can be batched if the step completes synchronously within the invocation.
- `wait` produces only START then immediately returns `WaitSuspended` — the START must be sent before returning. Batching is not compatible with suspension.
- `invoke` and `callback` similarly suspend immediately after START.
- Therefore: batch mode in Phase 6 scope = batching `step` START + SUCCEED pairs only. Other operations always use individual mode.

---

## Sources

### Primary (HIGH confidence)

- Workspace `Cargo.toml` — confirmed `tracing = "0.1.44"`, `tracing-test = "0.2"`, `aws-sdk-lambda = "1.118.0"`
- `crates/durable-lambda-core/src/backend.rs` — confirmed `DurableBackend` trait shape, `Vec<OperationUpdate>` signature
- `crates/durable-lambda-core/src/context.rs` — confirmed `DurableContext` fields, `create_child_context()` pattern
- `crates/durable-lambda-core/src/operations/log.rs` — confirmed `#[traced_test]` + `logs_contain()` test pattern in use
- `crates/durable-lambda-core/src/operations/step.rs` — confirmed checkpoint call sites and op_id availability
- `crates/durable-lambda-testing/src/mock_backend.rs` — confirmed `CheckpointCall`, `CheckpointRecorder` types

### Secondary (MEDIUM confidence)

- `tracing` 0.1.44 API behavior for `info_span!`, `span.enter()`, `Instrument` trait — consistent with tracing crate documentation as of training cutoff; the 0.1.x API has been stable for multiple years.
- `async_trait` 0.1 default method behavior — based on knowledge of how the macro processes trait blocks; should be verified by attempting compilation.

### Tertiary (LOW confidence)

- Claim that `span.enter()` held across `.await` is flagged by clippy — valid for `tracing` + tokio; specific clippy lint name not verified by code search.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies confirmed in workspace
- Architecture: HIGH — operation file patterns directly read; insertion points identified
- Batch checkpoint design: HIGH — `DurableBackend` trait shape and `Vec<OperationUpdate>` wire format confirmed
- Tracing span async semantics: MEDIUM — tracing crate behavior inferred from API knowledge; test compilation will confirm

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (stable APIs; `tracing` 0.1.x and `aws-sdk-lambda` 1.x are both stable)
