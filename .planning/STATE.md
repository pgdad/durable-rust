---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: AWS Integration Testing
status: executing
stopped_at: Completed 16-01 — 4 advanced-feature handlers, infra registration, and test assertions
last_updated: "2026-03-17T18:18:53.088Z"
last_activity: 2026-03-17 — Completed 11-02 (ECR dr-examples-c351 and IAM dr-lambda-exec-c351 deployed and verified)
progress:
  total_phases: 8
  completed_phases: 4
  total_plans: 9
  completed_plans: 8
  percent: 6
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-17)

**Core value:** Enable Rust durable Lambda handlers with 4-8x lower memory and zero behavioral divergence from Python SDK
**Current focus:** Phase 10 — Tooling and Prerequisites

## Current Position

Phase: 10 of 17 (Tooling and Prerequisites)
Plan: 2 of 3 in phase 11 (complete — awaiting Phase 12 image push before 11-03)
Status: Executing
Last activity: 2026-03-17 — Completed 11-02 (ECR dr-examples-c351 and IAM dr-lambda-exec-c351 deployed and verified)

Progress: [██░░░░░░░░] 6%

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

### Pending Todos

None yet.

### Blockers/Concerns

- Phase 15: Exact JSON field paths for GetDurableExecution response (callback_id location) must be confirmed against a live execution before finalizing polling shell functions — treat as provisional until then.

## Session Continuity

Last session: 2026-03-17T18:18:53.087Z
Stopped at: Completed 16-01 — 4 advanced-feature handlers, infra registration, and test assertions
Resume file: None
