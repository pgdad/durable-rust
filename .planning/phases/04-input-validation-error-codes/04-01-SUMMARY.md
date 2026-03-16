---
phase: 04-input-validation-error-codes
plan: 01
subsystem: api
tags: [validation, panic, builder-pattern, types]

# Dependency graph
requires: []
provides:
  - Validated option builders with assert! guards in StepOptions, CallbackOptions, MapOptions
  - StepOptions::retries signature changed from u32 to i32 with non-negative guard
affects:
  - 04-02
  - 04-03
  - all callers of StepOptions::retries (signature changed)

# Tech tracking
tech-stack:
  added: []
  patterns: [assert!-based builder validation with descriptive panic messages, TDD red-green for API changes]

key-files:
  created: []
  modified:
    - crates/durable-lambda-core/src/types.rs

key-decisions:
  - "StepOptions::retries changed from u32 to i32 so negative values can be rejected with a clear panic at construction time (integer literals coerce automatically)"
  - "CallbackOptions and MapOptions use assert!(x > 0) guards; StepOptions uses assert!(x >= 0) since zero retries and zero backoff are valid"
  - "Panic messages include the field name and the invalid value, e.g. 'StepOptions::retries: count must be >= 0, got -1'"

patterns-established:
  - "Builder validation: use assert! with 'Type::method: constraint description, got {value}' format"
  - "TDD order: write failing tests (RED commit) then implement guards (GREEN commit)"

requirements-completed: [FEAT-01, FEAT-02, FEAT-03, FEAT-04]

# Metrics
duration: 3min
completed: 2026-03-16
---

# Phase 04 Plan 01: Input Validation Guards Summary

**Panic-on-construction validation added to all option builders: StepOptions (retries i32>=0, backoff>=0), CallbackOptions (timeout>0, heartbeat>0), MapOptions (batch_size>0)**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-16T14:03:26Z
- **Completed:** 2026-03-16T14:06:47Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- All 5 option builder setter methods now validate inputs at construction time
- `StepOptions::retries` signature changed from `u32` to `i32` enabling negative-value detection
- 11 new tests covering both valid and invalid input paths (panic tests use `#[should_panic(expected = "...")]`)
- All 234 tests in `durable-lambda-core` pass (133 unit + 6 integration + 95 doctests)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add validation tests for all option builders** - `8bfd3d1` (test) — RED phase
2. **Task 2: Implement validation guards and retries signature change** - `828fac5` (feat) — GREEN phase

_Note: TDD tasks have two commits (test RED → feat GREEN)_

## Files Created/Modified
- `crates/durable-lambda-core/src/types.rs` - Added assert! guards to 5 builder methods, changed retries from u32 to i32, updated doc comments with Panics sections, added 11 validation tests

## Decisions Made
- `StepOptions::retries` takes `i32` not `u32` so `-1` can be rejected at compile-representable runtime with a clear message. Internal storage remains `Option<u32>` with a safe `count as u32` cast post-assert.
- `CallbackOptions` uses `> 0` (strictly positive) since `0` means "omit setter entirely" per API contract.
- `StepOptions::backoff_seconds` and `retries` use `>= 0` since zero backoff (immediate retry) and zero retries are both valid.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

Pre-existing compile errors in `durable-lambda-builder` and a pre-existing failing test in `backend.rs` are out-of-scope for this plan. They exist in other uncommitted work-in-progress files. All `types.rs`-scoped tests pass cleanly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Input validation guards are in place for `types.rs`; `04-02` and `04-03` can proceed
- The `retries` signature change (u32 -> i32) is complete; any callers using typed `u32` variables would need updating but all current callers use integer literals which coerce automatically
