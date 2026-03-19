# durable-lambda-builder

Builder-pattern API style for AWS Lambda durable execution workflows.

[![Docs.rs](https://docs.rs/durable-lambda-builder/badge.svg)](https://docs.rs/durable-lambda-builder)
[![Crates.io](https://img.shields.io/crates/v/durable-lambda-builder.svg)](https://crates.io/crates/durable-lambda-builder)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/pgdad/durable-rust#license)

## Overview

`durable-lambda-builder` provides the most configurable API style for the [durable-rust](https://github.com/pgdad/durable-rust) SDK. Use the builder pattern to configure tracing subscribers and error handlers before starting the Lambda runtime.

Call `durable_lambda_builder::handler(closure)` to create a `DurableHandlerBuilder`, optionally configure it with `.with_tracing()` and `.with_error_handler()`, then call `.run()` to start.

### API Style Comparison

All four API styles produce identical runtime behavior. They differ only in ergonomics:

| Crate | Style | Boilerplate | Configuration | Best for |
|---|---|---|---|---|
| [`durable-lambda-closure`](https://crates.io/crates/durable-lambda-closure) | Closure-native (recommended) | Minimal | None | Getting started, most use cases |
| [`durable-lambda-macro`](https://crates.io/crates/durable-lambda-macro) | Proc-macro | Lowest | None | Zero-boilerplate preference |
| [`durable-lambda-trait`](https://crates.io/crates/durable-lambda-trait) | Trait-based | Moderate | Via struct fields | Complex handlers with shared state |
| **`durable-lambda-builder`** | **Builder-pattern** | **Moderate** | **`.with_tracing()`, `.with_error_handler()`** | **Production deployments needing hooks** |

Choose `durable-lambda-builder` when you need runtime configuration hooks -- tracing subscribers for observability, error handlers for logging/transformation, or both.

## Features

- **`DurableHandlerBuilder`** with fluent configuration API
- **`.with_tracing(subscriber)`** to install a tracing subscriber before Lambda runtime starts
- **`.with_error_handler(fn)`** to intercept and transform errors before they propagate
- **`BuilderContext`** wrapping `DurableContext` with all 8 durable operations
- **`durable_lambda_builder::handler(closure).run()`** entry point
- **`prelude` module** re-exporting all types for single-line imports
- **Full access to all durable operations:** Step, Wait, Callback, Invoke, Parallel, Map, Child Context, Logging

## Getting Started

Add to your `Cargo.toml`:

```toml
[dependencies]
durable-lambda-builder = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

For tracing support, also add:

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Usage

### Basic Handler (no configuration)

```rust
use durable_lambda_builder::prelude::*;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |event: serde_json::Value, mut ctx: BuilderContext| async move {
            let order: Result<serde_json::Value, String> = ctx.step("validate", || async {
                Ok(serde_json::json!({"order_id": 42, "valid": true}))
            }).await?;

            let payment: Result<String, String> = ctx.step("charge", || async {
                Ok("tx-abc-123".to_string())
            }).await?;

            Ok(serde_json::json!({
                "order": order.unwrap(),
                "transaction": payment.unwrap(),
            }))
        },
    )
    .run()
    .await
}
```

### With Tracing

Install a tracing subscriber before the Lambda runtime starts. The subscriber is set as the global default via `tracing::subscriber::set_global_default`:

```rust
use durable_lambda_builder::prelude::*;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |event: serde_json::Value, mut ctx: BuilderContext| async move {
            tracing::info!("Processing event");

            let result: Result<i32, String> = ctx.step("work", || async {
                tracing::debug!("Executing step");
                Ok(42)
            }).await?;

            Ok(serde_json::json!({"result": result.unwrap()}))
        },
    )
    .with_tracing(fmt().json().finish())
    .run()
    .await
}
```

### With Error Handler

Intercept, log, or transform errors before they propagate to the Lambda runtime:

```rust
use durable_lambda_builder::prelude::*;
use durable_lambda_core::error::DurableError;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |event: serde_json::Value, mut ctx: BuilderContext| async move {
            let result: Result<i32, String> = ctx.step("work", || async { Ok(42) }).await?;
            Ok(serde_json::json!({"result": result.unwrap()}))
        },
    )
    .with_error_handler(|e: DurableError| {
        // Log the error, send to monitoring, enrich with metadata, etc.
        eprintln!("Handler error: {:?}", e);
        e // return the error (optionally transformed)
    })
    .run()
    .await
}
```

### Full Production Configuration

Chain all builder methods for a fully configured production handler:

```rust
use durable_lambda_builder::prelude::*;
use durable_lambda_core::error::DurableError;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |event: serde_json::Value, mut ctx: BuilderContext| async move {
            let result: Result<i32, String> = ctx.step_with_options(
                "process",
                StepOptions::new().retries(3).backoff_seconds(2).timeout_seconds(30),
                || async { Ok(42) },
            ).await?;

            ctx.log_with_data("processed", &serde_json::json!({"result": result}));

            Ok(serde_json::json!({"result": result.unwrap()}))
        },
    )
    .with_tracing(fmt().json().finish())
    .with_error_handler(|e: DurableError| {
        eprintln!("Error: {:?}", e);
        e
    })
    .run()
    .await
}
```

### All Operations

The `BuilderContext` exposes the same 8 operations as every other API style:

```rust
use durable_lambda_builder::prelude::*;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_builder::handler(
        |event: serde_json::Value, mut ctx: BuilderContext| async move {
            // Step
            let result: Result<i32, String> = ctx.step("work", || async { Ok(42) }).await?;

            // Wait
            ctx.wait("pause", 5).await?;

            // Child Context
            let sub: i32 = ctx.child_context(
                "subflow",
                |mut child_ctx: DurableContext| async move {
                    let r: Result<i32, String> =
                        child_ctx.step("inner", || async { Ok(99) }).await?;
                    Ok(r.unwrap())
                },
            ).await?;

            // Replay-safe logging
            ctx.log("all operations complete");

            Ok(serde_json::json!({"step": result.unwrap(), "sub": sub}))
        },
    )
    .run()
    .await
}
```

## Prelude

Import everything you need with a single line:

```rust
use durable_lambda_builder::prelude::*;
```

This re-exports `BuilderContext`, `DurableHandlerBuilder`, `DurableContext`, `DurableError`, `StepOptions`, `ExecutionMode`, and all other commonly used types.

## Testing

Test your handler logic with [`durable-lambda-testing`](https://crates.io/crates/durable-lambda-testing) -- no AWS credentials needed:

```rust
use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_handler_replays() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("validate", r#"{"order_id": 42}"#)
        .with_step_result("charge", r#""tx-abc-123""#)
        .build()
        .await;

    // Test against the DurableContext directly (BuilderContext wraps it)
    let order: Result<serde_json::Value, String> = ctx
        .step("validate", || async { panic!("not executed") })
        .await.unwrap();
    assert_eq!(order.unwrap()["order_id"], 42);

    assert_no_checkpoints(&calls).await;
}
```

## API Reference

| Type | Description |
|---|---|
| `DurableHandlerBuilder` | Builder with `.with_tracing()`, `.with_error_handler()`, `.run()` |
| `BuilderContext` | Wrapper context with all 8 durable operations |
| `handler(closure)` | Creates a new `DurableHandlerBuilder` |

Re-exported from `durable-lambda-core`:

| Type | Description |
|---|---|
| `DurableContext` | Core context type (used in parallel/map/child_context callbacks) |
| `DurableError` | SDK infrastructure error type |
| `StepOptions` | Step configuration (retries, backoff, timeout, retry_if) |
| `ExecutionMode` | Replaying or Executing |

Full API documentation: [docs.rs/durable-lambda-builder](https://docs.rs/durable-lambda-builder)

## License

Licensed under either of [MIT](https://github.com/pgdad/durable-rust/blob/main/LICENSE-MIT) or [Apache-2.0](https://github.com/pgdad/durable-rust/blob/main/LICENSE-APACHE) at your option.

## Repository

[https://github.com/pgdad/durable-rust](https://github.com/pgdad/durable-rust)
