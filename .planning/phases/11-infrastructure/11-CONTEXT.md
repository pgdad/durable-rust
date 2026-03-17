# Phase 11: Infrastructure - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Terraform manages all AWS resources for integration testing: ECR repository, IAM execution role, 44 Lambda functions with durable_config, Lambda aliases for qualified ARN invocation, 2 callee stub functions, and consistent resource tagging. All resources use local Terraform state.

</domain>

<decisions>
## Implementation Decisions

### Lambda naming
- Prefix: `dr-` (shortened from durable-rust)
- Pattern: `dr-{binary-name}-{suffix}` where binary-name comes from Cargo.toml (e.g., `dr-closure-basic-steps-a3f2`)
- Suffix: Terraform `random_id` resource generating 4-char hex suffix, stable per workspace (stored in state)
- Same pattern for callee stubs: `dr-order-enrichment-lambda-{suffix}`, `dr-fulfillment-lambda-{suffix}`
- Multi-workspace safe: different checkouts get different suffixes, no name collisions

### Invoke target resolution
- Invoke examples hardcode `order-enrichment-lambda` / `fulfillment-lambda` as callee names
- Resolution: test harness passes actual suffixed function name via event payload — handler reads `target_function` from event JSON
- Example source code stays unchanged — only the test invocation payload changes
- Combined workflow's `fulfillment-lambda` resolved the same way via event payload

### ECR strategy
- Single ECR repo: `dr-examples-{suffix}` with `force_delete = true`
- 44 image tags named after binaries: `closure-basic-steps`, `macro-invoke`, etc.
- Lifecycle policy: keep last 2 images per tag, expire within 2 days after becoming untagged/replaced
- Single repo maximizes Docker layer deduplication across all 44 images

### Terraform layout
- Flat file structure in `infra/` directory:
  - `infra/main.tf` — provider, terraform block, random_id
  - `infra/iam.tf` — IAM execution role + policy attachment
  - `infra/ecr.tf` — ECR repository + lifecycle policy
  - `infra/lambda.tf` — Lambda functions (for_each over locals map), aliases
  - `infra/stubs.tf` — Callee stub Lambda functions (Python runtime)
  - `infra/variables.tf` — Input variables
  - `infra/outputs.tf` — Output values (function names, ECR URI, suffix)
- Lambda function map defined as `locals` block in `lambda.tf` — all 44 entries as HCL map
- Local state file (no remote backend)

### Tagging scheme
- Tag key format: PascalCase
- Standard set applied to all resources via `default_tags` in provider block:
  - `Project = "durable-rust"`
  - `Milestone = "v1.1"`
  - `ManagedBy = "terraform"`
- Lambda-specific tag (added per resource):
  - `Style = "{closure|macro|trait|builder}"` — only on Lambda functions

### Provider configuration
- AWS provider with `region = "us-east-2"` hardcoded (from Phase 10 decision)
- `profile = "adfs"` in provider block
- AWS provider version: `>= 6.25.0` (required for `durable_config` block)
- `terraform apply -parallelism=5` documented as required to avoid ResourceConflictException

### Claude's Discretion
- Exact `durable_config` values (execution_timeout, retention_period_in_days)
- Lambda memory size and timeout for test functions
- Python stub handler code for order-enrichment-lambda and fulfillment-lambda
- Whether to split stubs into separate file or keep in lambda.tf

</decisions>

<specifics>
## Specific Ideas

- Multi-workspace: two people can run full test suites from different checkouts on the same AWS account without colliding
- `terraform destroy` must cleanly remove everything — ECR `force_delete = true` ensures images don't block repo deletion
- All 44 functions use `for_each` over a single locals map — no 44 duplicate resource blocks
- Callee stubs are minimal Python Lambdas (not Rust) — they just return a JSON response, no durable execution needed

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `examples/Dockerfile`: Multi-stage build with `ARG PACKAGE` — Terraform references ECR image URIs built from this
- Binary names in Cargo.toml: `closure-basic-steps` through `builder-combined-workflow` — these become Lambda function name suffixes and ECR tags
- `scripts/verify-prerequisites.sh`: Validates ADFS credentials and ECR access before Terraform runs

### Established Patterns
- Binary naming: `{style}-{operation}` across all 4 Cargo.toml files (closure, macro, trait, builder × 11 operations)
- Hardcoded callee names in source: `order-enrichment-lambda` (invoke.rs), `fulfillment-lambda` (combined_workflow.rs)

### Integration Points
- Build pipeline (Phase 12) pushes images to the ECR repo created here
- Test harness (Phase 13) uses Terraform outputs for function names and qualified ARNs
- Lambda functions reference ECR image URIs: `{account}.dkr.ecr.us-east-2.amazonaws.com/dr-examples-{suffix}:{tag}`

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 11-infrastructure*
*Context gathered: 2026-03-17*
