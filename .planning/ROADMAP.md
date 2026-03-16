# Roadmap: durable-rust

## Milestones

- ✅ **v1.0 Initial SDK Release** - Shipped 2026-03-13 (see MILESTONES.md)
- 🚧 **v1.1 GSD Tooling Transition** - Phases 1-2 (in progress)

## Phases

<details>
<summary>✅ v1.0 Initial SDK Release - SHIPPED 2026-03-13</summary>

Delivered under BMAD workflow. Full capability list in `.planning/MILESTONES.md`.

20 capabilities shipped: replay engine, 8 durable operations, 4 API styles, MockDurableContext testing framework, Python compliance suite, 28 e2e tests, 44 examples, migration guide, container deployment.

</details>

### 🚧 v1.1 GSD Tooling Transition (In Progress)

**Milestone Goal:** Migrate project management from BMAD tooling to GSD workflow infrastructure. No Rust source changes. Result: a clean repo with `.planning/` fully populated and no `_bmad*` directories.

#### Phase 1: GSD Infrastructure

**Goal**: All GSD planning files exist and the v1.0 milestone history is preserved before any BMAD artifacts are deleted.
**Depends on**: Nothing (first phase)
**Requirements**: GSD-01, GSD-02, GSD-03
**Success Criteria** (what must be TRUE):
  1. `.planning/MILESTONES.md` exists with v1.0 entry documenting all 20 delivered capabilities and key design decisions
  2. `.planning/REQUIREMENTS.md` exists with REQ-IDs for all 6 v1.1 scope items and a traceability table
  3. `.planning/ROADMAP.md` exists with phase definitions for v1.1 continuing from Phase 1
  4. `.planning/STATE.md` points to Phase 2 as the active position
**Plans:** 1/1 plans complete

Plans:
- [x] 01-01-PLAN.md — Verify GSD infrastructure and advance to Phase 2

#### Phase 2: BMAD Cleanup

**Goal**: All BMAD artifacts removed from the repository in four dedicated commits, leaving zero orphaned references.
**Depends on**: Phase 1
**Requirements**: BMAD-01, BMAD-02, BMAD-03
**Success Criteria** (what must be TRUE):
  1. `_bmad-output/` directory no longer exists in the repository (removed via `git rm -r` in a dedicated commit)
  2. `_bmad/` directory no longer exists in the repository (removed via `git rm -r` in a separate dedicated commit)
  3. `grep -r "_bmad" .` (excluding `.git/`) returns no matches in any tracked file
**Plans:** 1/1 plans complete

Plans:
- [ ] 02-01-PLAN.md — Remove BMAD directories and clean references in 4 atomic commits

## Progress

**Execution Order:** Phase 1 → Phase 2

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. GSD Infrastructure | 1/1 | Complete   | 2026-03-16 | - |
| 2. BMAD Cleanup | 1/1 | Complete   | 2026-03-16 | - |
