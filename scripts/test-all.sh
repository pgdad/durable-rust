#!/usr/bin/env bash
# scripts/test-all.sh — Integration test runner for durable-rust Lambda functions.
# Usage:
#   bash scripts/test-all.sh                    # Run all tests
#   bash scripts/test-all.sh closure-basic-steps # Run single named test
#
# Prerequisites: ADFS credentials valid, Lambda functions deployed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

# ---------------------------------------------------------------------------
# Result tracking
# ---------------------------------------------------------------------------
PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0
declare -a FAILURES=()

# ---------------------------------------------------------------------------
# run_test(test_name, test_fn)
# Runs a single test function in a subshell, captures output, records result.
# Uses if-conditional so set -e does NOT abort on test failure.
# ---------------------------------------------------------------------------
run_test() {
  local test_name="$1"
  local test_fn="$2"
  printf "  %-55s" "$test_name"
  local error_msg
  if error_msg=$("$test_fn" 2>&1); then
    echo "[PASS]"
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    echo "[FAIL]"
    FAILURES+=("${test_name}: $(echo "$error_msg" | tail -1)")
    FAIL_COUNT=$((FAIL_COUNT + 1))
  fi
}

# ---------------------------------------------------------------------------
# print_results
# Prints summary table and returns 0 if all passed, 1 if any failed.
# ---------------------------------------------------------------------------
print_results() {
  echo ""
  echo "=== Results: ${PASS_COUNT} passed, ${FAIL_COUNT} failed, ${SKIP_COUNT} skipped ==="
  if [[ ${#FAILURES[@]} -gt 0 ]]; then
    echo ""
    for failure in "${FAILURES[@]}"; do
      echo "  FAIL: ${failure}"
    done
  fi
  [[ $FAIL_COUNT -eq 0 ]]
}

# ---------------------------------------------------------------------------
# === Phase 14: Synchronous Operation Tests ===
# basic_steps, step_retries, typed_errors, parallel, map, child_contexts,
# replay_safe_logging, combined_workflow — 4 styles each = 32 tests
# ---------------------------------------------------------------------------

test_closure_basic_steps()         { assert_basic_steps "closure-basic-steps"; }
test_macro_basic_steps()           { assert_basic_steps "macro-basic-steps"; }
test_trait_basic_steps()           { assert_basic_steps "trait-basic-steps"; }
test_builder_basic_steps()         { assert_basic_steps "builder-basic-steps"; }

test_closure_step_retries()        { assert_step_retries "closure-step-retries"; }
test_macro_step_retries()          { assert_step_retries "macro-step-retries"; }
test_trait_step_retries()          { assert_step_retries "trait-step-retries"; }
test_builder_step_retries()        { assert_step_retries "builder-step-retries"; }

test_closure_typed_errors()        { assert_typed_errors "closure-typed-errors"; }
test_macro_typed_errors()          { assert_typed_errors "macro-typed-errors"; }
test_trait_typed_errors()          { assert_typed_errors "trait-typed-errors"; }
test_builder_typed_errors()        { assert_typed_errors "builder-typed-errors"; }

test_closure_parallel()            { assert_parallel "closure-parallel"; }
test_macro_parallel()              { assert_parallel "macro-parallel"; }
test_trait_parallel()              { assert_parallel "trait-parallel"; }
test_builder_parallel()            { assert_parallel "builder-parallel"; }

test_closure_map()                 { assert_map "closure-map"; }
test_macro_map()                   { assert_map "macro-map"; }
test_trait_map()                   { assert_map "trait-map"; }
test_builder_map()                 { assert_map "builder-map"; }

test_closure_child_contexts()      { assert_child_contexts "closure-child-contexts"; }
test_macro_child_contexts()        { assert_child_contexts "macro-child-contexts"; }
test_trait_child_contexts()        { assert_child_contexts "trait-child-contexts"; }
test_builder_child_contexts()      { assert_child_contexts "builder-child-contexts"; }

test_closure_replay_safe_logging() { assert_replay_safe_logging "closure-replay-safe-logging"; }
test_macro_replay_safe_logging()   { assert_replay_safe_logging "macro-replay-safe-logging"; }
test_trait_replay_safe_logging()   { assert_replay_safe_logging "trait-replay-safe-logging"; }
test_builder_replay_safe_logging() { assert_replay_safe_logging "builder-replay-safe-logging"; }

test_closure_combined_workflow()   { assert_combined_workflow "closure-combined-workflow"; }
test_macro_combined_workflow()     { assert_combined_workflow "macro-combined-workflow"; }
test_trait_combined_workflow()     { assert_combined_workflow "trait-combined-workflow"; }
test_builder_combined_workflow()   { assert_combined_workflow "builder-combined-workflow"; }

# ---------------------------------------------------------------------------
# === Phase 15: Async Operation Tests ===
# waits, callbacks, invoke — 4 styles each = 12 tests
# ---------------------------------------------------------------------------

test_closure_waits()               { assert_waits "closure-waits"; }
test_macro_waits()                 { assert_waits "macro-waits"; }
test_trait_waits()                 { assert_waits "trait-waits"; }
test_builder_waits()               { assert_waits "builder-waits"; }

test_closure_callbacks()           { assert_callbacks "closure-callbacks"; }
test_macro_callbacks()             { assert_callbacks "macro-callbacks"; }
test_trait_callbacks()             { assert_callbacks "trait-callbacks"; }
test_builder_callbacks()           { assert_callbacks "builder-callbacks"; }

test_closure_invoke()              { assert_invoke "closure-invoke"; }
test_macro_invoke()                { assert_invoke "macro-invoke"; }
test_trait_invoke()                { assert_invoke "trait-invoke"; }
test_builder_invoke()              { assert_invoke "builder-invoke"; }

# ---------------------------------------------------------------------------
# === Phase 16: Advanced Feature Tests ===
# saga_compensation, step_timeout, conditional_retry, batch_checkpoint
# These use specific binaries, not 4-style variants
# ---------------------------------------------------------------------------

test_closure_saga_compensation() {
  local fn_arn
  fn_arn=$(get_alias_arn "closure-saga-compensation")
  local result
  result=$(invoke_sync "$fn_arn" '{}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error"; return 1; }

  # The durable execution service unwraps SUCCEEDED responses and returns the
  # user JSON directly. Check the compensation fields in the unwrapped response.
  local saga_status
  saga_status=$(echo "$response_body" | jq -r '.status')
  [[ "$saga_status" == "rolled_back" ]] || \
    { echo "Expected status=rolled_back, got: $saga_status; body=$response_body"; return 1; }

  local seq
  seq=$(echo "$response_body" | jq -r '.compensation_sequence | join(",")')
  [[ "$seq" == "charge_card,book_flight,book_hotel" ]] || \
    { echo "Expected LIFO compensation_sequence, got: $seq"; return 1; }

  local all_succeeded
  all_succeeded=$(echo "$response_body" | jq -r '.all_succeeded')
  [[ "$all_succeeded" == "true" ]] || \
    { echo "Expected all_succeeded=true, got: $all_succeeded; body=$response_body"; return 1; }

  echo "saga compensation rollback succeeded in LIFO order"
}

test_closure_step_timeout() {
  local fn_arn
  fn_arn=$(get_alias_arn "closure-step-timeout")
  local result
  result=$(invoke_sync "$fn_arn" '{}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status"; return 1; }

  # The durable execution service converts FAILED durable responses into a
  # Lambda FunctionError. We expect FunctionError=Unhandled here.
  [[ "$fn_error" == "Unhandled" ]] || \
    { echo "Expected FunctionError=Unhandled (step timeout = FAILED), got: fn_error=${fn_error}; body=$response_body"; return 1; }

  # The error body should identify the timeout — errorType=STEP_TIMEOUT from the durable Error object.
  local error_type error_msg
  error_type=$(echo "$response_body" | jq -r '.errorType // ""')
  error_msg=$(echo "$response_body" | jq -r '.errorMessage // ""')
  [[ "$error_type" == "STEP_TIMEOUT" ]] || \
    { echo "Expected errorType=STEP_TIMEOUT, got: $error_type; body=$response_body"; return 1; }
  echo "$error_msg" | grep -qi "timed out\|timeout" || \
    { echo "Expected timeout in errorMessage, got: $error_msg"; return 1; }

  echo "step timeout correctly produced FunctionError with STEP_TIMEOUT"
}

test_closure_conditional_retry() {
  local fn_arn
  fn_arn=$(get_alias_arn "closure-conditional-retry")
  local result
  result=$(invoke_sync "$fn_arn" '{"error_type":"non_retryable"}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no Lambda FunctionError, got: $fn_error"; return 1; }

  # The durable execution service unwraps SUCCEEDED responses and returns the
  # user JSON directly. The handler returns Ok({"result": Err("non_retryable")})
  # which demonstrates the retry_if predicate skipped retrying the non-retryable error.
  local err_value
  err_value=$(echo "$response_body" | jq -r '.result.Err // ""')
  [[ "$err_value" == "non_retryable" ]] || \
    { echo "Expected result.Err=non_retryable (retry skipped), got: $err_value; body=$response_body"; return 1; }

  echo "non-retryable path verified: retry_if predicate correctly skipped retry on non-matching error"
}

test_closure_batch_checkpoint() {
  local fn_arn
  fn_arn=$(get_alias_arn "closure-batch-checkpoint")
  local result
  result=$(invoke_sync "$fn_arn" '{"batch":true}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error"; return 1; }

  # The durable execution service unwraps SUCCEEDED responses and returns the
  # user JSON directly (no Status envelope visible to the caller).
  local batch_mode
  batch_mode=$(echo "$response_body" | jq -r '.batch_mode')
  [[ "$batch_mode" == "true" ]] || \
    { echo "Expected batch_mode=true, got: $batch_mode; body=$response_body"; return 1; }

  local steps_completed
  steps_completed=$(echo "$response_body" | jq -r '.steps_completed')
  [[ "$steps_completed" == "5" ]] || \
    { echo "Expected steps_completed=5, got: $steps_completed; body=$response_body"; return 1; }

  echo "batch checkpoint handler succeeded with 5 steps"
}

# ---------------------------------------------------------------------------
# BINARY_TO_TEST
# Associative array mapping binary names to test function names.
# Used for single-test dispatch mode.
# ---------------------------------------------------------------------------
declare -A BINARY_TO_TEST

# Phase 14 — synchronous
BINARY_TO_TEST["closure-basic-steps"]="test_closure_basic_steps"
BINARY_TO_TEST["macro-basic-steps"]="test_macro_basic_steps"
BINARY_TO_TEST["trait-basic-steps"]="test_trait_basic_steps"
BINARY_TO_TEST["builder-basic-steps"]="test_builder_basic_steps"

BINARY_TO_TEST["closure-step-retries"]="test_closure_step_retries"
BINARY_TO_TEST["macro-step-retries"]="test_macro_step_retries"
BINARY_TO_TEST["trait-step-retries"]="test_trait_step_retries"
BINARY_TO_TEST["builder-step-retries"]="test_builder_step_retries"

BINARY_TO_TEST["closure-typed-errors"]="test_closure_typed_errors"
BINARY_TO_TEST["macro-typed-errors"]="test_macro_typed_errors"
BINARY_TO_TEST["trait-typed-errors"]="test_trait_typed_errors"
BINARY_TO_TEST["builder-typed-errors"]="test_builder_typed_errors"

BINARY_TO_TEST["closure-parallel"]="test_closure_parallel"
BINARY_TO_TEST["macro-parallel"]="test_macro_parallel"
BINARY_TO_TEST["trait-parallel"]="test_trait_parallel"
BINARY_TO_TEST["builder-parallel"]="test_builder_parallel"

BINARY_TO_TEST["closure-map"]="test_closure_map"
BINARY_TO_TEST["macro-map"]="test_macro_map"
BINARY_TO_TEST["trait-map"]="test_trait_map"
BINARY_TO_TEST["builder-map"]="test_builder_map"

BINARY_TO_TEST["closure-child-contexts"]="test_closure_child_contexts"
BINARY_TO_TEST["macro-child-contexts"]="test_macro_child_contexts"
BINARY_TO_TEST["trait-child-contexts"]="test_trait_child_contexts"
BINARY_TO_TEST["builder-child-contexts"]="test_builder_child_contexts"

BINARY_TO_TEST["closure-replay-safe-logging"]="test_closure_replay_safe_logging"
BINARY_TO_TEST["macro-replay-safe-logging"]="test_macro_replay_safe_logging"
BINARY_TO_TEST["trait-replay-safe-logging"]="test_trait_replay_safe_logging"
BINARY_TO_TEST["builder-replay-safe-logging"]="test_builder_replay_safe_logging"

BINARY_TO_TEST["closure-combined-workflow"]="test_closure_combined_workflow"
BINARY_TO_TEST["macro-combined-workflow"]="test_macro_combined_workflow"
BINARY_TO_TEST["trait-combined-workflow"]="test_trait_combined_workflow"
BINARY_TO_TEST["builder-combined-workflow"]="test_builder_combined_workflow"

# Phase 15 — async
BINARY_TO_TEST["closure-waits"]="test_closure_waits"
BINARY_TO_TEST["macro-waits"]="test_macro_waits"
BINARY_TO_TEST["trait-waits"]="test_trait_waits"
BINARY_TO_TEST["builder-waits"]="test_builder_waits"

BINARY_TO_TEST["closure-callbacks"]="test_closure_callbacks"
BINARY_TO_TEST["macro-callbacks"]="test_macro_callbacks"
BINARY_TO_TEST["trait-callbacks"]="test_trait_callbacks"
BINARY_TO_TEST["builder-callbacks"]="test_builder_callbacks"

BINARY_TO_TEST["closure-invoke"]="test_closure_invoke"
BINARY_TO_TEST["macro-invoke"]="test_macro_invoke"
BINARY_TO_TEST["trait-invoke"]="test_trait_invoke"
BINARY_TO_TEST["builder-invoke"]="test_builder_invoke"

# Phase 16 — advanced
BINARY_TO_TEST["closure-saga-compensation"]="test_closure_saga_compensation"
BINARY_TO_TEST["closure-step-timeout"]="test_closure_step_timeout"
BINARY_TO_TEST["closure-conditional-retry"]="test_closure_conditional_retry"
BINARY_TO_TEST["closure-batch-checkpoint"]="test_closure_batch_checkpoint"

# ---------------------------------------------------------------------------
# run_all_tests
# Runs every test in defined order: Phase 14 sync, Phase 15 async,
# Phase 16 advanced.
# ---------------------------------------------------------------------------
run_all_tests() {
  # Phase 14 — Synchronous Operation Tests
  run_test "closure-basic-steps"         test_closure_basic_steps
  run_test "macro-basic-steps"           test_macro_basic_steps
  run_test "trait-basic-steps"           test_trait_basic_steps
  run_test "builder-basic-steps"         test_builder_basic_steps

  run_test "closure-step-retries"        test_closure_step_retries
  run_test "macro-step-retries"          test_macro_step_retries
  run_test "trait-step-retries"          test_trait_step_retries
  run_test "builder-step-retries"        test_builder_step_retries

  run_test "closure-typed-errors"        test_closure_typed_errors
  run_test "macro-typed-errors"          test_macro_typed_errors
  run_test "trait-typed-errors"          test_trait_typed_errors
  run_test "builder-typed-errors"        test_builder_typed_errors

  run_test "closure-parallel"            test_closure_parallel
  run_test "macro-parallel"              test_macro_parallel
  run_test "trait-parallel"              test_trait_parallel
  run_test "builder-parallel"            test_builder_parallel

  run_test "closure-map"                 test_closure_map
  run_test "macro-map"                   test_macro_map
  run_test "trait-map"                   test_trait_map
  run_test "builder-map"                 test_builder_map

  run_test "closure-child-contexts"      test_closure_child_contexts
  run_test "macro-child-contexts"        test_macro_child_contexts
  run_test "trait-child-contexts"        test_trait_child_contexts
  run_test "builder-child-contexts"      test_builder_child_contexts

  run_test "closure-replay-safe-logging" test_closure_replay_safe_logging
  run_test "macro-replay-safe-logging"   test_macro_replay_safe_logging
  run_test "trait-replay-safe-logging"   test_trait_replay_safe_logging
  run_test "builder-replay-safe-logging" test_builder_replay_safe_logging

  run_test "closure-combined-workflow"   test_closure_combined_workflow
  run_test "macro-combined-workflow"     test_macro_combined_workflow
  run_test "trait-combined-workflow"     test_trait_combined_workflow
  run_test "builder-combined-workflow"   test_builder_combined_workflow

  # Phase 15 — Async Operation Tests
  run_test "closure-waits"               test_closure_waits
  run_test "macro-waits"                 test_macro_waits
  run_test "trait-waits"                 test_trait_waits
  run_test "builder-waits"              test_builder_waits

  run_test "closure-callbacks"           test_closure_callbacks
  run_test "macro-callbacks"             test_macro_callbacks
  run_test "trait-callbacks"             test_trait_callbacks
  run_test "builder-callbacks"           test_builder_callbacks

  run_test "closure-invoke"              test_closure_invoke
  run_test "macro-invoke"                test_macro_invoke
  run_test "trait-invoke"                test_trait_invoke
  run_test "builder-invoke"             test_builder_invoke

  # Phase 16 — Advanced Feature Tests
  run_test "closure-saga-compensation"  test_closure_saga_compensation
  run_test "closure-step-timeout"       test_closure_step_timeout
  run_test "closure-conditional-retry"  test_closure_conditional_retry
  run_test "closure-batch-checkpoint"   test_closure_batch_checkpoint
}

# ---------------------------------------------------------------------------
# main
# Entry point: credential gate, Terraform outputs, then run tests.
# ---------------------------------------------------------------------------
main() {
  echo "=== durable-rust Integration Tests ==="
  echo ""

  check_credentials
  load_tf_outputs

  if [[ $# -gt 0 ]]; then
    local requested="$1"
    if [[ -v "BINARY_TO_TEST[$requested]" ]]; then
      run_test "$requested" "${BINARY_TO_TEST[$requested]}"
    else
      echo "Unknown test: ${requested}"
      echo ""
      echo "Available tests:"
      for key in $(printf '%s\n' "${!BINARY_TO_TEST[@]}" | sort); do
        echo "  $key"
      done
      exit 1
    fi
  else
    run_all_tests
  fi

  print_results
}

main "$@"
