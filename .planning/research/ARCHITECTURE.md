# Architecture Research

**Domain:** GSD tooling transition for existing Rust SDK project
**Researched:** 2026-03-16
**Confidence:** HIGH

## Standard Architecture

### System Overview

```
durable-rust/ (git root)
├── .planning/                      GSD project management layer
│   ├── PROJECT.md                  Living project context (requirements, decisions)
│   ├── STATE.md                    Current position + accumulated context
│   ├── config.json                 GSD config (commit_docs, branching_strategy)
│   ├── ROADMAP.md                  Phase definitions (created at milestone start)
│   ├── MILESTONES.md               Shipped milestone archive (created at v1.0 capture)
│   ├── research/                   Research artifacts (this directory)
│   └── phases/                     Per-phase plans and summaries
│       └── 01-name/
│           ├── 01-01-PLAN.md
│           └── 01-01-SUMMARY.md
│
├── crates/                         Rust workspace — SDK production code (unchanged)
│   ├── durable-lambda-core/
│   ├── durable-lambda-macro/
│   ├── durable-lambda-closure/
│   ├── durable-lambda-trait/
│   ├── durable-lambda-builder/
│   └── durable-lambda-testing/
│
├── compliance/                     Python-Rust compliance suite (unchanged)
│   └── rust/
│
├── tests/                          Integration + parity tests (unchanged)
│   ├── e2e/
│   └── parity/
│
├── examples/                       API style examples (unchanged)
│   ├── closure-style/
│   ├── macro-style/
│   ├── trait-style/
│   └── builder-style/
│
├── docs/                           User-facing documentation (unchanged)
│
├── .claude/skills/                 BMAD agent skills dir — remove after transition
│
├── _bmad/                          BMAD tooling — REMOVE in this milestone
│   ├── _config/                    Agent manifests, IDE configs, skill manifests
│   ├── _memory/                    BMAD project memory
│   ├── bmm/                        BMAD method manager agents
│   ├── core/                       BMAD core agent files
│   └── tea/                        BMAD tea framework
│
└── _bmad-output/                   BMAD artifacts — REMOVE in this milestone
    ├── brainstorming/              Brainstorming session notes
    ├── implementation-artifacts/   Per-story implementation specs and retros
    ├── planning-artifacts/         Architecture, PRD, epics from BMAD planning
    │   ├── architecture.md
    │   ├── epics.md
    │   ├── prd.md
    │   └── product-brief-*.md
    ├── project-context.md          BMAD project context (superseded by PROJECT.md)
    └── test-artifacts/             (empty)
```

### Component Responsibilities

| Component | Responsibility | Notes |
|-----------|---------------|-------|
| `.planning/PROJECT.md` | Living requirements, decisions, constraints | Already created for v1.1 |
| `.planning/STATE.md` | Current phase position and accumulated context | Already created |
| `.planning/config.json` | GSD behavior: commit_docs, branching_strategy | Already created (research: true) |
| `.planning/ROADMAP.md` | Phase definitions for current milestone | Created during roadmap phase |
| `.planning/MILESTONES.md` | Archive of shipped milestones | Created when capturing v1.0 |
| `.planning/research/` | Research outputs feeding the roadmap | This directory |
| `.planning/phases/XX/` | Per-plan task lists and summaries | Created per phase during execution |
| `_bmad/` | BMAD tooling config and agents | Delete — no longer used |
| `_bmad-output/` | BMAD planning artifacts | Delete — superseded by .planning/ |

## Recommended Project Structure

After transition, the project root looks like this:

```
durable-rust/
├── .planning/
│   ├── PROJECT.md
│   ├── STATE.md
│   ├── ROADMAP.md
│   ├── MILESTONES.md
│   ├── config.json
│   ├── research/
│   │   ├── SUMMARY.md
│   │   ├── STACK.md
│   │   ├── FEATURES.md
│   │   ├── ARCHITECTURE.md     ← this file
│   │   └── PITFALLS.md
│   └── phases/
│       └── 01-gsd-tooling-transition/
│           ├── 01-01-PLAN.md
│           └── 01-01-SUMMARY.md
├── .claude/
│   └── skills/                  (BMAD skills, remove if unused post-transition)
├── .github/
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── README.md
├── crates/
├── compliance/
├── tests/
├── examples/
└── docs/
```

### Structure Rationale

- **`.planning/`:** GSD keeps all project management in one directory, tracked by git alongside code. Separates tooling metadata from SDK deliverables.
- **`crates/`:** Rust workspace crates — untouched by this milestone. No changes to SDK code.
- **BMAD directories removed:** `_bmad/` and `_bmad-output/` have no runtime dependency on any SDK code. Pure tooling overhead.

## Architectural Patterns

### Pattern 1: Separation of Tooling from Deliverables

**What:** GSD `.planning/` sits beside the Rust workspace without touching it. The `crates/`, `tests/`, `examples/`, `compliance/`, and `docs/` directories are pure SDK deliverables. Planning infrastructure is additive and isolated.

**When to use:** Always — this is the core GSD contract. Planning files never live inside `src/`, `crates/`, or any deliverable directory.

**Trade-offs:** Clean separation means no accidental inclusion of planning files in published crates. The slight cost is an extra directory at root — negligible.

### Pattern 2: Git-Tracked Planning Artifacts

**What:** `.planning/` files are committed to git. `config.json` currently has `workflow.research: true`, and commit_docs defaults to `true`. This means STATE.md, ROADMAP.md, and phase plans are all in version history.

**When to use:** This project — team uses Claude Code with git as the primary context source. Git history becomes the AI's context for future sessions.

**Trade-offs:** Planning documents visible to anyone with repo access. For this project (internal tooling transition) that is appropriate. If docs should be private, set `commit_docs: false` and gitignore `.planning/`.

### Pattern 3: Milestone Archive Before Removal

**What:** Before removing BMAD artifacts, capture the v1.0 milestone in `.planning/MILESTONES.md`. The BMAD `_bmad-output/implementation-artifacts/` and `_bmad-output/planning-artifacts/` represent the full v1.0 design history. That value should be summarized before removal, not just deleted.

**When to use:** Any time significant planning history exists in the old tooling. One-time capture at transition point.

**Trade-offs:** Takes a few minutes to write MILESTONES.md. The alternative — deleting without capture — loses institutional knowledge that informed the SDK design.

## Data Flow

### GSD Workflow Flow

```
Research (this phase)
    ↓
ROADMAP.md creation (phase definitions)
    ↓
Phase execution loop:
  plan-phase → PLAN.md + tasks defined
      ↓
  execute-phase → tasks committed per-task
      ↓
  STATE.md updated → next phase
    ↓
complete-milestone → MILESTONES.md entry
```

### BMAD Removal Flow

```
_bmad-output/project-context.md → already superseded by .planning/PROJECT.md
_bmad-output/planning-artifacts/ → summarize value in MILESTONES.md, then delete
_bmad-output/implementation-artifacts/ → historical only, delete
_bmad/ → tooling only, delete
.claude/skills/ → verify not referenced, delete if BMAD-only
```

### Key Data Flows

1. **Research to Roadmap:** SUMMARY.md phase recommendations feed directly into ROADMAP.md phase names and ordering. PITFALLS.md flags phases that need deeper research.
2. **STATE.md as session anchor:** Every new Claude Code session reads STATE.md first to recover position without re-reading all files. STATE.md references PROJECT.md for current requirements.
3. **Git as context layer:** Per-task commits tagged `feat({phase}-{plan})` let future Claude sessions reconstruct what was done without reading every PLAN.md and SUMMARY.md.

## BMAD Removal Strategy

### Safety Assessment

The BMAD directories have zero runtime coupling to the SDK:

| BMAD Directory | Coupling to Rust Code | Safe to Remove |
|---------------|----------------------|----------------|
| `_bmad/` | None — agent prompts and config only | YES |
| `_bmad-output/` | None — markdown artifacts only | YES |
| `.claude/skills/` | None — BMAD skill definitions | YES (verify first) |

No Rust source files import from, read, or reference any `_bmad*` path. Cargo.toml does not reference these directories. The `.gitignore` does not protect them. Removing them is a pure filesystem operation.

### Recommended Removal Order

**Step 1: Capture v1.0 milestone in MILESTONES.md (before deletion)**

Create `.planning/MILESTONES.md` with a v1.0 entry summarizing what shipped. Source the key data from `_bmad-output/planning-artifacts/epics.md` and `_bmad-output/implementation-artifacts/` before they are gone.

This is the only step where BMAD artifacts provide value that GSD needs to preserve.

**Step 2: Remove _bmad-output/ first**

```bash
git rm -r _bmad-output/
git commit -m "chore: remove BMAD output artifacts (superseded by .planning/)"
```

Remove output artifacts first because they are the larger directory and contain no tooling config. This commit is clean and easily reversible if needed.

**Step 3: Remove _bmad/ second**

```bash
git rm -r _bmad/
git commit -m "chore: remove BMAD tooling (superseded by GSD)"
```

Separate commit from step 2 so the tooling removal is distinct from artifact removal in git history. If the team ever wants to audit what BMAD looked like, the commit boundary makes it clear which was which.

**Step 4: Verify .claude/skills/ (conditional)**

```bash
ls /Users/esa/git/durable-rust/.claude/skills/
```

If these are BMAD agent skill definitions, remove them in a third commit. If they contain non-BMAD project-specific skills, keep and review individually.

**Step 5: Verify .gitignore needs no update**

The current `.gitignore` only ignores `/target/`, IDE files, and OS files. Neither `_bmad/` nor `_bmad-output/` are gitignored — they are tracked. Using `git rm -r` (not plain `rm`) handles removal from both filesystem and git index in one step.

### Why Separate Commits for Each Directory

PROJECT.md already records the key decision: "Remove BMAD artifacts in separate commit — clean separation of concerns in git history." Two separate commits (one per directory) refines this — if a future developer asks "when did the planning artifacts leave?", `git log --oneline` will show two distinct events rather than one undifferentiated "remove BMAD stuff" commit.

## Integration Points

### GSD ↔ Git Integration

| Event | Git action | Notes |
|-------|------------|-------|
| Research complete | No commit — orchestrator handles | This file is intermediate |
| ROADMAP.md created | `docs: initialize durable-rust milestone v1.1` | Commit .planning/ together |
| Each task completed | `feat({phase}-{plan}): {task}` | Code files only |
| Each plan completed | `docs({phase}-{plan}): complete {plan} plan` | Planning files only |
| Milestone complete | `docs: complete milestone v1.1` | STATE.md + MILESTONES.md |

### GSD ↔ Rust Workspace Integration

GSD has no build-time integration with the Rust workspace. The `.planning/` directory is not a Cargo workspace member and will never be. The only relationship is:

- GSD phases describe what to build in the Rust workspace
- GSD commits reference the same git repository
- Claude Code sessions read `.planning/STATE.md` as session context, then read Rust source as needed

### .claude/ Directory Boundary

The `.claude/skills/` directory under the project root is a BMAD artifact (agent skill definitions). It is distinct from `~/.claude/` (user-level Claude configuration) and `~/.claude/get-shit-done/` (GSD installation). Removing `.claude/skills/` from the project root does not affect GSD functionality.

## GSD Artifact Creation Order

For this milestone, artifacts should be created in this sequence:

1. **Research phase (current):** SUMMARY.md, STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md — feeds roadmap
2. **Roadmap phase:** ROADMAP.md — defines phases based on research
3. **Phase 1 execution:** MILESTONES.md (v1.0 capture) — before BMAD removal
4. **Phase 2 execution:** `git rm -r _bmad-output/` — remove output artifacts
5. **Phase 3 execution:** `git rm -r _bmad/` — remove tooling
6. **Milestone completion:** STATE.md final update, MILESTONES.md v1.1 entry

This ordering ensures: (a) research informs the roadmap, (b) v1.0 history is captured before deletion, (c) each removal is an isolated, reversible git commit.

## Anti-Patterns

### Anti-Pattern 1: Deleting BMAD Before Capturing v1.0 Milestone

**What people do:** Run `git rm -r _bmad*` immediately and commit, treating it as pure cleanup.

**Why it's wrong:** `_bmad-output/planning-artifacts/` contains the PRD, architecture decisions, and epic breakdown that informed the entire v1.0 SDK. Once deleted, the rationale for SDK design choices (why blake2b for operation IDs, why 4 API styles, why compliance/ is separate) becomes undiscoverable from git history.

**Do this instead:** Write MILESTONES.md with a v1.0 entry that summarizes key accomplishments and references the design decisions, then delete.

### Anti-Pattern 2: Merging Both BMAD Directories in One Commit

**What people do:** `git rm -r _bmad/ _bmad-output/` in one command and commit.

**Why it's wrong:** Loses the distinction between "the output artifacts we produced" and "the tooling we used." If the team ever wants to understand when tooling was swapped, a single commit conflates two different concerns.

**Do this instead:** Two commits — one per directory — as described in the removal order above.

### Anti-Pattern 3: Adding .planning/ to .gitignore

**What people do:** Treat planning docs as ephemeral and gitignore them.

**Why it's wrong:** This team uses Claude Code as a primary coding assistant. Planning docs in git = Claude has full project history available in future sessions without needing manual context injection. `commit_docs: true` (the default) is the right setting for this team profile.

**Do this instead:** Keep `.planning/` tracked. The config.json already reflects the correct default.

## Sources

- Direct inspection of `/Users/esa/git/durable-rust/` project structure (HIGH confidence)
- Direct inspection of `_bmad/` and `_bmad-output/` directory trees (HIGH confidence)
- GSD reference docs: `~/.claude/get-shit-done/references/planning-config.md` (HIGH confidence)
- GSD reference docs: `~/.claude/get-shit-done/references/git-integration.md` (HIGH confidence)
- GSD reference docs: `~/.claude/get-shit-done/references/git-planning-commit.md` (HIGH confidence)
- `.planning/PROJECT.md` — confirmed key decisions about separate BMAD removal commit (HIGH confidence)

---
*Architecture research for: GSD tooling transition (durable-rust)*
*Researched: 2026-03-16*
