# scripts/test-helpers.sh — Shared helper functions for Lambda integration tests. Source this file; do not run directly.
# Do NOT add a shebang — this file is sourced, not executed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TF_DIR="$REPO_ROOT/infra"
PROFILE="adfs"
REGION="us-east-2"

TERMINAL_STATES=("SUCCEEDED" "FAILED" "TIMED_OUT" "STOPPED")
TEST_RUN_ID=$(date +%Y%m%d%H%M%S)

# ---------------------------------------------------------------------------
# check_credentials
# Verifies ADFS credentials are valid before any Lambda invocations.
# Exits 1 if credentials are expired or missing.
# ---------------------------------------------------------------------------
check_credentials() {
  if ! aws sts get-caller-identity \
        --profile "$PROFILE" \
        --region "$REGION" \
        --output text >/dev/null 2>&1; then
    echo "ERROR: ADFS credentials expired or missing. Run: adfs-auth"
    exit 1
  fi
  echo "  [OK] ADFS credentials valid"
}

# ---------------------------------------------------------------------------
# load_tf_outputs
# Reads Terraform outputs into global variables ALIAS_ARNS, STUB_ARNS, SUFFIX.
# Exits 1 if no alias ARNs found (Terraform apply not yet run).
# ---------------------------------------------------------------------------
load_tf_outputs() {
  ALIAS_ARNS=$(terraform -chdir="$TF_DIR" output -json alias_arns)
  STUB_ARNS=$(terraform -chdir="$TF_DIR" output -json stub_alias_arns)
  SUFFIX=$(terraform -chdir="$TF_DIR" output -raw suffix)

  # Validate ALIAS_ARNS is a non-empty JSON object (not "{}")
  local key_count
  key_count=$(echo "$ALIAS_ARNS" | jq 'keys | length')
  if [[ "$key_count" -eq 0 ]]; then
    echo "ERROR: No alias ARNs found — has terraform apply completed?"
    exit 1
  fi
}

# ---------------------------------------------------------------------------
# get_alias_arn(binary_name)
# Looks up the Lambda alias ARN for the given binary name from ALIAS_ARNS.
# Returns 1 if not found.
# ---------------------------------------------------------------------------
get_alias_arn() {
  local name="$1"
  local arn
  arn=$(echo "$ALIAS_ARNS" | jq -r --arg name "$name" '.[$name]')
  if [[ -z "$arn" || "$arn" == "null" ]]; then
    echo "ERROR: Alias ARN not found for binary '${name}'" >&2
    return 1
  fi
  echo "$arn"
}

# ---------------------------------------------------------------------------
# get_stub_arn(stub_name)
# Looks up the Lambda alias ARN for the given stub function from STUB_ARNS.
# Returns 1 if not found.
# ---------------------------------------------------------------------------
get_stub_arn() {
  local name="$1"
  local arn
  arn=$(echo "$STUB_ARNS" | jq -r --arg name "$name" '.[$name]')
  if [[ -z "$arn" || "$arn" == "null" ]]; then
    echo "ERROR: Stub ARN not found for '${name}'" >&2
    return 1
  fi
  echo "$arn"
}

# ---------------------------------------------------------------------------
# make_exec_name(binary_name)
# Generates a unique durable execution name from the binary name and
# TEST_RUN_ID, truncated to 64 characters.
# ---------------------------------------------------------------------------
make_exec_name() {
  echo "${1}-${TEST_RUN_ID}" | cut -c1-64
}

# ---------------------------------------------------------------------------
# invoke_sync(function_arn, payload)
# Invokes a Lambda function synchronously (RequestResponse).
# Outputs pipe-delimited: status_code|fn_error|exec_arn|response_body
# ---------------------------------------------------------------------------
invoke_sync() {
  local function_arn="$1"
  local payload="$2"
  local out_file
  out_file=$(mktemp /tmp/dr-test-XXXXXX.json)

  local meta
  meta=$(aws lambda invoke \
    --profile "$PROFILE" \
    --region "$REGION" \
    --function-name "$function_arn" \
    --invocation-type RequestResponse \
    --cli-binary-format raw-in-base64-out \
    --payload "$payload" \
    "$out_file" 2>&1)

  local status_code fn_error exec_arn response_body
  status_code=$(echo "$meta" | jq -r '.StatusCode // 0')
  fn_error=$(echo "$meta" | jq -r '.FunctionError // empty')
  exec_arn=$(echo "$meta" | jq -r '.DurableExecutionArn // empty')
  response_body=$(cat "$out_file")
  rm -f "$out_file"

  echo "${status_code}|${fn_error}|${exec_arn}|${response_body}"
}

# ---------------------------------------------------------------------------
# invoke_async(function_arn, payload)
# Invokes a Lambda function asynchronously (Event invocation type).
# Uses --durable-execution-name for test isolation.
# Outputs: exec_arn (DurableExecutionArn)
# Returns 1 if no exec_arn is returned.
# ---------------------------------------------------------------------------
invoke_async() {
  local function_arn="$1"
  local payload="$2"
  local out_file
  out_file=$(mktemp /tmp/dr-test-XXXXXX.json)

  local exec_name
  exec_name=$(make_exec_name "$(basename "$function_arn")")

  local meta
  meta=$(aws lambda invoke \
    --profile "$PROFILE" \
    --region "$REGION" \
    --function-name "$function_arn" \
    --invocation-type Event \
    --cli-binary-format raw-in-base64-out \
    --durable-execution-name "$exec_name" \
    --payload "$payload" \
    "$out_file" 2>&1)

  rm -f "$out_file"

  local exec_arn
  exec_arn=$(echo "$meta" | jq -r '.DurableExecutionArn // empty')

  if [[ -z "$exec_arn" ]]; then
    echo "ERROR: No DurableExecutionArn returned from async invoke" >&2
    return 1
  fi

  echo "$exec_arn"
}

# ---------------------------------------------------------------------------
# wait_for_terminal_status(exec_arn, timeout_seconds=120)
# Polls get-durable-execution every 3 seconds until terminal state or timeout.
# Terminal states: SUCCEEDED, FAILED, TIMED_OUT, STOPPED.
# Outputs the final status string.
# Returns 1 on timeout (outputs "TIMEOUT").
# ---------------------------------------------------------------------------
wait_for_terminal_status() {
  local exec_arn="$1"
  local timeout_seconds="${2:-120}"
  local elapsed=0

  while [[ $elapsed -lt $timeout_seconds ]]; do
    local status
    status=$(aws lambda get-durable-execution \
      --profile "$PROFILE" \
      --region "$REGION" \
      --durable-execution-arn "$exec_arn" \
      --query 'Status' \
      --output text 2>/dev/null || echo "")

    for terminal in "${TERMINAL_STATES[@]}"; do
      if [[ "$status" == "$terminal" ]]; then
        echo "$status"
        return 0
      fi
    done

    sleep 3
    elapsed=$((elapsed + 3))
  done

  echo "TIMEOUT"
  return 1
}

# ---------------------------------------------------------------------------
# extract_callback_id(exec_arn, timeout_seconds=60)
# Polls get-durable-execution-history every 3 seconds until a CallbackStarted
# event is found and returns the callback_id.
# Returns 1 on timeout (outputs empty string).
# ---------------------------------------------------------------------------
extract_callback_id() {
  local exec_arn="$1"
  local timeout_seconds="${2:-60}"
  local elapsed=0

  while [[ $elapsed -lt $timeout_seconds ]]; do
    local callback_id
    callback_id=$(aws lambda get-durable-execution-history \
      --profile "$PROFILE" \
      --region "$REGION" \
      --durable-execution-arn "$exec_arn" \
      --query 'Events[?EventType==`CallbackStarted`].CallbackStartedDetails.CallbackId | [0]' \
      --output text 2>/dev/null || echo "")

    if [[ -n "$callback_id" && "$callback_id" != "None" && "$callback_id" != "null" ]]; then
      echo "$callback_id"
      return 0
    fi

    sleep 3
    elapsed=$((elapsed + 3))
  done

  echo ""
  return 1
}

# ---------------------------------------------------------------------------
# send_callback_success(callback_id, result_json='{"approved":true}')
# Sends SendDurableExecutionCallbackSuccess with the given callback_id and
# result JSON. Returns the exit code of the aws command.
# ---------------------------------------------------------------------------
send_callback_success() {
  local callback_id="$1"
  local result_json="${2:-'{\"approved\":true}'}"

  aws lambda send-durable-execution-callback-success \
    --profile "$PROFILE" \
    --region "$REGION" \
    --callback-id "$callback_id" \
    --result "$result_json" \
    --cli-binary-format raw-in-base64-out
}
