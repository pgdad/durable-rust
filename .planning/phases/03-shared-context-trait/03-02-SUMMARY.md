---
phase: 03-shared-context-trait
plan: 02
subsystem: api
tags: [rust, event-parsing, refactoring, deduplication, lambda]

# Dependency graph
requires:
  - phase: 03-shared-context-trait/03-01
    provides: DurableContextOps trait for all wrapper contexts
provides:
  - InvocationData struct in durable-lambda-core/src/event.rs
  - parse_invocation() function extracting all 5 fields from Lambda event envelope
  - All 4 handler locations (closure, trait, builder, macro) use shared parse_invocation()
affects:
  - 03-shared-context-trait/03-03
  - any future handler implementations

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "parse_invocation() as single extraction point for Lambda event envelope fields"
    - "InvocationData struct as value object carrying ARN, token, operations, marker, user_event"

key-files:
  created: []
  modified:
    - crates/durable-lambda-core/src/event.rs
    - crates/durable-lambda-closure/src/handler.rs
    - crates/durable-lambda-trait/src/handler.rs
    - crates/durable-lambda-builder/src/handler.rs
    - crates/durable-lambda-macro/src/expand.rs

key-decisions:
  - "Added #[derive(Debug)] to InvocationData to satisfy Result::unwrap_err() Debug bound in tests"
  - "Used Box::<dyn Error + Send + Sync>::from (function pointer) not closure per clippy redundant_closure"
  - "Macro expand.rs still uses fully-qualified ::durable_lambda_core::event::parse_invocation() paths"

patterns-established:
  - "Event extraction shared via parse_invocation(): all 4 handler crates delegate to core"
  - "InvocationData as structured return type for event envelope parsing"

requirements-completed: [ARCH-06]

# Metrics
duration: 7min
completed: 2026-03-16
---

# Phase 03 Plan 02: Handler Boilerplate Extraction Summary

**InvocationData struct + parse_invocation() in core/event.rs eliminate ~80 lines of duplicated Lambda event extraction across all 4 handler implementations (closure, trait, builder, macro)**

## Performance

- **Duration:** 7 min
- **Started:** 2026-03-16T14:04:09Z
- **Completed:** 2026-03-16T14:11:00Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Added `InvocationData` struct with all 5 fields (ARN, token, operations, marker, user_event) and `parse_invocation()` in `durable-lambda-core/src/event.rs`
- Added 5 targeted tests for `parse_invocation()` covering valid payload, missing fields, and marker variants
- Refactored all 4 handler locations (closure, trait, builder, macro) to call `parse_invocation()` instead of inline extraction
- Reduced ~80 lines of duplicated extraction blocks to a single shared function call

## Task Commits

Each task was committed atomically:

1. **Task 1: Add InvocationData struct and parse_invocation() to event.rs** - `1cd8b6b` (feat)
2. **Task 2: Refactor all 4 handler locations to use parse_invocation()** - `83edda4` (refactor)

## Files Created/Modified
- `crates/durable-lambda-core/src/event.rs` - Added InvocationData struct, parse_invocation(), 5 new tests
- `crates/durable-lambda-closure/src/handler.rs` - Replaced extraction block with parse_invocation()
- `crates/durable-lambda-trait/src/handler.rs` - Replaced extraction block with parse_invocation()
- `crates/durable-lambda-builder/src/handler.rs` - Replaced extraction block with parse_invocation()
- `crates/durable-lambda-macro/src/expand.rs` - Generated code uses ::durable_lambda_core::event::parse_invocation(); updated test assertions

## Decisions Made
- Added `#[derive(Debug)]` to `InvocationData` because `Result::unwrap_err()` requires `Debug` on the success type; `aws_sdk_lambda::types::Operation` already implements `Debug`
- Used `Box::<dyn Error + Send + Sync>::from` (function pointer) in `.map_err()` rather than closure form — clippy `redundant_closure` lint requires this pattern
- Macro-generated code retains fully-qualified `::durable_lambda_core::event::parse_invocation()` path because proc-macro output compiles in the user's crate namespace

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added #[derive(Debug)] to InvocationData**
- **Found during:** Task 1 (add InvocationData and parse_invocation)
- **Issue:** `Result<InvocationData, &'static str>::unwrap_err()` requires `Debug` on `T`; test assertions for missing-field error cases failed to compile
- **Fix:** Added `#[derive(Debug)]` to `InvocationData` struct
- **Files modified:** crates/durable-lambda-core/src/event.rs
- **Verification:** `cargo test -p durable-lambda-core event` passes
- **Committed in:** 1cd8b6b (Task 1 commit)

**2. [Rule 1 - Bug] Fixed clippy redundant_closure in .map_err() calls**
- **Found during:** Task 2 verification (cargo clippy --workspace -- -D warnings)
- **Issue:** `.map_err(|e| Box::<dyn Error + Send + Sync>::from(e))` is a redundant closure; clippy -D warnings treats this as an error
- **Fix:** Changed to `.map_err(Box::<dyn Error + Send + Sync>::from)` in all 3 wrapper handler files
- **Files modified:** closure/handler.rs, trait/handler.rs, builder/handler.rs
- **Verification:** `cargo clippy --workspace -- -D warnings` passes with no errors
- **Committed in:** 83edda4 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2x Rule 1 - Bug)
**Impact on plan:** Both fixes required for correctness and CI compliance. No scope creep.

## Issues Encountered
- Pre-existing `cargo fmt --all --check` failures in unrelated files (backend.rs, error.rs, operations/*.rs) throughout the workspace — logged to deferred-items.md, not fixed per out-of-scope rule. All 5 files modified in this plan pass `fmt --check`.

## Next Phase Readiness
- InvocationData and parse_invocation() are in place; ready for 03-03 plan
- No blockers identified

---
*Phase: 03-shared-context-trait*
*Completed: 2026-03-16*

## Self-Check: PASSED

- FOUND: crates/durable-lambda-core/src/event.rs (InvocationData at line 167, parse_invocation at line 213)
- FOUND: crates/durable-lambda-closure/src/handler.rs
- FOUND: crates/durable-lambda-trait/src/handler.rs
- FOUND: crates/durable-lambda-builder/src/handler.rs
- FOUND: crates/durable-lambda-macro/src/expand.rs
- FOUND commit: 1cd8b6b (feat: InvocationData and parse_invocation)
- FOUND commit: 83edda4 (refactor: all 4 handler locations)
