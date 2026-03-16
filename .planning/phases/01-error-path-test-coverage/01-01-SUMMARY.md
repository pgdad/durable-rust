---
phase: 01-error-path-test-coverage
plan: 01
subsystem: testing
tags: [rust, async, error-handling, durable-execution, mock-backend]

# Dependency graph
requires: []
provides:
  - "Error-path test suite covering 7 single-operation failure scenarios (TEST-01 to TEST-07)"
  - "FailingMockBackend and PassingMockBackend helpers for direct DurableContext construction"
  - "Pattern for constructing Operation structs in e2e tests without MockDurableContext builder"
affects:
  - 01-error-path-test-coverage
  - future-test-phases

# Tech tracking
tech-stack:
  added:
    - "aws-sdk-lambda workspace dep added to tests/e2e (for Operation struct construction)"
    - "aws-smithy-types workspace dep added to tests/e2e (for DateTime::from_secs)"
  patterns:
    - "FailingMockBackend pattern: DurableBackend impl that injects Err on checkpoint() for checkpoint failure tests"
    - "PassingMockBackend pattern: minimal DurableBackend impl for replay-path error tests"
    - "OperationIdGenerator::new(None).next_id() to compute deterministic operation IDs for pre-populated history"

key-files:
  created:
    - tests/e2e/tests/error_paths.rs
  modified:
    - tests/e2e/Cargo.toml

key-decisions:
  - "Used OperationStatus::Cancelled (not Pending) for TEST-01 replay mismatch — Cancelled is completed (handled by check_result) but extract_step_result returns ReplayMismatch for it"
  - "PassingMockBackend reused across TEST-04 through TEST-07 instead of MockDurableContext builder — tests need to construct history Operations directly with specific statuses not exposed by the builder"
  - "aws-sdk-lambda and aws-smithy-types added as direct deps to e2e Cargo.toml — required for constructing Operation structs with arbitrary statuses in error-path tests"

patterns-established:
  - "Error-path tests use direct DurableContext::new() with pre-constructed Operations rather than MockDurableContext builder when specific non-standard statuses are needed"
  - "matches! macro used for DurableError variant assertions — avoids string parsing, type-safe"
  - "expect() with descriptive messages used throughout — no bare unwrap() in test code"

requirements-completed: [TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06, TEST-07]

# Metrics
duration: 20min
completed: 2026-03-16
---

# Phase 01 Plan 01: Error Path Test Coverage — Single Operations Summary

**7 typed-error assertions covering replay mismatch, deserialization failure, checkpoint write failure, retry exhaustion, callback timeout, callback explicit failure, and invoke error using FailingMockBackend and pre-constructed Operation history**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-03-16T13:50:00Z
- **Completed:** 2026-03-16T14:11:36Z
- **Tasks:** 1
- **Files modified:** 3 (1 new, 2 modified)

## Accomplishments

- Created `tests/e2e/tests/error_paths.rs` with 7 test functions, each asserting a specific `DurableError` variant using the `matches!` macro
- Implemented `FailingMockBackend` that returns `CheckpointFailed` on every `checkpoint()` call, enabling TEST-03 without any AWS dependency
- Implemented `PassingMockBackend` that returns a stable token for tests needing working backends but pre-populated error state in history
- All 7 tests pass; full workspace test suite has zero failures; clippy reports zero warnings

## Task Commits

1. **Task 1: Create error_paths.rs with FailingMockBackend and single-operation error tests** - `9191d13` (test)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `tests/e2e/tests/error_paths.rs` — Error-path test suite with 7 tests (TEST-01 to TEST-07), `FailingMockBackend`, `PassingMockBackend`, and `first_op_id()` helper
- `tests/e2e/Cargo.toml` — Added `aws-sdk-lambda` and `aws-smithy-types` workspace deps for direct Operation construction
- `Cargo.lock` — Updated for two new direct deps

## Decisions Made

- **`OperationStatus::Cancelled` for TEST-01**: `Cancelled` is a completed status (processed by `check_result`), but `extract_step_result` returns `ReplayMismatch` for it since it's neither `Succeeded` nor `Failed`. `Pending` would not work as it's not a completed status and would take the retry re-execution path instead.
- **Direct `DurableContext::new()` construction over `MockDurableContext` builder**: The builder only supports standard success/failure cases. Error-path tests need `TimedOut`, `Cancelled`, `Pending` with specific attempt counts — statuses the builder doesn't expose. Direct construction with pre-built `Operation` structs is the right tool.
- **Separate `PassingMockBackend` from `FailingMockBackend`**: Tests for callback/invoke errors test the replay path (pre-populated history), not the checkpoint path. They need a passing backend so `DurableContext::new()` succeeds and `create_callback()` can replay from history without hitting the failing checkpoint.

## Deviations from Plan

None - plan executed exactly as written. All 7 test scenarios mapped directly to the described implementation approach. The plan's guidance about `OperationStatus::Cancelled` for TEST-01 and `attempt(4)` for TEST-04 was accurate.

## Issues Encountered

None. The unit test patterns in `step.rs`, `callback.rs`, and `invoke.rs` provided excellent reference implementations for constructing pre-populated `Operation` structs. The `make_callback_op` and `make_invoke_op` helper patterns from those modules were adapted directly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Single-operation error paths fully covered (TEST-01 through TEST-07)
- Plan 01-02 can proceed to multi-operation error scenarios (parallel all-branches-fail, map item failures, step closure panic)
- The `FailingMockBackend` and `PassingMockBackend` patterns established here are reusable in subsequent plans

---
*Phase: 01-error-path-test-coverage*
*Completed: 2026-03-16*

## Self-Check: PASSED

- FOUND: tests/e2e/tests/error_paths.rs
- FOUND: .planning/phases/01-error-path-test-coverage/01-01-SUMMARY.md
- FOUND: commit 9191d13 (test(01-01): add error-path tests for single-operation failure scenarios)
