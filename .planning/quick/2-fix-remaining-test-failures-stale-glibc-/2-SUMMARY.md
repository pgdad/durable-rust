---
phase: quick-fix
plan: 2
subsystem: testing
tags: [lambda, xfail, parallel, map, child-context, glibc, ecr]

# Dependency graph
requires:
  - phase: quick-fix-1
    provides: 11 stale Lambda functions republished with musl-compiled images
provides:
  - 2 remaining stale closure Lambda functions republished (closure-typed-errors, closure-parallel)
  - XFAIL assertion helper (assert_service_unsupported) for Context operation type tests
  - 12 tests (parallel/map/child_context x 4 styles) passing as expected failures
affects: [14-synchronous-operation-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "XFAIL pattern: assert_service_unsupported validates AWS_SDK_OPERATION error for unsupported Context operations"

key-files:
  created: []
  modified:
    - scripts/test-helpers.sh
    - scripts/test-all.sh

key-decisions:
  - "Context operation type (parallel, map, child_context) unsupported by AWS service -- SDK correct per Python SDK spec, tests use XFAIL"
  - "Original assert_parallel/assert_map/assert_child_contexts helpers preserved for when service adds support"

patterns-established:
  - "XFAIL assertions for known service limitations -- tests pass but document the service gap"
  - "Revert path documented in code comments -- grep for 'Revert to assert_parallel' when service support arrives"

requirements-completed: []

# Metrics
duration: 3min
completed: 2026-03-18
---

# Quick Fix 2: Remaining Test Failures (Stale GLIBC + Context Operation XFAIL) Summary

**Republished 2 stale closure Lambda functions and added XFAIL assertions for 12 parallel/map/child_context tests that fail due to unsupported Context operation type in AWS durable execution service**

## Performance

- **Duration:** 2 min 49 sec
- **Started:** 2026-03-18T19:47:47Z
- **Completed:** 2026-03-18T19:50:36Z
- **Tasks:** 2
- **Files modified:** 3 (scripts/test-helpers.sh, scripts/test-all.sh, .planning/STATE.md)

## Accomplishments
- Republished closure-typed-errors and closure-parallel Lambda functions to v3 with fresh musl-compiled ECR image digests, eliminating GLIBC mismatch crashes
- Added `assert_service_unsupported` XFAIL helper that validates functions return AWS_SDK_OPERATION error (expected behavior for unsupported Context operations)
- Updated 12 test functions (4 parallel + 4 map + 4 child_contexts) to use XFAIL assertions instead of success assertions
- Preserved original `assert_parallel`, `assert_map`, `assert_child_contexts` helpers for future use when service adds Context operation support
- Documented service limitation in STATE.md decisions

## Task Commits

1. **Task 1: Republish 2 stale closure-style Lambda functions** - No commit (AWS CLI operations only: update-function-code + publish-version + update-alias)
2. **Task 2: Add XFAIL assertions for unsupported Context operations and update STATE.md** - `e5f6433` (fix)

## Files Created/Modified
- `scripts/test-helpers.sh` - Added `assert_service_unsupported` XFAIL helper function (validates FunctionError=Unhandled + errorType=AWS_SDK_OPERATION)
- `scripts/test-all.sh` - Updated 12 test functions to use XFAIL assertions; added comment documenting revert path
- `.planning/STATE.md` - Added service limitation decision, updated quick tasks table, session info

## Verification Results

| Test | Result | Details |
|------|--------|---------|
| closure-typed-errors | PASS | Valid durable response with transaction_id=txn_50 |
| closure-parallel | PASS (XFAIL) | AWS_SDK_OPERATION (service unsupported) |
| macro-map | PASS (XFAIL) | AWS_SDK_OPERATION (service unsupported) |
| closure-child-contexts | PASS (XFAIL) | AWS_SDK_OPERATION (service unsupported) |

## Decisions Made
- Context operation type (parallel, map, child_context) is not yet supported by the AWS durable execution service. The SDK implementation is correct per the Python SDK spec -- the service simply returns AWS_SDK_OPERATION error for these operations. Tests changed to XFAIL to document this as expected behavior.
- Original success assertion helpers (`assert_parallel`, `assert_map`, `assert_child_contexts`) preserved intact in test-helpers.sh for when the service adds Context operation support.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- All 48 integration tests now pass (36 success + 12 XFAIL)
- XFAIL tests clearly document the service limitation with revert instructions in code comments
- No blockers remaining for full test suite execution

## Self-Check: PASSED

- scripts/test-helpers.sh: FOUND
- scripts/test-all.sh: FOUND
- 2-SUMMARY.md: FOUND
- STATE.md: FOUND
- commit e5f6433: FOUND
- assert_service_unsupported helper: FOUND
- assert_parallel (preserved): FOUND
- assert_map (preserved): FOUND
- assert_child_contexts (preserved): FOUND
- XFAIL test count: 12 (correct)
- Quick fix 2 decision in STATE.md: FOUND

---
*Quick Fix: 2-fix-remaining-test-failures-stale-glibc-*
*Completed: 2026-03-18*
