# Roadmap: durable-rust

## Milestones

- ✅ **v1.0 Production Hardening** - Phases 1-9 (shipped 2026-03-17)
- ✅ **v1.1 AWS Integration Testing** - Phases 10-17 (shipped 2026-03-19)
- 🚧 **v1.2 Crates.io Publishing** - Phases 18-20 (in progress)

## Phases

<details>
<summary>✅ v1.0 Production Hardening (Phases 1-9) - SHIPPED 2026-03-17</summary>

9 phases, 23 plans completed. Shipped comprehensive test coverage (100+ tests), input validation, structured error codes, operation-level observability (tracing spans), batch checkpoint optimization, saga/compensation pattern, step timeout, conditional retry, proc-macro type validation, and documentation overhaul.

</details>

<details>
<summary>✅ v1.1 AWS Integration Testing (Phases 10-17) - SHIPPED 2026-03-19</summary>

8 phases completed. Deployed all 48 example handlers as Lambda functions against real AWS, validated every SDK operation end-to-end with an automated test harness (48/48 tests passing: 32 real + 16 XFAIL). Delivered Terraform infrastructure, Docker build pipeline with cargo-chef caching, and comprehensive service limitation documentation.

</details>

### 🚧 v1.2 Crates.io Publishing (In Progress)

**Milestone Goal:** Establish a complete crate publishing pipeline from local dry-run to automated GitHub Actions release, making all 6 SDK crates available on crates.io.

- [x] **Phase 18: Crate Metadata** - All 6 publishable crates have complete, accurate Cargo.toml metadata and per-crate READMEs (completed 2026-03-19)
- [x] **Phase 19: Publishing Infrastructure** - crates.io token obtained, dependency-ordered publish script passes dry-run for all crates (completed 2026-03-19)
- [x] **Phase 20: CI/CD Automation** - GitHub Actions workflow publishes crates on release tags and validates dry-run on every PR (completed 2026-03-19)

## Phase Details

### Phase 18: Crate Metadata
**Goal**: All 6 publishable crates are ready for crates.io submission with complete Cargo.toml metadata, consistent versioning, and per-crate documentation pages
**Depends on**: Nothing (first phase of milestone)
**Requirements**: META-01, META-02, META-03
**Success Criteria** (what must be TRUE):
  1. Every publishable crate Cargo.toml contains license, repository, homepage, readme, documentation, description, categories, and keywords fields with no placeholder values
  2. All 6 crates share version 0.1.0 managed via workspace-level `[workspace.package]` version inheritance — bumping the root version propagates to all crates
  3. Each of the 6 crates has a README.md in its crate directory that renders correctly on crates.io (includes crate purpose, usage snippet, and links to docs.rs)
  4. `cargo metadata --no-deps` for each crate shows no missing required fields and no publish-blocking warnings
**Plans:** 2/2 plans complete
Plans:
- [ ] 18-01-PLAN.md — Workspace metadata, license files, and Cargo.toml updates for all crates
- [ ] 18-02-PLAN.md — Per-crate README.md files for crates.io rendering

### Phase 19: Publishing Infrastructure
**Goal**: A developer can validate all 6 crates locally with a single dry-run command, and publish them in dependency order with a single publish command once credentials are in place
**Depends on**: Phase 18
**Requirements**: PUB-01, PUB-02, PUB-03, PUB-04
**Success Criteria** (what must be TRUE):
  1. `cargo publish --dry-run` passes for all 6 crates individually without errors or warnings
  2. Running `scripts/publish.sh --dry-run` executes dry-run publishes in dependency order (core → macro → closure/trait/builder → testing) and exits 0 only if all 6 pass
  3. Running `scripts/publish.sh` (without --dry-run) publishes all 6 crates to crates.io in the correct order, waiting for crates.io indexing between each publish
  4. `~/.cargo/credentials.toml` contains a valid crates.io API token that `cargo publish` can use without additional flags
**Plans:** 2/2 plans complete
Plans:
- [ ] 19-01-PLAN.md — Create dependency-ordered publish script with dry-run validation
- [ ] 19-02-PLAN.md — Obtain crates.io API token (human checkpoint)

### Phase 20: CI/CD Automation
**Goal**: Pushing a release tag to GitHub triggers automated publishing of all crates, and every PR validates that crate metadata is publish-ready before merging
**Depends on**: Phase 19
**Requirements**: CI-01, CI-02, CI-03
**Success Criteria** (what must be TRUE):
  1. Pushing a tag matching `v*` to the GitHub repository triggers a GitHub Actions workflow that publishes all 6 crates to crates.io in dependency order
  2. The crates.io API token is stored as a GitHub repository secret (`CARGO_REGISTRY_TOKEN`) and the workflow references it without exposing it in logs
  3. Every PR to main triggers a CI check that runs `cargo publish --dry-run` for all 6 crates and fails the PR if any crate has metadata errors or missing fields
  4. A developer can trigger a release by creating and pushing a version tag (e.g., `git tag v0.1.0 && git push origin v0.1.0`) with no other manual steps required
**Plans:** 2/2 plans complete
Plans:
- [ ] 20-01-PLAN.md — Create release workflow and add publish-check job to CI
- [ ] 20-02-PLAN.md — Add CARGO_REGISTRY_TOKEN to GitHub repository secrets

## Progress

**Execution Order:** 18 -> 19 -> 20

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 10. Tooling and Prerequisites | v1.1 | 1/1 | Complete | 2026-03-17 |
| 11. Infrastructure | v1.1 | 3/3 | Complete | 2026-03-17 |
| 12. Docker Build Pipeline | v1.1 | 2/2 | Complete | 2026-03-17 |
| 13. Test Harness | v1.1 | 1/1 | Complete | 2026-03-17 |
| 14. Synchronous Operation Tests | v1.1 | 1/1 | Complete | 2026-03-18 |
| 15. Async Operation Tests | v1.1 | 2/2 | Complete | 2026-03-18 |
| 16. Advanced Feature Tests | v1.1 | 2/2 | Complete | 2026-03-17 |
| 17. Documentation | v1.1 | 0/TBD | Not started | - |
| 18. Crate Metadata | 2/2 | Complete    | 2026-03-19 | - |
| 19. Publishing Infrastructure | 2/2 | Complete    | 2026-03-19 | - |
| 20. CI/CD Automation | 2/2 | Complete    | 2026-03-19 | - |
