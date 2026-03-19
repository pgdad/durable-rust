# Requirements: durable-rust

**Defined:** 2026-03-19
**Core Value:** Enable Rust teams to write durable Lambda handlers with 4-8x lower memory and zero behavioral divergence from the official SDK

## v1.2 Requirements

Requirements for Crates.io Publishing milestone. Each maps to roadmap phases.

### Crate Metadata

- [x] **META-01**: All 6 publishable crates have required Cargo.toml fields (license, repository, homepage, readme, documentation)
- [x] **META-02**: All crates use consistent version (0.1.0) with workspace-level version management
- [x] **META-03**: Each crate has a crate-level README.md suitable for crates.io rendering

### Publishing Infrastructure

- [x] **PUB-01**: crates.io API token obtained and stored securely (local `~/.cargo/credentials.toml`)
- [x] **PUB-02**: Publish script handles dependency-ordered publishing (core → macro → closure/trait/builder → testing)
- [x] **PUB-03**: Publish script supports `--dry-run` mode for validation without actual publishing
- [x] **PUB-04**: `cargo publish --dry-run` passes for all 6 crates

### CI/CD

- [x] **CI-01**: GitHub Actions workflow publishes all crates on release tag push (e.g., `v*`)
- [x] **CI-02**: crates.io API token stored as GitHub repository secret
- [x] **CI-03**: CI workflow validates `cargo publish --dry-run` on every PR (catches metadata issues early)

## v1.3 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Release Automation

- **REL-01**: Automated changelog generation from commit messages
- **REL-02**: Version bump script that updates all workspace crates atomically
- **REL-03**: Pre-release validation checklist (tests, clippy, fmt, dry-run)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Publishing example crates | Examples are not library crates; they demonstrate usage, not provide API |
| Changelog generation | Manual for v1.2; automate in v1.3 |
| Semantic versioning enforcement | Too early; workspace at 0.1.0, breaking changes expected |
| crates.io badge in README | Nice-to-have, not a requirement |
| Workspace inheritance for all fields | Only version needs workspace-level management for now |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| META-01 | Phase 18 | Complete |
| META-02 | Phase 18 | Complete |
| META-03 | Phase 18 | Complete |
| PUB-01 | Phase 19 | Complete |
| PUB-02 | Phase 19 | Complete |
| PUB-03 | Phase 19 | Complete |
| PUB-04 | Phase 19 | Complete |
| CI-01 | Phase 20 | Complete |
| CI-02 | Phase 20 | Complete |
| CI-03 | Phase 20 | Complete |

**Coverage:**
- v1.2 requirements: 10 total
- Mapped to phases: 10
- Unmapped: 0

---
*Requirements defined: 2026-03-19*
*Last updated: 2026-03-19 after roadmap creation (all 10 requirements mapped)*
