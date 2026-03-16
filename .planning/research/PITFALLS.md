# Pitfalls Research

**Domain:** Project management tooling transition (BMAD to GSD) on an existing Rust SDK
**Researched:** 2026-03-16
**Confidence:** HIGH — based on direct inspection of this repository's actual structure and artifacts

---

## Critical Pitfalls

### Pitfall 1: Deleting Rust Source Code or Tests While Removing BMAD Artifacts

**What goes wrong:**
A developer (or AI agent) executing a broad "remove BMAD" deletion command accidentally includes Rust source files, test data, or compiled artifacts. `_bmad-output/implementation-artifacts/tests/` sits next to `.rs` test files; a glob like `rm -rf *tests*` or an overly broad path could hit `/tests/` at the repo root.

**Why it happens:**
BMAD directories are at the repo root level alongside `crates/`, `tests/`, `examples/`, and `docs/`. Tooling agents working with natural-language instructions like "delete all BMAD artifacts" may expand scope beyond the two target directories. AI coding assistants in particular can misinterpret ambiguous removal instructions.

**How to avoid:**
Delete by exact absolute path only. Never use globs or recursive deletes on partial directory names. The only two directories to remove are:
- `/Users/esa/git/durable-rust/_bmad/`
- `/Users/esa/git/durable-rust/_bmad-output/`

Verify before deletion with `ls -la _bmad/ _bmad-output/` and confirm no Rust source appears in the listing.

**Warning signs:**
- Removal command contains a wildcard (`*bmad*`, `*artifact*`, `*output*`)
- Removal operates on a relative path rather than an absolute path
- `cargo test` fails after deletion with "file not found" errors

**Phase to address:**
The phase that removes BMAD artifacts — must encode exact paths as acceptance criteria, not descriptive language.

---

### Pitfall 2: Breaking Git History by Force-Pushing or Squashing After BMAD Removal

**What goes wrong:**
To "clean up" the repo history, a developer amends the initial GSD commit or force-pushes to main, eliminating the BMAD-era commits. This destroys the audit trail of what was shipped as v1.0 and makes `git bisect` or blame unreliable.

**Why it happens:**
The BMAD output directories contain 30+ implementation artifact files that were committed as part of the project. When those files disappear, it can feel natural to "tidy" history. The PROJECT.md explicitly notes "Remove BMAD artifacts in separate commit" as a pending decision — if that decision is forgotten mid-execution, history may be rewritten instead.

**How to avoid:**
Remove BMAD in a dedicated, forward-only commit. Never amend, rebase-squash, or force-push the removal. The git log entry for the removal should be explicit (e.g., `chore: remove BMAD tooling artifacts`). Treat this as a one-way door — the history remains, the files are gone.

**Warning signs:**
- Any use of `git commit --amend`, `git rebase -i`, or `git push --force` in the context of this transition
- The removal being bundled into an unrelated commit (e.g., combined with a GSD setup commit)

**Phase to address:**
The phase that removes BMAD artifacts — must specify "separate commit, no force push" as an explicit constraint.

---

### Pitfall 3: Leaving Orphaned Cross-References to BMAD Artifacts

**What goes wrong:**
After `_bmad/` and `_bmad-output/` are deleted, markdown documents, commit messages, or GSD planning files still reference paths like `_bmad-output/planning-artifacts/architecture.md`. These become dead links. Future agents following those links fail silently or produce incorrect results.

**Why it happens:**
The BMAD `architecture.md` and `epics.md` files contain the authoritative design rationale for the v1.0 SDK — they were referenced throughout the project's implementation. The GSD `PROJECT.md` doesn't yet contain equivalent captured context from those documents.

**How to avoid:**
Before deleting BMAD directories, extract and migrate any still-relevant content (key architectural decisions, rationale, constraints) into `.planning/PROJECT.md` or dedicated GSD context files. Run a grep for `_bmad` across the entire repo after deletion to catch any remaining references.

**Warning signs:**
- `grep -r "_bmad" .` returns hits in `.planning/`, `docs/`, `README.md`, or any `.md` file after deletion
- `PROJECT.md` does not capture the 7-crate workspace rationale or the core replay engine design decisions

**Phase to address:**
The phase that sets up GSD infrastructure — capture critical context before removal happens.

---

### Pitfall 4: GSD Config or STATE.md Becoming Inconsistent with Reality

**What goes wrong:**
`.planning/STATE.md` or `.planning/config.json` is not updated after a phase completes, or is updated to reflect a state that hasn't actually been reached yet. AI agents reading STATE.md assume setup is complete and skip steps; or they re-run setup steps that are already done, potentially overwriting existing research files.

**Why it happens:**
GSD tooling relies on STATE.md as the single source of truth for where the project is. The current STATE.md already records "(First GSD milestone — no prior context)" which is accurate — but as phases progress, keeping this file synchronized requires discipline. AI agents are especially prone to optimistic state updates.

**How to avoid:**
Treat STATE.md as a write-at-completion artifact: only update it after a phase's acceptance criteria are verified, not when work begins. Include a STATE.md update step in every phase's definition of done.

**Warning signs:**
- STATE.md says "Phase X complete" but acceptance criteria haven't been verified
- Research files exist in `.planning/research/` but STATE.md still says "Not started"
- Multiple agents updating STATE.md concurrently without coordination

**Phase to address:**
Every phase — each phase should have an explicit "update STATE.md" step as its final action.

---

### Pitfall 5: Treating This Milestone as a Code Change and Running Cargo

**What goes wrong:**
An agent or developer runs `cargo build`, `cargo test`, or `cargo clippy` as part of the tooling transition, expecting them to validate the work. These commands have no bearing on whether GSD infrastructure is correctly set up and take significant time in a 7-crate workspace. Worse, if Rust toolchain or dependencies are in a transient state, false failures distract from the actual transition work.

**Why it happens:**
The team is accustomed to validating changes with `cargo test`. The transition milestone touches only planning infrastructure (`.planning/`, removing `_bmad/`, `_bmad-output/`), not Rust source. The instinct to "make sure nothing broke" is correct but manifests as the wrong check.

**How to avoid:**
Define explicit, non-Rust acceptance criteria for this milestone:
- `.planning/` structure is correct
- `_bmad/` and `_bmad-output/` no longer exist
- `git log --oneline` shows the expected commit sequence
- No broken references remain

Do not include `cargo test` in this milestone's definition of done.

**Warning signs:**
- Phase acceptance criteria mention `cargo test`
- An agent is waiting on a multi-minute `cargo build` as part of tooling verification

**Phase to address:**
Roadmap definition phase — acceptance criteria must be planning-only.

---

### Pitfall 6: Accidentally Creating GSD Files Outside `.planning/`

**What goes wrong:**
Research, roadmap, or config files land in the repo root or in `_bmad-output/` rather than in `.planning/`. This creates a second source of truth problem and defeats GSD's convention of using `.planning/` as the single planning directory.

**Why it happens:**
AI agents spawned from orchestrators may have working directory assumptions that differ from the repo root, or may interpret a relative path instruction incorrectly. This project already has an unusual layout (`.planning/` was created in the last commit; `_bmad-output/` still exists and contains `planning-artifacts/`).

**How to avoid:**
All GSD file writes must use absolute paths anchored to `/Users/esa/git/durable-rust/.planning/`. Verify after each phase that no new `.md` or `.json` files appeared at the repo root.

**Warning signs:**
- `ls /Users/esa/git/durable-rust/` shows new `.md` files at root
- Research files appear inside `_bmad-output/` directories
- `config.json` or `STATE.md` is duplicated at root

**Phase to address:**
Every phase — enforce absolute paths in all file write operations.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Leaving `_bmad/` in place "temporarily" | Avoids risk of deleting something important | BMAD and GSD tools both scan the repo; agents get confused by conflicting state files | Never — set a clear deletion milestone |
| Copying BMAD planning artifacts into GSD dirs wholesale | Preserves all context | GSD files in wrong format; future agents produce malformed output based on BMAD templates | Never — migrate selectively, format correctly |
| Skipping STATE.md updates to "go faster" | Saves one step per phase | Next agent reads stale state and re-does completed work or skips required work | Never for this milestone (low phase count) |
| Bundling BMAD removal with GSD setup in one commit | Fewer commits | Impossible to bisect whether a problem came from setup or removal; violates PROJECT.md's explicit decision | Never — keep commits separate |

---

## Integration Gotchas

Common mistakes when connecting BMAD-era context to the new GSD workflow.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| BMAD `sprint-status.yaml` → GSD state | Treating YAML story status as authoritative for what's "done" in GSD | Use PROJECT.md Validated list as the source of truth; the YAML is historical record only |
| BMAD `architecture.md` → GSD context | Linking to the BMAD file path after deletion | Extract key decisions into `.planning/PROJECT.md` before deletion |
| BMAD `epics.md` → GSD roadmap | Reusing BMAD epic structure as GSD phases | GSD phases have different granularity; redesign rather than rename |
| `_bmad-output/implementation-artifacts/` → git | Assuming these files must be preserved post-transition | They were planning scaffolding; the Rust code is the artifact — files can be deleted |

---

## Performance Traps

Not applicable to a pure tooling transition milestone. No scale concerns exist for `.planning/` file operations or directory removal.

---

## Security Mistakes

Not applicable to this domain. No credentials, secrets, or access controls are involved in BMAD-to-GSD migration.

---

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **GSD setup:** `.planning/` directory exists — verify it also contains `PROJECT.md`, `STATE.md`, `config.json`, and at minimum a `research/` subdirectory
- [ ] **BMAD removal:** `_bmad/` is gone — verify `_bmad-output/` is also gone (they are separate directories)
- [ ] **Orphan cleanup:** BMAD directories deleted — verify `grep -r "_bmad" .` returns zero hits outside of git history
- [ ] **Git history:** Removal committed — verify it is a standalone commit, not amended into another commit
- [ ] **Context preserved:** Key BMAD design rationale — verify `.planning/PROJECT.md` captures workspace structure, crate dependency graph, and replay engine design decisions before `_bmad-output/planning-artifacts/architecture.md` is deleted
- [ ] **STATE.md current:** Phase marked complete — verify each acceptance criterion is met, not just that the files exist

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Rust source accidentally deleted | HIGH | `git restore` the deleted files immediately; verify with `cargo test`; document what happened |
| Force-push destroyed history | HIGH | If remote still has old refs (GitHub reflog), recover via `git fetch` and `git reset`; if not recoverable, document the gap in PROJECT.md |
| Orphaned references in docs | LOW | Run `grep -r "_bmad" .` and fix each hit; add a search step to the phase's acceptance criteria |
| GSD files created in wrong location | LOW | Move files to `.planning/` with `mv`; update any internal cross-references; re-commit |
| STATE.md out of sync | LOW | Read actual directory/file state, reconcile with STATE.md, update with correct information |
| BMAD context lost before migration | MEDIUM | Recover from git history (`git show <commit>:<path>`) — BMAD files will still be in git history after deletion from working tree |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Deleting Rust source during BMAD removal | Phase: Remove BMAD artifacts | `cargo check` passes after deletion; `ls crates/ tests/ examples/` unchanged |
| Breaking git history | Phase: Remove BMAD artifacts | `git log --oneline` shows removal as standalone commit; no force-push |
| Orphaned cross-references | Phase: Set up GSD infrastructure (before removal) | `grep -r "_bmad" .` returns zero hits |
| STATE.md inconsistency | Every phase (final step) | STATE.md reflects actual file system state at phase end |
| Unnecessary cargo runs | Roadmap definition | Acceptance criteria contain no `cargo` commands |
| GSD files in wrong location | Every phase (file write step) | `ls /Users/esa/git/durable-rust/` shows no new `.md` files at root |
| Lost BMAD context | Phase: Set up GSD infrastructure (before removal) | PROJECT.md contains 7-crate structure, replay engine design decisions, and dependency graph |

---

## Sources

- Direct inspection of `/Users/esa/git/durable-rust/_bmad/` and `_bmad-output/` directory trees (2026-03-16)
- `/Users/esa/git/durable-rust/.planning/PROJECT.md` — milestone scope and constraints
- `/Users/esa/git/durable-rust/_bmad-output/implementation-artifacts/sprint-status.yaml` — BMAD state structure
- `/Users/esa/git/durable-rust/_bmad-output/planning-artifacts/architecture.md` — cross-reference risk assessment
- Git log analysis — commit history and separation-of-concerns decisions
- GSD template structure at `/Users/esa/.claude/get-shit-done/templates/research-project/`

---
*Pitfalls research for: BMAD to GSD tooling transition on durable-rust*
*Researched: 2026-03-16*
