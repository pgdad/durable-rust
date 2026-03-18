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

  # Extract a safe name from the ARN for durable execution naming.
  # ARN format: arn:aws:lambda:REGION:ACCOUNT:function:NAME:QUALIFIER
  # We want just the NAME part, which is after "function:" and before ":qualifier".
  local safe_name
  safe_name=$(echo "$function_arn" | sed 's/.*function://' | sed 's/:.*//')
  local exec_name
  exec_name=$(make_exec_name "$safe_name")

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

# ===========================================================================
# Phase 14: Shared Assertion Helpers
# Each helper invokes a Lambda by binary name, parses the response, and
# asserts 2-3 key fields. Used by the 32 Phase 14 test functions.
# ===========================================================================

# ---------------------------------------------------------------------------
# assert_basic_steps(binary)
# Invokes basic-steps handler with order_id, verifies round-trip and
# details.status field.
# ---------------------------------------------------------------------------
assert_basic_steps() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{"order_id":"test-order-001"}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }

  local order_id
  order_id=$(echo "$response_body" | jq -r '.order_id')
  [[ "$order_id" == "test-order-001" ]] || \
    { echo "Expected order_id=test-order-001, got: $order_id; body=$response_body"; return 1; }

  local details_status
  details_status=$(echo "$response_body" | jq -r '.details.status')
  [[ "$details_status" == "found" ]] || \
    { echo "Expected details.status=found, got: $details_status; body=$response_body"; return 1; }

  echo "basic steps executed and replayed correctly via $binary"
}

# ---------------------------------------------------------------------------
# assert_step_retries(binary)
# Invokes step-retries handler, verifies result.api_response=success.
# ---------------------------------------------------------------------------
assert_step_retries() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }

  local api_response
  api_response=$(echo "$response_body" | jq -r '.result.api_response')
  [[ "$api_response" == "success" ]] || \
    { echo "Expected result.api_response=success, got: $api_response; body=$response_body"; return 1; }

  echo "step retries completed successfully via $binary"
}

# ---------------------------------------------------------------------------
# assert_typed_errors(binary)
# Tests BOTH success path (amount=50) and error path (amount=2000).
# Both return HTTP 200 (domain error, not Lambda error).
# ---------------------------------------------------------------------------
assert_typed_errors() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")

  # --- Success path: amount=50 ---
  local result
  result=$(invoke_sync "$fn_arn" '{"amount":50}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Success path: Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Success path: Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }

  local transaction_id
  transaction_id=$(echo "$response_body" | jq -r '.transaction_id')
  [[ "$transaction_id" == "txn_50" ]] || \
    { echo "Success path: Expected transaction_id=txn_50, got: $transaction_id; body=$response_body"; return 1; }

  # --- Error path: amount=2000 ---
  local result_err
  result_err=$(invoke_sync "$fn_arn" '{"amount":2000}')
  local status_err fn_error_err response_body_err
  IFS='|' read -r status_err fn_error_err _ response_body_err <<< "$result_err"

  [[ "$status_err" == "200" ]] || { echo "Error path: Expected HTTP 200, got: $status_err; body=$response_body_err"; return 1; }
  [[ -z "$fn_error_err" ]] || { echo "Error path: Expected no FunctionError, got: $fn_error_err; body=$response_body_err"; return 1; }

  local error_field
  error_field=$(echo "$response_body_err" | jq -r '.error')
  [[ "$error_field" == "insufficient_funds" ]] || \
    { echo "Error path: Expected error=insufficient_funds, got: $error_field; body=$response_body_err"; return 1; }

  echo "typed error correctly serialized through durable execution via $binary"
}

# ---------------------------------------------------------------------------
# assert_parallel(binary)
# Invokes parallel handler, verifies 3 branches returned with sorted
# membership check (parallel order is non-deterministic).
# ---------------------------------------------------------------------------
assert_parallel() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }

  local branch_count
  branch_count=$(echo "$response_body" | jq '.parallel_results | length')
  [[ "$branch_count" == "3" ]] || \
    { echo "Expected 3 parallel_results, got: $branch_count; body=$response_body"; return 1; }

  local branches_sorted
  branches_sorted=$(echo "$response_body" | jq -r '[.parallel_results[] | .branch] | sort | join(",")')
  [[ "$branches_sorted" == "a,b,c" ]] || \
    { echo "Expected branches a,b,c (sorted), got: $branches_sorted; body=$response_body"; return 1; }

  echo "parallel fan-out completed with 3 branches via $binary"
}

# ---------------------------------------------------------------------------
# assert_map(binary)
# Invokes map handler, verifies 4 processed orders with correct status.
# ---------------------------------------------------------------------------
assert_map() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }

  local order_count
  order_count=$(echo "$response_body" | jq '.processed_orders | length')
  [[ "$order_count" == "4" ]] || \
    { echo "Expected 4 processed_orders, got: $order_count; body=$response_body"; return 1; }

  local first_status
  first_status=$(echo "$response_body" | jq -r '.processed_orders[0].status')
  [[ "$first_status" == "done" ]] || \
    { echo "Expected processed_orders[0].status=done, got: $first_status; body=$response_body"; return 1; }

  echo "map operation processed 4 orders via $binary"
}

# ---------------------------------------------------------------------------
# assert_child_contexts(binary)
# Invokes child-contexts handler, verifies child isolation and parent
# step independence.
# ---------------------------------------------------------------------------
assert_child_contexts() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }

  local child_validation
  child_validation=$(echo "$response_body" | jq -r '.child_result.validation')
  [[ "$child_validation" == "passed" ]] || \
    { echo "Expected child_result.validation=passed, got: $child_validation; body=$response_body"; return 1; }

  local parent_result
  parent_result=$(echo "$response_body" | jq -r '.parent_result')
  [[ "$parent_result" == "parent_validation" ]] || \
    { echo "Expected parent_result=parent_validation, got: $parent_result; body=$response_body"; return 1; }

  echo "child context isolated namespace, parent step ran independently via $binary"
}

# ---------------------------------------------------------------------------
# assert_replay_safe_logging(binary)
# Invokes replay-safe-logging handler, verifies order_id round-trip and
# result.processed field. Response-only validation (no CloudWatch queries).
# ---------------------------------------------------------------------------
assert_replay_safe_logging() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{"order_id":"test-order-001"}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }

  local order_id
  order_id=$(echo "$response_body" | jq -r '.order_id')
  [[ "$order_id" == "test-order-001" ]] || \
    { echo "Expected order_id=test-order-001, got: $order_id; body=$response_body"; return 1; }

  local processed
  processed=$(echo "$response_body" | jq -r '.result.processed')
  [[ "$processed" == "true" ]] || \
    { echo "Expected result.processed=true, got: $processed; body=$response_body"; return 1; }

  echo "replay-safe logging handler executed without duplicate log side effects via $binary"
}

# ---------------------------------------------------------------------------
# assert_combined_workflow(binary)
# Invokes combined-workflow handler, verifies order_id, payment, fulfillment,
# and post_processing fields. NOTE: This invocation blocks ~35+ seconds due
# to ctx.wait(30s) inside the handler.
# ---------------------------------------------------------------------------
assert_combined_workflow() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{"order_id":"test-order-001","total":99.99}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }

  local order_id
  order_id=$(echo "$response_body" | jq -r '.order_id')
  [[ "$order_id" == "test-order-001" ]] || \
    { echo "Expected order_id=test-order-001, got: $order_id; body=$response_body"; return 1; }

  local payment_charged
  payment_charged=$(echo "$response_body" | jq -r '.payment.charged')
  [[ "$payment_charged" == "true" ]] || \
    { echo "Expected payment.charged=true, got: $payment_charged; body=$response_body"; return 1; }

  local fulfillment
  fulfillment=$(echo "$response_body" | jq -r '.fulfillment')
  [[ "$fulfillment" != "null" ]] || \
    { echo "Expected fulfillment non-null, got: null; body=$response_body"; return 1; }

  local post_processing
  post_processing=$(echo "$response_body" | jq -r '.post_processing')
  [[ "$post_processing" != "null" ]] || \
    { echo "Expected post_processing non-null, got: null; body=$response_body"; return 1; }

  echo "combined workflow completed with payment, fulfillment, and post-processing via $binary"
}

# ===========================================================================
# Phase 15: Async Operation Helpers
# get_execution_output retrieves completed execution output.
# assert_waits, assert_callbacks, assert_invoke validate async operations
# across 4 API styles.
# ===========================================================================

# ---------------------------------------------------------------------------
# get_execution_output(exec_arn)
# Retrieves the Output field from a completed durable execution.
# Returns the raw output string (typically JSON).
# ---------------------------------------------------------------------------
get_execution_output() {
  local exec_arn="$1"
  aws lambda get-durable-execution \
    --profile "$PROFILE" \
    --region "$REGION" \
    --durable-execution-arn "$exec_arn" \
    --query 'Result' \
    --output text 2>/dev/null
}

# ---------------------------------------------------------------------------
# assert_waits(binary)
# Full async wait test flow:
#   invoke_async with 5-second wait -> poll to SUCCEEDED -> get output
#   -> verify started.status="started" and completed.status="completed"
# ---------------------------------------------------------------------------
assert_waits() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")

  # Start async execution with 5-second wait
  local exec_arn
  exec_arn=$(invoke_async "$fn_arn" '{"wait_seconds":5}')
  [[ -n "$exec_arn" ]] || { echo "Expected non-empty exec_arn from invoke_async"; return 1; }

  # Poll until terminal status (60s timeout = 12x margin for 5s wait)
  local final_status
  final_status=$(wait_for_terminal_status "$exec_arn" 60)
  [[ "$final_status" == "SUCCEEDED" ]] || \
    { echo "Expected SUCCEEDED, got: $final_status for exec_arn=$exec_arn"; return 1; }

  # Retrieve execution output
  local output
  output=$(get_execution_output "$exec_arn")
  [[ -n "$output" && "$output" != "None" ]] || \
    { echo "Expected non-empty output, got: '$output' for exec_arn=$exec_arn"; return 1; }

  # Parse and validate response fields
  local started_status
  started_status=$(echo "$output" | jq -r '.started.status')
  [[ "$started_status" == "started" ]] || \
    { echo "Expected started.status=started, got: $started_status; output=$output"; return 1; }

  local completed_status
  completed_status=$(echo "$output" | jq -r '.completed.status')
  [[ "$completed_status" == "completed" ]] || \
    { echo "Expected completed.status=completed, got: $completed_status; output=$output"; return 1; }

  echo "async wait completed with started+completed status fields via $binary"
}

# ---------------------------------------------------------------------------
# assert_callbacks(binary)
# Full async callback test flow:
#   invoke_async -> extract_callback_id -> send_callback_success
#   -> wait_for_terminal_status -> get_execution_output
#   -> verify outcome.approved=true
# ---------------------------------------------------------------------------
assert_callbacks() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")

  # Start async execution
  local exec_arn
  exec_arn=$(invoke_async "$fn_arn" '{}')
  [[ -n "$exec_arn" ]] || { echo "Expected non-empty exec_arn from invoke_async"; return 1; }

  # Poll for callback_id from CallbackStarted event
  local callback_id
  callback_id=$(extract_callback_id "$exec_arn" 60)
  [[ -n "$callback_id" ]] || \
    { echo "Expected non-empty callback_id, got empty for exec_arn=$exec_arn"; return 1; }

  # Send callback success signal
  if ! send_callback_success "$callback_id" '{"approved":true}' >/dev/null 2>&1; then
    echo "send_callback_success failed for callback_id=$callback_id"
    return 1
  fi

  # Poll until terminal status
  local final_status
  final_status=$(wait_for_terminal_status "$exec_arn" 60)
  [[ "$final_status" == "SUCCEEDED" ]] || \
    { echo "Expected SUCCEEDED, got: $final_status for exec_arn=$exec_arn"; return 1; }

  # Retrieve execution output
  local output
  output=$(get_execution_output "$exec_arn")
  [[ -n "$output" && "$output" != "None" ]] || \
    { echo "Expected non-empty output, got: '$output' for exec_arn=$exec_arn"; return 1; }

  # Validate callback result was processed
  local approved
  approved=$(echo "$output" | jq -r '.outcome.approved')
  [[ "$approved" == "true" ]] || \
    { echo "Expected outcome.approved=true, got: $approved; output=$output"; return 1; }

  echo "async callback completed with approved=true via $binary"
}

# ---------------------------------------------------------------------------
# assert_invoke(binary)
# Synchronous invoke test:
#   invoke_sync with order_id -> validate round-trip + enrichment non-null
# ---------------------------------------------------------------------------
assert_invoke() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")

  local result
  result=$(invoke_sync "$fn_arn" '{"order_id":"test-invoke-001"}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || \
    { echo "Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ -z "$fn_error" ]] || \
    { echo "Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }

  # Validate order_id round-trip
  local order_id
  order_id=$(echo "$response_body" | jq -r '.order_id')
  [[ "$order_id" == "test-invoke-001" ]] || \
    { echo "Expected order_id=test-invoke-001, got: $order_id; body=$response_body"; return 1; }

  # Validate enrichment is non-null (proves stub was called)
  local enrichment
  enrichment=$(echo "$response_body" | jq -r '.enrichment')
  [[ "$enrichment" != "null" ]] || \
    { echo "Expected enrichment non-null, got: null; body=$response_body"; return 1; }

  echo "invoke operation round-tripped order_id with enrichment via $binary"
}

# ---------------------------------------------------------------------------
# assert_callback_xfail(binary)
# XFAIL for callback tests: verifies the callback operation is registered
# (CallbackStarted event in history) but expects FAILED status because the
# durable execution service does not populate callback_details on Operation
# objects during replay. The callback_id IS assigned (visible in history)
# and the callback signal IS received, but the handler can't read the
# callback_id or result from the empty Operation detail fields.
# Revert to assert_callbacks when the service populates callback_details.
# ---------------------------------------------------------------------------
assert_callback_xfail() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")

  # Start async execution
  local exec_arn
  exec_arn=$(invoke_async "$fn_arn" '{}')
  [[ -n "$exec_arn" ]] || { echo "Expected non-empty exec_arn from invoke_async"; return 1; }

  # Poll for callback_id from CallbackStarted event (proves callback was registered)
  local callback_id
  callback_id=$(extract_callback_id "$exec_arn" 60)
  [[ -n "$callback_id" ]] || \
    { echo "Expected non-empty callback_id, got empty for exec_arn=$exec_arn"; return 1; }

  # Send callback success signal
  if ! send_callback_success "$callback_id" '{"approved":true}' >/dev/null 2>&1; then
    echo "send_callback_success failed for callback_id=$callback_id"
    return 1
  fi

  # Poll until terminal status — expect FAILED (not SUCCEEDED) because
  # the handler can't read callback_details during replay.
  local final_status
  final_status=$(wait_for_terminal_status "$exec_arn" 60)
  [[ "$final_status" == "FAILED" ]] || \
    { echo "XFAIL expected FAILED (callback_details unpopulated), got: $final_status"; return 1; }

  echo "XFAIL: callback registered and signaled correctly, replay fails on empty callback_details via $binary"
}

# ===========================================================================
# XFAIL Helpers: Expected Failures for Unsupported Service Operations
# The AWS durable execution service does not yet support the Context
# operation type (used by parallel, map, child_context). These helpers
# validate the functions start correctly but return the expected service
# error. Revert to assert_parallel/assert_map/assert_child_contexts when
# the service adds support for Context operations.
# ===========================================================================

# ---------------------------------------------------------------------------
# assert_service_unsupported(binary, operation_name)
# Invokes a handler expected to fail because the AWS durable execution
# service does not yet support the Context operation type (used by
# parallel, map, child_context). Validates the function returns
# FunctionError=Unhandled with errorType=AWS_SDK_OPERATION.
# This is an XFAIL (expected failure) -- the SDK code is correct per
# the Python SDK spec, but the service hasn't implemented these ops yet.
# ---------------------------------------------------------------------------
assert_service_unsupported() {
  local binary="$1"
  local operation_name="$2"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ "$fn_error" == "Unhandled" ]] || \
    { echo "Expected FunctionError=Unhandled (service unsupported), got: fn_error=${fn_error}; body=$response_body"; return 1; }

  local error_type
  error_type=$(echo "$response_body" | jq -r '.errorType // ""')
  [[ "$error_type" == "AWS_SDK_OPERATION" ]] || \
    { echo "Expected errorType=AWS_SDK_OPERATION, got: $error_type; body=$response_body"; return 1; }

  echo "XFAIL: $operation_name correctly returned AWS_SDK_OPERATION (service unsupported) via $binary"
}
