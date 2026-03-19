# Phase 19: Publishing Infrastructure - Context

**Gathered:** 2026-03-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Create a publish script (scripts/publish.sh) with dry-run mode that validates and publishes all 6 crates in dependency order. Obtain crates.io API token. Verify all 6 crates publish successfully.

Requirements: PUB-01, PUB-02, PUB-03, PUB-04.

</domain>

<decisions>
## Implementation Decisions

### Token acquisition flow
- User does NOT yet have a crates.io account — needs to be created via GitHub OAuth
- Plan includes a checkpoint task with step-by-step instructions for:
  1. Creating crates.io account at https://crates.io via GitHub OAuth
  2. Generating API token at https://crates.io/settings/tokens
  3. Running `cargo login` to store token in ~/.cargo/credentials.toml
- This is a human-action checkpoint — plan pauses until user completes it

### Publish script failure handling
- Abort immediately on first failure (set -e / exit on error)
- Already-published crates remain on crates.io (can't unpublish)
- On re-run, script should detect already-published versions and skip them
- 30-second fixed wait between publishes for crates.io indexing

### Dry-run scope
- `--dry-run` runs only `cargo publish --dry-run` for each crate in dependency order
- No additional checks (tests, clippy, fmt) — those are CI concerns (Phase 20)
- Dry-run does NOT require a crates.io API token
- Exit 0 only if all 6 crates pass dry-run

### Claude's Discretion
- Script output format (progress messages, colors, timing)
- How to detect already-published versions (query crates.io API or parse cargo output)
- Whether to add --allow-dirty flag support
- Script help text and usage instructions

</decisions>

<specifics>
## Specific Ideas

- Dependency order: core → macro → closure/trait/builder (parallel-safe but sequential for simplicity) → testing
- Script location: scripts/publish.sh (consistent with existing scripts/ directory)
- All 6 crates currently at version 0.1.0
- Repository URL: https://github.com/pgdad/durable-rust

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `scripts/` directory already has build-images.sh, test-all.sh, deploy-all.sh — established script patterns
- Root Cargo.toml has [workspace.package] with version = "0.1.0" (Phase 18)
- All 6 publishable crate Cargo.tomls have complete metadata (Phase 18)

### Established Patterns
- Scripts use `set -euo pipefail`, SCRIPT_DIR, REPO_ROOT variables
- Scripts source helper libraries (test-helpers.sh pattern)
- AWS scripts use profile/region flags — publish script is simpler (no AWS)

### Integration Points
- ~/.cargo/credentials.toml — cargo login writes the API token here
- crates.io registry — cargo publish pushes to this
- Phase 20 will reference this script from GitHub Actions

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 19-publishing-infrastructure*
*Context gathered: 2026-03-19*
