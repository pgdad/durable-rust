# Phase 8: Macro & Builder Improvements - Research

**Researched:** 2026-03-17
**Domain:** Rust proc-macros (syn 2.0), trybuild compile-fail testing, builder pattern configuration
**Confidence:** HIGH

## Summary

Phase 8 has two cleanly separated workstreams: (1) extend the `#[durable_execution]` proc-macro's `validate_signature()` to check the second parameter type and return type at the token level, and (2) add `.with_tracing()` and `.with_error_handler()` configuration methods to `DurableHandlerBuilder`. The codebase already contains the correct hooks for both — `expand.rs:87-106` for the macro and `handler.rs` for the builder.

The proc-macro work is purely additive to `validate_signature()`. The second parameter check uses `syn::FnArg::Typed(PatType)` with path-segment string matching on the last segment (not full resolution). The return type check matches `syn::ReturnType::Type(_, boxed_type)` then checks for `Result` as the outermost type path. Two new trybuild compile-fail test files follow the existing `tests/ui/` pattern, each with a matching `.stderr` file generated via `TRYBUILD=overwrite`.

The builder work adds `Option` fields to `DurableHandlerBuilder`, self-consuming configuration methods, and applies them in `run()`. Tracing uses `tracing::subscriber::set_global_default(subscriber)` from the workspace-available `tracing` crate — no new dependencies needed. The error handler is `Box<dyn Fn(DurableError) -> DurableError + Send + Sync>` stored as an `Option`.

**Primary recommendation:** Extend `validate_signature()` in `expand.rs` with two new checks, add two compile-fail UI tests, then add two builder methods with `Option` storage — all fully backward compatible.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Macro type validation scope**
- Validate second parameter type contains "DurableContext" (string match on type path segments, not full resolution — proc-macros can't do type-level analysis)
- Validate return type is `Result<serde_json::Value, DurableError>` or similar pattern (check outer Result wrapper)
- Error messages should suggest the correct signature: `expected DurableContext, found {actual}`
- Validation is best-effort at the token level — exotic type aliases or re-exports won't be caught, and that's acceptable

**Trybuild compile-fail tests**
- Add `fail_wrong_param_type.rs` — function with 2 params but wrong types (e.g., `i32, i32`)
- Add `fail_wrong_return_type.rs` — function with correct params but returns `String` instead of `Result`
- Each test needs a matching `.stderr` file with expected error output
- Existing trybuild infrastructure in `crates/durable-lambda-macro/tests/` is reused

**Builder configuration methods**
- `.with_tracing(subscriber)` — installs a tracing subscriber before running the Lambda handler
- `.with_error_handler(fn)` — wraps handler errors through a custom function before returning to Lambda runtime
- Both are optional, builder works without them (backward compatible)
- Configuration stored as `Option<T>` fields on `DurableHandlerBuilder`
- `PhantomData<Fut>` already handles the future type parameter

### Claude's Discretion
- Exact type-matching heuristic for DurableContext (path segment match vs full path comparison)
- Whether `.with_tracing()` accepts `impl Subscriber` or `Box<dyn Subscriber>`
- Error handler signature: `Fn(DurableError) -> DurableError` vs `Fn(Box<dyn Error>) -> Box<dyn Error>`
- Whether to add a `.with_name(str)` for handler identification in logs

### Deferred Ideas (OUT OF SCOPE)
- Builder `.with_middleware(fn)` for request/response interception — too complex for v1
- Macro support for custom event types (not just `serde_json::Value`) — would require generic expansion changes
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FEAT-29 | `#[durable_execution]` validates second parameter is DurableContext type | syn `FnArg::Typed(PatType)` with last-segment ident match; `validate_signature()` in `expand.rs:87-106` is the extension point |
| FEAT-30 | `#[durable_execution]` validates return type is `Result<Value, DurableError>` | `ReturnType::Type(_, box_type)` + `Type::Path` last-segment "Result" check; same extension point |
| FEAT-31 | Compile-fail trybuild tests for wrong parameter types and return types | `tests/ui/` directory with `.rs` + `.stderr` pairs; glob pattern `fail_*.rs` already wired in `trybuild.rs` |
| FEAT-32 | `DurableHandlerBuilder` gains `.with_tracing(subscriber)` method | `tracing::subscriber::set_global_default()` before `lambda_runtime::run()`; no new dependency needed |
| FEAT-33 | `DurableHandlerBuilder` gains `.with_error_handler(fn)` method | `Option<Box<dyn Fn(DurableError) -> DurableError + Send + Sync>>` field; applied in `run()` after handler call |
| FEAT-34 | Tests verify custom configuration takes effect | Unit tests in `handler.rs` or integration test in `e2e-tests`; tracing subscriber verified via `tracing-test` crate |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| syn | 2.0.117 (workspace) | Parse Rust AST in proc-macro | Official parsing library for proc-macros |
| quote | 1.0.45 (workspace) | Generate Rust token streams | Standard companion to syn |
| proc-macro2 | 1.0.106 (workspace) | TokenStream type for syn/quote | Bridges proc-macro and syn worlds |
| tracing | 0.1.44 (workspace) | `Subscriber` trait + `set_global_default` | Already used in durable-lambda-core; no new dep |
| trybuild | 1.0 (dev, already in macro crate) | Compile-fail test harness | Already configured and passing |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing-subscriber | 0.3.23 (workspace) | Concrete subscriber implementations users will inject | Not a dep of builder crate itself; users bring their own |
| tracing-test | 0.2 (workspace, dev) | Verify tracing output in tests | FEAT-34 test verification for with_tracing |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `tracing::subscriber::set_global_default` | `tracing_subscriber::util::SubscriberInitExt::init()` | `init()` panics on double-init; `set_global_default` returns `Result` — safer for library code |
| `Box<dyn Fn(DurableError) -> DurableError + Send + Sync>` | Generic `F: Fn(DurableError) -> DurableError` on builder | Generic approach makes `DurableHandlerBuilder` have 3 type parameters instead of 2; Box erases the type, simpler struct signature |
| Last-segment ident match | Full path string comparison | Full path would require `DurableContext` to appear exactly as typed; last-segment is more robust to module paths |

**No new installations needed.** All dependencies are already in the workspace.

## Architecture Patterns

### Recommended Project Structure
```
crates/durable-lambda-macro/
├── src/
│   ├── lib.rs             # proc_macro entry point (no changes)
│   └── expand.rs          # extend validate_signature() HERE
└── tests/
    ├── trybuild.rs        # no changes — glob picks up new files
    └── ui/
        ├── fail_not_async.rs           # existing
        ├── fail_not_async.stderr       # existing
        ├── fail_wrong_param_count.rs   # existing
        ├── fail_wrong_param_count.stderr # existing
        ├── fail_wrong_param_type.rs    # NEW (FEAT-31)
        ├── fail_wrong_param_type.stderr # NEW (generated)
        ├── fail_wrong_return_type.rs   # NEW (FEAT-31)
        └── fail_wrong_return_type.stderr # NEW (generated)

crates/durable-lambda-builder/
└── src/
    └── handler.rs         # add Option fields + builder methods + run() integration
```

### Pattern 1: syn Type Path Segment Matching

**What:** Extract the last path segment of a `syn::Type::Path` and compare its identifier to a string.
**When to use:** Checking that a parameter or return type "looks like" a known type at the token level without full type resolution.
**Example:**
```rust
// Source: https://docs.rs/syn/latest/syn/
use syn::{FnArg, PatType, ReturnType, Type};

fn check_second_param_is_durable_context(func: &ItemFn) -> Result<(), Error> {
    // func.sig.inputs is Punctuated<FnArg, Comma>
    // index 1 is the second parameter (guaranteed 2 params by earlier check)
    let second = func.sig.inputs.iter().nth(1).unwrap();

    if let FnArg::Typed(PatType { ty, .. }) = second {
        if let Type::Path(type_path) = ty.as_ref() {
            if let Some(last_seg) = type_path.path.segments.last() {
                if last_seg.ident != "DurableContext" {
                    return Err(Error::new_spanned(
                        ty,
                        format!(
                            "#[durable_execution] second parameter must be DurableContext, \
                             found `{}` — expected signature: \
                             async fn handler(event: serde_json::Value, ctx: DurableContext) \
                             -> Result<serde_json::Value, DurableError>",
                            last_seg.ident
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}
```

### Pattern 2: Return Type Validation

**What:** Extract `syn::ReturnType::Type(_, boxed_type)`, check it is a `Type::Path` whose last segment is "Result".
**When to use:** Validating that the handler returns `Result<_, _>` at the token level.
**Example:**
```rust
// Source: https://docs.rs/syn/latest/syn/enum.ReturnType.html
fn check_return_type_is_result(func: &ItemFn) -> Result<(), Error> {
    match &func.sig.output {
        ReturnType::Default => {
            return Err(Error::new_spanned(
                &func.sig.fn_token,
                "#[durable_execution] must return Result<serde_json::Value, DurableError>, \
                 found implicit ()"
            ));
        }
        ReturnType::Type(_, boxed_type) => {
            let is_result = if let Type::Path(tp) = boxed_type.as_ref() {
                tp.path.segments.last()
                    .map(|s| s.ident == "Result")
                    .unwrap_or(false)
            } else {
                false
            };

            if !is_result {
                return Err(Error::new_spanned(
                    boxed_type.as_ref(),
                    "#[durable_execution] return type must be \
                     Result<serde_json::Value, DurableError> — found non-Result type. \
                     Expected: -> Result<serde_json::Value, DurableError>"
                ));
            }
        }
    }
    Ok(())
}
```

### Pattern 3: Builder Configuration with Option Fields

**What:** Store optional configuration as `Option<Box<dyn ...>>` on the builder; consume them in `run()`.
**When to use:** Adding backward-compatible configuration that callers opt into.
**Example:**
```rust
// Source: tracing docs.rs/tracing/0.1.44
use tracing::Subscriber;

pub struct DurableHandlerBuilder<F, Fut>
where
    F: Fn(serde_json::Value, BuilderContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    handler: F,
    _phantom: std::marker::PhantomData<Fut>,
    tracing_subscriber: Option<Box<dyn Subscriber + Send + Sync + 'static>>,
    error_handler: Option<Box<dyn Fn(DurableError) -> DurableError + Send + Sync>>,
}

impl<F, Fut> DurableHandlerBuilder<F, Fut>
where
    F: Fn(serde_json::Value, BuilderContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value, DurableError>> + Send,
{
    /// Install a custom tracing subscriber before starting the Lambda runtime.
    pub fn with_tracing<S>(mut self, subscriber: S) -> Self
    where
        S: Subscriber + Send + Sync + 'static,
    {
        self.tracing_subscriber = Some(Box::new(subscriber));
        self
    }

    /// Route handler errors through a custom function before returning to Lambda.
    pub fn with_error_handler<H>(mut self, handler: H) -> Self
    where
        H: Fn(DurableError) -> DurableError + Send + Sync + 'static,
    {
        self.error_handler = Some(Box::new(handler));
        self
    }
}
```

**In `run()`:**
```rust
pub async fn run(self) -> Result<(), lambda_runtime::Error> {
    // Install subscriber first, before any async work
    if let Some(subscriber) = self.tracing_subscriber {
        tracing::subscriber::set_global_default(subscriber)
            .expect("tracing subscriber already set — call with_tracing() only once");
    }

    let error_handler = self.error_handler;
    // ... existing AWS config setup ...

    lambda_runtime::run(service_fn(|event: LambdaEvent<serde_json::Value>| {
        let error_handler = &error_handler;
        async move {
            // ... existing invocation parsing ...
            let result = handler(invocation.user_event, builder_ctx).await;

            let result = match result {
                Ok(v) => Ok(v),
                Err(e) => {
                    let e = if let Some(h) = error_handler {
                        h(e)
                    } else {
                        e
                    };
                    Err(e)
                }
            };

            result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }))
    .await
}
```

### Pattern 4: trybuild Compile-Fail Test

**What:** A `.rs` file that must fail to compile, with a `.stderr` file capturing expected error output.
**When to use:** Testing that proc-macro validation rejects invalid input.
**Example:**
```rust
// crates/durable-lambda-macro/tests/ui/fail_wrong_param_type.rs
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(x: i32, y: i32) -> Result<serde_json::Value, durable_lambda_core::error::DurableError> {
    Ok(serde_json::json!({}))
}

fn main() {}
```

**The `.stderr` file is generated with:**
```bash
cd crates/durable-lambda-macro && TRYBUILD=overwrite cargo test --test trybuild
```
Then verify with `git diff` that the captured error matches what is expected.

### Anti-Patterns to Avoid

- **Full type path string matching:** Don't use `quote!(#ty).to_string() == "DurableContext"` — token stream whitespace is inconsistent. Use `last_seg.ident == "DurableContext"` instead.
- **Calling `set_global_default` inside `service_fn` closure:** The subscriber must be installed before `lambda_runtime::run()`, not per-invocation. It is a one-time global operation.
- **Making error handler `async`:** Keep the error handler `Fn(DurableError) -> DurableError` (sync). Async error handlers require `Box<dyn Future>` boxing which complicates the closure storage pattern significantly.
- **Adding `with_tracing` to `DurableHandlerBuilder` constructor `handler()`:** The builder pattern requires `handler()` to return a basic builder; configuration methods come after. Never set defaults that require `Option` — start with `None`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Token-level type matching | Custom string parsing of `TokenStream` | `syn::Type::Path` + `PathSegment.ident` | syn is the official AST — custom parsing breaks on edge cases |
| Compile-fail test harness | Custom `build.rs` or `rustc` subprocess | `trybuild` 1.0 | Already in dev-deps; handles rustc version differences, stderr normalization |
| Installing tracing globally | Custom thread-local state | `tracing::subscriber::set_global_default` | Official API with proper error handling on double-init |
| Proc-macro error reporting | `panic!` in macro code | `syn::Error::new_spanned` → `.to_compile_error()` | Produces proper compiler diagnostics with source spans |

**Key insight:** The proc-macro ecosystem (syn + quote + trybuild) is a mature, opinionated stack. Deviation from it produces subtle bugs with token whitespace, span information, and compiler version compatibility.

## Common Pitfalls

### Pitfall 1: `self` pattern in FnArg
**What goes wrong:** `func.sig.inputs.iter().nth(1)` might be `FnArg::Receiver` (i.e., `&self`) on methods, causing a panic when pattern matching `FnArg::Typed`.
**Why it happens:** The check only applies to free functions, but the pattern match might be incomplete.
**How to avoid:** The existing `validate_signature` already only accepts free functions (not associated methods), so the `FnArg::Typed` arm is safe. Still, use `if let FnArg::Typed(pt) = second` defensively.
**Warning signs:** Compiler warning about non-exhaustive match on `FnArg`.

### Pitfall 2: trybuild `.stderr` file contents change between Rust versions
**What goes wrong:** A `.stderr` file captured on rustc 1.75 fails to match on rustc 1.82 because error message formatting changed.
**Why it happens:** trybuild does exact string matching of compiler output.
**How to avoid:** Capture `.stderr` files on the same Rust toolchain used in CI. Use `TRYBUILD=overwrite` to regenerate after toolchain updates. The existing `.stderr` files in `tests/ui/` were captured on the current toolchain and pass (verified).
**Warning signs:** `trybuild` test fails with "stderr didn't match" after Rust upgrade.

### Pitfall 3: `set_global_default` panics on double-init
**What goes wrong:** If `with_tracing()` is called and a test already installed a subscriber, `run()` panics.
**Why it happens:** `set_global_default` returns `Err(SetGlobalDefaultError)` on second call; using `.expect()` turns it into a panic.
**How to avoid:** For tests, use `.try_init()` or ignore the error. Document that `.with_tracing()` must only be called once per process. In tests, avoid calling `run()` entirely (test builder methods independently).
**Warning signs:** Flaky test failures with "subscriber already set" panic.

### Pitfall 4: Error handler closure lifetime with `lambda_runtime::run` closure
**What goes wrong:** The `error_handler: Option<Box<dyn Fn(DurableError) -> DurableError + ...>>` is captured by reference in the `service_fn` closure, causing lifetime issues.
**Why it happens:** `lambda_runtime::run(service_fn(...))` requires the closure to be `'static`. A reference to `self.error_handler` is not `'static`.
**How to avoid:** Move `error_handler` out of `self` before the `lambda_runtime::run` call (like `let error_handler = self.error_handler;`), then capture by value via `move` or reference to the local. The existing `run()` method moves `self.handler` via `let handler = &self.handler` — follow the same pattern by moving `error_handler` into an `Arc` or using a separate local.
**Warning signs:** "does not live long enough" or "captured variable cannot escape" compile errors.

### Pitfall 5: Type path check misses `mut ctx: DurableContext`
**What goes wrong:** The second parameter might be `mut ctx: DurableContext`. The `PatType::ty` is still `DurableContext` — the `mut` is on the pattern, not the type. This is safe because `pat` (the binding pattern) is separate from `ty` (the type).
**Why it happens:** Misreading the syn AST structure.
**How to avoid:** Only inspect `PatType.ty`, never `PatType.pat`, for type validation. The `mut` keyword is part of the `Pat::Ident` pattern, not the type.

## Code Examples

### Extending validate_signature() — Full Pattern
```rust
// Source: syn 2.0 docs, https://docs.rs/syn/latest/syn/
use syn::{Error, FnArg, ItemFn, PatType, ReturnType, Type};

fn validate_signature(func: &ItemFn) -> Result<(), Error> {
    // Existing checks (async, param count) ...

    // NEW: Check second parameter type
    let second = func.sig.inputs.iter().nth(1).unwrap(); // safe: count already checked
    if let FnArg::Typed(PatType { ty, .. }) = second {
        let is_durable_context = if let Type::Path(tp) = ty.as_ref() {
            tp.path.segments.last()
                .map(|s| s.ident == "DurableContext")
                .unwrap_or(false)
        } else {
            false
        };

        if !is_durable_context {
            return Err(Error::new_spanned(
                ty.as_ref(),
                "#[durable_execution] second parameter must be DurableContext — \
                 expected: async fn handler(event: serde_json::Value, ctx: DurableContext) \
                 -> Result<serde_json::Value, DurableError>",
            ));
        }
    }

    // NEW: Check return type
    if let ReturnType::Type(_, boxed_type) = &func.sig.output {
        let is_result = if let Type::Path(tp) = boxed_type.as_ref() {
            tp.path.segments.last()
                .map(|s| s.ident == "Result")
                .unwrap_or(false)
        } else {
            false
        };

        if !is_result {
            return Err(Error::new_spanned(
                boxed_type.as_ref(),
                "#[durable_execution] return type must be \
                 Result<serde_json::Value, DurableError> — \
                 expected: -> Result<serde_json::Value, DurableError>",
            ));
        }
    } else {
        // ReturnType::Default — no explicit return type
        return Err(Error::new_spanned(
            &func.sig.fn_token,
            "#[durable_execution] must explicitly return \
             Result<serde_json::Value, DurableError>",
        ));
    }

    Ok(())
}
```

### Compile-Fail Test: Wrong Parameter Type
```rust
// File: crates/durable-lambda-macro/tests/ui/fail_wrong_param_type.rs
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(x: i32, y: i32) -> Result<serde_json::Value, durable_lambda_core::error::DurableError> {
    Ok(serde_json::json!({}))
}

fn main() {}
```

### Compile-Fail Test: Wrong Return Type
```rust
// File: crates/durable-lambda-macro/tests/ui/fail_wrong_return_type.rs
use durable_lambda_core::context::DurableContext;
use durable_lambda_macro::durable_execution;

#[durable_execution]
async fn handler(event: serde_json::Value, ctx: DurableContext) -> String {
    String::from("bad return type")
}

fn main() {}
```

### Test for `with_tracing` Effect (FEAT-34)
```rust
// In crates/durable-lambda-builder/src/handler.rs #[cfg(test)]
// or in tests/e2e/tests/builder_config.rs

#[test]
fn test_with_tracing_builder_stores_subscriber() {
    // Verify the builder stores the subscriber (type-level test)
    let builder = handler(|_event, _ctx| async move {
        Ok(serde_json::json!({}))
    })
    .with_tracing(tracing_subscriber::fmt().finish());

    // If this compiles, with_tracing() accepts a Subscriber impl.
    // Actual subscriber installation tested via set_global_default behavior.
    drop(builder);
}

#[test]
fn test_with_error_handler_transforms_error() {
    // Verify error handler closure is stored and invocable
    let custom_err = DurableError::step_retry_scheduled("test");
    let expected_code = custom_err.code().to_string();

    let handler_fn = |e: DurableError| {
        // Transform error (e.g., add context)
        e // identity transform for test
    };

    let _builder = handler(|_event, _ctx| async move {
        Ok(serde_json::json!({}))
    })
    .with_error_handler(handler_fn);
    // Type check only — run() not called in unit tests
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `proc_macro::TokenStream` string manipulation | `syn` AST parsing | syn 1.0+ | Type-safe, span-preserving errors |
| trybuild manual stderr files | `TRYBUILD=overwrite` to regenerate | trybuild 0.1.x+ | Eliminates manual error text maintenance |
| Async fn in traits via `async-trait` macro | RPITIT (native Rust 1.75+) | Rust 1.75 (2023) | Already used in this project |

**Deprecated/outdated:**
- `darling` crate for attribute parsing: Not needed here — no custom macro attributes, only function signature validation.
- `proc_macro_error` crate: project already uses `syn::Error::new_spanned` consistently — keep that pattern.

## Open Questions

1. **tracing-subscriber as dev-dependency in builder crate**
   - What we know: `tracing-subscriber = "0.3.23"` is in workspace deps but not in builder's `Cargo.toml`
   - What's unclear: Whether `tracing-test` (already in `durable-lambda-core` dev-deps) can be used in builder tests, or whether it needs to be added to builder's dev-deps
   - Recommendation: Add `tracing-subscriber = { workspace = true }` and `tracing-test = { workspace = true }` to `durable-lambda-builder`'s `[dev-dependencies]` for FEAT-34 tests

2. **Error handler closure in `service_fn` capture**
   - What we know: `service_fn` closure must be `'static`; `self.error_handler` is `Option<Box<dyn Fn...>>`
   - What's unclear: Whether moving `self.error_handler` into a local before `lambda_runtime::run` and capturing it by reference (same pattern as `let handler = &self.handler`) satisfies the lifetime
   - Recommendation: Move into an `Arc<Option<...>>` if reference capture doesn't compile; the existing `backend.clone()` pattern shows Arc is the established pattern for shared ownership across invocations

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test + trybuild 1.0 |
| Config file | none — Cargo.toml `[dev-dependencies]` |
| Quick run command | `cargo test -p durable-lambda-macro --test trybuild` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FEAT-29 | Second param must be DurableContext | unit (expand.rs) + compile-fail | `cargo test -p durable-lambda-macro` | Partial — unit tests exist, compile-fail needs new files |
| FEAT-30 | Return type must be Result | unit (expand.rs) + compile-fail | `cargo test -p durable-lambda-macro` | Partial — unit tests exist, compile-fail needs new files |
| FEAT-31 | trybuild compile-fail tests | compile-fail (trybuild) | `cargo test -p durable-lambda-macro --test trybuild` | ❌ Wave 0: `fail_wrong_param_type.rs`, `fail_wrong_param_type.stderr`, `fail_wrong_return_type.rs`, `fail_wrong_return_type.stderr` |
| FEAT-32 | with_tracing() installs subscriber | unit (builder) | `cargo test -p durable-lambda-builder` | ❌ Wave 0: new test in handler.rs |
| FEAT-33 | with_error_handler() routes errors | unit (builder) | `cargo test -p durable-lambda-builder` | ❌ Wave 0: new test in handler.rs |
| FEAT-34 | Config methods take effect | integration/unit | `cargo test -p durable-lambda-builder` | ❌ Wave 0: verify subscriber stored; verify error handler called |

### Sampling Rate
- **Per task commit:** `cargo test -p durable-lambda-macro && cargo test -p durable-lambda-builder`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/durable-lambda-macro/tests/ui/fail_wrong_param_type.rs` — covers FEAT-29, FEAT-31
- [ ] `crates/durable-lambda-macro/tests/ui/fail_wrong_param_type.stderr` — generated via `TRYBUILD=overwrite`
- [ ] `crates/durable-lambda-macro/tests/ui/fail_wrong_return_type.rs` — covers FEAT-30, FEAT-31
- [ ] `crates/durable-lambda-macro/tests/ui/fail_wrong_return_type.stderr` — generated via `TRYBUILD=overwrite`
- [ ] New unit tests in `crates/durable-lambda-builder/src/handler.rs` — covers FEAT-32, FEAT-33, FEAT-34
- [ ] `tracing-subscriber` and `tracing-test` in `durable-lambda-builder` `[dev-dependencies]` — for FEAT-34 subscriber tests

## Sources

### Primary (HIGH confidence)
- `crates/durable-lambda-macro/src/expand.rs` — current `validate_signature()` implementation; extension point confirmed
- `crates/durable-lambda-builder/src/handler.rs` — `DurableHandlerBuilder` struct and `run()` method; `Option` field pattern confirmed
- `crates/durable-lambda-macro/tests/ui/` — 2 existing compile-fail tests with `.stderr` files; pattern confirmed working
- [syn 2.0 ReturnType](https://docs.rs/syn/latest/syn/enum.ReturnType.html) — `ReturnType::Type(RArrow, Box<Type>)` variant confirmed
- [tracing set_global_default](https://docs.rs/tracing/latest/tracing/subscriber/fn.set_global_default.html) — `fn set_global_default<S: Subscriber + Send + Sync + 'static>(subscriber: S) -> Result<(), SetGlobalDefaultError>` confirmed

### Secondary (MEDIUM confidence)
- [syn TypePath path segments](https://docs.rs/syn/latest/syn/) — `type_path.path.segments.last().ident` pattern for type name checking; confirmed via docs
- [trybuild TRYBUILD=overwrite](https://docs.rs/trybuild/latest/trybuild/) — env var for regenerating .stderr files; confirmed in docs
- [SubscriberInitExt](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/util/trait.SubscriberInitExt.html) — `init()` vs `try_init()` vs `set_global_default()` distinction confirmed

### Tertiary (LOW confidence)
- Error handler closure lifetime with `service_fn` — inferred from existing `handler` field capture pattern; needs compile-time verification during implementation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already in workspace, versions confirmed
- Architecture: HIGH — extension points (`validate_signature`, `DurableHandlerBuilder`) confirmed in codebase; syn type traversal patterns confirmed in docs
- Pitfalls: HIGH for macro pitfalls (verified against existing code); MEDIUM for builder closure lifetime (inferred)

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (stable ecosystem — syn 2.x, tracing 0.1.x are not fast-moving)
