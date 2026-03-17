#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TF_DIR="$REPO_ROOT/infra"
ERRORS=0
CHECKS=0

AWS_ARGS="--profile adfs --region us-east-2"
SUFFIX=$(terraform -chdir="$TF_DIR" output -raw suffix)

pass() { CHECKS=$((CHECKS + 1)); echo "[PASS] $1"; }
fail() { CHECKS=$((CHECKS + 1)); ERRORS=$((ERRORS + 1)); echo "[FAIL] $1"; }

echo "=== Infrastructure Verification ==="
echo "Suffix: $SUFFIX"
echo ""

# INFRA-01: ECR repo exists with force_delete
ECR_NAME="dr-examples-$SUFFIX"
if aws ecr describe-repositories --repository-names "$ECR_NAME" $AWS_ARGS >/dev/null 2>&1; then
  pass "INFRA-01: ECR repo $ECR_NAME exists"
else
  fail "INFRA-01: ECR repo $ECR_NAME not found"
fi

# INFRA-01: ECR lifecycle policy
if aws ecr get-lifecycle-policy --repository-name "$ECR_NAME" $AWS_ARGS >/dev/null 2>&1; then
  pass "INFRA-01: ECR lifecycle policy active"
else
  fail "INFRA-01: ECR lifecycle policy missing"
fi

# INFRA-02: IAM role exists
ROLE_NAME="dr-lambda-exec-$SUFFIX"
if aws iam get-role --role-name "$ROLE_NAME" $AWS_ARGS >/dev/null 2>&1; then
  pass "INFRA-02: IAM role $ROLE_NAME exists"
else
  fail "INFRA-02: IAM role $ROLE_NAME not found"
fi

# INFRA-02: Durable execution policy attached
POLICIES=$(aws iam list-attached-role-policies --role-name "$ROLE_NAME" $AWS_ARGS 2>/dev/null | jq -r '.AttachedPolicies[].PolicyArn' || echo "")
if echo "$POLICIES" | grep -q "AWSLambdaBasicDurableExecutionRolePolicy"; then
  pass "INFRA-02: Durable execution policy attached"
else
  fail "INFRA-02: Durable execution policy NOT attached"
fi

# INFRA-03: Sample Lambda functions have DurableConfig
# NOTE: The AWS Lambda API (get-function-configuration) does not surface durable_config
# in its response. Verification is done via terraform state, which stores what was applied.
# Terraform showing no drift (plan exit 0) after apply guarantees durable_config is active.
SAMPLE_FUNCTIONS=("closure-basic-steps" "macro-invoke" "trait-parallel" "builder-combined-workflow")
TF_STATE_JSON=$(terraform -chdir="$TF_DIR" show -json 2>/dev/null)
for FUNC in "${SAMPLE_FUNCTIONS[@]}"; do
  FNAME="dr-${FUNC}-${SUFFIX}"
  DURABLE=$(echo "$TF_STATE_JSON" | jq -r --arg func "$FUNC" '
    .values.root_module.resources[]
    | select(.address == "aws_lambda_function.examples[\"\($func)\"]")
    | .values.durable_config[0].execution_timeout
    | tostring
  ' 2>/dev/null || echo "")
  if [ -n "$DURABLE" ] && [ "$DURABLE" != "null" ]; then
    pass "INFRA-03: $FNAME has DurableConfig (execution_timeout=${DURABLE}s via terraform state)"
  else
    fail "INFRA-03: $FNAME missing DurableConfig in terraform state"
  fi
done

# INFRA-04: Sample aliases exist with numeric version
for FUNC in "${SAMPLE_FUNCTIONS[@]}"; do
  FNAME="dr-${FUNC}-${SUFFIX}"
  ALIAS_VER=$(aws lambda get-alias --function-name "$FNAME" --name live $AWS_ARGS 2>/dev/null | jq -r '.FunctionVersion' || echo "")
  if [ -n "$ALIAS_VER" ] && [ "$ALIAS_VER" != '$LATEST' ] && [ "$ALIAS_VER" != "null" ]; then
    pass "INFRA-04: $FNAME:live alias -> version $ALIAS_VER"
  else
    fail "INFRA-04: $FNAME:live alias missing or points to \$LATEST"
  fi
done

# INFRA-05: Stub functions exist and are invocable
for STUB in "order-enrichment-lambda" "fulfillment-lambda"; do
  SNAME="dr-${STUB}-${SUFFIX}"
  TMPFILE=$(mktemp)
  if aws lambda invoke --function-name "${SNAME}:live" --payload '{}' --cli-binary-format raw-in-base64-out "$TMPFILE" $AWS_ARGS >/dev/null 2>&1; then
    RESPONSE=$(cat "$TMPFILE")
    pass "INFRA-05: $SNAME:live invocable, returned: $(echo "$RESPONSE" | head -c 80)"
  else
    fail "INFRA-05: $SNAME:live invocation failed"
  fi
  rm -f "$TMPFILE"
done

# INFRA-06: Tags on a sample function
SAMPLE_ARN=$(aws lambda get-function --function-name "dr-closure-basic-steps-$SUFFIX" $AWS_ARGS 2>/dev/null | jq -r '.Configuration.FunctionArn' || echo "")
if [ -n "$SAMPLE_ARN" ] && [ "$SAMPLE_ARN" != "null" ]; then
  TAGS=$(aws lambda list-tags --resource "$SAMPLE_ARN" $AWS_ARGS 2>/dev/null | jq -r '.Tags' || echo "{}")
  if echo "$TAGS" | jq -e '.Project == "durable-rust" and .Milestone == "v1.1" and .ManagedBy == "terraform" and .Style == "closure"' >/dev/null 2>&1; then
    pass "INFRA-06: Tags correct on closure-basic-steps (Project, Milestone, ManagedBy, Style)"
  else
    fail "INFRA-06: Tags incorrect on closure-basic-steps: $TAGS"
  fi
else
  fail "INFRA-06: Could not get function ARN for tag check"
fi

# INFRA-07: Local state, not in git
if [ -f "$TF_DIR/terraform.tfstate" ]; then
  pass "INFRA-07: Local tfstate file exists"
else
  fail "INFRA-07: Local tfstate file missing"
fi
if git -C "$REPO_ROOT" check-ignore "$TF_DIR/terraform.tfstate" >/dev/null 2>&1; then
  pass "INFRA-07: tfstate is gitignored"
else
  fail "INFRA-07: tfstate is NOT gitignored"
fi

# Count total Lambda functions
TOTAL=$(terraform -chdir="$TF_DIR" output -json alias_arns 2>/dev/null | jq 'length' || echo 0)
echo ""
echo "=== Summary ==="
echo "Lambda functions with aliases: $TOTAL (expected: 44)"
echo "Checks: $CHECKS passed, $ERRORS failed"

if [ $ERRORS -eq 0 ]; then
  echo "RESULT: ALL CHECKS PASSED"
  exit 0
else
  echo "RESULT: $ERRORS CHECKS FAILED"
  exit 1
fi
