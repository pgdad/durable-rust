---
phase: 08-macro-builder-improvements
plan: "02"
subsystem: durable-lambda-builder
tags: [builder, tracing, error-handling, configuration, tdd]
dependency_graph:
  requires: []
  provides: [with_tracing, with_error_handler, DurableHandlerBuilder-config]
  affects: [durable-lambda-builder]
tech_stack:
  added: [tracing (runtime dep)]
  patterns: [builder-method-chaining, option-field-configuration, set_global_default]
key_files:
  created: []
  modified:
    - crates/durable-lambda-builder/src/handler.rs
    - crates/durable-lambda-builder/Cargo.toml
decisions:
  - "with_tracing stores subscriber as Option<Box<dyn tracing::Subscriber + Send + Sync + 'static>> to allow type erasure while preserving trait bounds required by set_global_default"
  - "error_handler stored as Option<Box<dyn Fn(DurableError) -> DurableError + Send + Sync>> allowing any closure matching the signature, captured by reference inside the lambda_runtime service_fn closure following existing handler pattern"
  - "run() installs tracing subscriber before AWS config init so all AWS SDK traces are captured by the subscriber"
  - "error transformation applied after handler.await, before converting to Box<dyn Error> — preserves DurableError type for handler to inspect/transform"
metrics:
  duration_seconds: 126
  completed_date: "2026-03-17"
  tasks_completed: 1
  tasks_total: 1
  files_modified: 2
  commits: 2
---

# Phase 08 Plan 02: DurableHandlerBuilder with_tracing and with_error_handler Summary

DurableHandlerBuilder extended with `with_tracing(subscriber)` and `with_error_handler(fn)` configuration methods using Option fields, tracing crate integration, and error routing — fully backward compatible.

## Tasks Completed

| # | Task | Commit | Status |
|---|------|--------|--------|
| 1 | Add with_tracing() and with_error_handler() to DurableHandlerBuilder | 1fadbe5 | Done |

### TDD Phases

| Phase | Commit | Description |
|-------|--------|-------------|
| RED | 42237fc | Failing tests for with_tracing(), with_error_handler(), chaining, backward compat |
| GREEN | 1fadbe5 | Full implementation — all 14 tests pass, workspace clean, clippy passes |

## What Was Built

Extended `DurableHandlerBuilder<F, Fut>` with two new optional configuration methods:

- `with_tracing(impl Subscriber + Send + Sync + 'static) -> Self` — stores subscriber in `Option<Box<dyn Subscriber>>` field; `run()` installs via `set_global_default` before AWS config/Lambda runtime initialization
- `with_error_handler(impl Fn(DurableError) -> DurableError + Send + Sync + 'static) -> Self` — stores closure in `Option<Box<dyn Fn(DurableError) -> DurableError>>` field; `run()` routes handler `Err` results through the closure before converting to `Box<dyn Error>`

Both methods return `Self` for fluent chaining. The zero-config path (`handler(fn).run()`) is unchanged.

## Deviations from Plan

None — plan executed exactly as written.

## Verification

```
cargo test -p durable-lambda-builder --lib   # 14 tests, all pass
cargo test --workspace                        # full workspace, all pass
cargo clippy --workspace -- -D warnings       # no warnings
```

## Self-Check

- [x] `crates/durable-lambda-builder/src/handler.rs` — exists and contains `with_tracing` and `with_error_handler`
- [x] `crates/durable-lambda-builder/Cargo.toml` — contains `tracing = { workspace = true }`
- [x] Commits 42237fc (RED) and 1fadbe5 (GREEN) exist in git log
