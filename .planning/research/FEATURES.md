# Feature Research

**Domain:** AWS Lambda Durable Execution SDK — Integration Testing Infrastructure
**Researched:** 2026-03-17
**Confidence:** HIGH (AWS API docs verified, Terraform provider support confirmed, existing codebase examined)

---

## Context: What Exists vs. What This Milestone Adds

The SDK is production-hardened with 100+ tests, all using `MockDurableContext`. Zero real AWS calls have ever been made. This milestone builds the infrastructure and test harness to validate all 44 example Lambda functions against live AWS Lambda Durable Execution. The goal is not testing the mock — it is proving the wire protocol and IAM configuration are correct against a real service.

**Existing assets to build on:**
- 44 Lambda binaries across 4 API styles (closure/macro/trait/builder), 11 handlers each
- `examples/Dockerfile` multi-stage build accepting `--build-arg PACKAGE=<crate-name>`
- Each style is a separate crate (`closure-style-example`, `macro-style-example`, etc.)
- 8 operation types: step, wait, callback, invoke, parallel, map, child_context, logging
- SDK features to validate: step timeout, conditional retry, saga/compensation, batch checkpoint

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features required for any credible integration test suite. Missing these means the milestone cannot claim "validated against real AWS."

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| ECR repository provisioning | Docker images must be pushed somewhere before Lambda can reference them; Lambda container images require ECR in same region | LOW | One ECR repo suffices if images are tagged by function name; alternatively one repo per style (4 repos). Single repo with tags is simpler Terraform. |
| Docker build and ECR push script | 44 binaries built from 4 crates, each needing a distinct image; Dockerfile already exists with `--build-arg PACKAGE=` | MEDIUM | Build-and-push must be scriptable in CI and locally. Cross-compilation to `linux/amd64` required on non-x86 build hosts. Each of 4 crates produces 11 images = 44 tags. |
| IAM execution role for durable functions | Lambda requires `lambda:ManageDurableState`, `lambda:CheckpointDurableExecutions`, `lambda:GetDurableExecution`, `lambda:ListDurableExecutions`; AWS provides managed policy `AWSLambdaBasicDurableExecutionRolePolicy` | LOW | One shared role works for all 44 functions in test context. In production each workflow would scope down permissions. |
| Terraform for Lambda function resources | 44 `aws_lambda_function` resources with `durable_config` block; Terraform AWS provider >= 6.25.0 required | MEDIUM | Use `for_each` over a map of function names to avoid 44 duplicate resource blocks. Each function needs a qualified alias (`$LATEST` or version) because durable invocations reject unqualified names. |
| `durable_config` block on each Lambda | Without this block the function is a plain Lambda — durable operations will fail at runtime | LOW | `execution_timeout` and `retention_period_in_days` are the key fields. Set `execution_timeout = 3600` (1 hour) for tests with wait/callback operations. |
| Per-test invocation with pass/fail result | Each of the 44 functions must be invokable and the result must be asserted against expected output | MEDIUM | Invoke via `aws lambda invoke`, parse `StatusCode` and response body. Non-zero function errors appear in the body with `FunctionError` key. |
| Execution status polling for async workflows | Wait/callback/invoke functions suspend and resume; synchronous invoke returns 202 mid-flight; test must poll `GetDurableExecution` until terminal state | MEDIUM | Poll with exponential backoff, timeout at test-level deadline (e.g. 5 minutes). Terminal states: `SUCCEEDED`, `FAILED`, `TIMED_OUT`, `STOPPED`. |
| Test result reporting: per-test pass/fail | A single command must show which of 44 tests passed and which failed, with reason | LOW | Simple table or colored output. Machine-readable JSON optional but not required for v1. |
| Cleanup / teardown of AWS resources | Test resources cost money and leave state that breaks re-runs; must be removable with one command | LOW | `terraform destroy` covers Lambda + IAM + ECR. Separate ECR image deletion step needed before destroy (ECR repos with images cannot be force-deleted by default without `force_delete = true`). |

### Differentiators (Competitive Advantage)

Features that move this beyond a basic smoke test suite and make it trustworthy for SDK validation.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Callback signal tooling | Callback functions suspend waiting for `SendDurableExecutionCallbackSuccess`; without tooling the test hangs forever. A helper script or Rust binary that extracts the callback ID from execution state and sends the signal is the only way to test callback handlers automatically | HIGH | Must: (1) invoke the callback function asynchronously, (2) poll execution state until `WAITING_FOR_CALLBACK` status appears, (3) extract `callback_id` from operation details, (4) call `aws lambda send-durable-execution-callback-success --callback-id <id> --result '{"approved":true}'`. This is the single most complex test feature. |
| Wait operation with shortened duration | The example `waits.rs` uses a 60-second wait — unacceptable in a CI test. Need either a test-specific variant that uses 5 seconds, or accept 60s as the test cost | LOW | Simplest approach: deploy a separate test variant of the waits handler with a 5-second wait. Alternatively accept the wait cost and set test timeout accordingly. |
| Invoke/chained test with caller + callee pair | `ctx.invoke()` calls another Lambda by function name. Both the caller and a simple "callee" function must be deployed. The callee does not need to be durable — it just needs to return a JSON response | MEDIUM | The existing `closure-invoke` example hardcodes `"order-enrichment-lambda"` as the target. The deployed callee function name must match. Either deploy an actual `order-enrichment-lambda` function or parameterize the invoke handler to accept the target name from the event payload. Parameterization is cleaner for testing. |
| Parallel and map execution validation | Parallel/map create child sub-executions; the test must verify all branches completed | MEDIUM | Invoke, poll to `SUCCEEDED`, inspect `Result` field. The handler returns `{"parallel_results": [...]}` — assert that all 3 branches appear in results. |
| Saga/compensation rollback validation | `step_with_compensation` + `run_compensations` is v1.0's flagship feature; integration test must verify that compensations run in reverse order on failure | HIGH | Requires a handler that intentionally fails a forward step after registering compensations, then calls `run_compensations`, and returns evidence of the compensation order in its response. The existing mock tests verify this, but real AWS checkpointing adds a new failure surface. |
| Idempotent test re-runs via execution names | `--durable-execution-name` parameter on invoke prevents duplicate executions when re-running a test without full teardown. Ensures test harness can retry safely | LOW | Pass a deterministic execution name derived from test name + run ID. If the execution already succeeded the same result is returned. |
| Parallel Docker builds | 44 images built sequentially is slow (~15-20 minutes). Building 4 crate images in parallel (each crate produces 11 tagged binaries in one build) reduces to ~4 minutes | LOW | `docker buildx build` produces a single image per crate; tag images inside as `{crate}-{binary}`. Or use a makefile with `make -j4`. |
| Terraform remote state | Multiple engineers sharing the same AWS account need consistent state; S3 backend with DynamoDB locking avoids conflicts | LOW | S3 bucket + DynamoDB table for state locking. Use the existing account's default infrastructure — no new VPC/networking. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| SAM CLI local testing | SAM has local durable execution support; seems like it would avoid needing real AWS | SAM local durable execution emulation is documented but incomplete for production semantics. The SDK's checkpoint protocol and operation ID generation must match the real AWS service exactly. Local emulation cannot catch ID mismatch bugs — which is the entire point of this milestone. Deploy to real AWS. | Use `MockDurableContext` (already built) for unit tests; deploy to real AWS for integration tests |
| One ECR repo per Lambda function (44 repos) | Seems like clean separation | Terraform overhead of 44 ECR resources is enormous; ECR costs, quota limits, and cleanup complexity multiply by 44. Images from the same crate share 95% of layers anyway. | One ECR repo per API style (4 repos), images tagged by binary name within each repo |
| Full CI pipeline running integration tests on every commit | Seems like good CI hygiene | Integration tests cost ~$0.50-2.00 per full run (Lambda invocations, ECR storage, execution retention), take 10-15 minutes, and require AWS credentials in CI. This is a manual/release gate, not a per-commit check. | Run unit tests (`cargo test --workspace`) on every commit; run integration tests manually before milestone sign-off or on a scheduled basis |
| Reusing the same Lambda functions across all 4 API styles in one test run | Seems efficient | The invoke example in each style hardcodes a target function name. If closure-invoke and macro-invoke both target the same callee, name collisions become a problem. The 4 styles exercise different code paths and must be independently validated. | Deploy all 44 functions with style-prefixed names; each style's invoke handler targets its own dedicated callee |
| Blue/green deployment of updated functions | Seems like production best practice | Adds Terraform complexity (aliases, weighted routing) for a test environment. The goal is one-shot validation, not zero-downtime deploy. | Publish a new function version on each `terraform apply`, update the alias to point to it |
| Automated cleanup after each test | Each test tears down and re-deploys its function | Lambda cold starts after re-deploy add 2-3 seconds per test; ECR image pull on first invoke adds 5-10 seconds. For 44 tests this is 5-7 minutes of dead time. | Deploy all 44 functions once at start, run all tests, destroy all resources at end |

---

## Feature Dependencies

```
ECR Repository
    └──requires──> Docker Build + Push
                       └──requires──> Terraform Lambda Resources
                                          └──requires──> IAM Execution Role
                                                             └──enables──> Test Invocation

Test Invocation
    └──requires──> Execution Status Polling
                       └──enables──> All 8 Operation Tests

Callback Signal Tooling
    └──requires──> Execution Status Polling (to get callback_id)
    └──requires──> Test Invocation (async mode)
    └──enables──> Callback Handler Tests

Invoke/Chained Tests
    └──requires──> Callee Lambda deployed (separate from invoke handler)
    └──enables──> ctx.invoke() validation

Saga/Compensation Tests
    └──requires──> A handler that fails forward and runs compensations
    └──requires──> Test Invocation + Execution Status Polling
    └──enables──> step_with_compensation + run_compensations validation

Parallel/Map Tests
    └──requires──> Test Invocation + Execution Status Polling
    └──enables──> ctx.parallel() + ctx.map() validation

Cleanup / Teardown
    └──requires──> ECR force_delete or pre-destroy image deletion
    └──enables──> Re-runnable test environment
```

### Dependency Notes

- **ECR repository requires Docker build:** Lambda container images can only reference ECR URIs in the same account and region. The Terraform resource references the image URI, so images must exist before `terraform apply` can complete.
- **IAM role requires AWS managed policy:** `AWSLambdaBasicDurableExecutionRolePolicy` must exist in the account — it is AWS-managed and always available.
- **Callback tooling requires async invocation:** Callback handlers suspend immediately; synchronous invoke would time out waiting for a callback that never arrives. Must use `--invocation-type Event`, then poll `GetDurableExecution`.
- **Invoke tests require callee deployment:** The `closure-invoke` example targets `"order-enrichment-lambda"`. This function must exist and be durable-enabled (or be a simple non-durable responder). Recommended: deploy a minimal `order-enrichment-lambda` that returns a hardcoded JSON response.
- **Saga tests require a purpose-built handler:** None of the existing 44 example handlers are designed to fail and run compensations — they all succeed. A new `test/saga-compensation-test` handler is needed that: registers 3 compensations, fails on step 4, calls `run_compensations`, and returns the compensation execution order in its response.

---

## MVP Definition

### Launch With (v1 — milestone sign-off)

Minimum scope to claim "validated against real AWS Lambda Durable Execution":

- [ ] ECR repository (1 per API style = 4 repos) — gating dependency for everything else
- [ ] Docker build + push script for all 44 functions — gating dependency for Lambda deploy
- [ ] IAM execution role with `AWSLambdaBasicDurableExecutionRolePolicy` — required for durable ops
- [ ] Terraform for all 44 Lambda functions with `durable_config` block and `$LATEST` alias — core infrastructure
- [ ] Execution status polling helper (shell function or Rust binary) — required for async tests
- [ ] Basic steps test: invoke all 4 `basic-steps` functions, assert `SUCCEEDED` — validates step checkpointing
- [ ] Step retries test: invoke all 4 `step-retries` functions, assert `SUCCEEDED` — validates retry path
- [ ] Wait operation test: deploy 5-second-wait test variant, assert `SUCCEEDED` — validates suspension/resume
- [ ] Callback test: async invoke all 4 `callbacks` functions, extract callback ID, send success signal, assert `SUCCEEDED` — validates external event resume
- [ ] Parallel test: invoke all 4 `parallel` functions, assert all 3 branches in result — validates fan-out
- [ ] Map test: invoke all 4 `map` functions, assert `SUCCEEDED` — validates map fan-out
- [ ] Child context test: invoke all 4 `child-contexts` functions, assert `SUCCEEDED` — validates namespace isolation
- [ ] Invoke test: deploy callee + invoke handlers, invoke caller, assert callee result in response — validates Lambda-to-Lambda
- [ ] Manual test instructions document — human-readable guide for running individual handlers
- [ ] `terraform destroy` + ECR cleanup script — must be runnable in one step

### Add After Validation (v1.x)

Once the core 44-handler smoke tests pass:

- [ ] Saga/compensation integration test — requires purpose-built handler; add when core suite is green
- [ ] Step timeout integration test — requires a handler that runs long enough to be timed out; add when core suite is green
- [ ] Conditional retry integration test — requires a handler with a flaky side-effect; add when core suite is green
- [ ] Batch checkpoint validation — compare checkpoint call counts between batch and non-batch mode; add when IAM logging is set up
- [ ] Idempotent re-run testing via execution names — add when test harness is stable enough to validate idempotency

### Future Consideration (v2+)

- [ ] CI integration for integration tests — defer until cost/credential management strategy is decided
- [ ] Cross-account or cross-region test matrix — defer; complexity not justified for internal SDK
- [ ] Load / concurrency tests — multiple parallel executions to validate service limits — defer to v2
- [ ] Execution history comparison (Rust vs Python) — validate that operation ID sequences match across SDKs — defer; requires Python SDK test environment

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| ECR repos + Docker build pipeline | HIGH | MEDIUM | P1 |
| IAM role for durable execution | HIGH | LOW | P1 |
| Terraform for 44 Lambda functions | HIGH | MEDIUM | P1 |
| Execution status polling helper | HIGH | LOW | P1 |
| Basic steps + step retries tests | HIGH | LOW | P1 |
| Callback signal tooling + test | HIGH | HIGH | P1 |
| Wait test (short-duration variant) | HIGH | LOW | P1 |
| Parallel + map tests | HIGH | LOW | P1 |
| Child context test | MEDIUM | LOW | P1 |
| Invoke/chained test + callee Lambda | HIGH | MEDIUM | P1 |
| Manual test instructions | HIGH | LOW | P1 |
| Cleanup / terraform destroy script | HIGH | LOW | P1 |
| Saga/compensation integration test | HIGH | HIGH | P2 |
| Step timeout integration test | MEDIUM | MEDIUM | P2 |
| Conditional retry integration test | MEDIUM | MEDIUM | P2 |
| Batch checkpoint validation | MEDIUM | MEDIUM | P2 |
| Terraform remote state (S3 backend) | MEDIUM | LOW | P2 |
| Idempotent re-run via execution names | LOW | LOW | P2 |
| Parallel Docker builds | LOW | LOW | P3 |
| CI integration for integration tests | MEDIUM | HIGH | P3 |

**Priority key:**
- P1: Must have for milestone sign-off (v1.1 complete)
- P2: Should have after core suite passes (v1.x)
- P3: Nice to have, future milestone

---

## Operation-Specific Testing Strategies

Each of the 8 SDK operations has distinct suspend/resume behavior that requires a tailored test strategy.

### Step (basic_steps, step_retries, typed_errors)
- **Behavior:** Synchronous checkpoint-on-success. Function does not suspend.
- **Test strategy:** Invoke synchronously (`--invocation-type RequestResponse`). Wait for response body. Assert `StatusCode: 200` and response JSON matches expected shape.
- **Complexity:** LOW — fastest tests in the suite.
- **Wait time:** < 5 seconds per test.

### Wait (waits)
- **Behavior:** Function suspends for specified duration, resumes automatically. Must not consume compute during wait.
- **Test strategy:** Deploy a test variant with `ctx.wait("test_wait", 5)` (5 seconds, not 60). Invoke asynchronously. Poll `GetDurableExecution` until `SUCCEEDED`. Verify response contains both `started` and `completed` fields.
- **Complexity:** LOW-MEDIUM — requires async invocation + polling.
- **Wait time:** ~10-15 seconds per test (5s wait + resume + poll).

### Callback (callbacks)
- **Behavior:** Function creates a callback, suspends indefinitely until `SendDurableExecutionCallbackSuccess` or `SendDurableExecutionCallbackFailure` is called externally.
- **Test strategy:** (1) Invoke asynchronously. (2) Poll `GetDurableExecution` until execution has an operation with type `CALLBACK` and status `WAITING`. (3) Extract `callback_id` from operation details. (4) Call `aws lambda send-durable-execution-callback-success --callback-id <id> --result '{"approved":true}'`. (5) Poll until `SUCCEEDED`. (6) Assert result contains `approved: true`.
- **Complexity:** HIGH — multi-step async test with state inspection.
- **Wait time:** 30-90 seconds depending on polling interval.

### Invoke (invoke)
- **Behavior:** Caller suspends while callee executes. Caller resumes when callee returns.
- **Test strategy:** Pre-deploy a simple callee function named `durable-test-order-enrichment` that immediately returns `{"enriched": true}`. Update the invoke handler to accept the callee function name from the event payload. Invoke caller synchronously (callee is fast). Assert response contains `enrichment.enriched: true`.
- **Complexity:** MEDIUM — requires callee deployment + handler parameterization.
- **Wait time:** < 10 seconds per test.

### Parallel (parallel)
- **Behavior:** Fan-out to N branches, each with own `DurableContext`. All branches must complete before parent resumes.
- **Test strategy:** Invoke synchronously (all 3 branches complete quickly). Assert `parallel_results` array has length 3 and each branch's result appears.
- **Complexity:** LOW — branches are fast steps, no suspension.
- **Wait time:** < 10 seconds per test.

### Map (map)
- **Behavior:** Like parallel but over an input array. Each item processed in its own child context.
- **Test strategy:** Invoke synchronously with a small array (3 items). Assert result array has same length as input. Verify each item's result.
- **Complexity:** LOW.
- **Wait time:** < 10 seconds per test.

### Child Context (child_contexts)
- **Behavior:** Isolated operation namespace within parent execution. No suspension.
- **Test strategy:** Invoke synchronously. Assert response contains both `child_result` and `parent_result` fields without errors.
- **Complexity:** LOW.
- **Wait time:** < 5 seconds per test.

### Saga/Compensation (combined_workflow or dedicated test handler)
- **Behavior:** `step_with_compensation` registers forward steps and their rollbacks. `run_compensations` fires rollbacks in reverse order.
- **Test strategy:** The existing combined_workflow handlers demonstrate the happy path. A dedicated `test-saga-failure` handler is needed that: (1) registers 3 compensations successfully, (2) intentionally returns an error from step 4, (3) calls `run_compensations`, (4) returns the compensation order in its response body. Test asserts compensation sequence is `[step_c, step_b, step_a]`.
- **Complexity:** HIGH — requires new test handler not in existing 44.
- **Wait time:** < 10 seconds per test.

---

## Infrastructure Sizing

| Resource | Count | Notes |
|----------|-------|-------|
| ECR repositories | 4 | One per API style: closure, macro, trait, builder |
| Docker images | 44 | One per binary, tagged as `{style}:{binary-name}` |
| Lambda functions | 44 + 2 | 44 example handlers + callee function + saga test handler |
| IAM roles | 1 | Shared durable execution role, used by all test functions |
| IAM policy attachments | 1 | `AWSLambdaBasicDurableExecutionRolePolicy` |
| Lambda aliases | 46 | One per function pointing to `$LATEST` (required for durable invocations) |

**Total estimated cost per full test run:** $1-3 (Lambda invocations at $0.0000002/request + ECR storage ~$0.10/GB/month + durable state retention 7 days)

---

## Competitor Feature Analysis

| Feature | Python SDK (Official) | JS SDK (Official) | Our Approach |
|---------|-----------------------|-------------------|--------------|
| Unit test infrastructure | `pytest` + mock context (built-in SDK) | Jest + mock context (built-in SDK) | `MockDurableContext` (built) |
| Integration test tooling | SAM CLI local durable execution + `sam local callback succeed` | SAM CLI + `sam local callback succeed` | Real AWS + custom polling helper |
| Container image deployment | Docker + ECR (documented pattern) | Docker + ECR (documented pattern) | Dockerfile exists; Terraform manages ECR + Lambda |
| Callback testing | `sam local callback succeed <id>` for local; AWS CLI for cloud | Same | AWS CLI `send-durable-execution-callback-success` directly |
| Execution inspection | SAM console + `GetDurableExecution` API | Same | `GetDurableExecution` polled in test harness |

---

## Sources

- [AWS Lambda Durable Functions Documentation](https://docs.aws.amazon.com/lambda/latest/dg/durable-functions.html) — HIGH confidence, official AWS docs
- [GetDurableExecution API Reference](https://docs.aws.amazon.com/lambda/latest/api/API_GetDurableExecution.html) — HIGH confidence, official API reference
- [ListDurableExecutionsByFunction CLI](https://docs.aws.amazon.com/cli/latest/reference/lambda/list-durable-executions-by-function.html) — HIGH confidence, official CLI docs
- [send-durable-execution-callback-success CLI](https://docs.aws.amazon.com/cli/latest/reference/lambda/send-durable-execution-callback-success.html) — HIGH confidence, official CLI docs
- [Terraform AWS Provider durable_config (v6.25.0)](https://github.com/hashicorp/terraform-provider-aws/issues/45354) — HIGH confidence, issue confirms release
- [Deploy Lambda durable functions with IaC](https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-iac.html) — HIGH confidence, official AWS docs
- [Configure Lambda durable functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-configuration.html) — HIGH confidence, official AWS docs
- [Invoking durable Lambda functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-invoking.html) — HIGH confidence, official AWS docs
- [SAM Testing and Debugging Durable Functions](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/test-and-debug-durable-functions.html) — MEDIUM confidence, SAM docs

---

*Feature research for: AWS Lambda Durable Execution integration testing infrastructure*
*Researched: 2026-03-17*
