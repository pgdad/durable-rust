# Builder-Style Examples

The builder-style API uses a fluent builder pattern: `durable_lambda_builder::handler(closure).run().await`. The handler is defined as an inline closure passed to the builder.

**When to choose this style:** You want inline handler definitions without a separate function, or you need to chain configuration like `.with_tracing()` or `.with_error_handler()` on the builder.

## Quick Start

```bash
cargo build -p builder-style-example
```

## Handler Pattern

```rust
use durable_lambda_builder::prelude::*;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |event: serde_json::Value, mut ctx: BuilderContext| async move {
            let result: Result<String, String> = ctx.step("work", || async {
                Ok("done".to_string())
            }).await?;
            Ok(serde_json::json!({"result": result.unwrap()}))
        },
    )
    .run()
    .await
}
```

## Handlers

This crate contains 11 example handlers covering all core durable operations.

| Binary | Source | Description |
|--------|--------|-------------|
| `builder-basic-steps` | `src/basic_steps.rs` | Checkpointed work units -- extract and validate data across steps |
| `builder-step-retries` | `src/step_retries.rs` | Automatic retry with exponential backoff on transient failures |
| `builder-typed-errors` | `src/typed_errors.rs` | Custom error types with serde serialization through step results |
| `builder-parallel` | `src/parallel.rs` | Concurrent fan-out with independent branches using `ctx.parallel()` |
| `builder-map` | `src/map.rs` | Parallel collection processing with batching via `ctx.map()` |
| `builder-child-contexts` | `src/child_contexts.rs` | Isolated subflows with independent checkpoint namespaces |
| `builder-replay-safe-logging` | `src/replay_safe_logging.rs` | Structured logging that is suppressed during replay |
| `builder-combined-workflow` | `src/combined_workflow.rs` | Multi-operation workflow combining steps, waits, and parallel execution |
| `builder-callbacks` | `src/callbacks.rs` | External signal coordination -- suspend and resume on external events |
| `builder-waits` | `src/waits.rs` | Time-based suspension with `ctx.wait()` |
| `builder-invoke` | `src/invoke.rs` | Durable Lambda-to-Lambda invocation via `ctx.invoke()` |

## Running

These are AWS Lambda handlers and cannot be run locally with `cargo run`. To execute them:

- **Deploy to AWS Lambda** with durable execution enabled, or
- **Write tests** using `MockDurableContext` from the `durable-lambda-testing` crate (no AWS credentials needed)

See the [root README](../../README.md) for full API documentation, testing patterns, and deployment instructions.
