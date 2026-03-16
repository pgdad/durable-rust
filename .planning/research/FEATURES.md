# Feature Research

**Domain:** Project management tooling transition (BMAD → GSD)
**Researched:** 2026-03-16
**Confidence:** HIGH

## Context

This is not a product feature research document — it is a transition feature research document. The
Rust SDK itself (durable-lambda-*) is complete at v1.0 and unchanged by this milestone. The subject
here is the GSD workflow tooling that replaces BMAD for managing the durable-rust project.

---

## BMAD → GSD Capability Mapping

What BMAD provided and what GSD replaces it with:

| BMAD Capability | BMAD Artifact | GSD Replacement | GSD Artifact |
|-----------------|---------------|-----------------|--------------|
| Project context for AI agents | `_bmad-output/project-context.md` | Living project context doc | `.planning/PROJECT.md` |
| Product requirements | `_bmad-output/planning-artifacts/prd.md` | Requirements in PROJECT.md (Validated/Active/Out of Scope) | `.planning/PROJECT.md` |
| Architecture documentation | `_bmad-output/planning-artifacts/architecture.md` | Research artifacts, codebase map | `.planning/research/ARCHITECTURE.md` |
| Epic/story breakdown | `_bmad-output/planning-artifacts/epics.md` | Phases and plans in ROADMAP | `.planning/ROADMAP.md` + `plans/` |
| Implementation readiness report | `_bmad-output/planning-artifacts/implementation-readiness-report.md` | Phase planning with success criteria | `.planning/plans/XX-YY-PLAN.md` |
| Brainstorming session logs | `_bmad-output/brainstorming/` | Research files, discussion phases | `.planning/research/` |
| Product brief | `_bmad-output/planning-artifacts/product-brief.md` | Summary.md research artifact | `.planning/research/SUMMARY.md` |
| Workflow state (implicit in file layout) | `_bmad/` directory presence | Explicit state tracking | `.planning/STATE.md` |
| Milestone history (none) | No equivalent | Milestone log | `.planning/MILESTONES.md` |
| Phase execution (manual) | No equivalent | Structured plan files with task lists | `.planning/plans/` |

---

## Feature Landscape

### Table Stakes (Team Expects These)

Features the team requires from GSD tooling to function. Missing these makes the transition
pointless.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `.planning/` directory structure | GSD's standard layout; every workflow command assumes it | LOW | Core prerequisite — nothing else works without it |
| `PROJECT.md` with Validated requirements | Team needs AI-readable context capturing all 17 shipped v1.0 requirements | LOW | Content already exists in BMAD; migration task is reformatting |
| `ROADMAP.md` milestone-grouped format | GSD workflows read ROADMAP for phase state; required by `/gsd:progress`, `/gsd:execute-phase`, etc. | LOW | Must use milestone-grouped format since v1.0 is already shipped |
| `STATE.md` | GSD uses STATE.md to resume context between sessions | LOW | Minimal file; references PROJECT.md and current milestone |
| `MILESTONES.md` with v1.0 entry | Documents what shipped so future milestones have a baseline | LOW | Captures all 17 v1.0 deliverables in standard format |
| BMAD artifact removal | Team expectation is clean repo; `_bmad/` and `_bmad-output/` are dead weight post-transition | LOW | Separate git commit per PROJECT.md decision |
| `config.json` settings | GSD config controls verbosity, team profile, and AI behavior; already present | LOW | Already exists at `.planning/config.json` |

### Differentiators (What GSD Adds That BMAD Didn't Have)

Features GSD provides that BMAD did not, creating net-new workflow value for the team.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Structured phase execution workflow | `/gsd:execute-phase` gives AI agents a repeatable execution loop with plan files, task lists, and git commits — BMAD had none of this | LOW | Plan files in `plans/` directory with task checklists |
| Phase planning with success criteria | `/gsd:plan-phase` produces verifiable acceptance criteria before implementation begins | LOW | Prevents "is this done?" ambiguity that BMAD left undefined |
| Phase verification | `/gsd:verify-phase` cross-checks deliverables against success criteria after execution | LOW | Closes the loop BMAD left open |
| AI-optimized session resumption | `/gsd:resume-project` loads full context in one command; BMAD required manual file reading | LOW | STATE.md + PROJECT.md read by the workflow |
| Mid-milestone research | `/gsd:research-phase` spawns targeted research mid-project when unknowns emerge | MEDIUM | Not needed for this transition milestone, but available |
| Milestone audit trail | `MILESTONES.md` creates permanent record of what shipped; BMAD had no milestone concept | LOW | Enables future milestone comparisons and retrospectives |
| Team profile system | `/gsd:set-profile` tunes AI behavior for team type (e.g., AI-heavy dev team) | LOW | One-time setup, already configured at `.planning/config.json` |

### Anti-Features (Don't Add These)

Features that seem like natural extensions but should be explicitly excluded from this transition
milestone.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Migrate BMAD epics content to ROADMAP plans | Feels like preserving history | The epics describe work already completed at v1.0 — migrating them inflates ROADMAP with done work and confuses AI agents about what's active | Capture completed work as v1.0 Validated requirements in PROJECT.md; let ROADMAP start clean with v1.1 |
| Re-architect `.planning/` beyond GSD standard | Customizing directory layout for project-specific needs | GSD workflows assume specific file paths; deviating breaks `/gsd:*` commands | Use GSD standard layout exactly |
| Add CI/CD or toolchain changes during transition | Transition is "in progress", feels like a good time | Out of scope per PROJECT.md; mixing tooling concerns muddies git history | Separate milestone post-transition |
| Port project-context.md AI rules into GSD docs | The 38 AI rules in `_bmad-output/project-context.md` are valuable | They already live in `.claude/CLAUDE.md` or equivalent; duplicating into `.planning/` creates maintenance burden | Keep AI rules in their existing home; reference from PROJECT.md Context section if needed |

---

## Feature Dependencies

```
GSD Infrastructure (.planning/ structure)
    └──requires──> PROJECT.md (content)
    └──requires──> STATE.md (content)
    └──requires──> ROADMAP.md (structure)
    └──requires──> MILESTONES.md (v1.0 entry)

BMAD Removal
    └──requires──> GSD Infrastructure (must exist before BMAD is removed)
    └──requires──> PROJECT.md (replaces _bmad-output/project-context.md)

ROADMAP.md (milestone-grouped)
    └──requires──> MILESTONES.md v1.0 entry (defines what was shipped)
    └──requires──> PROJECT.md Validated requirements (cross-reference)
```

### Dependency Notes

- **BMAD removal requires GSD infrastructure first:** Removing `_bmad/` and `_bmad-output/` before
  `.planning/` is populated leaves the project with no planning context. GSD setup must ship in a
  prior commit.
- **ROADMAP requires MILESTONES:** The milestone-grouped ROADMAP format references the v1.0 milestone
  to know what to collapse into `<details>`. If MILESTONES.md doesn't exist, the ROADMAP has no
  anchor point.
- **PROJECT.md is the keystone:** STATE.md, ROADMAP.md, and MILESTONES.md all reference or depend on
  PROJECT.md being accurate. It must be written first.

---

## MVP Definition

### Launch With (v1.1 Transition Milestone)

Minimum viable GSD infrastructure — the exact deliverables for this milestone.

- [ ] `.planning/PROJECT.md` — Updated with v1.1 milestone context and all 17 v1.0 Validated requirements
- [ ] `.planning/ROADMAP.md` — Milestone-grouped format with v1.0 collapsed and v1.1 phases defined
- [ ] `.planning/STATE.md` — Current state pointing to active milestone
- [ ] `.planning/MILESTONES.md` — v1.0 entry documenting what shipped
- [ ] Remove `_bmad/` directory — Dead BMAD tooling removed (separate commit)
- [ ] Remove `_bmad-output/` directory — Dead BMAD artifacts removed (separate commit)

### Add After Validation (Not in This Milestone)

- [ ] Research files beyond this transition (SUMMARY.md, STACK.md, ARCHITECTURE.md, PITFALLS.md) —
  add when a new feature milestone begins
- [ ] Phase plan files in `plans/` — generate during phase planning via `/gsd:plan-phase`

### Future Consideration (Post-Transition)

- [ ] Codebase map via `/gsd:map-codebase` — useful before new SDK work begins, not needed for
  tooling transition
- [ ] SDK v1.2+ milestone planning — deferred; this milestone is tooling-only per PROJECT.md

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| `.planning/` GSD directory structure | HIGH | LOW | P1 |
| `PROJECT.md` with v1.0 Validated requirements | HIGH | LOW | P1 |
| `ROADMAP.md` milestone-grouped | HIGH | LOW | P1 |
| `STATE.md` | MEDIUM | LOW | P1 |
| `MILESTONES.md` v1.0 entry | MEDIUM | LOW | P1 |
| BMAD directory removal | HIGH | LOW | P1 |
| Phase plan files for v1.1 phases | HIGH | LOW | P2 (generated during execution) |

**Priority key:**
- P1: Must have for transition milestone to be considered complete
- P2: Generated as part of normal GSD execution workflow, not pre-built
- P3: Nice to have, future consideration

---

## Competitor Feature Analysis

Not applicable — this is a tooling transition between two internal workflow systems (BMAD and GSD),
not a product competing in a market. The "competition" is the prior BMAD workflow and the relevant
comparison is capability parity plus net-new value.

**Summary of BMAD vs GSD for this team:**

| Capability | BMAD | GSD |
|------------|------|-----|
| Project context for AI agents | Static file, manually updated | Living document with structured sections |
| Milestone tracking | None | MILESTONES.md with standard entries |
| Phase execution | No structure | Plan files with task checklists and git integration |
| Phase verification | No structure | Explicit success criteria + verify-phase workflow |
| Session resumption | Manual | `/gsd:resume-project` one-command context load |
| State tracking | Implicit | STATE.md + explicit phase/milestone state |
| AI behavior tuning | Monolithic project-context.md | Configurable via config.json team profile |

---

## Sources

- `/Users/esa/git/durable-rust/.planning/PROJECT.md` — authoritative project scope and milestone
  goals
- `/Users/esa/git/durable-rust/_bmad-output/` — BMAD artifacts being replaced (direct inspection)
- `/Users/esa/.claude/get-shit-done/templates/` — GSD template set (direct inspection)
- `/Users/esa/.claude/get-shit-done/workflows/` — GSD workflow commands (direct inspection)
- Confidence: HIGH — all sources are local files with direct inspection, no external verification
  needed

---
*Feature research for: GSD tooling transition (BMAD → GSD) for durable-rust*
*Researched: 2026-03-16*
