# Phase 20: CI/CD Automation - Context

**Gathered:** 2026-03-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Create GitHub Actions workflows for automated crate publishing on release tags and publish-readiness validation on PRs. Add CARGO_REGISTRY_TOKEN as a GitHub repository secret.

Requirements: CI-01, CI-02, CI-03.

</domain>

<decisions>
## Implementation Decisions

### Release workflow trigger
- Trigger on `v*` tag push (e.g., `git tag v0.1.0 && git push origin v0.1.0`)
- Run full test suite BEFORE publishing: cargo fmt --check, cargo clippy, cargo test --workspace
- Only publish if all tests pass — prevents publishing broken code
- After successful publish, create a GitHub Release from the tag with auto-generated release notes
- Separate workflow file: `.github/workflows/release.yml`

### Secret management
- User has admin access to the GitHub repository
- Include a checkpoint:human-action task with step-by-step instructions for adding `CARGO_REGISTRY_TOKEN` to GitHub repo secrets
- Same pattern as Phase 19 token setup checkpoint

### PR dry-run check
- Add a new "publish-check" job to the existing `.github/workflows/ci.yml`
- Runs in parallel with existing "check" job
- Uses `scripts/publish.sh --dry-run` (reuse existing script, single source of truth)
- Fails the PR if any crate has metadata errors

### Claude's Discretion
- Whether to use `actions/cache` for the publish-check job
- Exact GitHub Release body format
- Whether the release workflow should use `scripts/publish.sh` or run cargo publish directly (script is preferred for consistency)
- Error handling if GitHub Release creation fails after successful publish

</decisions>

<specifics>
## Specific Ideas

- Existing CI at `.github/workflows/ci.yml` already has: checkout, rust-toolchain, cargo cache, fmt, clippy, build
- `scripts/publish.sh` supports `--dry-run` mode and handles dependency ordering
- Release workflow needs `CARGO_REGISTRY_TOKEN` secret for `cargo publish`
- GitHub auto-generated release notes use commit history since last tag
- Repository: https://github.com/pgdad/durable-rust

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `.github/workflows/ci.yml` — existing PR/push CI with fmt, clippy, build jobs
- `scripts/publish.sh` — dependency-ordered publish with --dry-run mode
- Existing cache configuration in ci.yml (Cargo registry + git + target)

### Established Patterns
- CI uses `dtolnay/rust-toolchain@stable` for Rust setup
- CI uses `actions/cache@v4` with Cargo.lock-based cache key
- CI uses `actions/checkout@v4`

### Integration Points
- `.github/workflows/ci.yml` — add publish-check job
- `.github/workflows/release.yml` — new file for tag-triggered publishing
- GitHub repo settings → Secrets → CARGO_REGISTRY_TOKEN

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 20-ci-cd-automation*
*Context gathered: 2026-03-19*
