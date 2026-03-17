# Phase 9: Documentation Overhaul - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning
**Source:** Auto-generated from prior context, codebase analysis, and requirement specs

<domain>
## Phase Boundary

Update README.md, CLAUDE.md, migration guide, inline rustdoc, and Cargo.toml metadata to cover new features from Phases 1-8. No code changes — documentation only. All 10 DOCS requirements are content additions to existing files.

</domain>

<decisions>
## Implementation Decisions

### README structure additions
- "Determinism Rules" section placed after "Operations Guide" — do/don't code examples showing Uuid::new_v4() outside step (wrong) vs inside step (right), Utc::now() pattern, rand::random() pattern
- Error handling example placed in "Quick Start" or new "Error Handling" section — show the three-arm match: `Ok(Ok(v))`, `Ok(Err(business_err))`, `Err(durable_err)`
- Troubleshooting FAQ as its own section near bottom — cover the 3 most common compile errors: `Send + 'static` bounds on parallel/map closures, missing `Serialize + DeserializeOwned` on result types, mandatory type annotations on step results
- Link to `_bmad-output/project-context.md` in a "Contributing / Implementation Rules" section

### Migration guide additions
- Determinism anti-patterns section with Python-equivalent gotchas
- Show Python `datetime.now()` outside activity → Rust `ctx.step("now", ...)` inside step pattern

### Inline documentation improvements
- `BatchResult<T>` in `types.rs` — add rustdoc example showing `BatchItemStatus::Succeeded` vs `Failed` per-item checking
- Parallel example in README — add inline comment explaining why `Box<dyn FnOnce(...) -> Pin<Box<dyn Future<...>>>>` boxing is needed (trait object for heterogeneous branch closures)
- Callback documentation — add ASCII diagram showing two separate operation IDs for `create_callback` and `callback_result`

### CLAUDE.md updates
- Note that Phase 3 introduced `DurableContextOps` trait — changes to context methods now go in one place
- Document new features: step timeout, conditional retry, batch checkpoint, saga/compensation
- Update architecture section to reflect current state

### Cargo.toml metadata
- Add `description`, `keywords`, `categories` to all 6 crate Cargo.toml files
- Keywords: `aws`, `lambda`, `durable-execution`, `serverless`, `workflow`
- Categories: `api-bindings`, `asynchronous`
- Description: one-line per crate matching its purpose

### Claude's Discretion
- Exact wording of troubleshooting FAQ answers
- Whether to add a "New in v2" section to README summarizing Phases 1-8 features
- Migration guide formatting (tables vs code blocks for anti-patterns)
- Whether Cargo.toml gets `license` and `repository` fields (internal project)

</decisions>

<code_context>
## Existing Code Insights

### Files to Modify
- `README.md` — 431 lines, well-structured with sections for features, quick start, API styles, operations guide, testing, project structure
- `CLAUDE.md` — 84 lines, build commands + architecture + critical rules
- `docs/migration-guide.md` — Python-to-Rust migration guide
- `_bmad-output/project-context.md` — 38 implementation rules (target of cross-reference link)
- `crates/*/Cargo.toml` — 6 crate Cargo.toml files needing metadata
- `crates/durable-lambda-core/src/types.rs` — BatchResult rustdoc location

### Established Patterns
- README uses GitHub-flavored markdown with tables, code blocks, and section anchors
- Existing code examples in README are syntactically correct and follow project conventions
- CLAUDE.md uses terse, actionable bullet points — not prose paragraphs

### Integration Points
- README "Operations Guide" section — insert determinism rules after this
- README bottom — add troubleshooting FAQ section
- CLAUDE.md "Architecture" section — update with DurableContextOps trait info
- Migration guide — append determinism section

</code_context>

<specifics>
## Specific Ideas

- Determinism rules should feel like a "safety checklist" — scannable, with clear do/don't pairs
- Troubleshooting FAQ answers should include the actual compiler error message users see, then the fix
- The parallel boxing explanation should be practical: "Rust needs trait objects because branches may have different types"

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 09-documentation-overhaul*
*Context gathered: 2026-03-17 via auto-mode*
