---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: unknown
last_updated: "2026-03-17T05:41:42.036Z"
progress:
  total_phases: 9
  completed_phases: 8
  total_plans: 21
  completed_plans: 21
---

# STATE.md

## Project Reference
**File**: PROJECT.md
**Core Value**: Enable Rust durable Lambda handlers with 4-8x lower memory and zero behavioral divergence from Python SDK
**Current Focus**: v2 milestone — production hardening, test coverage, developer experience

## Current Position
- **Phase**: 08-macro-builder-improvements
- **Plan**: 08-02 complete
- **Status**: In Progress
- **Last Activity**: 2026-03-17 — Completed 08-02 DurableHandlerBuilder with_tracing and with_error_handler
- **Progress**: [██████████] 95% 20/21 plans complete

## Performance Metrics
- **Total Plans**: TBD (phases not yet planned into individual plans)
- **Phases**: 9 total
- **Requirements**: 69 v1, 7 v2 deferred

## Accumulated Context

### Recent Decisions
- [01-01] OperationStatus::Cancelled used for replay mismatch test — it's a completed status (handled by check_result) but extract_step_result returns ReplayMismatch for it since it's neither Succeeded nor Failed
- [01-01] PassingMockBackend and FailingMockBackend patterns established for error-path tests — tests pre-populate error state in Operation history rather than using MockDurableContext builder
- [01-01] Direct DurableContext::new() construction used in error-path tests when specific non-standard statuses (TimedOut, Cancelled, Pending with attempt count) are needed
- Comprehensive codebase audit identified ~1,800 lines of duplicated delegation code across 3 wrapper crates
- Error path testing is the highest priority — almost no failure scenarios are currently tested
- Phases 1-2 (testing) and Phases 3-4 (architecture/validation) can proceed in parallel
- Phase 9 (docs) depends on feature phases completing first
- StepOptions::retries changed from u32 to i32 so negative values can be rejected at runtime with clear panic messages (integer literals coerce automatically, no caller changes needed)
- Builder validation uses assert! with format 'Type::method: constraint, got {value}' for consistent error messages
- CallbackOptions uses strictly positive (>0) guards; StepOptions uses non-negative (>=0) guards since zero retries/backoff are valid
- [04-02] No wildcard arm in DurableError::code() match — compiler enforces exhaustive coverage when new variants are added
- [04-02] Only AwsSdkOperation and AwsSdk variants qualify as retryable; CheckpointFailed is never retried even if its source message contains transient-sounding keywords
- [04-03] All 13 silent checkpoint_token if-let-Some sites replaced with ok_or_else error propagation — None token is an AWS API contract violation, not a normal case requiring silent skip
- [04-03] Used std::io::ErrorKind::InvalidData as source error kind for checkpoint_failed when token is missing — fits "unexpected data format from API" semantic
- [03-02] parse_invocation() as single extraction point for Lambda event envelope — all 4 handler crates (closure, trait, builder, macro) delegate to core/event.rs; InvocationData carries ARN, token, operations, marker, user_event
- [03-01] Used native RPITIT async fn in traits (Rust 1.75+) instead of async_trait macro for DurableContextOps — enables static dispatch without boxing overhead
- [03-01] P: Sync bound added to invoke trait method to satisfy Send on returned Future; inherent method only requires P: Serialize
- [03-01] DurableContextOps defined in ops_trait module (not context module) to keep context.rs focused on the core struct
- [03-03] Capture execution_mode() at function entry before step calls — replay engine transitions to Executing after consuming history, so post-step mode check shows Executing even in replay scenarios
- [03-03] Use assert_ops::<T>() compile-time pattern instead of test-only pub constructors on wrapper contexts — no test surface added to library API
- [01-02] Panic test (TEST-11) uses #[allow(unreachable_code)] after panic! macro to satisfy type inference for the Ok arm in the branch closure
- [01-02] Map closure parameter order is item-first (|item: I, ctx: DurableContext|) matching map() signature FnOnce(I, DurableContext) — distinct from parallel's FnOnce(DurableContext)
- [01-03] Use DurableError::checkpoint_failed for step closure panics — panics are a form of failed checkpoint, not a new error category
- [01-03] Step closure bounds now require 'static (same as parallel branches) — closures already move owned data per CLAUDE.md, practically always satisfied
- [01-03] Panic message from JoinError Display contains "panicked" reliably — used as assertion keyword in test_step_closure_panic_returns_error
- [02-01] DurableError::WaitSuspended is a struct variant requiring { .. } pattern match, not a unit variant
- [02-01] BatchResult results must be sorted by index before value assertions — concurrent execution may reorder them
- [02-01] Zero-branch parallel produces exactly 2 checkpoints (outer START + SUCCEED) with empty BatchResult
- [02-02] filter_map(|item| item.result) used for Option<i32> aggregation in parallel results — Copy type, no .copied() needed
- [02-02] values.sort() required before assert_eq! in test_parallel_in_child_in_parallel — tokio::spawn execution order is non-deterministic
- [02-03] History gap test uses only 2 steps — after step2 executes (gap), engine transitions to Executing mode; step3 would not replay from pre-loaded history; documented as defined behavior
- [02-03] CheckpointCall struct has no client_token field in mock_backend.rs — plan description was inaccurate; test written against the actual struct
- [05-01] Used RetryPredicate type alias for Arc<dyn Fn(&dyn Any) -> bool + Send + Sync> to satisfy clippy::type_complexity
- [05-01] retry_if predicate returning false causes immediate FAIL without consuming retry budget (FEAT-14 — predicate checked before retry budget)
- [05-01] No retry_if predicate defaults to retrying all errors (backward compatible behavior preserved)
- [05-01] StepTimeout uses tokio::time::timeout on &mut JoinHandle with handle.abort() on expiry; no checkpoint sent on timeout
- [05-02] All 7 e2e tests written in one pass — Tasks 1 and 2 share commit abe339f; implementation was complete from Plan 01
- [05-02] #[non_exhaustive] struct variants require { field, .. } in external crate pattern matches — fixed at compile time
- [05-03] Used tokio::time::pause() + advance() for step timeout parity test — proves timeout fires without real sleep delay, keeping test suite fast
- [05-03] BranchFn type alias required in parallel parity tests — inline Box::new closures without type alias cannot coerce Box::pin return to Pin<Box<dyn Future + Send>>
- [06-01] span.enter() guard pattern used (not .instrument()) because &mut self async methods can't move body into async block for .instrument()
- [06-01] tracing::trace!("durable_operation") required inside each span for tracing_test logs_contain() detection — events needed, not just span creation
- [06-01] Span tests use per-file MockBackend instead of MockDurableContext — avoids type conflicts when durable_lambda_testing used as dev-dep of durable_lambda_core
- [06-02] Child contexts always start with batch_mode=false — children must not inherit parent batch mode to avoid update loss on independent flush
- [06-02] RETRY in batch mode auto-flushes before returning WaitSuspended — suspension requires checkpoint persisted before Lambda exits; deferred RETRY would lose the update
- [06-02] batch_checkpoint() default method delegates to checkpoint() — RealBackend inherits automatically, MockBackend overrides for distinct tracking
- [06-02] MockBackend::new() return type preserved as 3-tuple; batch_call_counter() accessor added — avoids breaking all existing test callers
- [07-01] CompensationRecord is not Debug — contains a closure (CompensateFn); child contexts start with empty compensations (not inherited)
- [07-01] run_compensations uses continue-on-error semantics — all compensations attempt even when one fails; per-item status captured in CompensationResult
- [07-01] Partial rollback resume via replay engine op_id tracking — completed compensations skip closure execution on re-invocation
- [07-01] CompensateFn wraps typed G: FnOnce(T) -> GFut with JSON serialization/deserialization for type erasure; sub_type="Compensation" for checkpoint protocol
- [07-02] Wrapper crate trait impl delegation uses self.inner.method() pattern consistent with existing wrappers — not DurableContext:: prefix
- [07-02] Partial rollback e2e test uses OperationIdGenerator directly to compute compensation op IDs (steps consume 1-3, compensations 4+ in LIFO) for pre-loading history
- [08-02] with_tracing stores subscriber as Option<Box<dyn tracing::Subscriber + Send + Sync>> for type erasure; run() installs via set_global_default before Lambda runtime initialization
- [08-02] error_handler stored as Option<Box<dyn Fn(DurableError) -> DurableError + Send + Sync>>; applied after handler.await before Box<dyn Error> conversion — preserves DurableError type for transformation

### Pending Todos
- None — ready to begin Phase 1 execution

### Blockers
- None identified

## Session Continuity
- **Last Session**: 2026-03-17 — Completed 08-02 DurableHandlerBuilder with_tracing() and with_error_handler() (TDD, 14 tests pass)
- **Stopped At**: Completed 08-macro-builder-improvements/08-02-PLAN.md
- **Next Action**: Continue Phase 08 remaining plans
