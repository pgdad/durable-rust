# Requirements: durable-rust

**Defined:** 2026-03-17
**Core Value:** Enable Rust teams to write durable Lambda handlers with 4-8x lower memory and zero behavioral divergence from the official SDK

## v1.1 Requirements

Requirements for AWS Integration Testing milestone. Each maps to roadmap phases.

### Tooling

- [x] **TOOL-01**: All missing tooling installed on Ubuntu (Terraform, AWS CLI v2, Docker CE + Buildx, jq)
- [x] **TOOL-02**: AWS CLI configured with `adfs` profile and `us-east-2` region

### Infrastructure

- [x] **INFRA-01**: Terraform creates ECR repositories for all 4 API styles (closure, macro, trait, builder)
- [x] **INFRA-02**: Terraform creates IAM execution role with `AWSLambdaBasicDurableExecutionRolePolicy` managed policy
- [x] **INFRA-03**: Terraform creates all 44 example Lambda functions with `durable_config` block and `durable_execution_timeout`
- [x] **INFRA-04**: Terraform creates Lambda aliases for qualified ARN invocation on every function
- [x] **INFRA-05**: Terraform creates 2 callee stub Lambda functions (`order-enrichment-lambda`, `fulfillment-lambda`) for invoke/chained tests
- [x] **INFRA-06**: All Terraform-managed resources have consistent labels/tags for identification (project, milestone, style)
- [x] **INFRA-07**: Terraform uses local state file (no remote backend)
- [x] **INFRA-08**: `terraform destroy` cleanly removes all resources including ECR images (`force_delete = true`)

### Build Pipeline

- [x] **BUILD-01**: Dockerfile updated with cargo-chef for fast dependency-layer caching
- [x] **BUILD-02**: Build script builds all 44 Docker images from the 4 example crates
- [x] **BUILD-03**: Build script pushes all 44 images to ECR with per-binary tags
- [x] **BUILD-04**: Build supports parallel execution (4 crates built concurrently)

### Test Harness

- [x] **TEST-01**: Single command (`test-all.sh`) runs all integration tests and reports per-test pass/fail
- [x] **TEST-02**: Execution status polling helper waits for durable execution to reach terminal state (SUCCEEDED/FAILED/TIMED_OUT)
- [x] **TEST-03**: Callback signal tooling extracts callback_id from execution state and sends `SendDurableExecutionCallbackSuccess`
- [x] **TEST-04**: Test harness validates ADFS credential validity before starting test run
- [x] **TEST-05**: Per-test pass/fail output with test name, status, and failure reason
- [x] **TEST-06**: Each test individually runnable via command-line argument

### Operation Tests

- [ ] **OPTEST-01**: Step tests pass — all 4 styles' `basic_steps` handlers invoked and return SUCCEEDED
- [ ] **OPTEST-02**: Step retry tests pass — all 4 styles' `step_retries` handlers invoked and return SUCCEEDED
- [ ] **OPTEST-03**: Typed error tests pass — all 4 styles' `typed_errors` handlers invoked and return expected error
- [ ] **OPTEST-04**: Wait tests pass — test variant with 5-second wait deployed, invoked async, polled to SUCCEEDED
- [ ] **OPTEST-05**: Callback tests pass — all 4 styles' `callbacks` handlers invoked async, callback signal sent, polled to SUCCEEDED
- [ ] **OPTEST-06**: Invoke tests pass — caller invokes callee stub, returns callee result in response
- [ ] **OPTEST-07**: Parallel tests pass — all 4 styles' `parallel` handlers invoked, all branches present in result
- [ ] **OPTEST-08**: Map tests pass — all 4 styles' `map` handlers invoked and return SUCCEEDED
- [ ] **OPTEST-09**: Child context tests pass — all 4 styles' `child_contexts` handlers invoked and return SUCCEEDED
- [ ] **OPTEST-10**: Logging tests pass — all 4 styles' `replay_safe_logging` handlers invoked and return SUCCEEDED
- [ ] **OPTEST-11**: Combined workflow tests pass — all 4 styles' `combined_workflow` handlers invoked and return SUCCEEDED

### Advanced Feature Tests

- [ ] **ADV-01**: Saga/compensation test — purpose-built handler registers compensations, fails forward step, runs compensations in reverse order, returns compensation sequence
- [ ] **ADV-02**: Step timeout test — handler with long-running step times out at configured threshold
- [ ] **ADV-03**: Conditional retry test — handler with retry_if predicate retries on matching errors only
- [ ] **ADV-04**: Batch checkpoint test — handler using batch mode makes fewer checkpoint calls than non-batch equivalent

### Documentation

- [ ] **DOC-01**: Manual test instructions document — how to invoke each handler individually with expected output
- [ ] **DOC-02**: Cleanup/teardown instructions — single-command resource destruction
- [ ] **DOC-03**: Tooling installation instructions for Ubuntu

## v1.2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### CI Integration

- **CI-01**: Integration tests run on scheduled CI (weekly or pre-release)
- **CI-02**: Cost reporting per test run

### Cross-SDK Validation

- **XSDK-01**: Execution history comparison — Rust vs Python operation ID sequences match

## Out of Scope

| Feature | Reason |
|---------|--------|
| SAM CLI local testing | Local durable execution emulation incomplete; cannot catch wire protocol mismatches |
| One ECR repo per Lambda (44 repos) | Excessive Terraform overhead; images share 95% of layers |
| Per-commit integration tests in CI | $1-3 per run + 10-15 min; manual/release gate only |
| Remote Terraform state (S3 backend) | User specified local state; single developer context |
| New VPCs/subnets/NAT gateways | User constraint — use default VPC only |
| Blue/green deployment | Test environment, not production; simple publish + alias sufficient |
| Cross-account or cross-region testing | Complexity not justified for internal SDK validation |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| TOOL-01 | Phase 10 | Complete |
| TOOL-02 | Phase 10 | Complete |
| INFRA-01 | Phase 11 | Complete |
| INFRA-02 | Phase 11 | Complete |
| INFRA-03 | Phase 11 | Complete |
| INFRA-04 | Phase 11 | Complete |
| INFRA-05 | Phase 11 | Complete |
| INFRA-06 | Phase 11 | Complete |
| INFRA-07 | Phase 11 | Complete |
| INFRA-08 | Phase 11 | Complete |
| BUILD-01 | Phase 12 | Complete |
| BUILD-02 | Phase 12 | Complete |
| BUILD-03 | Phase 12 | Complete |
| BUILD-04 | Phase 12 | Complete |
| TEST-01 | Phase 13 | Complete |
| TEST-02 | Phase 13 | Complete |
| TEST-03 | Phase 13 | Complete |
| TEST-04 | Phase 13 | Complete |
| TEST-05 | Phase 13 | Complete |
| TEST-06 | Phase 13 | Complete |
| OPTEST-01 | Phase 14 | Pending |
| OPTEST-02 | Phase 14 | Pending |
| OPTEST-03 | Phase 14 | Pending |
| OPTEST-04 | Phase 15 | Pending |
| OPTEST-05 | Phase 15 | Pending |
| OPTEST-06 | Phase 15 | Pending |
| OPTEST-07 | Phase 14 | Pending |
| OPTEST-08 | Phase 14 | Pending |
| OPTEST-09 | Phase 14 | Pending |
| OPTEST-10 | Phase 14 | Pending |
| OPTEST-11 | Phase 14 | Pending |
| ADV-01 | Phase 16 | Pending |
| ADV-02 | Phase 16 | Pending |
| ADV-03 | Phase 16 | Pending |
| ADV-04 | Phase 16 | Pending |
| DOC-01 | Phase 17 | Pending |
| DOC-02 | Phase 17 | Pending |
| DOC-03 | Phase 17 | Pending |

**Coverage:**
- v1.1 requirements: 38 total
- Mapped to phases: 38
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-17*
*Last updated: 2026-03-17 — TOOL-01 and TOOL-02 completed (10-01-PLAN.md)*
