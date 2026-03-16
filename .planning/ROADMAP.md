# ROADMAP.md

## Milestone: v2 — Production Hardening & Developer Experience

## Phases

- [ ] **Phase 1: Error Path Test Coverage** - Tests for failure scenarios, retries, panics, and checkpoint errors
- [x] **Phase 2: Boundary & Replay Engine Tests** - Edge case tests for options, nesting, operation names, and replay robustness (completed 2026-03-16)
- [x] **Phase 3: Shared Context Trait** - Extract DurableContextOps trait to eliminate ~1,800 lines of duplicated delegation code (completed 2026-03-16)
- [x] **Phase 4: Input Validation & Error Codes** - Validate options at construction, add structured error codes to DurableError (completed 2026-03-16)
- [x] **Phase 5: Step Timeout & Conditional Retry** - Per-step time limits and retry predicates for error-aware retry policies (completed 2026-03-16)
- [x] **Phase 6: Observability & Batch Checkpoint** - Tracing spans per operation and batched checkpoint API to reduce AWS calls (completed 2026-03-16)
- [ ] **Phase 7: Saga / Compensation Pattern** - First-class support for distributed transaction rollback with durable compensations
- [ ] **Phase 8: Macro & Builder Improvements** - Type validation in proc-macro, configuration methods on builder pattern
- [ ] **Phase 9: Documentation Overhaul** - Determinism rules, error examples, troubleshooting FAQ, cross-references, metadata

## Phase Details

### Phase 1: Error Path Test Coverage
**Goal**: Every failure scenario in the SDK has an explicit test proving correct error behavior.
**Depends on**: Nothing
**Requirements**: TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06, TEST-07, TEST-08, TEST-09, TEST-10, TEST-11
**Success Criteria** (what must be TRUE):
  1. Replay mismatch between operation types returns DurableError::replay_mismatch with expected/actual info
  2. Serialization type mismatches between closure return and history produce clear DurableError, not panics
  3. Checkpoint write failures (simulated via MockBackend) propagate as DurableError::checkpoint_failed
  4. Step with retries(3) exhausts all 4 attempts then surfaces the final error to the caller
  5. Callback timeout, failure, invoke errors, and all-branch-failure in parallel each return typed errors
  6. Panic in step closure or parallel branch is caught and converted to DurableError, not process abort
**Plans**: 3 plans
Plans:
- [ ] 01-01-PLAN.md — Single-operation error tests (replay mismatch, serialization, checkpoint, retry, callback, invoke)
- [ ] 01-02-PLAN.md — Batch operation error tests (parallel all-fail, map position failures, parallel branch panic)
- [ ] 01-03-PLAN.md — Step closure panic safety fix (production code + test)

### Phase 2: Boundary & Replay Engine Tests
**Goal**: Edge cases and boundary conditions for all options, nesting depths, and replay engine semantics have explicit tests.
**Depends on**: Phase 1
**Requirements**: TEST-12, TEST-13, TEST-14, TEST-15, TEST-16, TEST-17, TEST-18, TEST-19, TEST-20, TEST-21, TEST-22
**Success Criteria** (what must be TRUE):
  1. Zero-duration wait, zero/negative batch_size, and zero branches have defined, tested behavior
  2. Operation names with empty strings, unicode, and 255+ characters work or fail with clear errors
  3. 5-level nested child contexts and 3-level nested parallel-in-child-in-parallel produce correct operation IDs
  4. Same history replayed 100 times produces bit-identical results every time
  5. Duplicate and missing operation IDs in history have defined behavior with tests
**Plans**: 3 plans
Plans:
- [ ] 02-01-PLAN.md — Option boundary tests (zero wait, batch_size edges, parallel 0/1, name edge cases, validation panics)
- [ ] 02-02-PLAN.md — Deep nesting tests (5-level child contexts, parallel-in-child-in-parallel)
- [ ] 02-03-PLAN.md — Replay engine robustness (deterministic replay 100x, duplicate IDs, history gaps, token evolution)

### Phase 3: Shared Context Trait
**Goal**: A single DurableContextOps trait defines all context methods, implemented by all wrapper contexts, enabling generic handler code.
**Depends on**: Nothing
**Requirements**: ARCH-01, ARCH-02, ARCH-03, ARCH-04, ARCH-05, ARCH-06
**Success Criteria** (what must be TRUE):
  1. `DurableContextOps` trait exists in durable-lambda-core with all 21 methods
  2. ClosureContext, TraitContext, and BuilderContext each implement DurableContextOps
  3. A generic function `async fn logic<C: DurableContextOps>(ctx: &mut C)` compiles and runs with all 4 context types
  4. Handler boilerplate (AWS config, event extraction, context creation) lives in one shared function, not duplicated 4 times
  5. All existing tests pass without modification
  6. Parity tests verify generic handler produces identical results across all approaches
**Plans**: 3 plans
Plans:
- [x] 03-01-PLAN.md — Define DurableContextOps trait and implement for all 4 context types (completed 2026-03-16)
- [ ] 03-02-PLAN.md — Extract handler boilerplate into shared parse_invocation() function
- [ ] 03-03-PLAN.md — Generic handler parity tests across all context types

### Phase 4: Input Validation & Error Codes
**Goal**: Invalid configuration is caught at construction time, and all DurableError variants have stable programmatic codes.
**Depends on**: Nothing
**Requirements**: FEAT-01, FEAT-02, FEAT-03, FEAT-04, FEAT-05, FEAT-06, FEAT-07, FEAT-08
**Success Criteria** (what must be TRUE):
  1. `StepOptions::new().retries(-1)` panics with descriptive message mentioning the invalid value
  2. `CallbackOptions::new().timeout_seconds(0)` panics with descriptive message
  3. `MapOptions::new().batch_size(0)` panics with descriptive message
  4. `DurableError::replay_mismatch(...).code()` returns `"REPLAY_MISMATCH"` (and similarly for all variants)
  5. Backend retry detection in backend.rs uses error codes instead of string matching on error messages
  6. Checkpoint response with None checkpoint_token returns DurableError instead of panicking
**Plans**: 3 plans
Plans:
- [ ] 04-01-PLAN.md — Input validation guards for StepOptions, CallbackOptions, MapOptions
- [ ] 04-02-PLAN.md — Error codes (.code() method) and structured retry detection refactor
- [ ] 04-03-PLAN.md — Defensive checkpoint token handling across all operation files

### Phase 5: Step Timeout & Conditional Retry
**Goal**: Steps can be time-bounded and retries can be filtered by error type, preventing wasted compute on non-transient failures.
**Depends on**: Phase 4 (needs validated options)
**Requirements**: FEAT-09, FEAT-10, FEAT-11, FEAT-12, FEAT-13, FEAT-14, FEAT-15, FEAT-16, TEST-23, TEST-24, TEST-25
**Success Criteria** (what must be TRUE):
  1. `StepOptions::new().timeout_seconds(5)` causes step to return DurableError::step_timeout after 5 seconds
  2. Step completing within timeout returns normal result
  3. `StepOptions::new().retry_if(|e| e.is_transient())` retries only when predicate returns true
  4. Non-matching errors fail immediately without consuming retry budget
  5. Default behavior (no retry_if) retries all errors, preserving backward compatibility
  6. Cross-approach parity tests verify step timeout and conditional retry work identically across all 4 styles
**Plans**: 3 plans
Plans:
- [ ] 05-01-PLAN.md — StepOptions extensions (timeout_seconds, retry_if) + DurableError::StepTimeout + step_with_options integration
- [ ] 05-02-PLAN.md — E2E tests for step timeout and conditional retry (FEAT-12, FEAT-16)
- [ ] 05-03-PLAN.md — Cross-approach parity tests (TEST-23, TEST-24, TEST-25)

### Phase 6: Observability & Batch Checkpoint
**Goal**: Every operation emits structured tracing spans, and sequential steps can batch checkpoint writes to halve AWS API calls.
**Depends on**: Phase 3 (trait provides single instrumentation point)
**Requirements**: FEAT-17, FEAT-18, FEAT-19, FEAT-20, FEAT-21, FEAT-22, FEAT-23, FEAT-24
**Success Criteria** (what must be TRUE):
  1. Each step/wait/callback/invoke/parallel/map/child_context emits a tracing span with name, type, and operation ID
  2. Nested operations produce parent-child span hierarchy matching context nesting
  3. batch_checkpoint() accepts multiple OperationUpdate items and makes a single AWS API call
  4. Sequential 5-step workflow in batch mode produces fewer checkpoint calls than individual mode
  5. Individual checkpoint mode still works (batch is opt-in, not mandatory)
**Plans**: 2 plans
Plans:
- [ ] 06-01-PLAN.md — Tracing spans for all 7 operation types + span emission tests (FEAT-17, FEAT-18, FEAT-19, FEAT-20)
- [ ] 06-02-PLAN.md — Batch checkpoint API, DurableContext batch mode, and checkpoint reduction tests (FEAT-21, FEAT-22, FEAT-23, FEAT-24)

### Phase 7: Saga / Compensation Pattern
**Goal**: Users can register compensation closures that execute in reverse order when a workflow fails, with durable checkpointing of the rollback itself.
**Depends on**: Phase 1 (needs error path tests as foundation)
**Requirements**: FEAT-25, FEAT-26, FEAT-27, FEAT-28
**Success Criteria** (what must be TRUE):
  1. `ctx.step_with_compensation("charge", forward_fn, compensate_fn)` registers compensation and executes forward
  2. On workflow failure, all registered compensations execute in reverse registration order
  3. Compensation execution is checkpointed — replay of compensation produces identical rollback
  4. Compensation failure is captured as DurableError, not swallowed
  5. Partial rollback (compensate 3 of 5) resumes from checkpoint on re-invocation
**Plans**: TBD

### Phase 8: Macro & Builder Improvements
**Goal**: The proc-macro validates parameter and return types at compile time, and the builder pattern supports pre-run configuration.
**Depends on**: Phase 3 (trait enables builder configuration)
**Requirements**: FEAT-29, FEAT-30, FEAT-31, FEAT-32, FEAT-33, FEAT-34
**Success Criteria** (what must be TRUE):
  1. `#[durable_execution] async fn handler(x: i32, y: i32)` produces a compile error mentioning DurableContext
  2. `#[durable_execution] async fn handler(e: Value, c: DurableContext) -> String` produces a compile error mentioning Result
  3. trybuild compile-fail tests verify both wrong-type and wrong-return-type errors
  4. `handler(fn).with_tracing(subscriber).run()` configures custom tracing before execution
  5. `handler(fn).with_error_handler(fn).run()` routes errors through custom handler
**Plans**: TBD

### Phase 9: Documentation Overhaul
**Goal**: README, migration guide, and inline docs cover determinism rules, error handling patterns, and troubleshooting with no gaps.
**Depends on**: Phase 5, Phase 7 (needs new features to document)
**Requirements**: DOCS-01, DOCS-02, DOCS-03, DOCS-04, DOCS-05, DOCS-06, DOCS-07, DOCS-08, DOCS-09, DOCS-10
**Success Criteria** (what must be TRUE):
  1. README has "Determinism Rules" section with explicit do/don't examples (Uuid, Utc::now outside steps)
  2. README has error handling example showing `Ok(Ok(v))`, `Ok(Err(e))`, `Err(durable_err)` matching
  3. README has troubleshooting FAQ covering Send+Static, Serialize bounds, and type annotation errors
  4. README links to project-context.md in a "Contributing" section
  5. Migration guide has determinism anti-patterns section
  6. All Cargo.toml files have description, keywords, and categories fields
  7. CLAUDE.md documents the wrapper crate duplication pattern (or its elimination if Phase 3 completed)
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Error Path Test Coverage | 0/3 | Planned | - |
| 2. Boundary & Replay Engine Tests | 3/3 | Complete   | 2026-03-16 |
| 3. Shared Context Trait | 3/3 | Complete   | 2026-03-16 |
| 4. Input Validation & Error Codes | 3/3 | Complete   | 2026-03-16 |
| 5. Step Timeout & Conditional Retry | 3/3 | Complete   | 2026-03-16 |
| 6. Observability & Batch Checkpoint | 2/2 | Complete   | 2026-03-16 |
| 7. Saga / Compensation Pattern | 0/TBD | Not started | - |
| 8. Macro & Builder Improvements | 0/TBD | Not started | - |
| 9. Documentation Overhaul | 0/TBD | Not started | - |
