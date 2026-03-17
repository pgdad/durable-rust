# Roadmap: durable-rust

## Milestones

- ✅ **v1.0 Production Hardening** - Phases 1-9 (shipped 2026-03-17)
- 🚧 **v1.1 AWS Integration Testing** - Phases 10-17 (in progress)

## Phases

<details>
<summary>✅ v1.0 Production Hardening (Phases 1-9) - SHIPPED 2026-03-17</summary>

9 phases, 23 plans completed. Shipped comprehensive test coverage (100+ tests), input validation, structured error codes, operation-level observability (tracing spans), batch checkpoint optimization, saga/compensation pattern, step timeout, conditional retry, proc-macro type validation, and documentation overhaul.

</details>

### 🚧 v1.1 AWS Integration Testing (In Progress)

**Milestone Goal:** Deploy all 44 example handlers as Lambda functions against real AWS, validate every SDK operation end-to-end with an automated test harness.

- [x] **Phase 10: Tooling and Prerequisites** - Install and configure all required tools on the developer machine
- [ ] **Phase 11: Infrastructure** - Terraform manages all AWS resources (ECR, IAM, 44 Lambda functions, aliases, stubs)
- [ ] **Phase 12: Docker Build Pipeline** - Build and push all 44 container images to ECR with cargo-chef caching
- [ ] **Phase 13: Test Harness** - Single-command test runner with per-test reporting and credential validation
- [ ] **Phase 14: Synchronous Operation Tests** - All synchronous operations validated against real Lambda (step, parallel, map, child_context, logging, combined_workflow)
- [ ] **Phase 15: Async Operation Tests** - Wait, callback, and invoke operations validated with state polling
- [ ] **Phase 16: Advanced Feature Tests** - Saga/compensation, step timeout, conditional retry, batch checkpoint validated
- [ ] **Phase 17: Documentation** - Manual test instructions, teardown guide, tooling installation doc

## Phase Details

### Phase 10: Tooling and Prerequisites
**Goal**: Developer machine has all required tools installed and configured to interact with AWS
**Depends on**: Nothing (first phase of milestone)
**Requirements**: TOOL-01, TOOL-02
**Success Criteria** (what must be TRUE):
  1. `terraform version` outputs 1.14.0 or higher
  2. `aws --version` outputs aws-cli/2.x and `aws sts get-caller-identity --profile adfs` returns a valid account ID
  3. `docker buildx version` outputs a valid version and `docker info` shows the daemon is running
  4. `jq --version` outputs 1.7 or higher
**Plans:** 1/1 plans complete
Plans:
- [x] 10-01-PLAN.md — Verify prerequisites and create verification script

### Phase 11: Infrastructure
**Goal**: All AWS resources exist and are correctly configured so Lambda functions can be invoked with durable execution enabled
**Depends on**: Phase 10
**Requirements**: INFRA-01, INFRA-02, INFRA-03, INFRA-04, INFRA-05, INFRA-06, INFRA-07, INFRA-08
**Success Criteria** (what must be TRUE):
  1. `terraform apply` completes without errors and `terraform plan` shows no changes afterward
  2. All 44 Lambda functions exist in us-east-2 with `durable_config` set and a `live` alias pointing to a published version
  3. The 2 callee stub functions (`order-enrichment-lambda`, `fulfillment-lambda`) exist and are invocable
  4. `terraform destroy` removes all resources cleanly including ECR images (no orphaned resources in the AWS account)
  5. All resources carry project/milestone/style tags visible in the AWS console
**Plans:** 1/3 plans executed
Plans:
- [ ] 11-01-PLAN.md — Create all Terraform files (7 .tf + 2 Python stubs), terraform init + validate
- [ ] 11-02-PLAN.md — Targeted apply of ECR + IAM resources (gateway for Phase 12 image push)
- [ ] 11-03-PLAN.md — Full terraform apply + comprehensive smoke verification (after Phase 12 images)

### Phase 12: Docker Build Pipeline
**Goal**: All 44 container images are built and pushed to ECR, with a repeatable one-command build that uses cargo-chef layer caching
**Depends on**: Phase 11 (ECR repositories must exist before push)
**Requirements**: BUILD-01, BUILD-02, BUILD-03, BUILD-04
**Success Criteria** (what must be TRUE):
  1. Running `scripts/build-images.sh` from a clean state produces all 44 images in ECR with per-binary tags
  2. A second run after a source-only change completes in under 10 minutes (dependency layer reused via cargo-chef)
  3. All 4 example crates build concurrently (parallel build visible in script output)
  4. Lambda functions updated to reference new image URIs are invocable immediately after the push
**Plans**: TBD

### Phase 13: Test Harness
**Goal**: A working test execution framework exists that can run any subset of tests, report per-test results, and fail fast on expired credentials
**Depends on**: Phase 12 (Lambda functions must have valid images before any test runs)
**Requirements**: TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06
**Success Criteria** (what must be TRUE):
  1. `scripts/test-all.sh` runs to completion and prints a per-test PASS/FAIL summary table
  2. `scripts/test-all.sh basic-steps-closure` runs only that single test case
  3. Running with expired ADFS credentials exits immediately with a clear `CREDENTIAL_EXPIRED` message before invoking any Lambda
  4. The polling helper correctly waits for a durable execution to reach SUCCEEDED/FAILED/TIMED_OUT without busy-looping
  5. The callback tooling extracts a callback_id from execution state and sends a success signal without manual steps
**Plans**: TBD

### Phase 14: Synchronous Operation Tests
**Goal**: All synchronous operations (step, retry, typed errors, parallel, map, child context, logging, combined workflow) return SUCCEEDED against real AWS for all 4 API styles
**Depends on**: Phase 13
**Requirements**: OPTEST-01, OPTEST-02, OPTEST-03, OPTEST-07, OPTEST-08, OPTEST-09, OPTEST-10, OPTEST-11
**Success Criteria** (what must be TRUE):
  1. All basic_steps handlers (4 styles) return SUCCEEDED on first invocation and replayed invocation with no new checkpoints
  2. All step_retries handlers (4 styles) return SUCCEEDED after the configured number of retries
  3. All typed_errors handlers (4 styles) return the expected typed error in the execution result
  4. All parallel, map, child_context, replay_safe_logging, and combined_workflow handlers (4 styles each) return SUCCEEDED
  5. `scripts/test-all.sh` shows all 32 synchronous test cases as PASS
**Plans**: TBD

### Phase 15: Async Operation Tests
**Goal**: Wait, callback, and invoke operations complete successfully against real AWS, with correct state polling before callback signal dispatch
**Depends on**: Phase 14 (polling infrastructure validated by synchronous tests)
**Requirements**: OPTEST-04, OPTEST-05, OPTEST-06
**Success Criteria** (what must be TRUE):
  1. All wait handlers (4 styles) are invoked asynchronously, polled until SUCCEEDED after the wait duration completes
  2. All callback handlers (4 styles) are invoked, polled to SUSPENDED, receive a callback success signal, then poll to SUCCEEDED — no race conditions
  3. All invoke handlers (4 styles) successfully call the `order-enrichment-lambda` stub and return its result in the execution output
  4. `scripts/test-all.sh` shows all 12 async test cases as PASS
**Plans**: TBD

### Phase 16: Advanced Feature Tests
**Goal**: Saga/compensation, step timeout, conditional retry, and batch checkpoint are validated against real Lambda execution
**Depends on**: Phase 15 (core operation suite confirmed green)
**Requirements**: ADV-01, ADV-02, ADV-03, ADV-04
**Success Criteria** (what must be TRUE):
  1. The saga test handler registers 3 compensations, fails on step 4, invokes `run_compensations`, and the execution result contains the compensation sequence in reverse order
  2. The step timeout test handler returns FAILED with a timeout error when the step closure exceeds the configured threshold
  3. The conditional retry handler retries on matching errors and does not retry on non-matching errors, confirmed by execution step count in the result
  4. The batch checkpoint handler produces fewer checkpoint API calls than the equivalent non-batch handler, confirmed by CloudWatch metrics or execution metadata
**Plans**: TBD

### Phase 17: Documentation
**Goal**: Any developer can set up the tooling, run individual tests manually, and tear down all AWS resources using written instructions
**Depends on**: Phase 16 (documents a complete, confirmed-working system)
**Requirements**: DOC-01, DOC-02, DOC-03
**Success Criteria** (what must be TRUE):
  1. A developer following the tooling installation doc from scratch can reach a working `terraform plan` in a single session without external help
  2. The manual test instructions document lists every handler with the exact `aws lambda invoke` command and expected output
  3. A single command from the cleanup/teardown instructions removes all AWS resources and leaves the account in the state it was before the milestone
**Plans**: TBD

## Progress

**Execution Order:** 10 -> 11 -> 12 -> 13 -> 14 -> 15 -> 16 -> 17

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 10. Tooling and Prerequisites | v1.1 | Complete    | 2026-03-17 | 2026-03-17 |
| 11. Infrastructure | 1/3 | In Progress|  | - |
| 12. Docker Build Pipeline | v1.1 | 0/TBD | Not started | - |
| 13. Test Harness | v1.1 | 0/TBD | Not started | - |
| 14. Synchronous Operation Tests | v1.1 | 0/TBD | Not started | - |
| 15. Async Operation Tests | v1.1 | 0/TBD | Not started | - |
| 16. Advanced Feature Tests | v1.1 | 0/TBD | Not started | - |
| 17. Documentation | v1.1 | 0/TBD | Not started | - |
