# Phase 4: Input Validation & Error Codes - Research

**Researched:** 2026-03-16
**Domain:** Rust builder-pattern validation, enum method extension, string-matching refactor
**Confidence:** HIGH

## Summary

Phase 4 is a self-contained hardening pass over `durable-lambda-core`. The changes fall into three narrow buckets: (1) adding panicking guards to option builder methods, (2) adding a `.code() -> &str` method to `DurableError`, and (3) replacing the `is_retryable_error` string-matching function in `backend.rs` with a match on structured error codes.

All work is localized to two source files in `durable-lambda-core`: `src/types.rs` (options structs) and `src/error.rs` + `src/backend.rs` (error codes and retry detection). There are no external dependencies to add — the patterns are standard Rust idioms that already appear in the codebase.

The main subtlety is the type mismatch that already exists: `StepOptions::retries` accepts `u32` (cannot be negative at the type level), while `backoff_seconds` accepts `i32` (can be negative). The requirement to make `retries(-1)` panic implies the signatures must change to accept `i32` so invalid values can be caught and rejected at runtime — or the tests demonstrate the pattern using `backoff_seconds` which already accepts `i32`. Similarly, `CallbackOptions::timeout_seconds` already accepts `i32` and currently treats 0 as "no timeout", so FEAT-02 requires inverting that assumption to enforce > 0. `MapOptions::batch_size` accepts `usize`, so 0 is the only invalid case the type doesn't already rule out.

**Primary recommendation:** Change `StepOptions::retries` to accept `i32` (mirrors `backoff_seconds`), add `panic!` guards in all three option setter methods, add `.code()` as a plain method on `DurableError` that matches on `self`, and replace `is_retryable_error`'s `to_string()`-then-substring approach with a `match self` on the `DurableError` variants.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FEAT-01 | StepOptions validates retries >= 0 and backoff_seconds >= 0 | Type analysis: `retries` is `u32` (always >= 0), `backoff_seconds` is `i32` (needs guard). Signature change for `retries` needed to accept negative input and panic. |
| FEAT-02 | CallbackOptions validates timeout_seconds > 0 and heartbeat_timeout_seconds > 0 | Both fields are `i32` with default 0. Docs say 0 = "no timeout"; requirement flips this to require > 0 when set. Guard added to setter. |
| FEAT-03 | MapOptions validates batch_size > 0 when set | Field is `usize`; type already excludes negatives. Only guard needed: 0 is invalid. |
| FEAT-04 | Invalid option values panic or return descriptive error at construction | Panicking with `panic!("...")` is idiomatic for builder precondition violations in Rust. All three setters get `assert!`/`panic!` guards. |
| FEAT-05 | DurableError gains `.code() -> &str` for programmatic error matching | Add `pub fn code(&self) -> &str` method; match on all variants; return `"SNAKE_CASE"` string constants. |
| FEAT-06 | Each DurableError variant returns a unique, stable error code | 13 variants identified; each gets a unique uppercase string code. |
| FEAT-07 | Backend retry detection uses structured error codes instead of string matching | `is_retryable_error` in `backend.rs` currently uses `to_string().to_lowercase().contains(...)`. Replace with `match err` on DurableError variants + `.code()`. |
| FEAT-08 | Checkpoint token None assumption replaced with defensive error handling | All 13 `if let Some(token) = response.checkpoint_token()` sites silently ignore None. FEAT-08 says respond with DurableError when token is None on responses that require a token update. |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| thiserror | 2.0.18 | Derive `Error` on `DurableError` | Already in use; `.code()` is a hand-written method, not a derive |
| (none new) | — | No new dependencies needed | All patterns are pure Rust standard library |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::any::type_name | stdlib | Used in existing serialization errors | Reference pattern only |

**Installation:** No new crates required.

## Architecture Patterns

### Recommended Project Structure

All changes are within existing files:
```
crates/durable-lambda-core/src/
├── types.rs         # Add panic guards to StepOptions, CallbackOptions, MapOptions
├── error.rs         # Add .code() method, add error code constants
└── backend.rs       # Refactor is_retryable_error to use .code()
```

### Pattern 1: Builder Method Panic Guard (FEAT-01, FEAT-02, FEAT-03, FEAT-04)

**What:** Asserting preconditions inside builder setter methods with descriptive `panic!` messages.
**When to use:** When invalid option values would cause silent wrong behavior or confusing downstream errors.

```rust
// In types.rs — StepOptions
// IMPORTANT: Change signature from u32 to i32 so callers CAN pass -1.
// The u32 type currently prevents .retries(-1) from even compiling.
pub fn retries(mut self, count: i32) -> Self {
    assert!(
        count >= 0,
        "StepOptions::retries: count must be >= 0, got {}",
        count
    );
    self.retries = Some(count as u32);
    self
}

pub fn backoff_seconds(mut self, seconds: i32) -> Self {
    assert!(
        seconds >= 0,
        "StepOptions::backoff_seconds: seconds must be >= 0, got {}",
        seconds
    );
    self.backoff_seconds = Some(seconds);
    self
}

// In types.rs — CallbackOptions
pub fn timeout_seconds(mut self, seconds: i32) -> Self {
    assert!(
        seconds > 0,
        "CallbackOptions::timeout_seconds: seconds must be > 0, got {}",
        seconds
    );
    self.timeout_seconds = seconds;
    self
}

// In types.rs — MapOptions
pub fn batch_size(mut self, size: usize) -> Self {
    assert!(
        size > 0,
        "MapOptions::batch_size: size must be > 0, got {}",
        size
    );
    self.batch_size = Some(size);
    self
}
```

**Note on `StepOptions::retries` signature change:** The current signature `pub fn retries(mut self, count: u32)` means `StepOptions::new().retries(-1)` is a *compile error* today (Rust rejects negative literal as u32). The success criterion says it should *panic with a descriptive message* — so the signature must change to `i32`. The internal storage can remain `Option<u32>` with a safe cast after the guard.

**Note on `CallbackOptions` default:** Currently `CallbackOptions` default is 0 for both timeouts, and the docs say "0 means no timeout (default)". FEAT-02 says `timeout_seconds(0)` must panic. This means 0 should no longer be a valid value for the setter — users who want "no timeout" simply don't call the setter. The default (0) remains valid as the unset sentinel. This is a semantic change: the setter now enforces "if you set it, it must be positive".

### Pattern 2: `.code()` Method on DurableError (FEAT-05, FEAT-06)

**What:** A `pub fn code(&self) -> &str` method that matches on all DurableError variants and returns a stable uppercase string constant.
**When to use:** When callers need to match errors programmatically without depending on display string format.

```rust
// In error.rs — add after the constructor methods
impl DurableError {
    /// Return a stable, programmatic error code for this error variant.
    ///
    /// Codes are SCREAMING_SNAKE_CASE and stable across versions.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::error::DurableError;
    ///
    /// let err = DurableError::replay_mismatch("Step", "Wait", 0);
    /// assert_eq!(err.code(), "REPLAY_MISMATCH");
    /// ```
    pub fn code(&self) -> &str {
        match self {
            Self::ReplayMismatch { .. }       => "REPLAY_MISMATCH",
            Self::CheckpointFailed { .. }     => "CHECKPOINT_FAILED",
            Self::Serialization { .. }        => "SERIALIZATION",
            Self::Deserialization { .. }      => "DESERIALIZATION",
            Self::AwsSdk(_)                   => "AWS_SDK",
            Self::AwsSdkOperation(_)          => "AWS_SDK_OPERATION",
            Self::StepRetryScheduled { .. }   => "STEP_RETRY_SCHEDULED",
            Self::WaitSuspended { .. }        => "WAIT_SUSPENDED",
            Self::CallbackSuspended { .. }    => "CALLBACK_SUSPENDED",
            Self::CallbackFailed { .. }       => "CALLBACK_FAILED",
            Self::InvokeSuspended { .. }      => "INVOKE_SUSPENDED",
            Self::InvokeFailed { .. }         => "INVOKE_FAILED",
            Self::ParallelFailed { .. }       => "PARALLEL_FAILED",
            Self::MapFailed { .. }            => "MAP_FAILED",
            Self::ChildContextFailed { .. }   => "CHILD_CONTEXT_FAILED",
        }
    }
}
```

**Note on `#[non_exhaustive]`:** The enum has `#[non_exhaustive]`. The `match self` inside the impl block is *inside the defining crate* and does not need a wildcard arm — `#[non_exhaustive]` only affects downstream crates. This is safe and exhaustive as written.

### Pattern 3: Structured Retry Detection (FEAT-07)

**What:** Replace the current `is_retryable_error` string-scan with a match on error codes.
**When to use:** After `.code()` is added, string-matching on display output is fragile and should be replaced.

Current implementation in `backend.rs`:
```rust
fn is_retryable_error(err: &DurableError) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("throttl")
        || msg.contains("rate exceeded")
        || msg.contains("too many requests")
        || msg.contains("service unavailable")
        || msg.contains("internal server error")
        || msg.contains("timed out")
        || msg.contains("timeout")
}
```

Problem: This uses string matching on the *Display* output, which means:
- It can accidentally match unrelated errors that contain those words
- It depends on display message formatting being stable
- It doesn't match on the semantic error type

Replacement (after .code() exists):
```rust
fn is_retryable_error(err: &DurableError) -> bool {
    // Only AWS SDK operation errors can be transient. All other
    // DurableError variants (ReplayMismatch, Serialization, etc.) are
    // deterministic failures that should not be retried.
    match err {
        DurableError::AwsSdkOperation(source) => {
            // Still need to inspect the underlying AWS error message
            // since AwsSdkOperation boxes any StdError.
            let msg = source.to_string().to_lowercase();
            msg.contains("throttl")
                || msg.contains("rate exceeded")
                || msg.contains("too many requests")
                || msg.contains("service unavailable")
                || msg.contains("internal server error")
                || msg.contains("timed out")
                || msg.contains("timeout")
        }
        DurableError::AwsSdk(sdk_err) => {
            let msg = sdk_err.to_string().to_lowercase();
            msg.contains("throttl")
                || msg.contains("service unavailable")
                || msg.contains("timed out")
        }
        // All other variants are deterministic SDK errors — never retry.
        _ => false,
    }
}
```

**Key insight:** The current implementation would, in theory, retry a `DurableError::CheckpointFailed` if the wrapped error message happened to contain "timeout". Matching on the variant first ensures we only retry actual AWS transient failures, not replay mismatches or serialization errors that contain the word "timeout" in their context.

### Pattern 4: Defensive Checkpoint Token Handling (FEAT-08)

**What:** The current pattern `if let Some(token) = response.checkpoint_token() { self.set_checkpoint_token(token.to_string()); }` silently ignores a `None` checkpoint token in the response. FEAT-08 says this should return `DurableError` instead of silently continuing with a stale token.

**Current locations (13 sites across 6 files):**
- `operations/step.rs`: 4 sites
- `operations/callback.rs`: 1 site
- `operations/wait.rs`: 1 site
- `operations/map.rs`: 2 sites
- `operations/parallel.rs`: 2 sites
- `operations/child_context.rs`: 2 sites
- `operations/invoke.rs`: 1 site

**Replacement pattern:**
```rust
// Before
if let Some(token) = start_response.checkpoint_token() {
    self.set_checkpoint_token(token.to_string());
}

// After — return DurableError if token is absent
let new_token = start_response
    .checkpoint_token()
    .ok_or_else(|| DurableError::checkpoint_failed(
        name,
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "checkpoint response missing checkpoint_token",
        ),
    ))?;
self.set_checkpoint_token(new_token.to_string());
```

**Scope question:** The success criterion says "Checkpoint response with None checkpoint_token returns DurableError instead of panicking." Technically the current code doesn't panic on None — it silently continues. The intent is that a missing token is an error condition. Apply this defensive pattern consistently across all 13 sites.

### Anti-Patterns to Avoid

- **Changing `retries` to accept `u32` with a different guard:** The type `u32` prevents passing -1 *at compile time*. The requirement says it must *panic at runtime*. Change the parameter type to `i32`.
- **Using `Error::source()` chain for retry detection:** The retry detection must look at the variant first, then descend into the source only for `AwsSdkOperation` / `AwsSdk`.
- **Adding new `DurableError` variants for checkpoint-token-missing:** Use the existing `CheckpointFailed` variant — checkpoint_token missing from checkpoint response is a checkpoint failure.
- **Making `.code()` fallible:** Return `&'static str` (string literal), not `Result` or `String`. The match is exhaustive.
- **Applying FEAT-08 to replay (get_execution_state) calls:** `get_execution_state` doesn't return a checkpoint_token — only `checkpoint()` calls do. Only apply the defensive guard to `checkpoint()` responses.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Error code registry | Custom CodedError trait or macro | Plain `match self` method on DurableError | 13 variants; no indirection needed |
| Retry classification | External crate or HTTP status codes | Match on DurableError variant + source inspection | All retryable errors are already AwsSdkOperation |
| Input validation framework | Validation crate (validator, garde) | Direct `assert!` in setter | Builder pattern with panics is idiomatic Rust for invariants |

## Common Pitfalls

### Pitfall 1: `#[non_exhaustive]` Confusion Inside the Crate

**What goes wrong:** Developer adds wildcard `_ => unreachable!()` or `_ => "UNKNOWN"` to the `code()` match because they see `#[non_exhaustive]` on the enum.
**Why it happens:** `#[non_exhaustive]` prevents exhaustive matching *downstream* (in other crates). Inside the defining crate, the match must still be exhaustive and the compiler will enforce it.
**How to avoid:** Write an exhaustive match with no wildcard. The compiler will remind you to update `code()` when a new variant is added.
**Warning signs:** `_ =>` arm in `code()` within `error.rs`.

### Pitfall 2: Forgetting `retries` Signature Change

**What goes wrong:** Tests for FEAT-01 (`retries(-1)`) fail to compile because `retries` still takes `u32`.
**Why it happens:** `u32` literally cannot represent -1; the success criterion implies runtime panic, not compile error.
**How to avoid:** Change `pub fn retries(mut self, count: u32)` to `pub fn retries(mut self, count: i32)`. Update internal cast: `self.retries = Some(count as u32)` after the guard.
**Warning signs:** Compile errors in test code that calls `.retries(-1)`.

### Pitfall 3: Breaking the No-Timeout Convention for CallbackOptions

**What goes wrong:** Adding `assert!(seconds > 0)` in `timeout_seconds` breaks existing callers that pass 0 meaning "no timeout".
**Why it happens:** Current docs say `0 means no timeout`. FEAT-02 changes this to "must be > 0 if set". The zero-meaning-no-timeout behavior lives in the default (don't call the setter), not the setter itself.
**How to avoid:** Update docs: "If you want no timeout, do not call `timeout_seconds()`. This setter requires a positive value." Ensure no existing non-test production call path passes 0 intentionally (search the codebase).
**Warning signs:** Existing tests using `.timeout_seconds(0)` that are not marked as expected-panic.

### Pitfall 4: Applying FEAT-08 Too Broadly

**What goes wrong:** Applying the checkpoint-token None guard to `get_execution_state` responses, which correctly return no checkpoint token.
**Why it happens:** `get_execution_state` and `checkpoint` have different response shapes. Only `checkpoint` responses carry a `checkpoint_token`.
**How to avoid:** Only modify the 13 `if let Some(token) = response.checkpoint_token()` sites that follow `backend().checkpoint()` calls — not `get_execution_state` calls.
**Warning signs:** `get_execution_state` calls returning errors about missing checkpoint_token.

### Pitfall 5: is_retryable_error Tests Break After Refactor

**What goes wrong:** Existing backend tests (`is_retryable_detects_throttling`, `is_retryable_detects_timeout`, `is_retryable_rejects_non_transient`) fail after refactoring because the test constructs errors with `DurableError::checkpoint_failed(...)` containing the word "Throttling" — which the new code no longer matches (since `CheckpointFailed` maps to `_ => false`).
**Why it happens:** Tests in `backend.rs` currently verify that `is_retryable_error` catches "Throttling" in a `CheckpointFailed` error. The new logic won't match that.
**How to avoid:** Update the tests to use `DurableError::AwsSdkOperation(...)` wrapping the throttle error — which is the actual type produced during real AWS failures. The current test was testing the wrong variant.
**Warning signs:** `is_retryable_detects_throttling` passes before refactor but fails after.

## Code Examples

### Complete .code() Implementation

```rust
// Source: standard Rust pattern for typed error codes
impl DurableError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ReplayMismatch { .. }     => "REPLAY_MISMATCH",
            Self::CheckpointFailed { .. }   => "CHECKPOINT_FAILED",
            Self::Serialization { .. }      => "SERIALIZATION",
            Self::Deserialization { .. }    => "DESERIALIZATION",
            Self::AwsSdk(_)                 => "AWS_SDK",
            Self::AwsSdkOperation(_)        => "AWS_SDK_OPERATION",
            Self::StepRetryScheduled { .. } => "STEP_RETRY_SCHEDULED",
            Self::WaitSuspended { .. }      => "WAIT_SUSPENDED",
            Self::CallbackSuspended { .. }  => "CALLBACK_SUSPENDED",
            Self::CallbackFailed { .. }     => "CALLBACK_FAILED",
            Self::InvokeSuspended { .. }    => "INVOKE_SUSPENDED",
            Self::InvokeFailed { .. }       => "INVOKE_FAILED",
            Self::ParallelFailed { .. }     => "PARALLEL_FAILED",
            Self::MapFailed { .. }          => "MAP_FAILED",
            Self::ChildContextFailed { .. } => "CHILD_CONTEXT_FAILED",
        }
    }
}
```

### Test Pattern for Validation Panics

```rust
// Standard Rust pattern for testing panic messages
#[test]
#[should_panic(expected = "retries must be >= 0, got -1")]
fn step_options_rejects_negative_retries() {
    StepOptions::new().retries(-1);
}

#[test]
#[should_panic(expected = "timeout_seconds must be > 0, got 0")]
fn callback_options_rejects_zero_timeout() {
    CallbackOptions::new().timeout_seconds(0);
}

#[test]
#[should_panic(expected = "batch_size must be > 0, got 0")]
fn map_options_rejects_zero_batch() {
    MapOptions::new().batch_size(0);
}
```

### Test Pattern for .code()

```rust
#[test]
fn all_variants_have_unique_stable_codes() {
    let errors: Vec<(&str, DurableError)> = vec![
        ("REPLAY_MISMATCH", DurableError::replay_mismatch("A", "B", 0)),
        ("CHECKPOINT_FAILED", DurableError::checkpoint_failed("x",
            std::io::Error::new(std::io::ErrorKind::Other, "e"))),
        // ... all variants
    ];
    let mut codes = std::collections::HashSet::new();
    for (expected_code, err) in &errors {
        assert_eq!(err.code(), *expected_code);
        assert!(codes.insert(err.code()), "duplicate code: {}", err.code());
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| String-based error matching | Structured error codes | Phase 4 | Backend retry becomes variant-based, not display-string-based |
| Silent None checkpoint_token | Return DurableError | Phase 4 | Invalid AWS responses surface as typed errors instead of silent stale state |
| No runtime validation | Panic at setter call | Phase 4 | Invalid configs caught at construction, not buried in AWS errors |

**Deprecated/outdated:**
- `is_retryable_error` using `to_string().to_lowercase().contains(...)` — replaced with variant match.
- `if let Some(token) = response.checkpoint_token()` silent skip — replaced with `.ok_or_else(...)?.` error propagation.

## Open Questions

1. **Should `retries` parameter type change break the public API?**
   - What we know: `u32 -> i32` is a breaking change for callers who pass typed `u32` values (rare — most call `.retries(3)` with integer literals, which coerce to either type).
   - What's unclear: Are there downstream callers in wrapper crates or examples that pass `u32` variables?
   - Recommendation: Search all wrapper crates (`durable-lambda-{closure,trait,builder}`) and examples for `.retries(` calls before changing. The literal `3` coerces to `i32` fine; a typed `let count: u32 = 3; opts.retries(count)` would break.

2. **How broadly should FEAT-08 apply?**
   - What we know: There are 13 `if let Some(token) = response.checkpoint_token()` sites.
   - What's unclear: AWS SDK documentation on when `checkpoint_token` can legitimately be absent in a success response.
   - Recommendation: Apply the defensive guard to all 13 sites. If the AWS server sometimes omits the token legitimately, this will surface as test failures that clarify the contract.

3. **backoff_seconds validation: reject negative or clamp?**
   - What we know: `backoff_seconds` is `i32`. Negative values passed to the AWS API would be... unknown behavior.
   - What's unclear: Whether `-1` as backoff is meaningful (e.g., "use server default") in the AWS API.
   - Recommendation: Panic on negative backoff — if the AWS API has a "use default" sentinel, it should be expressed as `None` (i.e., not calling `backoff_seconds()`), not `-1`.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust's built-in `#[test]` + `#[tokio::test]` (tokio 1.50.0) |
| Config file | No separate config — `cargo test` discovers all `#[test]` items |
| Quick run command | `cargo test -p durable-lambda-core` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FEAT-01 | `StepOptions::new().retries(-1)` panics with message | unit | `cargo test -p durable-lambda-core step_options_rejects_negative_retries` | ❌ Wave 0 |
| FEAT-01 | `StepOptions::new().backoff_seconds(-1)` panics with message | unit | `cargo test -p durable-lambda-core step_options_rejects_negative_backoff` | ❌ Wave 0 |
| FEAT-01 | `StepOptions::new().retries(0)` succeeds (zero is valid) | unit | `cargo test -p durable-lambda-core step_options_accepts_zero_retries` | ❌ Wave 0 |
| FEAT-02 | `CallbackOptions::new().timeout_seconds(0)` panics | unit | `cargo test -p durable-lambda-core callback_options_rejects_zero_timeout` | ❌ Wave 0 |
| FEAT-02 | `CallbackOptions::new().timeout_seconds(1)` succeeds | unit | `cargo test -p durable-lambda-core callback_options_accepts_positive_timeout` | ❌ Wave 0 |
| FEAT-03 | `MapOptions::new().batch_size(0)` panics | unit | `cargo test -p durable-lambda-core map_options_rejects_zero_batch` | ❌ Wave 0 |
| FEAT-03 | `MapOptions::new().batch_size(1)` succeeds | unit | `cargo test -p durable-lambda-core map_options_accepts_positive_batch` | ❌ Wave 0 |
| FEAT-05 | `.code()` method exists on DurableError | unit | `cargo test -p durable-lambda-core error_code_method_exists` | ❌ Wave 0 |
| FEAT-06 | Each variant returns unique stable code | unit | `cargo test -p durable-lambda-core all_error_variants_have_unique_codes` | ❌ Wave 0 |
| FEAT-06 | `replay_mismatch(...).code()` returns `"REPLAY_MISMATCH"` | unit | `cargo test -p durable-lambda-core replay_mismatch_code` | ❌ Wave 0 |
| FEAT-07 | `is_retryable_error` does not match CheckpointFailed variants | unit | `cargo test -p durable-lambda-core is_retryable_ignores_non_aws_errors` | ❌ Wave 0 |
| FEAT-07 | `is_retryable_error` matches AwsSdkOperation with throttle source | unit | `cargo test -p durable-lambda-core is_retryable_detects_throttling` (update existing) | ✅ needs update |
| FEAT-08 | Checkpoint response with None token returns DurableError | unit | `cargo test -p durable-lambda-core checkpoint_none_token_returns_error` | ❌ Wave 0 |

All new tests belong in the existing `#[cfg(test)]` modules within their respective files (`types.rs`, `error.rs`, `backend.rs`, `operations/step.rs`).

### Sampling Rate

- **Per task commit:** `cargo test -p durable-lambda-core`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --all --check`

### Wave 0 Gaps

- [ ] Tests for validation panics in `crates/durable-lambda-core/src/types.rs` tests module — covers FEAT-01, FEAT-02, FEAT-03
- [ ] Tests for `.code()` in `crates/durable-lambda-core/src/error.rs` tests module — covers FEAT-05, FEAT-06
- [ ] Updated `is_retryable_detects_throttling` test in `backend.rs` to use `AwsSdkOperation` — covers FEAT-07
- [ ] New test `checkpoint_none_token_returns_error` in operation test modules — covers FEAT-08

## Sources

### Primary (HIGH confidence)
- Direct source code reading — `crates/durable-lambda-core/src/error.rs` (all 15 DurableError variants, constructor methods)
- Direct source code reading — `crates/durable-lambda-core/src/types.rs` (StepOptions, CallbackOptions, MapOptions, field types and signatures)
- Direct source code reading — `crates/durable-lambda-core/src/backend.rs` (is_retryable_error implementation, 13 checkpoint_token sites identified)
- Direct source code reading — `crates/durable-lambda-core/src/operations/step.rs`, `callback.rs`, `map.rs`, `parallel.rs`, `child_context.rs`, `wait.rs`, `invoke.rs`

### Secondary (MEDIUM confidence)
- Rust Reference on `#[non_exhaustive]` — behavior confirmed: exhaustive match is required within defining crate, wildcard only required in downstream crates

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all patterns are stdlib Rust idioms already in use in this codebase
- Architecture: HIGH — all changes are localized to two source files; patterns are direct and unambiguous
- Pitfalls: HIGH — pitfalls derived from direct code reading of existing types and test patterns

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (stable codebase; no external deps changing)
