#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TF_DIR="$REPO_ROOT/infra"
PARALLELISM=5

echo "=== Deploying ECR + IAM (targeted apply) ==="
echo "Using: terraform -chdir=$TF_DIR"

# Verify prerequisites
"$SCRIPT_DIR/verify-prerequisites.sh" || { echo "Prerequisites check failed"; exit 1; }

# Init (idempotent)
terraform -chdir="$TF_DIR" init -input=false

# Targeted apply: ECR repo + lifecycle policy + IAM role + policies + random_id
terraform -chdir="$TF_DIR" apply \
  -target=aws_ecr_repository.examples \
  -target=aws_ecr_lifecycle_policy.examples \
  -target=aws_iam_role.lambda_exec \
  -target=aws_iam_role_policy_attachment.durable_exec \
  -target=aws_iam_role_policy.invoke_permission \
  -parallelism=$PARALLELISM \
  -auto-approve

# Output ECR URL for build pipeline
echo ""
echo "=== ECR Repository URL ==="
terraform -chdir="$TF_DIR" output -raw ecr_repo_url
echo ""
echo ""
echo "=== Resource Suffix ==="
terraform -chdir="$TF_DIR" output -raw suffix
echo ""
