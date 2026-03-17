# Phase 13: Test Harness - Research

**Researched:** 2026-03-17
**Domain:** Bash shell scripting, AWS CLI (Lambda Durable Execution APIs), jq, test orchestration
**Confidence:** HIGH

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TEST-01 | Single command (test-all.sh) runs all integration tests with per-test pass/fail | Shell function dispatch table + status aggregation pattern |
| TEST-02 | Execution status polling helper for durable executions (SUCCEEDED/FAILED/TIMED_OUT) | `list-durable-executions-by-function` + `get-durable-execution` APIs confirmed |
| TEST-03 | Callback signal tooling (extract callback_id, send SendDurableExecutionCallbackSuccess) | `get-durable-execution-history` CallbackStartedDetails.CallbackId confirmed |
| TEST-04 | ADFS credential validity check before starting | `aws sts get-caller-identity` pattern in verify-prerequisites.sh already established |
| TEST-05 | Per-test output with name, status, failure reason | PASS/FAIL counter + failure array aggregation pattern |
| TEST-06 | Individual test execution via CLI argument | Bash case/dispatch by function name from $1 argument |
</phase_requirements>

## Summary

Phase 13 builds a Bash test harness (`scripts/test-all.sh`) that can invoke any of the 44 deployed Lambda functions, wait for them to reach a terminal state, and report per-test PASS/FAIL. The harness is consumed by Phase 14 (synchronous operation tests) and Phase 15 (async operation tests), so its helper functions must be designed to serve both test phases without modification.

The domain is Bash + AWS CLI. No new infrastructure is needed — all AWS resources (Lambda aliases, ECR) are already deployed. The harness reads its configuration entirely from Terraform outputs so function ARNs are never hardcoded. The most critical technical question — how to extract a callback ID in a test scenario — is now fully resolved: `aws lambda get-durable-execution-history` returns a `CallbackStartedDetails.CallbackId` field in the event for the `CallbackStarted` event type. The `DurableExecutionArn` needed to call that API is a top-level JSON output field from `aws lambda invoke` (not an HTTP header as some documentation implies).

The waits handlers hardcode a 60-second duration (`ctx.wait("cooling_period", 60)`) — the test harness must tolerate this and set the polling timeout accordingly. This is a Phase 14/15 concern but the polling helper's configurable timeout must accommodate it. The invoke handlers hardcode the callee name `"order-enrichment-lambda"`, which is resolved via event payload injection per the Phase 11 decision.

**Primary recommendation:** Build the harness as a single `test-all.sh` with shared helper functions sourced or defined inline: `invoke_sync`, `invoke_async`, `wait_for_terminal_status`, `extract_callback_id`, `send_callback_success`, and `run_test`. Each test function calls these helpers. The harness exits non-zero if any test fails.

## Standard Stack

### Core

| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| Bash | 5.x | Script runtime | Already established in build-images.sh and verify-prerequisites.sh |
| AWS CLI v2 | 2.27+ | All Lambda API calls | Official tool; already required for ECR login |
| jq | 1.7+ | JSON field extraction | Already required in prerequisites; no alternative for CLI JSON parsing |
| Terraform CLI | 1.14.7 | Read outputs (function ARNs) | Already installed; outputs.tf has alias_arns map |

### No New Dependencies

This phase adds no new tools. All of the following are already present:
- `aws lambda invoke` — Lambda invocation
- `aws lambda list-durable-executions-by-function` — status polling
- `aws lambda get-durable-execution` — terminal state check
- `aws lambda get-durable-execution-history` — callback ID extraction
- `aws lambda send-durable-execution-callback-success` — callback signal
- `aws sts get-caller-identity` — credential check (in verify-prerequisites.sh)

## Architecture Patterns

### Recommended Script Structure

```
scripts/
├── verify-prerequisites.sh    # Existing — gates all scripts
├── build-images.sh            # Existing
├── deploy-ecr.sh              # Existing
└── test-all.sh                # NEW — Phase 13
```

`test-all.sh` is a single file. Helper functions are defined at the top, test functions follow, then a dispatch section at the bottom that either runs all tests or the one named in `$1`.

### Pattern 1: Credential Check (TEST-04)

**What:** Run `aws sts get-caller-identity` at harness startup; fail fast if expired or absent.
**When to use:** Always, first action before any Lambda call.
**Example:**
```bash
# Source: verify-prerequisites.sh pattern (already established)
PROFILE="adfs"
REGION="us-east-2"

check_credentials() {
  echo "=== Checking ADFS credentials ==="
  if ! aws sts get-caller-identity \
      --profile "$PROFILE" \
      --region "$REGION" \
      --output text > /dev/null 2>&1; then
    echo "ERROR: ADFS credentials expired or missing. Run: adfs-auth"
    exit 1
  fi
  echo "  [OK] ADFS credentials valid"
}
```

### Pattern 2: Read Terraform Outputs (Never Hardcode ARNs)

**What:** Pull alias ARNs and function names from `terraform output -json`.
**Why:** Suffix (c351) is workspace-specific; hardcoding breaks after `terraform destroy`/re-apply.
**Example:**
```bash
# Source: build-images.sh pattern (already established)
TF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/infra"

load_tf_outputs() {
  ALIAS_ARNS=$(terraform -chdir="$TF_DIR" output -json alias_arns)
  STUB_ARNS=$(terraform -chdir="$TF_DIR" output -json stub_alias_arns)
  SUFFIX=$(terraform -chdir="$TF_DIR" output -raw suffix)
}

# Get a specific function's alias ARN
get_alias_arn() {
  local binary_name="$1"
  echo "$ALIAS_ARNS" | jq -r --arg name "$binary_name" '.[$name]'
}
```

### Pattern 3: Synchronous Invocation (TEST-01, TEST-05)

**What:** Invoke a Lambda synchronously; capture response and check for function errors.
**When to use:** Step, parallel, map, child_context, logging tests — operations that do not suspend.
**Key detail:** `DurableExecutionArn` is a top-level JSON output field from `aws lambda invoke` (confirmed from CLI docs). The response payload is written to a temp file.

```bash
# Source: AWS CLI docs (aws lambda invoke output shape)
invoke_sync() {
  local function_arn="$1"
  local payload="$2"        # JSON string
  local out_file
  out_file=$(mktemp /tmp/lambda-out-XXXXXX.json)

  # aws lambda invoke writes metadata JSON to stdout; response body to out_file
  local meta
  meta=$(aws lambda invoke \
    --profile "$PROFILE" \
    --region "$REGION" \
    --function-name "$function_arn" \
    --invocation-type RequestResponse \
    --cli-binary-format raw-in-base64-out \
    --payload "$payload" \
    "$out_file" 2>&1)

  local status_code
  status_code=$(echo "$meta" | jq -r '.StatusCode // 0')

  local fn_error
  fn_error=$(echo "$meta" | jq -r '.FunctionError // empty')

  local exec_arn
  exec_arn=$(echo "$meta" | jq -r '.DurableExecutionArn // empty')

  local response_body
  response_body=$(cat "$out_file" 2>/dev/null || echo '{}')
  rm -f "$out_file"

  echo "$status_code|$fn_error|$exec_arn|$response_body"
}
```

### Pattern 4: Async Invocation (for Wait/Callback tests)

**What:** Invoke a Lambda asynchronously (Event invocation type); capture the DurableExecutionArn.
**When to use:** Wait and Callback operations — these suspend the Lambda immediately.
**Key detail:** Async invoke returns `StatusCode: 202` and `DurableExecutionArn` in the metadata JSON.

```bash
# Source: AWS CLI docs (--invocation-type Event + DurableExecutionArn output field)
invoke_async() {
  local function_arn="$1"
  local payload="$2"
  local out_file
  out_file=$(mktemp /tmp/lambda-out-XXXXXX.json)

  local meta
  meta=$(aws lambda invoke \
    --profile "$PROFILE" \
    --region "$REGION" \
    --function-name "$function_arn" \
    --invocation-type Event \
    --cli-binary-format raw-in-base64-out \
    --payload "$payload" \
    "$out_file" 2>&1)

  rm -f "$out_file"

  local exec_arn
  exec_arn=$(echo "$meta" | jq -r '.DurableExecutionArn // empty')

  echo "$exec_arn"
}
```

### Pattern 5: Status Polling Helper (TEST-02)

**What:** Poll `list-durable-executions-by-function` until the execution reaches a terminal state.
**Terminal states:** `SUCCEEDED`, `FAILED`, `TIMED_OUT`, `STOPPED`
**When to use:** After every async invocation, and optionally after synchronous invocation for durable state confirmation.
**Polling strategy:** Fixed 3-second interval (not exponential backoff — keeps the script simple and predictable); configurable timeout parameter.

```bash
# Source: AWS CLI docs (list-durable-executions-by-function, Status enum)
TERMINAL_STATES=("SUCCEEDED" "FAILED" "TIMED_OUT" "STOPPED")

wait_for_terminal_status() {
  local function_arn="$1"     # qualified ARN (with :live alias)
  local exec_arn="$2"         # DurableExecutionArn from invoke
  local timeout_seconds="${3:-120}"  # default 2-minute timeout
  local interval=3

  local elapsed=0
  while [[ $elapsed -lt $timeout_seconds ]]; do
    local status
    status=$(aws lambda get-durable-execution \
      --profile "$PROFILE" \
      --region "$REGION" \
      --durable-execution-arn "$exec_arn" \
      --query 'Status' \
      --output text 2>/dev/null || echo "UNKNOWN")

    for terminal in "${TERMINAL_STATES[@]}"; do
      if [[ "$status" == "$terminal" ]]; then
        echo "$status"
        return 0
      fi
    done

    sleep $interval
    elapsed=$((elapsed + interval))
  done

  echo "TIMEOUT"
  return 1
}
```

**Important:** `get-durable-execution` requires `--durable-execution-arn` (the full ARN). The ARN comes from either the `invoke` output or from `list-durable-executions-by-function` filtered by execution name.

### Pattern 6: Callback ID Extraction (TEST-03)

**What:** After async invoke, poll history until `CallbackStarted` event appears; extract `CallbackId`.
**Key API:** `aws lambda get-durable-execution-history` — returns `Events[].EventType` and `Events[].CallbackStartedDetails.CallbackId`.
**When to use:** Callback operation tests only.
**Critical:** Do NOT poll `get-durable-execution` for `WAITING_FOR_CALLBACK` status — that status value is not in the documented Status enum for that API. Use history events instead.

```bash
# Source: AWS CLI docs (get-durable-execution-history, CallbackStartedDetails)
extract_callback_id() {
  local exec_arn="$1"
  local timeout_seconds="${2:-60}"
  local interval=3
  local elapsed=0

  while [[ $elapsed -lt $timeout_seconds ]]; do
    local callback_id
    callback_id=$(aws lambda get-durable-execution-history \
      --profile "$PROFILE" \
      --region "$REGION" \
      --durable-execution-arn "$exec_arn" \
      --query 'Events[?EventType==`CallbackStarted`].CallbackStartedDetails.CallbackId | [0]' \
      --output text 2>/dev/null || echo "None")

    if [[ -n "$callback_id" && "$callback_id" != "None" && "$callback_id" != "null" ]]; then
      echo "$callback_id"
      return 0
    fi

    sleep $interval
    elapsed=$((elapsed + interval))
  done

  echo ""
  return 1
}
```

### Pattern 7: Send Callback Success (TEST-03)

**What:** Send `SendDurableExecutionCallbackSuccess` with the extracted callback ID.
**Result format:** JSON string (blob parameter); use `--cli-binary-format raw-in-base64-out`.

```bash
# Source: AWS CLI docs (send-durable-execution-callback-success)
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
```

### Pattern 8: Test Runner with Per-Test Reporting (TEST-05, TEST-06)

**What:** Each test is a Bash function. A central `run_test` function executes it, captures pass/fail, and records the result.
**Individual test execution (TEST-06):** Pass the test function name as `$1`; the dispatch section calls it directly.

```bash
# Per-test result tracking
PASS_COUNT=0
FAIL_COUNT=0
declare -a FAILURES=()

run_test() {
  local test_name="$1"
  local test_fn="$2"
  printf "  %-55s " "$test_name"

  local error_msg
  if error_msg=$("$test_fn" 2>&1); then
    echo "[PASS]"
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    echo "[FAIL]"
    FAILURES+=("$test_name: $error_msg")
    FAIL_COUNT=$((FAIL_COUNT + 1))
  fi
}

# Summary
print_results() {
  echo ""
  echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="
  for failure in "${FAILURES[@]}"; do
    echo "  FAIL: $failure"
  done
  [[ $FAIL_COUNT -eq 0 ]]
}

# Dispatch (TEST-06)
if [[ -n "${1:-}" ]]; then
  run_test "$1" "$1"   # run a single named test
else
  run_all_tests        # run everything
fi
```

### Pattern 9: Durable Execution Name for Test Isolation

**What:** Pass `--durable-execution-name` to prevent duplicate executions on re-run.
**Format:** `[a-zA-Z0-9-_]+`, max 64 chars.
**Recommended:** `{binary-name}-{test-run-id}` where `test-run-id=$(date +%Y%m%d%H%M%S)`.

```bash
# Unique test run ID set once at harness startup
TEST_RUN_ID=$(date +%Y%m%d%H%M%S)

make_exec_name() {
  local binary_name="$1"
  # Truncate to 64 chars; replace hyphens that exceed with underscores if needed
  echo "${binary_name}-${TEST_RUN_ID}" | cut -c1-64
}
```

### Anti-Patterns to Avoid

- **Hardcoding function ARNs or suffix**: Use `terraform output -json alias_arns | jq`.
- **Sleeping instead of polling for callback**: Always use `get-durable-execution-history` to detect `CallbackStarted`; never `sleep 30`.
- **Using unqualified function name**: Always use the `:live` alias ARN from Terraform outputs.
- **Checking `FunctionError` in the payload body instead of metadata**: `FunctionError` is in the `aws lambda invoke` stdout metadata JSON (not in the response file).
- **Reusing the same execution name across test runs**: Pass `--durable-execution-name` with a timestamp suffix to ensure idempotency without collision.
- **Setting `set -e` without per-command error handling**: `set -euo pipefail` is correct, but individual test functions must capture failures via `run_test`'s subshell, not propagate to the top level.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON parsing | Custom regex/awk | `jq` | jq handles nested fields, null safety, array filters |
| Function ARN lookup | Hardcoded map | `terraform output -json alias_arns \| jq` | Suffix changes per workspace; Terraform is the single source of truth |
| Status polling | Custom HTTP client | `aws lambda get-durable-execution` | CLI handles auth, retries, JSON parsing |
| Callback ID extraction | Parsing CloudWatch logs | `aws lambda get-durable-execution-history` | Official API; logs are not guaranteed to be structured |

**Key insight:** The AWS CLI handles all auth, signing, and JSON serialization. The test harness is glue code — keep it thin.

## Common Pitfalls

### Pitfall 1: DurableExecutionArn in Invoke Response

**What goes wrong:** Documentation discusses `X-Amz-Durable-Execution-Arn` as an HTTP header; developer tries to capture it from `curl` or looks for it in the wrong place.
**Why it happens:** The HTTP API spec shows it as a header, but the AWS CLI surfaces it as a top-level JSON field in the `aws lambda invoke` stdout metadata (alongside `StatusCode`, `ExecutedVersion`, `FunctionError`).
**How to avoid:** Parse `aws lambda invoke` stdout JSON with `jq -r '.DurableExecutionArn'`. Never use `curl` for Lambda invocation in this harness.
**Warning signs:** Getting empty string from `.DurableExecutionArn` but no error from invoke.

### Pitfall 2: Callback ID Not in get-durable-execution

**What goes wrong:** Developer polls `get-durable-execution` looking for a `CallbackId` field — it is not there.
**Why it happens:** `GetDurableExecution` returns execution-level metadata only (Status, Result, Error). Operation-level details including callback IDs live in `GetDurableExecutionHistory`.
**How to avoid:** Always use `get-durable-execution-history` with `--query 'Events[?EventType==\`CallbackStarted\`].CallbackStartedDetails.CallbackId | [0]'` to extract the callback ID.
**Warning signs:** Null or empty callback_id despite successful async invocation.

### Pitfall 3: Waits Handler Has 60-Second Hardcoded Duration

**What goes wrong:** Wait tests time out or take much longer than expected.
**Why it happens:** All four `waits.rs` handlers hardcode `ctx.wait("cooling_period", 60)` — 60 seconds. There is no short-duration test variant deployed.
**How to avoid:** Set the `wait_for_terminal_status` timeout to at least 120 seconds for wait tests. Accept the 60-second wait cost for Phase 15. Consider this when designing the Phase 15 test plan.
**Warning signs:** Polling loop exits with TIMEOUT at 120s; confirmed by checking the wait duration in the history API.

### Pitfall 4: Invoke Handler Requires Event Payload with order_id

**What goes wrong:** invoke handler returns `{"order_id": "unknown", ...}` instead of the test-supplied order ID.
**Why it happens:** `invoke.rs` reads `event["order_id"]` from the Lambda event payload. If payload is `{}` or `null`, it falls back to `"unknown"`.
**How to avoid:** Always pass `--payload '{"order_id":"test-123"}'` when testing invoke handlers.
**Warning signs:** Response contains `"order_id": "unknown"`.

### Pitfall 5: list-durable-executions-by-function Needs Qualifier

**What goes wrong:** `list-durable-executions-by-function` returns empty list even though executions exist.
**Why it happens:** The `--qualifier` parameter defaults to `$LATEST` if omitted. If the function was invoked via `:live` alias, the execution is tracked under the alias qualifier, not `$LATEST`.
**How to avoid:** When listing executions by function, use the full qualified ARN (`:live` alias) as the `--function-name`, or pass `--qualifier live` explicitly. Alternatively, use the `DurableExecutionArn` from the invoke response directly with `get-durable-execution` — it bypasses qualifier ambiguity entirely.
**Warning signs:** Empty `DurableExecutions` list after confirmed successful invocation.

### Pitfall 6: ADFS Credential Expiry Mid-Run

**What goes wrong:** Test run starts successfully but fails partway through with `ExpiredTokenException`.
**Why it happens:** ADFS sessions last 1-4 hours; a full test run with wait operations (60s each × 4 styles = 4+ minutes) can push against the session limit if started near expiry.
**How to avoid:** Run credential check at harness startup (TEST-04). Document ADFS refresh command in script header comment.
**Warning signs:** Sudden AWS API errors mid-run after earlier tests passed.

### Pitfall 7: set -e Breaks Test Collection

**What goes wrong:** The first failing test causes the whole harness to exit, skipping remaining tests.
**Why it happens:** `set -euo pipefail` causes any command exit non-zero to abort the script. Test functions that return non-zero trigger this.
**How to avoid:** Wrap each test invocation in the `run_test` function using a subshell: `if error_msg=$("$test_fn" 2>&1); then ...`. The subshell failure is captured by the `if` conditional and does not trigger `set -e`.
**Warning signs:** Test count shown in summary is less than expected; only first failure is shown.

## Code Examples

### Complete Polling Loop for Callback Test

```bash
# Source: AWS CLI docs — get-durable-execution-history CallbackStartedDetails
test_closure_callbacks() {
  local fn_arn
  fn_arn=$(get_alias_arn "closure-callbacks")
  local exec_name
  exec_name=$(make_exec_name "closure-callbacks")

  # Step 1: Invoke asynchronously
  local exec_arn
  exec_arn=$(aws lambda invoke \
    --profile "$PROFILE" \
    --region "$REGION" \
    --function-name "$fn_arn" \
    --invocation-type Event \
    --cli-binary-format raw-in-base64-out \
    --durable-execution-name "$exec_name" \
    --payload '{}' \
    /dev/null 2>&1 | jq -r '.DurableExecutionArn // empty')

  [[ -n "$exec_arn" ]] || { echo "No DurableExecutionArn returned"; return 1; }

  # Step 2: Poll history until CallbackStarted event appears
  local callback_id
  callback_id=$(extract_callback_id "$exec_arn" 60)
  [[ -n "$callback_id" ]] || { echo "CallbackId not found in history"; return 1; }

  # Step 3: Send callback success
  send_callback_success "$callback_id" '{"approved":true}'

  # Step 4: Poll until SUCCEEDED
  local status
  status=$(wait_for_terminal_status "$fn_arn" "$exec_arn" 120)
  [[ "$status" == "SUCCEEDED" ]] || { echo "Expected SUCCEEDED, got $status"; return 1; }
}
```

### Synchronous Test Function (Step Operations)

```bash
# Source: aws lambda invoke output shape (DurableExecutionArn, FunctionError)
test_closure_basic_steps() {
  local fn_arn
  fn_arn=$(get_alias_arn "closure-basic-steps")

  local out_file
  out_file=$(mktemp /tmp/dr-test-XXXXXX.json)
  local meta
  meta=$(aws lambda invoke \
    --profile "$PROFILE" \
    --region "$REGION" \
    --function-name "$fn_arn" \
    --cli-binary-format raw-in-base64-out \
    --payload '{"order_id":"test-123"}' \
    "$out_file" 2>&1)
  local fn_error
  fn_error=$(echo "$meta" | jq -r '.FunctionError // empty')
  local status_code
  status_code=$(echo "$meta" | jq -r '.StatusCode // 0')
  rm -f "$out_file"

  [[ "$status_code" == "200" && -z "$fn_error" ]] || {
    echo "Status=$status_code FunctionError=${fn_error:-none}"
    return 1
  }
}
```

### Harness Entry Point (TEST-01, TEST-06)

```bash
# Full harness skeleton
PROFILE="adfs"
REGION="us-east-2"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TF_DIR="$(cd "$SCRIPT_DIR/.." && pwd)/infra"
TEST_RUN_ID=$(date +%Y%m%d%H%M%S)
PASS_COUNT=0
FAIL_COUNT=0
declare -a FAILURES=()

main() {
  check_credentials      # TEST-04: fail fast on expired ADFS
  load_tf_outputs        # Read alias_arns from terraform output

  if [[ -n "${1:-}" ]]; then
    # TEST-06: run single named test
    run_test "$1" "$1"
  else
    # TEST-01: run all tests
    run_all_tests
  fi

  print_results
}

main "$@"
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Poll `get-durable-execution` for callback status | Use `get-durable-execution-history` events | `get-durable-execution` has no WAITING_FOR_CALLBACK status; history events are the correct source |
| Look for `DurableExecutionArn` in HTTP response header | Parse it from `aws lambda invoke` stdout JSON | CLI surfaces it as a top-level metadata field; no need for curl or header parsing |
| `sleep` before sending callback | Poll history for `CallbackStarted` event | Race-condition-free; eliminates non-deterministic test failures |

**Deprecated/outdated from prior research:**
- FEATURES.md stated "poll GetDurableExecution until WAITING_FOR_CALLBACK status" — `WAITING_FOR_CALLBACK` is NOT a valid Status enum value for `get-durable-execution`. Use `get-durable-execution-history` Events instead.
- SUMMARY.md concern about "exact JSON field paths for callback_id location" — now fully resolved: `Events[?EventType==\`CallbackStarted\`].CallbackStartedDetails.CallbackId`.

## Open Questions

1. **Does `aws lambda invoke` with `--invocation-type Event` reliably return `DurableExecutionArn` in CLI stdout?**
   - What we know: The CLI docs list it as a top-level output field for both invocation types.
   - What's unclear: Whether async invocations (Event type) populate it as quickly as sync, or whether there's a small delay before the ARN is available.
   - Recommendation: Capture it from the invoke response and add a non-null assertion. If empty, fall back to `list-durable-executions-by-function` filtered by `--durable-execution-name`.

2. **Do all four styles' waits handlers behave identically?**
   - What we know: All four `waits.rs` files hardcode `ctx.wait("cooling_period", 60)`.
   - What's unclear: Whether the test for Phase 15 should accept 60s or whether a short-duration variant should be built.
   - Recommendation: Phase 13 (harness) should set wait test timeout to 120s and document the 60s wait. Phase 15 (async tests) can decide whether to build a 5s variant.

3. **Does `list-durable-executions-by-function` require the exact alias ARN or just function name?**
   - What we know: The CLI docs say `--function-name` can be name, ARN, or partial ARN. `--qualifier` controls which version/alias.
   - What's unclear: Whether passing the full qualified alias ARN (`arn:...:live`) automatically applies the right qualifier.
   - Recommendation: Use `DurableExecutionArn` from invoke response directly with `get-durable-execution` for status polling — avoids the qualifier ambiguity entirely.

## Validation Architecture

> nyquist_validation key is absent from .planning/config.json — treating as enabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Manual bash execution + AWS CLI (no automated test runner for integration tests) |
| Config file | none — integration tests require live AWS |
| Quick run command | `bash scripts/test-all.sh closure-basic-steps` (single test) |
| Full suite command | `bash scripts/test-all.sh` (all tests) |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TEST-01 | test-all.sh runs all 44 tests and exits 0 on all pass | smoke | `bash scripts/test-all.sh` | No — Wave 0 |
| TEST-02 | wait_for_terminal_status returns SUCCEEDED for a basic-steps invoke | integration | `bash scripts/test-all.sh closure-basic-steps` | No — Wave 0 |
| TEST-03 | extract_callback_id returns a non-empty string from callback history | integration | `bash scripts/test-all.sh closure-callbacks` | No — Wave 0 |
| TEST-04 | Harness exits 1 with credential error message when ADFS expired | manual | manual (requires expired creds) | No — Wave 0 |
| TEST-05 | Output shows test name, PASS/FAIL, and failure reason when a test fails | smoke | `bash scripts/test-all.sh` with a deliberate bad ARN | No — Wave 0 |
| TEST-06 | Single test runnable via CLI arg | smoke | `bash scripts/test-all.sh closure-basic-steps` | No — Wave 0 |

### Sampling Rate

- **Per task commit:** `bash scripts/test-all.sh closure-basic-steps` (credential smoke test)
- **Per wave merge:** `bash scripts/test-all.sh` (full harness with all test stubs)
- **Phase gate:** All helper functions call correctly against deployed infrastructure before Phase 14 begins

### Wave 0 Gaps

- [ ] `scripts/test-all.sh` — the entire deliverable of this phase
- [ ] Test stubs (empty test functions that PASS) to validate harness framework before Phase 14 fills them in

*(No existing test infrastructure to extend — this is greenfield integration tooling)*

## Sources

### Primary (HIGH confidence)

- [AWS CLI docs — aws lambda invoke output shape](https://docs.aws.amazon.com/cli/latest/reference/lambda/invoke.html) — confirmed `DurableExecutionArn` as top-level JSON output field; confirmed `--durable-execution-name` flag
- [AWS API docs — GetDurableExecutionHistory response](https://docs.aws.amazon.com/lambda/latest/api/API_GetDurableExecutionHistory.html) — confirmed `Events[].CallbackStartedDetails.CallbackId` field; confirmed event type enum including `CallbackStarted`
- [AWS API docs — GetDurableExecution response](https://docs.aws.amazon.com/lambda/latest/api/API_GetDurableExecution.html) — confirmed Status enum: RUNNING, SUCCEEDED, FAILED, TIMED_OUT, STOPPED (no WAITING_FOR_CALLBACK)
- [AWS CLI docs — list-durable-executions-by-function](https://docs.aws.amazon.com/cli/latest/reference/lambda/list-durable-executions-by-function.html) — confirmed filter options and response shape
- [AWS CLI docs — send-durable-execution-callback-success](https://docs.aws.amazon.com/cli/latest/reference/lambda/send-durable-execution-callback-success.html) — confirmed `--callback-id` and `--result` parameters
- [AWS API docs — CallbackDetails](https://docs.aws.amazon.com/lambda/latest/api/API_CallbackDetails.html) — confirmed CallbackId field format (base64-encoded)
- Direct codebase inspection: `examples/closure-style/src/callbacks.rs`, `invoke.rs`, `waits.rs`, `basic_steps.rs`
- Direct codebase inspection: `infra/outputs.tf`, `infra/lambda.tf`, `scripts/verify-prerequisites.sh`, `scripts/build-images.sh`
- Phase 11 context: `11-CONTEXT.md`, `11-01-SUMMARY.md` — confirmed alias_arns output structure, suffix c351

### Secondary (MEDIUM confidence)

- [AWS API docs — Lambda Invoke HTTP headers](https://docs.aws.amazon.com/lambda/latest/api/API_Invoke.html) — `X-Amz-Durable-Execution-Arn` listed as HTTP response header; CLI surfacing confirmed separately
- [AWS examples docs — Durable functions](https://docs.aws.amazon.com/lambda/latest/dg/durable-examples.html) — callback pattern shows callbackId passed to external system from context
- DEV.to article — callback_id visible in Lambda console under "Durable Execution Details" tab (guidance for manual testing)

### Tertiary (LOW confidence)

- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — existing tools, no new dependencies
- Architecture: HIGH — patterns derived from existing scripts + verified API docs
- API field names for callback extraction: HIGH — confirmed from official GetDurableExecutionHistory API docs
- DurableExecutionArn from invoke: HIGH — confirmed from CLI docs output shape
- Pitfalls: HIGH — all verified against official API docs and existing codebase

**Research date:** 2026-03-17
**Valid until:** 2026-07-17 (90 days — stable AWS CLI API, unlikely to change)
