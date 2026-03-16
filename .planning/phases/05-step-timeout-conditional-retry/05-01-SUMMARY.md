---
phase: 05-step-timeout-conditional-retry
plan: "01"
subsystem: sdk-core
tags: [tokio, timeout, retry, step-options, durable-error, type-erasure, arc]

# Dependency graph
requires: []
provides:
  - StepOptions with timeout_seconds (u64) and retry_if (type-erased Arc predicate) fields
  - DurableError::StepTimeout variant with step_timeout() constructor and STEP_TIMEOUT code
  - step_with_options wrapped in tokio::time::timeout when timeout_seconds is set
  - Conditional retry evaluation: retry_if predicate checked before retry budget
  - RetryPredicate type alias for Arc<dyn Fn(&dyn Any) -> bool + Send + Sync>
affects: [06-invoke-options, 07-map-options, e2e-tests, compliance-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Type-erased closure storage via Arc<dyn Fn(&dyn Any)> for Clone-compatible predicates"
    - "is_some_and(&predicate) for concise downcast+evaluate pattern"
    - "RetryPredicate type alias to satisfy clippy::type_complexity"
    - "tokio::time::timeout on &mut JoinHandle for abort-on-expiry"

key-files:
  created: []
  modified:
    - crates/durable-lambda-core/src/types.rs
    - crates/durable-lambda-core/src/error.rs
    - crates/durable-lambda-core/src/operations/step.rs

key-decisions:
  - "Used RetryPredicate type alias for Arc<dyn Fn(&dyn Any) -> bool + Send + Sync> to satisfy clippy::type_complexity"
  - "retry_if uses is_some_and(&predicate) instead of map_or(false, |e| predicate(e)) per clippy::unnecessary-map-or"
  - "timeout applies to spawned task only (closure execution), not to checkpoint I/O"
  - "retry_if predicate returning false skips retry without consuming retry budget (FEAT-14)"
  - "No retry_if predicate defaults to retrying all errors (backward compatible)"
  - "StepTimeout returned immediately without sending checkpoint; caller propagates error"

patterns-established:
  - "TDD RED-GREEN: write failing tests, confirm failure, implement, confirm pass"
  - "Builder validation: assert!(cond, 'Type::method: constraint, got {val}') pattern"
  - "Manual Debug impl for types with fn fields: show '<predicate>' placeholder"

requirements-completed: [FEAT-09, FEAT-10, FEAT-11, FEAT-13, FEAT-14, FEAT-15]

# Metrics
duration: 6min
completed: 2026-03-16
---

# Phase 05 Plan 01: Step Timeout and Conditional Retry Infrastructure Summary

**Per-step execution timeouts via tokio::time::timeout and type-erased retry predicates via Arc<dyn Fn(&dyn Any)> added to StepOptions and integrated into step_with_options**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-16T17:31:15Z
- **Completed:** 2026-03-16T17:37:13Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Extended StepOptions with timeout_seconds (Option<u64>) and retry_if (Option<RetryPredicate>) using Arc-based storage for Clone support
- Added DurableError::StepTimeout variant with step_timeout() constructor and exhaustive code() arm returning "STEP_TIMEOUT"
- Integrated tokio::time::timeout wrapping in step_with_options; task aborted via handle.abort() on expiry
- Added retry predicate check before retry budget evaluation; false predicate triggers immediate FAIL checkpoint

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend StepOptions and DurableError with timeout and retry_if** - `168d355` (feat)
2. **Task 2: Integrate timeout and conditional retry into step_with_options** - `98061f8` (feat)

**Plan metadata:** (docs commit, see below)

_Note: TDD tasks — tests written and verified failing before each implementation_

## Files Created/Modified
- `crates/durable-lambda-core/src/types.rs` - Added RetryPredicate alias, timeout_seconds/retry_if fields, manual Debug impl, builder methods, getters; 10 new unit tests
- `crates/durable-lambda-core/src/error.rs` - Added StepTimeout variant, step_timeout() constructor, STEP_TIMEOUT code arm; 2 new tests, updated all_error_variants test
- `crates/durable-lambda-core/src/operations/step.rs` - Added Duration import, timeout-aware execution block, retry predicate check; 5 new integration tests

## Decisions Made
- Used `RetryPredicate` type alias for `Arc<dyn Fn(&dyn Any) -> bool + Send + Sync>` to satisfy clippy::type_complexity — avoids repeating the complex type in struct field and getter return type
- `retry_if` builder uses `is_some_and(&predicate)` instead of `map_or(false, |e| predicate(e))` — clippy::unnecessary-map-or and clippy::redundant-closure enforced
- Timeout wraps only the spawned task execution, not checkpoint I/O — keeps checkpoint protocol unaffected by user timeout
- `StepTimeout` is returned immediately as `Err`; no checkpoint is sent — the step never reached the checkpoint phase

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed four clippy::type_complexity, clippy::unnecessary-map-or, and clippy::redundant-closure warnings**
- **Found during:** Task 2 verification (cargo clippy --workspace -- -D warnings)
- **Issue:** Inline complex type `Arc<dyn Fn(&dyn Any) -> bool + Send + Sync>` repeated in struct field and getter; `map_or(false, ...)` pattern rejected by clippy
- **Fix:** Introduced `RetryPredicate` type alias; replaced `map_or` with `is_some_and`; replaced `|e| predicate(e)` closure with `&predicate`
- **Files modified:** `crates/durable-lambda-core/src/types.rs`
- **Verification:** `cargo clippy --workspace -- -D warnings` passes clean
- **Committed in:** `98061f8` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - clippy lint fixes)
**Impact on plan:** Necessary for CI compliance (clippy -D warnings). No scope creep.

## Issues Encountered
None — both tasks completed in one RED-GREEN cycle each.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Step timeout and conditional retry infrastructure is complete and tested
- Ready for Plan 05-02 (e2e tests covering timeout and retry_if scenarios)
- All 148 existing tests pass; no regressions introduced

---
*Phase: 05-step-timeout-conditional-retry*
*Completed: 2026-03-16*
