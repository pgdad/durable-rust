---
phase: 05-step-timeout-conditional-retry
plan: "03"
subsystem: parity-tests
tags: [tokio-time-pause, step-timeout, conditional-retry, batch-item-status, parallel, map, tdd]

# Dependency graph
requires: [05-01]
provides:
  - Cross-approach parity tests for step timeout (TEST-23)
  - Cross-approach parity tests for conditional retry via retry_if (TEST-23)
  - Complex workflow parity test: parallel + timeout step combination (TEST-24)
  - BatchItemStatus::Succeeded and BatchItemStatus::Failed per-item assertions (TEST-25)
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "tokio::time::pause() + tokio::time::advance() for instant timeout testing"
    - "tokio::select! to race step future against time advancement"
    - "BranchFn type alias for parallel branch boxed-future type to enable coercion"

key-files:
  created: []
  modified:
    - tests/parity/tests/parity.rs

key-decisions:
  - "Used tokio::time::pause() + advance() for step timeout test — avoids real 1-second sleep while proving the DurableContext timeout path fires correctly"
  - "BranchFn type alias required for parallel branches — inline Box::new closure without type alias cannot coerce Box::pin return to Pin<Box<dyn Future>> without explicit annotation"
  - "sort_by_key(|item| item.index) before assertions per decision [02-01] — concurrent execution may reorder batch items"

requirements-completed: [TEST-23, TEST-24, TEST-25]

# Metrics
duration: 3min
completed: 2026-03-16
---

# Phase 05 Plan 03: Step Timeout and Conditional Retry Parity Tests Summary

**7 new parity tests added covering step timeout, conditional retry via retry_if, parallel + timeout step combination, and per-item BatchItemStatus for both parallel and map operations**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-16T17:41:29Z
- **Completed:** 2026-03-16T17:44:30Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- Added 4 parity tests for TEST-23: step timeout (slow closure returns StepTimeout, fast closure returns Ok(Ok(42))), conditional retry_if (predicate false skips retry, no predicate retries all errors)
- Added 1 parity test for TEST-24: parallel containing step_with_options(timeout_seconds=5) + regular step both succeed via DurableContext
- Added 2 parity tests for TEST-25: BatchItemStatus::Succeeded and ::Failed verified per-item for parallel and map operations
- Total parity test count: 22 tests (was 15 before this plan)

## Task Commits

Each task was committed atomically:

1. **Task 1: Step timeout and conditional retry parity tests (TEST-23)** - `4b7e5c6` (test)
2. **Task 2: Complex workflow parity and BatchItemStatus verification (TEST-24, TEST-25)** - `19c7443` (test)

_Note: TDD tasks — tests written first, confirmed they compile, confirmed implementation from 05-01 makes them pass_

## Files Created/Modified
- `tests/parity/tests/parity.rs` - Added 7 new parity tests across TEST-23, TEST-24, and TEST-25 sections

## Decisions Made
- Used `tokio::time::pause()` + `tokio::time::advance()` for timeout test — proves the timeout path fires without a real 1-second sleep, keeping the test suite fast
- `BranchFn` type alias required in parallel tests to allow `Box::pin(async move { ... })` to coerce to `Pin<Box<dyn Future + Send>>`; without the alias the compiler cannot infer the trait object target type
- Sorted batch results by index before asserting per established decision [02-01] — concurrent tokio::spawn execution order is non-deterministic

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed two `#[non_exhaustive]` struct variant pattern match errors**
- **Found during:** Task 1 TDD compilation (cargo test run)
- **Issue:** `DurableError::StepTimeout { operation_name }` and `DurableError::StepRetryScheduled { operation_name }` both carry `#[non_exhaustive]` — pattern match without `..` is a compile error (E0638)
- **Fix:** Added `..` to both patterns: `StepTimeout { operation_name, .. }` and `StepRetryScheduled { operation_name, .. }`
- **Files modified:** `tests/parity/tests/parity.rs`
- **Verification:** `cargo test -p parity-tests` passes

**2. [Rule 1 - Bug] Fixed parallel branch type coercion failures in Task 2**
- **Found during:** Task 2 TDD compilation
- **Issue:** Inline `Box::new(|ctx| Box::pin(async move { ... }))` inside `vec![]` with mixed branches — compiler cannot coerce `Box::pin` return to `Pin<Box<dyn Future>>` without explicit target type
- **Fix:** Introduced `BranchFn` type alias matching the e2e test pattern (`Box<dyn FnOnce(DurableContext) -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>> + Send>`) and typed `let branches: Vec<BranchFn>`
- **Files modified:** `tests/parity/tests/parity.rs`
- **Verification:** `cargo test -p parity-tests` passes

---

**Total deviations:** 2 auto-fixed (Rule 1 - compile errors corrected)
**Impact on plan:** None — same semantics, just syntactically correct Rust

## Issues Encountered
None — both tasks completed in first attempt after fixing compile errors.

## User Setup Required
None — all tests use MockDurableContext, no AWS credentials needed.

## Next Phase Readiness
- All 7 new parity tests pass; total parity suite is 22 tests
- Full workspace test suite passes (148 tests before this plan, now ~155)
- Phase 05 plan 03 is the final plan in the phase
- Ready for Phase 06 (invoke options) per ROADMAP.md

---
*Phase: 05-step-timeout-conditional-retry*
*Completed: 2026-03-16*
