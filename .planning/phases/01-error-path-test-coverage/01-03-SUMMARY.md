---
phase: 01-error-path-test-coverage
plan: 03
subsystem: testing
tags: [panic-safety, tokio-spawn, step, error-paths, 'static-bounds]

# Dependency graph
requires:
  - phase: 01-error-path-test-coverage
    provides: error-path test infrastructure (FailingMockBackend, PassingMockBackend, DurableContext::new patterns)
provides:
  - Panic-safe step closure execution via tokio::spawn in step_with_options
  - 'static lifetime bounds on step() and step_with_options() across core and all 3 wrapper crates
  - test_step_closure_panic_returns_error test (TEST-10) in error_paths.rs
affects: [02-boundary-replay-engine-tests, 03-wrapper-crate-consolidation]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "tokio::spawn for panic isolation in durable step closures (mirrors parallel branch pattern)"
    - "'static + Send bounds on step closure types F and Fut (required by tokio::spawn)"
    - "Pre-clone pattern before move || closures when variable needed inside and after closure"

key-files:
  created: []
  modified:
    - crates/durable-lambda-core/src/operations/step.rs
    - crates/durable-lambda-core/src/ops_trait.rs
    - crates/durable-lambda-closure/src/context.rs
    - crates/durable-lambda-trait/src/context.rs
    - crates/durable-lambda-builder/src/context.rs
    - tests/e2e/tests/error_paths.rs

key-decisions:
  - "Use DurableError::checkpoint_failed (not a new variant) for step closure panics — panics are a form of failed checkpoint, not a new error category"
  - "step closure bounds now require 'static (same as parallel branches) — closures already move owned data per CLAUDE.md documented pattern, so this is practically always satisfied"
  - "std::io::Error::other(msg) preferred over std::io::Error::new(ErrorKind::Other, msg) per Clippy lint"
  - "Panic message captured from JoinError Display format which includes 'panicked' keyword for reliable assertion"

patterns-established:
  - "Panic boundary pattern: wrap async closure in tokio::spawn, map JoinError to DurableError with descriptive message"
  - "move || required on step closures that capture variables when 'static bound is present"

requirements-completed: [TEST-10]

# Metrics
duration: 45min
completed: 2026-03-16
---

# Phase 1 Plan 03: Step Closure Panic Safety Summary

**Panic-safe step closure execution via tokio::spawn with 'static bounds propagated to all wrapper crates and TEST-10 panic test added**

## Performance

- **Duration:** 45 min
- **Started:** 2026-03-16T14:00:00Z
- **Completed:** 2026-03-16T14:38:36Z
- **Tasks:** 2
- **Files modified:** 10 (6 production + 4 pre-existing formatting)

## Accomplishments
- Wrapped `f().await` in `step_with_options` with `tokio::spawn` so panics become `JoinError` → `DurableError::CheckpointFailed`
- Added `'static` bounds to `T`, `E`, `F`, `Fut` on `step()` and `step_with_options()` in core and all 3 wrapper crates (closure, trait, builder) plus `DurableContextOps` trait
- Fixed all existing closures across 20+ example and test files that now require `move` keyword due to `'static` bound
- Added `test_step_closure_panic_returns_error` to `tests/e2e/tests/error_paths.rs` — error_paths.rs now covers all 11 TEST-* items

## Task Commits

Each task was committed atomically:

1. **Task 1: Wrap step closure in tokio::spawn for panic safety** - `6adae1f` (feat)
2. **Task 2: Add test_step_closure_panic_returns_error for TEST-10** - `6f1e9dd` (test)

**Plan metadata:** (next commit — docs)

## Files Created/Modified
- `crates/durable-lambda-core/src/operations/step.rs` - tokio::spawn wrapper + 'static bounds on step and step_with_options
- `crates/durable-lambda-core/src/ops_trait.rs` - 'static bounds on DurableContextOps trait and DurableContext impl
- `crates/durable-lambda-closure/src/context.rs` - 'static bounds on ClosureContext step methods
- `crates/durable-lambda-trait/src/context.rs` - 'static bounds on TraitContext step methods
- `crates/durable-lambda-builder/src/context.rs` - 'static bounds on BuilderContext step methods
- `tests/e2e/tests/error_paths.rs` - TEST-10 test + MockDurableContext import + TEST-10 doc comment
- Various example files - added `move` keyword to step closures affected by 'static bound

## Decisions Made
- Used `DurableError::checkpoint_failed` for step panic errors (not a new variant) — panics are a form of failed checkpoint
- Step closure bounds now require `'static` matching the parallel branch model — already practically satisfied since step closures move owned data per documented pattern
- `std::io::Error::other(msg)` preferred per Clippy (auto-fixed during implementation)
- Panic message from `JoinError` contains "panicked" reliably — assertion uses `msg.contains("panicked")`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Applied cargo fmt to pre-existing formatting violations**
- **Found during:** Task 2 (formatting check verification)
- **Issue:** `cargo fmt --all --check` was failing due to pre-existing violations in backend.rs, error.rs, core operations, and e2e_workflows.rs — these would block the plan's success criteria
- **Fix:** Ran `cargo fmt --all` to apply canonical formatting across all affected files
- **Files modified:** crates/durable-lambda-core/src/{backend.rs, error.rs, operations/{callback,child_context,invoke,map,parallel,wait}.rs}, tests/e2e/tests/e2e_workflows.rs
- **Verification:** `cargo fmt --all --check` exits 0
- **Committed in:** `6f1e9dd` (Task 2 commit)

**2. [Rule 1 - Bug] Fixed Clippy lint: use std::io::Error::other()**
- **Found during:** Task 1 (clippy verification)
- **Issue:** `std::io::Error::new(std::io::ErrorKind::Other, msg)` triggers clippy `std_instead_of_core` / `io_error_other` lint
- **Fix:** Changed to `std::io::Error::other(format!(...))`
- **Files modified:** crates/durable-lambda-core/src/operations/step.rs
- **Verification:** `cargo clippy --workspace -- -D warnings` exits 0
- **Committed in:** `6adae1f` (Task 1 commit)

**3. [Rule 1 - Bug] Added move keyword to 20+ existing step closures**
- **Found during:** Task 1 (build verification after 'static bound change)
- **Issue:** Adding 'static bound to FnOnce() -> Fut caused E0373 errors on all closures that borrow variables without move
- **Fix:** Added `move` keyword to outer closures; pre-cloned variables needed both inside and after the closure (`let x_for_step = x.clone(); let result = ctx.step("n", move || { ... }); use x here`)
- **Files modified:** Examples (8 files × 4 styles), tests/e2e/tests/e2e_workflows.rs, crates/durable-lambda-core/src/operations/map.rs, crates/durable-lambda-testing/src/mock_context.rs
- **Verification:** `cargo build --workspace` exits 0
- **Committed in:** `6adae1f` (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (2 Rule 1 bug fixes, 1 Rule 1 cascading bound fix)
**Impact on plan:** All auto-fixes necessary for compilation, lint compliance, and format compliance. No scope creep.

## Issues Encountered
- Pre-existing formatting violations in core files required `cargo fmt --all` to satisfy the plan's `cargo fmt --all --check` success criterion

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 01 complete: all TEST-01 through TEST-11 error paths tested (10 scenarios, 11 test functions)
- Phase 02 (boundary/replay engine tests) can proceed independently
- Step closure panic safety is now production-grade and consistent with parallel branch panic handling

---
*Phase: 01-error-path-test-coverage*
*Completed: 2026-03-16*
