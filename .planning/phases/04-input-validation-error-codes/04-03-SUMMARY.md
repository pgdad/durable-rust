---
phase: 04-input-validation-error-codes
plan: 03
subsystem: core-operations
tags: [rust, error-handling, checkpoint, durable-execution, defensive-programming]

# Dependency graph
requires:
  - phase: 04-input-validation-error-codes
    provides: DurableError::checkpoint_failed constructor and CheckpointFailed variant (plan 04-01, 04-02)
provides:
  - Defensive checkpoint_token handling across all 7 operation files (13 sites)
  - Test proving None checkpoint_token path returns DurableError::CheckpointFailed
affects:
  - All future operation implementations that add checkpoint calls
  - AWS API contract violation handling

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ok_or_else(|| DurableError::checkpoint_failed(...))? for all checkpoint token extractions"
    - "NoneTokenMockBackend pattern for testing None API response paths"

key-files:
  created: []
  modified:
    - crates/durable-lambda-core/src/operations/step.rs
    - crates/durable-lambda-core/src/operations/callback.rs
    - crates/durable-lambda-core/src/operations/wait.rs
    - crates/durable-lambda-core/src/operations/invoke.rs
    - crates/durable-lambda-core/src/operations/parallel.rs
    - crates/durable-lambda-core/src/operations/map.rs
    - crates/durable-lambda-core/src/operations/child_context.rs

key-decisions:
  - "Replace all 13 silent if-let-Some checkpoint_token patterns with ok_or_else error propagation — None token is an AWS API contract violation, not a normal case"
  - "Use std::io::Error::new(ErrorKind::InvalidData, message) as the source error for checkpoint_failed — fits the existing DurableError::checkpoint_failed signature"
  - "Test type annotation uses Result<Result<i32, String>, DurableError> to match step()'s actual return type"

patterns-established:
  - "All checkpoint responses must propagate None token as DurableError::CheckpointFailed rather than silently continuing with stale token"
  - "NoneTokenMockBackend: minimal mock returning CheckpointDurableExecutionOutput::builder().build() with no token for testing error paths"

requirements-completed:
  - FEAT-08

# Metrics
duration: 5min
completed: 2026-03-16
---

# Phase 04 Plan 03: Defensive Checkpoint Token Handling Summary

**All 13 silent checkpoint_token `if let Some` sites replaced with `.ok_or_else(|| DurableError::checkpoint_failed(...))` error propagation across 7 operation files, plus test proving the None token error path**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-16T14:06:13Z
- **Completed:** 2026-03-16T14:10:43Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments
- Replaced all 13 silent `if let Some(token) = response.checkpoint_token()` patterns with defensive `.ok_or_else(|| DurableError::checkpoint_failed(...))` error propagation
- A missing `checkpoint_token` in any AWS checkpoint response now surfaces as a typed `DurableError::CheckpointFailed` instead of silently continuing with a stale token
- Added `checkpoint_none_token_returns_error` test with `NoneTokenMockBackend` that returns checkpoint responses with no token, proving the None token error path returns the correct error variant and message

## Task Commits

Each task was committed atomically:

1. **Task 1: step.rs, callback.rs, wait.rs, invoke.rs** - `315a0d6` (feat)
2. **Task 2: parallel.rs, map.rs, child_context.rs** - `a7b5cde` (feat)
3. **Task 3: checkpoint_none_token_returns_error test** - `0af1c4f` (test)

## Files Created/Modified
- `crates/durable-lambda-core/src/operations/step.rs` - 4 sites updated + NoneTokenMockBackend + test added
- `crates/durable-lambda-core/src/operations/callback.rs` - 1 site updated (create_callback START)
- `crates/durable-lambda-core/src/operations/wait.rs` - 1 site updated (wait START)
- `crates/durable-lambda-core/src/operations/invoke.rs` - 1 site updated (invoke START)
- `crates/durable-lambda-core/src/operations/parallel.rs` - 2 sites updated (outer START + outer SUCCEED)
- `crates/durable-lambda-core/src/operations/map.rs` - 2 sites updated (outer START + outer SUCCEED)
- `crates/durable-lambda-core/src/operations/child_context.rs` - 2 sites updated (START + SUCCEED)

## Decisions Made
- None checkpoint_token is now treated as an AWS API contract violation, not a normal case — silent continuation with a stale token causes subtle downstream corruption
- Used `std::io::ErrorKind::InvalidData` as the error kind for the source error — fits the "unexpected data format from API" semantic
- Test uses `Result<Result<i32, String>, DurableError>` type to match `step()`'s actual double-wrapped return type

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Test type annotation required adjustment: the plan's example used `Result<i32, DurableError>` but `ctx.step()` returns `Result<Result<T, E>, DurableError>` — fixed by changing the annotation to `Result<Result<i32, String>, DurableError>`. No behavioral change.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 13 checkpoint token sites are now defensive — any future AWS API contract violation will surface as a typed error
- The `checkpoint_none_token_returns_error` test provides a regression guard for the None token path
- Phase 04 plan 04 (or next plan) can proceed

---
*Phase: 04-input-validation-error-codes*
*Completed: 2026-03-16*
