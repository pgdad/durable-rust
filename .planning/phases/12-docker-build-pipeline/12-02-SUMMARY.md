---
phase: 12-docker-build-pipeline
plan: 02
subsystem: infra
tags: [docker, ecr, cargo-chef, parallel-builds, lambda-container, bash, aws]

# Dependency graph
requires:
  - phase: 12-01
    provides: "examples/Dockerfile with cargo-chef multistage build accepting PACKAGE and BINARY_NAME args"
  - phase: 11-02
    provides: "ECR repository dr-examples-c351 in us-east-2 and IAM role dr-lambda-exec-c351"
provides:
  - "scripts/build-images.sh — one-command parallel build and push of all 44 Lambda container images"
  - "All 44 per-binary images present in ECR dr-examples-c351 with tags matching lambda.tf handler keys"
  - "Phase 11 plan 03 (full terraform apply) is now unblocked"
affects:
  - 11-03-full-terraform-apply
  - 13-integration-testing
  - 15-end-to-end-tests

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Parallel crate builds using bash background jobs (&) with PID tracking and per-exit-code failure detection"
    - "Base image pre-pull before parallelism to prevent Docker layer-store contention (4 simultaneous pulls avoided)"
    - "ECR tag names = lambda.tf handler map keys (e.g., closure-basic-steps) — contract enforced by hardcoded CRATE_BINS array"
    - "ECR image count verification using unique tag count (imageIds[*].imageTag | length) not raw imageIds length"

key-files:
  created:
    - scripts/build-images.sh
  modified: []

key-decisions:
  - "ECR image verification uses unique tag count (imageIds[*].imageTag | length) not raw length(imageIds) — raw count includes untagged manifest digests and overcounts"
  - "Base images pre-pulled serially before 4 parallel crate jobs to prevent Docker layer-store contention on simultaneous pulls"
  - "Binary names hardcoded in CRATE_BINS associative array (not computed) to guarantee exact match with lambda.tf handler map keys"

patterns-established:
  - "build-images.sh pattern: prerequisites gate → ECR login → base image pre-pull → parallel crate jobs → PID tracking → failure count → summary with ECR verification"

requirements-completed: [BUILD-02, BUILD-03, BUILD-04]

# Metrics
duration: 7min
completed: 2026-03-17
---

# Phase 12 Plan 02: Docker Build Pipeline — Build and Push Summary

**scripts/build-images.sh builds all 44 Lambda container images in parallel (4 crates x 11 binaries) and pushes them to ECR dr-examples-c351, unblocking full terraform apply**

## Performance

- **Duration:** ~7 min (script runtime 30-60 min first cold build; plan execution time excluding build wait)
- **Started:** 2026-03-17T12:49:53-04:00
- **Completed:** 2026-03-17
- **Tasks:** 2 (Task 1 auto + Task 2 human-verify checkpoint)
- **Files modified:** 1

## Accomplishments

- Created `scripts/build-images.sh` with parallel execution of 4 crate builds using bash background jobs
- Base images pre-pulled serially to prevent Docker layer contention across 4 simultaneous jobs
- All 44 images pushed to ECR dr-examples-c351 with per-binary tags matching lambda.tf handler map keys exactly
- ECR verification confirmed 44 unique tagged images present

## Task Commits

Each task was committed atomically:

1. **Task 1: Create build-images.sh with parallel crate builds and ECR push** - `26fe540` (feat)
2. **Task 1 deviation fix: ECR unique tag count** - `1dae1a0` (fix)
3. **Task 2: Verify all 44 images built and pushed** - (checkpoint approved, no additional code changes)

## Files Created/Modified

- `scripts/build-images.sh` - Parallel build-and-push script: prerequisites gate, ECR login, base image pre-pull, 4 concurrent crate jobs (closure/macro/trait/builder x 11 binaries), PID tracking, failure detection, ECR count verification

## Decisions Made

- ECR image count verification uses `imageIds[*].imageTag | length` (unique tagged count) instead of `length(imageIds)` (raw count includes untagged manifest digests). Raw count returns 88+ on a 44-image repo because each push creates both a tagged and an untagged digest entry.
- Binary names hardcoded in `CRATE_BINS` associative array — computed names risk drifting from lambda.tf handler map keys which must match exactly.
- Base images (`lukemathwalker/cargo-chef:latest-rust-1` and `public.ecr.aws/lambda/provided:al2023`) pulled once before parallelism — prevents 4 simultaneous Docker pulls from contending on the same layer store.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] ECR image count verification used wrong JMESPath query**
- **Found during:** Task 2 (Verify all 44 images built and pushed to ECR)
- **Issue:** Plan specified `--query 'length(imageIds)'` which counts all imageId objects including untagged manifest digests, returning ~88 instead of 44 for a repo with 44 tagged images
- **Fix:** Changed query to `--query 'imageIds[*].imageTag | length(@)'` which counts only tagged images (unique tags), correctly returning 44
- **Files modified:** `scripts/build-images.sh`
- **Verification:** ECR count returned 44 (matching expected value after fix)
- **Committed in:** `1dae1a0` (fix(12-02): use unique tag count for ECR image verification)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Required for correct verification output. Fix was minimal (JMESPath query change only). No scope creep.

## Issues Encountered

- Cold cargo-chef build took ~45 minutes (expected — first run compiles all workspace dependencies). Subsequent source-only runs will take 5-10 minutes as the dep layer is cached.

## User Setup Required

None — no external service configuration required beyond what was set up in Phase 11.

## Next Phase Readiness

- All 44 Lambda container images are in ECR with correct tags — Phase 11 Plan 03 (full terraform apply to deploy 44 Lambda functions) is now unblocked
- `scripts/build-images.sh` is repeatable — re-run after any source changes to update images before next terraform apply
- Tags match exactly the handler map keys in `infra/lambda.tf` — no rename mapping needed

---
*Phase: 12-docker-build-pipeline*
*Completed: 2026-03-17*
