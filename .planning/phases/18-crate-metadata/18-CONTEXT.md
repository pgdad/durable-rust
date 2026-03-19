# Phase 18: Crate Metadata - Context

**Gathered:** 2026-03-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Make all 6 publishable crates (durable-lambda-core, durable-lambda-closure, durable-lambda-macro, durable-lambda-trait, durable-lambda-builder, durable-lambda-testing) crates.io-ready with complete Cargo.toml metadata, workspace-level version inheritance, and per-crate README documentation. Example crates and test crates are NOT published.

Requirements: META-01, META-02, META-03.

</domain>

<decisions>
## Implementation Decisions

### License choice
- Dual license: MIT OR Apache-2.0 (Rust ecosystem standard)
- SPDX expression in Cargo.toml: `license = "MIT OR Apache-2.0"`
- Create both LICENSE-MIT and LICENSE-APACHE files at repo root

### Workspace inheritance
- Maximum inheritance via `[workspace.package]`
- Inherited fields: version, edition, license, repository, homepage, keywords, categories
- Per-crate only: name, description (unique per crate), readme (unique path per crate)
- Non-publishable crates (examples, tests, compliance) use `publish = false`

### Per-crate README depth
- Standalone comprehensive READMEs for each of the 6 publishable crates
- Each README: purpose, full feature list, multiple usage examples, links to docs.rs and root README
- ~100+ lines per crate, self-contained for crates.io browsing
- Content should cover the crate's specific API style and how it differs from alternatives

### Repository and docs links
- repository = "https://github.com/pgdad/durable-rust"
- homepage = "https://github.com/pgdad/durable-rust"
- documentation = "https://docs.rs/{crate-name}" (auto-generated per crate)

### Claude's Discretion
- Exact README structure and section ordering
- Whether to include badges (crates.io version, docs.rs, license) in READMEs
- Ordering of workspace.package fields
- Whether keywords/categories need per-crate customization or can all be identical

</decisions>

<specifics>
## Specific Ideas

- Keywords already set on all crates: ["aws", "lambda", "durable-execution", "serverless", "workflow"]
- Categories already set: ["api-bindings", "asynchronous"]
- All crates currently at version 0.1.0 — this version will be inherited from workspace
- The macro crate (durable-lambda-macro) is a proc-macro — its README should note this and show the attribute syntax
- The testing crate README should emphasize "no AWS credentials needed" as its key differentiator

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- Root `README.md` exists with project overview — per-crate READMEs can reference it
- `CLAUDE.md` has architecture section with dependency graph — useful for READMEs
- Each crate already has `description`, `keywords`, `categories` in Cargo.toml

### Established Patterns
- Workspace dependencies already managed in root Cargo.toml `[workspace.dependencies]`
- No `[workspace.package]` section exists yet — must be added
- Each crate's Cargo.toml uses `{ path = "../durable-lambda-core" }` for core dependency

### Integration Points
- Root Cargo.toml workspace definition — add `[workspace.package]` section
- Each crate's Cargo.toml — switch to `version.workspace = true`, `license.workspace = true`, etc.
- Non-publishable crates (examples, tests, compliance) — add `publish = false`

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 18-crate-metadata*
*Context gathered: 2026-03-19*
