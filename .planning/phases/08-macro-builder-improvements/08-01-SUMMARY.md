---
phase: 08-macro-builder-improvements
plan: "01"
subsystem: testing
tags: [proc-macro, syn, trybuild, compile-fail, type-validation]

# Dependency graph
requires: []
provides:
  - compile-time type validation for #[durable_execution] second parameter (must be DurableContext)
  - compile-time type validation for #[durable_execution] return type (must be Result<...>)
  - 4 trybuild compile-fail tests covering all macro rejection cases
  - actionable error messages pointing developers to expected signature
affects: [durable-lambda-macro, developer-experience]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "syn type checking: last path segment ident comparison for Type::Path validation"
    - "TDD for proc-macros: unit tests with parse_quote! RED before GREEN implementation"
    - "trybuild TRYBUILD=overwrite for .stderr generation then verify without flag"

key-files:
  created:
    - crates/durable-lambda-macro/tests/ui/fail_wrong_param_type.rs
    - crates/durable-lambda-macro/tests/ui/fail_wrong_param_type.stderr
    - crates/durable-lambda-macro/tests/ui/fail_wrong_return_type.rs
    - crates/durable-lambda-macro/tests/ui/fail_wrong_return_type.stderr
  modified:
    - crates/durable-lambda-macro/src/expand.rs

key-decisions:
  - "Last path segment ident checked for DurableContext and Result — allows fully qualified paths (durable_lambda_core::context::DurableContext) and bare names"
  - "ReturnType::Default (implicit ()) rejected at fn_token span so error points to the fn keyword"
  - "FnArg::Receiver arm handled defensively (if let) even though free functions never have self"
  - "Error message for wrong second param shows full expected signature for immediate copy-paste fix"

patterns-established:
  - "validate_signature() extension point: add checks after param_count, before Ok(())"
  - "syn path segment matching: type_path.path.segments.last().map(|seg| seg.ident == \"TypeName\")"

requirements-completed: [FEAT-29, FEAT-30, FEAT-31]

# Metrics
duration: 2min
completed: 2026-03-17
---

# Phase 8 Plan 01: Macro Type Validation Summary

**compile-time DurableContext and Result type checks added to #[durable_execution] via syn path segment inspection, with 4 trybuild compile-fail tests**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-17T05:37:58Z
- **Completed:** 2026-03-17T05:40:32Z
- **Tasks:** 2
- **Files modified:** 5 (1 modified + 4 created)

## Accomplishments
- Extended `validate_signature()` in expand.rs with second-parameter type check (FEAT-29) and return-type check (FEAT-30)
- Added 4 new unit tests via TDD (RED/GREEN), all 11 unit tests pass
- Created 2 trybuild compile-fail tests with auto-generated `.stderr` files (FEAT-31)
- All 4 trybuild tests pass; full workspace tests clean (0 failures)

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: add failing unit tests** - `0c5432a` (test)
2. **Task 1 GREEN: implement type validation** - `05c0851` (feat)
3. **Task 2: add trybuild compile-fail tests** - `50688aa` (feat)

_Note: TDD task has two commits (test RED then feat GREEN)_

## Files Created/Modified
- `crates/durable-lambda-macro/src/expand.rs` - Extended `validate_signature()` with DurableContext and Result type checks; added FnArg/PatType/ReturnType/Type imports; added 4 new unit tests
- `crates/durable-lambda-macro/tests/ui/fail_wrong_param_type.rs` - Compile-fail test: handler with (i32, i32) params
- `crates/durable-lambda-macro/tests/ui/fail_wrong_param_type.stderr` - Expected error output mentioning DurableContext
- `crates/durable-lambda-macro/tests/ui/fail_wrong_return_type.rs` - Compile-fail test: handler returning String
- `crates/durable-lambda-macro/tests/ui/fail_wrong_return_type.stderr` - Expected error output mentioning Result

## Decisions Made
- Last path segment ident used for type name matching — tolerates both bare `DurableContext` and fully qualified `durable_lambda_core::context::DurableContext`
- `ReturnType::Default` (implicit `()`) rejected at `fn_token` span so the error highlights the `fn` keyword
- FnArg::Receiver arm handled with `if let` defensively, even though free functions never have `self`
- Error message for wrong second param includes full expected signature for immediate developer fix

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All FEAT-29, FEAT-30, FEAT-31 requirements complete
- validate_signature() now enforces full signature correctness at compile time
- 4 compile-fail trybuild tests provide regression coverage for all validation paths
- Ready for any remaining plans in phase 08-macro-builder-improvements

---
*Phase: 08-macro-builder-improvements*
*Completed: 2026-03-17*

## Self-Check: PASSED
- All 5 implementation files exist on disk
- All 3 task commits (0c5432a, 05c0851, 50688aa) verified in git log
