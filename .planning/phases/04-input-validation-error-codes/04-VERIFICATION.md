---
phase: 04-input-validation-error-codes
verified: 2026-03-16T14:30:00Z
status: passed
score: 6/6 must-haves verified
re_verification: false
---

# Phase 4: Input Validation & Error Codes Verification Report

**Phase Goal:** Invalid configuration is caught at construction time, and all DurableError variants have stable programmatic codes.
**Verified:** 2026-03-16T14:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| #   | Truth                                                                                                    | Status     | Evidence                                                                                                             |
| --- | -------------------------------------------------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------- |
| 1   | `StepOptions::new().retries(-1)` panics with descriptive message mentioning the invalid value            | VERIFIED   | `types.rs:202-208` — assert! with "StepOptions::retries: count must be >= 0, got {count}"; test at line 765-768     |
| 2   | `CallbackOptions::new().timeout_seconds(0)` panics with descriptive message                              | VERIFIED   | `types.rs:329-336` — assert! with "CallbackOptions::timeout_seconds: seconds must be > 0, got {seconds}"; test 797  |
| 3   | `MapOptions::new().batch_size(0)` panics with descriptive message                                        | VERIFIED   | `types.rs:516-522` — assert! with "MapOptions::batch_size: size must be > 0, got {size}"; test at line 825-828      |
| 4   | `DurableError::replay_mismatch(...).code()` returns `"REPLAY_MISMATCH"` (and similarly for all variants) | VERIFIED   | `error.rs:514-532` — exhaustive match, no wildcard arm, 15 variants; tests `error_code_replay_mismatch` + uniqueness |
| 5   | Backend retry detection uses error codes instead of string matching on error messages                    | VERIFIED   | `backend.rs:144-165` — `is_retryable_error` matches on `DurableError::AwsSdkOperation` and `AwsSdk` variants only   |
| 6   | Checkpoint response with None checkpoint_token returns DurableError instead of panicking                 | VERIFIED   | All 13 `if let Some(token)` sites replaced; `checkpoint_none_token_returns_error` test at `step.rs:1195`             |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact                                                        | Expected                                              | Status     | Details                                                                                                                                |
| --------------------------------------------------------------- | ----------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/durable-lambda-core/src/types.rs`                       | Validated option builders with panic guards           | VERIFIED   | `retries(i32)`, `backoff_seconds(i32)`, `timeout_seconds(i32)`, `heartbeat_timeout_seconds(i32)`, `batch_size(usize)` all have asserts |
| `crates/durable-lambda-core/src/error.rs`                       | `.code()` method on DurableError                      | VERIFIED   | `pub fn code(&self) -> &'static str` at line 514, exhaustive match, no wildcard arm                                                   |
| `crates/durable-lambda-core/src/backend.rs`                     | Structured retry detection using variant matching     | VERIFIED   | `DurableError::AwsSdkOperation` and `DurableError::AwsSdk` matched first; all other variants return false via `_ => false`            |
| `crates/durable-lambda-core/src/operations/step.rs`             | Defensive checkpoint token handling + None token test | VERIFIED   | 4 checkpoint_token sites use `ok_or_else`; `NoneTokenMockBackend` + `checkpoint_none_token_returns_error` test present                 |
| `crates/durable-lambda-core/src/operations/callback.rs`         | Defensive checkpoint token handling                   | VERIFIED   | 1 checkpoint_token site uses `ok_or_else`                                                                                             |
| `crates/durable-lambda-core/src/operations/wait.rs`             | Defensive checkpoint token handling                   | VERIFIED   | 1 checkpoint_token site uses `ok_or_else`                                                                                             |
| `crates/durable-lambda-core/src/operations/invoke.rs`           | Defensive checkpoint token handling                   | VERIFIED   | 1 checkpoint_token site uses `ok_or_else`                                                                                             |
| `crates/durable-lambda-core/src/operations/parallel.rs`         | Defensive checkpoint token handling                   | VERIFIED   | 2 checkpoint_token sites use `ok_or_else`                                                                                             |
| `crates/durable-lambda-core/src/operations/map.rs`              | Defensive checkpoint token handling                   | VERIFIED   | 2 checkpoint_token sites use `ok_or_else`                                                                                             |
| `crates/durable-lambda-core/src/operations/child_context.rs`    | Defensive checkpoint token handling                   | VERIFIED   | 2 checkpoint_token sites use `ok_or_else`                                                                                             |

**Total checkpoint_token sites converted:** 13 (grep confirmed: `\.checkpoint_token\(\)\.ok_or_else` — 13 occurrences across 7 files)
**Remaining silent `if let Some(token) = .*checkpoint_token()` patterns:** 0 (grep confirmed)

### Key Link Verification

| From                    | To                            | Via                                                           | Status   | Details                                                                                                     |
| ----------------------- | ----------------------------- | ------------------------------------------------------------- | -------- | ----------------------------------------------------------------------------------------------------------- |
| `types.rs`              | all callers of `.retries()`   | signature change `u32` -> `i32` with assert guard             | VERIFIED | `pub fn retries(mut self, count: i32)` at line 201; internal storage remains `Option<u32>` via `count as u32` |
| `error.rs`              | `backend.rs`                  | DurableError variant matching in `is_retryable_error`         | VERIFIED | `match err { DurableError::AwsSdkOperation(source) => ..., DurableError::AwsSdk(sdk_err) => ..., _ => false }` |
| `operations/*.rs`       | `error.rs`                    | `DurableError::checkpoint_failed` constructor in all 13 sites | VERIFIED | All 13 sites call `DurableError::checkpoint_failed(&name, std::io::Error::new(ErrorKind::InvalidData, "checkpoint response missing checkpoint_token"))` |

### Requirements Coverage

| Requirement | Source Plan   | Description                                                        | Status     | Evidence                                                                                         |
| ----------- | ------------- | ------------------------------------------------------------------ | ---------- | ------------------------------------------------------------------------------------------------ |
| FEAT-01     | 04-01-PLAN.md | StepOptions validates retries >= 0 and backoff_seconds >= 0        | SATISFIED  | `types.rs` lines 201-209, 228-235; tests `step_options_rejects_negative_retries/backoff`         |
| FEAT-02     | 04-01-PLAN.md | CallbackOptions validates timeout_seconds > 0 and heartbeat > 0    | SATISFIED  | `types.rs` lines 329-336, 356-363; tests `callback_options_rejects_zero_timeout/heartbeat`       |
| FEAT-03     | 04-01-PLAN.md | MapOptions validates batch_size > 0 when set                       | SATISFIED  | `types.rs` lines 516-522; test `map_options_rejects_zero_batch`                                  |
| FEAT-04     | 04-01-PLAN.md | Invalid option values panic or return descriptive error at construction | SATISFIED | All 5 builder methods use `assert!` with field name + value in the panic message                 |
| FEAT-05     | 04-02-PLAN.md | DurableError gains `.code() -> &str` for programmatic error matching | SATISFIED | `pub fn code(&self) -> &'static str` at `error.rs:514`; 15-variant exhaustive match             |
| FEAT-06     | 04-02-PLAN.md | Each DurableError variant returns a unique, stable error code      | SATISFIED  | `all_error_variants_have_unique_codes` test uses HashSet to verify 14 testable variants; no duplicates |
| FEAT-07     | 04-02-PLAN.md | Backend retry detection uses structured error codes instead of string matching | SATISFIED | `backend.rs:144-165` — variant match first; string check only inside `AwsSdkOperation`/`AwsSdk` arms |
| FEAT-08     | 04-03-PLAN.md | Checkpoint token None assumption replaced with defensive error handling | SATISFIED | 13 sites converted; `checkpoint_none_token_returns_error` test proves the error path            |

**Coverage:** 8/8 requirements for Phase 4 satisfied. No orphaned requirements detected.

**Orphaned requirement check:** REQUIREMENTS.md maps FEAT-01 through FEAT-08 to Phase 4. All 8 are claimed by the three plans and verified above.

### Anti-Patterns Found

None. Grep across all 10 modified files found zero instances of:
- `TODO`, `FIXME`, `XXX`, `HACK`, `PLACEHOLDER`
- `return null`, `return {}`, stub patterns

### Human Verification Required

None. All success criteria are programmatically verifiable:
- Panic behavior is tested via `#[should_panic(expected = "...")]` tests
- `.code()` return values are tested via assert_eq
- Retry logic is tested via unit tests with known inputs
- None-token error path is tested via `NoneTokenMockBackend`

All tests pass: `cargo test --workspace` — zero failures across all crates and doctests.
Clippy passes: `cargo clippy --workspace -- -D warnings` — no warnings.

---

## Summary

Phase 4 goal is fully achieved. The codebase now enforces:

1. **Construction-time validation** — five option builder methods (`StepOptions::retries`, `StepOptions::backoff_seconds`, `CallbackOptions::timeout_seconds`, `CallbackOptions::heartbeat_timeout_seconds`, `MapOptions::batch_size`) all panic immediately with descriptive messages when given invalid values. `StepOptions::retries` signature changed from `u32` to `i32` enabling negative-value rejection.

2. **Stable error codes** — `DurableError::code()` returns a unique `&'static str` for each of the 15 variants using an exhaustive match with no wildcard arm. Adding a new variant will produce a compile error, enforcing code updates.

3. **Structured retry detection** — `is_retryable_error` in `backend.rs` now matches on DurableError variants first. Only `AwsSdkOperation` and `AwsSdk` are eligible; `CheckpointFailed` with "Throttling" in its message is no longer incorrectly retried.

4. **Defensive checkpoint token handling** — all 13 `if let Some(token) = response.checkpoint_token()` sites across 7 operation files replaced with `.ok_or_else(|| DurableError::checkpoint_failed(...))`. A `checkpoint_none_token_returns_error` test proves the error path.

---

_Verified: 2026-03-16T14:30:00Z_
_Verifier: Claude (gsd-verifier)_
