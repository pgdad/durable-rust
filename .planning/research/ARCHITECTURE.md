# Architecture Research

**Domain:** AWS Lambda integration testing infrastructure for a Rust SDK
**Researched:** 2026-03-17
**Confidence:** HIGH (official AWS docs + direct codebase inspection)

---

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                         Developer Workstation                         │
│                                                                       │
│  durable-rust/                                                        │
│  ├── infra/                  ← Terraform (new)                        │
│  │   ├── main.tf             ← ECR repo + all 44 Lambda functions     │
│  │   ├── iam.tf              ← shared execution role                  │
│  │   ├── variables.tf                                                 │
│  │   └── outputs.tf                                                   │
│  ├── scripts/                ← Shell tooling (new)                    │
│  │   ├── build-images.sh     ← Docker build + push for all 44 bins   │
│  │   └── test-all.sh         ← end-to-end test runner                 │
│  └── examples/               ← existing 4 × 11 binary crates         │
└────────────────────────────────────┬─────────────────────────────────┘
                                     │  docker push / terraform apply
                                     ▼
┌──────────────────────────────────────────────────────────────────────┐
│                            AWS (us-east-2)                            │
│                                                                       │
│  ECR: durable-rust-examples                                           │
│  ├── :closure-basic-steps                                             │
│  ├── :closure-callbacks                                               │
│  ├── :closure-invoke                                                  │
│  ├── :macro-basic-steps   ... (44 tags total)                        │
│  └── :builder-combined-workflow                                       │
│                                                                       │
│  IAM Role: durable-rust-lambda-role (AWSLambdaBasicDurableExecRole)  │
│                                                                       │
│  Lambda Functions (44):                                               │
│  ├── durable-rust-closure-basic-steps:1 (alias: live)                │
│  ├── durable-rust-closure-callbacks:1                                 │
│  ├── durable-rust-closure-invoke:1 ─────────────────┐                │
│  ├── durable-rust-macro-invoke:1  ──────────────────┤                │
│  │                                                   │ invokes        │
│  ├── order-enrichment-lambda:1  ◄───────────────────┘                │
│  ├── fulfillment-lambda:1       ◄─── combined-workflow invoke target  │
│  └── ...                                                              │
│                                                                       │
│  AWS Lambda Durable Execution Service                                 │
│  └── checkpoint_durable_execution / get_durable_execution_state      │
└──────────────────────────────────────────────────────────────────────┘
                                     │
                         test-all.sh │ aws lambda invoke + poll
                                     ▼
                          per-test PASS / FAIL report
```

### Component Responsibilities

| Component | Responsibility | Implementation |
|-----------|----------------|----------------|
| `infra/` | Declare all AWS resources as code | Terraform HCL |
| `scripts/build-images.sh` | Build + tag + push all 44 Docker images | Bash + Docker CLI |
| `scripts/test-all.sh` | Invoke each function, poll status, report | Bash + AWS CLI |
| ECR repo (single) | Store all 44 tagged images | `aws_ecr_repository` |
| IAM execution role (shared) | Grant checkpoint + CloudWatch permissions | `aws_iam_role` with managed policy |
| Lambda functions (44) | Run example handlers with durable execution | `aws_lambda_function` + `aws_lambda_alias` |
| Stub targets (2) | Serve as invoke/fulfillment targets | Minimal Lambda functions |

---

## Recommended Project Structure

```
durable-rust/
├── infra/                          # Terraform — ALL new infra lives here
│   ├── main.tf                     # ECR repo + aws_lambda_function for_each
│   ├── iam.tf                      # shared execution role + policy attachment
│   ├── variables.tf                # account_id, region, image_tag, env
│   ├── outputs.tf                  # function ARNs, alias ARNs, ECR repo URL
│   └── stubs/                      # stub handler sources for invoke targets
│       ├── order_enrichment/
│       │   └── main.py             # minimal Python durable stub
│       └── fulfillment/
│           └── main.py
├── scripts/
│   ├── build-images.sh             # build + push all 44 images (or subset)
│   └── test-all.sh                 # invoke all functions + poll + report
└── examples/                       # existing — not modified
    ├── Dockerfile                  # existing — parameterised with BINARY_NAME
    ├── closure-style/
    ├── macro-style/
    ├── trait-style/
    └── builder-style/
```

### Structure Rationale

- **`infra/` not `deploy/`:** The directory holds infrastructure declaration (what exists), not deployment scripts (how to run). Terraform itself is the deployment mechanism. `infra/` is the Rust ecosystem convention (consistent with aws-sdk-rust, the official AWS CDK naming, and community Terraform repos). `deploy/` implies imperative scripts; Terraform is declarative.
- **Single `infra/` flat module, not `infra/modules/`:** 44 Lambda functions deployed from one ECR repo is a single concern. A child module would add indirection with no benefit at this scale. Reach for modules when the same pattern repeats across multiple environments.
- **`scripts/` at workspace root:** Scripts orchestrate the build-push-test cycle. They are not Rust code and do not belong inside `infra/` (infra doesn't build images) or `examples/` (examples don't deploy themselves).
- **Stub handlers in `infra/stubs/`:** The `invoke` and `combined_workflow` examples call `order-enrichment-lambda` and `fulfillment-lambda`. These need to exist as real Lambda functions. Python stubs (5 lines each) are the simplest option — no Rust compile cycle, no additional Cargo workspace member, managed by Terraform alongside the rest of the infra.

---

## Architectural Patterns

### Pattern 1: Single ECR Repository, Per-Binary Tags

**What:** One ECR repository (`durable-rust-examples`) with 44 image tags, one per binary name. Each tag maps exactly to one Lambda function.

**When to use:** When all images share the same Dockerfile structure and the same base layer. This is true here — all 44 binaries are built from `examples/Dockerfile` with a single `BINARY_NAME` ARG.

**Trade-offs:**
- PRO: One IAM policy controls push access. One `aws_ecr_repository` resource. No ECR lifecycle rules to replicate 44 times.
- PRO: Image tags are self-documenting (`closure-basic-steps`, `macro-invoke`, etc.).
- CON: A stale tag scan would surface all 44. Not a concern for a testing repo.
- CON: Cannot set per-function image retention independently. Not needed here.

**Why not 44 separate repos:** Terraform would need 44 `aws_ecr_repository` resources. ECR lifecycle rules, IAM resource ARNs, and outputs multiply by 44 with no benefit.

```hcl
resource "aws_ecr_repository" "examples" {
  name                 = "durable-rust-examples"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = false   # testing repo — skip CVE scans for speed
  }
}
```

### Pattern 2: Terraform `for_each` Over a Handler Map

**What:** Define a local map of `{ "closure-basic-steps" = { style = "closure", handler = "basic-steps" }, ... }` and use `for_each` on `aws_lambda_function` and `aws_lambda_alias`.

**When to use:** When 44 resources share the same structure with only name/image-tag varying. This is exactly the case here.

**Trade-offs:**
- PRO: Adding a handler = adding one line to the map. Zero Terraform duplication.
- PRO: `terraform plan` shows exactly which functions will be created/updated.
- PRO: `terraform destroy` removes everything cleanly.
- CON: `for_each` on a large map makes `terraform plan` output verbose. Acceptable — this is a test environment.

```hcl
locals {
  handlers = {
    "closure-basic-steps"       = { style = "closure" }
    "closure-callbacks"         = { style = "closure" }
    "closure-invoke"            = { style = "closure" }
    # ... (44 entries total)
    "builder-combined-workflow" = { style = "builder" }
  }
}

resource "aws_lambda_function" "examples" {
  for_each = local.handlers

  function_name = "durable-rust-${each.key}"
  role          = aws_iam_role.lambda_exec.arn
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.examples.repository_url}:${each.key}"

  durable_config {
    execution_timeout  = 3600
    retention_period   = 7
  }

  timeout     = 900   # Lambda invocation timeout; durable_config.execution_timeout governs durable lifecycle
  memory_size = 256
}

resource "aws_lambda_alias" "live" {
  for_each         = local.handlers
  name             = "live"
  function_name    = aws_lambda_function.examples[each.key].function_name
  function_version = aws_lambda_function.examples[each.key].version
}
```

**Critical:** Durable functions require a qualified ARN (version number or alias). The `live` alias satisfies this. Test invocations must target `durable-rust-{name}:live`, not the unqualified function name.

### Pattern 3: Parameterised Dockerfile with Per-Binary Build Args

**What:** The existing `examples/Dockerfile` already accepts `ARG PACKAGE` for the crate name. Extend it with a second `ARG BINARY_NAME` to select which binary to install as `bootstrap`.

**When to use:** This is the only viable approach for 44 separate binaries without 44 separate Dockerfiles.

**Trade-offs:**
- PRO: One Dockerfile maintained in one place.
- PRO: Docker layer caching on the compile stage is shared: if the Rust workspace hasn't changed, `cargo build --release -p closure-style-example` rebuilds no source. When only `BINARY_NAME` changes, only the `COPY --from=builder` line reruns (trivially fast).
- CON: All binaries in a given style (e.g., `closure-style-example`) are compiled in one `cargo build` invocation, producing ~11 binaries. The script then iterates over binary names, copying the right binary out. This is actually optimal — one compile per style, not one per binary.

**Build strategy in `scripts/build-images.sh`:**

```bash
# Compile all 4 style packages once each, building all binaries per package.
# Then for each binary name, build the final Lambda image by copying the
# correct pre-compiled binary into the runtime layer.

STYLES=("closure-style-example" "macro-style-example" "trait-style-example" "builder-style-example")
REPO_URL="<account>.dkr.ecr.us-east-2.amazonaws.com/durable-rust-examples"

# Stage 1: compile all workspaces (4 separate docker build --target builder runs)
# or alternatively: cargo build --release --workspace on host to populate target/

# Stage 2: for each binary, docker build selecting binary from pre-compiled output
for BINARY in closure-basic-steps closure-callbacks ... builder-combined-workflow; do
  PACKAGE=$(package_for_binary "$BINARY")   # e.g. closure-basic-steps -> closure-style-example
  docker build \
    -f examples/Dockerfile \
    --build-arg PACKAGE="$PACKAGE" \
    --build-arg BINARY_NAME="$BINARY" \
    -t "${REPO_URL}:${BINARY}" .
  docker push "${REPO_URL}:${BINARY}"
done
```

**Layer caching note:** Build all binaries for one style before moving to the next style. Docker's build cache for the `cargo build` layer is keyed on `PACKAGE`, so builds within the same style hit cache after the first.

**Dockerfile modification needed:**

```dockerfile
ARG PACKAGE=closure-style-example
ARG BINARY_NAME=closure-basic-steps   # the specific binary within PACKAGE

FROM rust:1-bookworm AS builder
ARG PACKAGE
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release -p "${PACKAGE}"

FROM public.ecr.aws/lambda/provided:al2023
ARG BINARY_NAME
COPY --from=builder "/usr/src/app/target/release/${BINARY_NAME}" "${LAMBDA_RUNTIME_DIR}/bootstrap"
CMD ["handler"]
```

### Pattern 4: Lambda Invocation and Durable Execution Status Polling

**What:** The test harness invokes each Lambda function by its `live` alias, then polls `list-durable-executions-by-function` until the execution reaches `SUCCEEDED` or `FAILED`.

**When to use:** All tests except `callbacks` and `waits`. Those two require special handling (see below).

**Invocation and polling flow:**

```
1. aws lambda invoke --function-name durable-rust-{name}:live \
       --cli-binary-format raw-in-base64-out \
       --payload '{"test":true}' /tmp/response.json
   → HTTP 200 with immediate return value (synchronous, completes in < 15 min)
   OR
   → HTTP 202 if invoked as Event (async, for waits/callbacks that run > 15 min)

2. For async: poll list-durable-executions-by-function \
       --function-name durable-rust-{name} --qualifier live \
       --statuses RUNNING
   → Loop until RUNNING list is empty or timeout

3. Check final status:
   aws lambda list-durable-executions-by-function \
       --function-name durable-rust-{name} --qualifier live \
       --statuses SUCCEEDED FAILED TIMED_OUT
   → PASS if SUCCEEDED, FAIL otherwise
```

**Execution ARN format** (confirmed from AWS docs):
```
arn:aws:lambda:us-east-2:{account}:function:durable-rust-{name}:1/durable-execution/{exec-name}/{exec-id}
```

### Pattern 5: Callback Test Chaining

**What:** The `callbacks` handler calls `ctx.create_callback(...)` which suspends the execution waiting for an external signal. The test harness must:
1. Invoke the function asynchronously (Event type) — it suspends immediately
2. Retrieve the callback ID from the execution state
3. Call `send-durable-execution-callback-success` with the callback ID and a result payload
4. Poll for the execution to reach `SUCCEEDED`

**How to get the callback ID:** The callback ID is embedded in the durable execution operation state, accessible via `get-durable-execution-state`. The operation with type `Callback` and status `WAITING` contains the callback token.

```bash
# Step 1: invoke async
aws lambda invoke --function-name durable-rust-closure-callbacks:live \
    --invocation-type Event \
    --payload '{}' /tmp/out.json

# Step 2: wait for WAITING status, get execution ARN
EXEC_ARN=$(aws lambda list-durable-executions-by-function \
    --function-name durable-rust-closure-callbacks \
    --qualifier live \
    --statuses RUNNING \
    | jq -r '.DurableExecutions[0].DurableExecutionArn')

# Step 3: get callback ID from operation state
CALLBACK_ID=$(aws lambda get-durable-execution-state \
    --durable-execution-arn "$EXEC_ARN" \
    | jq -r '.Operations[] | select(.Type == "Callback") | .CallbackId')

# Step 4: send callback success
aws lambda send-durable-execution-callback-success \
    --callback-id "$CALLBACK_ID" \
    --body '{"approved": true}'

# Step 5: poll for SUCCEEDED
```

### Pattern 6: Invoke Test Chaining (caller → callee)

**What:** The `invoke` handlers call `ctx.invoke("enrich_order", "order-enrichment-lambda", ...)`. The callee function must exist and be invocable.

**Architecture decision:** Deploy two stub Lambda functions:
- `order-enrichment-lambda` — returns `{"enriched": true, "order_id": "<input>"}`
- `fulfillment-lambda` — returns `{"fulfillment_id": "ff-001", "status": "started"}`

These are minimal Python functions (no durable execution required for the callee — the durable execution is on the caller side). They live in `infra/stubs/` and are deployed by Terraform alongside the main functions.

**Critical:** The callee function name must match the string literal in each handler's source code. All 4 `invoke.rs` files use `"order-enrichment-lambda"` and all 4 `combined_workflow.rs` files use `"fulfillment-lambda"`. Deploy exactly these names.

---

## Data Flow

### Build → Push → Deploy → Test → Verify Cycle

```
[Rust source + Dockerfile]
        │
        │ scripts/build-images.sh
        │ (4 cargo builds, 44 docker builds, 44 docker pushes)
        ▼
[ECR: durable-rust-examples:{binary-name}]
        │
        │ terraform apply
        │ (reads ECR image URIs from outputs or variables)
        ▼
[Lambda functions: durable-rust-{binary-name}:live]
        │
        │ scripts/test-all.sh
        │ (44 invocations + status polls)
        ▼
[Per-test PASS/FAIL report]
```

### Build Order for Layer Cache Efficiency

```
Phase 1: Docker auth
  aws ecr get-login-password | docker login ECR_URL

Phase 2: Build by style (groups binaries sharing a compile layer)
  closure-style:  cargo build -p closure-style-example  → 11 binaries
  macro-style:    cargo build -p macro-style-example    → 11 binaries
  trait-style:    cargo build -p trait-style-example    → 11 binaries
  builder-style:  cargo build -p builder-style-example  → 11 binaries

Phase 3: Within each style, docker build + push all 11 binaries sequentially
  (docker cache HIT on the cargo layer for binaries 2..11 within same style)

Phase 4: terraform apply
  (updates Lambda function image URIs to latest pushed tags)
```

**Alternative:** Run `cargo build --release --workspace` on host, then use a simpler Dockerfile that only copies pre-compiled binaries (skipping the cargo stage entirely). This is 4–8x faster in CI. Viable if a build host has Rust installed (which the CI already does).

---

## Lambda Naming Convention

### Function Names

```
durable-rust-{style}-{handler}
```

Where:
- `{style}` = `closure`, `macro`, `trait`, `builder`
- `{handler}` = `basic-steps`, `callbacks`, `child-contexts`, `combined-workflow`, `invoke`, `map`, `parallel`, `replay-safe-logging`, `step-retries`, `typed-errors`, `waits`

This matches the existing binary names in Cargo.toml (which already follow `{style}-{handler}`).

**Examples:**
```
durable-rust-closure-basic-steps
durable-rust-macro-callbacks
durable-rust-trait-invoke
durable-rust-builder-combined-workflow
```

### Alias

All functions get a single alias: `live`. Tests always invoke `function-name:live`.

### ECR Tags

Tags mirror the binary name exactly:
```
durable-rust-examples:closure-basic-steps
durable-rust-examples:macro-invoke
```

This makes the image → function mapping trivial to trace.

### Stub Functions

```
order-enrichment-lambda       (no durable-rust- prefix — must match source literal)
fulfillment-lambda            (no durable-rust- prefix — must match source literal)
```

---

## Scaling Considerations

This is a testing infrastructure, not a production service. Scaling concerns apply differently:

| Concern | Current Scope | Mitigation |
|---------|--------------|------------|
| Build time (44 images) | ~15 min cold, ~3 min warm (cache hits) | Build by style; use `--parallel` docker builds if needed |
| Terraform plan time | ~30s for 44 functions | Acceptable; use `-target` to update single function |
| Lambda cold starts | Rust: ~50ms vs Python: ~300ms | Non-issue; tests wait for completion anyway |
| ECR storage cost | 44 images × ~5MB each = ~220MB | Negligible; add lifecycle rule if cost matters |
| Concurrent test runs | 44 simultaneous Lambda invocations | Safe; Lambda scales horizontally by default |

---

## Anti-Patterns

### Anti-Pattern 1: One ECR Repository Per Function

**What people do:** Create 44 `aws_ecr_repository` resources, one per handler.

**Why it's wrong:** 44 repositories with identical configuration, 44 lifecycle rule blocks, 44 IAM resource ARNs. `terraform destroy` becomes a maintenance hazard. ECR soft limits (1,000 repos per account) are not a concern at 44, but the duplication brings no benefit.

**Do this instead:** Single repo with 44 tags. Tags are namespaced and self-documenting.

### Anti-Pattern 2: Unqualified Lambda ARN for Durable Invocation

**What people do:** Invoke `durable-rust-closure-basic-steps` (no qualifier).

**Why it's wrong:** AWS Lambda Durable Execution requires a qualified ARN (version number or alias). Invoking without a qualifier returns an error or silently falls back to `$LATEST`-equivalent behavior that is not guaranteed to be deterministic across replays.

**Do this instead:** Always invoke `durable-rust-closure-basic-steps:live`. The `live` alias points to a specific published version. Terraform creates this alias automatically.

### Anti-Pattern 3: Separate Dockerfile Per Style

**What people do:** Create `closure-style/Dockerfile`, `macro-style/Dockerfile`, `trait-style/Dockerfile`, `builder-style/Dockerfile`.

**Why it's wrong:** The Dockerfiles are identical except for the PACKAGE arg. Four files to maintain, four places to update when the base image changes.

**Do this instead:** Parameterise the existing `examples/Dockerfile` with `ARG PACKAGE` and `ARG BINARY_NAME`. The build script loops over (PACKAGE, BINARY_NAME) pairs.

### Anti-Pattern 4: Polling `get-durable-execution-state` Instead of `list-durable-executions-by-function`

**What people do:** Poll `get-durable-execution-state` (the checkpoint introspection API) to detect completion.

**Why it's wrong:** `get-durable-execution-state` returns low-level operation checkpoints, not high-level execution status. It requires knowing the durable execution ARN up front and is paginated. It is designed for debugging, not status polling.

**Do this instead:** Use `list-durable-executions-by-function --statuses RUNNING` in a loop. When the list returns empty, the execution has completed. Then check `--statuses SUCCEEDED` to confirm success vs failure.

### Anti-Pattern 5: Separate IAM Role Per Function

**What people do:** Create 44 `aws_iam_role` resources.

**Why it's wrong:** All 44 functions need identical permissions (`AWSLambdaBasicDurableExecutionRolePolicy` + CloudWatch Logs). 44 roles bring administrative complexity with no security benefit in a test environment.

**Do this instead:** One shared `durable-rust-lambda-role` attached to all 44 functions. The managed policy `AWSLambdaBasicDurableExecutionRolePolicy` grants exactly `lambda:CheckpointDurableExecutions` and `lambda:GetDurableExecutionState`. Add `lambda:InvokeFunction` for the `invoke` handlers (they call `order-enrichment-lambda`).

---

## Integration Points

### New Components and Their Integration With Existing Code

| New Component | Integrates With | Interface |
|---------------|-----------------|-----------|
| `infra/main.tf` | `examples/` (binary names) | Image tags match `[[bin]] name =` values in Cargo.toml — no code change needed |
| `infra/main.tf` | ECR repository | `image_uri = "${aws_ecr_repository.examples.repository_url}:${each.key}"` |
| `scripts/build-images.sh` | `examples/Dockerfile` | `--build-arg PACKAGE=... --build-arg BINARY_NAME=...` — requires 1 Dockerfile change |
| `scripts/build-images.sh` | Terraform outputs | Script reads ECR repo URL from `terraform output -raw ecr_repo_url` |
| `scripts/test-all.sh` | Lambda aliases | Invokes `durable-rust-{name}:live` via `aws lambda invoke` |
| `scripts/test-all.sh` | AWS CLI | `list-durable-executions-by-function`, `send-durable-execution-callback-success` |
| `infra/stubs/` | `examples/closure-style/src/invoke.rs` et al. | Function name string `"order-enrichment-lambda"` in source must match deployed stub name |

### Modified Existing Components

| Component | Change Required | Reason |
|-----------|----------------|--------|
| `examples/Dockerfile` | Add `ARG BINARY_NAME`; change `COPY` to use `${BINARY_NAME}` | Current Dockerfile copies the package name as the binary, which only works for packages with a single binary matching the package name |

### Unchanged Existing Components

- All 6 library crates in `crates/` — no modification
- All 44 example source files in `examples/*/src/` — no modification
- `Cargo.toml` workspace — no modification (infra is not a Rust workspace member)
- `.github/workflows/ci.yml` — no modification (Terraform + Docker CI is separate)
- `tests/e2e/` and `tests/parity/` — no modification (remain MockDurableContext-based)

---

## IAM Policy Requirements

The `AWSLambdaBasicDurableExecutionRolePolicy` managed policy grants:
- `lambda:CheckpointDurableExecutions` — write checkpoints (called by `RealBackend::checkpoint`)
- `lambda:GetDurableExecutionState` — read checkpoints on replay (called by `RealBackend::get_execution_state`)

Additional policies needed for specific handlers:
- `lambda:InvokeFunction` on `order-enrichment-lambda` and `fulfillment-lambda` ARNs — for `invoke` and `combined_workflow` handlers
- CloudWatch Logs (`logs:CreateLogGroup`, `logs:CreateLogStream`, `logs:PutLogEvents`) — included in `AWSLambdaBasicDurableExecutionRolePolicy`

**Terraform provider version requirement:** `>= 6.25.0` for `durable_config` block support on `aws_lambda_function`.

---

## Sources

- [Configure Lambda durable functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-configuration.html) — DurableConfig structure, ExecutionTimeout, RetentionPeriodInDays
- [Deploy Lambda durable functions with IaC](https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-iac.html) — Terraform HCL for `durable_config`, managed policy ARN, alias requirement
- [Deploy and invoke with AWS CLI](https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-cli.html) — complete CLI workflow, ARN format, publish-version requirement
- [ListDurableExecutionsByFunction API](https://docs.aws.amazon.com/lambda/latest/api/API_ListDurableExecutionsByFunction.html) — status polling, ARN format, RUNNING/SUCCEEDED/FAILED values
- [SendDurableExecutionCallbackSuccess API](https://docs.aws.amazon.com/lambda/latest/api/API_SendDurableExecutionCallbackSuccess.html) — callback ID in URI, result body format
- [terraform-aws-modules/lambda](https://registry.terraform.io/modules/terraform-aws-modules/lambda/aws) — container image + ECR patterns
- Direct codebase inspection: `examples/Dockerfile`, all 4 `Cargo.toml` files, `crates/durable-lambda-core/src/backend.rs`

---

*Architecture research for: AWS Lambda integration testing infrastructure for durable-rust SDK*
*Researched: 2026-03-17*
