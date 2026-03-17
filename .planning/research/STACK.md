# Stack Research

**Domain:** AWS Lambda Durable Execution integration testing infrastructure (Rust SDK)
**Researched:** 2026-03-17
**Confidence:** HIGH (all versions verified against official releases and docs)

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Terraform | 1.14.7 | Provision ECR, Lambda, IAM, aliases | Latest stable (2026-03-11). Mature HCL syntax, no breaking changes on 1.x. Local state is fine for single-developer validation project. |
| AWS Provider (hashicorp/aws) | >= 6.25.0 (current: 6.36.0) | `aws_lambda_function` with `durable_config` block, ECR, IAM | 6.25.0 is the **minimum** that introduced `durable_config` block for Lambda durable functions. Pin to `~> 6.25` to avoid 7.x surprises. |
| AWS CLI v2 | 2.27.x (latest at research time) | ECR login, Lambda invoke for testing, callback signals | v2 required — v1 is Python-based and does not support `--cli-binary-format raw-in-base64-out` which is needed for base64 payloads. Official installer, not pip. |
| Docker / Docker Buildx | Docker CE + Buildx plugin (latest stable) | Build `provided.al2023` container images for Lambda | Multi-stage Dockerfile already in `examples/Dockerfile`. Buildx needed if building non-native architecture. |
| jq | 1.7.1 (Ubuntu 24.04 apt) | Parse Lambda invoke JSON responses in test harness | Standard in test scripts for extracting `.StatusCode`, decoding base64 log output. |
| Bash | 5.x (system) | Test harness script (`scripts/test-all.sh`) | All Lambda invocations are AWS CLI calls. Bash + jq is the minimal dependency approach — no Python, no Node, no test framework. Simpler than any alternative for this use case. |

### Supporting Libraries / Tools

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cargo-lambda | 1.9.1 | Cross-compile Rust binaries for Lambda target | Use when building **zip-based** Lambda deployments or when the Docker build is too slow for iteration. This project uses container images, so cargo-lambda is optional but useful for local iteration. |
| tflint | latest (install script) | Lint Terraform HCL before apply | Use in CI or pre-apply to catch deprecated arguments, missing tags, invalid `durable_config` parameters. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `aws ecr get-login-password` | Authenticate Docker to ECR | Standard v2 CLI command. Token valid 12 hours. Pipe to `docker login --password-stdin`. |
| `docker buildx` | Multi-platform image builds | Required only if building for `arm64` on an `x86_64` host. The existing Dockerfile targets `public.ecr.aws/lambda/provided:al2023` which is a multi-arch base image. |

## Architecture Choice: x86_64 vs arm64

**Recommendation: x86_64 for this milestone.**

- x86_64 is the AWS Lambda default when `architectures` is not specified in Terraform `aws_lambda_function`
- The existing `examples/Dockerfile` does not specify `--platform`, so it builds natively for the host architecture (almost certainly x86_64 on the development machine)
- x86_64 avoids the QEMU-emulation overhead of cross-compiling on an x86_64 host
- arm64 (Graviton2) is 34% cheaper per invocation and has better cold-start performance — this is a **v1.2 optimization**, not a v1.1 testing requirement
- The `public.ecr.aws/lambda/provided:al2023` base image supports both; to switch later, add `--platform linux/arm64` to docker build and `architectures = ["arm64"]` to `aws_lambda_function`

## Installation

```bash
# Terraform 1.14.7 — official HashiCorp apt repo
wget -O - https://apt.releases.hashicorp.com/gpg | sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/hashicorp.list
sudo apt-get update && sudo apt-get install terraform

# AWS CLI v2 — official installer (NOT pip, NOT apt — those give v1)
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install
aws --version  # should report aws-cli/2.x

# Docker CE + Buildx plugin — official Docker apt repo
sudo apt-get install ca-certificates curl gnupg
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list
sudo apt-get update && sudo apt-get install docker-ce docker-ce-cli containerd.io docker-buildx-plugin
sudo usermod -aG docker $USER  # re-login required

# jq
sudo apt-get install jq

# cargo-lambda (optional — for local binary builds without Docker)
pip3 install cargo-lambda
# OR via Homebrew on Linux:
brew install cargo-lambda/cargo-lambda/cargo-lambda
```

## Version Compatibility

| Package | Compatible With | Notes |
|---------|----------------|-------|
| AWS Provider >= 6.25.0 | Terraform >= 1.0 | `durable_config` block requires provider 6.25.0 minimum — earlier providers silently ignore or error on it |
| AWS Provider 6.x | Terraform 1.14.x | Provider 6.x targets Terraform 1.x; provider 7.x will require Terraform 2.x (not yet released) |
| `aws_lambda_function` `durable_config` | AWS provider >= 6.25 | `execution_timeout` (seconds, max 31622400) and `retention_period` (days) are the two block arguments |
| AWS CLI v2.27+ | Lambda durable execution APIs | `lambda:SendDurableExecutionCallbackSuccess`, `lambda:SendDurableExecutionCallbackFailure` available in v2 |
| Docker buildx | docker-ce >= 20.10 | Buildx is now a first-class plugin installed with docker-buildx-plugin package — no separate install |
| `public.ecr.aws/lambda/provided:al2023` | x86_64 and arm64 | Base image is multi-arch; specify `--platform linux/amd64` to pin explicitly and avoid ambiguity |
| Rust stable 1.82.0+ | lambda_runtime 1.1.1, tokio 1.50.0 | Already validated — do not change toolchain for this milestone |

## Terraform State

Use **local state** for this milestone. This is a single-developer validation project against one AWS account (`adfs` profile, `us-east-2`). Add `terraform.tfstate` and `terraform.tfstate.backup` to `.gitignore`. No DynamoDB lock table needed. Upgrade to S3 backend if the project becomes multi-developer.

## Lambda Function Configuration Notes

Every Lambda function needs:
1. `publish = true` — creates a numbered version, required for durable execution invocation
2. `aws_lambda_alias` pointing to the latest version — durable functions require a qualified ARN (version or alias)
3. IAM role with `AWSLambdaBasicDurableExecutionRolePolicy` attached — includes `lambda:CheckpointDurableExecution` and `lambda:GetDurableExecutionState`
4. `durable_config` block with `execution_timeout` and `retention_period`
5. `package_type = "Image"` and `image_uri` pointing to ECR

The durable_config cannot be added to a function created without it. Always include it from the start.

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Terraform 1.14 | OpenTofu 1.9 | If HashiCorp BSL license is a blocker for your organization; OpenTofu is the open-source fork, API-compatible |
| Terraform + Bash test harness | AWS SAM | SAM provides local Lambda emulation and has native durable function support, but adds a second IaC tool. Excluded per project constraints (no SAM). |
| Terraform + Bash test harness | AWS CDK | TypeScript/Python-based, significantly more complex for a validation project. Excluded per project constraints (no CDK). |
| Terraform + Bash test harness | Pulumi | Excluded per project constraints (no Pulumi). |
| Bash test harness | pytest + boto3 | Python-based, would be cleaner for assertion-heavy testing. Viable for v1.2 if test complexity increases. For v1.1, Bash + jq keeps zero Python dependencies. |
| Docker container image Lambda | cargo-lambda ZIP deploy | cargo-lambda zip deploy is faster to iterate but requires managing binary format differences. Container image matches the existing Dockerfile and is the standard approach for Rust Lambda. |
| x86_64 architecture | arm64 / Graviton2 | Use arm64 in v1.2+ for 34% cost reduction. Requires `--platform linux/arm64` docker build and cross-compilation on x86_64 dev machines. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| AWS SAM | Second IaC tool, project constraint | Terraform |
| AWS CDK | Second IaC tool, project constraint | Terraform |
| Pulumi | Second IaC tool, project constraint | Terraform |
| AWS CLI v1 (`pip install awscli`) | Does not support `aws ecr get-login-password`; `--cli-binary-format` flag missing; will be deprecated | AWS CLI v2 official installer |
| `apt install awscli` on Ubuntu 22.04 | Installs v1.x, not v2 | Official installer from awscli.amazonaws.com |
| `terraform-provider-aws < 6.25` | Missing `durable_config` block — will not configure durable execution on Lambda | hashicorp/aws >= 6.25.0 |
| Terraform `default` workspace for multiple envs | State isolation issues | Single workspace is fine; add `-var-file` for any env variation |
| Global `aws_iam_policy` wildcards (`Resource: "*"`) | Lambda durable execution security guidance requires scoping to specific function ARNs | Scope `lambda:CheckpointDurableExecution` and `lambda:GetDurableExecutionState` to the function ARN with `:*` suffix |

## Sources

- [HashiCorp Releases — Terraform](https://releases.hashicorp.com/terraform/) — Terraform 1.14.7 confirmed current stable
- [GitHub — terraform-provider-aws releases](https://github.com/hashicorp/terraform-provider-aws/releases) — AWS provider 6.36.0 current; 6.25.0 minimum for durable_config
- [AWS Docs — Deploy Lambda durable functions with IaC](https://docs.aws.amazon.com/lambda/latest/dg/durable-getting-started-iac.html) — durable_config block syntax, provider 6.25 requirement — HIGH confidence
- [AWS Docs — Security and permissions for Lambda durable functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-security.html) — IAM actions, AWSLambdaBasicDurableExecutionRolePolicy — HIGH confidence
- [AWS Docs — Installing AWS CLI v2](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html) — official installer, not pip — HIGH confidence
- [GitHub — cargo-lambda releases](https://github.com/cargo-lambda/cargo-lambda/releases) — v1.9.1 current stable (2026-02-26) — HIGH confidence
- [AWS Docs — Lambda architecture selection](https://docs.aws.amazon.com/lambda/latest/dg/foundation-arch.html) — x86_64 is default — HIGH confidence
- [AWS Docs — ECR registry authentication](https://docs.aws.amazon.com/AmazonECR/latest/userguide/registry_auth.html) — `aws ecr get-login-password` pattern — HIGH confidence

---
*Stack research for: AWS Lambda Durable Execution integration testing infrastructure*
*Researched: 2026-03-17*
