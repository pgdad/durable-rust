# Phase 3: Shared Context Trait - Research

**Researched:** 2026-03-16
**Domain:** Rust trait design, async generics, cross-crate abstraction, code deduplication
**Confidence:** HIGH

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| ARCH-01 | DurableContextOps trait with all 44 context methods | Audit confirms 21 methods per wrapper; trait design pattern identified |
| ARCH-02 | ClosureContext implements DurableContextOps via delegation | ClosureContext already has all 21 methods delegating to `inner`; trivial impl |
| ARCH-03 | TraitContext implements DurableContextOps via delegation | TraitContext already has all 21 methods delegating to `inner`; trivial impl |
| ARCH-04 | BuilderContext implements DurableContextOps via delegation | BuilderContext already has all 21 methods delegating to `inner`; trivial impl |
| ARCH-05 | Generic handler functions accepting `impl DurableContextOps` work across all approaches | Requires DurableContext itself to also implement the trait; static dispatch confirmed viable |
| ARCH-06 | Handler boilerplate extraction — shared setup_lambda_runtime() function | ~35 lines of identical boilerplate across 3 handler files + macro expand.rs; shared function in core |
</phase_requirements>

---

## Summary

Phase 3 eliminates ~2,400 lines of near-identical delegation code by introducing a `DurableContextOps` trait that all four context types implement. The codebase already contains three wrapper types (ClosureContext, TraitContext, BuilderContext) plus the core DurableContext, each with identical sets of 21 public methods that all delegate to the same inner `DurableContext`. The trait extraction is a pure mechanical refactoring — no behavioral changes.

The trait must handle generic async methods (`step<T, E, F, Fut>`, `step_with_options`, `invoke`, `parallel`, `child_context`, `map`), which requires Rust's native async-fn-in-traits (RPITIT, stabilized in Rust 1.75+) or the `async-trait` crate. Because the target is **static dispatch** (`impl DurableContextOps` bounds, not `dyn`), native async fn in traits works cleanly. The project is on Rust 1.94 and already uses `async-trait = "0.1"` for `DurableBackend` and `DurableHandler`, so either approach is technically available. However, since `DurableContextOps` is for generic bounds only (never object safety), native async fn in traits is preferred as it avoids boxing overhead.

The handler boilerplate (AWS config init, event extraction, `DurableContext` construction) is duplicated across three handler files plus `expand.rs` in the macro crate. Extracting a shared `setup_lambda_invocation` function into `durable-lambda-core::event` or a new `durable-lambda-core::setup` module eliminates ~100 lines of duplication across 4 locations.

**Primary recommendation:** Define `DurableContextOps` in `durable-lambda-core::ops_trait`, implement it on `DurableContext` and all three wrapper types using native async fn in traits (no `#[async_trait]` needed for this trait), and extract handler boilerplate into a `durable-lambda-core::event::parse_invocation` function that returns a structured `InvocationData` type.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust native async fn in traits | 1.75+ (project is on 1.94) | `async fn` in trait definitions without boxing | Stabilized, zero overhead, no macro needed |
| `async-trait = "0.1"` | already in workspace | Alternative if dynamic dispatch needed | Already present; use only if object safety required |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde::Serialize + DeserializeOwned` | already in workspace | Generic bounds on step/map/parallel | Required for all operation type parameters |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Native async fn in traits | `#[async_trait]` macro | async-trait boxes futures (runtime cost); only needed for dyn/object safety — unnecessary here |
| Single `DurableContextOps` trait | Separate `StepOps + WaitOps + ...` traits | Splitting traits makes generic bounds verbose; single trait is simpler for users |
| Trait in `durable-lambda-core` | Trait in each wrapper crate | Core is the authoritative crate; wrapper crates depend on core, not vice-versa |

**Installation:**

No new dependencies needed. The trait uses only items already in workspace.

---

## Architecture Patterns

### Where the Trait Lives

The `DurableContextOps` trait must live in `durable-lambda-core` because:

1. `DurableContext` lives in `durable-lambda-core::context`
2. All wrapper crates depend on core (one-way dependency graph is documented in CLAUDE.md)
3. Wrapper crates cannot be referenced from core

Recommended path: `crates/durable-lambda-core/src/ops_trait.rs`

### Recommended Project Structure After Phase 3

```
crates/durable-lambda-core/src/
├── backend.rs         # DurableBackend trait (unchanged)
├── context.rs         # DurableContext struct (unchanged)
├── error.rs           # DurableError (unchanged)
├── event.rs           # parse_operations, extract_user_event + NEW: parse_invocation()
├── lib.rs             # Add pub use ops_trait::DurableContextOps
├── ops_trait.rs       # NEW: DurableContextOps trait definition + impl for DurableContext
├── operation_id.rs    # (unchanged)
├── operations/        # (unchanged)
├── replay.rs          # (unchanged)
└── types.rs           # (unchanged)

crates/durable-lambda-closure/src/
├── context.rs         # ClosureContext: ADD impl DurableContextOps
├── handler.rs         # Use shared parse_invocation() from core
├── lib.rs             # Re-export DurableContextOps in prelude
└── prelude.rs         # Add DurableContextOps to exports

# (same changes for durable-lambda-trait and durable-lambda-builder)

tests/parity/tests/
└── parity.rs          # Add generic handler parity tests
```

### Pattern 1: Native Async Fn in Trait with Generic Methods

The `DurableContextOps` trait declares all operation methods using native Rust async fn syntax. Generic methods use `where` bounds directly on the method.

```rust
// File: crates/durable-lambda-core/src/ops_trait.rs
// Source: Rust Reference, RPITIT stabilized Rust 1.75

use std::future::Future;
use serde::{Serialize, de::DeserializeOwned};
use crate::error::DurableError;
use crate::types::{
    BatchResult, CallbackHandle, CallbackOptions, ExecutionMode,
    MapOptions, ParallelOptions, StepOptions,
};

/// Shared interface for all durable context types.
///
/// Implemented by `DurableContext`, `ClosureContext`, `TraitContext`,
/// and `BuilderContext`. Enables generic handler code that works with
/// any context type.
///
/// # Examples
///
/// ```no_run
/// use durable_lambda_core::ops_trait::DurableContextOps;
/// use durable_lambda_core::error::DurableError;
/// use serde_json::Value;
///
/// async fn business_logic<C: DurableContextOps>(ctx: &mut C) -> Result<Value, DurableError> {
///     let result: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await?;
///     Ok(serde_json::json!({"result": result.unwrap()}))
/// }
/// ```
pub trait DurableContextOps {
    // Operation methods (async)
    fn step<T, E, F, Fut>(
        &mut self,
        name: &str,
        f: F,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send,
        E: Serialize + DeserializeOwned + Send,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send;

    fn step_with_options<T, E, F, Fut>(
        &mut self,
        name: &str,
        options: StepOptions,
        f: F,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send,
        E: Serialize + DeserializeOwned + Send,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send;

    fn wait(&mut self, name: &str, duration_secs: i32)
        -> impl Future<Output = Result<(), DurableError>> + Send;

    fn create_callback(
        &mut self,
        name: &str,
        options: CallbackOptions,
    ) -> impl Future<Output = Result<CallbackHandle, DurableError>> + Send;

    fn callback_result<T: DeserializeOwned>(
        &self,
        handle: &CallbackHandle,
    ) -> Result<T, DurableError>;

    fn invoke<T, P>(
        &mut self,
        name: &str,
        function_name: &str,
        payload: &P,
    ) -> impl Future<Output = Result<T, DurableError>> + Send
    where
        T: DeserializeOwned,
        P: Serialize;

    fn parallel<T, F, Fut>(
        &mut self,
        name: &str,
        branches: Vec<F>,
        options: ParallelOptions,
    ) -> impl Future<Output = Result<BatchResult<T>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce(crate::context::DurableContext) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, DurableError>> + Send + 'static;

    fn child_context<T, F, Fut>(
        &mut self,
        name: &str,
        f: F,
    ) -> impl Future<Output = Result<T, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send,
        F: FnOnce(crate::context::DurableContext) -> Fut + Send,
        Fut: Future<Output = Result<T, DurableError>> + Send;

    fn map<T, I, F, Fut>(
        &mut self,
        name: &str,
        items: Vec<I>,
        options: MapOptions,
        f: F,
    ) -> impl Future<Output = Result<BatchResult<T>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        I: Send + 'static,
        F: FnOnce(I, crate::context::DurableContext) -> Fut + Send + 'static + Clone,
        Fut: Future<Output = Result<T, DurableError>> + Send + 'static;

    // State query methods (sync)
    fn execution_mode(&self) -> ExecutionMode;
    fn is_replaying(&self) -> bool;
    fn arn(&self) -> &str;
    fn checkpoint_token(&self) -> &str;

    // Log methods (sync)
    fn log(&self, message: &str);
    fn log_with_data(&self, message: &str, data: &serde_json::Value);
    fn log_debug(&self, message: &str);
    fn log_warn(&self, message: &str);
    fn log_error(&self, message: &str);
    fn log_debug_with_data(&self, message: &str, data: &serde_json::Value);
    fn log_warn_with_data(&self, message: &str, data: &serde_json::Value);
    fn log_error_with_data(&self, message: &str, data: &serde_json::Value);
}
```

### Pattern 2: Implementation on DurableContext

`DurableContext` already has all 21 methods. The implementation just delegates each trait method to `self.method()`:

```rust
// In crates/durable-lambda-core/src/ops_trait.rs (same file, below trait def)
// Source: Direct code analysis

impl DurableContextOps for DurableContext {
    fn step<T, E, F, Fut>(
        &mut self, name: &str, f: F,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where
        T: Serialize + DeserializeOwned + Send,
        E: Serialize + DeserializeOwned + Send,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        DurableContext::step(self, name, f)
    }

    // ... (same delegation pattern for all 21 methods)
    fn execution_mode(&self) -> ExecutionMode { self.execution_mode() }
    fn is_replaying(&self) -> bool { self.is_replaying() }
    fn arn(&self) -> &str { self.arn() }
    // etc.
}
```

### Pattern 3: Implementation on Wrapper Types

Each wrapper context already has all 21 methods that delegate to `self.inner`. The trait implementation is equally trivial:

```rust
// In crates/durable-lambda-closure/src/context.rs
// Source: Direct code analysis

impl durable_lambda_core::ops_trait::DurableContextOps for ClosureContext {
    fn step<T, E, F, Fut>(
        &mut self, name: &str, f: F,
    ) -> impl Future<Output = Result<Result<T, E>, DurableError>> + Send
    where ...
    {
        self.inner.step(name, f)  // delegates to DurableContext::step
    }

    fn execution_mode(&self) -> ExecutionMode { self.inner.execution_mode() }
    // ...
}
```

### Pattern 4: Handler Boilerplate Extraction

The event-extraction block duplicated across all 4 handler locations should become a shared type and function:

```rust
// New in crates/durable-lambda-core/src/event.rs

/// Structured data extracted from a Lambda invocation payload.
pub struct InvocationData {
    pub durable_execution_arn: String,
    pub checkpoint_token: String,
    pub operations: Vec<aws_sdk_lambda::types::Operation>,
    pub next_marker: Option<String>,
    pub user_event: serde_json::Value,
}

/// Parse all durable execution fields from a Lambda event payload.
///
/// Extracts ARN, checkpoint token, initial operations, pagination marker,
/// and user event from the standard durable Lambda event envelope.
///
/// # Errors
///
/// Returns `Err(&'static str)` if required fields are missing.
pub fn parse_invocation(
    payload: &serde_json::Value,
) -> Result<InvocationData, &'static str> {
    let durable_execution_arn = payload["DurableExecutionArn"]
        .as_str()
        .ok_or("missing DurableExecutionArn in event")?
        .to_string();
    let checkpoint_token = payload["CheckpointToken"]
        .as_str()
        .ok_or("missing CheckpointToken in event")?
        .to_string();
    let initial_state = &payload["InitialExecutionState"];
    let operations = parse_operations(initial_state);
    let next_marker = initial_state["NextMarker"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let user_event = extract_user_event(initial_state);
    Ok(InvocationData { durable_execution_arn, checkpoint_token, operations, next_marker, user_event })
}
```

### Pattern 5: Generic Handler Functions (ARCH-05 Verification)

The success criterion requires `async fn logic<C: DurableContextOps>(ctx: &mut C)` to work. This compiles directly with the static dispatch approach:

```rust
// Source: Rust type system analysis

async fn logic<C: DurableContextOps>(ctx: &mut C) -> Result<serde_json::Value, DurableError> {
    let result: Result<i32, String> = ctx.step("validate", || async { Ok(42) }).await?;
    Ok(serde_json::json!({"result": result}))
}

// Compiles with all 4 context types:
// logic(&mut durable_ctx).await       // DurableContext (macro approach)
// logic(&mut closure_ctx).await       // ClosureContext (closure approach)
// logic(&mut trait_ctx).await         // TraitContext (trait approach)
// logic(&mut builder_ctx).await       // BuilderContext (builder approach)
```

### Anti-Patterns to Avoid

- **Using `dyn DurableContextOps`:** The trait has generic methods which are not object-safe. Never use `Box<dyn DurableContextOps>` or `Arc<dyn DurableContextOps>`. Static dispatch only.
- **Using `#[async_trait]` on DurableContextOps:** Not needed since we're using static dispatch. Adds boxing overhead for no benefit.
- **Removing the wrapper struct methods:** Do not remove `pub fn step(...)` etc. from ClosureContext/TraitContext/BuilderContext. The trait `impl` delegates to them; the existing public API is unchanged.
- **Putting the trait in wrapper crates:** The dependency graph prohibits wrapper crates from being imported by core. Trait belongs in core.
- **Duplicating the trait per prelude:** Each wrapper prelude should re-export `DurableContextOps` from core, not define its own.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Generic async methods in trait | Custom wrapper/erased type | Native async fn in traits (Rust 1.75+) | Rust supports this natively since 1.75; no boxing needed for static dispatch |
| Handler boilerplate dedup | Custom macro | Plain function returning struct | Functions are simpler and testable; macro is overkill for 4 call sites |
| Trait method body reuse | Manual copy-paste into each impl | Delegation `self.inner.method()` or `DurableContext::method(self, ...)` | All wrapper types already have delegation in place |

**Key insight:** Every wrapper context already does `self.inner.step(name, f)`. The trait impl for each wrapper is literally wrapping those existing delegations one more time. This is the simplest possible refactoring.

---

## Common Pitfalls

### Pitfall 1: Return-position `impl Trait` in trait methods requires careful naming

**What goes wrong:** When writing `fn step(...) -> impl Future<Output = ...> + Send` in a trait, the compiler needs to know the type is `Send`. Without careful bounds, complex generic chains can produce confusing "the future is not Send" errors.

**Why it happens:** Rust RPITIT captures all lifetimes and types in the returned opaque future. If any input generic is not `Send`, the future may not be `Send` either.

**How to avoid:** Ensure all closure/future generics are explicitly bounded with `+ Send` (already the case in the existing DurableContext method signatures). Copy the exact where-clauses from the existing DurableContext implementations.

**Warning signs:** Compiler error about "future is not Send" or "opaque type does not satisfy bounds".

### Pitfall 2: Method name collision between trait impl and inherent impl

**What goes wrong:** ClosureContext has `pub fn step(...)` as an inherent method. When `impl DurableContextOps for ClosureContext` also defines `fn step(...)`, callers get the inherent method via UFCS, not the trait method. This is expected behavior but can cause confusion.

**Why it happens:** Rust prefers inherent methods over trait methods for direct calls. Trait methods require qualified syntax or a `use DurableContextOps` import.

**How to avoid:** Keep inherent methods in place (do NOT remove them). The trait impl delegates to the inherent impl. Generic callers use the trait. Concrete callers use the inherent impl. Both coexist correctly.

**Warning signs:** Removing inherent methods to "avoid duplication" — this breaks backward compatibility for users of the concrete types.

### Pitfall 3: `invoke`/`parallel`/`map` generic parameter mismatch

**What goes wrong:** The `parallel` and `map` methods take closures that accept `DurableContext` (the core type) even when called on ClosureContext/TraitContext/BuilderContext. The trait must match this signature exactly.

**Why it happens:** The design decision is that child contexts (passed to parallel/map/child_context branches) are always `DurableContext` — not the wrapper type. The trait methods must reflect this.

**How to avoid:** In the `DurableContextOps` trait, `parallel` and `map` branch closures take `DurableContext` parameter, not `impl DurableContextOps`. Copy the exact signatures from the existing wrapper contexts.

**Warning signs:** Type mismatch errors where callers try to pass `|mut ctx: ClosureContext|` to a `parallel` call on `ClosureContext`.

### Pitfall 4: Forgetting `DurableContext` also needs the trait impl (for macro approach)

**What goes wrong:** Only implementing the trait for the three wrappers. Success criterion 3 requires the generic function to work with all 4 context types, including bare `DurableContext`.

**Why it happens:** `DurableContext` is not a "wrapper" — it IS the core type. Easy to overlook.

**How to avoid:** Implement `DurableContextOps` for `DurableContext` in `ops_trait.rs` alongside the trait definition.

**Warning signs:** Parity tests that test `DurableContext` directly fail with "does not implement DurableContextOps".

### Pitfall 5: Boilerplate extraction breaks `expand.rs` (macro approach)

**What goes wrong:** The boilerplate in `expand.rs` generates code using fully-qualified paths like `::durable_lambda_core::event::parse_operations`. If `parse_invocation` is added to `event.rs`, `expand.rs` must be updated to use it — but the generated code must still compile for users who don't import the new function explicitly.

**Why it happens:** Proc-macros emit source text that is compiled in the user's crate. All referenced paths must be valid there.

**How to avoid:** Update `expand.rs` to use `::durable_lambda_core::event::parse_invocation` with the same fully-qualified path style already used in the file. Test by running the existing trybuild tests.

---

## Code Examples

### Exact method inventory (21 methods per context wrapper)

Verified by direct code inspection of `crates/durable-lambda-closure/src/context.rs`:

```
Async operation methods (9):
  step, step_with_options, wait, create_callback, invoke, parallel, child_context, map
  + callback_result (sync despite being operation-related)

State query methods (4):
  execution_mode, is_replaying, arn, checkpoint_token

Log methods (8):
  log, log_with_data, log_debug, log_warn, log_error,
  log_debug_with_data, log_warn_with_data, log_error_with_data

Total: 21 public methods per wrapper context
```

Note: The requirement document says "44 methods" but the actual count is **21 public methods** per wrapper type. The "44" is likely a rough estimate from the project analysis phase. The planner should plan for 21 methods.

### Handler boilerplate that repeats in all 4 locations

Identical block (verified by reading all 3 handler.rs files + expand.rs):

```rust
// Source: closure/handler.rs lines 77-100, trait/handler.rs lines 130-165,
//         builder/handler.rs lines 129-162, macro/expand.rs lines 36-75
let durable_execution_arn = payload["DurableExecutionArn"]
    .as_str()
    .ok_or("missing DurableExecutionArn in event")?
    .to_string();
let checkpoint_token = payload["CheckpointToken"]
    .as_str()
    .ok_or("missing CheckpointToken in event")?
    .to_string();
let initial_state = &payload["InitialExecutionState"];
let operations = parse_operations(initial_state);
let next_marker = initial_state["NextMarker"]
    .as_str()
    .filter(|s| !s.is_empty())
    .map(|s| s.to_string());
let user_event = extract_user_event(initial_state);
let durable_ctx = DurableContext::new(
    backend, durable_execution_arn, checkpoint_token, operations, next_marker,
).await
 .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
```

This exact block (approximately 20 lines) appears in all 4 handler locations.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `#[async_trait]` required for async fn in traits | Native async fn in traits (RPITIT) | Rust 1.75 (Dec 2023) | No boxing for static dispatch; simpler code |
| `Box<dyn Fn>` for generic trait methods | `impl Trait` in return position | Rust 1.75 | Zero-cost static dispatch |

**Deprecated/outdated:**

- Using `#[async_trait]` for `DurableContextOps`: Not needed since we need static dispatch only. Reserve `#[async_trait]` for `DurableBackend` and `DurableHandler` which ARE used as `dyn Trait`.

---

## Open Questions

1. **The "44 methods" discrepancy**
   - What we know: Each wrapper context has exactly 21 public methods (confirmed by code audit)
   - What's unclear: The REQUIREMENTS.md and ROADMAP.md both say "44 context methods" — this may count the wrapper-specific constructors, `pub(crate)` methods, or the 21 methods × 2 (DurableContext + wrapper) = 42 ≈ 44
   - Recommendation: Plan the trait with the 21 verified methods. If there are additional methods to include, the planner will discover them during implementation.

2. **Should `MacroContext`/`DurableContext` also go in the prelude exports?**
   - What we know: Success criterion requires `DurableContext` to implement the trait; preludes currently don't export `DurableContext`
   - What's unclear: Whether to add `DurableContextOps` to wrapper preludes only or also to `durable-lambda-core`'s public API
   - Recommendation: Add `pub use ops_trait::DurableContextOps` to `durable-lambda-core::lib.rs` and re-export it from each wrapper prelude. `DurableContext` already in each prelude as needed.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (tokio::test for async) |
| Config file | Cargo.toml workspace members |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ARCH-01 | `DurableContextOps` trait compiles with all 21 methods | unit (compile check) | `cargo build -p durable-lambda-core` | ❌ Wave 0 |
| ARCH-02 | ClosureContext implements DurableContextOps | unit (compile check) | `cargo build -p durable-lambda-closure` | ❌ Wave 0 |
| ARCH-03 | TraitContext implements DurableContextOps | unit (compile check) | `cargo build -p durable-lambda-trait` | ❌ Wave 0 |
| ARCH-04 | BuilderContext implements DurableContextOps | unit (compile check) | `cargo build -p durable-lambda-builder` | ❌ Wave 0 |
| ARCH-05 | Generic `async fn logic<C: DurableContextOps>` runs with all 4 context types | integration | `cargo test -p parity-tests generic_handler` | ❌ Wave 0 |
| ARCH-06 | Handler boilerplate in shared function, not duplicated | unit + compile | `cargo build --workspace` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --workspace`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `crates/durable-lambda-core/src/ops_trait.rs` — covers ARCH-01, plus `impl DurableContextOps for DurableContext` for ARCH-05
- [ ] `impl DurableContextOps for ClosureContext` in `crates/durable-lambda-closure/src/context.rs` — covers ARCH-02
- [ ] `impl DurableContextOps for TraitContext` in `crates/durable-lambda-trait/src/context.rs` — covers ARCH-03
- [ ] `impl DurableContextOps for BuilderContext` in `crates/durable-lambda-builder/src/context.rs` — covers ARCH-04
- [ ] Generic handler test function in `tests/parity/tests/parity.rs` — covers ARCH-05
- [ ] `parse_invocation()` function and `InvocationData` struct in `crates/durable-lambda-core/src/event.rs` — covers ARCH-06

*(All gaps are new code additions, not separate test files. The test for ARCH-05 is added to the existing parity test file.)*

---

## Sources

### Primary (HIGH confidence)
- Direct code inspection: `crates/durable-lambda-closure/src/context.rs`, `crates/durable-lambda-trait/src/context.rs`, `crates/durable-lambda-builder/src/context.rs` — exact method inventory (21 methods each)
- Direct code inspection: `crates/durable-lambda-closure/src/handler.rs`, `crates/durable-lambda-trait/src/handler.rs`, `crates/durable-lambda-builder/src/handler.rs`, `crates/durable-lambda-macro/src/expand.rs` — handler boilerplate duplication (verified identical blocks)
- Direct code inspection: `Cargo.toml` workspace — Rust edition 2021, `async-trait = "0.1"`, `rustc 1.94.0`
- Direct code inspection: `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md` — phase requirements ARCH-01 through ARCH-06

### Secondary (MEDIUM confidence)
- Rust Reference: RPITIT (Return Position `impl Trait` In Traits) stabilized in Rust 1.75 — enables native `async fn` in traits without boxing for static dispatch

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Method inventory: HIGH — verified by counting `pub fn` lines in all three wrapper context files
- Trait design: HIGH — based on current Rust stable language features (Rust 1.94 in use)
- Handler boilerplate: HIGH — all 4 locations read and compared directly
- 44-method claim: LOW — project documents say 44, actual count is 21; flagged as open question

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (stable domain; trait/async changes unlikely in 30 days)
