---
phase: 01-error-path-test-coverage
plan: 02
subsystem: testing
tags: [rust, tokio, durable-execution, parallel, map, batch-result, error-paths]

# Dependency graph
requires:
  - phase: 01-error-path-test-coverage
    plan: 01
    provides: error_paths.rs file with shared mock backends and single-operation tests (TEST-01 to TEST-07)

provides:
  - Batch operation error-path tests: TEST-08 (parallel all-fail), TEST-09 (map positions), TEST-11 (parallel panic)
  - Proof that branch-level DurableError returns are captured in BatchResult, not propagated as Err
  - Proof that tokio JoinError (from panic) propagates as Err(DurableError::ParallelFailed)

affects:
  - future parallel/map implementation changes (these tests lock behavioral contracts)
  - any phase adding new batch operation error modes

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Branch-level DurableError goes into BatchResult::Failed, panic JoinError propagates as DurableError::ParallelFailed"
    - "BoxedBranchFn type alias pattern for parallel test vectors"
    - "PassingMockBackend reused for execute-mode batch operation tests"

key-files:
  created: []
  modified:
    - tests/e2e/tests/error_paths.rs

key-decisions:
  - "Panic test (TEST-11) uses #[allow(unreachable_code)] after the panic! macro to satisfy type inference for the Ok arm"
  - "Map closure in TEST-09 uses |item: i32, _ctx: DurableContext| (item first, ctx second) matching map.rs FnOnce(I, DurableContext) signature"

patterns-established:
  - "Batch error tests use PassingMockBackend with empty history (execute mode) — no pre-loaded operations needed"
  - "For parallel test branch vectors: declare BranchFn type alias inline then build Vec<BranchFn>"

requirements-completed: [TEST-08, TEST-09, TEST-11]

# Metrics
duration: 2min
completed: 2026-03-16
---

# Phase 01 Plan 02: Batch Operation Error-Path Tests Summary

**3 tests proving parallel branch errors go to BatchResult while branch panics become Err(DurableError::ParallelFailed), and map isolates per-item failures at all positions**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-16T14:15:02Z
- **Completed:** 2026-03-16T14:17:51Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- TEST-08: `test_parallel_all_branches_fail` — proves that when all branches return `Err(DurableError::...)`, `parallel()` returns `Ok(BatchResult)` with every item `BatchItemStatus::Failed`, not `Err`
- TEST-09: `test_map_item_failures_at_different_positions` — 5-item map with failures at indices 0, 2, 4 and successes at 1, 3; asserts correct `status`, `error`, and `result` values at each position
- TEST-11: `test_parallel_branch_panic_returns_error` — branch that calls `panic!()` causes `tokio::spawn` JoinError, which the SDK converts to `Err(DurableError::ParallelFailed)` via `map_err + ?`

## Task Commits

Each task was committed atomically:

1. **Task 1: Add batch operation error tests to error_paths.rs** - `0238ca9` (test)

**Plan metadata:** (included in final state commit)

## Files Created/Modified

- `/home/esa/git/durable-rust/tests/e2e/tests/error_paths.rs` — Appended 3 test functions (TEST-08, TEST-09, TEST-11) and updated module-level doc comment and imports (`std::future::Future`, `std::pin::Pin`, `BatchItemStatus`, `MapOptions`, `ParallelOptions`)

## Decisions Made

- Used `#[allow(unreachable_code)]` on the `Ok(0i32)` after `panic!()` in TEST-11 to satisfy Rust's type inference for the branch return type.
- Map closure parameter order is `|item: I, ctx: DurableContext|` (item first) matching the `map()` signature `F: FnOnce(I, DurableContext) -> Fut`.
- Type alias `BranchFn` declared inline per test function (not module-level) to avoid naming conflicts between TEST-08 and TEST-11.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 01-02 complete; Phase 01 (error-path test coverage) is now fully executed (Plans 01 and 02 complete)
- All 10 error-path tests green (`cargo test --test error_paths`)
- Full workspace green (`cargo test --workspace`)
- Clippy clean (`cargo clippy --workspace -- -D warnings`)

---
*Phase: 01-error-path-test-coverage*
*Completed: 2026-03-16*

## Self-Check: PASSED
