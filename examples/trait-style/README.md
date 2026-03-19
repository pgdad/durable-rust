# Trait-Style Examples

The trait-style API defines handlers by implementing the `DurableHandler` trait on a struct. This is the standard Rust trait-object pattern.

**When to choose this style:** You prefer explicit trait implementations, want to store handler state on a struct, or are familiar with the trait-object pattern from other Rust frameworks.

## Quick Start

```bash
cargo build -p trait-style-example
```

## Handler Pattern

```rust
use async_trait::async_trait;
use durable_lambda_trait::prelude::*;

struct MyHandler;

#[async_trait]
impl DurableHandler for MyHandler {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        let result: Result<String, String> = ctx.step("work", || async {
            Ok("done".to_string())
        }).await?;
        Ok(serde_json::json!({"result": result.unwrap()}))
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(MyHandler).await
}
```

## Handlers

This crate contains 11 example handlers covering all core durable operations.

| Binary | Source | Description |
|--------|--------|-------------|
| `trait-basic-steps` | `src/basic_steps.rs` | Checkpointed work units -- extract and validate data across steps |
| `trait-step-retries` | `src/step_retries.rs` | Automatic retry with exponential backoff on transient failures |
| `trait-typed-errors` | `src/typed_errors.rs` | Custom error types with serde serialization through step results |
| `trait-parallel` | `src/parallel.rs` | Concurrent fan-out with independent branches using `ctx.parallel()` |
| `trait-map` | `src/map.rs` | Parallel collection processing with batching via `ctx.map()` |
| `trait-child-contexts` | `src/child_contexts.rs` | Isolated subflows with independent checkpoint namespaces |
| `trait-replay-safe-logging` | `src/replay_safe_logging.rs` | Structured logging that is suppressed during replay |
| `trait-combined-workflow` | `src/combined_workflow.rs` | Multi-operation workflow combining steps, waits, and parallel execution |
| `trait-callbacks` | `src/callbacks.rs` | External signal coordination -- suspend and resume on external events |
| `trait-waits` | `src/waits.rs` | Time-based suspension with `ctx.wait()` |
| `trait-invoke` | `src/invoke.rs` | Durable Lambda-to-Lambda invocation via `ctx.invoke()` |

## Running

These are AWS Lambda handlers and cannot be run locally with `cargo run`. To execute them:

- **Deploy to AWS Lambda** with durable execution enabled, or
- **Write tests** using `MockDurableContext` from the `durable-lambda-testing` crate (no AWS credentials needed)

See the [root README](../../README.md) for full API documentation, testing patterns, and deployment instructions.
