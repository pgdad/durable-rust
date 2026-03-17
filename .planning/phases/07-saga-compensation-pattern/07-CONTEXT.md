# Phase 7: Saga / Compensation Pattern - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning
**Source:** Auto-generated from prior context, codebase analysis, and requirement specs

<domain>
## Phase Boundary

First-class saga support: users register compensation closures alongside forward steps. On workflow failure, compensations execute in reverse registration order with durable checkpointing. This is a new operation built on top of existing step/checkpoint infrastructure — no changes to the replay engine or operation ID generation.

</domain>

<decisions>
## Implementation Decisions

### API surface
- `ctx.step_with_compensation(name, forward_fn, compensate_fn)` — follows existing parameter convention (name, options, closure)
- Forward function signature identical to `step()` — `FnOnce() -> impl Future<Output = Result<T, E>>`
- Compensation function receives the forward result as input — `FnOnce(T) -> impl Future<Output = Result<(), CompensationError>>`
- Compensation closures must be `Send + 'static` (consistent with step/parallel closure requirements from CLAUDE.md)
- Return type: `Result<Result<T, E>, DurableError>` — same two-level pattern as `step()`

### Compensation registration and storage
- Compensations stored in a `Vec<CompensationRecord>` on `DurableContext`
- Each record contains: operation name, serialized forward result, and the compensation closure (type-erased via `Box<dyn FnOnce>`)
- Registration happens after forward step succeeds — failed forward steps have nothing to compensate
- `CompensationRecord` must be serializable for checkpoint persistence (closure stored as operation reference, not the closure itself)

### Execution semantics
- Compensations fire on explicit `ctx.run_compensations()` call, not automatically on any error
- Reverse order: last-registered compensation runs first (stack semantics)
- Each compensation is checkpointed as its own operation (START + SUCCEED/FAIL) — durable rollback
- Compensation operations use `OperationType::Context` with `sub_type: "Compensation"` — consistent with parallel/map/child_context pattern
- Child context isolation: each compensation runs in its own child context for operation ID namespacing

### Failure handling
- Compensation failure is captured per-item, not abort-on-first — all compensations attempt to run
- Results returned as `CompensationResult` with per-item success/failure status (like `BatchResult` pattern)
- A `DurableError::CompensationFailed` variant for infrastructure failures during compensation execution
- Compensation of a compensation is NOT supported (no recursive saga) — keep it simple

### Replay behavior
- During replay, compensation operations replay from history like any other operation (no special handling)
- Forward step results cached; compensation closures NOT re-executed during replay (same as step closures)
- Partial compensation (3 of 5 complete, re-invocation) resumes from checkpoint — compensations already completed are skipped

### Claude's Discretion
- Internal data structure for `CompensationRecord` (Vec vs VecDeque)
- Whether `step_with_compensation` also accepts `StepOptions` (timeout, retries) for the forward step
- Exact naming of the `CompensationResult` struct fields
- Whether to add `step_with_compensation` to `DurableContextOps` trait (recommended: yes)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `step_with_options()` in `operations/step.rs`: The forward execution path can be reused — `step_with_compensation` wraps a regular step call and registers the compensation on success
- `BatchResult<T>` / `BatchItemStatus` in `types.rs`: Pattern for per-item success/failure reporting — reuse for `CompensationResult`
- `create_child_context()` in `context.rs`: Each compensation gets its own child context for isolated operation IDs
- `DurableError` with `.code()` method: New `CompensationFailed` variant follows established pattern with `"COMPENSATION_FAILED"` code
- `DurableContextOps` trait in `ops_trait.rs`: New methods should be added here and delegated by all 3 wrapper crates

### Established Patterns
- Checkpoint protocol: START then SUCCEED/FAIL per operation — compensations follow this
- Operation type: `OperationType::Context` with `sub_type` discriminator — used by parallel, map, child_context
- Type erasure: `Arc<dyn Fn>` for Clone, `Box<dyn FnOnce>` for single-use — compensations are single-use (Box)
- Mock testing: `MockDurableContext` builder pattern with `.with_step_result()` — needs `.with_compensation()` equivalent

### Integration Points
- `DurableContext` struct gains `compensations: Vec<CompensationRecord>` field
- `create_child_context()` must NOT copy parent compensations (compensation registration is parent-scoped)
- `ops_trait.rs` gains `step_with_compensation()` and `run_compensations()` methods
- Wrapper crates delegate new methods to `self.inner`
- `error.rs` gains `DurableError::CompensationFailed` variant and `compensation_failed()` constructor

</code_context>

<specifics>
## Specific Ideas

- Compensation pattern should feel like database transactions: "if any of these fail, undo what succeeded"
- The API should be usable without understanding saga theory — `step_with_compensation` is the only new concept users need to learn
- Example use case: order processing where charge → reserve_inventory → ship, and failure at any point rolls back prior steps

</specifics>

<deferred>
## Deferred Ideas

- Nested sagas (compensation within compensation) — too complex for v1, could be a future enhancement
- Automatic compensation on any `DurableError` (instead of explicit `run_compensations()`) — potential future sugar
- Timeout on entire saga (all compensations must complete within N seconds) — could compose with step timeout

</deferred>

---

*Phase: 07-saga-compensation-pattern*
*Context gathered: 2026-03-17 via auto-mode*
