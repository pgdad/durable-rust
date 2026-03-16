# Phase 2: BMAD Cleanup - Context

**Gathered:** 2026-03-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Remove all BMAD artifacts from the repository: `_bmad/`, `_bmad-output/`, and `.claude/skills/bmad-*`. Clean up references in `.planning/` files. Result: zero BMAD presence in the working tree.

</domain>

<decisions>
## Implementation Decisions

### Removal order and commit strategy
- Four dedicated commits, in this order:
  1. `git rm -r _bmad-output/` — BMAD planning artifacts
  2. `git rm -r _bmad/` — BMAD framework tooling
  3. `git rm -r .claude/skills/bmad-*` — BMAD skill directories (all 53)
  4. Clean `_bmad` references from `.planning/` files — doc cleanup commit
- Each commit is atomic and self-contained
- Use exact paths only, never glob patterns for rm operations

### Verification scope
- BMAD-03 ("no orphaned references") means functional references only — paths that would cause errors
- Planning doc cleanup is a quality improvement, not a verification requirement
- After all removals: `grep -r "_bmad" . --include="*.md" --include="*.yaml" --include="*.json"` excluding `.git/` should return zero functional references
- Verify `crates/`, `tests/`, `examples/`, `docs/` are completely untouched

### .planning/ doc cleanup
- Remove or rephrase `_bmad` mentions in .planning/ files (research, plans, milestones, state)
- These are historical references, not functional — cleanup is cosmetic but desired
- Don't rewrite entire documents; just remove/update the specific lines referencing BMAD paths

### Claude's Discretion
- Exact wording of cleaned-up lines in .planning/ files
- Whether to leave brief historical notes ("previously managed with BMAD") or remove completely

</decisions>

<specifics>
## Specific Ideas

- User explicitly wants clean git history: each removal type in its own commit
- Verification should confirm Rust source is untouched — `crates/`, `tests/`, `examples/`, `docs/` must have zero changes

</specifics>

<code_context>
## Existing Code Insights

### Targets for removal
- `_bmad/` — 508 files, BMAD framework (core, bmm, tea modules, configs, memory)
- `_bmad-output/` — 37 files, v1.0 planning artifacts (PRD, architecture, epics, implementation stories, brainstorming)
- `.claude/skills/bmad-*` — 53 directories, all prefixed with `bmad-`

### References to clean
- `.planning/` files contain ~13 references to `_bmad` paths (research notes, plan descriptions, milestone history)
- No Rust source files reference `_bmad`

### Integration Points
- `.claude/skills/` directory will be empty after removal — that's fine, GSD skills live at `~/.claude/get-shit-done/`

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-bmad-cleanup*
*Context gathered: 2026-03-16*
