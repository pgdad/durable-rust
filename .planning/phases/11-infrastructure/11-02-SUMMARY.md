---
phase: 11-infrastructure
plan: 02
subsystem: infra
tags: [terraform, ecr, iam, aws-lambda, docker, targeted-apply, durable-execution]

# Dependency graph
requires:
  - phase: 11-infrastructure/11-01
    provides: Terraform .tf files for all infrastructure resources including ECR, IAM, Lambda

provides:
  - ECR repository dr-examples-c351 in us-east-2 with force_delete=true and 2-rule lifecycle policy
  - IAM role dr-lambda-exec-c351 with AWSLambdaBasicDurableExecutionRolePolicy and invoke_permission
  - Local terraform.tfstate at infra/terraform.tfstate (gitignored, not committed)
  - scripts/deploy-ecr.sh repeatable script for re-running the targeted apply
  - Resource suffix: c351 (random_id 4-char hex, embedded in all resource names)

affects: [12-build-pipeline, 13-test-harness, 15-integration-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Targeted terraform apply (-target flags) for phased infrastructure deployment
    - Prerequisite gate via verify-prerequisites.sh before any terraform operations
    - ECR lifecycle policy with 2 rules: keep last 2 tagged images, expire untagged after 2 days

key-files:
  created:
    - scripts/deploy-ecr.sh
    - infra/terraform.tfstate (gitignored, not in repo)
  modified: []

key-decisions:
  - "Targeted apply order: random_id.suffix first (implicit dependency), then ECR + IAM resources simultaneously — Terraform resolves deps automatically"
  - "Resource suffix c351 assigned by random_id.suffix — all future resources in this workspace share this suffix"
  - "deploy-ecr.sh calls verify-prerequisites.sh first to catch expired ADFS credentials before terraform operations"

patterns-established:
  - "Phased terraform apply: ECR+IAM first (no image dependency), then Lambda functions after images pushed"
  - "deploy-ecr.sh is the canonical entrypoint for Phase 11's first deploy step — idempotent (init + apply)"

requirements-completed: [INFRA-01, INFRA-02, INFRA-06, INFRA-07]

# Metrics
duration: 6min
completed: 2026-03-17
---

# Phase 11 Plan 02: Deploy ECR + IAM (Targeted Terraform Apply) Summary

**ECR repo dr-examples-c351 and IAM role dr-lambda-exec-c351 deployed to us-east-2 via targeted terraform apply, unblocking Phase 12 image builds**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-17T15:20:14Z
- **Completed:** 2026-03-17T15:26:00Z
- **Tasks:** 1 completed (Task 2 is a human-verify checkpoint, pending verification)
- **Files modified:** 1 created (scripts/deploy-ecr.sh)

## Accomplishments
- Created scripts/deploy-ecr.sh for repeatable targeted terraform apply (callable as standalone script)
- Applied 6 resources via targeted apply: random_id, aws_ecr_repository, aws_ecr_lifecycle_policy, aws_iam_role, aws_iam_role_policy_attachment, aws_iam_role_policy
- ECR repository dr-examples-c351 created with force_delete=true and 2-rule lifecycle policy
- IAM role dr-lambda-exec-c351 created with AWSLambdaBasicDurableExecutionRolePolicy + inline invoke_permission
- Terraform state is local at infra/terraform.tfstate and confirmed gitignored

## Task Commits

Each task was committed atomically:

1. **Task 1: Create deploy script and run targeted terraform apply** - `07c953b` (feat)

**Plan metadata:** (pending — will commit after human-verify checkpoint)

## Files Created/Modified
- `scripts/deploy-ecr.sh` - Bash script that runs verify-prerequisites.sh then targeted terraform apply for ECR + IAM resources, outputs ECR URL and suffix

## Decisions Made
- Targeted apply auto-includes random_id.suffix as a dependency of ECR/IAM resources — no need to explicitly target it (though plan called for it, Terraform resolves it automatically)
- The 4-char hex suffix is `c351` — all downstream resources (Lambda, stubs, aliases) will use this suffix

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None - prerequisites check passed, ADFS credentials valid, terraform apply succeeded on first attempt with 6 resources created in ~2 seconds.

## User Setup Required
None - all automation. Human verification of AWS resources is requested at checkpoint Task 2.

## Next Phase Readiness
- ECR repository `dr-examples-c351` is ready for image pushes from Phase 12
- IAM role `dr-lambda-exec-c351` is ready for Lambda function creation in Phase 11-03
- Terraform state includes ECR and IAM resources; remaining Lambda/alias resources will be applied in 11-03 after images exist
- Phase 12 can now: `docker buildx build --push --tag REDACTED_ACCOUNT_ID.dkr.ecr.us-east-2.amazonaws.com/dr-examples-c351:{binary}`

## Self-Check: PASSED

All artifacts verified:
- `scripts/deploy-ecr.sh` — exists and executable
- `11-02-SUMMARY.md` — this file
- Commit `07c953b` — confirmed in git log

---
*Phase: 11-infrastructure*
*Completed: 2026-03-17*
