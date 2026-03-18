---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: AWS Integration Testing
status: executing
stopped_at: Completed 15-02-PLAN.md
last_updated: "2026-03-18T19:02:16Z"
last_activity: 2026-03-18 — Quick fix 1: republished 11 stale Lambda functions with musl-compiled images
progress:
  total_phases: 8
  completed_phases: 7
  total_plans: 12
  completed_plans: 12
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-17)

**Core value:** Enable Rust durable Lambda handlers with 4-8x lower memory and zero behavioral divergence from Python SDK
**Current focus:** Phase 15 — Async Operation Tests

## Current Position

Phase: 15 of 17 (Async Operation Tests)
Plan: 2 of 2 in phase 15 (complete)
Status: Executing
Last activity: 2026-03-18 — Quick fix 1: republished 11 stale Lambda functions with musl-compiled images

Progress: [██████████] 100%

## Performance Metrics

**Velocity:**
- Total plans completed: 4
- Average duration: 8 min
- Total execution time: ~30 min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 10-tooling-and-prerequisites | 1 | 5 min | 5 min |
| 11-infrastructure P01 | 1 | ~15 min | 15 min |
| 11-infrastructure P02 | 1 | 6 min | 6 min |

*Updated after each plan completion*
| Phase 12-docker-build-pipeline P01 | 3 | 1 tasks | 2 files |
| Phase 12-docker-build-pipeline P02 | 7 | 2 tasks | 1 files |
| Phase 11-infrastructure P03 | 10 | 2 tasks | 3 files |
| Phase 13-test-harness P01 | 2 | 2 tasks | 2 files |
| Phase 16-advanced-feature-tests P01 | 3 | 2 tasks | 7 files |
| Phase 16-advanced-feature-tests P02 | 82 | 2 tasks | 12 files |
| Phase 14-synchronous-operation-tests P01 | 2 | 2 tasks | 2 files |
| Phase 15-async-operation-tests P01 | 5 | 2 tasks | 4 files |
| Phase 15 P02 | 2 | 2 tasks | 2 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- v1.1: durable_config is creation-only — Terraform must include it from first apply (no retrofitting)
- v1.1: All durable invocations require qualified ARN — every function needs a `live` alias
- v1.1: cargo-chef required in Dockerfile — prevents 60-min cold builds on source changes
- v1.1: Callback tests must poll for SUSPENDED state before sending signal (never use sleep)
- v1.1: `terraform apply -parallelism=5` required to avoid ResourceConflictException at 44-function scale
- v1.1: Two callee stubs needed: `order-enrichment-lambda` (invoke tests) and `fulfillment-lambda` (combined_workflow)
- 10-01: ADFS profile region NOT modified — explicit --region us-east-2 flag used on every AWS CLI call
- 10-01: Docker daemon checked via docker info (Docker Desktop compatible), not systemctl
- [Phase 11-infrastructure]: 44 Lambda functions use for_each over locals map with durable_config (execution_timeout=3600, retention_period=7); publish = true required for live alias versioning
- [Phase 11-infrastructure]: random_id suffix (4-char hex) ensures multi-workspace safe naming across all resources; force_delete=true on ECR for clean destroy
- [Phase 11-infrastructure]: terraform apply -parallelism=5 required to avoid ResourceConflictException at 44-function scale
- [Phase 11-infrastructure]: 11-02: Targeted apply order lets Terraform resolve random_id.suffix dependency automatically; suffix c351 is now fixed for all downstream resources
- [Phase 11-infrastructure]: 11-02: deploy-ecr.sh gates on verify-prerequisites.sh to catch expired ADFS credentials before any terraform operations
- [Phase 12-docker-build-pipeline]: 12-01: Full workspace cargo chef cook (no -p) chosen to avoid cross-crate dep resolution failures; all 4 example crates share durable-lambda-core
- [Phase 12-docker-build-pipeline]: 12-01: BINARY_NAME ARG added separately from PACKAGE to fix bug where Dockerfile assumed crate name equals binary name
- [Phase 12-docker-build-pipeline]: 12-02: ECR image count verification uses unique tag count (imageIds[*].imageTag | length) not raw length(imageIds) — raw count includes untagged manifest digests
- [Phase 12-docker-build-pipeline]: 12-02: Base images pre-pulled serially before 4 parallel crate jobs to prevent Docker layer-store contention on simultaneous pulls
- [Phase 12-docker-build-pipeline]: 12-02: Binary names hardcoded in CRATE_BINS array (not computed) to guarantee exact match with lambda.tf handler map keys
- [Phase 11-infrastructure]: 11-03: DurableConfig verified via terraform state (not AWS API) — get-function-configuration does not surface DurableConfig in response
- [Phase 11-infrastructure]: 11-03: --provenance=false required in docker build — BuildKit creates OCI index manifests by default which Lambda rejects
- [Phase 13-test-harness]: test-helpers.sh is a sourceable library (no shebang, no chmod +x) — enforces correct usage pattern
- [Phase 13-test-harness]: Stub test functions return 0 so harness framework is verifiable before any real tests exist
- [Phase 13-test-harness]: 3-second polling interval for wait_for_terminal_status and extract_callback_id — no busy-loop
- [Phase 16-advanced-feature-tests]: 16-01: CRATE_BINS total computed dynamically via wc -w to avoid stale hardcoded count as binaries grow
- [Phase 16-advanced-feature-tests]: 16-01: test_closure_conditional_retry tests non-retryable path only; retryable path deferred per RESEARCH open question about StepRetryScheduled async behavior
- [Phase 16-advanced-feature-tests]: 16-02: Durable execution service SUCCEEDS → unwraps Result JSON and returns to caller directly (no Status envelope visible)
- [Phase 16-advanced-feature-tests]: 16-02: Durable execution service FAILED → converts to FunctionError=Unhandled with errorType/errorMessage from Error object
- [Phase 16-advanced-feature-tests]: 16-02: Context/Compensation sub_type rejected by service after step FAIL checkpoint — saga uses regular ctx.step() for compensation operations
- [Phase 16-advanced-feature-tests]: 16-02: execution_timeout must be ≤900s for synchronous invocation (changed from 3600 to 840)
- [Phase 16-advanced-feature-tests]: 16-02: musl cross-compilation (x86_64-unknown-linux-musl) required to avoid GLIBC 2.38 vs 2.34 mismatch between build host and Lambda al2023
- [Phase 14-synchronous-operation-tests]: 14-01: Shared assertion helpers in test-helpers.sh reduce 32 tests to 8 reusable functions
- [Phase 14-synchronous-operation-tests]: 14-01: Parallel branch assertions use sorted membership check (not index access) for non-deterministic order
- [Phase 14-synchronous-operation-tests]: 14-01: Typed errors test validates both success and error paths in a single function call
- [Phase 15-async-operation-tests]: 15-01: ctx.wait() accepts i32 not u64 -- use as_i64() with cast for event-driven wait_seconds extraction
- [Phase 15-async-operation-tests]: 15-02: get_execution_output uses --query Output --output text for async result retrieval
- [Phase 15-async-operation-tests]: 15-02: assert_callbacks sends {approved:true} and validates outcome.approved only (not callback_id per user decision)
- [Quick fix 1]: Lambda caches old ECR image digest when tag is reused — must call update-function-code to force re-resolve

### Pending Todos

None yet.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 1 | Fix macro-basic-steps Lambda runtime exit error (11 stale GLIBC images) | 2026-03-18 | 9037008 | [1-fix-macro-basic-steps-lambda-runtime-exi](./quick/1-fix-macro-basic-steps-lambda-runtime-exi/) |

### Blockers/Concerns

- Phase 15: Exact JSON field paths for GetDurableExecution response (callback_id location) must be confirmed against a live execution before finalizing polling shell functions — treat as provisional until then.

## Session Continuity

Last session: 2026-03-18T19:02:16Z
Stopped at: Completed quick fix 1 (11 stale Lambda functions republished)
Resume file: None
