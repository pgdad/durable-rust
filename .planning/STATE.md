---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: AWS Integration Testing
status: executing
stopped_at: Completed 12-02 — all 44 ECR images verified, Phase 11-03 unblocked
last_updated: "2026-03-17T16:59:50.413Z"
last_activity: 2026-03-17 — Completed 11-02 (ECR dr-examples-c351 and IAM dr-lambda-exec-c351 deployed and verified)
progress:
  total_phases: 8
  completed_phases: 2
  total_plans: 6
  completed_plans: 5
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

### Pending Todos

None yet.

### Blockers/Concerns

- Phase 15: Exact JSON field paths for GetDurableExecution response (callback_id location) must be confirmed against a live execution before finalizing polling shell functions — treat as provisional until then.

## Session Continuity

Last session: 2026-03-17T16:58:55.881Z
Stopped at: Completed 12-02 — all 44 ECR images verified, Phase 11-03 unblocked
Resume file: None
