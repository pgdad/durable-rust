---
phase: 07-saga-compensation-pattern
plan: 01
subsystem: core
tags: [rust, saga, compensation, durable-execution, checkpoint, replay]

# Dependency graph
requires:
  - phase: 06-observability-batch-checkpoint
    provides: DurableContext with batch checkpoint API, tracing spans, checkpoint protocol
  - phase: 03-core-architecture-refactor
    provides: DurableContext struct, DurableBackend trait, operations pattern
provides:
  - DurableError::CompensationFailed variant with code COMPENSATION_FAILED
  - CompensateFn type alias, CompensationRecord, CompensationResult, CompensationItem, CompensationStatus types
  - DurableContext::compensations field with push_compensation/take_compensations/compensation_count
  - DurableContext::step_with_compensation — forward step + typed compensation registration
  - DurableContext::step_with_compensation_opts — same with StepOptions support
  - DurableContext::run_compensations — LIFO execution with per-item checkpointing and replay
affects:
  - 07-02 (integration tests will use this API)
  - e2e-tests (new compensation workflow tests)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - CompensateFn type-erased async closure (FnOnce(Value) -> Pin<Box<dyn Future>>)
    - LIFO compensation stack: registered in order, executed in reverse
    - Continue-on-error compensation semantics: all compensations attempt regardless of failures
    - Partial rollback resume: replay engine tracks completed compensations by op_id
    - Context/START + Context/SUCCEED|FAIL with sub_type=Compensation (mirrors child_context protocol)

key-files:
  created:
    - crates/durable-lambda-core/src/operations/compensation.rs
  modified:
    - crates/durable-lambda-core/src/error.rs
    - crates/durable-lambda-core/src/types.rs
    - crates/durable-lambda-core/src/context.rs
    - crates/durable-lambda-core/src/lib.rs
    - crates/durable-lambda-core/src/operations/mod.rs

key-decisions:
  - "CompensationRecord does NOT implement Debug — contains a closure (CompensateFn), which is not Debug"
  - "Compensations are NOT inherited by child contexts (create_child_context starts empty)"
  - "run_compensations uses continue-on-error semantics — all compensations attempt even when one fails"
  - "Partial rollback resume: replay engine check_result() detects already-completed compensation op_ids and skips closure execution"
  - "CompensateFn wraps typed G: FnOnce(T) -> GFut with JSON serialization/deserialization to achieve type erasure"
  - "Context/START checkpoint sent before executing compensation closure; Context/SUCCEED or Context/FAIL sent after"

patterns-established:
  - "Compensation checkpoint sub_type='Compensation' (distinct from 'Context' used by child_context)"
  - "Type erasure for compensation closures via Box<dyn FnOnce(Value) -> Pin<Box<dyn Future>>>"
  - "Borrow checker split: extract needed data from replay_engine().check_result() before calling replay_engine_mut()"

requirements-completed: [FEAT-25, FEAT-26, FEAT-27]

# Metrics
duration: 57min
completed: 2026-03-17
---

# Phase 07 Plan 01: Saga Compensation Pattern — Core Implementation Summary

**Saga/compensation pattern with type-erased LIFO closures, per-item Context/START+SUCCEED/FAIL checkpointing, and partial-rollback resume via replay engine op_id tracking**

## Performance

- **Duration:** 57 min
- **Started:** 2026-03-17T00:00:00Z
- **Completed:** 2026-03-17T00:57:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added `DurableError::CompensationFailed` variant with code `COMPENSATION_FAILED` and `compensation_failed()` constructor
- Added compensation types: `CompensateFn`, `CompensationRecord`, `CompensationResult`, `CompensationItem`, `CompensationStatus`
- Added `compensations: Vec<CompensationRecord>` field to `DurableContext` with `push_compensation`, `take_compensations`, `compensation_count` methods
- Implemented `step_with_compensation` delegating to `step()` and registering typed compensation on success
- Implemented `step_with_compensation_opts` delegating to `step_with_options()`
- Implemented `run_compensations` with LIFO execution, `Context/START + Context/SUCCEED|FAIL` checkpointing, continue-on-error semantics, and partial rollback resume via replay
- Exported `CompensationResult`, `CompensationItem`, `CompensationStatus` from `lib.rs`
- 19 compensation-specific unit tests pass; full workspace (112 tests) clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Add CompensationFailed error variant and compensation types** - `4a5367c` (feat)
2. **Task 2: Implement step_with_compensation and run_compensations** - `1c9f8f2` (feat)

**Plan metadata:** (created in this step) (docs: complete plan)

_Note: TDD tasks — test and implementation committed together per task_

## Files Created/Modified

- `crates/durable-lambda-core/src/operations/compensation.rs` — `step_with_compensation`, `step_with_compensation_opts`, `run_compensations` implementations with 19 co-located unit tests
- `crates/durable-lambda-core/src/error.rs` — `CompensationFailed` variant, `compensation_failed()` constructor, `code()` exhaustive match updated
- `crates/durable-lambda-core/src/types.rs` — `CompensateFn` type alias, `CompensationRecord`, `CompensationResult`, `CompensationItem`, `CompensationStatus`
- `crates/durable-lambda-core/src/context.rs` — `compensations` field, `push_compensation`, `take_compensations`, `compensation_count` methods; field initialized empty in both `new()` and `create_child_context()`
- `crates/durable-lambda-core/src/lib.rs` — exports `CompensationResult`, `CompensationItem`, `CompensationStatus`
- `crates/durable-lambda-core/src/operations/mod.rs` — registered `pub mod compensation`

## Decisions Made

- **CompensationRecord is not Debug:** Contains a closure (`CompensateFn`), which cannot implement Debug. Documented in rustdoc.
- **Child contexts start with empty compensations:** `create_child_context()` initializes `compensations: Vec::new()`. Compensations are per-context, not inherited.
- **Continue-on-error semantics:** `run_compensations` never aborts on a failing compensation — all are attempted. Per-item status captured in `CompensationResult.items`.
- **Type erasure pattern:** The typed `G: FnOnce(T) -> GFut` compensation is wrapped in a `CompensateFn` (Box<dyn FnOnce(Value) -> Pin<...>>) that deserializes the JSON value to `T` before calling the original closure.
- **Borrow checker split:** `replay_engine().check_result()` data extracted into a local variable before calling `replay_engine_mut().track_replay()` — avoids simultaneous immutable+mutable borrows of `self`.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- Borrow checker error in `run_compensations`: attempted to call `replay_engine_mut()` while holding an immutable reference from `check_result()`. Fixed by extracting the needed data (succeeded flag and error message) into a local variable before the mutable borrow. Resolved immediately without deviation from plan scope.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Core saga/compensation API is complete and fully tested
- `step_with_compensation`, `step_with_compensation_opts`, and `run_compensations` are ready for use in integration/e2e tests
- Partial rollback resume works correctly via replay engine

---
*Phase: 07-saga-compensation-pattern*
*Completed: 2026-03-17*
