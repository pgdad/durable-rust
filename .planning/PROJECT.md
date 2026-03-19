# PROJECT.md

## What This Is

An idiomatic Rust SDK for AWS Lambda Durable Execution providing full feature parity with the official AWS Python Durable Lambda SDK. Supports 8 core operations (Step, Wait, Callback, Invoke, Parallel, Map, Child Context, Logging) across 4 API styles (closure, macro, trait, builder) with a replay engine, mock testing infrastructure, Python-Rust compliance verification, step timeouts, conditional retry, saga/compensation pattern, batch checkpointing, and operation-level tracing spans.

## Core Value

Enable Rust teams to write durable Lambda handlers with 4-8x lower memory and order-of-magnitude faster cold starts than Python, with zero behavioral divergence from the official SDK.

## Requirements

### Validated (shipped & confirmed)
- ✓ 8 core durable operations with replay semantics — v0
- ✓ 4 API styles with behavioral parity — v0
- ✓ MockDurableContext for credential-free testing — v0
- ✓ Deterministic operation ID generation (blake2b) — v0
- ✓ Python-Rust compliance suite — v0
- ✓ Exponential backoff retry on transient AWS failures — v0
- ✓ Comprehensive error path tests (11 tests) and step closure panic safety — v1.0
- ✓ Boundary condition tests (19 tests: nesting, replay determinism, edge values) — v1.0
- ✓ DurableContextOps shared trait eliminating ~1,800 lines of delegation duplication — v1.0
- ✓ Input validation guards + structured error codes on DurableError — v1.0
- ✓ Step timeout (timeout_seconds) and conditional retry (retry_if predicate) — v1.0
- ✓ Tracing spans for all 7 operations + batch checkpoint API (90% call reduction) — v1.0
- ✓ Saga/compensation pattern (step_with_compensation + run_compensations) — v1.0
- ✓ Proc-macro type validation + builder .with_tracing()/.with_error_handler() — v1.0
- ✓ Documentation overhaul (determinism rules, error examples, troubleshooting FAQ) — v1.0

### Validated (shipped & confirmed) — v1.1
- ✓ Terraform infrastructure for all Lambda functions with durable execution — v1.1
- ✓ ECR repository and Docker image build pipeline for all 4 API styles — v1.1
- ✓ IAM roles/policies for Lambda execution and durable execution API calls — v1.1
- ✓ All 48 example handlers deployed as individual Lambda functions — v1.1
- ✓ Automated test harness — 48/48 tests passing (32 real + 16 XFAIL) — v1.1
- ✓ Callback testing tooling (SendDurableExecutionCallbackSuccess) — v1.1

### Validated (shipped & confirmed) — v1.2
- ✓ All 6 crates published to crates.io v1.2.0 with complete metadata — v1.2
- ✓ Workspace-level version inheritance (bump root, all crates update) — v1.2
- ✓ Dual MIT/Apache-2.0 license with LICENSE-MIT and LICENSE-APACHE files — v1.2
- ✓ Dependency-ordered publish script with dry-run mode and idempotent re-runs — v1.2
- ✓ GitHub Actions release workflow (v* tag → test → publish → GitHub Release) — v1.2
- ✓ PR publish-check CI job catches metadata issues before merging — v1.2

### Active (next milestone scope)

(None — awaiting next milestone definition)

### Out of Scope
- Multi-runtime support (tokio only) — AWS Lambda ecosystem is tokio-based
- Custom serialization formats (JSON only) — matches Python SDK wire format
- Rate limiting — client-side rate limiting before AWS calls (v2 candidate)
- Cancellation support — cancel waiting/callback operations (v2 candidate)
- Callback heartbeat — method to send heartbeat during callback wait (v2 candidate)
- Changelog generation — manual for now, automate later
- Publishing example crates — examples are not library crates, not published

## Context

Shipped v1.0 Production Hardening (9 phases, 23 plans). Shipped v1.1 AWS Integration Testing (8 phases, 12 plans) — 48 Lambda functions deployed, 48/48 tests passing. Shipped v1.2 Crates.io Publishing (3 phases, 6 plans) — all 6 crates live on crates.io v1.2.0 with automated CI/CD release pipeline.

17,595 lines of Rust across 6 library crates. 20 phases, 41 plans completed across 3 milestones. AWS profile: adfs, region: us-east-2. Repository: https://github.com/pgdad/durable-rust.

## Constraints

- **Rust stable toolchain 1.82.0+** — no nightly features
- **Operation ID format must match Python SDK** — blake2b("{counter}") truncated to 64 hex chars; divergence breaks cross-SDK replay
- **Checkpoint protocol is fixed** — START then SUCCEED/FAIL per operation; dictated by AWS API
- **No cross-approach crate dependencies** — closure/trait/builder crates depend only on core
- **All workspace dependency versions in root Cargo.toml** — never pin in individual crates
- **CI: `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test --workspace`** — all must pass

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Shared DurableContextOps trait | 1,800 lines of identical delegation code across 3 wrapper crates | ✓ Good — enables generic handlers, eliminates duplication |
| Validate options at construction | Negative retries, zero timeouts silently accepted | ✓ Good — fail-fast prevents undefined behavior |
| Structured error codes on DurableError | Retry detection used fragile string matching | ✓ Good — programmatic error matching, exhaustive match enforced |
| Batch checkpoint API | Each operation makes 2 AWS calls (START + SUCCEED) | ✓ Good — 90% call reduction in batch mode |
| Saga pattern as first-class feature | #1 missing pattern; primary durable execution use case | ✓ Good — compensation/rollback without manual bookkeeping |
| Arc<dyn Fn> for retry_if predicate | StepOptions must remain Clone | ✓ Good — type erasure without sacrificing Clone |
| tokio::spawn for step closure panic safety | Panics in user closures should not crash process | ✓ Good — JoinError converted to DurableError |
| Step timeout via tokio::time::timeout | Long-running step closures can hang forever | ✓ Good — per-step deadline with abort on expiry |
| Dual MIT/Apache-2.0 license | Rust ecosystem standard for maximum compatibility | ✓ Good — matches tokio, serde, clap conventions |
| Workspace version inheritance | 6 crates need synchronized versions | ✓ Good — single bump in root propagates everywhere |
| Publish script with is_published() + "already exists" handling | crates.io API caching can miss recently-published crates | ✓ Good — idempotent re-runs after partial failures |
| Step-wrapped invoke instead of ctx.invoke() | AWS durable execution service doesn't populate chained_invoke_details during replay | ⚠️ Revisit when service adds support |

---
*Last updated: 2026-03-19 after v1.2 milestone completion*
