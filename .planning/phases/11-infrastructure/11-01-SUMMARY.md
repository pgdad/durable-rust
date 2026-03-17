---
phase: 11-infrastructure
plan: 01
subsystem: infra
tags: [terraform, aws-lambda, ecr, iam, python, hcl, durable-execution]

# Dependency graph
requires:
  - phase: 10-tooling-and-prerequisites
    provides: Terraform installed, ADFS credentials configured, prerequisites verified
provides:
  - 7 Terraform .tf files defining all AWS infrastructure for integration testing
  - 44 Lambda function definitions with durable_config blocks and live aliases
  - ECR repository with lifecycle policy for Docker images
  - IAM execution role with AWSLambdaBasicDurableExecutionRolePolicy
  - 2 Python callee stub functions (order_enrichment, fulfillment)
  - .terraform.lock.hcl locking providers (aws 6.36.0, random 3.8.1, archive 2.7.1)
  - Gitignore entries preventing terraform state from being committed
affects: [12-build-pipeline, 13-test-harness, 14-integration-tests]

# Tech tracking
tech-stack:
  added:
    - hashicorp/aws provider ~>6.25 (6.36.0 locked)
    - hashicorp/random provider ~>3.0 (3.8.1 locked)
    - hashicorp/archive provider (2.7.1 locked)
  patterns:
    - for_each over locals map for 44 near-identical Lambda functions
    - random_id suffix for multi-workspace safe naming (dr-{binary}-{suffix})
    - durable_config block on every aws_lambda_function resource (creation-only)
    - publish = true on all Lambda resources to get real version numbers for aliases
    - archive_file data source for Python stub packaging

key-files:
  created:
    - infra/main.tf
    - infra/iam.tf
    - infra/ecr.tf
    - infra/lambda.tf
    - infra/stubs.tf
    - infra/variables.tf
    - infra/outputs.tf
    - infra/stubs/order_enrichment.py
    - infra/stubs/fulfillment.py
    - infra/.terraform.lock.hcl
  modified:
    - .gitignore

key-decisions:
  - "44 Lambda functions defined via for_each over a single locals map in lambda.tf — one-line addition to expand"
  - "durable_config uses execution_timeout=3600 and retention_period=7 (NOT retention_period_in_days)"
  - "publish = true on all aws_lambda_function resources to ensure .version returns numeric version for aliases"
  - "force_delete = true on ECR repo to allow terraform destroy even when images are present"
  - "All resources share random_id suffix (4-char hex) for multi-workspace collision safety"
  - "Two callee stubs (order_enrichment, fulfillment) packaged via archive_file data source"
  - "terraform apply -parallelism=5 required to avoid ResourceConflictException at 44-function scale"

patterns-established:
  - "Naming pattern: dr-{binary-name}-{suffix} for Lambda functions, dr-examples-{suffix} for ECR"
  - "Tag pattern: PascalCase default_tags via provider block (Project, Milestone, ManagedBy) + per-resource Style tag"
  - "Two-phase deploy: terraform apply -target=aws_ecr_repository.examples first, push images, then full apply"

requirements-completed: [INFRA-01, INFRA-02, INFRA-03, INFRA-04, INFRA-05, INFRA-06, INFRA-07, INFRA-08]

# Metrics
duration: 3min
completed: 2026-03-17
---

# Phase 11 Plan 01: Infrastructure — Terraform IaC Summary

**7 Terraform files defining 44 Lambda functions with durable_config, ECR repo, IAM role, Python stubs, and live aliases — validated with terraform validate + fmt clean**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-17T15:14:09Z
- **Completed:** 2026-03-17T15:17:28Z
- **Tasks:** 2 of 2
- **Files modified:** 11

## Accomplishments

- Complete Terraform IaC for AWS integration testing: 7 .tf files, 2 Python stubs, provider lock file
- All 44 Lambda functions defined via for_each with durable_config blocks (creation-only), publish = true, and live aliases
- terraform validate exits 0 and terraform fmt -check passes with zero issues
- Gitignore updated to prevent terraform.tfstate from being committed, lock file committed to pin providers

## Task Commits

Each task was committed atomically:

1. **Task 1: Create infra/ directory with all 7 Terraform files and Python stubs** - `7018a11` (feat)
2. **Task 2: Update .gitignore, run terraform init and validate** - `e1a9034` (chore)

## Files Created/Modified

- `infra/main.tf` - Provider config (AWS ~>6.25, profile=adfs, default_tags), random_id suffix, locals
- `infra/iam.tf` - IAM execution role, AWSLambdaBasicDurableExecutionRolePolicy attachment, invoke permission
- `infra/ecr.tf` - ECR repo dr-examples-{suffix} with force_delete, lifecycle policy (keep 2 / expire untagged after 2 days)
- `infra/lambda.tf` - 44-entry locals map, for_each aws_lambda_function with durable_config, for_each aws_lambda_alias
- `infra/stubs.tf` - archive_file + aws_lambda_function + aws_lambda_alias for order_enrichment and fulfillment
- `infra/variables.tf` - image_tag variable (default: "latest")
- `infra/outputs.tf` - ecr_repo_url, suffix, alias_arns, stub_alias_arns, function_names
- `infra/stubs/order_enrichment.py` - Returns enriched order data with order_id, priority, region
- `infra/stubs/fulfillment.py` - Returns fulfillment_id, status, estimated_delivery
- `infra/.terraform.lock.hcl` - Provider locks: aws 6.36.0, random 3.8.1, archive 2.7.1
- `.gitignore` - Added Terraform state exclusions (infra/.terraform/, *.tfstate, stubs/*.zip)

## Decisions Made

- Used `terraform fmt` auto-fix on lambda.tf (HCL column alignment differs from manual spacing)
- durable_config field names: `execution_timeout` and `retention_period` (not `retention_period_in_days`) per Terraform HCL vs JSON API distinction
- Python stubs use archive_file data source for Terraform-native zip packaging with hash-based drift detection

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Applied terraform fmt to fix HCL formatting in lambda.tf**
- **Found during:** Task 2 (terraform fmt -check step)
- **Issue:** lambda.tf column alignment in the handlers map was not normalized to Terraform canonical format
- **Fix:** Ran `terraform -chdir=infra fmt` to auto-normalize
- **Files modified:** infra/lambda.tf
- **Verification:** terraform fmt -check exits 0 after fix
- **Committed in:** e1a9034 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Formatting normalization is required for CI compliance. No scope creep.

## Issues Encountered

None — terraform validate passed on first run after init.

## User Setup Required

None — no external service configuration required for this plan. Terraform infrastructure files are created and validated but not applied. AWS credentials (ADFS profile) are required only when running `terraform apply`.

## Next Phase Readiness

- All 7 .tf files pass `terraform validate` and `terraform fmt -check`
- Provider lock file committed — reproducible provider versions ensured
- Phase 12 (Build Pipeline) can now reference infra/outputs.tf for ECR URL
- Phase 13 (Test Harness) can reference alias_arns and stub_alias_arns outputs
- First deploy: `terraform apply -target=aws_ecr_repository.examples -parallelism=5` then push images, then full apply

---
*Phase: 11-infrastructure*
*Completed: 2026-03-17*
