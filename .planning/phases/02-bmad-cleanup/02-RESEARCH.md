# Phase 2: BMAD Cleanup - Research

**Researched:** 2026-03-16
**Domain:** Git repository cleanup — directory removal, reference scrubbing
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Removal order and commit strategy:**
- Four dedicated commits, in this order:
  1. `git rm -r _bmad-output/` — BMAD planning artifacts
  2. `git rm -r _bmad/` — BMAD framework tooling
  3. `git rm -r .claude/skills/bmad-*` — BMAD skill directories (all 53)
  4. Clean `_bmad` references from `.planning/` files — doc cleanup commit
- Each commit is atomic and self-contained
- Use exact paths only, never glob patterns for rm operations

**Verification scope:**
- BMAD-03 ("no orphaned references") means functional references only — paths that would cause errors
- Planning doc cleanup is a quality improvement, not a verification requirement
- After all removals: `grep -r "_bmad" . --include="*.md" --include="*.yaml" --include="*.json"` excluding `.git/` should return zero functional references
- Verify `crates/`, `tests/`, `examples/`, `docs/` are completely untouched

**`.planning/` doc cleanup:**
- Remove or rephrase `_bmad` mentions in .planning/ files (research, plans, milestones, state)
- These are historical references, not functional — cleanup is cosmetic but desired
- Don't rewrite entire documents; just remove/update the specific lines referencing BMAD paths

### Claude's Discretion

- Exact wording of cleaned-up lines in `.planning/` files
- Whether to leave brief historical notes ("previously managed with BMAD") or remove completely

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BMAD-01 | `_bmad-output/` directory removed from repository in a dedicated commit | Directory confirmed tracked (37 files, 5 subdirs); `git rm -r` handles both filesystem and index removal |
| BMAD-02 | `_bmad/` directory removed from repository in a dedicated commit | Directory confirmed tracked (508 files, 5 subdirs); separate commit after BMAD-01 |
| BMAD-03 | No orphaned references to `_bmad` remain in tracked files | 14 `.planning/` files contain references; Rust source (`crates/`, `tests/`, `examples/`, `docs/`) has zero references — confirmed |
</phase_requirements>

---

## Summary

Phase 2 is a pure repository cleanup phase with no Rust source changes. Three tracked directories must be removed — `_bmad-output/` (37 files), `_bmad/` (508 files), and `.claude/skills/bmad-*` (54 directories) — each in its own dedicated `git rm -r` commit. A fourth commit cleans residual `_bmad` text references from 14 `.planning/` files.

All four removal targets are confirmed present via direct filesystem inspection. No Rust source files reference `_bmad` in any form — the blast radius is limited entirely to planning infrastructure. The `.claude/skills/` directory will be empty after removal; that is expected because GSD skills live at `~/.claude/get-shit-done/`, not in the project repo.

The critical constraint is commit atomicity: each removal type gets its own commit in strict order (`_bmad-output/` first, `_bmad/` second, `.claude/skills/bmad-*` third, doc cleanup fourth). This was an explicit project decision captured in STATE.md and confirmed in CONTEXT.md.

**Primary recommendation:** Execute four sequential `git rm`/`git commit` operations in the locked order, then verify with `grep -r "_bmad" . --exclude-dir=.git` returning zero hits.

---

## Standard Stack

### Core Operations

| Tool | Purpose | Why Standard |
|------|---------|--------------|
| `git rm -r` | Remove tracked directories from index + filesystem | Single command handles both; correct for tracked files |
| `grep -r` | Verify zero residual references | Shell-native; no dependencies |
| `git status` | Confirm clean working tree after each commit | Sanity check |

### Commit Message Conventions

Based on recent commit history (conventional commits style observed in repo):

| Commit # | Suggested Message |
|----------|------------------|
| 1 | `chore: remove _bmad-output/ planning artifacts` |
| 2 | `chore: remove _bmad/ framework tooling` |
| 3 | `chore: remove .claude/skills bmad skill directories` |
| 4 | `docs: clean _bmad references from .planning/ files` |

---

## Architecture Patterns

### Confirmed Removal Targets

**Target 1: `_bmad-output/` — 37 files**
```
_bmad-output/
├── brainstorming/
├── implementation-artifacts/
├── planning-artifacts/
├── project-context.md
└── test-artifacts/
```
Status: Tracked. No `.gitignore` entry. Content already captured in `.planning/MILESTONES.md` (Phase 1 complete).

**Target 2: `_bmad/` — 508 files**
```
_bmad/
├── _config/
├── _memory/
├── bmm/
├── core/
└── tea/
```
Status: Tracked. Pure tooling — no runtime value post-GSD migration.

**Target 3: `.claude/skills/bmad-*` — 54 directories**

All 54 skill directories share the `bmad-` prefix. Full list confirmed:
`bmad-advanced-elicitation`, `bmad-agent-tea-tea`, `bmad-analyst`, `bmad-architect`, `bmad-brainstorming`, `bmad-check-implementation-readiness`, `bmad-code-review`, `bmad-correct-course`, `bmad-create-architecture`, `bmad-create-epics-and-stories`, `bmad-create-prd`, `bmad-create-product-brief`, `bmad-create-story`, `bmad-create-ux-design`, `bmad-dev`, `bmad-dev-story`, `bmad-document-project`, `bmad-domain-research`, `bmad-edit-prd`, `bmad-editorial-review-prose`, `bmad-editorial-review-structure`, `bmad-generate-project-context`, `bmad-help`, `bmad-index-docs`, `bmad-market-research`, `bmad-master`, `bmad-party-mode`, `bmad-pm`, `bmad-qa`, `bmad-qa-generate-e2e-tests`, `bmad-quick-dev`, `bmad-quick-dev-new-preview`, `bmad-quick-flow-solo-dev`, `bmad-quick-spec`, `bmad-retrospective`, `bmad-review-adversarial-general`, `bmad-review-edge-case-hunter`, `bmad-shard-doc`, `bmad-sm`, `bmad-sprint-planning`, `bmad-sprint-status`, `bmad-tea-teach-me-testing`, `bmad-tea-testarch-atdd`, `bmad-tea-testarch-automate`, `bmad-tea-testarch-ci`, `bmad-tea-testarch-framework`, `bmad-tea-testarch-nfr`, `bmad-tea-testarch-test-design`, `bmad-tea-testarch-test-review`, `bmad-tea-testarch-trace`, `bmad-tech-writer`, `bmad-technical-research`, `bmad-ux-designer`, `bmad-validate-prd`

Status: Tracked. `.claude/skills/` will be empty after removal — expected and acceptable.

**Target 4: `.planning/` file references — 14 files**

Files containing `_bmad` text references (confirmed via grep):
- `.planning/research/STACK.md`
- `.planning/research/ARCHITECTURE.md`
- `.planning/research/PITFALLS.md`
- `.planning/research/SUMMARY.md`
- `.planning/research/FEATURES.md`
- `.planning/REQUIREMENTS.md`
- `.planning/PROJECT.md`
- `.planning/STATE.md`
- `.planning/ROADMAP.md`
- `.planning/MILESTONES.md`
- `.planning/phases/01-gsd-infrastructure/01-RESEARCH.md`
- `.planning/phases/01-gsd-infrastructure/01-01-PLAN.md`
- `.planning/phases/01-gsd-infrastructure/01-01-SUMMARY.md`
- `.planning/phases/02-bmad-cleanup/02-CONTEXT.md` (this file itself — leave as-is; it is historical record)

Note: `REQUIREMENTS.md`, `ROADMAP.md`, `STATE.md` references are definitional (they name the requirement or describe what this phase does). Those should remain or be rephrased to past tense after execution, not blindly deleted.

### Execution Sequence

```
Step 1: Verify targets exist
  ls -la _bmad-output/ _bmad/ .claude/skills/ | head -5

Step 2: Remove _bmad-output/ (BMAD-01)
  git rm -r _bmad-output/
  git commit -m "chore: remove _bmad-output/ planning artifacts"

Step 3: Remove _bmad/ (BMAD-02)
  git rm -r _bmad/
  git commit -m "chore: remove _bmad/ framework tooling"

Step 4: Remove .claude/skills/bmad-* (54 dirs, one by one or via script)
  # Enumerate exact paths — no globs per constraint
  git rm -r .claude/skills/bmad-advanced-elicitation
  git rm -r .claude/skills/bmad-agent-tea-tea
  ... (all 54)
  git commit -m "chore: remove .claude/skills bmad skill directories"

Step 5: Clean .planning/ references
  # Edit each file to remove/rephrase _bmad path mentions
  git add .planning/...
  git commit -m "docs: clean _bmad references from .planning/ files"

Step 6: Verify zero references
  grep -r "_bmad" . --include="*.md" --include="*.yaml" --include="*.json" \
    --exclude-dir=.git
  # Must return empty

Step 7: Confirm Rust source untouched
  git diff HEAD~4 -- crates/ tests/ examples/ docs/
  # Must return empty
```

### Anti-Patterns to Avoid

- **Using glob paths in `git rm`:** The constraint is "exact paths only." Do not run `git rm -r .claude/skills/bmad-*` as a shell glob — enumerate each directory path explicitly or use a script that resolves the paths first.
- **Single commit for all removals:** The project requires four separate commits. Combining them violates the explicit commit strategy decision.
- **Deleting `02-CONTEXT.md` references:** The CONTEXT.md file is a historical planning artifact. Its `_bmad` mentions are definitional, not orphaned. Do not edit it.
- **Editing REQUIREMENTS.md to remove requirement text:** The `_bmad` mentions in REQUIREMENTS.md are the requirement definitions themselves (`BMAD-01`, `BMAD-02`, `BMAD-03`). After phase completion, update checkbox status — do not delete the requirement text.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Removing tracked files from git | Custom deletion script | `git rm -r <exact-path>` | `git rm` updates the index atomically; plain `rm` leaves index dirty |
| Finding residual references | Custom scanner | `grep -r "_bmad" . --exclude-dir=.git` | Shell-native, reliable, already specified in acceptance criteria |
| Verifying Rust source untouched | Manual file check | `git diff HEAD~N -- crates/ tests/ examples/ docs/` | Diff covers entire tree in one command |

**Key insight:** This phase has no algorithmic complexity. The entire implementation is filesystem operations and text edits. Any scripting overhead is waste.

---

## Common Pitfalls

### Pitfall 1: Glob expansion on `.claude/skills/bmad-*`

**What goes wrong:** Shell glob expansion of `bmad-*` may silently fail or match unexpected entries if the shell is in the wrong directory, or may attempt to remove non-bmad directories if any new directory names were created.

**Why it happens:** The constraint says "exact paths only" for safety. With 54 directories, the temptation to use a glob is high.

**How to avoid:** Enumerate all 54 directory names explicitly (verified list above). Or resolve with `ls .claude/skills/ | grep bmad` first, review the output, then pass each resolved path to `git rm -r`.

**Warning signs:** `git status` shows staged deletions of unexpected paths.

### Pitfall 2: Forgetting that `.claude/skills/` will be empty

**What goes wrong:** After removing all 54 `bmad-*` directories, `.claude/skills/` will be empty. Git does not track empty directories — the directory itself will disappear from the working tree. This is expected but can look alarming.

**Why it happens:** Git does not version empty directories.

**How to avoid:** Confirm this is expected behavior before committing. The context document explicitly states: "`.claude/skills/` directory will be empty after removal — that's fine, GSD skills live at `~/.claude/get-shit-done/`."

### Pitfall 3: Editing REQUIREMENTS.md requirement text

**What goes wrong:** During `.planning/` cleanup, an agent removes the BMAD-01/02/03 requirement lines because they contain `_bmad` text.

**Why it happens:** The grep for `_bmad` will hit REQUIREMENTS.md, and a broad "remove all references" instruction will delete the definitions.

**How to avoid:** For REQUIREMENTS.md, the only change needed is updating checkboxes from `[ ]` to `[x]` after each requirement is met. The requirement text itself must remain.

### Pitfall 4: Accidentally touching Rust source

**What goes wrong:** A broad rm or cleanup operation accidentally modifies or deletes files in `crates/`, `tests/`, `examples/`, or `docs/`.

**Why it happens:** `_bmad-output/implementation-artifacts/tests/` sits near the repo root's `/tests/` directory. Any relative-path confusion could reach `/tests/`.

**How to avoid:** Use absolute paths for all removals. After each commit, run `git diff HEAD~1 -- crates/ tests/ examples/ docs/` to confirm zero diff.

### Pitfall 5: Committing doc cleanup before directory removals complete

**What goes wrong:** Doc cleanup is done first, removing references to directories that still exist in the working tree, making the commit message misleading ("cleaned references" before the referenced items are gone).

**Why it happens:** Temptation to do the "easy" text edits first.

**How to avoid:** Follow the locked commit order. Doc cleanup is commit #4, after all three `git rm` commits.

---

## Code Examples

### Removing a tracked directory

```bash
# Correct: git rm handles index + filesystem together
git rm -r _bmad-output/
git commit -m "chore: remove _bmad-output/ planning artifacts"

# Wrong: plain rm leaves git index dirty
rm -rf _bmad-output/
```

### Verifying zero orphaned references after cleanup

```bash
# Check all text file types, exclude git history
grep -r "_bmad" . \
  --include="*.md" \
  --include="*.yaml" \
  --include="*.json" \
  --include="*.toml" \
  --include="*.rs" \
  --exclude-dir=.git

# If output is empty: BMAD-03 is satisfied
```

### Confirming Rust source untouched

```bash
# Verify no diff in Rust source dirs across all cleanup commits
# (adjust HEAD~N to cover all commits in this phase)
git diff HEAD~4 -- crates/ tests/ examples/ docs/
# Expected output: (empty)
```

### Updating REQUIREMENTS.md checkboxes (not deleting text)

```markdown
# Before:
- [ ] **BMAD-01**: `_bmad-output/` directory removed from repository in a dedicated commit

# After BMAD-01 commit:
- [x] **BMAD-01**: `_bmad-output/` directory removed from repository in a dedicated commit
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `rm -rf` + `git add -A` | `git rm -r` | `git rm` is atomic — one command for both filesystem and index; no risk of "rm'd but still staged" state |

---

## Open Questions

None. All targets are confirmed via direct filesystem inspection. Commit strategy is locked. Verification commands are specified. No ambiguity remains.

---

## Validation Architecture

> `nyquist_validation` key is absent from `.planning/config.json` — treated as enabled. However, this phase contains no executable code and no test framework applies. Validation is git-based.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | None — phase is git operations only |
| Config file | N/A |
| Quick run command | `grep -r "_bmad" . --exclude-dir=.git --include="*.md" --include="*.yaml" --include="*.json"` (must return empty) |
| Full suite command | `git diff HEAD~4 -- crates/ tests/ examples/ docs/` (must return empty) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BMAD-01 | `_bmad-output/` directory no longer exists | smoke | `ls _bmad-output/ 2>&1 | grep "No such file"` | N/A — filesystem check |
| BMAD-02 | `_bmad/` directory no longer exists | smoke | `ls _bmad/ 2>&1 | grep "No such file"` | N/A — filesystem check |
| BMAD-03 | Zero `_bmad` references in tracked files | smoke | `grep -r "_bmad" . --exclude-dir=.git --include="*.md" --include="*.yaml" --include="*.json"` returns empty | N/A — grep check |

### Sampling Rate

- **Per task commit:** Run BMAD-01/02/03 smoke check for the specific task just completed
- **Per wave merge:** All three smoke checks + `git diff HEAD~4 -- crates/ tests/ examples/ docs/`
- **Phase gate:** All checks green before `/gsd:verify-work`

### Wave 0 Gaps

None — no test files to create. All validation is git/shell commands, not automated test suites.

---

## Sources

### Primary (HIGH confidence)

- Direct filesystem inspection — `/Users/esa/git/durable-rust/_bmad-output/`, `_bmad/`, `.claude/skills/` (2026-03-16, first-hand)
- `.planning/phases/02-bmad-cleanup/02-CONTEXT.md` — locked decisions, exact file counts, commit strategy
- `.planning/REQUIREMENTS.md` — canonical requirement definitions BMAD-01, BMAD-02, BMAD-03
- `.planning/STATE.md` — confirmed blockers and constraints for Phase 2
- `grep -r "_bmad" .planning/` output — exact list of 14 files with references

### Secondary (MEDIUM confidence)

- Git documentation — `git rm -r` behavior (well-established git feature, stable across versions)

---

## Metadata

**Confidence breakdown:**
- Removal targets: HIGH — confirmed via `ls` and `find` count
- Commit strategy: HIGH — locked in CONTEXT.md
- Reference list: HIGH — confirmed via grep output
- Rust source safety: HIGH — grep confirmed zero references in `crates/`, `tests/`, `examples/`, `docs/`

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (stable domain — no library versions or APIs involved)
