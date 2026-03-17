# Pitfalls Research

**Domain:** AWS Lambda Durable Execution — Integration Testing Infrastructure (Rust SDK)
**Researched:** 2026-03-17
**Confidence:** HIGH (official AWS docs + GitHub issues + community verification)

---

## Critical Pitfalls

### Pitfall 1: durable_config Cannot Be Added to Existing Lambda Functions

**What goes wrong:**
Terraform `apply` silently destroys and recreates all 44 Lambda functions when `durable_config` is added after initial creation. Any in-flight durable executions are orphaned. The re-creation also rotates function ARNs unless aliases are used, breaking any downstream references.

**Why it happens:**
The `DurableConfig` property (both in Terraform and CloudFormation) is a creation-only attribute. AWS does not support enabling durable execution on an already-deployed function — the function must be created with durable execution enabled. Developers often prototype without it, then try to retrofit.

**How to avoid:**
Define `durable_config` in every `aws_lambda_function` resource from the first `terraform apply`. Never deploy the 44 functions without it, even for a quick smoke test. Use `terraform plan` output to verify no `forces replacement` annotations appear on Lambda function resources before applying.

```hcl
resource "aws_lambda_function" "example" {
  # ...
  durable_config {
    execution_timeout    = 900   # 15 minutes max per invocation
    retention_period     = 7     # days to retain checkpoint data
  }
}
```

**Warning signs:**
- `terraform plan` shows `# aws_lambda_function.X must be replaced` when only adding `durable_config`
- Applying against already-deployed stack with no `durable_config` block

**Phase to address:**
Terraform infrastructure phase (phase that creates all Lambda resources). Define `durable_config` in the module template before any `terraform apply` runs.

---

### Pitfall 2: Lambda Invocation Without Qualified ARN Fails for Durable Functions

**What goes wrong:**
Test harness invokes Lambda by function name only (e.g., `closure-basic-steps`). AWS rejects the call with `InvalidParameterValueException: Durable functions require a qualified ARN`. All 44 integration tests fail immediately with this error, which is easy to misdiagnose as a permissions problem.

**Why it happens:**
Durable functions require a version or alias qualifier on every invocation. An unqualified ARN (function name alone, or `arn:aws:lambda:region:account:function:name` without `:version` suffix) is explicitly rejected by the durable execution API. The `$LATEST` qualifier is accepted but breaks replay determinism when code changes between invocations.

**How to avoid:**
Create an alias (e.g., `live`) for every Lambda function in Terraform and always invoke via the alias ARN. Use `$LATEST` only during initial development. Structure the test harness to build qualified ARNs:

```hcl
resource "aws_lambda_alias" "live" {
  name             = "live"
  function_name    = aws_lambda_function.example.function_name
  function_version = aws_lambda_function.example.version
}
```

Test harness invocation: `arn:aws:lambda:us-east-2:ACCOUNT:function:closure-basic-steps:live`

**Warning signs:**
- `InvalidParameterValueException` on `InvokeWithResponseStream` or standard `Invoke` calls
- Test harness using bare function names from Terraform outputs without `:alias` suffix

**Phase to address:**
Terraform infrastructure phase. Every Lambda resource must have a companion `aws_lambda_alias` resource. Test harness must read alias ARNs from Terraform outputs, not bare function ARNs.

---

### Pitfall 3: Terraform ResourceConflictException When Deploying 44 Functions in Parallel

**What goes wrong:**
`terraform apply` with default parallelism (`-parallelism=10`) triggers concurrent Lambda creation and update operations. AWS returns `ResourceConflictException: An update is in progress for resource` on roughly 20-30% of functions, causing partial apply failures. Re-running `terraform apply` generally resolves it but the non-determinism is disruptive.

**Why it happens:**
AWS Lambda has internal throttling on concurrent modification operations per account/region. When Terraform creates 44 Lambda functions, their associated IAM role attachments, and aliases simultaneously, the Lambda control plane rejects some concurrent updates. This is a known upstream issue in the Terraform AWS provider (hashicorp/terraform-provider-aws#5154, #38755).

**How to avoid:**
Reduce Terraform parallelism to avoid hitting Lambda's internal concurrency limits:

```bash
terraform apply -parallelism=5
```

Alternatively, use `depends_on` to serialize Lambda creation after the shared IAM role is fully attached, and group functions into waves of 10 or fewer.

**Warning signs:**
- `ResourceConflictException` in `terraform apply` output
- Some functions created successfully, others failed in same apply run
- Errors resolve on re-run with no code changes

**Phase to address:**
Terraform infrastructure phase. Document `-parallelism=5` as the required flag in the deployment script. Add a `make deploy` target that wraps `terraform apply -parallelism=5`.

---

### Pitfall 4: Docker Build Rebuilds Full Workspace for Every Binary Change

**What goes wrong:**
The current `Dockerfile` runs `cargo build --release -p "${PACKAGE}"` with a full workspace `COPY . .` before building. Any source file change — even in an unrelated crate — invalidates the Docker layer cache and triggers a full Rust recompile. Building 44 images sequentially takes 4-8 hours. Even with layer caching, a single `Cargo.lock` change rebuilds everything.

**Why it happens:**
Docker layer caching invalidates on any `COPY` that touches changed files. Since the entire workspace is copied in one layer, any `.rs` file change anywhere invalidates the build cache. Rust's incremental compilation does not carry across Docker build invocations without explicit cache mounts.

**How to avoid:**
Use `cargo-chef` for dependency layer caching and Docker BuildKit cache mounts for Rust's `target/` directory:

```dockerfile
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /usr/src/app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG PACKAGE=closure-basic-steps
COPY --from=planner /usr/src/app/recipe.json recipe.json
# Build dependencies only (cached layer — only invalidated by Cargo.toml/Cargo.lock changes)
RUN cargo chef cook --release --recipe-path recipe.json
# Build source (only invalidated by source changes)
COPY . .
RUN cargo build --release -p "${PACKAGE}"
```

Additionally, use `--build-arg PACKAGE=` to build only the needed binary per image, not the full workspace.

**Warning signs:**
- Docker build time exceeds 10 minutes for a single binary on second build
- `cargo build` output shows "Compiling aws-sdk-lambda" on every build (dependency cache miss)
- Build log shows no `Using cache` lines in the dependency compilation stage

**Phase to address:**
Docker build pipeline phase. Establish the cargo-chef Dockerfile pattern before building any images. Also consider a single build container that compiles all 44 binaries once using `cargo build --release --workspace`, then copies individual artifacts into 44 separate runtime images — one compilation, 44 images.

---

### Pitfall 5: ADFS Credential Expiry Mid-Test Run

**What goes wrong:**
ADFS temporary credentials typically expire after 1-4 hours. A full integration test run covering 44 Lambda functions with durable execution suspend/resume cycles (some callbacks wait minutes) can easily exceed that window. AWS SDK calls fail with `ExpiredTokenException`, test suite reports failures that are actually credential issues, and half-run state is left in AWS (deployed functions, orphaned checkpoints, dangling ECR images).

**Why it happens:**
ADFS-issued STS credentials have a session duration set by the identity provider, typically 1-4 hours. Long test runs — especially any test that sleeps waiting for a callback or wait operation — consume session time. The Rust AWS SDK caches credentials and does not transparently refresh ADFS credentials (it cannot, without re-authenticating against ADFS).

**How to avoid:**
- Structure the test run so infrastructure deploy (`terraform apply`) and test execution are separate phases. Refresh credentials before each phase.
- Add a credential validity check at the start of the test harness: invoke `aws sts get-caller-identity` and verify the token expiry is at least 30 minutes out before starting.
- Keep durable execution timeout values in integration tests short (under 2 minutes). Use `timeout_seconds` on step operations; never let callback tests wait longer than 5 minutes.
- Design the test harness to emit a clear `CREDENTIAL_EXPIRED` exit code distinct from test failures.

```bash
# At test harness startup
EXPIRY=$(aws sts get-caller-identity --query 'Credentials.Expiration' --output text 2>/dev/null)
REMAINING=$(( $(date -d "$EXPIRY" +%s) - $(date +%s) ))
if [ "$REMAINING" -lt 1800 ]; then
  echo "ERROR: ADFS credentials expire in less than 30 minutes. Refresh before running." >&2
  exit 1
fi
```

**Warning signs:**
- Test failures that appear as `ExpiredTokenException` or `UnrecognizedClientException` in AWS SDK errors
- `terraform apply` or `terraform destroy` failing mid-run with auth errors
- All tests failing simultaneously after a successful partial run (batch expiry)

**Phase to address:**
Test harness phase. Build the credential validity gate as the first step of both `make deploy` and `make test`. Document the 1-4 hour window in the developer runbook.

---

### Pitfall 6: Callback Tests Are Non-Deterministic Without Explicit Sequencing

**What goes wrong:**
Tests for `ctx.wait_for_callback()` invoke the Lambda (which suspends waiting for a callback signal), then immediately call `SendDurableExecutionCallbackSuccess`. If the callback signal arrives before the Lambda has fully checkpointed the `WaitForCallback` operation, the signal is lost or ignored, and the Lambda waits indefinitely. Tests time out rather than completing.

**Why it happens:**
Lambda durable execution's `waitForCallback` flow is: (1) Lambda checkpoints callback token, (2) Lambda suspends (execution terminates), (3) external system calls `SendDurableExecutionCallbackSuccess` with the token, (4) Lambda is re-invoked and resumes. If step (3) runs before step (1) completes — which happens when a test calls the callback API immediately after invoking the Lambda — the signal arrives before the checkpoint exists. The callback is dropped.

**How to avoid:**
Poll `GetDurableExecution` status until the execution reaches `SUSPENDED` state before sending the callback signal. Never send a callback signal based on a fixed sleep duration:

```bash
# Wait for SUSPENDED state before sending callback
EXECUTION_ID="test-$(uuid)"
aws lambda invoke ... --execution-id "$EXECUTION_ID" &

# Poll until suspended (not a fixed sleep)
for i in $(seq 1 30); do
  STATUS=$(aws lambda get-durable-execution \
    --function-name "closure-callbacks:live" \
    --execution-id "$EXECUTION_ID" \
    --query 'Status' --output text)
  [ "$STATUS" = "SUSPENDED" ] && break
  sleep 2
done

aws lambda send-durable-execution-callback-success \
  --function-name "closure-callbacks:live" \
  --execution-id "$EXECUTION_ID" \
  --callback-result '{"approved": true}'
```

**Warning signs:**
- Callback tests pass when run slowly (debugger), fail when run at full speed
- Lambda executions stuck in `RUNNING` state for minutes after callback is sent
- Tests that intermittently timeout on first run but pass on retry

**Phase to address:**
Test harness phase. Implement a `wait_for_suspended` helper function used by all callback test cases. Never use `sleep` as a substitute for polling execution state.

---

### Pitfall 7: $LATEST Qualifier Breaks Replay Determinism During Active Development

**What goes wrong:**
Tests invoke Lambda using `$LATEST`. A durable execution starts, runs three steps, then the developer deploys a code change (updating step names or reordering operations). The in-flight execution resumes using the new code. The replay engine tries to match checkpointed operations by position to the new operation sequence, produces incorrect results, or panics. Tests that were passing silently regress.

**Why it happens:**
`$LATEST` always points to the most recent code. When Lambda replays from a checkpoint, it runs the new code with old checkpoint data. If operation ordering has changed since the checkpoint was written, the replay engine replays operations against the wrong checkpoints — a fundamentally broken state.

**How to avoid:**
Always invoke integration tests via a named alias (`live`, `test`, `v1`). Publish a new version and update the alias atomically as part of each deployment. Tests reference the alias ARN; they never hardcode `$LATEST`.

For test teardown: delete (or wait for expiry of) all durable executions before redeploying code that changes operation ordering. The Terraform `retention_period` of 7 days means old checkpoints survive across deployments.

**Warning signs:**
- Tests pass in isolation but fail when run after a deployment of changed operation ordering
- `DurableError::ReplayMismatch` errors in Lambda logs during integration tests
- Inconsistent results between first test run and re-run without code changes

**Phase to address:**
Terraform infrastructure phase (alias creation) and test harness phase (always use alias ARNs).

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Use `$LATEST` for Lambda invocations | Faster initial setup, no alias management | Replay breaks when code changes mid-execution; flaky tests in CI | Never in integration tests; only during local smoke testing of a single function |
| Single shared IAM role for all 44 functions | One role to manage | Over-permissioned; any function compromise reaches all functions' resources | Never in production; acceptable for internal development milestone if scope is reviewed later |
| Skip ECR lifecycle policies | No cleanup setup time | Storage costs accumulate; 44 repos × multiple tags × large Rust images = significant monthly cost | Never; add lifecycle policies from day one |
| Hard-coded `sleep 10` in callback tests | Simpler test code | Race condition on slow networks or cold starts; tests are non-deterministic | Never; always poll execution state |
| Building all 44 images in a single Docker layer | Simpler Dockerfile | Full recompile on any source change; 60+ minute build on cache miss | Never; use cargo-chef from the start |
| Skip `terraform destroy` in CI teardown | Faster CI | Resources persist after failed runs; cost accumulates; stale state conflicts next run | Never in CI; always destroy in teardown |
| `cargo build --workspace` inside Docker | Simpler build command | Builds all 60+ binary targets (including test crates) when only 44 are needed | Never; always `-p package-name` per image |
| CloudWatch log retention set to "Never expire" | No log loss | Silent cost accumulation; 44 functions × high invocation frequency = significant storage | Never; set 7-30 day retention on all log groups |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| ECR + Terraform | Pushing Docker image after `terraform apply` but the ECR repo was just created in the same apply | Separate `terraform apply` into two phases: first create ECR repos (`-target=aws_ecr_repository`), then push images, then `terraform apply` for Lambda functions |
| Docker + Rust + al2023 | Building with `rust:bookworm` (Debian glibc 2.36) but targeting `provided.al2023` (glibc 2.34) — binary runs fine; reverse is the problem | Build inside `public.ecr.aws/lambda/provided:al2023` or `rust:1-bookworm` — Debian glibc 2.36 produces binaries that require glibc ≥ 2.36, which is newer than al2023's 2.34. Must build on a system with glibc ≤ 2.34 or use musl static linking |
| Docker + Rust + musl | Using `x86_64-unknown-linux-musl` target but not installing musl-tools, causing link failures | Either use `rust:1-alpine` base image (musl native), or install `musl-tools` in the builder stage, or use `cross` crate for cross-compilation |
| Terraform + ECR | Terraform manages ECR repos but test images are pushed outside Terraform; `terraform destroy` leaves images behind (destroy fails on non-empty repo) | Add `force_delete = true` to `aws_ecr_repository` resources, or script image cleanup before `terraform destroy` |
| AWS SDK (Rust) + ADFS | `AWS_PROFILE=adfs` is set but the SDK uses `aws-config` with `BehaviorVersion::latest()` which respects the profile — however, `~/.aws/credentials` ADFS entries use `aws_session_token` which expires silently | Always call `aws sts get-caller-identity` before any AWS SDK operation in integration tests to fail fast on expired credentials |
| Lambda + durable execution + Terraform provider version | Using AWS Terraform provider < 6.25.0; `durable_config` block is not recognized, silently ignored, Terraform plan shows no errors but functions are deployed without durable execution enabled | Pin `required_providers { aws = ">= 6.25.0" }` in the Terraform configuration |
| Callback API + execution ID | Using the Lambda function invocation `RequestId` as the durable execution ID — these are different identifiers | The durable execution ID is the `executionId` you supply when invoking, or the one Lambda auto-generates. The `RequestId` in the Lambda response is the HTTP request ID. Use `GetDurableExecution` to look up by `executionId` |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Building 44 Docker images sequentially without cache | CI pipeline takes 3-8 hours per run; developers abandon integration tests | Use cargo-chef for layer caching; build all binaries in one `cargo build --workspace` pass, then copy into 44 runtime images | From first build without caching optimization |
| Cold starts adding variance to test timing | Tests with 100ms timeout assertions fail intermittently; Rust Lambda cold starts on container images are 1-3 seconds | Add a warm-up invocation before each timed test (invoke once, discard result, then measure) or use provisioned concurrency for timing-sensitive tests | Every test run that exercises a freshly-deployed or idle function |
| `cargo build --release` for every Docker image | Workspace recompiles all 6 library crates + all 44 example binary crates for each image build | Build all binaries once: `cargo build --release --workspace`, copy each binary to its image; or use a matrix Docker build that caches the compiled artifacts volume | Any build pipeline that runs `docker build` 44 times independently |
| CloudWatch Logs accumulation from 44 functions during test runs | AWS bill grows unexpectedly week over week; `aws logs describe-log-groups` shows hundreds of groups | Set 7-day retention on all CloudWatch log groups via Terraform `aws_cloudwatch_log_group` resource | After ~2 weeks of active testing without retention policies |
| ECR image storage from parallel builds | Multiple CI runs push different tags; ECR accumulates large Rust images (300-500 MB each × 44 repos = 13-22 GB per build cycle) at $0.10/GB-month | Apply ECR lifecycle policies keeping only 3 most recent images per repo; add `force_delete = true` to repos | After 5-10 CI runs without cleanup |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Single IAM role for all 44 Lambda functions with wildcard resource permissions | If one function is compromised via malicious input, it can invoke all other functions, read all checkpoint data, and call `CheckpointDurableExecution` on any execution | Create one IAM role per function; scope `lambda:CheckpointDurableExecution` and `lambda:GetDurableExecutionState` to only that function's ARN |
| Attaching `AdministratorAccess` to Lambda execution roles during development | A compromised Lambda function has full AWS account access; can create IAM users, exfiltrate S3 data, spin up EC2 instances | Use `AWSLambdaBasicDurableExecutionRolePolicy` as the baseline; add only explicitly needed permissions. Use IAM Access Analyzer to verify minimum required permissions |
| Storing ADFS credentials in environment variables inside Docker containers | Credentials baked into image layers are recoverable from ECR with sufficient IAM access | Never `ENV AWS_ACCESS_KEY_ID=...` in Dockerfiles; credentials must come from the runtime environment (mounted at test run time) |
| Committing `terraform.tfstate` to git | State file contains all Lambda ARNs, role ARNs, execution role credentials in plaintext | Use S3 remote backend with DynamoDB locking; never commit `.tfstate` files |
| Lambda function URLs without auth for test callback endpoints | Any internet user can send callback signals to your test Lambda functions, corrupting test state | Use `LAMBDA_URL_AUTH_TYPE = "AWS_IAM"` if Lambda URLs are used; prefer invoking via SDK with ADFS credentials |
| Over-broad `lambda:InvokeFunction` in test runner IAM policy | Test runner can invoke any Lambda in the account, not just the 44 test functions | Scope invoke permissions to specific function ARN patterns: `arn:aws:lambda:us-east-2:ACCOUNT:function:closure-*` etc. |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Test output shows only pass/fail with no Lambda logs on failure | Developer must manually `aws logs tail` to diagnose a failing test, extending debug cycle from minutes to tens of minutes | Test harness automatically fetches the last N log lines from the Lambda function's CloudWatch log group when a test fails, and prints them inline |
| `make test` runs all 44 tests sequentially, taking 20+ minutes | Developer waits 20 minutes to know if their change broke anything | Add `make test-fast` target that runs only the closure-style tests (11 functions) as a quick smoke test; full suite is separate |
| No test isolation — tests share execution IDs | Test A's checkpoint data interferes with Test B if both use the same execution ID, producing misleading failures | Generate a unique `executionId` per test run (e.g., `test-$(date +%s)-$(uuidgen)`) to ensure isolation |
| Terraform outputs don't include alias ARNs | Test harness developer must manually construct qualified ARNs | Output both `function_arn` and `alias_arn` for every function from Terraform; test harness reads `alias_arn` directly |
| No single-command teardown | Developer finishes testing, forgets to destroy resources; weekly bill accumulates | Add `make clean` that runs `terraform destroy -auto-approve`; print cost estimate after deploy with `infracost` or manual calculation |

---

## "Looks Done But Isn't" Checklist

- [ ] **Lambda durable execution enabled:** Check each function actually has `durable_config` block — Terraform provider versions < 6.25.0 silently ignore it. Verify with `aws lambda get-function-configuration --function-name NAME | jq .DurableConfig`
- [ ] **Function aliases created:** `terraform output` should show qualified ARNs ending in `:live` for all 44 functions. An alias-less deploy means invocations will fail.
- [ ] **IAM policy attached:** `AWSLambdaBasicDurableExecutionRolePolicy` must appear in `aws iam list-attached-role-policies`. A function without it fails on first checkpoint with `AccessDenied`.
- [ ] **ECR lifecycle policies set:** `aws ecr get-lifecycle-policy --repository-name NAME` should return a policy. No policy = unbounded image accumulation.
- [ ] **CloudWatch log group retention set:** `aws logs describe-log-groups --log-group-name-prefix /aws/lambda/closure` should show non-null `retentionInDays` for all groups.
- [ ] **Test isolation verified:** Two concurrent test runs for the same function must use different `executionId` values. Verify the test harness generates unique IDs per invocation.
- [ ] **Callback test sequencing verified:** Callback tests must poll for `SUSPENDED` status before sending callback signal. A test that uses `sleep 5` instead of polling is unreliable.
- [ ] **Docker build uses cargo-chef:** `docker history IMAGE` should show a cached dependency layer that does not change when only source (not Cargo.toml) changes.
- [ ] **ADFS credential check at test startup:** Running `make test` with expired credentials should fail immediately with a clear error, not with 44 `ExpiredTokenException` test failures.
- [ ] **Terraform remote state configured:** `terraform.tfstate` must not exist locally; verify `.terraform/terraform.tfstate` points to S3 backend.

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| durable_config missing on deployed functions | HIGH | `terraform destroy` all Lambda functions, then `terraform apply` with `durable_config` blocks — no in-place update possible |
| Orphaned durable executions blocking teardown | MEDIUM | Call `aws lambda list-durable-executions` per function, wait for all to reach terminal state (`SUCCEEDED`/`FAILED`), then destroy. Or set short `retention_period` and wait for auto-expiry |
| ADFS credentials expired mid-test | LOW | Re-authenticate via ADFS, export new credentials, re-run failed tests with `make test RESUME=true` if harness supports resume |
| ECR repos blocked from deletion by non-empty images | LOW | `aws ecr list-images --repository-name NAME | jq` to list image digests, `aws ecr batch-delete-image` to remove all, then `terraform destroy` |
| Terraform state drift after manual Lambda changes | MEDIUM | `terraform import` the drifted resource back into state, or `terraform state rm` and re-apply |
| ResourceConflictException left 20 functions uncreated | LOW | `terraform apply -parallelism=5` (re-run); Terraform's plan will only attempt the failed resources |
| Docker build cache corrupted (wrong glibc linked) | MEDIUM | `docker buildx prune --all` to clear builder cache, then rebuild with explicit `--platform linux/amd64` if on ARM host |
| Callback test left execution in SUSPENDED state | LOW | `aws lambda send-durable-execution-callback-failure` with the execution ID and any payload to unblock it, then wait for `FAILED` terminal state |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| durable_config creation-only | Infrastructure phase (Terraform Lambda resources) | `aws lambda get-function-configuration` on all 44 functions shows `DurableConfig` block |
| Unqualified ARN invocation failure | Infrastructure phase (alias creation) + test harness phase (ARN construction) | Test harness reads alias ARNs from Terraform outputs; `terraform output` shows `:live` suffix on all ARNs |
| Terraform ResourceConflictException (44 functions) | Infrastructure phase (deployment script) | `make deploy` uses `-parallelism=5`; zero `ResourceConflictException` in apply output |
| Docker build cache invalidation | Docker build pipeline phase | Second `docker build` with no source changes completes in under 60 seconds |
| ADFS credential expiry | Test harness phase | `make test` fails fast with `CREDENTIAL_EXPIRED` when session has < 30 minutes remaining |
| Callback race condition | Test harness phase (callback test implementation) | Callback tests pass 10/10 times in parallel execution |
| $LATEST replay divergence | Infrastructure phase (alias creation) + test harness (alias ARN usage) | Deploying new code does not cause in-flight test executions to fail |
| IAM over-permissioning | Infrastructure phase (IAM role design) | `aws iam simulate-principal-policy` confirms each role can only access its own function's checkpoint API |
| ECR image accumulation | Infrastructure phase (lifecycle policies) | `aws ecr describe-images` shows max 3 images per repo after 5 CI runs |
| CloudWatch log cost | Infrastructure phase (log group resources) | All 44 `/aws/lambda/` log groups have `retentionInDays <= 30` |
| Terraform state committed to git | Infrastructure phase (backend config) | `.gitignore` includes `*.tfstate`; `git log --all -- '*.tfstate'` returns empty |

---

## Sources

- [Security and permissions for Lambda durable functions — AWS Lambda docs](https://docs.aws.amazon.com/lambda/latest/dg/durable-security.html) (HIGH confidence)
- [Best practices for Lambda durable functions — AWS Lambda docs](https://docs.aws.amazon.com/lambda/latest/dg/durable-best-practices.html) (HIGH confidence)
- [Deploy Lambda durable functions with Infrastructure as Code — AWS Lambda docs](https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-iac.html) (HIGH confidence)
- [Invoking durable Lambda functions — AWS Lambda docs](https://docs.aws.amazon.com/lambda/latest/dg/durable-invoking.html) (HIGH confidence)
- [Lambda durable functions basic concepts — AWS Lambda docs](https://docs.aws.amazon.com/lambda/latest/dg/durable-basic-concepts.html) (HIGH confidence)
- [ResourceConflictException on concurrent Lambda updates — terraform-provider-aws #5154](https://github.com/hashicorp/terraform-provider-aws/issues/5154) (HIGH confidence — open known issue)
- [cargo-chef workspace monorepo build time regression — cargo-chef #273](https://github.com/LukeMathWalker/cargo-chef/issues/273) (HIGH confidence — documented issue)
- [5x Faster Rust Docker Builds with cargo-chef — Luca Palmieri](https://lpalmieri.com/posts/fast-rust-docker-builds/) (HIGH confidence — cargo-chef author)
- [Lambda durable functions Terraform support request — terraform-provider-aws #45354](https://github.com/hashicorp/terraform-provider-aws/issues/45354) (MEDIUM confidence — issue thread, provider 6.25.0 requirement confirmed)
- [Rust in AL2023 — Amazon Linux 2023 docs](https://docs.aws.amazon.com/linux/al2023/ug/rust.html) (HIGH confidence — official docs, glibc 2.34 version confirmed)
- [ECR lifecycle policies — AWS automated cleanup blog](https://aws.amazon.com/blogs/compute/automated-cleanup-of-unused-images-in-amazon-ecr/) (HIGH confidence)
- [IAM temporary credentials — IAM user guide](https://docs.aws.amazon.com/IAM/latest/UserGuide/id_credentials_temp.html) (HIGH confidence — ADFS expiry mechanics)

---
*Pitfalls research for: AWS Lambda Durable Execution integration testing infrastructure (Rust SDK)*
*Researched: 2026-03-17*
