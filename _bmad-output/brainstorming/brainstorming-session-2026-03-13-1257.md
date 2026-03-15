---
stepsCompleted: [1, 2, 3, 4]
inputDocuments: []
session_topic: 'Rust Durable Lambda SDK — idiomatic Rust implementation matching AWS Python Durable Lambda SDK feature parity'
session_goals: 'Design a Rust SDK for AWS Durable Lambda with full Python SDK feature parity, idiomatic Rust patterns, and aws-sdk-rust integration'
selected_approach: 'ai-recommended'
techniques_used: ['First Principles Thinking', 'Cross-Pollination', 'Morphological Analysis']
ideas_generated: 37
context_file: ''
session_active: false
workflow_completed: true
facilitation_notes: 'User is decisive and technically precise. Corrected assumptions about replay model early — deep understanding of the AWS durable execution model. Prefers explicit, AI-friendly patterns for junior dev team. Wants all 4 API approaches implemented for comparison despite clear preference signals.'
---

# Brainstorming Session Results

**Facilitator:** Esa
**Date:** 2026-03-13

## Session Overview

**Topic:** Rust Durable Lambda SDK — creating an idiomatic Rust implementation of the AWS Durable Lambda SDK, matching all features of the official Python SDK, built on the AWS Rust SDK.

**Goals:** Full feature parity with the Python Durable Lambda SDK, using idiomatic Rust patterns (ownership, traits, async, type safety, macros), integrated with the official `aws-sdk-rust`.

### Session Setup

This session explores the design space for bringing durable execution to Rust-based AWS Lambda functions — covering state management, workflow orchestration, retries, checkpointing, and all capabilities provided by the existing Python SDK.

## Technique Selection

**Approach:** AI-Recommended Techniques
**Analysis Context:** Complex SDK design challenge translating Python paradigms to idiomatic Rust

**Recommended Techniques:**

- **First Principles Thinking:** Strip away Python-specific assumptions to find the irreducible core of durable execution, then rebuild from Rust's strengths.
- **Cross-Pollination:** Pull proven patterns from tokio, tower, axum, serde, lambda_runtime, Temporal, and Restate into the SDK design.
- **Morphological Analysis:** Systematically map every design dimension and its options into a decision matrix for the 4 API approaches.

## Technique Execution Results

### First Principles Thinking

**Interactive Focus:** Identifying the fundamental execution model, serialization strategy, error handling, async model, and ownership patterns.

**Key Findings:**

**FP#1 — Replay-with-Memoization, Not Resume**
The durable function replays from the top on every invocation. Specific AWS API calls (durable operations) check a log of previous results — if a result exists at the current position, it returns the cached value. If not, the operation executes, checkpoints the result via the AWS API, and continues. The function code IS the state machine.

**FP#2 — 9 AWS API Primitives**
The `aws-sdk-lambda` crate exposes 9 durable execution operations:
- `checkpoint_durable_execution`
- `get_durable_execution`
- `get_durable_execution_history`
- `get_durable_execution_state`
- `list_durable_executions_by_function`
- `send_durable_execution_callback_failure`
- `send_durable_execution_callback_heartbeat`
- `send_durable_execution_callback_success`
- `stop_durable_execution`

**FP#3 — 8 Python SDK Abstractions (Feature Parity Target)**
The Python SDK builds these user-facing operations on top of the raw API:
1. **Steps** — checkpointed work with retries and execution semantics
2. **Waits** — time-based suspension (no compute cost)
3. **Callbacks** — wait for external signals via callback ID
4. **Invoke** — call other Lambda functions durably
5. **Parallel** — fan-out with configurable completion criteria
6. **Map** — parallel collection processing with batching
7. **Child Contexts** — isolated subflows with own checkpoint namespace
8. **Logger** — replay-safe, deduplicated structured logging

**FP#4 — Positional Log Walker**
The replay engine is a cursor over an ordered log of `(sequence_position, operation_name, serialized_result)` entries. Each durable operation advances the cursor. This is fundamentally a state monad.

**FP#5 — Eager History Loading**
On every invocation, call `get_durable_execution_history` once, load into `Vec<HistoryEntry>`, store with a cursor index. Two-phase model: replay phase (cursor < vec.len()) and execution phase (cursor == vec.len()).

```rust
enum ExecutionMode {
    Replaying { history: Vec<HistoryEntry>, cursor: usize },
    Executing,
}
```

**FP#6 — Serde is the Serialization Layer**
`Serialize + DeserializeOwned` trait bounds on all checkpoint values. Compile-time guarantees that data is serializable — something Python cannot provide.

**FP#7 — Serde-Only, No Custom SerDes**
Eliminates the Python SDK's `SerDes` configuration surface entirely. Extended types (datetime, Decimal, UUID, bytes) are handled natively by existing serde-compatible crates.

**FP#8 — Typed Error Enum**
`DurableError` enum with `thiserror` derives for SDK errors. User step functions return `Result<T, E>` with their own error types.

```rust
#[derive(Debug, thiserror::Error)]
pub enum DurableError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("execution terminated: {0}")]
    ExecutionTerminated(String),
    #[error("invocation failed: {0}")]
    InvocationFailed(String),
    #[error("callback failed: {0}")]
    CallbackFailed(String),
    #[error("step interrupted")]
    StepInterrupted,
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("AWS SDK error: {0}")]
    AwsSdk(#[from] aws_sdk_lambda::Error),
}
```

**FP#9 — Fully Serializable Errors**
Both `T` and `E` in step results require `Serialize + DeserializeOwned`. Failures are checkpointed with full type information and replay as the exact same typed error.

```rust
#[derive(Serialize, Deserialize)]
enum CheckpointResult<T, E> {
    Ok(T),
    Err(E),
}
```

**FP#10 (Revised) — Parallel IS Local Concurrency**
The Python SDK uses `ThreadPoolExecutor` with threads. In Rust, parallel branches run as `tokio::spawn` tasks within the same Lambda invocation. Each branch gets an owned child context.

**FP#11 — Always Parallel with Send + 'static Bounds**
Branch closures require `Send + 'static`. Consistent behavior in both replay and live execution. Standard tokio pattern — no surprises for Rust developers.

**FP#12 — Owned Child Contexts**
Parent context creates fully owned child contexts for parallel/map branches. Each child holds its own `ExecutionMode`, operation counter, and name prefix. Only the AWS client (`Arc<LambdaService>`) is shared.

```rust
struct DurableContext {
    mode: ExecutionMode,
    operation_counter: usize,
    name_prefix: String,
    lambda_service: Arc<LambdaService>,
    execution_id: String,
}

impl DurableContext {
    fn create_child(&mut self, name: &str) -> DurableContext {
        DurableContext {
            mode: self.extract_child_history(name),
            operation_counter: 0,
            name_prefix: format!("{}{name}.", self.name_prefix),
            lambda_service: Arc::clone(&self.lambda_service),
            execution_id: self.execution_id.clone(),
        }
    }
}
```

### Cross-Pollination

**Interactive Focus:** Transferring patterns from lambda_runtime, tower, axum, Temporal, Restate, serde, thiserror, and the aws-sdk-lambda crate.

**Key Findings:**

**CP#1 — Mirror Lambda Runtime Entry Point**
Use `durable_lambda::run(handler).await` in `main()` — the pattern every Rust Lambda developer already knows.

**CP#2 — Tower Service Pattern for Middleware**
Durable execution as a tower middleware layer enables composability with logging, tracing, and auth layers.

**CP#3 — Proc Macros for Boilerplate Elimination**
`#[durable_execution]` proc macro handles registration, history loading, and context creation. Most ergonomic for the common case.

**CP#4 — Context Methods Return Futures (Restate Pattern)**
Every context method returns a future polled when awaited. Natural async Rust — `ctx.step()` returns `StepFuture<T>`.

**CP#5 — Closures Over Derive Macros for Steps**
`ctx.step("name", || do_work(arg1, arg2))` is already clean in Rust. Closures capture arguments naturally — a derive macro would be over-engineering.

**CP#6 — Extractor Pattern from Axum**
Handler arguments extracted from the Lambda invocation automatically. Function signature declares what it needs.

**CP#7 — AI-Assistance Friendly Design**
AI coding tools work best with explicit, repetitive, pattern-based code. Favors closure-native and builder approaches over trait-heavy or macro-magic approaches. The team relies heavily on Claude Code CLI and Copilot.

**CP#8 — Closure + Builder Merged Approach**
Simple default path: `ctx.step("name", || work()).await?`. Optional config via chaining: `.with_retry()`, `.with_timeout()`, `.named()`. One pattern to learn.

**CP#9 — Documentation-as-Code for AI Discoverability**
Rich `///` doc comments with inline examples on every public method. Doc comments become the prompt context AI tools use to generate user code.

**CP#10 — thiserror for SDK, User Chooses Error Strategy**
SDK uses `thiserror`. Users can use `anyhow`, custom `thiserror` enums, or simple strings — only requirement is `Serialize + DeserializeOwned`.

**CP#11 — Match AWS SDK Ergonomics**
Builder patterns that mirror how `aws-sdk-lambda` already works. Developers see familiar patterns.

**CP#12 — Testing Crate with MockDurableContext**
`durable-lambda-testing` crate provides local replay simulation without AWS credentials. Enables AI-generated tests.

```rust
#[tokio::test]
async fn test_order_workflow() {
    let ctx = MockDurableContext::new()
        .with_step_result("validate", json!({"valid": true}))
        .with_step_result("charge", json!({"transaction_id": "tx123"}));
    let result = handler(test_event(), &mut ctx).await.unwrap();
    assert_eq!(result.status, "completed");
}
```

**CP#13 — Cargo Workspace with Shared Examples**
All 4 approaches as sibling crates in a workspace. Same workflow implemented in all 4 styles in `examples/` for direct comparison.

### Morphological Analysis

**Interactive Focus:** Systematically mapping 12 design dimensions with options for each of the 4 API approaches.

**Design Matrix:**

| Dimension | Macro | Trait | Closure | Builder |
|---|---|---|---|---|
| 1. Entry Point | `#[durable_main]` | `durable_lambda::run` | `durable_lambda::run` | `durable_lambda::run` |
| 2. Handler Signature | event + `&mut ctx` | event + `&mut ctx` | event + `&mut ctx` | event + `&mut ctx` |
| 3. Step Definition | proc macro | trait object | closure | closure + builder |
| 4. Step Configuration | attributes | config struct | config struct | builder methods |
| 5. Wait Operations | simple call | simple call | simple call | builder |
| 6. Callbacks | two-phase | two-phase | both | both |
| 7. Invoke | infer type | turbofish | infer type | AWS-style builder |
| 8. Parallel | vec of boxed closures | tuple of closures | vec of boxed closures | builder |
| 9. Map | closure | closure | closure | builder |
| 10. Child Contexts | closure | explicit lifecycle | closure | closure |
| 11. Logger | tracing | tracing | tracing | both |
| 12. BatchResult | methods | both | both | both |

## Idea Organization and Prioritization

### Thematic Organization

**Theme 1: Core Architecture (6 ideas)** — Replay engine, history loading, execution mode, concurrency model, child context ownership
**Theme 2: Type System & Serialization (4 ideas)** — Serde-only, typed errors, serializable errors, checkpoint result enum
**Theme 3: API Ergonomics & AI Friendliness (6 ideas)** — Entry point, closures, AI-friendly design, closure+builder, docs-as-code, AWS SDK style
**Theme 4: Ecosystem Integration (6 ideas)** — Tower, proc macros, futures, extractors, thiserror, testing
**Theme 5: Project Structure (3 ideas)** — Cargo workspace, AWS API mapping, feature parity target
**Theme 6: Design Matrix (12 dimensions)** — Complete option mapping across all 4 approaches

### Prioritization Results

**Top Priority — Implement First:**
1. `durable-lambda-core` — Replay engine, AWS API wrapper, serde checkpointing, error types, DurableContext with child contexts
2. `durable-lambda-testing` — MockDurableContext for local development without AWS credentials

**Then in parallel — the 4 approaches:**
3. `durable-lambda-macro` — Proc macro approach
4. `durable-lambda-trait` — Trait-based approach
5. `durable-lambda-closure` — Closure-native approach
6. `durable-lambda-builder` — Builder pattern approach

**Validation:**
7. `examples/` — Same order-processing workflow in all 4 styles for team comparison

### Action Plan

1. Initialize cargo workspace with 6 crates
2. Define core types — `DurableContext`, `ExecutionMode`, `HistoryEntry`, `CheckpointResult<T,E>`, `DurableError`, `BatchResult<T>`
3. Implement replay engine in core — eager history loading, cursor-based replay, checkpoint via `aws-sdk-lambda`
4. Build `MockDurableContext` in testing crate
5. Implement all 4 approaches against the core, following the morphological matrix
6. Create comparison examples — identical order-processing workflow in all 4 styles
7. Team evaluation — developers try each approach with AI assistance and report preference

## Session Summary and Insights

**Key Achievements:**
- 37 ideas generated across 3 complementary techniques
- Complete architectural foundation established for the Rust Durable Lambda SDK
- 12-dimension design matrix mapping all 4 API approaches
- Clear implementation priority order and action plan

**Breakthrough Insights:**
- Durable execution is replay-with-memoization, NOT checkpoint-and-resume — the function code IS the state machine
- Serde eliminates the entire custom serialization layer from the Python SDK — compile-time guarantees with zero configuration
- AI-assistance friendliness strongly favors closure-native and builder approaches for the team's junior developers
- Parallel operations use local concurrency (tokio::spawn), not AWS-managed fan-out — requiring Send + 'static bounds on branch closures

**Project Structure:**
```
durable-rust/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── durable-lambda-core/      # Shared: replay engine, AWS calls, serde
│   ├── durable-lambda-macro/     # Approach 1: proc macros
│   ├── durable-lambda-trait/     # Approach 2: trait-based
│   ├── durable-lambda-closure/   # Approach 3: closure-native
│   ├── durable-lambda-builder/   # Approach 4: builder pattern
│   └── durable-lambda-testing/   # Test utilities, mock context
├── examples/
│   ├── order-processing/
│   │   ├── macro-style/
│   │   ├── trait-style/
│   │   ├── closure-style/
│   │   └── builder-style/
│   └── ai-workflow/
│       ├── macro-style/
│       ├── trait-style/
│       ├── closure-style/
│       └── builder-style/
└── docs/
```
