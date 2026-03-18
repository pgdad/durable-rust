# Phase 14: Synchronous Operation Tests - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement 32 test functions in test-all.sh replacing stubs for all synchronous operations (basic_steps, step_retries, typed_errors, parallel, map, child_contexts, replay_safe_logging, combined_workflow) across 4 API styles (closure, macro, trait, builder). Each test invokes the deployed Lambda synchronously and validates the response.

Requirements: OPTEST-01, OPTEST-02, OPTEST-03, OPTEST-07, OPTEST-08, OPTEST-09, OPTEST-10, OPTEST-11.

</domain>

<decisions>
## Implementation Decisions

### Combined workflow handling
- Keep combined_workflow tests in Phase 14 (synchronous invocation)
- Synchronous invoke will block ~35+ seconds per style due to ctx.wait(30s) + ctx.invoke() — acceptable for integration tests
- No explicit bash timeout — rely on AWS Lambda's synchronous invoke limit (840s)
- Validate key fields: order_id present, payment.charged=true, fulfillment non-null, post_processing non-null
- Use realistic payload: {"order_id": "test-order-001", "total": 99.99}

### Assertion depth
- Validate 2-3 key response fields per handler (matches Phase 16 pattern)
- For array operations (parallel, map): validate count + spot-check one item's structure
- Validate round-trip: send order_id in payload, verify same value returned in response
- replay_safe_logging: response-only validation (order_id + result.processed), no CloudWatch log queries
- Each test's echo message should describe what was proven (e.g., "typed error correctly serialized through durable execution")

### Typed errors test paths
- Test BOTH success and error paths within a single test function per style
- Success path: {"amount": 50} -> check transaction_id field
- Error path: {"amount": 2000} -> check error="insufficient_funds" + balance/required fields
- Both paths return HTTP 200 (domain error, not Lambda failure) — assertions should clarify this distinction
- Keep 32-test structure intact (one combined function per style, not separate functions)

### Cross-style consistency
- Test each style independently (no cross-comparison between styles)
- All 4 styles get identical assertions via shared helper functions
- Shared assertion helpers (assert_basic_steps, assert_parallel, etc.) added to test-helpers.sh
- Each test_*_operation() calls the shared helper with style-specific binary name

### Claude's Discretion
- Exact payload values for handlers that don't require specific input (parallel, map, child_contexts, replay_safe_logging)
- Helper function signatures and internal implementation
- Order of assertions within each helper
- Error message formatting

</decisions>

<specifics>
## Specific Ideas

- Follow the Phase 16 test pattern exactly: invoke_sync -> parse pipe-delimited output -> field-level jq assertions
- Use get_alias_arn() for all ARN lookups (already in test-helpers.sh)
- Handler source code in examples/{style}/src/{operation}.rs shows exact JSON output structure
- basic_steps returns: {"order_id": "...", "details": {"status": "found", "items": 3}}
- step_retries returns: {"result": {"api_response": "success"}}
- parallel returns: {"parallel_results": [{"branch": "a"}, {"branch": "b"}, {"branch": "c"}]}
- map returns: {"processed_orders": [{"order_id": "order-1", "status": "done"}, ...]}
- child_contexts returns: {"child_result": {"validation": "passed", "normalized": true}, "parent_result": "parent_validation"}
- replay_safe_logging returns: {"order_id": "...", "result": {"processed": true}}
- combined_workflow returns: {"order_id": "...", "payment": {"charged": true, "txn": "txn_123"}, "fulfillment": {...}, "post_processing": {"receipt": "sent", "inventory": "updated"}}

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `scripts/test-helpers.sh`: invoke_sync(), get_alias_arn(), check_credentials(), load_tf_outputs() — all ready to use
- `scripts/test-all.sh`: 32 stub functions with BINARY_TO_TEST map and run_all_tests() orchestration — stubs replaced in-place
- Phase 16 test implementations: 4 fully working tests (saga, timeout, conditional_retry, batch_checkpoint) as reference pattern

### Established Patterns
- invoke_sync returns `status_code|fn_error|exec_arn|response_body` parsed via IFS='|' read
- Assertions use jq -r for field extraction, bash [[ ]] for comparison
- Error path: `{ echo "Expected X, got: Y; body=$response_body"; return 1; }`
- Success echo at end of each test function describes what was verified

### Integration Points
- ALIAS_ARNS loaded from Terraform outputs by load_tf_outputs() (called once in main)
- Binary names in BINARY_TO_TEST map match Terraform lambda.tf handler keys
- fulfillment-lambda stub available via get_stub_arn() for combined_workflow's ctx.invoke()

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 14-synchronous-operation-tests*
*Context gathered: 2026-03-18*
