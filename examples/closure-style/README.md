# Closure-Style Examples

The closure-style API is the **recommended default** for most teams. Pass closures directly to `ctx.step()`, `ctx.parallel()`, and other operations for the most ergonomic handler definitions.

**When to choose this style:** You want the most concise syntax, are comfortable with Rust closures, and do not need struct-based handler state.

## Quick Start

```bash
cargo build -p closure-style-example
```

## Handler Pattern

```rust
use durable_lambda_closure::prelude::*;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    let result: Result<String, String> = ctx.step("work", || async {
        Ok("done".to_string())
    }).await?;
    Ok(serde_json::json!({"result": result.unwrap()}))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
```

## Handlers

This crate contains 15 example handlers: 11 core operations and 4 advanced features.

### Core Operations

| Binary | Source | Description |
|--------|--------|-------------|
| `closure-basic-steps` | `src/basic_steps.rs` | Checkpointed work units -- extract and validate data across steps |
| `closure-step-retries` | `src/step_retries.rs` | Automatic retry with exponential backoff on transient failures |
| `closure-typed-errors` | `src/typed_errors.rs` | Custom error types with serde serialization through step results |
| `closure-parallel` | `src/parallel.rs` | Concurrent fan-out with independent branches using `ctx.parallel()` |
| `closure-map` | `src/map.rs` | Parallel collection processing with batching via `ctx.map()` |
| `closure-child-contexts` | `src/child_contexts.rs` | Isolated subflows with independent checkpoint namespaces |
| `closure-replay-safe-logging` | `src/replay_safe_logging.rs` | Structured logging that is suppressed during replay |
| `closure-combined-workflow` | `src/combined_workflow.rs` | Multi-operation workflow combining steps, waits, and parallel execution |
| `closure-callbacks` | `src/callbacks.rs` | External signal coordination -- suspend and resume on external events |
| `closure-waits` | `src/waits.rs` | Time-based suspension with `ctx.wait()` |
| `closure-invoke` | `src/invoke.rs` | Durable Lambda-to-Lambda invocation via `ctx.invoke()` |

### Advanced Features

These handlers demonstrate features currently unique to the closure-style examples.

| Binary | Source | Description |
|--------|--------|-------------|
| `closure-saga-compensation` | `src/saga_compensation.rs` | Durable rollback with `ctx.step_with_compensation()` and `ctx.run_compensations()` |
| `closure-step-timeout` | `src/step_timeout.rs` | Per-step deadline enforcement via `StepOptions::new().timeout_seconds()` |
| `closure-conditional-retry` | `src/conditional_retry.rs` | Predicate-gated retries via `StepOptions::new().retry_if()` |
| `closure-batch-checkpoint` | `src/batch_checkpoint.rs` | Reduce checkpoint calls by 90% with `ctx.enable_batch_mode()` |

## Running

These are AWS Lambda handlers and cannot be run locally with `cargo run`. To execute them:

- **Deploy to AWS Lambda** with durable execution enabled, or
- **Write tests** using `MockDurableContext` from the `durable-lambda-testing` crate (no AWS credentials needed)

See the [root README](../../README.md) for full API documentation, testing patterns, and deployment instructions.
