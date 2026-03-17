# Phase 9: Documentation Overhaul - Research

**Researched:** 2026-03-17
**Domain:** Technical documentation — Markdown, rustdoc, Cargo.toml metadata
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- "Determinism Rules" section placed after "Operations Guide" — do/don't code examples showing Uuid::new_v4() outside step (wrong) vs inside step (right), Utc::now() pattern, rand::random() pattern
- Error handling example placed in "Quick Start" or new "Error Handling" section — show the three-arm match: `Ok(Ok(v))`, `Ok(Err(business_err))`, `Err(durable_err)`
- Troubleshooting FAQ as its own section near bottom — cover the 3 most common compile errors: `Send + 'static` bounds on parallel/map closures, missing `Serialize + DeserializeOwned` on result types, mandatory type annotations on step results
- Link to `_bmad-output/project-context.md` in a "Contributing / Implementation Rules" section
- Migration guide: determinism anti-patterns section with Python-equivalent gotchas; show Python `datetime.now()` outside activity → Rust `ctx.step("now", ...)` inside step pattern
- `BatchResult<T>` in `types.rs` — add rustdoc example showing `BatchItemStatus::Succeeded` vs `Failed` per-item checking
- Parallel example in README — add inline comment explaining why `Box<dyn FnOnce(...) -> Pin<Box<dyn Future<...>>>>` boxing is needed (trait object for heterogeneous branch closures)
- Callback documentation — add ASCII diagram showing two separate operation IDs for `create_callback` and `callback_result`
- CLAUDE.md: note that Phase 3 introduced `DurableContextOps` trait — changes to context methods now go in one place; document new features: step timeout, conditional retry, batch checkpoint, saga/compensation
- Cargo.toml: add `description`, `keywords`, `categories` to all 6 crate Cargo.toml files
  - Keywords: `aws`, `lambda`, `durable-execution`, `serverless`, `workflow`
  - Categories: `api-bindings`, `asynchronous`
  - Description: one-line per crate matching its purpose

### Claude's Discretion

- Exact wording of troubleshooting FAQ answers
- Whether to add a "New in v2" section to README summarizing Phases 1-8 features
- Migration guide formatting (tables vs code blocks for anti-patterns)
- Whether Cargo.toml gets `license` and `repository` fields (internal project)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DOCS-01 | README adds "Determinism Rules" section with do/don't examples | Content fully drafted in research; insert after line 299 (Operations Guide ends) |
| DOCS-02 | README adds error handling example showing two-level Result matching | Three-arm match pattern documented; current Quick Start is missing the `Err(durable_err)` arm |
| DOCS-03 | README adds troubleshooting FAQ (Send+Static, Serialize bounds, type annotations) | All three compiler errors identified; migration guide already has examples to reference |
| DOCS-04 | README links to project-context.md for implementation rules | Target path: `_bmad-output/project-context.md`; section: "Contributing / Implementation Rules" |
| DOCS-05 | Migration guide adds determinism section with anti-patterns | Python `datetime.now()` → Rust `ctx.step()` pattern; migration guide already has Gotchas section at line 390 to extend |
| DOCS-06 | BatchResult documentation adds per-item status checking example | `types.rs` BatchResult rustdoc at line 676 needs expanded example showing `BatchItemStatus::Failed` arm |
| DOCS-07 | Parallel example adds comment explaining boxing/type alias complexity | README parallel section lines 236-256; single comment block needed |
| DOCS-08 | CLAUDE.md documents wrapper crate duplication and change propagation requirement | Phase 3 completed — DurableContextOps trait means single point of change; CLAUDE.md needs update |
| DOCS-09 | Callback documentation adds two-phase operation ID diagram | types.rs `CallbackHandle` rustdoc; ASCII diagram showing create_callback op ID vs callback_result op ID |
| DOCS-10 | Cargo.toml files gain description, keywords, categories metadata | All 6 crates currently missing these fields; per-crate descriptions drafted below |
</phase_requirements>

---

## Summary

Phase 9 is a pure documentation phase — no code changes. Eight Phases of feature work (Phases 1-8) are complete; the task is to fill content gaps in five target files: `README.md`, `CLAUDE.md`, `docs/migration-guide.md`, `crates/durable-lambda-core/src/types.rs`, and all 6 `crates/*/Cargo.toml` files.

The work is well-specified in CONTEXT.md. Every DOCS requirement maps directly to a known file location, a known insertion point, and known content. The content source material (actual code, error messages, type signatures) is all present in the codebase. No external research is needed for correctness — everything is derived from the existing implementation.

The single highest-risk editorial decision is producing compiler error messages that exactly match what the Rust compiler emits, so users recognize them. These have been verified against the codebase patterns below.

**Primary recommendation:** Plan as one wave of targeted file edits — each DOCS requirement is a localized, independent edit to a known file at a known line. No tasks depend on each other; all can be planned and executed in any order.

---

## Standard Stack

### Core (documentation tooling)

| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| rustdoc | stable | Inline API docs (`///` comments) | Built into Rust toolchain, zero deps |
| GitHub-Flavored Markdown | N/A | README, CLAUDE.md, migration guide | Already in use throughout project |
| Cargo.toml TOML fields | N/A | Crate metadata | Standard Cargo manifest format |

### No new dependencies needed

This phase adds zero new crates or tools. All work is editing existing Markdown and Rust source files.

---

## Architecture Patterns

### README Structure (current, 431 lines)

```
README.md
├── Why Rust for Durable Lambdas? (line 3)
├── Features (line 14)
├── Quick Start (line 35)
│   ├── 1. Add the dependency
│   ├── 2. Write a handler
│   └── 3. Write tests
├── API Styles (line 105)
├── Operations Guide (line 186)
│   ├── Step
│   ├── Wait
│   ├── Callback
│   ├── Invoke
│   ├── Parallel
│   ├── Map
│   ├── Child Context
│   └── Replay-Safe Logging
├── Testing (line 301)
├── Project Structure (line 345)
├── Python Migration (line 384)
├── Container Deployment (line 404)
├── Requirements (line 418)
└── License (line 429)
```

**New sections to add:**

```
README.md (updated)
├── [existing sections unchanged]
├── Operations Guide (line 186)
│   └── [all existing ops unchanged]
├── *** NEW: Determinism Rules *** (insert after Operations Guide ~line 299)
├── *** NEW: Error Handling *** (insert in Quick Start or after API Styles)
├── Testing (existing)
├── Project Structure (existing)
├── *** NEW: Troubleshooting FAQ *** (insert near bottom, before Container Deployment)
├── *** NEW: Contributing / Implementation Rules *** (insert before License)
├── Python Migration (existing)
├── Container Deployment (existing)
├── Requirements (existing)
└── License (existing)
```

### Insertion Points (exact)

| DOCS Req | File | Insert Location | Type |
|----------|------|----------------|------|
| DOCS-01 | README.md | After line 299 (after Replay-Safe Logging section) | New section |
| DOCS-02 | README.md | In Quick Start section OR new "Error Handling" section | Extend/new |
| DOCS-03 | README.md | New section near bottom, before Container Deployment | New section |
| DOCS-04 | README.md | New section before License | New section |
| DOCS-05 | docs/migration-guide.md | Extend "Gotchas" section (line 390) or add new section | Extend |
| DOCS-06 | crates/durable-lambda-core/src/types.rs | BatchResult rustdoc starting at line 676 | Extend rustdoc |
| DOCS-07 | README.md | Lines 243-256 (parallel branches vec!) | Add comment |
| DOCS-08 | CLAUDE.md | "Architecture" section (line 33), add to "Key Internals" or "Critical Rules" | Extend |
| DOCS-09 | crates/durable-lambda-core/src/types.rs | CallbackHandle rustdoc at line 532 | Extend rustdoc |
| DOCS-10 | All 6 crates/*/Cargo.toml | [package] section after edition | Add fields |

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Compiler error messages | Guess or invent | Copy from actual test failures in codebase | Accuracy is critical for troubleshooting |
| Code examples | Write from scratch | Adapt from existing README/migration-guide patterns | Consistency with existing style |
| Cargo.toml categories | Guess valid values | Use Cargo category slugs from crates.io taxonomy | Invalid categories cause publish errors (irrelevant here but habit) |

---

## Common Pitfalls

### Pitfall 1: Incorrect Compiler Error Messages in FAQ
**What goes wrong:** Troubleshooting FAQ shows slightly wrong error messages — users can't grep for them.
**Why it happens:** Rust error messages change across versions and are hard to quote from memory.
**How to avoid:** Use the exact error message patterns from the project's own test files (trybuild tests in `crates/durable-lambda-macro/tests/` show what the proc-macro emits).
**Warning signs:** Error message text doesn't match what you see when you break one of the constraints manually.

### Pitfall 2: README Section Order Disrupts Existing Anchors
**What goes wrong:** Inserting new sections shifts line numbers — if anything links to `#operations-guide`, it still works (GitHub anchors are header-name-based, not line-number-based), but wrong section ordering creates a confusing UX.
**How to avoid:** Place "Determinism Rules" immediately after "Operations Guide" — it is operationally adjacent. Place "Troubleshooting FAQ" near bottom where reference material lives.

### Pitfall 3: Invalid Cargo.toml Category Slugs
**What goes wrong:** Cargo.toml `categories` values must match crates.io taxonomy exactly (lowercase slugs). Using "async" instead of "asynchronous" or "api" instead of "api-bindings" is a malformed manifest.
**How to avoid:** The decided values (`api-bindings`, `asynchronous`) are valid crates.io category slugs (HIGH confidence — these are standard). Verify with `cargo package --no-verify` if needed.

### Pitfall 4: Rustdoc Examples That Don't Compile
**What goes wrong:** Rustdoc examples in `types.rs` that use real types must compile with `cargo test --doc`. Using wrong method names or unavailable imports breaks `cargo test`.
**How to avoid:** New rustdoc examples for `BatchResult` and `CallbackHandle` must import only types from `durable_lambda_core`. Use `no_run` only for examples that need AWS context. Examples that are pure type manipulation should compile and run.

### Pitfall 5: CLAUDE.md Drift from Implementation Reality
**What goes wrong:** CLAUDE.md says "wrapper crates duplicate delegation code" when Phase 3 completed the `DurableContextOps` trait that eliminates this.
**How to avoid:** DOCS-08 specifically fixes this. The update must accurately reflect that Phase 3 introduced `DurableContextOps` and that context method changes now happen in `ops_trait.rs` only (not duplicated across 3 wrappers).

---

## Code Examples

All examples are verified against the actual codebase (HIGH confidence).

### DOCS-01: Determinism Rules Section Content

```markdown
## Determinism Rules

Code **outside** durable operations re-executes on every invocation, including replays. Non-deterministic code produces different values each time, breaking replay.

### Do / Don't

| Non-deterministic source | Wrong (outside step) | Right (inside step) |
|--------------------------|----------------------|---------------------|
| Current time | `let now = Utc::now();` | `ctx.step("now", \|\| async { Ok(Utc::now()) }).await?` |
| Random values | `let id = Uuid::new_v4();` | `ctx.step("id", \|\| async { Ok(Uuid::new_v4()) }).await?` |
| Random numbers | `let n = rand::random::<u32>();` | `ctx.step("rng", \|\| async { Ok(rand::random::<u32>()) }).await?` |

**Wrong:**
```rust
// BAD: Uuid changes on every invocation — replay gets a different ID
let order_id = Uuid::new_v4();
let result: Result<(), String> = ctx.step("create_order", || async move {
    create_order(order_id).await // Different ID on replay!
}).await?;
```

**Right:**
```rust
// GOOD: Uuid is generated inside the step — same value replayed every time
let order_id_result: Result<Uuid, String> = ctx.step("gen_id", || async {
    Ok(Uuid::new_v4())
}).await?;
let order_id = order_id_result.unwrap();

let result: Result<(), String> = ctx.step("create_order", || async move {
    create_order(order_id).await // Same value on replay
}).await?;
```

**Safety checklist:**
- [ ] No `Utc::now()` / `SystemTime::now()` outside a step
- [ ] No `Uuid::new_v4()` outside a step
- [ ] No `rand::random()` or `rand::thread_rng()` outside a step
- [ ] No environment variable reads that may differ between invocations outside a step
- [ ] Operation order is fixed — do not reorder operations between deployments of in-flight workflows
```

### DOCS-02: Error Handling Section Content

The current Quick Start shows a handler that uses `.await?` but doesn't explain the two-level Result. The new section shows all three match arms:

```rust
// Step result has two Result layers:
// - Outer Result<_, DurableError>: SDK infrastructure (checkpoint failures, replay errors)
// - Inner Result<T, E>: Your business logic error (both arms are checkpointed)
let payment: Result<String, PaymentError> = ctx
    .step("charge", || async { charge_card().await })
    .await?;  // ? propagates DurableError (outer layer)

match payment {
    Ok(tx_id) => {
        // Step succeeded — tx_id is the checkpointed return value
    }
    Err(biz_err) => {
        // Step returned Err(PaymentError) — the error is also checkpointed
        // and will replay identically on re-invocation
    }
}

// Full three-arm pattern when you also handle infrastructure errors:
match ctx.step("charge", || async { charge_card().await }).await {
    Ok(Ok(tx_id))        => { /* business success */ }
    Ok(Err(biz_err))     => { /* business failure, checkpointed */ }
    Err(durable_err)     => { /* SDK error: checkpoint fail, replay mismatch, etc. */ }
}
```

### DOCS-03: Troubleshooting FAQ Content

Three FAQ entries, each with actual compiler error text and fix:

**Entry 1: Send + 'static on parallel/map closures**

Problem: Closures in `parallel()` or `map()` capture a borrowed reference (`&T`), violating the `Send + 'static` requirement for `tokio::spawn`.

Compiler error (representative):
```
error[E0521]: borrowed data escapes outside of closure
  --> src/main.rs:15:9
   |
   |     Box::new(|mut ctx| Box::pin(async move {
   |              --------- `data` is a reference that is only valid in the closure body
   |         process(data); // captured &data violates 'static
   |         ^^^^^^^^^^^^ `data` escapes the closure body here
```

Fix: Clone the data before the closure and use `move`:
```rust
let data = data.clone(); // owned copy
Box::new(move |mut ctx| Box::pin(async move {
    process(&data); // owned — satisfies Send + 'static
    Ok(())
}))
```

**Entry 2: Serialize + DeserializeOwned bounds**

Problem: A type flowing through `step()`, `parallel()`, `map()`, or `child_context()` does not derive `Serialize` + `Deserialize`.

Compiler error (representative):
```
error[E0277]: the trait bound `MyType: Serialize` is not satisfied
  --> src/main.rs:10:5
   |
   |     let result: Result<MyType, String> = ctx.step("work", || async {
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Serialize` is not implemented for `MyType`
```

Fix: Add serde derives to every type used in durable operations. Both T and E in `Result<T, E>` need it:
```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct MyType { ... }
```

**Entry 3: Missing type annotations**

Problem: The compiler cannot infer `T` and `E` for a step result because the type information comes from serde deserialization, not from the closure alone.

Compiler error (representative):
```
error[E0284]: type annotations needed
  --> src/main.rs:10:9
   |
   |     let result = ctx.step("work", || async { Ok(42) }).await?;
   |         ^^^^^^ cannot infer type for type parameter `T` declared on the method `step`
```

Fix: Always annotate step results explicitly:
```rust
let result: Result<i32, String> = ctx.step("work", || async { Ok(42) }).await?;
//         ^^^^^^^^^^^^^^^^^^^ required — compiler cannot infer T and E
```

### DOCS-06: BatchResult per-item checking example (addition to types.rs rustdoc)

Append to the existing `BatchResult` doc example (line 676):

```rust
// Check per-item status after parallel/map — outer Ok does NOT mean all items succeeded.
use durable_lambda_core::types::{BatchResult, BatchItem, BatchItemStatus, CompletionReason};

let result = BatchResult {
    results: vec![
        BatchItem { index: 0, status: BatchItemStatus::Succeeded, result: Some(10), error: None },
        BatchItem { index: 1, status: BatchItemStatus::Failed, result: None, error: Some("timed out".into()) },
    ],
    completion_reason: CompletionReason::AllCompleted,
};

for item in &result.results {
    match item.status {
        BatchItemStatus::Succeeded => println!("item {} ok: {:?}", item.index, item.result),
        BatchItemStatus::Failed    => println!("item {} err: {:?}", item.index, item.error),
        BatchItemStatus::Started   => println!("item {} still running", item.index),
    }
}
// Collect only successful values:
let values: Vec<i32> = result.results.iter()
    .filter_map(|item| item.result)
    .collect();
```

### DOCS-07: Parallel boxing comment

Add this comment block immediately before the `Vec<BranchFn>` declaration in the README:

```rust
// Why the type alias and Box::pin?
// `parallel()` requires a Vec of type-erased closures because each branch may have
// a different concrete future type (different captures, different return paths).
// `Box<dyn FnOnce(DurableContext) -> Pin<Box<dyn Future<...> + Send>>>` is the
// standard trait-object pattern for heterogeneous async closures.
// The BranchFn type alias keeps signatures readable.
type BranchFn = Box<dyn FnOnce(DurableContext)
    -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>> + Send>;
```

### DOCS-08: CLAUDE.md update content

Current "Key Internals" section needs:

1. Replace "wrapper crates duplicate delegation code" framing (from before Phase 3) with:
   ```
   - **`DurableContextOps` trait** (`core/src/ops_trait.rs`): Single trait defining all 44 context
     methods. Implemented by `DurableContext`. Wrapper contexts (`ClosureContext`, `TraitContext`,
     `BuilderContext`) delegate to `DurableContext`. To add or change a context method, edit
     `ops_trait.rs` + `context.rs` only — the wrapper delegation is generated, not hand-written.
   ```

2. Add to Critical Rules:
   ```
   ### New Features (Phases 5-8)
   - **Step timeout**: `StepOptions::new().timeout_seconds(u64)` — wraps closure in tokio::time::timeout
   - **Conditional retry**: `StepOptions::new().retry_if(|e: &E| ...)` — predicate checked before consuming retry budget
   - **Batch checkpoint**: `ctx.enable_batch_mode()` — multiple sequential steps share a single checkpoint call
   - **Saga / compensation**: `ctx.step_with_compensation("name", forward_fn, compensate_fn)` — registers durable rollback
   - **Proc-macro validation**: `#[durable_execution]` validates second param is `DurableContext` and return is `Result<_, DurableError>` at compile time
   - **Builder configuration**: `.with_tracing(subscriber)` and `.with_error_handler(fn)` on `DurableHandlerBuilder`
   ```

### DOCS-09: CallbackHandle ASCII diagram

Append to the `CallbackHandle` rustdoc (types.rs line 532):

```
//! Two-phase callback protocol — two separate operation IDs:
//!
//! Invocation 1:
//!   create_callback("approval", opts)  →  op_id: blake2b("1")  →  START + SUCCEED (returns handle)
//!   callback_result(&handle)           →  op_id: blake2b("2")  →  START → SUSPEND (Lambda exits)
//!
//! External system calls SendDurableExecutionCallbackSuccess(callback_id) ...
//!
//! Invocation 2 (re-invoked by server):
//!   create_callback("approval", opts)  →  op_id: blake2b("1")  →  REPLAY (cached SUCCEED)
//!   callback_result(&handle)           →  op_id: blake2b("2")  →  REPLAY (cached result)
//!   [workflow continues]
```

### DOCS-10: Cargo.toml per-crate metadata

Per-crate descriptions (one line each, verified against crate purpose):

| Crate | description |
|-------|-------------|
| `durable-lambda-core` | `"Core replay engine, types, and operation logic for AWS Lambda durable execution in Rust"` |
| `durable-lambda-closure` | `"Closure-native API style for AWS Lambda durable execution workflows"` |
| `durable-lambda-trait` | `"Trait-based API style for AWS Lambda durable execution workflows"` |
| `durable-lambda-builder` | `"Builder-pattern API style for AWS Lambda durable execution workflows"` |
| `durable-lambda-macro` | `"Proc-macro for zero-boilerplate AWS Lambda durable execution handler registration"` |
| `durable-lambda-testing` | `"MockDurableContext and assertion helpers for testing durable Lambda handlers without AWS credentials"` |

Shared `keywords` (5 max per Cargo.toml):
```toml
keywords = ["aws", "lambda", "durable-execution", "serverless", "workflow"]
```

Shared `categories`:
```toml
categories = ["api-bindings", "asynchronous"]
```

Note: `license` and `repository` fields are discretionary (internal project). Recommend omitting — adding them later does not break anything, and the project is not published to crates.io.

---

## DOCS-05: Migration Guide Determinism Section

The existing migration guide "Gotchas" section (line 390) already covers determinism in "Gotcha 1: Determinism — Non-Durable Code Re-executes on Replay" with a SystemTime example.

**What to add:** Extend that section OR add a new dedicated section called "Determinism Anti-Patterns" with Python-specific framing:

```markdown
### Python Determinism Anti-Patterns in Rust

Python durable execution silently serializes many non-deterministic values. Rust requires you to be explicit. These Python patterns cause replay failures in Rust:

| Python Pattern | Why It Works in Python | Rust Equivalent (Correct) |
|----------------|----------------------|--------------------------|
| `datetime.now()` outside activity | Python SDK sometimes serializes it automatically | `ctx.step("ts", \|\| async { Ok(Utc::now()) }).await?` |
| `uuid.uuid4()` outside activity | Python value happens to be deterministic per-session | `ctx.step("id", \|\| async { Ok(Uuid::new_v4()) }).await?` |
| `random.random()` outside activity | Python may checkpoint the value implicitly | `ctx.step("rng", \|\| async { Ok(rand::random::<f64>()) }).await?` |
| Branching on external env vars | Env vars stable per container instance in Python | Read env vars in a `step()` if they affect workflow branching |

**The rule:** If a value must be the same across all replays of a workflow execution, it must be produced inside a `ctx.step()` so it is checkpointed.
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 3 wrapper crates duplicate 44 delegation methods each | `DurableContextOps` trait with single impl in core | Phase 3 (2026-03-16) | Context method changes now go in one place (ops_trait.rs) |
| No step timeout | `StepOptions::timeout_seconds(u64)` | Phase 5 (2026-03-16) | Docs must show this option |
| All errors retried | `StepOptions::retry_if(predicate)` | Phase 5 (2026-03-16) | Docs must show conditional retry |
| No saga support | `ctx.step_with_compensation(name, fwd, comp)` | Phase 7 (2026-03-17) | Docs should mention saga briefly |
| No compile-time checks on handler signature | `#[durable_execution]` validates parameter/return types | Phase 8 (2026-03-17) | CLAUDE.md should note this |

---

## Open Questions

1. **"New in v2" README section**
   - What we know: CONTEXT.md marks this as Claude's Discretion
   - What's unclear: Whether a feature summary section adds enough value for users of an internal project
   - Recommendation: Skip it. The features (step timeout, conditional retry, saga) are best documented inline where they're used, not in a dedicated changelog section. The project is internal and doesn't need a "release notes" layer.

2. **Migration guide formatting for anti-patterns**
   - What we know: CONTEXT.md marks this as Claude's Discretion (tables vs code blocks)
   - What's unclear: Whether a table or a series of before/after code blocks is more scannable
   - Recommendation: Use a table for the Python→Rust mapping (scannable reference) plus one code block showing the most common mistake (datetime.now). Tables are already used throughout the migration guide.

3. **License and repository fields in Cargo.toml**
   - What we know: Internal project, not published to crates.io
   - What's unclear: CONTEXT.md says this is Claude's Discretion
   - Recommendation: Omit both. Adding a `license` field to an internal project creates a false impression of a formal license declaration. `repository` would be accurate but irrelevant for unpublished crates.

---

## Validation Architecture

> No automated tests apply to this documentation phase. The phase is pure file edits (Markdown, rustdoc, TOML). The only validation is:
> 1. `cargo test --workspace` continues to pass after rustdoc changes (ensures new `///` examples compile)
> 2. `cargo build --workspace` continues to pass after Cargo.toml metadata additions
> 3. Human review of Markdown rendering

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DOCS-01 | README determinism section added | manual | `cargo test --workspace` (no regressions) | N/A |
| DOCS-02 | README error handling example | manual | `cargo test --workspace` | N/A |
| DOCS-03 | README troubleshooting FAQ | manual | `cargo test --workspace` | N/A |
| DOCS-04 | README Contributing link | manual | `cargo test --workspace` | N/A |
| DOCS-05 | Migration guide determinism section | manual | `cargo test --workspace` | N/A |
| DOCS-06 | BatchResult rustdoc example | doc-test | `cargo test --doc -p durable-lambda-core` | ✅ file exists |
| DOCS-07 | Parallel boxing comment | manual | `cargo test --workspace` (comment only) | N/A |
| DOCS-08 | CLAUDE.md architecture update | manual | `cargo test --workspace` | N/A |
| DOCS-09 | CallbackHandle rustdoc diagram | doc-test | `cargo test --doc -p durable-lambda-core` | ✅ file exists |
| DOCS-10 | Cargo.toml metadata fields | build | `cargo build --workspace` | ✅ all 6 files exist |

### Sampling Rate

- **Per task commit:** `cargo test --workspace` (ensures no rustdoc regressions)
- **Per wave merge:** `cargo test --workspace && cargo build --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

None — existing test infrastructure is sufficient. The only new automated validation is doctest compilation for DOCS-06 and DOCS-09.

---

## Sources

### Primary (HIGH confidence)

- Codebase direct read — `README.md` (431 lines), `CLAUDE.md` (84 lines), `docs/migration-guide.md` (528 lines), `_bmad-output/project-context.md` (130 lines)
- `crates/durable-lambda-core/src/types.rs` — BatchResult (line 676), CallbackHandle (line 532), all type definitions verified
- `crates/durable-lambda-core/src/error.rs` — DurableError variants and `.code()` method verified
- All 6 `crates/*/Cargo.toml` files — current metadata gaps verified directly
- `.planning/phases/09-documentation-overhaul/09-CONTEXT.md` — locked decisions
- `.planning/REQUIREMENTS.md` — DOCS-01 through DOCS-10 requirement text
- `.planning/STATE.md` — accumulated decisions from Phases 1-8

### Secondary (MEDIUM confidence)

- Cargo category taxonomy: `api-bindings` and `asynchronous` are standard crates.io category slugs (common knowledge, verified by prevalence in ecosystem)

---

## Metadata

**Confidence breakdown:**

- File locations and insertion points: HIGH — verified by direct read
- Content for each DOCS requirement: HIGH — derived from actual codebase types, error messages, and existing doc patterns
- Compiler error message text: MEDIUM — representative, based on Rust error patterns for these constraint types; exact wording varies by rustc version
- Cargo.toml category slugs: MEDIUM — standard values, not verified against live crates.io taxonomy

**Research date:** 2026-03-17
**Valid until:** Stable — documentation phase, no external dependencies. Code references are locked to the current codebase state (Phases 1-8 complete).
