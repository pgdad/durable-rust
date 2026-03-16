# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.1 — GSD Tooling Transition

**Shipped:** 2026-03-16
**Phases:** 2 | **Plans:** 2 | **Sessions:** 1

### What Was Built
- GSD planning infrastructure (PROJECT.md, MILESTONES.md, REQUIREMENTS.md, ROADMAP.md, STATE.md)
- v1.0 historical record with 20 capabilities and 7 design decisions
- Clean repo with zero BMAD presence (545+ files removed in 4 atomic commits)

### What Worked
- Auto-advance chain (`--auto`) allowed discuss → plan → execute to flow without manual intervention
- 4-commit removal strategy kept git history clean and reviewable
- Research phase correctly identified `.claude/skills/bmad-*` as an additional removal target not in original requirements
- Plan checker caught a verify gap (Phase 1 files missing from Task 3 automated check) before execution

### What Was Inefficient
- Phase 1 (GSD Infrastructure) was essentially verification of already-created files — most work was done during milestone initialization
- `git rm` left empty directory shells requiring manual `rm -rf` cleanup — a known git behavior but worth noting for future removal phases

### Patterns Established
- Definitional exclusion pattern: planning docs that describe removal work are exempt from the "zero references" check
- 4-commit atomic removal: output artifacts → framework → skills → doc cleanup

### Key Lessons
1. For tooling transitions, the research phase adds value by discovering scope items (like `.claude/skills/`) that aren't obvious from requirements alone
2. Empty directory cleanup after `git rm -r` should be included in plan tasks, not left as a manual step

### Cost Observations
- Model mix: ~20% opus (orchestrator), ~80% sonnet (researchers, planners, executors, verifiers)
- Sessions: 1 continuous session
- Notable: Simple milestone completed end-to-end in a single session with auto-advance

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | N/A | N/A | Managed under BMAD (no GSD data) |
| v1.1 | 1 | 2 | First GSD milestone; established planning infrastructure |

### Cumulative Quality

| Milestone | Tests | Coverage | Zero-Dep Additions |
|-----------|-------|----------|-------------------|
| v1.0 | 28 e2e + parity + compliance | Full | N/A |
| v1.1 | N/A (docs-only) | N/A | 0 |

### Top Lessons (Verified Across Milestones)

1. (Single milestone so far — lessons above will be cross-validated in future milestones)
