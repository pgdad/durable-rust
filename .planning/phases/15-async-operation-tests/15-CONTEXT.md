# Phase 15: Async Operation Tests - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement 12 test functions in test-all.sh replacing stubs for async operations (waits, callbacks, invoke) across 4 API styles. Wait and callback tests use async invocation + polling. Invoke tests use synchronous invocation. Includes modifying waits.rs handlers to accept event-driven wait duration and redeploying updated images.

Requirements: OPTEST-04, OPTEST-05, OPTEST-06.

</domain>

<decisions>
## Implementation Decisions

### Wait handler duration
- Modify all 4 styles' waits.rs to read wait duration from event payload: `event["wait_seconds"].as_u64().unwrap_or(60)`
- Test sends `{"wait_seconds": 5}` for a ~5-second wait instead of 60 seconds
- Handler change + Docker rebuild + ECR push + Lambda update is part of Phase 15 plan (self-contained, not a prerequisite)
- Validate terminal status SUCCEEDED + response fields: started.status=="started" and completed.status=="completed"

### Invoke test approach
- Use synchronous invocation (invoke_sync) — proven to work from combined_workflow in Phase 14
- Invoke handler calls order-enrichment-lambda with {"order_id": "test-invoke-001"}
- Validate round-trip: order_id matches sent value, enrichment field is non-null
- No async invocation needed for invoke tests

### Callback test flow
- Full flow with retries: invoke_async -> extract_callback_id (polls every 3s with timeout) -> send_callback_success({"approved": true}) -> wait_for_terminal_status -> get_execution_output
- Trust existing extract_callback_id() helper — designed for this exact flow
- Validate SUCCEEDED status + outcome.approved==true (proves callback result received and processed)
- Don't assert callback_id in response — it's an internal ID
- Need get_execution_output(exec_arn) helper to retrieve result after async completion

### Shared helper pattern
- Create assert_waits(binary_name), assert_callbacks(binary_name), assert_invoke(binary_name) in test-helpers.sh
- Each encapsulates the full async flow (multi-step for waits/callbacks, simpler for invoke)
- One-liner callers in test-all.sh — consistent with Phase 14 pattern
- Add get_execution_output(exec_arn) as a new reusable helper in test-helpers.sh for retrieving async execution results

### Claude's Discretion
- Exact error messages in assertion failures
- get_execution_output helper implementation details (JMESPath query for Output field)
- Whether wait tests also verify timing (e.g., execution took >= 5s)
- Order of operations within assert_callbacks flow

</decisions>

<specifics>
## Specific Ideas

- waits.rs handler returns: {"started": {"status": "started"}, "completed": {"status": "completed"}}
- callbacks.rs handler returns: {"outcome": {"approved": true/false, "callback_id": "..."}}
- invoke.rs handler returns: {"order_id": "...", "enrichment": {...}}
- invoke.rs calls "order-enrichment-lambda" (stub from Phase 11, available via get_stub_arn())
- Wait tests: invoke_async with {"wait_seconds": 5} -> poll SUCCEEDED -> get_execution_output -> validate fields
- Callback tests: invoke_async -> extract_callback_id -> send_callback_success({"approved": true}) -> poll SUCCEEDED -> get_execution_output -> validate outcome.approved
- Invoke tests: invoke_sync with {"order_id": "test-invoke-001"} -> validate order_id round-trip + enrichment non-null
- STATE.md blocker: callback_id extraction is provisional — test against live execution will confirm

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `scripts/test-helpers.sh`: invoke_async(), wait_for_terminal_status(), extract_callback_id(), send_callback_success() — all ready for async tests
- `scripts/test-helpers.sh`: invoke_sync(), get_alias_arn() — ready for invoke tests
- `scripts/test-helpers.sh`: 8 assert_* helpers from Phase 14 as pattern reference
- `scripts/test-all.sh`: 12 Phase 15 stub functions with BINARY_TO_TEST map entries
- `scripts/build-images.sh`: Rebuild + push pipeline for updated waits handlers

### Established Patterns
- Phase 14 assert_* helpers: get_alias_arn -> invoke_sync -> IFS='|' read -> jq assertions -> echo
- Async polling: wait_for_terminal_status polls every 3s, returns terminal status string
- Callback extraction: extract_callback_id polls get-durable-execution-history for CallbackStarted event

### Integration Points
- order-enrichment-lambda stub ARN via get_stub_arn("order-enrichment-lambda")
- ALIAS_ARNS loaded from Terraform outputs (load_tf_outputs in main)
- Docker images rebuilt via scripts/build-images.sh, Lambda updated via terraform apply
- 4 waits.rs files: examples/{closure,macro,trait,builder}-style/src/waits.rs

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 15-async-operation-tests*
*Context gathered: 2026-03-18*
