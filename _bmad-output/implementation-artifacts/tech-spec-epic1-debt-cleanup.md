---
title: 'Epic 1 Tech Debt Cleanup'
type: 'chore'
created: '2026-03-14'
status: 'done'
context: []
---

# Epic 1 Tech Debt Cleanup

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Epic 1 retrospective identified 6 minor tech debt items (3 Medium, 3 Low) across core and closure crates. Carrying these into Epic 2 risks compounding — especially the silent parser defaults and missing FAIL test which affect patterns reused by every new operation.

**Approach:** Fix all 6 items in a single pass: add missing test, fix misleading errors, remove unused dep, fix parser defaults, clean up imports, improve doc example.

## Boundaries & Constraints

**Always:** All existing tests must continue passing. No behavioral changes — only fixes, test additions, and cleanups.

**Ask First:** N/A — all items are pre-approved from retrospective.

**Never:** Do not refactor beyond the 6 identified items. Do not add new features or change public APIs.

</frozen-after-approval>

## Code Map

- `crates/durable-lambda-core/src/operations/step.rs` -- step operation: missing FAIL test + misleading serde error
- `crates/durable-lambda-core/src/replay.rs` -- replay engine: insert_operation doc example
- `crates/durable-lambda-closure/Cargo.toml` -- unused tracing dependency
- `crates/durable-lambda-closure/src/handler.rs` -- silent parser defaults
- `crates/durable-lambda-closure/src/context.rs` -- HashMap import ordering in tests

## Tasks & Acceptance

**Execution:**
- [ ] `crates/durable-lambda-core/src/operations/step.rs` -- Add `test_step_execute_fail_checkpoint` test verifying FAIL checkpoint is sent when closure returns Err with no retries configured -- closes Med review item from Story 1.4
- [ ] `crates/durable-lambda-core/src/operations/step.rs` -- Replace synthetic serde errors (`serde_json::from_str::<Value>("").unwrap_err()`) with a proper `DurableError` variant or descriptive error message for missing step_details -- closes Low review item from Story 1.4
- [ ] `crates/durable-lambda-core/src/replay.rs` -- Update `insert_operation` doc example to actually demonstrate insertion (currently only shows empty check) -- closes Low review item from Story 1.3
- [ ] `crates/durable-lambda-closure/Cargo.toml` -- Remove `tracing = { workspace = true }` from dependencies -- closes Med review item from Story 1.6
- [ ] `crates/durable-lambda-closure/src/handler.rs` -- Change `parse_operation_type` and `parse_operation_status` catch-all `_` arms to return `None` instead of silently defaulting -- closes Med review item from Story 1.6
- [ ] `crates/durable-lambda-closure/src/context.rs` -- Move `use std::collections::HashMap` to top of test module (before struct definitions) -- closes Low review item from Story 1.6

**Acceptance Criteria:**
- Given a step closure returning Err with no retries, when executed, then a FAIL checkpoint is sent (verified by new test)
- Given an unknown operation type or status string in Lambda event JSON, when parsed, then the operation is skipped (not silently converted)
- Given all changes applied, when `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check` run, then all pass with zero warnings

## Verification

**Commands:**
- `cargo test --workspace` -- expected: all tests pass including new FAIL checkpoint test
- `cargo clippy --workspace -- -D warnings` -- expected: zero warnings
- `cargo fmt --check` -- expected: no formatting differences
