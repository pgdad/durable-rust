# Phase 12: Docker Build Pipeline - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Build and push all 44 container images to ECR with cargo-chef layer caching and parallel builds. After this phase completes, Phase 11 plan 11-03 can run full terraform apply to create Lambda functions.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
- Build strategy: how to build 44 images efficiently (per-crate vs per-binary, Docker layer reuse)
- cargo-chef Dockerfile integration (recipe.json caching for dependency layer)
- Build script interface: flags, progress output, error handling, selective rebuild
- How to handle PACKAGE vs BINARY_NAME args in Dockerfile (research identified gap)
- Parallel build implementation (e.g., 4 crates × 11 binaries, background jobs)
- ECR login handling in build script (aws ecr get-login-password)
- Image tagging convention when pushing to ECR

</decisions>

<specifics>
## Specific Ideas

- ECR repo already deployed: `dr-examples-c351` at `REDACTED_ACCOUNT_ID.dkr.ecr.us-east-2.amazonaws.com/dr-examples-c351`
- Suffix `c351` is available from `terraform -chdir=infra output -raw suffix`
- ECR repo URL from `terraform -chdir=infra output -raw ecr_repo_url`
- 44 image tags match binary names: `closure-basic-steps`, `macro-invoke`, etc.
- All AWS CLI calls must use `--profile adfs --region us-east-2`
- After this phase completes, plan 11-03 executes to create Lambda functions referencing these images

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `examples/Dockerfile`: Existing multi-stage build (rust:1-bookworm → provided:al2023), uses `ARG PACKAGE` — needs BINARY_NAME arg added
- `scripts/verify-prerequisites.sh`: Validates Docker daemon + ECR access before builds
- `scripts/deploy-ecr.sh`: ECR login pattern already established
- 4 example Cargo.toml files enumerate all binary targets

### Established Patterns
- Each crate produces 11 binaries with naming pattern `{style}-{operation}`
- Crate names: closure-style-example, macro-style-example, trait-style-example, builder-style-example
- Binary names: closure-basic-steps through builder-combined-workflow

### Integration Points
- ECR repo created by Phase 11: `dr-examples-c351`
- Lambda functions in `infra/lambda.tf` reference `image_uri = "${ecr_repo_url}:${binary_name}"`
- Plan 11-03 (full terraform apply) depends on all 44 images existing in ECR
- Build script reads ECR URL from Terraform outputs or accepts as parameter

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 12-docker-build-pipeline*
*Context gathered: 2026-03-17*
