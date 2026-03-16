---
phase: 06-observability-batch-checkpoint
plan: 01
subsystem: observability
tags: [tracing, spans, tracing-test, opentelemetry, diagnostics]

# Dependency graph
requires:
  - phase: 05-step-timeout-conditional-retry
    provides: All 7 durable operation implementations (step, wait, callback, invoke, parallel, map, child_context)
provides:
  - tracing::info_span! on all 7 durable operations with op.name, op.type, op.id fields
  - 9 span verification tests using #[traced_test] + logs_contain()
  - child_context hierarchy test proving nested spans work
  - op.id test proving 64-char hex operation IDs are captured
affects: [06-02, future-observability, any-phase-adding-operations]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "info_span! + span.enter() guard at top of &mut self async methods (not .instrument() which requires moving body)"
    - "tracing::trace!(\"durable_operation\") event immediately after span.enter() — required for tracing_test logs_contain() detection"
    - "#[allow(clippy::await_holding_lock)] on async methods holding span guard across .await"
    - "Local MockBackend pattern for span tests — avoids MockDurableContext type conflicts in intra-crate tests"

key-files:
  created: []
  modified:
    - crates/durable-lambda-core/src/operations/step.rs
    - crates/durable-lambda-core/src/operations/wait.rs
    - crates/durable-lambda-core/src/operations/callback.rs
    - crates/durable-lambda-core/src/operations/invoke.rs
    - crates/durable-lambda-core/src/operations/parallel.rs
    - crates/durable-lambda-core/src/operations/map.rs
    - crates/durable-lambda-core/src/operations/child_context.rs

key-decisions:
  - "Used span.enter() guard pattern (not .instrument()) because &mut self methods cannot move body into async block"
  - "Added tracing::trace!(\"durable_operation\") inside each span — tracing_test logs_contain() only detects events, not span entry/exit without an event inside the span"
  - "Used existing per-file MockBackend structs for span tests instead of MockDurableContext — avoids circular type conflicts when durable_lambda_testing is used as dev-dependency of durable_lambda_core itself"
  - "op.type = 'child_context' used as span field name for child_context operation — dot notation is valid in tracing field names"

patterns-established:
  - "Span test pattern: use per-file MockBackend, #[traced_test], call operation, assert logs_contain(\"durable_operation\") + logs_contain(op_name) + logs_contain(op_type)"
  - "New operations added to durable_lambda_core must follow the info_span! + trace! + _guard pattern established here"

requirements-completed: [FEAT-17, FEAT-18, FEAT-19, FEAT-20]

# Metrics
duration: 30min
completed: 2026-03-16
---

# Phase 06 Plan 01: Tracing Spans for All 7 Durable Operations Summary

**tracing::info_span! with op.name/op.type/op.id added to all 7 durable operations, verified by 9 #[traced_test] tests including child_context hierarchy proof**

## Performance

- **Duration:** 30 min
- **Started:** 2026-03-16T18:36:00Z
- **Completed:** 2026-03-16T19:06:57Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- All 7 operation methods (step_with_options, wait, create_callback, invoke, parallel, map, child_context) now emit tracing::info_span! with op.name, op.type, and op.id fields
- 9 span tests added across 7 files, all passing: one per operation type (7), one hierarchy test (FEAT-18), one op_id test (FEAT-19)
- child_context_span_hierarchy test proves nested spans appear in logs when a step runs inside a child_context
- Full workspace test suite (157+ tests) passes with zero regressions
- cargo clippy --workspace -- -D warnings and cargo fmt pass clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Add tracing spans to all 7 operation methods** - `454febd` (feat)
2. **Task 2: Add span tests for all operation types** - `1159ddf` (test)

## Files Created/Modified
- `crates/durable-lambda-core/src/operations/step.rs` - info_span with op.type='step', 2 span tests
- `crates/durable-lambda-core/src/operations/wait.rs` - info_span with op.type='wait', 1 span test
- `crates/durable-lambda-core/src/operations/callback.rs` - info_span with op.type='callback', 1 span test
- `crates/durable-lambda-core/src/operations/invoke.rs` - info_span with op.type='invoke', 1 span test
- `crates/durable-lambda-core/src/operations/parallel.rs` - info_span with op.type='parallel', 1 span test
- `crates/durable-lambda-core/src/operations/map.rs` - info_span with op.type='map', 1 span test
- `crates/durable-lambda-core/src/operations/child_context.rs` - info_span with op.type='child_context', 2 span tests (emits + hierarchy)

## Decisions Made
- Used `span.enter()` guard pattern instead of `.instrument()` because all 7 methods are `&mut self` async — moving the method body into an `async move` block for `.instrument()` would conflict with the mutable borrow.
- Added `tracing::trace!("durable_operation")` immediately after `span.enter()` — this was required because `tracing_test`'s `logs_contain()` only detects events (tracing::info!/debug!/trace!), not span entry/exit. Without an event inside the span, the span fields are never formatted into the captured output.
- Used each file's existing local MockBackend for span tests instead of `MockDurableContext` from `durable_lambda_testing` — when `durable_lambda_testing` is used as a dev-dependency inside `durable_lambda_core`, Rust compiles it against a separate instance of `durable_lambda_core`, causing type conflicts for types like `ParallelOptions`, `CallbackOptions`, etc.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] tracing::trace! event required for logs_contain() detection**
- **Found during:** Task 2 (span tests)
- **Issue:** `tracing::info_span!` + `span.enter()` alone does not emit any log event; `tracing_test`'s `logs_contain()` only detects tracing events (info!/debug!/trace!), not span entry/exit boundaries. All 9 span tests failed with "assertion failed: logs_contain("durable_operation")".
- **Fix:** Added `tracing::trace!("durable_operation")` immediately after `span.enter()` in all 7 operations. This event fires within the span context, so the formatted output includes span fields like `durable_operation{op.name="validate" op.type="step" op.id="..."}`.
- **Files modified:** All 7 operation files
- **Verification:** All 9 span tests pass
- **Committed in:** `454febd` (Task 1) and `1159ddf` (Task 2)

**2. [Rule 3 - Blocking] MockDurableContext type conflict for intra-crate tests**
- **Found during:** Task 2 (initial span test attempt)
- **Issue:** Using `MockDurableContext` from `durable_lambda_testing` inside `durable_lambda_core`'s own test modules caused type mismatches — Rust compiled two separate instances of `durable_lambda_core` types (one local, one from the testing crate's dependency), resulting in "mismatched types" errors for `ParallelOptions`, `CallbackOptions`, `MapOptions`, and `DurableContext`.
- **Fix:** Replaced `MockDurableContext::new().build().await` with each file's existing local MockBackend pattern (e.g., `ParallelMockBackend::new()`, `ChildContextMockBackend::new()`), keeping the same test logic.
- **Files modified:** All 7 operation test modules
- **Verification:** Compilation succeeds, all tests pass

---

**Total deviations:** 2 auto-fixed (1 bug in span detection mechanism, 1 blocking type conflict)
**Impact on plan:** Both auto-fixes essential for tests to compile and pass. Span instrumentation in library code is unchanged.

## Issues Encountered
- `#[allow(clippy::await_holding_lock)]` was placed on functions per plan guidance but the actual clippy lint for span guards is `clippy::await_holding_span_guard` (not `await_holding_lock`). Neither lint fires in practice, so the allow attributes are harmless precautions.

## Next Phase Readiness
- All 7 durable operations now emit structured tracing spans — ready for Phase 06 Plans 02+ (batch checkpoint, additional observability features)
- The span pattern (info_span! + trace! + _guard) is documented as a project convention for any new operations

---
*Phase: 06-observability-batch-checkpoint*
*Completed: 2026-03-16*
