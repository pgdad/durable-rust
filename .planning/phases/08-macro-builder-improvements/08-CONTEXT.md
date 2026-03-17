# Phase 8: Macro & Builder Improvements - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning
**Source:** Auto-generated from prior context, codebase analysis, and requirement specs

<domain>
## Phase Boundary

Improve the proc-macro and builder-pattern API styles. The proc-macro gains compile-time type validation for the second parameter (DurableContext) and return type (Result<Value, DurableError>). The builder gains pre-run configuration methods (.with_tracing, .with_error_handler). No changes to closure or trait approaches.

</domain>

<decisions>
## Implementation Decisions

### Macro type validation scope
- Validate second parameter type contains "DurableContext" (string match on type path segments, not full resolution — proc-macros can't do type-level analysis)
- Validate return type is `Result<serde_json::Value, DurableError>` or similar pattern (check outer Result wrapper)
- Error messages should suggest the correct signature: `expected DurableContext, found {actual}`
- Validation is best-effort at the token level — exotic type aliases or re-exports won't be caught, and that's acceptable

### Trybuild compile-fail tests
- Add `fail_wrong_param_type.rs` — function with 2 params but wrong types (e.g., `i32, i32`)
- Add `fail_wrong_return_type.rs` — function with correct params but returns `String` instead of `Result`
- Each test needs a matching `.stderr` file with expected error output
- Existing trybuild infrastructure in `crates/durable-lambda-macro/tests/` is reused

### Builder configuration methods
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

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `expand.rs:88-106` — existing `validate_signature()` function with parameter count check; extend with type checks
- `tests/ui/` directory with 2 existing `.rs` + `.stderr` compile-fail tests — established trybuild pattern
- `trybuild.rs` test runner already configured and working
- `DurableHandlerBuilder` struct with `PhantomData<Fut>` — add `Option` fields for configuration

### Established Patterns
- Proc-macro validation uses `syn::Error::new_spanned` for precise error locations
- Parameter extraction: `func.sig.inputs` is `Punctuated<FnArg, Comma>`; second item is `FnArg::Typed(PatType)` with `.ty` field
- Return type: `func.sig.output` is `ReturnType::Type(_, Box<Type>)`
- Builder pattern: `self`-consuming methods that return `Self` for chaining

### Integration Points
- `expand.rs` — add type checks to `validate_signature()`
- `handler.rs` — add optional fields and builder methods to `DurableHandlerBuilder`
- `handler.rs::run()` — apply tracing subscriber and error handler before Lambda registration
- `tests/trybuild.rs` — automatically picks up new `.rs` files in `tests/ui/`

</code_context>

<specifics>
## Specific Ideas

- Macro error messages should be actionable: show what was found AND what's expected
- Builder methods should be discoverable via IDE autocomplete — doc comments with examples on each

</specifics>

<deferred>
## Deferred Ideas

- Builder `.with_middleware(fn)` for request/response interception — too complex for v1
- Macro support for custom event types (not just `serde_json::Value`) — would require generic expansion changes

</deferred>

---

*Phase: 08-macro-builder-improvements*
*Context gathered: 2026-03-17 via auto-mode*
