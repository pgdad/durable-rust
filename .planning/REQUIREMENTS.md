# REQUIREMENTS.md

## v1 Requirements (this milestone)

### Testing — Error Paths & Failure Scenarios
- [x] **TEST-01**: Replay mismatch detection — step expects type A but history has type B returns DurableError::replay_mismatch
- [x] **TEST-02**: Serialization failure — step result type mismatch between closure return and history data
- [x] **TEST-03**: Checkpoint failure — AWS SDK error during checkpoint write (network timeout, invalid token)
- [x] **TEST-04**: Retry exhaustion — step with retries(3) fails all 4 attempts and returns final error
- [x] **TEST-05**: Callback timeout expiration — callback exceeds timeout_seconds and returns error
- [x] **TEST-06**: Callback explicit failure signal — callback receives failure from external system
- [x] **TEST-07**: Invoke error — target Lambda returns error payload
- [x] **TEST-08**: Parallel all-branches-fail — all parallel branches return errors
- [x] **TEST-09**: Map item failures at different positions — first, middle, last item failures
- [x] **TEST-10**: Step closure panic — panic in user closure does not crash context
- [x] **TEST-11**: Parallel branch panic — panic in one branch doesn't affect others

### Testing — Boundary Conditions
- [x] **TEST-12**: Zero-duration wait — `wait("name", 0)` behavior
- [x] **TEST-13**: Map with batch_size edge cases — 0, 1, greater than collection size
- [x] **TEST-14**: Parallel with 0 branches and 1 branch
- [x] **TEST-15**: Operation names — empty string, unicode characters, 255+ characters
- [x] **TEST-16**: Negative option values — retries(-1), backoff_seconds(-1), timeout_seconds(0)
- [x] **TEST-17**: Deeply nested child contexts — 5+ levels
- [x] **TEST-18**: Nested parallel inside child context inside parallel (3-level nesting)

### Testing — Replay Engine Robustness
- [x] **TEST-19**: Deterministic replay — same history produces identical results across 100 runs
- [x] **TEST-20**: Duplicate operation IDs in history — behavior is defined
- [x] **TEST-21**: History gap — missing operation IDs between existing ones
- [x] **TEST-22**: Checkpoint token evolution — token changes after each checkpoint verified

### Testing — Cross-Approach & Integration
- [ ] **TEST-23**: Same workflow logic run through all 4 API styles produces identical operation sequences
- [ ] **TEST-24**: Complex workflow parity — parallel + map + child_context across all approaches
- [ ] **TEST-25**: BatchItemStatus verification — per-item success/failure status checked in parallel/map results

### Architecture — Code Duplication Elimination
- [x] **ARCH-01**: DurableContextOps trait — shared trait with all 44 context methods
- [x] **ARCH-02**: ClosureContext implements DurableContextOps via delegation
- [x] **ARCH-03**: TraitContext implements DurableContextOps via delegation
- [x] **ARCH-04**: BuilderContext implements DurableContextOps via delegation
- [x] **ARCH-05**: Generic handler functions accepting `impl DurableContextOps` work across all approaches
- [x] **ARCH-06**: Handler boilerplate extraction — shared setup_lambda_runtime() function

### Features — Input Validation
- [x] **FEAT-01**: StepOptions validates retries >= 0 and backoff_seconds >= 0
- [x] **FEAT-02**: CallbackOptions validates timeout_seconds > 0 and heartbeat_timeout_seconds > 0
- [x] **FEAT-03**: MapOptions validates batch_size > 0 when set
- [x] **FEAT-04**: Invalid option values panic or return descriptive error at construction

### Features — Error Handling Improvements
- [x] **FEAT-05**: DurableError gains `.code() -> &str` for programmatic error matching
- [x] **FEAT-06**: Each DurableError variant returns a unique, stable error code
- [x] **FEAT-07**: Backend retry detection uses structured error codes instead of string matching
- [x] **FEAT-08**: Checkpoint token None assumption replaced with defensive error handling

### Features — Step Timeout
- [x] **FEAT-09**: StepOptions gains `.timeout_seconds(u64)` field
- [x] **FEAT-10**: Step closure wrapped in tokio::time::timeout when timeout set
- [x] **FEAT-11**: Step exceeding timeout returns DurableError::step_timeout with operation name
- [x] **FEAT-12**: Tests for step timeout (exceeds, completes within, zero timeout)

### Features — Conditional Retry
- [x] **FEAT-13**: StepOptions gains `.retry_if(Fn(&E) -> bool)` predicate
- [x] **FEAT-14**: Retry only when predicate returns true; non-matching errors fail immediately
- [x] **FEAT-15**: Default predicate (no retry_if) retries all errors (backward compatible)
- [x] **FEAT-16**: Tests for conditional retry (transient retries, non-transient fails fast)

### Features — Operation Observability
- [ ] **FEAT-17**: Each operation wrapped in tracing::span with operation name, type, and ID fields
- [ ] **FEAT-18**: Parent-child span hierarchy matches context nesting
- [ ] **FEAT-19**: Span enters on operation start, exits on completion
- [ ] **FEAT-20**: Tests verify spans are emitted with correct fields

### Features — Batch Checkpoint API
- [ ] **FEAT-21**: DurableBackend gains batch_checkpoint() accepting Vec<OperationUpdate>
- [ ] **FEAT-22**: Sequential steps can opt into batched checkpoint mode
- [ ] **FEAT-23**: Single checkpoint call for N operation updates
- [ ] **FEAT-24**: Tests verify batch reduces checkpoint call count

### Features — Saga / Compensation Pattern
- [ ] **FEAT-25**: ctx.step_with_compensation(name, forward_fn, compensate_fn)
- [ ] **FEAT-26**: Compensation closures execute in reverse order on workflow failure
- [ ] **FEAT-27**: Compensation execution is itself checkpointed (durable rollback)
- [ ] **FEAT-28**: Tests for compensation order, compensation failure, partial rollback

### Features — Macro Type Validation
- [ ] **FEAT-29**: #[durable_execution] validates second parameter is DurableContext type
- [ ] **FEAT-30**: #[durable_execution] validates return type is Result<Value, DurableError>
- [ ] **FEAT-31**: Compile-fail trybuild tests for wrong parameter types and return types

### Features — Builder Configuration
- [ ] **FEAT-32**: DurableHandlerBuilder gains .with_tracing(subscriber) method
- [ ] **FEAT-33**: DurableHandlerBuilder gains .with_error_handler(fn) method
- [ ] **FEAT-34**: Tests verify custom configuration takes effect

### Documentation
- [ ] **DOCS-01**: README adds "Determinism Rules" section with do/don't examples
- [ ] **DOCS-02**: README adds error handling example showing two-level Result matching
- [ ] **DOCS-03**: README adds troubleshooting FAQ (Send+Static, Serialize bounds, type annotations)
- [ ] **DOCS-04**: README links to project-context.md for implementation rules
- [ ] **DOCS-05**: Migration guide adds determinism section with anti-patterns
- [ ] **DOCS-06**: BatchResult documentation adds per-item status checking example
- [ ] **DOCS-07**: Parallel example adds comment explaining boxing/type alias complexity
- [ ] **DOCS-08**: CLAUDE.md documents wrapper crate duplication and change propagation requirement
- [ ] **DOCS-09**: Callback documentation adds two-phase operation ID diagram
- [ ] **DOCS-10**: Cargo.toml files gain description, keywords, categories metadata

## v2 Requirements (deferred)
- [ ] **V2-01**: Rate limiting — client-side rate limiting before AWS API calls
- [ ] **V2-02**: Cancellation support — cancel waiting/callback operations
- [ ] **V2-03**: Callback heartbeat — method to send heartbeat during callback wait
- [ ] **V2-04**: Schema validation — validate operation structure against expected schema
- [ ] **V2-05**: Idempotency tokens — auto-generate client_token for retry safety
- [ ] **V2-06**: Operation result local caching — memoize results within invocation
- [ ] **V2-07**: Multiple handler registration — run multiple handlers in one Lambda

## Out of Scope
- Custom serialization formats — must match Python SDK wire format (JSON)
- Multi-runtime support — AWS Lambda ecosystem is tokio-only
- Crate publishing — internal project
- UI/dashboard — out of scope for SDK

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| TEST-01..TEST-11 | Phase 1 | Not started |
| TEST-12..TEST-18 | Phase 2 | Not started |
| TEST-19..TEST-22 | Phase 2 | Not started |
| TEST-23..TEST-25 | Phase 5 | Not started |
| ARCH-01..ARCH-06 | Phase 3 | Not started |
| FEAT-01..FEAT-04 | Phase 4 | Not started |
| FEAT-05..FEAT-08 | Phase 4 | Not started |
| FEAT-09..FEAT-12 | Phase 5 | Not started |
| FEAT-13..FEAT-16 | Phase 5 | Not started |
| FEAT-17..FEAT-20 | Phase 6 | Not started |
| FEAT-21..FEAT-24 | Phase 6 | Not started |
| FEAT-25..FEAT-28 | Phase 7 | Not started |
| FEAT-29..FEAT-31 | Phase 8 | Not started |
| FEAT-32..FEAT-34 | Phase 8 | Not started |
| DOCS-01..DOCS-10 | Phase 9 | Not started |

**Coverage:** 69 v1 requirements, 69 mapped, 0 unmapped
