# STATE.md

## Project Reference
**File**: PROJECT.md
**Core Value**: Enable Rust durable Lambda handlers with 4-8x lower memory and zero behavioral divergence from Python SDK
**Current Focus**: v2 milestone — production hardening, test coverage, developer experience

## Current Position
- **Phase**: 04-input-validation-error-codes
- **Plan**: 04-02 complete, 04-03 next
- **Status**: Executing
- **Last Activity**: 2026-03-16 — Completed 04-02 error codes and structured retry detection
- **Progress**: ░░░░░░░░░░ 0/9 phases (Phase 4 in progress)

## Performance Metrics
- **Total Plans**: TBD (phases not yet planned into individual plans)
- **Phases**: 9 total
- **Requirements**: 69 v1, 7 v2 deferred

## Accumulated Context

### Recent Decisions
- Comprehensive codebase audit identified ~1,800 lines of duplicated delegation code across 3 wrapper crates
- Error path testing is the highest priority — almost no failure scenarios are currently tested
- Phases 1-2 (testing) and Phases 3-4 (architecture/validation) can proceed in parallel
- Phase 9 (docs) depends on feature phases completing first
- StepOptions::retries changed from u32 to i32 so negative values can be rejected at runtime with clear panic messages (integer literals coerce automatically, no caller changes needed)
- Builder validation uses assert! with format 'Type::method: constraint, got {value}' for consistent error messages
- CallbackOptions uses strictly positive (>0) guards; StepOptions uses non-negative (>=0) guards since zero retries/backoff are valid
- [04-02] No wildcard arm in DurableError::code() match — compiler enforces exhaustive coverage when new variants are added
- [04-02] Only AwsSdkOperation and AwsSdk variants qualify as retryable; CheckpointFailed is never retried even if its source message contains transient-sounding keywords

### Pending Todos
- None — ready to begin Phase 1 execution

### Blockers
- None identified

## Session Continuity
- **Last Session**: 2026-03-16 — Completed 04-02 error codes and structured retry detection
- **Stopped At**: Completed 04-input-validation-error-codes/04-02-PLAN.md
- **Next Action**: Execute 04-03 plan (next in phase 04)
