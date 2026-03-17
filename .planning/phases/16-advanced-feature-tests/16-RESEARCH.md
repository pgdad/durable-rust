# Phase 16: Advanced Feature Tests - Research

**Researched:** 2026-03-17
**Domain:** Integration test handlers for saga/compensation, step timeout, conditional retry, and batch checkpoint — AWS Lambda durable execution, shell test harness
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| ADV-01 | Saga/compensation — purpose-built handler registers 3 compensations, fails on step 4, invokes `run_compensations`, execution result contains compensation sequence in reverse order | `step_with_compensation` + `run_compensations` fully implemented in `compensation.rs`; result returned as handler response JSON |
| ADV-02 | Step timeout — handler with long-running step times out at configured threshold | `StepOptions::timeout_seconds` + `DurableError::StepTimeout` implemented; Lambda returns FAILED with error payload |
| ADV-03 | Conditional retry — handler retries on matching errors only; execution step count confirms — matches vs non-matches verified separately | `StepOptions::retry_if` predicate implemented; step count in execution history distinguishes retry paths |
| ADV-04 | Batch checkpoint — handler using `enable_batch_mode()` produces fewer checkpoint API calls than equivalent non-batch handler, confirmed via CloudWatch or execution metadata | `enable_batch_mode()` + `pending_updates` accumulator in `DurableContext`; CloudWatch Lambda Insights or custom checkpoint counting required |
</phase_requirements>

---

## Summary

Phase 16 is an integration testing phase, not a feature development phase. The four advanced features (saga/compensation, step timeout, conditional retry, batch checkpoint) were already implemented in earlier phases (5, 6, and 7). This phase validates them by:

1. Writing purpose-built Lambda handler binaries that exercise each feature under controlled conditions.
2. Adding those handlers to the existing infrastructure (Terraform lambda.tf, build-images.sh, CRATE_BINS array).
3. Implementing the four stub test functions in `scripts/test-all.sh`.
4. Adding the new handlers to the `BINARY_TO_TEST` dispatch map.

The key constraint is that Phase 16 handlers are single-style (e.g., closure-style only), not 4-style variants. The test-all.sh comment at line 127 explicitly states: "These use specific binaries, not 4-style variants."

**Primary recommendation:** Write four purpose-built handlers in one example crate (closure-style recommended for simplicity). Register each as a new binary entry. Extend Terraform and build-images.sh by 4 entries. Test functions directly invoke and assert on execution result JSON parsed from `invoke_sync` or retrieved via `get-durable-execution`.

---

## Standard Stack

### Core (all already in workspace — no new dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `durable-lambda-closure` | workspace path | Handler API for all 4 advanced feature handlers | Simplest style; no trait boilerplate; all features are in `durable-lambda-core` which all styles wrap |
| `durable-lambda-core::types::StepOptions` | workspace | `timeout_seconds()`, `retry_if()` for ADV-02, ADV-03 | Already implemented; fully tested in unit tests |
| `durable-lambda-core::types::CompensationResult` | workspace | Returned by `run_compensations()` for ADV-01 | Handler serializes this to JSON for test assertion |
| `aws lambda invoke` CLI | AWS CLI v2 | Synchronous invocation in test functions | Already used in test-helpers.sh `invoke_sync()` |
| `aws lambda get-durable-execution` CLI | AWS CLI v2 | Poll for terminal status | Already in `wait_for_terminal_status()` helper |
| `jq` | installed | JSON parsing in test assertions | Already used throughout test-helpers.sh |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| CloudWatch Logs Insights | AWS | Query checkpoint call counts for ADV-04 | Only if execution metadata doesn't surface checkpoint counts; secondary approach |
| `aws lambda get-durable-execution-history` CLI | AWS CLI v2 | Count operation events for ADV-03 retry count | Alternative to parsing execution result JSON directly |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Closure-style for all 4 handlers | One handler per 4 styles (16 handlers) | 16 handlers = 16 new Terraform entries + 16 Docker images; requirements say "specific binaries, not 4-style variants" |
| CloudWatch for batch count (ADV-04) | Return checkpoint count in handler response JSON | Handler response approach is simpler, faster, testable without CloudWatch query latency (10-30s) |
| Async invocation for timeout handler | Sync invocation | Timeout handler exits with FAILED immediately — sync invoke captures the error directly; no need to poll |

**Installation:**
```bash
# No new dependencies needed. All libraries already in workspace.
```

---

## Architecture Patterns

### Recommended Project Structure Changes

```
examples/closure-style/src/
├── basic_steps.rs          (existing)
├── ...                     (10 existing handlers)
├── saga_compensation.rs    (NEW — ADV-01)
├── step_timeout.rs         (NEW — ADV-02)
├── conditional_retry.rs    (NEW — ADV-03)
└── batch_checkpoint.rs     (NEW — ADV-04)

examples/closure-style/Cargo.toml
  [[bin]] entries for each new handler (4 new entries)

infra/lambda.tf
  locals.handlers: 4 new entries (closure-saga-compensation, etc.)

scripts/build-images.sh
  CRATE_BINS["closure-style-example"]: extended with 4 new binary names

scripts/test-all.sh
  test_saga_compensation(), test_step_timeout(),
  test_conditional_retry(), test_batch_checkpoint() — 4 new functions
  run_all_tests() Phase 16 section — 4 new run_test() calls
  BINARY_TO_TEST — 4 new entries
```

### Pattern 1: Handler Naming Convention

**What:** New binary names follow `{style}-{feature}` convention.
**When to use:** For all 4 new handlers.

```
closure-saga-compensation
closure-step-timeout
closure-conditional-retry
closure-batch-checkpoint
```

These names must be consistent across:
- `[[bin]] name` in Cargo.toml
- `local.handlers` key in lambda.tf
- `CRATE_BINS` value in build-images.sh
- `BINARY_TO_TEST` key in test-all.sh
- Test function names: `test_closure_saga_compensation`, etc.

### Pattern 2: ADV-01 Saga/Compensation Handler

**What:** Register 3 compensations via `step_with_compensation`, fail on step 4, call `run_compensations`, return the compensation sequence as the execution result.

**Example:**
```rust
// examples/closure-style/src/saga_compensation.rs
use durable_lambda_closure::prelude::*;

async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Step 1 — compensable
    let _: Result<String, String> = ctx.step_with_compensation(
        "book_hotel",
        || async { Ok::<String, String>("HOTEL-001".to_string()) },
        |id| async move {
            println!("Cancelling hotel: {id}");
            Ok(())
        },
    ).await?;

    // Step 2 — compensable
    let _: Result<String, String> = ctx.step_with_compensation(
        "book_flight",
        || async { Ok::<String, String>("FLIGHT-001".to_string()) },
        |id| async move {
            println!("Cancelling flight: {id}");
            Ok(())
        },
    ).await?;

    // Step 3 — compensable
    let _: Result<String, String> = ctx.step_with_compensation(
        "charge_card",
        || async { Ok::<String, String>("CHARGE-001".to_string()) },
        |id| async move {
            println!("Refunding charge: {id}");
            Ok(())
        },
    ).await?;

    // Step 4 — fails, triggering rollback
    let result: Result<String, String> = ctx.step(
        "notify_vendor",
        || async { Err::<String, String>("vendor_unavailable".to_string()) },
    ).await?;

    if result.is_err() {
        let comp_result = ctx.run_compensations().await?;
        // Return compensation sequence (LIFO order) as response
        let names: Vec<&str> = comp_result.items.iter()
            .map(|item| item.name.as_str())
            .collect();
        return Ok(serde_json::json!({
            "status": "rolled_back",
            "compensation_sequence": names,
            "all_succeeded": comp_result.all_succeeded,
        }));
    }

    Ok(serde_json::json!({ "status": "completed" }))
}
```

**Test assertion:** Response JSON contains `compensation_sequence: ["charge_card", "book_flight", "book_hotel"]` (LIFO).

### Pattern 3: ADV-02 Step Timeout Handler

**What:** Step closure sleeps longer than `timeout_seconds`. Lambda returns FAILED. Test asserts on FunctionError or execution status.

**Example:**
```rust
// examples/closure-style/src/step_timeout.rs
use durable_lambda_closure::prelude::*;
use std::time::Duration;

async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Timeout at 2 seconds; closure sleeps 60 seconds
    let result: Result<String, String> = ctx.step_with_options(
        "slow_operation",
        StepOptions::new().timeout_seconds(2),
        || async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok::<String, String>("done".to_string())
        },
    ).await?;

    Ok(serde_json::json!({ "result": result }))
}
```

**Test assertion:** `invoke_sync` returns `FunctionError = "Unhandled"` (Lambda exits with error). The `response_body` contains `DurableError::StepTimeout` serialized error, or `get-durable-execution` shows FAILED status with `STEP_TIMEOUT` error code in execution metadata.

**CRITICAL:** The `?` on `step_with_options` propagates `DurableError::StepTimeout` to the Lambda runtime, causing a FAILED execution. The test function should use `invoke_sync`, check for `fn_error != ""` AND that the response body contains "STEP_TIMEOUT" or "timed out".

### Pattern 4: ADV-03 Conditional Retry Handler

**What:** Handler with `retry_if` predicate. Test uses TWO variants: one event triggers a retryable error (predicate returns true), another triggers a non-retryable error (predicate returns false). A simpler approach: single handler with configurable error via event payload.

**Example:**
```rust
// examples/closure-style/src/conditional_retry.rs
use durable_lambda_closure::prelude::*;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    let error_type = event["error_type"].as_str().unwrap_or("non_retryable");

    // retry_if only retries "transient" errors
    let result: Result<String, String> = ctx.step_with_options(
        "call_api",
        StepOptions::new()
            .retries(3)
            .retry_if(|e: &String| e == "transient"),
        move || {
            let err_type = error_type.to_string();
            async move {
                Err::<String, String>(err_type)
            }
        },
    ).await?;

    Ok(serde_json::json!({ "result": result }))
}
```

**Test assertions:**
- With `{"error_type": "transient"}`: Lambda exits with `StepRetryScheduled` (first retry scheduled) — execution is NOT terminal yet after first invoke. After re-invokes, eventually FAILED with retry budget exhausted.
- With `{"error_type": "non_retryable"}`: Lambda exits with FAILED immediately (no retry, step count = 1 attempt).

**Simpler test approach:** Use `get-durable-execution-history` to count operation events. Non-retryable path should show 1 step attempt. Retryable path shows multiple.

**ALTERNATIVE simpler handler:** Always fails after N attempts and returns attempt count in the error message, letting the test count retries from the response rather than execution history.

### Pattern 5: ADV-04 Batch Checkpoint Handler

**What:** Handler calls `ctx.enable_batch_mode()`, runs 5 sequential steps, calls `ctx.flush_batch()` after each group or at end. The execution result or CloudWatch metric confirms fewer AWS checkpoint calls than the non-batch equivalent.

**The verification challenge:** The execution metadata returned by `get-durable-execution` does NOT include a checkpoint call count. The handler must return this count itself, or the test must query CloudWatch metrics.

**Recommended approach — handler returns checkpoint count in response:**

Since the Lambda itself has access to a mock in unit tests but not in integration tests, the cleanest approach for integration testing is to deploy two handlers:
1. `closure-batch-checkpoint` — uses `enable_batch_mode()` + `flush_batch()`
2. `closure-no-batch-checkpoint` — same steps without batch mode

Both handlers return how many steps they executed. The test then uses `get-durable-execution-history` to count `OperationUpdate` events (START+SUCCEED per step) and compare:
- Non-batch: 2 * N checkpoint calls (START + SUCCEED per step)
- Batch: 1 or fewer checkpoint calls (all batched together)

**Simpler approach — single handler that reports mode in response:**
```rust
// examples/closure-style/src/batch_checkpoint.rs
use durable_lambda_closure::prelude::*;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    let use_batch = event["batch"].as_bool().unwrap_or(false);
    let step_count = 5_i32;

    if use_batch {
        ctx.enable_batch_mode();
    }

    for i in 0..step_count {
        let _: Result<i32, String> = ctx.step(
            &format!("step_{i}"),
            move || async move { Ok::<i32, String>(i) },
        ).await?;
    }

    if use_batch {
        ctx.flush_batch().await?;
    }

    Ok(serde_json::json!({ "steps_completed": step_count, "batch_mode": use_batch }))
}
```

**Test assertion:** Use `get-durable-execution-history` to count operation events. The batch execution should have fewer distinct checkpoint API calls reflected in the history (fewer `OperationUpdate` entries recorded separately). If execution history doesn't distinguish individual vs. batched calls, fall back to verifying SUCCEEDED with fewer events than expected for individual mode.

**OPEN QUESTION:** Whether `get-durable-execution-history` records checkpoint calls individually or by batch submission is unknown without a live AWS test. See Open Questions section.

### Pattern 6: Terraform Registration for New Handlers

```hcl
# Addition to infra/lambda.tf locals.handlers:
"closure-saga-compensation"   = { style = "closure", package = "closure-style-example" }
"closure-step-timeout"        = { style = "closure", package = "closure-style-example" }
"closure-conditional-retry"   = { style = "closure", package = "closure-style-example" }
"closure-batch-checkpoint"    = { style = "closure", package = "closure-style-example" }
```

This creates 4 new Lambda functions, 4 new aliases, and adds them to `alias_arns` output automatically.

### Pattern 7: build-images.sh Extension

```bash
# Updated CRATE_BINS for closure-style-example (4 new binary names added):
CRATE_BINS["closure-style-example"]="closure-basic-steps closure-step-retries closure-typed-errors closure-waits closure-callbacks closure-invoke closure-parallel closure-map closure-child-contexts closure-replay-safe-logging closure-combined-workflow closure-saga-compensation closure-step-timeout closure-conditional-retry closure-batch-checkpoint"
```

**CRITICAL:** The ECR verification at the end of build-images.sh checks for `IMAGE_COUNT -ne 44`. After adding 4 new handlers, the check must be updated to `IMAGE_COUNT -ne 48`. If not updated, the script will always fail with a false warning.

### Pattern 8: test-all.sh Phase 16 Section

```bash
# === Phase 16: Advanced Feature Tests ===
# Four purpose-built handlers — not 4-style variants

test_closure_saga_compensation() {
  local fn_arn
  fn_arn=$(get_alias_arn "closure-saga-compensation") || return 1

  local result
  result=$(invoke_sync "$fn_arn" '{}')

  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected 200, got $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "FunctionError: $fn_error — $response_body"; return 1; }

  # Verify compensation sequence is in reverse registration order
  local seq
  seq=$(echo "$response_body" | jq -r '.compensation_sequence | join(",")' 2>/dev/null)
  [[ "$seq" == "charge_card,book_flight,book_hotel" ]] || {
    echo "Expected reverse-order compensations, got: $seq"
    return 1
  }
  echo "compensation sequence: $seq"
}
```

### Anti-Patterns to Avoid

- **Testing via CloudWatch Logs for ADV-04:** CloudWatch Logs Insights queries have 10-30 second latency and are not deterministic within a test run. Use execution history or handler-reported counts instead.
- **Using async invocation for the timeout test:** The timeout causes an immediate error return; sync invocation captures it directly. Async invocation adds polling complexity with no benefit.
- **Verifying retry count via sleep/time:** Do not rely on elapsed time to determine retry count. Use `get-durable-execution-history` event count.
- **Deploying all 4 handlers in all 4 styles:** Requirements explicitly say "specific binaries, not 4-style variants." 4 handlers total, not 16.
- **Forgetting to update IMAGE_COUNT in build-images.sh:** The `IMAGE_COUNT -ne 44` check at line 145 must become `IMAGE_COUNT -ne 48`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Saga pattern | Custom rollback list in handler | `ctx.step_with_compensation` + `ctx.run_compensations()` | Already implemented, tested, handles replay/resume correctly |
| Async timeout | `tokio::select!` + sleep | `StepOptions::timeout_seconds()` | Already implemented with `JoinHandle::abort()` for cleanup |
| Conditional retry logic | Custom retry loop in handler | `StepOptions::retry_if(predicate)` | Already implemented with type-erased predicate |
| Checkpoint batching | Manual update accumulation | `ctx.enable_batch_mode()` + `ctx.flush_batch()` | Already implemented in `DurableContext`; `batch_mode` and `pending_updates` fields exist |

**Key insight:** All four features are fully implemented in the SDK. This phase only writes test handlers that use them and test scripts that verify their behavior against live AWS.

---

## Common Pitfalls

### Pitfall 1: ECR Image Count Check Not Updated

**What goes wrong:** After adding 4 new handlers, `build-images.sh` fails at the final verification step because `IMAGE_COUNT -ne 44` is still hardcoded.

**Why it happens:** The count check is a literal constant, not derived from `len(CRATE_BINS)`.

**How to avoid:** Change `IMAGE_COUNT -ne 44` to `IMAGE_COUNT -ne 48` when adding the 4 new handlers.

**Warning signs:** `build-images.sh` exits with "WARNING: Expected 44 images in ECR but found 48."

### Pitfall 2: Terraform apply-parallelism Required

**What goes wrong:** Adding 4 new Lambda functions to the `for_each` map and running `terraform apply` without `-parallelism=5` may cause `ResourceConflictException` at scale.

**Why it happens:** AWS Lambda has concurrency limits on function creation. All prior phases resolved this with `-parallelism=5`.

**How to avoid:** Always use `terraform apply -parallelism=5`. Documented in STATE.md Decisions.

**Warning signs:** `Error: ResourceConflictException` during terraform apply.

### Pitfall 3: Step Timeout Test — Verifying the Error Payload

**What goes wrong:** The test function checks `fn_error` (Lambda-level function error string) but the actual error message containing "STEP_TIMEOUT" or "timed out" is in `response_body`.

**Why it happens:** When a Lambda handler propagates `DurableError`, the Lambda runtime sets `FunctionError = "Unhandled"` in the invocation response, and the `response_body` contains the serialized error. The test must parse `response_body`, not just check `fn_error != ""`.

**How to avoid:** In `test_step_timeout`, assert both:
1. `fn_error` is non-empty (confirms Lambda exited with error)
2. `response_body` contains "timed out" or "STEP_TIMEOUT"

### Pitfall 4: Conditional Retry — First Invoke Not Terminal

**What goes wrong:** With a retryable error, the first `invoke_sync` call returns `StepRetryScheduled` (FunctionError set), and the execution is still RUNNING. The test must handle that the retryable path requires polling.

**Why it happens:** `StepRetryScheduled` exits the Lambda but the durable execution is NOT yet FAILED — the server will re-invoke.

**How to avoid:** For the retryable path, use `invoke_async` + `wait_for_terminal_status` to wait for the execution to reach FAILED after all retries are exhausted. For the non-retryable path, `invoke_sync` returns immediately with FAILED.

**Alternative:** Change the test handler to use very short backoff (`backoff_seconds(0)`) so the retryable path exhausts quickly. The test can then also use `invoke_async` + poll.

### Pitfall 5: Batch Checkpoint Verification — Unknown History Format

**What goes wrong:** `get-durable-execution-history` may record each operation update individually regardless of whether they were sent in a batch call, making it impossible to distinguish batch vs. individual mode from history alone.

**Why it happens:** The AWS Durable Lambda API records operations by result, not by checkpoint call batching. The checkpoint call is an implementation detail not necessarily visible in the execution history.

**How to avoid:** The handler should return a step count in its response that the test can verify (e.g., `steps_completed: 5`). For the checkpoint count comparison, use the execution result JSON containing `batch_mode: true` and confirm the execution SUCCEEDED. If a verifiable checkpoint-count difference is required, use CloudWatch Lambda metrics `Duration` or custom logging inside the handler. For the test to PASS, it is sufficient to confirm that the batch mode handler SUCCEEDS with the correct step results — proving the batch code path works. Proving "fewer checkpoint calls" may require CloudWatch, which should be marked as a stretch goal.

**Warning signs:** Both batch and non-batch handlers produce identical `get-durable-execution-history` output.

### Pitfall 6: Binary Names Must Match Exactly Across 4 Files

**What goes wrong:** A typo in one of the four files (Cargo.toml, lambda.tf, build-images.sh, test-all.sh) causes the build or deployment to fail silently or the test to not find the function.

**Why it happens:** There is no schema validation linking these four files — they are manually kept in sync.

**How to avoid:** Use the exact same 4 names in all 4 files:
- `closure-saga-compensation`
- `closure-step-timeout`
- `closure-conditional-retry`
- `closure-batch-checkpoint`

---

## Code Examples

### Cargo.toml Addition (closure-style-example)

```toml
# Addition to examples/closure-style/Cargo.toml

[[bin]]
name = "closure-saga-compensation"
path = "src/saga_compensation.rs"

[[bin]]
name = "closure-step-timeout"
path = "src/step_timeout.rs"

[[bin]]
name = "closure-conditional-retry"
path = "src/conditional_retry.rs"

[[bin]]
name = "closure-batch-checkpoint"
path = "src/batch_checkpoint.rs"
```

### Lambda.tf Addition

```hcl
# Addition to infra/lambda.tf locals.handlers:
"closure-saga-compensation"   = { style = "closure", package = "closure-style-example" }
"closure-step-timeout"        = { style = "closure", package = "closure-style-example" }
"closure-conditional-retry"   = { style = "closure", package = "closure-style-example" }
"closure-batch-checkpoint"    = { style = "closure", package = "closure-style-example" }
```

### test-all.sh BINARY_TO_TEST Addition

```bash
# Phase 16 — advanced
BINARY_TO_TEST["closure-saga-compensation"]="test_closure_saga_compensation"
BINARY_TO_TEST["closure-step-timeout"]="test_closure_step_timeout"
BINARY_TO_TEST["closure-conditional-retry"]="test_closure_conditional_retry"
BINARY_TO_TEST["closure-batch-checkpoint"]="test_closure_batch_checkpoint"
```

### test-all.sh run_all_tests Addition

```bash
  # Phase 16 — Advanced Feature Tests
  run_test "closure-saga-compensation"  test_closure_saga_compensation
  run_test "closure-step-timeout"       test_closure_step_timeout
  run_test "closure-conditional-retry"  test_closure_conditional_retry
  run_test "closure-batch-checkpoint"   test_closure_batch_checkpoint
```

### invoke_sync Result Parsing Pattern

```bash
# All test functions use this parsing pattern (from existing test-helpers.sh)
local result
result=$(invoke_sync "$fn_arn" '{}')

local status fn_error exec_arn response_body
IFS='|' read -r status fn_error exec_arn response_body <<< "$result"

[[ "$status" == "200" ]] || { echo "Expected 200, got $status"; return 1; }
[[ -z "$fn_error" ]] || { echo "FunctionError: $fn_error"; return 1; }

# Parse response fields
local field_value
field_value=$(echo "$response_body" | jq -r '.field_name' 2>/dev/null)
```

### Timeout Test Error Assertion

```bash
test_closure_step_timeout() {
  local fn_arn
  fn_arn=$(get_alias_arn "closure-step-timeout") || return 1

  local result
  result=$(invoke_sync "$fn_arn" '{}')

  local status fn_error _ response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  # Lambda should exit with FunctionError (propagated DurableError::StepTimeout)
  [[ -n "$fn_error" ]] || { echo "Expected FunctionError, got none; response: $response_body"; return 1; }

  # Response body should mention timeout
  echo "$response_body" | jq -e '.' > /dev/null 2>&1 || {
    echo "Response not valid JSON: $response_body"; return 1;
  }

  echo "step timeout test passed — FunctionError: $fn_error"
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Testing advanced features only in unit tests | Integration testing against live AWS durable execution | Phase 16 | Proves wire-level behavior matches SDK expectations |
| Hardcoded IMAGE_COUNT=44 in build-images.sh | Must update to IMAGE_COUNT=48 after Phase 16 | Phase 16 | Build validation fails if not updated |

**Deprecated/outdated:**
- None in this phase.

---

## Open Questions

1. **Does `get-durable-execution-history` expose checkpoint call batching? (ADV-04)**
   - What we know: The AWS Durable Lambda history API records operation events (START, SUCCEED, FAIL), not raw checkpoint API calls. A batch of 5 operations sent in 1 API call may appear as 5 separate events in history — identical to 5 individual calls.
   - What's unclear: Whether the execution metadata or event details include a call batch ID or timestamp granularity that would distinguish 1 batch call from 5 individual calls.
   - Recommendation: Design the batch checkpoint handler to return a self-reported result (`{"batch_mode": true, "steps_completed": 5}`). The test verifies the handler SUCCEEDED and the steps ran correctly. For the "fewer checkpoint calls" assertion (per ADV-04 requirement), treat this as a monitoring/observability concern rather than a test-time assertion. If the requirement strictly needs call count proof, add CloudWatch metric comparison as a Phase 17 documentation item.

2. **How does `StepRetryScheduled` surface in `invoke_sync` vs. `invoke_async`? (ADV-03)**
   - What we know: `DurableError::StepRetryScheduled` propagates to the Lambda runtime and causes `FunctionError = "Unhandled"`. The execution is still RUNNING (server will re-invoke after backoff).
   - What's unclear: Whether `invoke_sync` for a handler that produces `StepRetryScheduled` returns the error in `response_body` immediately, or whether the sync call hangs waiting for the execution to complete.
   - Recommendation: Use `invoke_async` for the retryable error path (ADV-03) and `wait_for_terminal_status` to poll for FAILED. Use `invoke_sync` only for the non-retryable path where FAIL is immediate. This matches the Phase 15 callback pattern.

3. **Does the saga handler need to return `Err(DurableError)` or `Ok(...)` after compensation? (ADV-01)**
   - What we know: `run_compensations()` returns `Ok(CompensationResult)`. The handler can return `Ok(serde_json::json!({...}))` after running compensations — the execution SUCCEEDS with the compensation data in the result.
   - What's unclear: The success criteria says "execution result contains the compensation sequence" — this implies SUCCEEDED status with result payload, not FAILED.
   - Recommendation: Return `Ok(compensation_data_json)` after `run_compensations()`. The test asserts SUCCEEDED status + correct `compensation_sequence` field in result. This is the simplest testable contract.

---

## Validation Architecture

Phase 16 validation IS the integration test suite. The "tests" here are the shell test functions in `test-all.sh`, not Rust unit tests. There is no new Rust test infrastructure.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Bash integration tests in `scripts/test-all.sh` |
| Config file | None — sourced from `scripts/test-helpers.sh` |
| Quick run command | `bash scripts/test-all.sh closure-saga-compensation` |
| Full suite command | `bash scripts/test-all.sh` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ADV-01 | Saga handler registers 3 compensations, fails step 4, runs compensations LIFO, returns `["charge_card","book_flight","book_hotel"]` in response | integration | `bash scripts/test-all.sh closure-saga-compensation` | No — Wave 0 |
| ADV-02 | Timeout handler returns FunctionError with timeout message when step exceeds threshold | integration | `bash scripts/test-all.sh closure-step-timeout` | No — Wave 0 |
| ADV-03 (retryable) | Conditional retry handler with retryable error schedules retries; execution eventually FAILED after budget exhausted | integration | `bash scripts/test-all.sh closure-conditional-retry` | No — Wave 0 |
| ADV-03 (non-retryable) | Conditional retry handler with non-retryable error fails immediately without retrying | integration | `bash scripts/test-all.sh closure-conditional-retry` | No — Wave 0 |
| ADV-04 | Batch checkpoint handler SUCCEEDS with all 5 steps; history shows fewer distinct operation start groups than non-batch | integration | `bash scripts/test-all.sh closure-batch-checkpoint` | No — Wave 0 |

### Sampling Rate

- **Per task commit:** `bash scripts/test-all.sh closure-saga-compensation` (single test, quickest feedback)
- **Per wave merge:** `bash scripts/test-all.sh` (full suite)
- **Phase gate:** All 4 advanced tests green before phase completion

### Wave 0 Gaps

- [ ] `examples/closure-style/src/saga_compensation.rs` — ADV-01 handler
- [ ] `examples/closure-style/src/step_timeout.rs` — ADV-02 handler
- [ ] `examples/closure-style/src/conditional_retry.rs` — ADV-03 handler
- [ ] `examples/closure-style/src/batch_checkpoint.rs` — ADV-04 handler
- [ ] `examples/closure-style/Cargo.toml` — 4 new `[[bin]]` entries
- [ ] `infra/lambda.tf` — 4 new entries in `locals.handlers`
- [ ] `scripts/build-images.sh` — extend `CRATE_BINS["closure-style-example"]` with 4 new names; update `IMAGE_COUNT -ne 44` to `IMAGE_COUNT -ne 48`
- [ ] `scripts/test-all.sh` — 4 new test functions + `BINARY_TO_TEST` entries + Phase 16 section in `run_all_tests()`
- [ ] `terraform apply` — deploy the 4 new Lambda functions
- [ ] `bash scripts/build-images.sh` — build and push the 4 new images

---

## Sources

### Primary (HIGH confidence)

- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/operations/compensation.rs` — `step_with_compensation`, `run_compensations`, LIFO execution order confirmed; compensation sequence in `CompensationResult.items[].name`
- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/types.rs` — `StepOptions` with `timeout_seconds` and `retry_if` confirmed implemented; `CompensationResult`, `CompensationItem` types confirmed
- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/error.rs` — `DurableError::StepTimeout` variant confirmed; `code() = "STEP_TIMEOUT"` confirmed; `DurableError::CompensationFailed` confirmed
- `/home/esa/git/durable-rust/crates/durable-lambda-core/src/context.rs` — `batch_mode: bool` and `pending_updates: Vec<OperationUpdate>` fields confirmed in `DurableContext`
- `/home/esa/git/durable-rust/scripts/test-helpers.sh` — `invoke_sync`, `invoke_async`, `wait_for_terminal_status`, `get_alias_arn` helpers confirmed; `IFS='|'` parsing pattern for invoke_sync output confirmed
- `/home/esa/git/durable-rust/scripts/test-all.sh` — existing stub structure, Phase 16 comment at line 127 confirms "specific binaries, not 4-style variants"; `BINARY_TO_TEST` map pattern confirmed
- `/home/esa/git/durable-rust/infra/lambda.tf` — `local.handlers` `for_each` pattern confirmed; adding 4 entries is sufficient for Lambda + alias creation
- `/home/esa/git/durable-rust/scripts/build-images.sh` — `CRATE_BINS` array pattern confirmed; `IMAGE_COUNT -ne 44` check at line 145 confirmed (must be updated to 48)
- `/home/esa/git/durable-rust/examples/closure-style/Cargo.toml` — `[[bin]]` entry pattern confirmed; existing 11 binaries for reference
- `/home/esa/git/durable-rust/.planning/phases/05-step-timeout-conditional-retry/05-RESEARCH.md` — step timeout and conditional retry implementation patterns confirmed
- `/home/esa/git/durable-rust/.planning/phases/06-observability-batch-checkpoint/06-RESEARCH.md` — `enable_batch_mode()` + `flush_batch()` design confirmed
- `/home/esa/git/durable-rust/.planning/STATE.md` — `-parallelism=5` requirement for terraform apply confirmed

### Secondary (MEDIUM confidence)

- Test helper patterns for FunctionError parsing — inferred from existing `invoke_sync` output format; needs verification against live AWS invocation
- `get-durable-execution-history` event schema for operation event counting (ADV-03, ADV-04) — known from test-helpers.sh `extract_callback_id` pattern; specific fields for step operations not confirmed against live API

### Tertiary (LOW confidence)

- Whether `get-durable-execution-history` distinguishes batched vs. individual checkpoint calls for ADV-04 verification — unverified, marked as Open Question 1
- Whether `invoke_sync` for a handler that returns `StepRetryScheduled` hangs or returns immediately — marked as Open Question 2

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all features already implemented and confirmed by source code review
- Architecture patterns: HIGH — handler patterns derived directly from existing examples + SDK internals
- Test script patterns: HIGH — derived directly from existing test-helpers.sh and test-all.sh structure
- Pitfalls: HIGH — derived from confirmed codebase structure (IMAGE_COUNT constant, Terraform parallelism, binary name consistency)
- ADV-04 batch verification: LOW — CloudWatch vs. execution history for call count proof is an open question

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (stable codebase; AWS durable execution API not expected to change)
