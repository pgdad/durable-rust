# Retrospective

Living retrospective — updated at each milestone completion.

---

## Milestone: v1.0 — Production Hardening

**Shipped:** 2026-03-17
**Phases:** 9 | **Plans:** 23 | **Commits:** 120

### What Was Built
- 30+ error path and boundary condition tests (Phases 1-2)
- DurableContextOps shared trait eliminating 1,800 lines of delegation duplication (Phase 3)
- Input validation guards + .code() on all DurableError variants + defensive checkpoint token handling (Phase 4)
- Step timeout (timeout_seconds) and conditional retry (retry_if predicate) (Phase 5)
- Tracing spans for all 7 operations + batch checkpoint API with 90% call reduction (Phase 6)
- Saga/compensation pattern with step_with_compensation + run_compensations (Phase 7)
- Proc-macro type validation + builder .with_tracing()/.with_error_handler() (Phase 8)
- README determinism rules, error handling guide, troubleshooting FAQ, migration guide updates (Phase 9)

### What Worked
- **Parallel execution of independent plans** — Wave-based parallelism cut execution time significantly (Phases 4, 5, 8 each had parallel waves)
- **TDD pattern** — Writing tests first caught design issues early, especially for type erasure patterns (retry_if predicate, compensation closures)
- **Research → Plan → Verify → Execute pipeline** — Upfront research prevented dead ends (e.g., discovering tokio::spawn requirement for panic safety before implementation)
- **Incremental phase verification** — Each phase verified independently against its goal, catching gaps immediately (e.g., Phase 4 checker caught missing FEAT-08 test)

### What Was Inefficient
- **Documentation phase (9) could have been incremental** — Writing all docs at the end required re-reading the entire codebase; documenting alongside each feature phase would have been faster
- **Some research was over-scoped** — Phase 9 (docs) didn't need full research; a --skip-research flag would have saved time

### Patterns Established
- `StepOptions` builder with validation-on-construction (panic with descriptive message)
- Type-erased predicates via `Arc<dyn Fn(&dyn Any) -> bool>` for Clone-safe closures
- `OperationType::Context` with `sub_type` discriminator for all composite operations (parallel, map, child_context, compensation)
- Exhaustive `.code()` match on DurableError with no wildcard arm — compiler enforces updates on new variants
- `DurableContextOps` as single point of change for all context methods

### Key Lessons
- Phase 3 (shared trait) should have been Phase 1 — it simplified all subsequent phases by providing a single implementation point
- Plan checker caught a real gap in Phase 4 (missing FEAT-08 test) that would have been a production bug
- The --auto flag on plan-phase chains discuss → plan → verify → execute seamlessly for phases with clear scope

### Cost Observations
- Model mix: primarily sonnet for research/planning/execution agents, opus for orchestration
- Sessions: 1 extended session covering all 9 phases
- Notable: Parallel agent spawning (3 agents for Phase 4 Wave 1) was highly efficient

---

## Cross-Milestone Trends

| Metric | v1.0 |
|--------|------|
| Phases | 9 |
| Plans | 23 |
| Commits | 120 |
| Rust LOC | 21,348 |
| Tests | 100+ |
| Duration | ~20 hours |
| Verification pass rate | 100% (all 9 phases passed) |
| Revision iterations | 2 (Phase 4 + Phase 6 each needed 1 revision) |
