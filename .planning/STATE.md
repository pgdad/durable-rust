# STATE.md

## Project Reference
**File**: PROJECT.md
**Core Value**: Enable Rust durable Lambda handlers with 4-8x lower memory and zero behavioral divergence from Python SDK
**Current Focus**: v2 milestone — production hardening, test coverage, developer experience

## Current Position
- **Phase**: 04-input-validation-error-codes
- **Plan**: 04-03 complete, next plan TBD
- **Status**: Executing
- **Last Activity**: 2026-03-16 — Completed 04-03 defensive checkpoint_token error propagation across all 13 sites
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
- [04-03] All 13 silent checkpoint_token if-let-Some sites replaced with ok_or_else error propagation — None token is an AWS API contract violation, not a normal case requiring silent skip
- [04-03] Used std::io::ErrorKind::InvalidData as source error kind for checkpoint_failed when token is missing — fits "unexpected data format from API" semantic
- [03-02] parse_invocation() as single extraction point for Lambda event envelope — all 4 handler crates (closure, trait, builder, macro) delegate to core/event.rs; InvocationData carries ARN, token, operations, marker, user_event
- [03-01] Used native RPITIT async fn in traits (Rust 1.75+) instead of async_trait macro for DurableContextOps — enables static dispatch without boxing overhead
- [03-01] P: Sync bound added to invoke trait method to satisfy Send on returned Future; inherent method only requires P: Serialize
- [03-01] DurableContextOps defined in ops_trait module (not context module) to keep context.rs focused on the core struct

### Pending Todos
- None — ready to begin Phase 1 execution

### Blockers
- None identified

## Session Continuity
- **Last Session**: 2026-03-16 — Completed 03-02 handler boilerplate extraction into parse_invocation()
- **Stopped At**: Completed 03-shared-context-trait/03-02-PLAN.md
- **Next Action**: Execute 03-03 plan (next in phase 03)
