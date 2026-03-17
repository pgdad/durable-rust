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

### Active (next milestone scope)
- [ ] Terraform infrastructure for all Lambda functions with durable execution enabled
- [ ] ECR repository and Docker image build pipeline for all 4 API styles
- [ ] IAM roles/policies for Lambda execution and durable execution API calls
- [ ] All ~44 example handlers deployed as individual Lambda functions
- [ ] Automated test harness — single command runs all tests with per-test pass/fail
- [ ] Manual test instructions for individual handler invocation and verification
- [ ] Callback testing tooling (SendDurableExecutionCallbackSuccess/Failure)
- [ ] All missing tooling installed (Terraform, AWS CLI, Docker, etc.)

## Current Milestone: v1.1 AWS Integration Testing

**Goal:** Build complete AWS infrastructure and test harness to validate all SDK features and examples against real AWS Lambda Durable Execution.

**Target features:**
- Terraform-managed AWS resources (ECR, Lambda, IAM)
- Docker build pipeline for all example handlers
- Automated end-to-end test runner with per-test reporting
- Manual test execution documentation
- Callback signal tooling for interactive testing

### Out of Scope
- Multi-runtime support (tokio only) — AWS Lambda ecosystem is tokio-based
- Custom serialization formats (JSON only) — matches Python SDK wire format
- Crate publishing to crates.io — internal project for now
- Rate limiting — client-side rate limiting before AWS calls (v2 candidate)
- Cancellation support — cancel waiting/callback operations (v2 candidate)
- Callback heartbeat — method to send heartbeat during callback wait (v2 candidate)

## Context

Shipped v1.0 Production Hardening milestone with 21,348 lines of Rust across 6 library crates + test suites. 120 commits, 9 phases, 23 plans. The SDK is now production-hardened with comprehensive test coverage (100+ tests), input validation, structured error codes, operation-level observability, batch checkpoint optimization, and first-class saga support.

v1.1 focuses on validating the SDK against real AWS infrastructure. All v1.0 tests use MockDurableContext — no real AWS calls. This milestone deploys all example handlers as Lambda functions and verifies they work end-to-end against the AWS Lambda Durable Execution service. AWS profile: adfs, region: us-east-2, default VPC (no new networking resources).

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

---
*Last updated: 2026-03-17 after v1.1 milestone start*
