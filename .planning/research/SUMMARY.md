# Project Research Summary

**Project:** durable-rust — BMAD to GSD tooling transition (v1.1 milestone)
**Domain:** Project management tooling transition on an existing Rust SDK
**Researched:** 2026-03-16
**Confidence:** HIGH

## Executive Summary

This milestone is a pure infrastructure transition — no Rust source code changes. The durable-rust SDK shipped v1.0 complete with 6 crates, 44 examples, 28 e2e tests, and a full compliance suite. The project was managed under the BMAD workflow and must now be brought onto GSD. The GSD `.planning/` directory already exists with `PROJECT.md`, `STATE.md`, and a minimal `config.json`. What remains is creating three missing GSD planning files (`ROADMAP.md`, `MILESTONES.md`, `REQUIREMENTS.md`), then removing BMAD artifacts (`_bmad/` — 508 files, `_bmad-output/` — 37 files) in two separate git commits.

The recommended approach is a three-phase execution: (1) complete GSD infrastructure setup and capture v1.0 milestone history before any deletion, (2) remove `_bmad-output/` in a dedicated commit, and (3) remove `_bmad/` in a second dedicated commit. This ordering is non-negotiable — removing BMAD before GSD infrastructure is populated leaves the project with no planning context, and the `_bmad-output/planning-artifacts/` directory contains SDK design rationale that must be summarized into `MILESTONES.md` before it disappears.

The key risk is context loss: `_bmad-output/planning-artifacts/architecture.md` and `epics.md` contain the authoritative reasoning behind v1.0 design decisions (why blake2b for operation IDs, the 4 API styles, the compliance suite structure). Once deleted, these are only recoverable from git history. The second risk is accidental scope expansion during deletion — the `_bmad-output/implementation-artifacts/tests/` subdirectory sits near the repo's `/tests/` directory, and any glob-based deletion could reach production test files. All deletions must use exact absolute paths.

---

## Key Findings

### Recommended Stack

This milestone requires no new dependencies or technologies. The stack is the GSD planning file infrastructure already partially in place at `.planning/`. The existing `config.json` is minimal but valid — GSD uses defaults for all missing keys, and no expansion is needed for a tooling-only milestone.

**Core technologies:**
- **GSD `.planning/` infrastructure** — project management layer — already partially initialized; completing it is the primary deliverable
- **Git** — version control for planning artifacts — `commit_docs` defaults to `true`, keeping `.planning/` files in history; correct for this AI-heavy team profile

**See:** `.planning/research/STACK.md` for full BMAD artifact inventory and disposition.

### Expected Features

This is a capability transition, not a product feature addition. The deliverables are workflow infrastructure, not user-facing features.

**Must have (table stakes for transition to be complete):**
- `.planning/ROADMAP.md` in milestone-grouped format — required by all `/gsd:*` execution commands
- `.planning/MILESTONES.md` with v1.0 entry — anchors phase numbering and preserves shipped history
- BMAD artifact removal (`_bmad/` and `_bmad-output/`) — dead weight with no runtime value post-transition
- `PROJECT.md` Validated requirements current — already present; verify 17 v1.0 requirements are captured
- `STATE.md` pointing to active milestone — already present; update after ROADMAP is created

**Should have (GSD adds net-new value BMAD lacked):**
- Structured phase execution via plan files in `.planning/phases/` — BMAD had no equivalent
- Explicit success criteria per phase — prevents "is this done?" ambiguity
- Phase verification workflow (`/gsd:verify-phase`) — closes the loop BMAD left open

**Defer to post-transition:**
- Codebase map via `/gsd:map-codebase` — useful before new SDK work, not needed for tooling transition
- Research files for SDK v1.2+ features — belongs in the next feature milestone

**See:** `.planning/research/FEATURES.md` for full BMAD → GSD capability mapping.

### Architecture Approach

The GSD `.planning/` directory sits beside the Rust workspace without touching it. `crates/`, `tests/`, `examples/`, `compliance/`, and `docs/` are pure SDK deliverables untouched by this milestone. Planning infrastructure is additive and isolated — a deliberate GSD contract that keeps planning files out of Cargo workspace scope. After transition, the repo root will have no `_bmad*` directories and a complete `.planning/` tree.

**Major components:**
1. `.planning/PROJECT.md` — living requirements and decisions (EXISTS, authoritative)
2. `.planning/STATE.md` — current phase position and session context (EXISTS, needs update post-ROADMAP)
3. `.planning/ROADMAP.md` — phase definitions for v1.1 milestone (MISSING — create in Phase 1)
4. `.planning/MILESTONES.md` — v1.0 milestone archive (MISSING — create in Phase 1 before deletion)
5. `_bmad/` — BMAD tooling framework (REMOVE in Phase 2, separate commit)
6. `_bmad-output/` — BMAD project artifacts (REMOVE in Phase 2, separate commit)

**See:** `.planning/research/ARCHITECTURE.md` for full removal strategy and anti-patterns.

### Critical Pitfalls

1. **Deleting Rust source during BMAD removal** — use exact absolute paths only (`/Users/esa/git/durable-rust/_bmad/` and `/Users/esa/git/durable-rust/_bmad-output/`); never globs; verify with `ls` before deletion; confirm `crates/` and `tests/` are untouched after

2. **Removing BMAD before capturing v1.0 context** — `_bmad-output/planning-artifacts/architecture.md` and `epics.md` contain irreplaceable design rationale; extract key decisions into `MILESTONES.md` before any deletion; this is a one-way door

3. **Bundling BMAD removal into GSD setup commit** — PROJECT.md explicitly records "Remove BMAD artifacts in separate commit"; two commits minimum (one per directory); enables clean git bisect and clear audit trail

4. **STATE.md optimistic updates** — only update STATE.md after acceptance criteria are verified, not when work begins; include STATE.md update as the final step of every phase definition of done

5. **GSD files written outside `.planning/`** — all file writes must use absolute paths anchored to `/Users/esa/git/durable-rust/.planning/`; verify after each phase that no new `.md` files appeared at repo root

**See:** `.planning/research/PITFALLS.md` for full pitfall catalog, recovery strategies, and phase mapping.

---

## Implications for Roadmap

Based on combined research, a two-phase structure is recommended for this milestone. The phases map directly to the dependency chain identified across all four research files.

### Phase 1: Complete GSD Infrastructure

**Rationale:** This phase must come first because BMAD removal depends on GSD infrastructure existing, ROADMAP depends on MILESTONES anchoring v1.0, and MILESTONES depends on PROJECT.md being accurate. Nothing else can proceed until this foundation is in place.

**Delivers:**
- `.planning/ROADMAP.md` in milestone-grouped format (v1.0 collapsed, v1.1 phases defined)
- `.planning/MILESTONES.md` with v1.0 entry (17 deliverables documented, design rationale preserved)
- Verified `.planning/PROJECT.md` — confirm 17 v1.0 Validated requirements are captured
- Updated `.planning/STATE.md` — pointing to v1.1 Phase 2

**Addresses (from FEATURES.md):** All P1 table-stakes features except BMAD removal

**Avoids (from PITFALLS.md):**
- Pitfall 3 (orphaned cross-references): capture `architecture.md` and `epics.md` content before they are deleted
- Pitfall 4 (STATE.md inconsistency): update STATE.md only after each acceptance criterion is met
- Pitfall 6 (GSD files in wrong location): use absolute paths for all file writes

**Research flag:** Standard patterns — no research-phase needed. GSD template files are authoritative; BMAD output provides all source content.

---

### Phase 2: Remove BMAD Artifacts

**Rationale:** Removal depends on Phase 1 being complete — specifically, MILESTONES.md must exist and PROJECT.md must contain the extracted design context before BMAD artifacts are deleted. The two BMAD directories must be removed in two separate commits to maintain the clean audit trail required by PROJECT.md's key decision.

**Delivers:**
- `_bmad-output/` removed via `git rm -r` (commit 1: `chore: remove BMAD output artifacts`)
- `_bmad/` removed via `git rm -r` (commit 2: `chore: remove BMAD tooling`)
- `.claude/skills/` verified and removed if BMAD-only (commit 3 if applicable)
- Zero `_bmad` references remaining across repo (`grep -r "_bmad" .` returns nothing outside git history)

**Addresses (from FEATURES.md):** BMAD directory removal (P1 table stakes)

**Avoids (from PITFALLS.md):**
- Pitfall 1 (accidental Rust source deletion): exact absolute paths, `ls` verification before deletion, `cargo check` after
- Pitfall 2 (breaking git history): no amend, no rebase, no force push; standalone commits only
- Pitfall 3 (orphaned cross-references): run `grep -r "_bmad" .` as acceptance criterion
- Pitfall 5 (unnecessary cargo runs): acceptance criteria contain no `cargo` commands

**Research flag:** Standard patterns — no research-phase needed. Removal is a mechanical `git rm` operation with verification steps.

---

### Phase Ordering Rationale

- **Phase 1 before Phase 2** is a hard dependency: MILESTONES.md must exist before BMAD artifacts are deleted, and GSD infrastructure must be operational before the BMAD safety net is removed
- **No Phase 3** is needed: this milestone has no Rust source changes; post-transition SDK work belongs to a future milestone
- **Two phases, not three** is deliberate: the BMAD removal sub-commits (one per directory) are implementation details within Phase 2, not separate roadmap phases; the phase boundary tracks architectural intent, not file count

### Research Flags

Phases with standard patterns (skip `/gsd:research-phase`):
- **Phase 1 (GSD Infrastructure):** GSD templates are the authoritative spec; no unknowns; BMAD output provides all source content needed for MILESTONES.md
- **Phase 2 (BMAD Removal):** Pure filesystem operations with well-understood git mechanics; no new patterns needed

No phases require deeper research. This is a tooling transition, not a feature build. All patterns are established.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All sources are local files with direct inspection; no external dependencies or version uncertainty |
| Features | HIGH | BMAD artifacts directly inspected; GSD templates directly inspected; capability mapping is exact |
| Architecture | HIGH | Repository structure directly inspected; GSD reference docs authoritative; no inference required |
| Pitfalls | HIGH | Pitfalls derived from direct examination of actual file structure and git history; not theoretical |

**Overall confidence:** HIGH

### Gaps to Address

- **`.claude/skills/` disposition:** Research notes this directory exists and may be BMAD-only, but directs "verify first." Phase 2 acceptance criteria should include an `ls` of this directory before deciding whether to remove it. Not blocking, but needs a verification step.
- **PROJECT.md completeness:** Research flags the need to verify 17 v1.0 Validated requirements are in PROJECT.md. This is a read-verify step, not a gap in knowledge — but it must be confirmed in Phase 1 before MILESTONES.md is written.

---

## Sources

### Primary (HIGH confidence)
- `/Users/esa/git/durable-rust/.planning/PROJECT.md` — project scope, key decisions, Validated requirements
- `/Users/esa/git/durable-rust/_bmad-output/` — BMAD artifacts being replaced (direct inspection)
- `/Users/esa/git/durable-rust/_bmad/` — BMAD tooling structure (direct inspection)
- `~/.claude/get-shit-done/templates/` — GSD template set (authoritative)
- `~/.claude/get-shit-done/workflows/` — GSD workflow commands (authoritative)
- `~/.claude/get-shit-done/references/` — GSD planning config, git integration references

### Secondary
None — all sources are primary for this domain (local filesystem inspection + authoritative GSD tooling docs).

---
*Research completed: 2026-03-16*
*Ready for roadmap: yes*
