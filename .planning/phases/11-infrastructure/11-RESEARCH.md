# Phase 11: Infrastructure - Research

**Researched:** 2026-03-17
**Domain:** Terraform IaC for AWS Lambda Durable Execution (ECR, IAM, 44 Lambda functions, aliases, Python stubs)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Lambda naming: `dr-{binary-name}-{suffix}` pattern where suffix is a Terraform `random_id` 4-char hex (e.g., `dr-closure-basic-steps-a3f2`)
- Same pattern for callee stubs: `dr-order-enrichment-lambda-{suffix}`, `dr-fulfillment-lambda-{suffix}`
- Single ECR repo: `dr-examples-{suffix}`, 44 image tags named after binaries, lifecycle policy keep 2 / 2-day expiry for untagged
- Flat Terraform layout: `infra/main.tf`, `infra/iam.tf`, `infra/ecr.tf`, `infra/lambda.tf`, `infra/stubs.tf`, `infra/variables.tf`, `infra/outputs.tf`
- Lambda function map defined as `locals` block in `lambda.tf` — all 44 entries as HCL map
- Local Terraform state (no remote backend)
- Tagging: PascalCase keys via `default_tags` in provider block: `Project = "durable-rust"`, `Milestone = "v1.1"`, `ManagedBy = "terraform"`; per-Lambda tag: `Style = "{closure|macro|trait|builder}"`
- Provider: `region = "us-east-2"`, `profile = "adfs"`, AWS provider `>= 6.25.0`
- `terraform apply -parallelism=5` documented as required
- Invoke target resolution: test harness passes actual suffixed function name via event payload; handler reads `target_function` from event JSON (example source code stays unchanged)
- Callee stubs: Python runtime, not durable execution
- `force_delete = true` on ECR repo for clean destroy

### Claude's Discretion
- Exact `durable_config` values (execution_timeout, retention_period_in_days)
- Lambda memory size and timeout for test functions
- Python stub handler code for order-enrichment-lambda and fulfillment-lambda
- Whether to split stubs into separate file or keep in lambda.tf (decision: stubs.tf per locked layout)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INFRA-01 | Terraform creates ECR repository for all examples | Single `aws_ecr_repository` with `dr-examples-{suffix}` name, `force_delete = true`, lifecycle policy |
| INFRA-02 | Terraform creates IAM execution role with `AWSLambdaBasicDurableExecutionRolePolicy` | `aws_iam_role` + `aws_iam_role_policy_attachment` with correct managed policy ARN |
| INFRA-03 | Terraform creates all 44 Lambda functions with `durable_config` block and `durable_execution_timeout` | `for_each` over locals map in `lambda.tf`, `durable_config` block with `execution_timeout` + `retention_period` fields, `publish = true` required |
| INFRA-04 | Terraform creates Lambda aliases for qualified ARN invocation | `aws_lambda_alias` `for_each` over same map, `function_version = aws_lambda_function.examples[each.key].version` |
| INFRA-05 | Terraform creates 2 callee stub Lambda functions | `stubs.tf` with `aws_lambda_function` for Python runtime functions; name pattern `dr-order-enrichment-lambda-{suffix}`, `dr-fulfillment-lambda-{suffix}` |
| INFRA-06 | All resources have consistent PascalCase tags | `default_tags` in provider block for shared tags; `Style` tag added per Lambda resource |
| INFRA-07 | Terraform uses local state file | No `backend` block in `main.tf`; `.gitignore` updated with `*.tfstate`, `*.tfstate.backup`, `.terraform/` |
| INFRA-08 | `terraform destroy` cleanly removes all resources | `force_delete = true` on ECR repo; lifecycle policy prevents image accumulation |
</phase_requirements>

---

## Summary

This phase creates all AWS infrastructure for integration testing the durable-rust SDK. The work is pure Terraform HCL — no Rust code is written. The primary output is an `infra/` directory in the workspace root containing 7 flat `.tf` files that provision: one ECR repository with 44 image tag slots, one IAM execution role, 44 Lambda functions with `durable_config` blocks, 44 `live` aliases, 2 Python stub callees, and consistent PascalCase tags across all resources.

Three hard constraints make sequencing and correctness critical. First, `durable_config` is creation-only: once a Lambda function exists without it, Terraform must destroy and recreate the function to add it — there is no in-place update. Define it in every `aws_lambda_function` resource before the first `terraform apply`. Second, all durable function invocations require a qualified ARN (alias or version number); an unqualified function name is rejected by the API. Every function needs a `live` alias and `publish = true` on the resource so `.version` returns a real version number. Third, Terraform's default parallelism of 10 concurrent operations triggers `ResourceConflictException` at 44-function scale; always run `terraform apply -parallelism=5`.

The naming scheme uses a `random_id` Terraform resource (4-char hex) as a suffix on all resource names, making multiple-checkout parallel testing from the same AWS account safe. The ECR repo and all Lambda functions share the same suffix, ensuring consistent naming across the workspace.

**Primary recommendation:** Build `infra/` directory bottom-up: `main.tf` (provider + random_id) → `iam.tf` → `ecr.tf` → `lambda.tf` (locals map + for_each) → `stubs.tf` → `variables.tf` → `outputs.tf`. Do NOT run `terraform apply` until images are pushed to ECR (Lambda `image_uri` is a required field). First deploy: `terraform apply -target=aws_ecr_repository.examples -parallelism=5` to create ECR, then push images (Phase 12), then full `terraform apply -parallelism=5`.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Terraform | 1.14.7 (current stable) | Declare all AWS resources | Official HashiCorp IaC tool; local state is correct for single-developer validation |
| hashicorp/aws provider | `~> 6.25` (minimum 6.25.0, current 6.36.0) | `aws_lambda_function` with `durable_config` block, ECR, IAM | 6.25.0 is the minimum version with `durable_config` block support — earlier versions silently ignore it |
| AWS CLI v2 | 2.27+ | Credential validation check in deploy script | Required for ECR login (`get-login-password`) and durable execution APIs |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| hashicorp/random provider | `~> 3.0` | `random_id` for 4-char hex suffix | Ensures unique naming per workspace to prevent multi-user collisions |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Terraform local state | S3 + DynamoDB backend | Remote state is correct for multi-developer; user specified local — acceptable for single-developer milestone |
| `for_each` on locals map | 44 separate resource blocks | Duplication is unmaintainable; `for_each` is the correct pattern |
| Single ECR repo, 44 tags | 44 separate ECR repos | 44 repos multiply IAM policy ARNs, lifecycle rules, outputs with zero benefit |

**Installation:** Terraform is already installed (Phase 10 complete). AWS CLI v2 already installed.

---

## Architecture Patterns

### Recommended Project Structure
```
durable-rust/
├── infra/                       # Terraform — ALL new infra lives here
│   ├── main.tf                  # terraform block, provider, random_id
│   ├── iam.tf                   # execution role + policy attachment
│   ├── ecr.tf                   # ECR repository + lifecycle policy
│   ├── lambda.tf                # locals map (44 entries) + for_each resources + aliases
│   ├── stubs.tf                 # 2 Python callee stub Lambda functions
│   ├── variables.tf             # input variables (image_tag, account_id)
│   └── outputs.tf               # alias ARNs, ECR repo URL, suffix
└── examples/
    └── Dockerfile               # existing — modified: add ARG BINARY_NAME
```

Note: `infra/stubs/` directory holds Python handler code:
```
infra/
└── stubs/
    ├── order_enrichment.py      # returns {"enriched": true, "order_id": "..."}
    └── fulfillment.py           # returns {"fulfillment_id": "ff-001", "status": "started"}
```

### Pattern 1: Provider and Random Suffix (main.tf)

**What:** Terraform block pins provider versions; `random_id` generates stable 4-char hex suffix per workspace.

**When to use:** Always — foundational setup.

```hcl
# Source: https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-iac.html
terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.25"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.0"
    }
  }
}

provider "aws" {
  region  = "us-east-2"
  profile = "adfs"

  default_tags {
    tags = {
      Project   = "durable-rust"
      Milestone = "v1.1"
      ManagedBy = "terraform"
    }
  }
}

resource "random_id" "suffix" {
  byte_length = 2  # 2 bytes = 4 hex chars
}

locals {
  suffix = random_id.suffix.hex  # e.g. "a3f2"
}
```

### Pattern 2: IAM Role with Managed Policy (iam.tf)

**What:** Single IAM execution role for all 44 Lambda functions + 2 stubs. Attaches the managed policy ARN. Adds `lambda:InvokeFunction` for invoke/combined_workflow handlers.

**When to use:** One role is correct for a test environment; avoid per-function roles.

```hcl
# Source: https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-iac.html
resource "aws_iam_role" "lambda_exec" {
  name = "dr-lambda-exec-${local.suffix}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "lambda.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "durable_exec" {
  role       = aws_iam_role.lambda_exec.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicDurableExecutionRolePolicy"
}

# Required for invoke.rs and combined_workflow.rs handlers
resource "aws_iam_role_policy" "invoke_permission" {
  name = "dr-invoke-permission-${local.suffix}"
  role = aws_iam_role.lambda_exec.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect   = "Allow"
      Action   = ["lambda:InvokeFunction"]
      Resource = [
        "arn:aws:lambda:us-east-2:${data.aws_caller_identity.current.account_id}:function:dr-order-enrichment-lambda-${local.suffix}",
        "arn:aws:lambda:us-east-2:${data.aws_caller_identity.current.account_id}:function:dr-fulfillment-lambda-${local.suffix}",
      ]
    }]
  })
}

data "aws_caller_identity" "current" {}
```

### Pattern 3: ECR Repository with Lifecycle Policy (ecr.tf)

**What:** Single ECR repo for all 44 image tags. `force_delete = true` allows `terraform destroy` even when images are present. Lifecycle policy keeps 2 most recent images per tag and expires untagged images after 2 days.

**When to use:** Always — ECR repo must exist before images can be pushed.

```hcl
resource "aws_ecr_repository" "examples" {
  name                 = "dr-examples-${local.suffix}"
  image_tag_mutability = "MUTABLE"
  force_delete         = true

  image_scanning_configuration {
    scan_on_push = false
  }
}

resource "aws_ecr_lifecycle_policy" "examples" {
  repository = aws_ecr_repository.examples.name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last 2 images per tag"
        selection = {
          tagStatus     = "tagged"
          tagPatternList = ["*"]
          countType     = "imageCountMoreThan"
          countNumber   = 2
        }
        action = { type = "expire" }
      },
      {
        rulePriority = 2
        description  = "Expire untagged images after 2 days"
        selection = {
          tagStatus   = "untagged"
          countType   = "sinceImagePushed"
          countUnit   = "days"
          countNumber = 2
        }
        action = { type = "expire" }
      }
    ]
  })
}
```

### Pattern 4: Lambda for_each with durable_config (lambda.tf)

**What:** Single `locals` block defines all 44 handlers as a map. `aws_lambda_function` and `aws_lambda_alias` both use `for_each` over the same map. `publish = true` is REQUIRED — without it, `.version` returns `"$LATEST"` and the alias cannot point to an immutable version, breaking durable replay determinism.

**When to use:** This is the only maintainable approach for 44 functions.

```hcl
# Source: Terraform for_each pattern + AWS docs durable_config
locals {
  handlers = {
    # Closure style
    "closure-basic-steps"         = { style = "closure", package = "closure-style-example" }
    "closure-step-retries"        = { style = "closure", package = "closure-style-example" }
    "closure-typed-errors"        = { style = "closure", package = "closure-style-example" }
    "closure-waits"               = { style = "closure", package = "closure-style-example" }
    "closure-callbacks"           = { style = "closure", package = "closure-style-example" }
    "closure-invoke"              = { style = "closure", package = "closure-style-example" }
    "closure-parallel"            = { style = "closure", package = "closure-style-example" }
    "closure-map"                 = { style = "closure", package = "closure-style-example" }
    "closure-child-contexts"      = { style = "closure", package = "closure-style-example" }
    "closure-replay-safe-logging" = { style = "closure", package = "closure-style-example" }
    "closure-combined-workflow"   = { style = "closure", package = "closure-style-example" }
    # Macro style
    "macro-basic-steps"           = { style = "macro", package = "macro-style-example" }
    "macro-step-retries"          = { style = "macro", package = "macro-style-example" }
    "macro-typed-errors"          = { style = "macro", package = "macro-style-example" }
    "macro-waits"                 = { style = "macro", package = "macro-style-example" }
    "macro-callbacks"             = { style = "macro", package = "macro-style-example" }
    "macro-invoke"                = { style = "macro", package = "macro-style-example" }
    "macro-parallel"              = { style = "macro", package = "macro-style-example" }
    "macro-map"                   = { style = "macro", package = "macro-style-example" }
    "macro-child-contexts"        = { style = "macro", package = "macro-style-example" }
    "macro-replay-safe-logging"   = { style = "macro", package = "macro-style-example" }
    "macro-combined-workflow"     = { style = "macro", package = "macro-style-example" }
    # Trait style
    "trait-basic-steps"           = { style = "trait", package = "trait-style-example" }
    "trait-step-retries"          = { style = "trait", package = "trait-style-example" }
    "trait-typed-errors"          = { style = "trait", package = "trait-style-example" }
    "trait-waits"                 = { style = "trait", package = "trait-style-example" }
    "trait-callbacks"             = { style = "trait", package = "trait-style-example" }
    "trait-invoke"                = { style = "trait", package = "trait-style-example" }
    "trait-parallel"              = { style = "trait", package = "trait-style-example" }
    "trait-map"                   = { style = "trait", package = "trait-style-example" }
    "trait-child-contexts"        = { style = "trait", package = "trait-style-example" }
    "trait-replay-safe-logging"   = { style = "trait", package = "trait-style-example" }
    "trait-combined-workflow"     = { style = "trait", package = "trait-style-example" }
    # Builder style
    "builder-basic-steps"         = { style = "builder", package = "builder-style-example" }
    "builder-step-retries"        = { style = "builder", package = "builder-style-example" }
    "builder-typed-errors"        = { style = "builder", package = "builder-style-example" }
    "builder-waits"               = { style = "builder", package = "builder-style-example" }
    "builder-callbacks"           = { style = "builder", package = "builder-style-example" }
    "builder-invoke"              = { style = "builder", package = "builder-style-example" }
    "builder-parallel"            = { style = "builder", package = "builder-style-example" }
    "builder-map"                 = { style = "builder", package = "builder-style-example" }
    "builder-child-contexts"      = { style = "builder", package = "builder-style-example" }
    "builder-replay-safe-logging" = { style = "builder", package = "builder-style-example" }
    "builder-combined-workflow"   = { style = "builder", package = "builder-style-example" }
  }
}

resource "aws_lambda_function" "examples" {
  for_each = local.handlers

  function_name = "dr-${each.key}-${local.suffix}"
  role          = aws_iam_role.lambda_exec.arn
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.examples.repository_url}:${each.key}"
  publish       = true  # REQUIRED: makes .version return a real version number for alias

  timeout     = 900   # Lambda invocation timeout; durable_config.execution_timeout governs durable lifecycle
  memory_size = 256

  durable_config {
    execution_timeout = 3600   # 1 hour max per durable execution
    retention_period  = 7      # days to retain checkpoint state
  }

  tags = {
    Style = each.value.style
  }
}

resource "aws_lambda_alias" "live" {
  for_each = local.handlers

  name             = "live"
  function_name    = aws_lambda_function.examples[each.key].function_name
  function_version = aws_lambda_function.examples[each.key].version
}
```

### Pattern 5: Python Callee Stubs (stubs.tf)

**What:** Two minimal Python Lambda functions that serve as invocation targets for `ctx.invoke()` in invoke.rs and combined_workflow.rs examples. Not durable — they just return JSON. Named with the same suffix pattern but note: the handlers hardcode `"order-enrichment-lambda"` and `"fulfillment-lambda"` as callee names. The test harness resolves the actual suffixed name via event payload (`target_function` field).

**When to use:** Required for INFRA-05; invoke and combined_workflow tests will fail without these stubs.

```hcl
# Python stub inline code — avoids managing separate zip files
data "archive_file" "order_enrichment" {
  type        = "zip"
  output_path = "${path.module}/stubs/order_enrichment.zip"
  source {
    content  = file("${path.module}/stubs/order_enrichment.py")
    filename = "lambda_function.py"
  }
}

resource "aws_lambda_function" "order_enrichment" {
  function_name    = "dr-order-enrichment-lambda-${local.suffix}"
  role             = aws_iam_role.lambda_exec.arn
  handler          = "lambda_function.lambda_handler"
  runtime          = "python3.13"
  filename         = data.archive_file.order_enrichment.output_path
  source_code_hash = data.archive_file.order_enrichment.output_base64sha256
  publish          = true
  timeout          = 30
  memory_size      = 128
}

resource "aws_lambda_alias" "order_enrichment_live" {
  name             = "live"
  function_name    = aws_lambda_function.order_enrichment.function_name
  function_version = aws_lambda_function.order_enrichment.version
}

# Repeat pattern for fulfillment stub
```

**Python stub handler code (infra/stubs/order_enrichment.py):**
```python
import json

def lambda_handler(event, context):
    order_id = event.get("order_id", "unknown")
    return {
        "enriched": True,
        "order_id": order_id,
        "details": {"priority": "standard", "region": "us-east-2"}
    }
```

**Python stub handler code (infra/stubs/fulfillment.py):**
```python
import json

def lambda_handler(event, context):
    return {
        "fulfillment_id": "ff-001",
        "status": "started",
        "estimated_delivery": "2 business days"
    }
```

### Pattern 6: Variables and Outputs

**variables.tf:** Minimal — most values are hardcoded or derived from data sources.
```hcl
# image_tag defaults to "latest" but can be overridden during builds
variable "image_tag" {
  description = "Docker image tag to deploy (e.g., 'git-abc123')"
  type        = string
  default     = "latest"
}
```

**outputs.tf:** Export alias ARNs (used by test harness) and ECR URL (used by build script).
```hcl
output "ecr_repo_url" {
  value       = aws_ecr_repository.examples.repository_url
  description = "ECR repository URL for pushing images"
}

output "suffix" {
  value       = local.suffix
  description = "4-char hex suffix used in all resource names"
}

output "alias_arns" {
  value       = { for k, v in aws_lambda_alias.live : k => v.arn }
  description = "Map of binary name to live alias ARN for test harness"
}

output "stub_alias_arns" {
  value = {
    "order-enrichment-lambda" = aws_lambda_alias.order_enrichment_live.arn
    "fulfillment-lambda"      = aws_lambda_alias.fulfillment_live.arn
  }
  description = "Alias ARNs for callee stub functions"
}
```

### Anti-Patterns to Avoid

- **No `publish = true` on aws_lambda_function:** Without it, `.version` returns `"$LATEST"` and the alias points to `$LATEST`, breaking durable replay determinism when code changes.
- **Unqualified Lambda ARN in invocations:** Always use `:live` alias ARN from `terraform output`. Never invoke by bare function name.
- **`terraform apply` at default parallelism:** Default `-parallelism=10` triggers `ResourceConflictException` at 44-function scale. Always use `-parallelism=5`.
- **Applying before images exist in ECR:** Lambda `image_uri` is required at creation time. Use `-target=aws_ecr_repository.examples` for first apply, push images (Phase 12), then full apply.
- **Missing `durable_config` on first apply:** Cannot be added later without destroying and recreating all 44 functions.
- **Committing terraform.tfstate to git:** Contains sensitive ARNs. Add to `.gitignore` before first apply.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Unique naming across dev checkouts | Custom naming scripts | `random_id` Terraform resource | Stable per workspace, multi-user safe, survives `terraform plan` idempotently |
| 44 separate resource blocks | Manual HCL repetition | `for_each` over locals map | Adding a handler is one line; `terraform destroy` cleans all 44 atomically |
| IAM trust policy JSON | Hand-written JSON string | `jsonencode()` Terraform function | Type-safe, validated at plan time |
| Python zip packaging | Custom zip script | `archive_file` data source | Terraform-native, hash-based drift detection |

**Key insight:** Terraform's `for_each` on a single `locals` map is the only scalable approach for 44 nearly-identical resources. The map key becomes the function name discriminant; the map value carries per-function metadata (style, package). Never break this into 44 separate resource blocks.

---

## Common Pitfalls

### Pitfall 1: durable_config Is Creation-Only

**What goes wrong:** Adding `durable_config` to an already-deployed Lambda function causes Terraform to destroy and recreate it. Any in-flight durable executions are orphaned. ARNs rotate (though aliases mitigate this).

**Why it happens:** `DurableConfig` is a creation-only attribute in the AWS Lambda API. AWS does not support enabling durable execution on an existing function in-place.

**How to avoid:** Define `durable_config` in every `aws_lambda_function` resource before the first `terraform apply`. Run `terraform plan` and verify no `forces replacement` annotations before applying anything to an existing stack.

**Warning signs:** `terraform plan` output shows `# aws_lambda_function.examples["X"] must be replaced` when you only added `durable_config`.

### Pitfall 2: Missing publish = true Breaks Aliases

**What goes wrong:** `aws_lambda_alias.live.function_version` is set to `"$LATEST"`. Durable invocations reject `$LATEST` qualifier. All 44 tests fail with `InvalidParameterValueException`.

**Why it happens:** Without `publish = true` on `aws_lambda_function`, the `.version` attribute returns `"$LATEST"` not a version number like `"1"`. The alias then points to `$LATEST`, not an immutable version.

**How to avoid:** Always set `publish = true` on every `aws_lambda_function` resource. The official AWS docs example omits this but relies on provider behavior that auto-publishes — do not rely on that; be explicit.

**Warning signs:** `terraform output alias_arns` shows ARNs without a trailing `:live`; or invocations return `InvalidParameterValueException: Durable functions require a qualified ARN`.

### Pitfall 3: Terraform Apply at Full Parallelism

**What goes wrong:** `terraform apply` (default `-parallelism=10`) triggers concurrent creation of 44 Lambda functions + aliases + IAM attachments. Lambda control plane throttles with `ResourceConflictException` on ~20-30% of resources. Apply partially fails.

**Why it happens:** Known upstream issue in terraform-provider-aws (#5154). Lambda's internal concurrency limit on modification operations per account is hit by default Terraform parallelism.

**How to avoid:** Always use `terraform apply -parallelism=5`. Document this in a `Makefile` or deploy script so it is never accidentally omitted.

**Warning signs:** `ResourceConflictException: An update is in progress for resource` in apply output; some functions created, others failed in the same run; errors resolve on re-run.

### Pitfall 4: Lambda image_uri Required at Creation Time

**What goes wrong:** Running full `terraform apply` before any images are pushed to ECR fails. Lambda requires `image_uri` to resolve to an existing image at creation time.

**Why it happens:** `aws_lambda_function` with `package_type = "Image"` validates that the `image_uri` exists at apply time. An empty or non-existent tag causes the resource creation to fail.

**How to avoid:** Two-phase deploy: (1) `terraform apply -target=aws_ecr_repository.examples -parallelism=5` to create ECR repo; (2) push images (Phase 12 build pipeline); (3) full `terraform apply -parallelism=5`. Document this order clearly.

**Warning signs:** `Error: creating Lambda Function: InvalidParameterValueException: Image does not exist`.

### Pitfall 5: Stub Callee Names Must Not Include Suffix in Source Code

**What goes wrong:** The invoke.rs and combined_workflow.rs example sources hardcode `"order-enrichment-lambda"` and `"fulfillment-lambda"` as callee names. But Terraform deploys them as `dr-order-enrichment-lambda-{suffix}` and `dr-fulfillment-lambda-{suffix}`. The test harness must bridge this gap via event payload (`target_function` field), not by modifying example source.

**Why it happens:** Source code cannot know the runtime suffix at compile time.

**How to avoid:** The test harness passes the actual suffixed function name as `target_function` in the event JSON when invoking invoke/combined_workflow handlers. The stubs do NOT need to be named without the prefix — the handler reads the target from the event payload and invokes via AWS SDK.

**Warning signs:** Invoke tests fail with `Function not found: order-enrichment-lambda` — indicates the handler is reading a hardcoded name instead of the event payload.

### Pitfall 6: terraform.tfstate Committed to Git

**What goes wrong:** State file contains all Lambda ARNs, IAM role ARNs, and potentially sensitive resource IDs. Committed to git, it leaks infrastructure details.

**How to avoid:** Add to `.gitignore` before the first `terraform apply`:
```
infra/terraform.tfstate
infra/terraform.tfstate.backup
infra/.terraform/
infra/.terraform.lock.hcl  # keep this one — it locks provider versions
```
Actually `.terraform.lock.hcl` SHOULD be committed (it ensures reproducible provider versions). Only exclude `.terraform/`, `*.tfstate`, and `*.tfstate.backup`.

---

## Code Examples

### Complete durable_config Block (Verified)

```hcl
# Source: https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-iac.html
# Field names confirmed: execution_timeout and retention_period (NOT retention_period_in_days)
durable_config {
  execution_timeout = 3600   # seconds; max 31,536,000 (1 year)
  retention_period  = 7      # days; range 1-365
}
```

### Managed Policy ARN (Verified)

```hcl
# Source: AWS docs IaC getting started guide — exact ARN including /service-role/ path
policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicDurableExecutionRolePolicy"
```

### ECR Lifecycle Policy JSON (for Terraform jsonencode)

```hcl
# Source: https://docs.aws.amazon.com/AmazonECR/latest/userguide/lifecycle_policy_examples.html
# tagPatternList ["*"] matches all tagged images regardless of tag name
policy = jsonencode({
  rules = [
    {
      rulePriority = 1
      description  = "Keep last 2 tagged images"
      selection = {
        tagStatus      = "tagged"
        tagPatternList = ["*"]
        countType      = "imageCountMoreThan"
        countNumber    = 2
      }
      action = { type = "expire" }
    },
    {
      rulePriority = 2
      description  = "Expire untagged images after 2 days"
      selection = {
        tagStatus   = "untagged"
        countType   = "sinceImagePushed"
        countUnit   = "days"
        countNumber = 2
      }
      action = { type = "expire" }
    }
  ]
})
```

### Verification Commands (Post-Apply)

```bash
# Verify durable_config is active on all functions
aws lambda get-function-configuration \
  --function-name dr-closure-basic-steps-a3f2 \
  --region us-east-2 \
  --profile adfs \
  | jq '.DurableConfig'
# Expected: {"ExecutionTimeout": 3600, "RetentionPeriodInDays": 7}

# Verify all aliases exist and point to real version (not $LATEST)
terraform -chdir=infra output -json alias_arns | jq 'to_entries[] | .value' | head -5
# Expected: "arn:aws:lambda:us-east-2:ACCOUNT:function:dr-closure-basic-steps-a3f2:live"

# Verify IAM policy attached
aws iam list-attached-role-policies \
  --role-name dr-lambda-exec-a3f2 \
  --profile adfs \
  | jq '.AttachedPolicies[].PolicyArn'
# Expected: "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicDurableExecutionRolePolicy"

# Verify ECR lifecycle policy
aws ecr get-lifecycle-policy \
  --repository-name dr-examples-a3f2 \
  --profile adfs \
  --region us-east-2
# Expected: policy JSON with 2 rules
```

### Makefile Targets (deploy script)

```makefile
.PHONY: init plan apply destroy

PARALLELISM := 5
TF_DIR      := infra

init:
	terraform -chdir=$(TF_DIR) init

plan:
	terraform -chdir=$(TF_DIR) plan

# First-time: create ECR before images are pushed
ecr:
	terraform -chdir=$(TF_DIR) apply -target=aws_ecr_repository.examples -parallelism=$(PARALLELISM) -auto-approve

# Full deploy (after images are in ECR)
apply:
	terraform -chdir=$(TF_DIR) apply -parallelism=$(PARALLELISM)

destroy:
	terraform -chdir=$(TF_DIR) destroy -parallelism=$(PARALLELISM)
```

---

## Complete 44 Binary Name Inventory

All binary names confirmed by direct Cargo.toml inspection. These are the exact `[[bin]] name` values from the 4 example crates:

| Binary Name | Style | Cargo Package |
|-------------|-------|---------------|
| closure-basic-steps | closure | closure-style-example |
| closure-step-retries | closure | closure-style-example |
| closure-typed-errors | closure | closure-style-example |
| closure-waits | closure | closure-style-example |
| closure-callbacks | closure | closure-style-example |
| closure-invoke | closure | closure-style-example |
| closure-parallel | closure | closure-style-example |
| closure-map | closure | closure-style-example |
| closure-child-contexts | closure | closure-style-example |
| closure-replay-safe-logging | closure | closure-style-example |
| closure-combined-workflow | closure | closure-style-example |
| macro-basic-steps | macro | macro-style-example |
| macro-step-retries | macro | macro-style-example |
| macro-typed-errors | macro | macro-style-example |
| macro-waits | macro | macro-style-example |
| macro-callbacks | macro | macro-style-example |
| macro-invoke | macro | macro-style-example |
| macro-parallel | macro | macro-style-example |
| macro-map | macro | macro-style-example |
| macro-child-contexts | macro | macro-style-example |
| macro-replay-safe-logging | macro | macro-style-example |
| macro-combined-workflow | macro | macro-style-example |
| trait-basic-steps | trait | trait-style-example |
| trait-step-retries | trait | trait-style-example |
| trait-typed-errors | trait | trait-style-example |
| trait-waits | trait | trait-style-example |
| trait-callbacks | trait | trait-style-example |
| trait-invoke | trait | trait-style-example |
| trait-parallel | trait | trait-style-example |
| trait-map | trait | trait-style-example |
| trait-child-contexts | trait | trait-style-example |
| trait-replay-safe-logging | trait | trait-style-example |
| trait-combined-workflow | trait | trait-style-example |
| builder-basic-steps | builder | builder-style-example |
| builder-step-retries | builder | builder-style-example |
| builder-typed-errors | builder | builder-style-example |
| builder-waits | builder | builder-style-example |
| builder-callbacks | builder | builder-style-example |
| builder-invoke | builder | builder-style-example |
| builder-parallel | builder | builder-style-example |
| builder-map | builder | builder-style-example |
| builder-child-contexts | builder | builder-style-example |
| builder-replay-safe-logging | builder | builder-style-example |
| builder-combined-workflow | builder | builder-style-example |

**Total: 44 binary names confirmed.** These become ECR image tags and Lambda function name components.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `aws_lambda_function` without `durable_config` | `durable_config` block required | AWS provider 6.25.0 (late 2024) | Older provider versions silently ignore `durable_config`; must pin `>= 6.25.0` |
| Separate ECR repo per Lambda | Single ECR repo with per-binary tags | Architecture decision | 44x fewer resources; one lifecycle policy |
| Manual per-function versioning (`aws_lambda_version` resource) | `publish = true` on `aws_lambda_function` | Terraform provider v2.0+ | Simpler; publish = true auto-publishes on each apply |
| `retain_period_in_days` | `retention_period` | Provider 6.25.0 | Field renamed in Terraform HCL; JSON API uses `RetentionPeriodInDays` |

**Field name clarification (CRITICAL):**
- Terraform HCL `durable_config` block: `execution_timeout` + `retention_period`
- AWS JSON API (`create-function --durable-config`): `ExecutionTimeout` + `RetentionPeriodInDays`
- Do NOT confuse them. The Terraform field is `retention_period` (not `retention_period_in_days`).

---

## Open Questions

1. **`publish = true` with durable_config — official docs are silent**
   - What we know: Without `publish = true`, `.version` returns `"$LATEST"`. Durable invocations require qualified ARN. The official AWS IaC docs example does not show `publish = true`.
   - What's unclear: Whether the AWS provider 6.25.0+ with `durable_config` automatically implies versioning behavior, or whether `publish = true` is still required.
   - Recommendation: Include `publish = true` explicitly. It is harmless if redundant and critical if required. Verify by checking `terraform output alias_arns` after first apply — alias ARN should end in `:live`, and `aws lambda get-alias` should return a `FunctionVersion` that is a number like `"1"`, not `"$LATEST"`.

2. **ECR lifecycle policy `tagPatternList = ["*"]` vs no tagPatternList**
   - What we know: ECR lifecycle rules with `tagStatus = "tagged"` require either `tagPrefixList` or `tagPatternList` (the newer syntax). Using `["*"]` as the pattern should match all tagged images.
   - What's unclear: Whether `tagPatternList = ["*"]` is actually valid in AWS ECR lifecycle JSON or whether a different wildcard syntax is needed.
   - Recommendation: Test with `aws ecr put-lifecycle-policy` directly if Terraform apply fails with a lifecycle policy error. Fallback: use `tagStatus = "any"` rule instead.

3. **`data.aws_caller_identity.current` availability**
   - What we know: The `data.aws_caller_identity` data source requires valid AWS credentials at `terraform plan` time.
   - What's unclear: Whether ADFS profile credentials are available during `terraform plan` in CI-like contexts.
   - Recommendation: Use `data.aws_caller_identity.current.account_id` but document that ADFS credentials must be active before running any Terraform command.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Terraform validation — `terraform validate` + `terraform plan` |
| Config file | `infra/main.tf` (terraform block) |
| Quick run command | `terraform -chdir=infra validate` |
| Full suite command | `terraform -chdir=infra plan` + manual AWS verification |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INFRA-01 | ECR repo `dr-examples-{suffix}` created with lifecycle policy | smoke | `aws ecr describe-repositories --region us-east-2 --profile adfs \| jq '.repositories[].repositoryName'` | No — post-apply verification |
| INFRA-02 | IAM role with AWSLambdaBasicDurableExecutionRolePolicy attached | smoke | `aws iam list-attached-role-policies --role-name dr-lambda-exec-{suffix} --profile adfs` | No — post-apply verification |
| INFRA-03 | All 44 Lambda functions have DurableConfig in configuration | smoke | `aws lambda get-function-configuration --function-name dr-closure-basic-steps-{suffix} --profile adfs --region us-east-2 \| jq '.DurableConfig'` | No — post-apply verification |
| INFRA-04 | All 44 aliases exist, FunctionVersion is numeric not $LATEST | smoke | `aws lambda get-alias --function-name dr-closure-basic-steps-{suffix} --name live --profile adfs --region us-east-2 \| jq '.FunctionVersion'` | No — post-apply verification |
| INFRA-05 | 2 stub functions exist and are invocable | smoke | `aws lambda invoke --function-name dr-order-enrichment-lambda-{suffix}:live --payload '{}' /tmp/stub-test.json --profile adfs --region us-east-2` | No — post-apply verification |
| INFRA-06 | All resources have PascalCase tags | smoke | `aws lambda list-tags --resource {function-arn} --profile adfs --region us-east-2` | No — post-apply verification |
| INFRA-07 | Local tfstate present, not in git | unit | `ls infra/terraform.tfstate && git check-ignore infra/terraform.tfstate` | No — Wave 0 |
| INFRA-08 | `terraform destroy` exits 0 | smoke | `terraform -chdir=infra destroy -parallelism=5 -auto-approve` (teardown only) | No — end-of-phase manual |

### Sampling Rate
- **Per task commit:** `terraform -chdir=infra validate`
- **Per wave merge:** `terraform -chdir=infra plan` (dry run)
- **Phase gate:** Full apply + all smoke verification commands green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `infra/.gitignore` or root `.gitignore` update — covers INFRA-07 (tfstate not committed)
- [ ] `infra/terraform.lock.hcl` — committed after `terraform init` to lock provider versions
- [ ] Framework install: Terraform already installed (Phase 10); `terraform -chdir=infra init` generates lock file
- [ ] Verification script: `scripts/verify-infra.sh` — runs all smoke commands above with actual suffix from `terraform output`

---

## Sources

### Primary (HIGH confidence)
- [AWS Docs — Deploy Lambda durable functions with IaC](https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-iac.html) — complete Terraform HCL example, managed policy ARN, provider version requirement, `durable_config` field names (`execution_timeout`, `retention_period`)
- [AWS Docs — Configure Lambda durable functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-configuration.html) — `ExecutionTimeout`, `RetentionPeriodInDays`, `AllowInvokeLatest` API-level field names; max values (31,536,000 sec, 365 days)
- [AWS Docs — Deploy and invoke with AWS CLI](https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-cli.html) — `publish-version` step shown; qualified ARN requirement for durable invocations
- [AWS Docs — Security and permissions for Lambda durable functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-security.html) — `AWSLambdaBasicDurableExecutionRolePolicy` grants `lambda:CheckpointDurableExecution` + `lambda:GetDurableExecutionState`; `lambda:InvokeFunction` must be added separately
- [AWS Docs — Lambda versioning](https://docs.aws.amazon.com/lambda/latest/api/API_PublishVersion.html) — versions are NOT auto-published; `publish = true` in Terraform triggers publish on each apply
- Direct codebase inspection: `examples/Dockerfile`, all 4 example `Cargo.toml` files — 44 binary names confirmed

### Secondary (MEDIUM confidence)
- [AWS Docs — ECR lifecycle policy examples](https://docs.aws.amazon.com/AmazonECR/latest/userguide/lifecycle_policy_examples.html) — `tagPatternList`, `countType: imageCountMoreThan`, `sinceImagePushed` field names
- [GitHub — terraform-provider-aws #5154](https://github.com/hashicorp/terraform-provider-aws/issues/5154) — ResourceConflictException at default parallelism (open known issue)
- [GitHub — terraform-provider-aws #45354](https://github.com/hashicorp/terraform-provider-aws/issues/45354) — durable_config support introduced in provider 6.25.0

### Tertiary (LOW confidence)
- `publish = true` requirement for `.version` returning numeric version — inferred from standard Terraform Lambda versioning behavior; official durable IaC docs do not explicitly address this interaction

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions verified against official docs and GitHub releases
- Architecture: HIGH — patterns derived from official AWS IaC docs + direct codebase inspection
- durable_config field names: HIGH — verified against two official AWS doc pages
- Managed policy ARN: HIGH — exact ARN confirmed in official IaC doc Terraform example
- `publish = true` requirement: MEDIUM — standard Terraform behavior, but official durable docs are silent on this specific interaction
- Pitfalls: HIGH — durable_config creation-only and ResourceConflictException confirmed in official docs and GitHub issues

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (30 days — Terraform AWS provider releases frequently but durable_config block is stable)
