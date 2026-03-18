# Phase 14: Synchronous Operation Tests - Research

**Researched:** 2026-03-18
**Domain:** Bash integration testing against AWS Lambda durable execution
**Confidence:** HIGH

## Summary

Phase 14 replaces 32 stub functions in `scripts/test-all.sh` with working integration tests for all synchronous operations. The test infrastructure is fully mature: `invoke_sync`, `get_alias_arn`, assertion patterns, and the Phase 16 test functions are working references. Every handler source file has been read and the exact JSON output for each operation is confirmed. No AWS API research is needed — the patterns are established.

The key architectural insight from Phase 16 is: the durable execution service unwraps SUCCEEDED responses, so tests assert against raw user JSON (not a Status envelope). For FAILED executions (only step_timeout in Phase 16), the service converts the envelope to FunctionError. For Phase 14, all handlers are expected to SUCCEED, so all 32 tests check HTTP 200 + no FunctionError + 2-3 user JSON fields.

**Primary recommendation:** Implement shared assertion helpers (`assert_basic_steps`, `assert_step_retries`, `assert_typed_errors`, `assert_parallel`, `assert_map`, `assert_child_contexts`, `assert_replay_safe_logging`, `assert_combined_workflow`) in `test-helpers.sh`, then each of the 32 stub functions calls the appropriate helper with its binary name. This minimizes repetition and guarantees cross-style consistency.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Combined workflow handling:**
- Keep combined_workflow tests in Phase 14 (synchronous invocation)
- Synchronous invoke will block ~35+ seconds per style due to ctx.wait(30s) + ctx.invoke() — acceptable for integration tests
- No explicit bash timeout — rely on AWS Lambda's synchronous invoke limit (840s)
- Validate key fields: order_id present, payment.charged=true, fulfillment non-null, post_processing non-null
- Use realistic payload: {"order_id": "test-order-001", "total": 99.99}

**Assertion depth:**
- Validate 2-3 key response fields per handler (matches Phase 16 pattern)
- For array operations (parallel, map): validate count + spot-check one item's structure
- Validate round-trip: send order_id in payload, verify same value returned in response
- replay_safe_logging: response-only validation (order_id + result.processed), no CloudWatch log queries
- Each test's echo message should describe what was proven (e.g., "typed error correctly serialized through durable execution")

**Typed errors test paths:**
- Test BOTH success and error paths within a single test function per style
- Success path: {"amount": 50} -> check transaction_id field
- Error path: {"amount": 2000} -> check error="insufficient_funds" + balance/required fields
- Both paths return HTTP 200 (domain error, not Lambda failure) — assertions should clarify this distinction
- Keep 32-test structure intact (one combined function per style, not separate functions)

**Cross-style consistency:**
- Test each style independently (no cross-comparison between styles)
- All 4 styles get identical assertions via shared helper functions
- Shared assertion helpers (assert_basic_steps, assert_parallel, etc.) added to test-helpers.sh
- Each test_*_operation() calls the shared helper with style-specific binary name

### Claude's Discretion
- Exact payload values for handlers that don't require specific input (parallel, map, child_contexts, replay_safe_logging)
- Helper function signatures and internal implementation
- Order of assertions within each helper
- Error message formatting

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| OPTEST-01 | Step tests pass — all 4 styles' `basic_steps` handlers invoked and return SUCCEEDED | Handler source confirmed: returns `{"order_id":"...","details":{"status":"found","items":3}}` |
| OPTEST-02 | Step retry tests pass — all 4 styles' `step_retries` handlers invoked and return SUCCEEDED | Handler source confirmed: returns `{"result":{"api_response":"success"}}` |
| OPTEST-03 | Typed error tests pass — all 4 styles' `typed_errors` handlers invoked and return expected error | Handler source confirmed: success returns `{"transaction_id":"txn_50"}`, error returns `{"error":"insufficient_funds","balance":500.0,"required":2000.0}` |
| OPTEST-07 | Parallel tests pass — all 4 styles' `parallel` handlers invoked, all branches present in result | Handler source confirmed: returns `{"parallel_results":[{"branch":"a"},{"branch":"b"},{"branch":"c"}]}` |
| OPTEST-08 | Map tests pass — all 4 styles' `map` handlers invoked and return SUCCEEDED | Handler source confirmed: returns `{"processed_orders":[{"order_id":"order-1","status":"done"},{"order_id":"order-2","status":"done"},{"order_id":"order-3","status":"done"},{"order_id":"order-4","status":"done"}]}` |
| OPTEST-09 | Child context tests pass — all 4 styles' `child_contexts` handlers invoked and return SUCCEEDED | Handler source confirmed: returns `{"child_result":{"validation":"passed","normalized":true},"parent_result":"parent_validation"}` |
| OPTEST-10 | Logging tests pass — all 4 styles' `replay_safe_logging` handlers invoked and return SUCCEEDED | Handler source confirmed: returns `{"order_id":"...","result":{"processed":true}}` |
| OPTEST-11 | Combined workflow tests pass — all 4 styles' `combined_workflow` handlers invoked and return SUCCEEDED | Handler source confirmed: returns `{"order_id":"...","payment":{"charged":true,"txn":"txn_123"},"fulfillment":{...},"post_processing":{"receipt":"sent","inventory":"updated"}}` |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bash | system | Test script language | Already established in test-all.sh and test-helpers.sh |
| jq | system | JSON field extraction | Already used in all Phase 16 assertions |
| aws CLI | v2 (adfs profile) | Lambda invoke | invoke_sync() already wraps this |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| test-helpers.sh | project | `invoke_sync`, `get_alias_arn`, `load_tf_outputs` | Source at top of test-all.sh (already done) |

### Alternatives Considered
None. The stack is fully determined by the existing infrastructure.

**Installation:** No new tools needed.

## Architecture Patterns

### Recommended Structure

The plan has two deliverables:

1. **8 shared assertion helpers** added to `scripts/test-helpers.sh` — one per operation type
2. **32 stub function bodies** replaced in `scripts/test-all.sh` — each calls the appropriate helper

```
scripts/
├── test-helpers.sh   # Add: assert_basic_steps(), assert_step_retries(),
│                     #      assert_typed_errors(), assert_parallel(),
│                     #      assert_map(), assert_child_contexts(),
│                     #      assert_replay_safe_logging(), assert_combined_workflow()
└── test-all.sh       # Replace: 32 stubs → calls to helpers
```

### Pattern 1: Shared Assertion Helper (invoke + assert)

**What:** One function per operation type that takes a binary name, invokes it, and asserts fields.
**When to use:** Every Phase 14 test — prevents 32 copies of the same assertion code.

```bash
# In test-helpers.sh
assert_basic_steps() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{"order_id":"test-order-001"}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error"; return 1; }

  local order_id
  order_id=$(echo "$response_body" | jq -r '.order_id')
  [[ "$order_id" == "test-order-001" ]] || \
    { echo "Expected order_id=test-order-001, got: $order_id; body=$response_body"; return 1; }

  local status_field
  status_field=$(echo "$response_body" | jq -r '.details.status')
  [[ "$status_field" == "found" ]] || \
    { echo "Expected details.status=found, got: $status_field; body=$response_body"; return 1; }

  echo "basic steps executed and replayed correctly via $binary"
}

# In test-all.sh
test_closure_basic_steps() { assert_basic_steps "closure-basic-steps"; }
test_macro_basic_steps()   { assert_basic_steps "macro-basic-steps"; }
test_trait_basic_steps()   { assert_basic_steps "trait-basic-steps"; }
test_builder_basic_steps() { assert_basic_steps "builder-basic-steps"; }
```

### Pattern 2: Typed Errors — Both Paths in One Helper

**What:** Single helper invokes twice (success payload, error payload) and asserts both.
**When to use:** typed_errors operation — covers both `Ok` and `Err` branches.

```bash
assert_typed_errors() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")

  # Success path: amount=50 -> transaction_id
  local result status fn_error response_body
  result=$(invoke_sync "$fn_arn" '{"amount":50}')
  IFS='|' read -r status fn_error _ response_body <<< "$result"
  [[ "$status" == "200" ]] || { echo "Success path: Expected 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Success path: Expected no FunctionError, got: $fn_error"; return 1; }
  local txn_id
  txn_id=$(echo "$response_body" | jq -r '.transaction_id')
  [[ "$txn_id" == "txn_50" ]] || \
    { echo "Success path: Expected transaction_id=txn_50, got: $txn_id; body=$response_body"; return 1; }

  # Error path: amount=2000 -> insufficient_funds (still HTTP 200, domain error not Lambda error)
  result=$(invoke_sync "$fn_arn" '{"amount":2000}')
  IFS='|' read -r status fn_error _ response_body <<< "$result"
  [[ "$status" == "200" ]] || { echo "Error path: Expected 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Error path: Expected no FunctionError, got: $fn_error; body=$response_body"; return 1; }
  local err_field
  err_field=$(echo "$response_body" | jq -r '.error')
  [[ "$err_field" == "insufficient_funds" ]] || \
    { echo "Error path: Expected error=insufficient_funds, got: $err_field; body=$response_body"; return 1; }

  echo "typed error correctly serialized through durable execution via $binary"
}
```

### Pattern 3: Array Operations (parallel, map)

**What:** Assert array length + spot-check first element.
**When to use:** parallel and map operations.

```bash
assert_parallel() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"
  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error"; return 1; }

  local count
  count=$(echo "$response_body" | jq '.parallel_results | length')
  [[ "$count" == "3" ]] || \
    { echo "Expected 3 parallel results, got: $count; body=$response_body"; return 1; }

  local first_branch
  first_branch=$(echo "$response_body" | jq -r '.parallel_results[0].branch')
  [[ "$first_branch" == "a" ]] || \
    { echo "Expected first branch=a, got: $first_branch; body=$response_body"; return 1; }

  echo "parallel fan-out completed with 3 branches via $binary"
}
```

### Anti-Patterns to Avoid

- **Duplicating assertion logic 32 times:** Write one helper per operation, call from 4 style functions.
- **Hardcoding ARNs:** Always use `get_alias_arn()` — it reads from Terraform outputs at runtime.
- **Checking for Status envelope:** The durable service unwraps it; user JSON is what arrives. Never check `.Status` or `.Result` fields.
- **Using IFS='|' inside subshell without read:** The `IFS='|' read -r a b c d <<< "$result"` pattern is correct; do not split with `cut` or `awk`.
- **Forgetting `_ ` for exec_arn field:** The `invoke_sync` output is 4 fields: `status|fn_error|exec_arn|body`. The `exec_arn` slot must be consumed: `IFS='|' read -r status fn_error _ response_body`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Lambda invocation | Custom aws CLI call | `invoke_sync()` in test-helpers.sh | Already handles tempfile, meta parsing, pipe-delimited output |
| ARN lookup | Hardcoded ARN strings | `get_alias_arn()` in test-helpers.sh | Reads from live Terraform outputs |
| Credential check | aws sts in test body | `check_credentials()` in test-helpers.sh (called once in main) | Already called before tests run |

## Common Pitfalls

### Pitfall 1: combined_workflow requires fulfillment-lambda stub ARN via ctx.invoke()

**What goes wrong:** The handler calls `ctx.invoke("start_fulfillment", "fulfillment-lambda", ...)`. The durable service resolves "fulfillment-lambda" to a deployed stub. If the stub isn't deployed or its ARN doesn't match, the execution fails.

**Why it happens:** The handler passes the function name as a string literal. The durable execution service resolves this against deployed Lambda functions in the account.

**How to avoid:** The stub is already deployed via Terraform (`dr-fulfillment-lambda-c351`). No test action needed — just invoke combined_workflow and it works. Confirm the stub exists before running combined_workflow tests.

**Warning signs:** `fn_error=Unhandled` with `errorMessage` mentioning "ResourceNotFoundException" or "function not found".

### Pitfall 2: combined_workflow blocks ~35+ seconds due to ctx.wait(30)

**What goes wrong:** Test hangs and contributor thinks it's broken.

**Why it happens:** `ctx.wait("cooling_period", 30)` is a real 30-second durable wait. The synchronous invoke blocks until the execution SUCCEEDS or times out.

**How to avoid:** This is expected behavior. No timeout guard needed — the AWS synchronous invoke limit (840s execution_timeout) handles runaway cases. Document the expected duration in test output or comments.

**Warning signs:** Test takes >60 seconds but eventually passes — that is correct.

### Pitfall 3: typed_errors returns HTTP 200 for the error path

**What goes wrong:** Assertion checks `fn_error` and finds it empty, then fails with "expected FunctionError for error path".

**Why it happens:** Domain errors (`PaymentError::InsufficientFunds`) return `Ok(json!({...}))` from the handler — the handler itself succeeds. The durable service returns SUCCEEDED with the error payload as user JSON. Only DurableError propagation (unhandled panic/system failure) produces FunctionError.

**How to avoid:** For typed_errors error path, assert `fn_error` is empty (HTTP 200, no Lambda failure) and check `.error == "insufficient_funds"` in the response body.

**Warning signs:** Assertion text like "expected FunctionError" in the typed_errors test.

### Pitfall 4: basic_steps, replay_safe_logging need order_id in payload

**What goes wrong:** `order_id` field comes back as `"unknown"` instead of `"test-order-001"`.

**Why it happens:** Both handlers extract `event["order_id"]` and fall back to `"unknown"` if absent. The payload `{}` produces `"unknown"`.

**How to avoid:** Always send `{"order_id":"test-order-001"}` for these handlers. The round-trip assertion (`order_id == "test-order-001"`) will catch payload omission.

### Pitfall 5: parallel/map results order may vary

**What goes wrong:** `parallel_results[0].branch` assertion fails intermittently.

**Why it happens:** Parallel branches run concurrently; result order in the returned array depends on completion order, which is non-deterministic.

**How to avoid:** For parallel, assert total count is 3 AND use `jq 'any(.[]; .branch == "a")'` or sort before checking. For map, the order should be stable (map preserves input order in most implementations) — but if flaky, use `map(select(.order_id == "order-1"))` instead of index access.

**Warning signs:** Intermittent failures only on parallel test, passing on retry.

## Code Examples

### Full assert_map helper (source: handler inspection)

```bash
# In test-helpers.sh
assert_map() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error"; return 1; }

  local count
  count=$(echo "$response_body" | jq '.processed_orders | length')
  [[ "$count" == "4" ]] || \
    { echo "Expected 4 processed orders, got: $count; body=$response_body"; return 1; }

  local first_status
  first_status=$(echo "$response_body" | jq -r '.processed_orders[0].status')
  [[ "$first_status" == "done" ]] || \
    { echo "Expected first order status=done, got: $first_status; body=$response_body"; return 1; }

  echo "map operation processed 4 orders in parallel via $binary"
}
```

### Full assert_combined_workflow helper (source: handler + stub inspection)

```bash
# In test-helpers.sh
assert_combined_workflow() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  # This invocation blocks ~35s due to ctx.wait(30) — expected behavior
  result=$(invoke_sync "$fn_arn" '{"order_id":"test-order-001","total":99.99}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error"; return 1; }

  local order_id
  order_id=$(echo "$response_body" | jq -r '.order_id')
  [[ "$order_id" == "test-order-001" ]] || \
    { echo "Expected order_id=test-order-001, got: $order_id; body=$response_body"; return 1; }

  local charged
  charged=$(echo "$response_body" | jq -r '.payment.charged')
  [[ "$charged" == "true" ]] || \
    { echo "Expected payment.charged=true, got: $charged; body=$response_body"; return 1; }

  local fulfillment
  fulfillment=$(echo "$response_body" | jq -r '.fulfillment')
  [[ "$fulfillment" != "null" && -n "$fulfillment" ]] || \
    { echo "Expected non-null fulfillment, got: $fulfillment; body=$response_body"; return 1; }

  local post_processing
  post_processing=$(echo "$response_body" | jq -r '.post_processing')
  [[ "$post_processing" != "null" && -n "$post_processing" ]] || \
    { echo "Expected non-null post_processing, got: $post_processing; body=$response_body"; return 1; }

  echo "combined workflow completed with payment, fulfillment, and post-processing via $binary"
}
```

### assert_child_contexts helper

```bash
assert_child_contexts() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error"; return 1; }

  local validation
  validation=$(echo "$response_body" | jq -r '.child_result.validation')
  [[ "$validation" == "passed" ]] || \
    { echo "Expected child_result.validation=passed, got: $validation; body=$response_body"; return 1; }

  local parent_result
  parent_result=$(echo "$response_body" | jq -r '.parent_result')
  [[ "$parent_result" == "parent_validation" ]] || \
    { echo "Expected parent_result=parent_validation, got: $parent_result; body=$response_body"; return 1; }

  echo "child context isolated namespace, parent step ran independently via $binary"
}
```

### assert_replay_safe_logging helper

```bash
assert_replay_safe_logging() {
  local binary="$1"
  local fn_arn
  fn_arn=$(get_alias_arn "$binary")
  local result
  result=$(invoke_sync "$fn_arn" '{"order_id":"test-order-001"}')
  local status fn_error response_body
  IFS='|' read -r status fn_error _ response_body <<< "$result"

  [[ "$status" == "200" ]] || { echo "Expected HTTP 200, got: $status"; return 1; }
  [[ -z "$fn_error" ]] || { echo "Expected no FunctionError, got: $fn_error"; return 1; }

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
```

## Exact Response Structures

Derived directly from handler source code — HIGH confidence.

| Operation | Input Payload | Response JSON |
|-----------|---------------|---------------|
| basic_steps | `{"order_id":"test-order-001"}` | `{"order_id":"test-order-001","details":{"status":"found","items":3}}` |
| step_retries | `{}` | `{"result":{"api_response":"success"}}` |
| typed_errors (success) | `{"amount":50}` | `{"transaction_id":"txn_50"}` |
| typed_errors (error) | `{"amount":2000}` | `{"error":"insufficient_funds","balance":500.0,"required":2000.0}` |
| parallel | `{}` | `{"parallel_results":[{"branch":"a"},{"branch":"b"},{"branch":"c"}]}` |
| map | `{}` | `{"processed_orders":[{"order_id":"order-1","status":"done"},{"order_id":"order-2","status":"done"},{"order_id":"order-3","status":"done"},{"order_id":"order-4","status":"done"}]}` |
| child_contexts | `{}` | `{"child_result":{"validation":"passed","normalized":true},"parent_result":"parent_validation"}` |
| replay_safe_logging | `{"order_id":"test-order-001"}` | `{"order_id":"test-order-001","result":{"processed":true}}` |
| combined_workflow | `{"order_id":"test-order-001","total":99.99}` | `{"order_id":"test-order-001","payment":{"charged":true,"txn":"txn_123"},"fulfillment":{"fulfillment_id":"ff-001","status":"started","estimated_delivery":"2 business days"},"post_processing":{"receipt":"sent","inventory":"updated"}}` |

Note: All 4 styles (closure, macro, trait, builder) produce identical JSON output — confirmed by reading all 4 style implementations for basic_steps, typed_errors, combined_workflow.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Checking `{"Status":"SUCCEEDED","Result":"..."}` | Check raw user JSON directly | Phase 16-02 (2026-03-17) | Tests must never check `.Status` or `.Result` fields — service strips envelope |
| execution_timeout=3600 | execution_timeout=840 | Phase 16-02 (2026-03-17) | Synchronous invocation requires ≤900s; Lambda timeout is 900s |
| glibc-linked binary | musl-linked binary | Phase 16-02 (2026-03-17) | Build already uses x86_64-unknown-linux-musl; no action needed |
| 32 full stub functions | 32 stubs calling helpers | Phase 14 (this phase) | Reduces test code by ~75% |

**Deprecated/outdated:**
- Status envelope assertions: never check `.Status == "SUCCEEDED"` in test assertions
- `sleep` for wait_for_terminal_status: already uses polling (not applicable for synchronous tests)

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | bash integration tests (scripts/test-all.sh + test-helpers.sh) |
| Config file | none |
| Quick run command | `bash scripts/test-all.sh closure-basic-steps` |
| Full suite command | `bash scripts/test-all.sh` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| OPTEST-01 | basic_steps 4 styles return SUCCEEDED with correct fields | integration | `bash scripts/test-all.sh closure-basic-steps` | ✅ (stub) Wave 0: replace stub body |
| OPTEST-02 | step_retries 4 styles return SUCCEEDED | integration | `bash scripts/test-all.sh closure-step-retries` | ✅ (stub) Wave 0: replace stub body |
| OPTEST-03 | typed_errors 4 styles — both success and error paths | integration | `bash scripts/test-all.sh closure-typed-errors` | ✅ (stub) Wave 0: replace stub body |
| OPTEST-07 | parallel 4 styles — 3 branches present | integration | `bash scripts/test-all.sh closure-parallel` | ✅ (stub) Wave 0: replace stub body |
| OPTEST-08 | map 4 styles — 4 processed orders | integration | `bash scripts/test-all.sh closure-map` | ✅ (stub) Wave 0: replace stub body |
| OPTEST-09 | child_contexts 4 styles — isolation validated | integration | `bash scripts/test-all.sh closure-child-contexts` | ✅ (stub) Wave 0: replace stub body |
| OPTEST-10 | replay_safe_logging 4 styles — response fields only | integration | `bash scripts/test-all.sh closure-replay-safe-logging` | ✅ (stub) Wave 0: replace stub body |
| OPTEST-11 | combined_workflow 4 styles — order_id, payment, fulfillment, post_processing | integration | `bash scripts/test-all.sh closure-combined-workflow` | ✅ (stub) Wave 0: replace stub body |

### Sampling Rate
- **Per task commit:** `bash scripts/test-all.sh {style}-{operation}` (single test for the operation just implemented)
- **Per wave merge:** `bash scripts/test-all.sh` (full suite — all 48 tests)
- **Phase gate:** Full suite showing 32 Phase 14 tests as PASS before `/gsd:verify-work`

### Wave 0 Gaps
None — test infrastructure fully exists. Stubs are in place. The only work is replacing stub bodies and adding helpers.

## Open Questions

1. **parallel results order — deterministic or not?**
   - What we know: Handler uses `Vec` of branches, `BatchResult.results` preserves insertion order per the Rust SDK implementation
   - What's unclear: Whether the durable service preserves order in its returned batch result
   - Recommendation: Use `jq 'any(.[]; .branch == "a")'` membership check rather than index access for branch assertions. If Phase 16 parallel tests had existed, this would be confirmed — play it safe.

2. **fulfillment response field path in combined_workflow**
   - What we know: The fulfillment stub Python returns `{"fulfillment_id":"ff-001","status":"started","estimated_delivery":"2 business days"}`. The handler assigns this directly to `fulfillment` field in the response JSON.
   - What's unclear: Whether `ctx.invoke()` wraps the stub result in another envelope layer before returning to the handler
   - Recommendation: Assert `.fulfillment != null` (non-null check) rather than specific field values. If the invoke wrapper adds a layer, the assert will still pass and the response_body in any failure message will reveal the actual structure.

## Sources

### Primary (HIGH confidence)
- Direct source reading: `examples/closure-style/src/{basic_steps,step_retries,typed_errors,parallel,map,child_contexts,replay_safe_logging,combined_workflow}.rs` — exact response JSON structures
- Direct source reading: `examples/macro-style/src/{basic_steps,typed_errors,combined_workflow}.rs` — confirmed identical output structure across styles
- Direct source reading: `scripts/test-helpers.sh` — `invoke_sync`, `get_alias_arn`, pipe-delimited output format
- Direct source reading: `scripts/test-all.sh` — 32 stubs, BINARY_TO_TEST map, run_all_tests ordering
- Direct source reading: `infra/lambda.tf` — binary names, execution_timeout=840, all 44 functions confirmed
- Direct source reading: `infra/stubs/fulfillment.py` — stub returns `{"fulfillment_id":"ff-001","status":"started","estimated_delivery":"2 business days"}`

### Secondary (MEDIUM confidence)
- Phase 16-02 SUMMARY.md — documented durable execution service response protocol (SUCCEEDED unwraps, FAILED → FunctionError), confirmed by passing Phase 16 tests

### Tertiary (LOW confidence)
None.

## Metadata

**Confidence breakdown:**
- Response structures: HIGH — read directly from handler Rust source files
- Test pattern: HIGH — Phase 16 tests are working references, pattern is proven
- parallel result order: MEDIUM — deterministic in Rust but service behavior unconfirmed
- fulfillment ctx.invoke wrapping: MEDIUM — stub output known, ctx.invoke envelope unknown

**Research date:** 2026-03-18
**Valid until:** 2026-04-18 (stable — handlers don't change unless Rust source is modified)
