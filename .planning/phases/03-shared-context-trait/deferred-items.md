# Deferred Items — Phase 03

## Pre-existing Formatting Issues

Found during 03-01 execution: `cargo fmt --all --check` fails on several files NOT modified in this plan. These issues pre-date this work.

Files with pre-existing formatting issues:
- `crates/durable-lambda-core/src/backend.rs` (lines 279, 288, 319, 328, 336)
- `crates/durable-lambda-core/src/error.rs` (lines 513, 559, 589)
- `crates/durable-lambda-core/src/operations/callback.rs` (lines 108, 117)
- `crates/durable-lambda-core/src/operations/child_context.rs` (lines 116, 125, 168, 177)
- `crates/durable-lambda-core/src/operations/invoke.rs` (lines 125, 134)
- `crates/durable-lambda-core/src/operations/map.rs` (lines 134, 143)
- `tests/e2e/tests/e2e_workflows.rs` (multiple locations)

**Recommendation:** Run `cargo fmt --all` in a dedicated formatting cleanup commit before merging.
