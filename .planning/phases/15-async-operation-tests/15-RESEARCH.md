# Phase 15: Async Operation Tests - Research

**Researched:** 2026-03-18
**Domain:** AWS Lambda Durable Execution — async invocation, wait/callback/invoke operations, shell test harness
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Wait handler duration**
- Modify all 4 styles' waits.rs to read wait duration from event payload: `event["wait_seconds"].as_u64().unwrap_or(60)`
- Test sends `{"wait_seconds": 5}` for a ~5-second wait instead of 60 seconds
- Handler change + Docker rebuild + ECR push + Lambda update is part of Phase 15 plan (self-contained, not a prerequisite)
- Validate terminal status SUCCEEDED + response fields: started.status=="started" and completed.status=="completed"

**Invoke test approach**
- Use synchronous invocation (invoke_sync) — proven to work from combined_workflow in Phase 14
- Invoke handler calls order-enrichment-lambda with {"order_id": "test-invoke-001"}
- Validate round-trip: order_id matches sent value, enrichment field is non-null
- No async invocation needed for invoke tests

**Callback test flow**
- Full flow with retries: invoke_async -> extract_callback_id (polls every 3s with timeout) -> send_callback_success({"approved": true}) -> wait_for_terminal_status -> get_execution_output
- Trust existing extract_callback_id() helper — designed for this exact flow
- Validate SUCCEEDED status + outcome.approved==true (proves callback result received and processed)
- Don't assert callback_id in response — it's an internal ID
- Need get_execution_output(exec_arn) helper to retrieve result after async completion

**Shared helper pattern**
- Create assert_waits(binary_name), assert_callbacks(binary_name), assert_invoke(binary_name) in test-helpers.sh
- Each encapsulates the full async flow (multi-step for waits/callbacks, simpler for invoke)
- One-liner callers in test-all.sh — consistent with Phase 14 pattern
- Add get_execution_output(exec_arn) as a new reusable helper in test-helpers.sh for retrieving async execution results

### Claude's Discretion
- Exact error messages in assertion failures
- get_execution_output helper implementation details (JMESPath query for Output field)
- Whether wait tests also verify timing (e.g., execution took >= 5s)
- Order of operations within assert_callbacks flow

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| OPTEST-04 | Wait tests pass — test variant with 5-second wait deployed, invoked async, polled to SUCCEEDED | Handler source code confirmed, waits.rs modification strategy clear, async poll flow already in test-helpers.sh |
| OPTEST-05 | Callback tests pass — all 4 styles' callbacks handlers invoked async, callback signal sent, polled to SUCCEEDED | Callback handler source confirmed (create_callback + callback_result), extract_callback_id/send_callback_success already in test-helpers.sh |
| OPTEST-06 | Invoke tests pass — caller invokes callee stub, returns callee result in response | invoke.rs source confirmed (calls "order-enrichment-lambda"), stub response structure confirmed, invoke_sync proven from combined_workflow |
</phase_requirements>

## Summary

Phase 15 adds 12 async integration tests (waits, callbacks, invoke — 4 API styles each) to the already-working test harness. All 3 helpers (`invoke_async`, `extract_callback_id`, `send_callback_success`, `wait_for_terminal_status`) already exist in `scripts/test-helpers.sh`. The pattern to follow is identical to Phase 14's `assert_*` helpers, extended with multi-step async flows.

The primary complication is that 4 waits.rs handlers hardcode `ctx.wait("cooling_period", 60)` — a 60-second wait that would make tests impractical. These must be modified to read duration from the event payload before deployment. All 4 crate images then need rebuilding, ECR pushing, and Lambda aliasing via Terraform. This is a self-contained task within Phase 15, not a prerequisite.

One key open item from STATE.md is confirmed: the `get-durable-execution` response shape for the `Output` field must be verified against a live execution. The shell helper `get_execution_output(exec_arn)` must query the correct JSON path. The `wait_for_terminal_status` helper already uses `--query 'Status'` so the shape is partially known; `Output` is expected to be a peer of `Status` in the response.

**Primary recommendation:** Implement Phase 15 in two waves: Wave 1 modifies the 4 waits.rs files, rebuilds images, and redeploys; Wave 2 adds the 3 shared assertion helpers and replaces the 12 test stubs.

## Standard Stack

### Core
| Component | Version/Status | Purpose | Why Standard |
|-----------|---------------|---------|--------------|
| `scripts/test-helpers.sh` | Existing — Phase 13/14 | Sourceable shell library with all async helpers | Already proven against live AWS; all async primitives present |
| `scripts/test-all.sh` | Existing — Phase 13/14/16 | Test runner with 12 Phase 15 stubs waiting to be replaced | Framework contract established; just need to fill stubs |
| `scripts/build-images.sh` | Existing — Phase 12 | Builds 48 Docker images from 4 crates, pushes to ECR | Established pipeline with parallelism and verification |
| `aws lambda invoke --invocation-type Event` | AWS CLI v2 | Async Lambda invocation that returns DurableExecutionArn | Proven pattern in invoke_async() |
| `aws lambda get-durable-execution` | AWS CLI v2 | Polls execution status; used in wait_for_terminal_status() | Already in use and working |
| `aws lambda get-durable-execution-history` | AWS CLI v2 | Polls for CallbackStarted event; used in extract_callback_id() | Already in use and working |
| `aws lambda send-durable-execution-callback-success` | AWS CLI v2 | Sends approval signal to suspended callback; used in send_callback_success() | Already in use and working |
| Terraform apply | Existing infra config | Publishes new Lambda versions and updates aliases | Required after image push to promote new binary via `live` alias |

### Reusable Async Primitives (Already in test-helpers.sh)

| Helper | Signature | What It Does |
|--------|-----------|--------------|
| `invoke_async(fn_arn, payload)` | → exec_arn | Invokes with Event type, extracts DurableExecutionArn |
| `wait_for_terminal_status(exec_arn, [timeout_s=120])` | → status string | Polls every 3s until SUCCEEDED/FAILED/TIMED_OUT/STOPPED or timeout |
| `extract_callback_id(exec_arn, [timeout_s=60])` | → callback_id | Polls history for CallbackStarted event every 3s |
| `send_callback_success(callback_id, [result_json])` | → exit code | Sends SendDurableExecutionCallbackSuccess |
| `get_alias_arn(binary_name)` | → ARN string | Looks up from ALIAS_ARNS terraform output |
| `invoke_sync(fn_arn, payload)` | → status|fn_error|exec_arn|body | Synchronous invocation, pipe-delimited output |

### Missing Helper (Must Create)

| Helper | Signature | Implementation Notes |
|--------|-----------|---------------------|
| `get_execution_output(exec_arn)` | → JSON string | `aws lambda get-durable-execution --query 'Output'`; Output field holds the JSON result of a completed execution. See Open Questions #1 for field path uncertainty. |

**Installation:** No new packages needed. All tooling already installed and working.

## Architecture Patterns

### Recommended Structure for Phase 15

The plan follows the exact Phase 14 architecture: 3 new shared helpers in `test-helpers.sh` + 12 one-liner stub replacements in `test-all.sh`.

```
scripts/
├── test-helpers.sh       # Add: get_execution_output, assert_waits, assert_callbacks, assert_invoke
└── test-all.sh           # Replace: 12 stubs → one-liner calls to assert_waits/callbacks/invoke
examples/
├── closure-style/src/waits.rs   # Modify: hardcoded 60 → event["wait_seconds"].as_u64().unwrap_or(60)
├── macro-style/src/waits.rs     # Same modification
├── trait-style/src/waits.rs     # Same modification
└── builder-style/src/waits.rs   # Same modification
```

### Pattern 1: Async Test Helper (Wait/Callback Flow)

**What:** invoke_async → poll for state → get output → assert fields

**When to use:** Any test that requires time-based suspension (waits) or external signal (callbacks)

**Example — assert_waits:**
```bash
# Source: scripts/test-helpers.sh (Phase 14 pattern extended for async)
assert_waits() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")

  local exec_arn
  exec_arn=$(invoke_async "$fn_arn" '{"wait_seconds":5}')
  [[ -n "$exec_arn" ]] || { echo "No exec_arn returned"; return 1; }

  local final_status
  final_status=$(wait_for_terminal_status "$exec_arn" 60)
  [[ "$final_status" == "SUCCEEDED" ]] || \
    { echo "Expected SUCCEEDED, got: $final_status"; return 1; }

  local output
  output=$(get_execution_output "$exec_arn")

  local started_status
  started_status=$(echo "$output" | jq -r '.started.status')
  [[ "$started_status" == "started" ]] || \
    { echo "Expected started.status=started, got: $started_status; output=$output"; return 1; }

  local completed_status
  completed_status=$(echo "$output" | jq -r '.completed.status')
  [[ "$completed_status" == "completed" ]] || \
    { echo "Expected completed.status=completed, got: $completed_status; output=$output"; return 1; }

  echo "wait operation suspended and resumed correctly via $binary"
}
```

**Example — assert_callbacks:**
```bash
# Source: scripts/test-helpers.sh (Phase 14 pattern extended for async)
assert_callbacks() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")

  local exec_arn
  exec_arn=$(invoke_async "$fn_arn" '{}')
  [[ -n "$exec_arn" ]] || { echo "No exec_arn returned"; return 1; }

  local callback_id
  callback_id=$(extract_callback_id "$exec_arn" 60)
  [[ -n "$callback_id" ]] || { echo "No callback_id found in history"; return 1; }

  send_callback_success "$callback_id" '{"approved":true}' || \
    { echo "send_callback_success failed"; return 1; }

  local final_status
  final_status=$(wait_for_terminal_status "$exec_arn" 60)
  [[ "$final_status" == "SUCCEEDED" ]] || \
    { echo "Expected SUCCEEDED after callback, got: $final_status"; return 1; }

  local output
  output=$(get_execution_output "$exec_arn")

  local approved
  approved=$(echo "$output" | jq -r '.outcome.approved')
  [[ "$approved" == "true" ]] || \
    { echo "Expected outcome.approved=true, got: $approved; output=$output"; return 1; }

  echo "callback signal received and processed correctly via $binary"
}
```

**Example — assert_invoke:**
```bash
# Source: scripts/test-helpers.sh (Phase 14 synchronous pattern)
assert_invoke() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{"order_id":"test-invoke-001"}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status; body=$response_body"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }

  local order_id
  order_id=$(echo "$response_body" | jq -r '.order_id')
  [[ "$order_id" == "test-invoke-001" ]] || \
    { echo "Expected order_id=test-invoke-001, got: $order_id; body=$response_body"; return 1; }

  local enrichment
  enrichment=$(echo "$response_body" | jq -r '.enrichment')
  [[ "$enrichment" != "null" ]] || \
    { echo "Expected enrichment non-null; body=$response_body"; return 1; }

  echo "invoke operation called order-enrichment-lambda and returned result via $binary"
}
```

### Pattern 2: Waits Handler Modification (4 Files, Identical Change)

**What:** Read wait duration from event payload instead of hardcoding 60

**When to use:** Before any wait tests can run — a prerequisite step within Phase 15

**Example (closure-style/src/waits.rs):**
```rust
// Before (all 4 styles have this):
ctx.wait("cooling_period", 60).await?;

// After — closure-style:
let wait_secs = _event["wait_seconds"].as_u64().unwrap_or(60);
ctx.wait("cooling_period", wait_secs).await?;
```

The handler parameter must be renamed from `_event` to `event` in styles that prefix with underscore. The macro-style and trait-style also use `_event`; the builder-style uses `_event`. All 4 closures must expose the event parameter to read from it.

**Concrete change per file:**
- `closure-style/src/waits.rs` — rename `_event` to `event`, add `let wait_secs = event["wait_seconds"].as_u64().unwrap_or(60);` before the wait call, change `60` to `wait_secs`
- `macro-style/src/waits.rs` — same (uses `_event` parameter name)
- `trait-style/src/waits.rs` — same (uses `_event` in handle method signature)
- `builder-style/src/waits.rs` — same (uses `_event` in closure parameter)

### Pattern 3: Rebuild and Redeploy After Handler Change

**What:** After modifying waits.rs in all 4 crates, rebuild images and update Lambda aliases

**Steps:**
1. `bash scripts/build-images.sh` — rebuilds all 48 images (waits binaries are 4 of them); cargo-chef caches deps so only recompiles changed source
2. `terraform -chdir=infra apply -parallelism=5` — publishes new versions and updates `live` aliases

**Notes:**
- Only the 4 waits binaries actually changed; all 48 are rebuilt because build-images.sh operates at the crate level (all 11 binaries per crate are rebuilt together). This is by design per Phase 12 decisions.
- Terraform aliasing is required because `get_alias_arn()` resolves to the `live` alias ARN, and the alias must point to the new published version.

### Anti-Patterns to Avoid

- **Polling without timeout:** Both `wait_for_terminal_status` and `extract_callback_id` have timeout parameters. Always pass explicit values suited to the operation (5s wait → 60s timeout sufficient; callbacks need time to enter SUSPENDED state).
- **Sending callback before SUSPENDED:** Do not send the callback signal before `extract_callback_id` returns a callback_id. The execution must be in SUSPENDED state first. `extract_callback_id` handles this by polling the history until the CallbackStarted event appears.
- **Using invoke_sync for callbacks/waits:** These operations suspend execution; a synchronous invocation would hang until the Lambda's wall-clock timeout. Always use `invoke_async` for waits and callbacks.
- **Asserting on Status envelope:** The durable execution service unwraps SUCCEEDED responses before returning to the caller. For synchronous invocations (invoke tests), assert on raw user JSON directly, not on `{"Status":"SUCCEEDED"}`.
- **Asserting callback_id in response:** The callback handler's response includes `callback_id` in the outcome, but per user decision, do not assert on this field — it's an internal ID with no test value.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async execution polling | Custom sleep loop | `wait_for_terminal_status()` in test-helpers.sh | Already implemented, 3s interval, timeout handling, returns status string |
| CallbackStarted event extraction | Custom history query | `extract_callback_id()` in test-helpers.sh | Already implemented, handles polling and null/None guards |
| Callback signal dispatch | Direct AWS CLI invocation inline | `send_callback_success()` in test-helpers.sh | Already implemented with correct CLI parameters |
| Execution output retrieval | Inline `aws lambda get-durable-execution` | `get_execution_output()` — new helper to add | Centralizes the query for reuse across wait and callback tests |
| Lambda ARN resolution | Hardcoded ARNs | `get_alias_arn()` in test-helpers.sh | Reads from Terraform outputs, handles suffix-namespaced names |
| Docker build + ECR push | Manual per-binary docker commands | `scripts/build-images.sh` | Handles ECR login, parallel crate builds, image count verification |

**Key insight:** The test infrastructure is fully built. Phase 15 is additive — modifying 4 handler files and writing 3 assertion helpers plus 12 one-liners.

## Common Pitfalls

### Pitfall 1: Callback Race — Sending Signal Before SUSPENDED State
**What goes wrong:** `send_callback_success` is called before `extract_callback_id` finds a CallbackStarted event; the signal is rejected or lost.
**Why it happens:** The execution hasn't reached the callback suspension point yet when the signal is sent.
**How to avoid:** Always call `extract_callback_id(exec_arn, 60)` first. It polls the history until CallbackStarted appears. Only then call `send_callback_success`.
**Warning signs:** `send_callback_success` exits non-zero, or `wait_for_terminal_status` returns FAILED instead of SUCCEEDED.

### Pitfall 2: Forgetting to Rename `_event` in Handler
**What goes wrong:** Rust compiler error — unused variable prefix `_` means the compiler does not bind the variable, so `event["wait_seconds"]` fails to compile.
**Why it happens:** All 4 waits.rs files use `_event` (underscore prefix) to suppress unused-variable warnings. Once we read from it, the underscore must be removed.
**How to avoid:** In each waits.rs modification, rename the parameter from `_event` to `event`.
**Warning signs:** `cargo build` error: `expected expression` or `cannot index into value of type _`.

### Pitfall 3: Terraform Alias Not Updated After ECR Push
**What goes wrong:** Tests invoke the old Lambda version (60-second wait), causing test timeout.
**Why it happens:** ECR push only uploads the image; Lambda still uses the version from the previous `terraform apply`. The `live` alias must be updated to point to the new published version.
**How to avoid:** Always run `terraform apply` after `build-images.sh` completes. The apply triggers `image_uri` change detection, publishes a new version, and updates the alias.
**Warning signs:** Wait test times out after 60+ seconds (old handler) or passes but takes 65s+ instead of ~10s.

### Pitfall 4: get_execution_output Field Path Uncertainty
**What goes wrong:** The JMESPath query for the execution output field is wrong, returning null or an error.
**Why it happens:** The exact field name in `GetDurableExecution` response for the user result is not confirmed from a live execution (flagged as provisional in STATE.md).
**How to avoid:** In the first test run, add debug output (`echo "raw: $(aws lambda get-durable-execution ...)"`) to observe the full response structure. The most likely query is `--query 'Output' --output text` and then parsing the JSON string, but this must be verified.
**Warning signs:** `get_execution_output` returns empty or null; jq assertions fail with "null".

### Pitfall 5: invoke.rs Function Name vs Deployed Name
**What goes wrong:** `ctx.invoke("enrich_order", "order-enrichment-lambda", ...)` uses the function name `"order-enrichment-lambda"` but the deployed function is `"dr-order-enrichment-lambda-{suffix}"`.
**Why it happens:** The durable execution service resolves the function name internally. It may look up by a registered alias or base name rather than the full Terraform-prefixed name.
**How to avoid:** This was already proven working in `combined_workflow` (Phase 14) which calls `"fulfillment-lambda"` with the same naming convention. Trust the existing invoke.rs handler as-is; no changes needed.
**Warning signs:** Invoke test returns FunctionError with a "ResourceNotFoundException" for the function name.

### Pitfall 6: `get_execution_output` Returns JSON-in-String
**What goes wrong:** The `Output` field from `GetDurableExecution` may be a JSON string (doubly encoded), requiring `jq -r | jq` or `--raw-output` plus a second parse.
**Why it happens:** AWS APIs often return JSON payloads as string-escaped fields.
**How to avoid:** If `output=$(get_execution_output "$exec_arn")` followed by `echo "$output" | jq '.outcome.approved'` returns an error, try `echo "$output" | jq -r '.' | jq '.outcome.approved'`. Implement `get_execution_output` to handle this: `aws ... --output text` (removes the outer string quotes if AWS CLI handles it), or `aws ... --output json | jq -r '.Output'`.
**Warning signs:** jq parse error: "Cannot index string with string".

## Code Examples

Verified patterns from existing source:

### Handler Response Structures (Confirmed from Source Code)

**waits.rs (after modification):**
```rust
// Source: examples/*/src/waits.rs
Ok(serde_json::json!({
    "started": started.unwrap_or_default(),   // {"status": "started"}
    "completed": completed.unwrap_or_default(), // {"status": "completed"}
}))
// Full response: {"started":{"status":"started"},"completed":{"status":"completed"}}
```

**callbacks.rs (all 4 styles identical):**
```rust
// Source: examples/*/src/callbacks.rs
Ok(serde_json::json!({ "outcome": outcome.unwrap_or_default() }))
// outcome = {"approved": true, "callback_id": "<id>"}
// Full response: {"outcome":{"approved":true,"callback_id":"..."}}
```

**invoke.rs (all 4 styles identical):**
```rust
// Source: examples/*/src/invoke.rs
Ok(serde_json::json!({
    "order_id": order_id,     // round-trip of event["order_id"]
    "enrichment": enrichment, // response from order-enrichment-lambda
}))
```

**order-enrichment-lambda stub response (from infra/stubs/order_enrichment.py):**
```python
# Source: infra/stubs/order_enrichment.py
return {
    "enriched": True,
    "order_id": order_id,
    "details": {"priority": "standard", "region": "us-east-2"}
}
```

### get_execution_output Helper (Design)

```bash
# Source: Pattern derived from wait_for_terminal_status() in test-helpers.sh
# The exact --query path must be verified on first test run (see Open Questions #1)
get_execution_output() {
  local exec_arn="$1"
  aws lambda get-durable-execution \
    --profile "$PROFILE" \
    --region "$REGION" \
    --durable-execution-arn "$exec_arn" \
    --query 'Output' \
    --output text 2>/dev/null
}
```

If `Output` field returns a JSON-encoded string, use `jq -r '.' | jq` downstream.

### Waits Handler Modification (Representative — closure-style)

```rust
// Source: examples/closure-style/src/waits.rs — BEFORE
async fn handler(
    _event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // ...
    ctx.wait("cooling_period", 60).await?;
```

```rust
// AFTER
async fn handler(
    event: serde_json::Value,    // rename: _event -> event
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // ...
    let wait_secs = event["wait_seconds"].as_u64().unwrap_or(60);
    ctx.wait("cooling_period", wait_secs).await?;
```

### One-Liner Test Stubs to Replace (test-all.sh)

```bash
// Source: scripts/test-all.sh (Phase 15 stub replacements)
test_closure_waits()     { assert_waits "closure-waits"; }
test_macro_waits()       { assert_waits "macro-waits"; }
test_trait_waits()       { assert_waits "trait-waits"; }
test_builder_waits()     { assert_waits "builder-waits"; }

test_closure_callbacks() { assert_callbacks "closure-callbacks"; }
test_macro_callbacks()   { assert_callbacks "macro-callbacks"; }
test_trait_callbacks()   { assert_callbacks "trait-callbacks"; }
test_builder_callbacks() { assert_callbacks "builder-callbacks"; }

test_closure_invoke()    { assert_invoke "closure-invoke"; }
test_macro_invoke()      { assert_invoke "macro-invoke"; }
test_trait_invoke()      { assert_invoke "trait-invoke"; }
test_builder_invoke()    { assert_invoke "builder-invoke"; }
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hardcoded 60s wait in waits.rs | Event-driven duration via `event["wait_seconds"]` | Phase 15 | Reduces test time from 65s+ to ~10s per wait test |
| Combined workflow as only async proof | Dedicated async test helpers (assert_waits, assert_callbacks) | Phase 15 | Isolates each async operation for clear failure attribution |
| invoke tested only inside combined_workflow | Dedicated assert_invoke helper using invoke_sync | Phase 15 | invoke tests run in seconds without async overhead |

**Deprecated/outdated:**
- 12 stub functions in test-all.sh: These return `echo "STUB — not yet implemented"` and will be replaced by one-liner helper calls.

## Open Questions

1. **get_execution_output field path in GetDurableExecution response**
   - What we know: `wait_for_terminal_status` uses `--query 'Status'` successfully, confirming the CLI and API work. GetDurableExecution returns an object with at least a `Status` field.
   - What's unclear: Whether the completed execution's output is in `Output`, `Result`, `Payload`, or another field. STATE.md flags this as provisional.
   - Recommendation: In the first task of the plan, implement `get_execution_output` with `--query 'Output'` as the best guess. Add a debug step to print the raw response on first execution against a live wait test. If the field is wrong, the plan must correct the query before asserting on it. Consider outputting the full response body during the first test run to discover the shape.

2. **Wait test timeout sizing**
   - What we know: With `wait_seconds=5`, the Lambda will suspend for ~5 seconds then complete. The full round-trip (invoke_async + 5s wait + service re-invoke + finish_processing) will take ~10-15 seconds.
   - What's unclear: Whether there's additional service overhead (cold start, checkpoint storage) that could push this to 30+ seconds in practice.
   - Recommendation: Use a 60-second timeout in `wait_for_terminal_status`. This gives 4x margin and stays well under practical test run time.

3. **Invoke function name resolution by durable execution service**
   - What we know: `combined_workflow` uses `ctx.invoke("start_fulfillment", "fulfillment-lambda", ...)` with the base name (not `dr-fulfillment-lambda-{suffix}`), and this worked in Phase 14.
   - What's unclear: Whether the service resolves by function name prefix, by account-level alias, or by another mechanism.
   - Recommendation: Use the existing invoke.rs handlers unchanged. They use `"order-enrichment-lambda"` which follows the same naming convention as `"fulfillment-lambda"`. If it fails, investigate the exact function name the service resolves to.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | bash (no formal test framework) + AWS CLI v2 |
| Config file | none — test harness is self-contained in scripts/ |
| Quick run command | `bash scripts/test-all.sh closure-waits` |
| Full suite command | `bash scripts/test-all.sh` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| OPTEST-04 | 4 wait handlers invoked async, polled to SUCCEEDED, output validated | integration | `bash scripts/test-all.sh closure-waits` (and macro/trait/builder variants) | ❌ Wave 1 |
| OPTEST-05 | 4 callback handlers: async invoke → extract callback_id → send success → SUCCEEDED | integration | `bash scripts/test-all.sh closure-callbacks` (and macro/trait/builder variants) | ❌ Wave 1 |
| OPTEST-06 | 4 invoke handlers: sync invoke → order_id round-trip + enrichment non-null | integration | `bash scripts/test-all.sh closure-invoke` (and macro/trait/builder variants) | ❌ Wave 1 |

### Sampling Rate
- **Per task commit:** `bash -n scripts/test-helpers.sh && bash -n scripts/test-all.sh`
- **Per wave merge:** `bash scripts/test-all.sh` (requires live AWS credentials + deployed functions)
- **Phase gate:** All 12 Phase 15 tests show PASS in `bash scripts/test-all.sh` before completion

### Wave 0 Gaps
- [ ] `scripts/test-helpers.sh` — add `get_execution_output()`, `assert_waits()`, `assert_callbacks()`, `assert_invoke()` helpers
- [ ] `examples/*/src/waits.rs` (4 files) — modify `_event` → `event`, hardcoded `60` → `event["wait_seconds"].as_u64().unwrap_or(60)`
- [ ] `scripts/test-all.sh` — replace 12 stub bodies with one-liner helper calls
- [ ] Docker rebuild + ECR push: `bash scripts/build-images.sh`
- [ ] Lambda redeployment: `terraform -chdir=infra apply -parallelism=5`

## Sources

### Primary (HIGH confidence)
- `scripts/test-helpers.sh` (read directly) — all existing async helpers confirmed: invoke_async, wait_for_terminal_status, extract_callback_id, send_callback_success
- `scripts/test-all.sh` (read directly) — 12 Phase 15 stub functions confirmed, BINARY_TO_TEST map confirmed
- `examples/*/src/waits.rs` (all 4 read directly) — hardcoded `ctx.wait("cooling_period", 60)` confirmed in all 4 files
- `examples/*/src/callbacks.rs` (all 4 read directly) — handler returns `{"outcome":{"approved":bool,"callback_id":str}}` confirmed
- `examples/*/src/invoke.rs` (all 4 read directly) — calls `"order-enrichment-lambda"` with `{"order_id": order_id}`, returns `{"order_id":str,"enrichment":obj}` confirmed
- `infra/stubs/order_enrichment.py` (read directly) — stub returns `{"enriched":true,"order_id":str,"details":{...}}` confirmed
- `.planning/phases/16-advanced-feature-tests/16-02-SUMMARY.md` (read directly) — durable execution service protocol confirmed: SUCCEEDED unwraps to raw JSON, service handles async re-invocation

### Secondary (MEDIUM confidence)
- `.planning/STATE.md` (read directly) — "callback_id extraction is provisional" blocker confirmed; combined_workflow proved ctx.invoke works with base function name

### Tertiary (LOW confidence)
- `GetDurableExecution` `Output` field path — extrapolated from `wait_for_terminal_status` using `--query 'Status'` pattern; not confirmed from live execution

## Metadata

**Confidence breakdown:**
- Handler source code: HIGH — all 4 waits.rs, callbacks.rs, invoke.rs files read directly
- Test helpers (existing): HIGH — test-helpers.sh read directly; all async primitives confirmed present
- get_execution_output field path: LOW — cannot confirm without live execution; flagged in Open Questions
- Rebuild/redeploy pipeline: HIGH — build-images.sh and terraform apply read and confirmed from Phase 12 patterns

**Research date:** 2026-03-18
**Valid until:** 2026-04-17 (stable — no external library changes; only AWS CLI API behavior could shift)
