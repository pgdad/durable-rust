# Project Research Summary

**Project:** durable-rust — AWS Lambda Durable Execution integration testing infrastructure
**Domain:** Integration testing / infrastructure-as-code for a Rust SDK
**Researched:** 2026-03-17
**Confidence:** HIGH

## Executive Summary

The durable-rust SDK is production-hardened with 100+ unit tests using `MockDurableContext`, but zero real AWS calls have ever been made. This milestone validates that the SDK's wire protocol, checkpoint sequencing, and operation ID generation are correct against live AWS Lambda Durable Execution. The recommended approach is Terraform (≥ 1.14.7, AWS provider ≥ 6.25.0) for infrastructure, a parameterised Dockerfile with cargo-chef caching for all 44 container images, and a Bash+jq test harness invoking via `aws lambda invoke`. No SAM, no CDK, no Pulumi — all explicitly excluded.

The infrastructure is not complex, but it has three hard constraints that make sequencing critical: (1) `durable_config` is a creation-only Lambda attribute — it cannot be retrofitted without destroying and recreating all 44 functions; (2) all durable invocations must use a qualified ARN (alias or version) or AWS rejects them outright; (3) Docker layer caching degrades catastrophically without cargo-chef, turning every iterative build into a 60+ minute full workspace recompile. Getting these three things right in Phase 1 prevents every major failure mode downstream.

The highest-complexity test feature is callback signal testing: the test harness must invoke a function asynchronously, poll until the execution reaches `SUSPENDED`, extract the callback ID from execution state, send a callback success signal, then poll to `SUCCEEDED`. A fixed `sleep` instead of state polling causes non-deterministic failures. The saga/compensation test requires a purpose-built handler not present in the existing 44, making it a P2 item after the core smoke test suite is green.

## Key Findings

### Recommended Stack

See `.planning/research/STACK.md` for full details.

The infrastructure layer is Terraform 1.14.7 with `hashicorp/aws >= 6.25.0` (the minimum version that introduced the `durable_config` block). Earlier provider versions silently ignore `durable_config`, deploying plain Lambda functions that fail at runtime on the first checkpoint call. AWS CLI v2 (official installer, not `apt`) is required for `aws ecr get-login-password`, `--cli-binary-format raw-in-base64-out`, and the durable execution callback APIs. Docker CE with Buildx handles multi-stage image builds. Local Terraform state is correct for this single-developer validation project.

**Core technologies:**
- Terraform 1.14.7: infrastructure declaration for ECR, Lambda (44 functions), IAM, aliases
- hashicorp/aws >= 6.25.0 (pin `~> 6.25`): required for `durable_config` block on `aws_lambda_function`
- AWS CLI v2.27+: ECR login, Lambda invoke, durable execution status polling, callback signalling
- Docker CE + Buildx: parameterised multi-stage builds for all 44 binaries from 4 crates
- cargo-chef: dependency layer caching — prevents full workspace recompile on source changes
- Bash 5.x + jq 1.7: test harness; zero Python/Node dependencies
- x86_64 architecture: default Lambda target, no cross-compilation overhead (arm64 Graviton2 is a v1.2+ cost optimisation)

### Expected Features

See `.planning/research/FEATURES.md` for full details and the feature dependency graph.

**Must have (table stakes — required for milestone sign-off):**
- ECR repository (1 per API style = 4 repos) — gating dependency for everything else
- Docker build + push script for all 44 functions with cargo-chef layer caching
- IAM execution role with `AWSLambdaBasicDurableExecutionRolePolicy`
- Terraform for all 44 Lambda functions with `durable_config` + `live` alias
- Execution status polling helper (`list-durable-executions-by-function` loop)
- Basic steps + step retries tests (synchronous invoke, assert SUCCEEDED)
- Wait test with 5-second test variant (not the 60-second example value)
- Callback test with proper SUSPENDED-state polling before signal dispatch
- Parallel and map tests (fan-out branch count validation)
- Child context test
- Invoke/chained test with deployed callee stub (`order-enrichment-lambda`)
- Manual test instructions document
- Cleanup script (`terraform destroy` + ECR image deletion)

**Should have (after core suite is green):**
- Saga/compensation integration test — requires purpose-built failure handler (not in the existing 44)
- Step timeout integration test — requires a handler designed to be timed out
- Conditional retry integration test — requires a handler with a controllable failure mode
- Batch checkpoint validation
- Idempotent re-run via execution names

**Defer (v2+):**
- CI integration for integration tests (cost + credential management strategy not decided)
- Cross-account or cross-region test matrix
- Load / concurrency tests
- Execution history comparison between Rust and Python SDKs
- arm64 / Graviton2 cost optimisation (34% cheaper, requires cross-compilation setup)

### Architecture Approach

See `.planning/research/ARCHITECTURE.md` for full structure, Terraform HCL patterns, and Dockerfile details.

The project adds two new top-level directories to the workspace: `infra/` (Terraform) and `scripts/` (Bash). The 44 Lambda functions share a single ECR repository with per-binary image tags (`durable-rust-examples:closure-basic-steps`). Terraform uses `for_each` over a handler map to create all 44 `aws_lambda_function` + `aws_lambda_alias` resources from a single block — adding a handler is one line in the map. Two Python stub functions (`order-enrichment-lambda`, `fulfillment-lambda`) live in `infra/stubs/` and are deployed alongside the main functions; their names must match the string literals hardcoded in the `invoke.rs` and `combined_workflow.rs` example sources. No existing Rust crates are modified; only `examples/Dockerfile` gets a `BINARY_NAME` ARG added.

**Major components:**
1. `infra/` (Terraform) — ECR repo, 44 Lambda functions, aliases, IAM role, Python stubs; `durable_config` on every function from first apply
2. `scripts/build-images.sh` — 4 cargo builds (one per style), then 44 Docker builds copying the correct binary; cargo-chef caches the dependency layer across source changes
3. `scripts/test-all.sh` — invokes each function by alias ARN, polls `list-durable-executions-by-function`, reports per-test PASS/FAIL; callback tests poll for SUSPENDED state before signalling
4. ECR (1 repo, 44 tags) — single repository; `force_delete = true` enables clean teardown
5. IAM (1 shared role) — `AWSLambdaBasicDurableExecutionRolePolicy` + `lambda:InvokeFunction` for invoke/combined_workflow handlers
6. Python stubs (2 functions in `infra/stubs/`) — minimal non-durable responders for `ctx.invoke()` targets

### Critical Pitfalls

See `.planning/research/PITFALLS.md` for full details, recovery strategies, and a "Looks Done But Isn't" checklist.

1. **`durable_config` is creation-only** — Adding it to an existing Lambda function forces replace (destroy + recreate). Define it in every `aws_lambda_function` resource before the very first `terraform apply`. Check `terraform plan` for `forces replacement` before applying anything.

2. **Unqualified Lambda ARN is rejected** — Durable invocations without a version or alias qualifier return `InvalidParameterValueException`. Create a `live` alias for every function in Terraform. The test harness must read alias ARNs from Terraform outputs and always append `:live`.

3. **Terraform `ResourceConflictException` at default parallelism** — Concurrent creation of 44 functions + aliases + IAM attachments triggers Lambda control plane throttling. Always run `terraform apply -parallelism=5`.

4. **Docker build cache invalidation without cargo-chef** — Without cargo-chef, any source change triggers a full workspace recompile for every image (4-8 hours cold). Establish the cargo-chef Dockerfile pattern before building any images.

5. **Callback race condition** — Sending a callback signal before the execution reaches `SUSPENDED` drops the signal and causes the test to hang indefinitely. Always poll `GetDurableExecution` for `SUSPENDED` status before calling `send-durable-execution-callback-success`. Never use `sleep` as a substitute for state polling.

6. **ADFS credential expiry mid-run** — ADFS sessions last 1-4 hours; a full test run with callbacks and wait operations can exceed that. Add a `aws sts get-caller-identity` expiry check at the start of both `make deploy` and `make test`, failing fast with `CREDENTIAL_EXPIRED` when less than 30 minutes remain.

## Implications for Roadmap

The research establishes a clear dependency chain: infrastructure must exist before images can be referenced, images must be pushed before Lambda functions can be deployed, and functions must be deployed before any test can run. Within tests, synchronous operations (step, parallel, map, child_context) are simpler and should be validated first; asynchronous operations (wait, callback, invoke) build on the polling infrastructure; saga/compensation requires a new handler and comes last.

### Phase 1: Infrastructure Foundation

**Rationale:** `durable_config` is creation-only; this constraint makes it mandatory to get the Terraform structure right before any AWS resource exists. All downstream work depends on correctly configured Lambda functions with aliases. This phase has the highest rework cost if done wrong.

**Delivers:** ECR repository, IAM role, all 44 Lambda functions with `durable_config`, `live` aliases, Python stubs, Terraform outputs with alias ARNs

**Addresses:** ECR provisioning, IAM role, Lambda + durable_config, aliases (all P1 table stakes)

**Avoids:** durable_config creation-only pitfall, unqualified ARN rejection, ResourceConflictException (`-parallelism=5` baked into `make deploy`)

**Stack:** Terraform 1.14.7, hashicorp/aws ~> 6.25.0, AWS CLI v2, local state

### Phase 2: Docker Build Pipeline

**Rationale:** Images must exist in ECR before `terraform apply` can complete the Lambda function resources (image_uri is a required field). The build pipeline is also the highest-risk phase for build-time regressions; establishing cargo-chef caching early prevents the 60-minute cold-build trap.

**Delivers:** `scripts/build-images.sh` that builds all 44 images with cargo-chef layer caching and pushes to ECR; documented one-command build invocation

**Addresses:** Docker build + push (P1 table stake)

**Avoids:** Docker build cache invalidation pitfall (cargo-chef from the start), glibc version mismatch (build inside `rust:1-bookworm`, `provided:al2023` has glibc 2.34), ECR image accumulation (lifecycle policies)

**Stack:** Docker CE + Buildx, cargo-chef, `examples/Dockerfile` with `BINARY_NAME` ARG added

**Note:** `examples/Dockerfile` requires one change: add `ARG BINARY_NAME`; this is the only modification to existing files.

### Phase 3: Synchronous Operation Tests

**Rationale:** Step, parallel, map, and child_context operations complete synchronously (no suspension/resume cycle). These are the simplest tests and validate the most fundamental SDK behaviour — checkpoint write and replay. Getting these green establishes confidence in the infrastructure before tackling async complexity.

**Delivers:** `scripts/test-all.sh` with synchronous test cases for: basic-steps, step-retries, typed-errors, parallel, map, child-contexts (all 4 API styles each = 24 tests); per-test PASS/FAIL report; unique execution ID generation for test isolation

**Addresses:** Basic steps test, step retries test, parallel test, map test, child context test (all P1)

**Avoids:** $LATEST replay divergence (tests use `:live` alias from Terraform outputs), test isolation failures (unique execution IDs), Terraform state in git (`.gitignore`)

### Phase 4: Async Operation Tests

**Rationale:** Wait and callback operations suspend Lambda execution mid-flight, requiring asynchronous invocation and status polling. Invoke tests require a deployed callee stub. These build on the polling infrastructure established for synchronous tests but add the SUSPENDED-state check for callbacks. These are the highest-complexity test cases.

**Delivers:** Execution status polling helper (`wait_for_status` shell function); wait test with 5-second test variant; callback test with proper SUSPENDED-state polling before signal dispatch; invoke/chained test using `order-enrichment-lambda` callee stub; all for all 4 API styles (20 tests)

**Addresses:** Wait test, callback test, invoke/chained test, execution status polling helper (all P1)

**Avoids:** Callback race condition pitfall (poll for SUSPENDED, never sleep), async invocation patterns (Event type for callback/wait), ADFS credential expiry check at harness startup

### Phase 5: Advanced SDK Feature Tests

**Rationale:** Saga/compensation requires a purpose-built test handler that does not exist in the current 44. Step timeout and conditional retry also need purpose-built handlers. These are P2 features that validate the SDK's advanced capabilities after the core smoke test suite is confirmed green.

**Delivers:** New `test-saga-failure` handler (registers 3 compensations, fails on step 4, calls `run_compensations`, returns compensation order); integration tests for saga/compensation, step timeout, conditional retry, batch checkpoint; updated Terraform for new handlers

**Addresses:** Saga/compensation test, step timeout test, conditional retry test, batch checkpoint validation (all P2)

**Avoids:** Missing validation of the SDK's flagship post-v1.0 features

### Phase 6: Operational Hardening

**Rationale:** Cost controls, developer experience improvements, and documentation are deferred from earlier phases to avoid premature optimisation. Once the test suite is green, these items make the infrastructure maintainable.

**Delivers:** CloudWatch log group retention policies (7 days); ECR lifecycle policies (keep 3 most recent images); `make test-fast` target (closure-style only, 11 tests); manual test instructions document; `make clean` one-command teardown; developer runbook with ADFS credential refresh instructions

**Addresses:** Cleanup script, manual test instructions, CloudWatch log retention, ECR storage costs (all P1 table stakes except where noted P2/P3)

**Avoids:** Silent cost accumulation (log retention + ECR lifecycle), missing teardown documentation

### Phase Ordering Rationale

- **Infrastructure before images:** Lambda `image_uri` is required at `terraform apply` time. ECR must exist and images must be pushed before the Lambda function resources can be created. The recommended approach is a two-step apply: first `terraform apply -target=aws_ecr_repository` to create the repo, then push images, then full `terraform apply` for everything else.
- **Synchronous tests before async tests:** Polling infrastructure (invocation + status check) is simpler to validate with synchronous functions that complete immediately. Establishing a working test harness loop against simple cases reduces debugging surface when async complexity is introduced.
- **Core smoke tests before advanced SDK tests:** Saga/compensation and step timeout tests require new handlers. Writing and deploying those handlers before the basic infrastructure is validated adds scope to an already-uncertain phase. Validate the 44 existing handlers first.
- **Operational hardening last:** Lifecycle policies and log retention are set-and-forget configurations. They do not block test execution and are lower risk to defer than any of the infrastructure or test phases.

### Research Flags

Phases with standard patterns (skip research-phase):
- **Phase 1 (Infrastructure):** Terraform + Lambda + ECR patterns are thoroughly documented in official AWS IaC docs and the `hashicorp/aws` provider changelog. The pitfalls are known and specific (durable_config creation-only, qualified ARN requirement, provider version pin). No additional research needed.
- **Phase 2 (Docker Build Pipeline):** cargo-chef pattern is well-documented by its author (Luca Palmieri). The Dockerfile structure is derived directly from the existing `examples/Dockerfile` with one ARG addition. No additional research needed.
- **Phase 3 (Synchronous Tests):** Synchronous Lambda invocation and status polling are standard patterns with high-confidence official documentation.

Phases likely needing deeper research during planning:
- **Phase 4 (Async Tests):** The exact API shape of `GetDurableExecution` and `ListDurableExecutionsByFunction` responses (field names, status enum values, callback ID location in operation state) should be validated against live API responses during implementation. The research has HIGH confidence on the pattern, but field-level API details are worth confirming against a real execution before building the polling loop.
- **Phase 5 (Advanced SDK Tests):** The new test handlers for saga/compensation, step timeout, and conditional retry need careful design to produce deterministic, inspectable results. Handler design is implementation work, not research, but the test assertions require understanding the exact response shapes these SDK features produce.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified against official releases and docs. AWS provider 6.25.0 minimum confirmed via GitHub issue thread and official IaC docs. Terraform 1.14.7 is current stable. |
| Features | HIGH | Features derived from official AWS Lambda Durable Execution docs, verified against actual SDK capabilities and existing codebase. 44 binary count confirmed by direct Cargo.toml inspection. |
| Architecture | HIGH | Patterns derived from official AWS docs + direct codebase inspection (`examples/Dockerfile`, all 4 Cargo.toml files, `crates/durable-lambda-core/src/backend.rs`). Terraform `for_each` pattern is standard. |
| Pitfalls | HIGH | durable_config creation-only behaviour confirmed in official IaC docs. ResourceConflictException is a known open issue in the Terraform provider (issue #5154). cargo-chef caching verified by the library author. |

**Overall confidence:** HIGH

### Gaps to Address

- **Exact API response shapes for async polling:** The research describes the pattern for polling `GetDurableExecution` and extracting callback IDs from operation state, but the precise JSON field paths (e.g., `Operations[].CallbackId` vs `Operations[].Metadata.CallbackToken`) should be verified against a live execution before the polling shell functions are finalised in Phase 4. The architecture research provides the best available field names, but treat them as provisional until confirmed.

- **Invoke handler callee name parameterisation:** All 4 `invoke.rs` example files hardcode `"order-enrichment-lambda"` as the callee. The architecture research recommends deploying a stub function with exactly that name. This works for a fixed test environment but means the invoke handler cannot be tested against a differently-named function without a source change. This is acceptable for the milestone but should be noted as a constraint in the developer runbook.

- **combined_workflow callee target:** The `combined_workflow.rs` examples use `"fulfillment-lambda"`. This stub also needs to be deployed. The feature research confirms this requirement but it is easy to miss when listing the infrastructure components — the "44 Lambda functions" count does not include these two stubs.

- **`$LATEST` vs published version for first deploy:** On first `terraform apply`, Lambda publishes version `1` and the `live` alias points to it. Subsequent applies that change the image tag will publish version `2`, and Terraform will update the alias. This is the desired behaviour, but the `version` attribute on `aws_lambda_function` requires `publish = true` — if omitted, the `aws_lambda_alias` `function_version` attribute will be empty and the alias will not be created correctly. This must be explicit in the Terraform resource block.

## Sources

### Primary (HIGH confidence)

- [AWS Docs — Deploy Lambda durable functions with IaC](https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-iac.html) — `durable_config` block syntax, provider 6.25.0 minimum, alias requirement
- [AWS Docs — Security and permissions for Lambda durable functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-security.html) — IAM actions, `AWSLambdaBasicDurableExecutionRolePolicy`
- [AWS Docs — Invoking durable Lambda functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-invoking.html) — qualified ARN requirement, execution naming, async invocation patterns
- [AWS Docs — ListDurableExecutionsByFunction API](https://docs.aws.amazon.com/lambda/latest/api/API_ListDurableExecutionsByFunction.html) — status polling, RUNNING/SUCCEEDED/FAILED values
- [AWS Docs — SendDurableExecutionCallbackSuccess API](https://docs.aws.amazon.com/lambda/latest/api/API_SendDurableExecutionCallbackSuccess.html) — callback ID format, result body
- [AWS Docs — Configure Lambda durable functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-configuration.html) — ExecutionTimeout, RetentionPeriodInDays fields
- [AWS Docs — Best practices for Lambda durable functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-best-practices.html) — pitfall avoidance patterns
- [AWS Docs — Installing AWS CLI v2](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html) — official installer requirement
- [HashiCorp Releases — Terraform](https://releases.hashicorp.com/terraform/) — 1.14.7 confirmed current stable
- [GitHub — terraform-provider-aws releases](https://github.com/hashicorp/terraform-provider-aws/releases) — 6.36.0 current; 6.25.0 minimum for durable_config
- [Luca Palmieri — 5x Faster Rust Docker Builds with cargo-chef](https://lpalmieri.com/posts/fast-rust-docker-builds/) — cargo-chef Dockerfile pattern
- [Amazon Linux 2023 docs — Rust](https://docs.aws.amazon.com/linux/al2023/ug/rust.html) — glibc 2.34 version confirmed
- Direct codebase inspection: `examples/Dockerfile`, all 4 example `Cargo.toml` files, `crates/durable-lambda-core/src/backend.rs`

### Secondary (MEDIUM confidence)

- [GitHub — terraform-provider-aws #5154](https://github.com/hashicorp/terraform-provider-aws/issues/5154) — ResourceConflictException on concurrent Lambda updates (open known issue)
- [GitHub — terraform-provider-aws #45354](https://github.com/hashicorp/terraform-provider-aws/issues/45354) — durable_config Terraform support, provider 6.25.0 requirement confirmed via issue thread
- [SAM Testing and Debugging Durable Functions](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/test-and-debug-durable-functions.html) — confirms SAM local emulation is incomplete for production semantics (why real AWS is required)

---
*Research completed: 2026-03-17*
*Ready for roadmap: yes*
