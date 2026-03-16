# Phase 1: GSD Infrastructure - Research

**Researched:** 2026-03-16
**Domain:** GSD planning file creation and state management
**Confidence:** HIGH

## Summary

Phase 1 establishes the GSD planning infrastructure before any BMAD artifacts are deleted. The goal is to ensure all `.planning/` files that record v1.0 history and define the v1.1 roadmap exist and are correct before Phase 2 removes the BMAD source material.

The critical discovery for planning: the four core files (MILESTONES.md, REQUIREMENTS.md, ROADMAP.md, STATE.md) **already exist** as of the v1.1 initialization commit on 2026-03-16 (commit `e2ebc02`). Phase 1 work is therefore primarily verification, gap-filling, and the STATE.md advancement to Phase 2 — not creation from scratch.

The single plan for this phase (01-01) must verify all four success criteria from ROADMAP.md, make any corrections, and then advance STATE.md to Phase 2. All changes must be committed to git immediately (per project memory rule).

**Primary recommendation:** Verify each success criterion against the existing files, patch any gaps, then update STATE.md Phase field from "1" to "2" in a single commit.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| GSD-01 | MILESTONES.md exists capturing v1.0 as shipped milestone with validated capabilities | MILESTONES.md already exists at `.planning/MILESTONES.md` with all 20 capabilities and key design decisions. Plan must verify completeness against the 20-item capability list and 7-item key decisions table. |
| GSD-02 | REQUIREMENTS.md exists with REQ-IDs for all v1.1 scope items | REQUIREMENTS.md already exists at `.planning/REQUIREMENTS.md` with GSD-01/02/03 and BMAD-01/02/03 (6 total), plus traceability table. Plan must verify all 6 IDs are present with traceability rows. |
| GSD-03 | ROADMAP.md exists with phased execution plan continuing from Phase 1 | ROADMAP.md already exists at `.planning/ROADMAP.md` with Phase 1 and Phase 2 definitions. Plan must verify Phase 2 (BMAD Cleanup) is defined with goals, dependencies, requirements, and success criteria. |
</phase_requirements>

## Current File Inventory

All files inspected on 2026-03-16 — actual content confirmed by Read tool.

| File | Path | Status | Key Content |
|------|------|--------|-------------|
| MILESTONES.md | `.planning/MILESTONES.md` | EXISTS | v1.0 entry with 20 capabilities + 7 design decisions; v1.1 active entry stub |
| REQUIREMENTS.md | `.planning/REQUIREMENTS.md` | EXISTS | 6 v1.1 IDs (GSD-01..03, BMAD-01..03), traceability table, Future/Out-of-Scope sections |
| ROADMAP.md | `.planning/ROADMAP.md` | EXISTS | Phase 1 + Phase 2 definitions; progress table; v1.0 history entry |
| STATE.md | `.planning/STATE.md` | EXISTS, NEEDS UPDATE | Shows Phase 1 as current; must advance to Phase 2 after plan completion |
| PROJECT.md | `.planning/PROJECT.md` | EXISTS | Core project description, constraints, key decisions |
| config.json | `.planning/config.json` | EXISTS | `{"workflow": {"research": true}}` |

## Architecture Patterns

### GSD File Roles

| File | Purpose | Who Reads It |
|------|---------|-------------|
| MILESTONES.md | Permanent history of shipped work | Future planners, human team |
| REQUIREMENTS.md | Current milestone requirements with IDs for traceability | Planner, verifier |
| ROADMAP.md | Phase execution plan for current milestone | Planner, verifier, STATE.md sync |
| STATE.md | Current active position — phase/plan/status | Every GSD agent on resume |
| PROJECT.md | Stable project identity and constraints | Every GSD agent for context |

### STATE.md Update Pattern

When a phase completes, STATE.md must be updated:

```
## Current Position

Phase: 2 of 2 (BMAD Cleanup)        ← increment
Plan: 0 of 1 in current phase        ← reset to 0
Status: Ready to plan                 ← reset
Last activity: 2026-03-16 — Phase 1 complete, GSD infrastructure verified
```

The `Progress` bar also updates. With 0 plans complete across 2 total phases (2 plans total), after plan 01-01 completes: 1/2 = 50%.

### Success Criteria Verification Pattern

Each of the 4 success criteria from ROADMAP.md maps to a specific check:

| # | Success Criterion | Verification Action |
|---|------------------|---------------------|
| 1 | MILESTONES.md has v1.0 entry with all 20 capabilities and key design decisions | Count rows in "Delivered Capabilities" table (must be 20); count rows in "Key Design Decisions" table (must be 7) |
| 2 | REQUIREMENTS.md has REQ-IDs for all 6 v1.1 items and traceability table | Verify GSD-01, GSD-02, GSD-03, BMAD-01, BMAD-02, BMAD-03 all present; verify traceability table has 6 rows |
| 3 | ROADMAP.md has phase definitions for v1.1 continuing from Phase 1 | Verify Phase 2 (BMAD Cleanup) section exists with goal, depends-on, requirements, and success criteria |
| 4 | STATE.md points to Phase 2 as the active position | Update Phase field to "2 of 2 (BMAD Cleanup)" and Last activity date |

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| File presence check | Custom validation script | Read tool + eyeball count | This is a one-time doc verification, not recurring automation |
| STATE.md update | Automated parser | Edit tool with targeted line replacement | File format is simple markdown; direct edit is safer than scripting |

## Common Pitfalls

### Pitfall 1: Treating Existing Files as "Not Done"
**What goes wrong:** Planner schedules tasks to "create" files that already exist, causing either duplicate creation or wasted effort reviewing a file that just needs a final line change.
**Why it happens:** Phase description says "create" but initialization already ran.
**How to avoid:** The plan for 01-01 must open each file, verify its contents against the success criteria, and only patch what is actually missing or wrong.

### Pitfall 2: Forgetting STATE.md Is the Phase Gate
**What goes wrong:** All files verified as complete but STATE.md is never updated, leaving Phase 1 forever "current" and Phase 2 never starting.
**Why it happens:** STATE.md is not listed in the REQUIREMENTS.md IDs — it's an operational artifact, not a deliverable.
**How to avoid:** Make STATE.md advancement the final explicit action in plan 01-01, after all GSD-0X criteria are verified.

### Pitfall 3: Not Committing After Verification
**What goes wrong:** Files are verified/patched but not committed. If the session ends, the verification is lost and must be repeated.
**Why it happens:** Verification feels like "nothing changed" so commit is skipped.
**How to avoid:** Even if no file content changed, the STATE.md update ALWAYS produces a commit. Per project memory: commit docs immediately as created or modified.

### Pitfall 4: Partial MILESTONES.md v1.0 Entry
**What goes wrong:** MILESTONES.md v1.0 entry exists but the capabilities count or design decisions are incomplete.
**Why it happens:** The success criterion says "all 20 delivered capabilities" — must be verified against the authoritative list in PROJECT.md.
**How to avoid:** Cross-reference the 20 items in MILESTONES.md against PROJECT.md's "Validated" requirements list. The current MILESTONES.md already has 20 rows (confirmed) — verification is a count-check.

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|-----------------|--------|
| BMAD workflow artifacts in `_bmad/` | GSD `.planning/` directory | Phase 2 removes BMAD; this phase establishes the replacement |
| BMAD epics/stories in `_bmad-output/` | MILESTONES.md history entries | v1.0 planning context preserved in MILESTONES.md before deletion |

## Validation Architecture

> `workflow.nyquist_validation` is not set in `.planning/config.json` (only `workflow.research: true` is present), so nyquist_validation is absent — treat as enabled.

### Test Framework

Phase 1 is entirely documentation work. There is no test framework applicable. All verification is human/agent inspection of file contents against defined success criteria.

| Property | Value |
|----------|-------|
| Framework | N/A — documentation phase |
| Config file | N/A |
| Quick run command | Manual: read each file, verify counts |
| Full suite command | Manual: verify all 4 success criteria from ROADMAP.md |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GSD-01 | MILESTONES.md has 20 capability rows and 7 design decision rows | manual-only | N/A — count table rows by inspection | ✅ `.planning/MILESTONES.md` |
| GSD-02 | REQUIREMENTS.md has 6 REQ-IDs and traceability table | manual-only | N/A — verify IDs by inspection | ✅ `.planning/REQUIREMENTS.md` |
| GSD-03 | ROADMAP.md has Phase 2 definition with all required fields | manual-only | N/A — verify section by inspection | ✅ `.planning/ROADMAP.md` |

**Justification for manual-only:** All deliverables are Markdown documentation files. Automated test infrastructure is not applicable to document structure verification. Verification is performed inline by the executing agent using Read tool inspection.

### Wave 0 Gaps

None — no test infrastructure needed for this documentation phase.

## Open Questions

1. **Should MILESTONES.md v1.1 entry be expanded before Phase 2 runs?**
   - What we know: The v1.1 entry currently has only a stub ("Started: 2026-03-16, see ROADMAP.md")
   - What's unclear: Should Phase 1 flesh this out, or wait for Phase 2 completion?
   - Recommendation: Leave as stub; Phase 2 completion is the right time to record v1.1 as shipped. Phase 1 only needs to ensure v1.0 entry is complete.

2. **Does the ROADMAP.md "Plans: TBD" line need updating?**
   - What we know: ROADMAP.md Phase 1 currently shows `Plans: TBD` but has the actual plan list below it.
   - What's unclear: Is `Plans: TBD` a formatting artifact or intentional?
   - Recommendation: The planner should remove the `Plans: TBD` line from Phase 1 when the plan is defined — it is a duplicate/stale line next to the actual plan list.

## Sources

### Primary (HIGH confidence)

- Read tool — `.planning/MILESTONES.md` — inspected full content, confirmed 20 capabilities + 7 decisions
- Read tool — `.planning/REQUIREMENTS.md` — inspected full content, confirmed 6 IDs + traceability
- Read tool — `.planning/ROADMAP.md` — inspected full content, confirmed Phase 1 + Phase 2 definitions
- Read tool — `.planning/STATE.md` — inspected full content, confirmed Phase 1 as current position
- Read tool — `.planning/PROJECT.md` — inspected full content, confirms project constraints
- git log — commit `e2ebc02` confirms planning files created 2026-03-16

### Secondary (MEDIUM confidence)

- N/A for this phase (no external libraries or frameworks)

### Tertiary (LOW confidence)

- N/A

## Metadata

**Confidence breakdown:**
- Current file state: HIGH — directly inspected all files with Read tool
- Success criteria mapping: HIGH — criteria from ROADMAP.md, files verified to exist
- STATE.md update pattern: HIGH — format inferred directly from existing STATE.md content
- Pitfalls: HIGH — derived from direct file inspection, not speculation

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (stable domain — documentation files do not change unless edited)
