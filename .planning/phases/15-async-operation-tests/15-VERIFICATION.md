---
phase: 15-async-operation-tests
verified: 2026-03-18T16:30:00Z
status: human_needed
score: 6/6 must-haves verified (automated); runtime correctness needs human
re_verification: false
human_verification:
  - test: "Run bash scripts/test-all.sh with valid ADFS credentials and confirm all 12 Phase 15 tests report [PASS]"
    expected: "closure-waits, macro-waits, trait-waits, builder-waits, closure-callbacks, macro-callbacks, trait-callbacks, builder-callbacks, closure-invoke, macro-invoke, trait-invoke, builder-invoke all show [PASS]"
    why_human: "Tests invoke real AWS Lambda durable execution endpoints; cannot verify live AWS state programmatically. STATE.md also notes the get_execution_output Output field path was provisional and must be confirmed against a live execution."
  - test: "Invoke one wait handler manually: aws lambda invoke --profile adfs --region us-east-2 --function-name <closure-waits-live-arn> --invocation-type Event --durable-execution-name manual-verify-1 --payload '{\"wait_seconds\":5}' /tmp/out.json, then poll get-durable-execution until SUCCEEDED and call get-durable-execution with --query Output to confirm the field name is Output and the value contains started.status=started and completed.status=completed"
    expected: "get-durable-execution --query 'Output' returns valid JSON with .started.status==\"started\" and .completed.status==\"completed\""
    why_human: "STATE.md line 111 flags this as a blocker: 'Exact JSON field paths for GetDurableExecution response (callback_id location) must be confirmed against a live execution before finalizing polling shell functions — treat as provisional until then.' The Output field used in get_execution_output() has never been confirmed against an actual SUCCEEDED execution response."
  - test: "Run one callback test manually: invoke closure-callbacks async, wait for CallbackStarted event (extract_callback_id), send callback success, poll to SUCCEEDED, and confirm outcome.approved=true in the Output field"
    expected: "assert_callbacks passes end-to-end: exec_arn returned, callback_id extracted from history, approval signal accepted, SUCCEEDED reached, outcome.approved=true in output"
    why_human: "The callback flow involves 5 sequential AWS API calls with real-time state transitions. The provisional Output field concern applies here too — if Output is wrong, callbacks appear to work but silently return empty assertions."
---

# Phase 15: Async Operation Tests Verification Report

**Phase Goal:** Wait, callback, and invoke operations complete successfully against real AWS, with correct state polling before callback signal dispatch
**Verified:** 2026-03-18T16:30:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | All 4 waits.rs handlers accept event-driven wait duration via event["wait_seconds"] | VERIFIED | All 4 files contain `event["wait_seconds"].as_i64().unwrap_or(60) as i32` and pass `cargo build --workspace` |
| 2 | Updated Lambda images are deployed and reachable via live alias (15-01) | ? UNCERTAIN | Cannot verify live AWS state without credentials — SUMMARY reports successful redeploy |
| 3 | assert_waits helper: invoke_async with 5s wait -> poll SUCCEEDED -> validate started/completed | VERIFIED | Lines 537-571 of test-helpers.sh implement the complete async flow with jq assertions |
| 4 | assert_callbacks helper: invoke_async -> extract_callback_id -> send_callback_success -> wait -> validate outcome.approved | VERIFIED | Lines 580-621 implement the full callback signal-then-poll pattern |
| 5 | assert_invoke helper: invoke_sync, order_id round-trip, enrichment non-null | VERIFIED | Lines 628-656 implement synchronous invoke with both validations |
| 6 | 12 Phase 15 test functions wired to correct assertion helpers in test-all.sh | VERIFIED | Lines 108-121 contain one-liner delegations; zero STUB strings; BINARY_TO_TEST and run_all_tests both include all 12 entries |

**Score:** 5/6 truths fully verified statically; 1 uncertain (live AWS deployment state); runtime correctness requires human verification

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `examples/closure-style/src/waits.rs` | Event-driven wait duration via event["wait_seconds"] | VERIFIED | `event["wait_seconds"].as_i64().unwrap_or(60) as i32` at line 23, wired to `ctx.wait()` at line 24 |
| `examples/macro-style/src/waits.rs` | Event-driven wait duration | VERIFIED | Same pattern at lines 24-25 |
| `examples/trait-style/src/waits.rs` | Event-driven wait duration | VERIFIED | Same pattern at lines 24-25 |
| `examples/builder-style/src/waits.rs` | Event-driven wait duration | VERIFIED | Same pattern at lines 18-19 |
| `scripts/test-helpers.sh` | 4 new helpers: get_execution_output, assert_waits, assert_callbacks, assert_invoke | VERIFIED | 656 lines; Phase 15 section at lines 509-656; all 4 functions present and substantive |
| `scripts/test-all.sh` | 12 Phase 15 test stubs replaced with one-liner helper calls | VERIFIED | Lines 108-121 contain all 12 one-liner delegations; 0 STUB strings; all 12 in BINARY_TO_TEST and run_all_tests |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `test-helpers.sh:assert_waits` | `invoke_async + wait_for_terminal_status + get_execution_output` | async flow chain | WIRED | Lines 544, 549, 555 — all 3 helpers called in sequence with exec_arn threading through |
| `test-helpers.sh:assert_callbacks` | `invoke_async + extract_callback_id + send_callback_success + wait_for_terminal_status + get_execution_output` | callback flow chain | WIRED | Lines 587, 592, 597, 604, 610 — full 5-step chain implemented |
| `test-helpers.sh:assert_invoke` | `invoke_sync` | synchronous invocation | WIRED | Line 634 calls invoke_sync; lines 644-652 validate order_id and enrichment |
| `test-all.sh:test_closure_waits` | `test-helpers.sh:assert_waits` | one-liner delegation | WIRED | Line 108: `test_closure_waits() { assert_waits "closure-waits"; }` — confirmed |
| `waits.rs:wait_secs` | `ctx.wait("cooling_period", wait_secs)` | event payload read | WIRED | All 4 files: extraction immediately precedes ctx.wait() call with no intervening logic |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| OPTEST-04 | 15-01-PLAN.md, 15-02-PLAN.md | Wait tests pass — test variant with 5-second wait deployed, invoked async, polled to SUCCEEDED | SATISFIED (static) | All 4 waits.rs files accept `wait_seconds`, assert_waits implements full async flow; live test result needs human confirmation |
| OPTEST-05 | 15-02-PLAN.md | Callback tests pass — all 4 styles invoked async, callback signal sent, polled to SUCCEEDED | SATISFIED (static) | assert_callbacks implements full signal-then-poll flow for all 4 binaries; live test result needs human confirmation |
| OPTEST-06 | 15-02-PLAN.md | Invoke tests pass — caller invokes callee stub, returns callee result in response | SATISFIED (static) | assert_invoke validates order_id round-trip and enrichment non-null for all 4 invoke binaries; live test result needs human confirmation |

All 3 phase 15 requirements (OPTEST-04, OPTEST-05, OPTEST-06) are mapped in REQUIREMENTS.md traceability table to Phase 15 with status "Complete". No orphaned requirements found.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `scripts/test-helpers.sh` | 527 | `--query 'Output' --output text` — provisional field path noted in STATE.md (line 111) as unconfirmed | Warning | If AWS GetDurableExecution response uses a different field name, get_execution_output returns empty string, causing assert_waits and assert_callbacks to fail the `[[ -n "$output" ]]` check; assertions never reach jq |

No TODO/FIXME comments found in any Phase 15 modified files. No empty implementations or placeholder returns.

### Human Verification Required

#### 1. Full 12-test async suite execution

**Test:** Run `bash scripts/test-all.sh` (or only Phase 15 subset via individual binary names) with valid ADFS credentials against the deployed Lambda functions.

**Expected:** All 12 tests — closure/macro/trait/builder waits, callbacks, invoke — report `[PASS]`.

**Why human:** Tests invoke real AWS Lambda durable execution endpoints. The code logic is correct and fully wired, but correctness of the live execution service integration (state polling, callback signal delivery, invoke chaining to order-enrichment-lambda) cannot be asserted statically.

#### 2. Confirm get_execution_output Output field path

**Test:** After any SUCCEEDED async execution completes, run `aws lambda get-durable-execution --profile adfs --region us-east-2 --durable-execution-arn <exec_arn>` without `--query` to inspect the raw response. Confirm the user result is stored under `Output` (and not `Result`, `Payload`, or another key).

**Expected:** Raw response JSON contains `"Output": "<user-json-string>"` which `--query Output --output text` correctly retrieves.

**Why human:** STATE.md line 111 explicitly calls this out as a blocker: "Exact JSON field paths for GetDurableExecution response (callback_id location) must be confirmed against a live execution before finalizing polling shell functions — treat as provisional until then." This affects all assert_waits and assert_callbacks output validation steps.

#### 3. Callback signal timing verification

**Test:** Manually invoke a callbacks handler and observe that `extract_callback_id` successfully finds the `CallbackStarted` event in the execution history before the 60-second timeout. Confirm `send_callback_success` with `{"approved":true}` drives the execution to SUCCEEDED.

**Expected:** callback_id extracted within a few seconds of invocation, success signal accepted, execution reaches SUCCEEDED within 60 seconds, output contains `outcome.approved=true`.

**Why human:** The real-time state machine transitions (RUNNING -> SUSPENDED at callback wait -> SUCCEEDED after signal) require live AWS service behavior to confirm. Race conditions between polling and state transitions cannot be replicated statically.

### Gaps Summary

No automated gaps found. All artifacts exist, are substantive, are fully wired, and have no placeholder patterns. All 3 requirement IDs (OPTEST-04, OPTEST-05, OPTEST-06) are covered by verifiable implementation.

The only outstanding concern is runtime validation: the `get_execution_output` function uses `--query 'Output'` which was documented as provisional in STATE.md. This is a known risk that the phase authors acknowledged, and it is not a gap in the implementation logic — it is a live-service contract assumption that can only be confirmed by execution.

---

_Verified: 2026-03-18T16:30:00Z_
_Verifier: Claude (gsd-verifier)_
