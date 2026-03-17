#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TF_DIR="$REPO_ROOT/infra"
PARALLELISM=5

echo "=== Full Infrastructure Deploy ==="
echo "Using: terraform -chdir=$TF_DIR"

# Verify prerequisites
"$SCRIPT_DIR/verify-prerequisites.sh" || { echo "Prerequisites check failed"; exit 1; }

# Init (idempotent)
terraform -chdir="$TF_DIR" init -input=false

# Full apply with parallelism=5 (REQUIRED to avoid ResourceConflictException)
terraform -chdir="$TF_DIR" apply -parallelism=$PARALLELISM -auto-approve

# Verify no drift
echo ""
echo "=== Verifying no drift (terraform plan) ==="
terraform -chdir="$TF_DIR" plan -detailed-exitcode -parallelism=$PARALLELISM
PLAN_EXIT=$?
if [ $PLAN_EXIT -eq 0 ]; then
  echo "PASS: No changes detected"
elif [ $PLAN_EXIT -eq 2 ]; then
  echo "WARNING: terraform plan detected drift"
  exit 1
fi

echo ""
echo "=== Deploy complete ==="
terraform -chdir="$TF_DIR" output -raw suffix
echo ""
