---
phase: 04-input-validation-error-codes
plan: 02
subsystem: error-handling
tags: [rust, error-types, retry-logic, durable-lambda]

# Dependency graph
requires: []
provides:
  - ".code() method on DurableError returning &'static str stable error codes for all 15 variants"
  - "Structured variant-based retry detection in is_retryable_error (not string scanning)"
  - "Key fix: CheckpointFailed with transient messages no longer incorrectly retried"
affects:
  - 04-input-validation-error-codes
  - any phase using DurableError for programmatic error matching

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Error codes as SCREAMING_SNAKE_CASE &'static str from exhaustive match (no wildcard arm)"
    - "Retry detection via DurableError variant matching — only AwsSdkOperation/AwsSdk are retryable"
    - "TDD: RED (failing tests) committed before GREEN (implementation)"

key-files:
  created: []
  modified:
    - crates/durable-lambda-core/src/error.rs
    - crates/durable-lambda-core/src/backend.rs

key-decisions:
  - "No wildcard arm in code() match — compiler enforces exhaustive coverage when new variants are added"
  - "Only AwsSdkOperation and AwsSdk variants qualify as retryable; all others are deterministic"
  - "CheckpointFailed is never retried even if its source message contains transient error keywords"

patterns-established:
  - "code() pattern: exhaustive match returning &'static str, no _ wildcard, forces update on new variant"
  - "Retry gate: match on variant first, then inspect message only for the eligible variants"

requirements-completed: [FEAT-05, FEAT-06, FEAT-07]

# Metrics
duration: 5min
completed: 2026-03-16
---

# Phase 04 Plan 02: Error Codes and Structured Retry Detection Summary

**Stable SCREAMING_SNAKE_CASE error codes on DurableError via exhaustive .code() match, plus variant-gated retry logic that prevents incorrect retry of non-transient errors like CheckpointFailed.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-16T14:03:22Z
- **Completed:** 2026-03-16T14:07:34Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Added `.code()` method to DurableError returning a stable `&'static str` for all 15 variants, with an exhaustive match (no wildcard arm) so the compiler enforces updates when new variants are added
- Added `error_code_replay_mismatch` and `all_error_variants_have_unique_codes` tests verifying every code string and uniqueness via HashSet
- Refactored `is_retryable_error` from fragile string-scanning on display output to structured DurableError variant matching — only `AwsSdkOperation` and `AwsSdk` are retryable
- Fixed key behavioral bug: `CheckpointFailed` containing "Throttling" in its source message was previously incorrectly retried; now correctly returns false
- Updated existing tests that used wrong variant (`checkpoint_failed`) to use correct `aws_sdk_operation` variant, and added 6 new backend retry tests

## Task Commits

Each task was committed atomically:

1. **Task 1: Add .code() method to DurableError with tests** - `2ed85a8` (feat)
2. **Task 2: Refactor is_retryable_error to use variant matching** - `b2b949f` (feat)

**Plan metadata:** (committed after SUMMARY creation)

_Note: Both tasks used TDD workflow (RED failing tests first, then GREEN implementation)._

## Files Created/Modified
- `crates/durable-lambda-core/src/error.rs` - Added `.code()` method (exhaustive match, 15 variants), added uniqueness/per-variant tests
- `crates/durable-lambda-core/src/backend.rs` - Replaced string-scanning `is_retryable_error` with variant-matched impl, updated and added 9 tests total

## Decisions Made
- No wildcard arm in `.code()` match — exhaustive match inside the defining crate means adding a new DurableError variant will produce a compile error, forcing `.code()` to be updated. This is intentional: stability is enforced by the type system.
- `CheckpointFailed` is classified as non-retryable even when its wrapped source error mentions throttling. The rationale is that a checkpoint failure is a structural SDK failure, not a transient AWS API error. Only `AwsSdkOperation` and `AwsSdk` represent direct AWS API calls that can transiently fail.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Linter auto-reverted test changes to backend.rs during RED phase. Resolved by re-reading file and reapplying changes before proceeding to GREEN.
- Pre-existing compilation errors in `durable-lambda-builder` from other in-progress plan files (04-01, 04-03 uncommitted changes in working tree). Confirmed out-of-scope by verifying they exist in baseline before my changes. Not fixed per deviation rules scope boundary.

## User Setup Required
None - no external service configuration required.

## Self-Check: PASSED

All files exist and all commits verified:
- `crates/durable-lambda-core/src/error.rs` - FOUND
- `crates/durable-lambda-core/src/backend.rs` - FOUND
- `.planning/phases/04-input-validation-error-codes/04-02-SUMMARY.md` - FOUND
- Commit `2ed85a8` - FOUND
- Commit `b2b949f` - FOUND

## Next Phase Readiness
- `.code()` method is stable API surface; callers can now match errors programmatically without parsing display strings
- `is_retryable_error` correctly gates retry only to AWS transient errors; no false positives on deterministic errors
- All 133 unit tests in `durable-lambda-core` pass (0 failures)

---
*Phase: 04-input-validation-error-codes*
*Completed: 2026-03-16*
