# durable-lambda-trait

Trait-based API style for AWS Lambda durable execution workflows.

[![Docs.rs](https://docs.rs/durable-lambda-trait/badge.svg)](https://docs.rs/durable-lambda-trait)
[![Crates.io](https://img.shields.io/crates/v/durable-lambda-trait.svg)](https://crates.io/crates/durable-lambda-trait)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/pgdad/durable-rust/blob/main/LICENSE-MIT)

## Overview

`durable-lambda-trait` provides a trait-based API for the [durable-rust](https://github.com/pgdad/durable-rust) SDK. Implement the `DurableHandler` trait on your struct, and use `durable_lambda_trait::run(MyHandler)` to start the Lambda runtime.

This style is ideal for complex handlers that benefit from an object-oriented pattern -- shared configuration, database clients, or other state can live as struct fields, naturally accessible inside `handle()` via `&self`.

### API Style Comparison

All four API styles produce identical runtime behavior. They differ only in ergonomics:

| Crate | Style | Boilerplate | Configuration | Best for |
|---|---|---|---|---|
| [`durable-lambda-closure`](https://crates.io/crates/durable-lambda-closure) | Closure-native (recommended) | Minimal | None | Getting started, most use cases |
| [`durable-lambda-macro`](https://crates.io/crates/durable-lambda-macro) | Proc-macro | Lowest | None | Zero-boilerplate preference |
| **`durable-lambda-trait`** | **Trait-based** | **Moderate** | **Via struct fields** | **Complex handlers with shared state** |
| [`durable-lambda-builder`](https://crates.io/crates/durable-lambda-builder) | Builder-pattern | Moderate | `.with_tracing()`, `.with_error_handler()` | Production deployments needing hooks |

Choose `durable-lambda-trait` when your handler needs shared state (config, clients, caches) or when you prefer a familiar OOP pattern.

## Features

- **`DurableHandler` trait** with `async fn handle(&self, event, ctx)` method
- **`TraitContext`** wrapping `DurableContext` with all 8 durable operations
- **`durable_lambda_trait::run(handler)`** single entry point handling all runtime wiring
- **`prelude` module** re-exporting all types for single-line imports
- **Struct fields as shared state** -- configuration, clients, and caches accessible via `&self`
- **Full access to all durable operations:** Step, Wait, Callback, Invoke, Parallel, Map, Child Context, Logging

## Getting Started

Add to your `Cargo.toml`:

```toml
[dependencies]
durable-lambda-trait = "0.1"
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

Note: `async-trait` is required because Rust does not yet support async methods in traits natively (as of this crate version).

## Usage

### Basic Handler

```rust
use durable_lambda_trait::prelude::*;
use async_trait::async_trait;

struct OrderProcessor;

#[async_trait]
impl DurableHandler for OrderProcessor {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
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
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_trait::run(OrderProcessor).await
}
```

### Handler with Shared State

The trait-based approach shines when your handler needs access to shared configuration or clients:

```rust
use durable_lambda_trait::prelude::*;
use async_trait::async_trait;

struct PaymentProcessor {
    api_key: String,
    max_retries: u32,
    environment: String,
}

#[async_trait]
impl DurableHandler for PaymentProcessor {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        // Access struct fields via &self
        ctx.log_with_data("config", &serde_json::json!({
            "environment": self.environment,
            "max_retries": self.max_retries,
        }));

        let api_key = self.api_key.clone();
        let max_retries = self.max_retries;

        let result: Result<String, String> = ctx.step_with_options(
            "charge",
            StepOptions::new().retries(max_retries as usize),
            move || {
                let key = api_key.clone();
                async move {
                    // Use the API key from struct fields
                    Ok(format!("charged with key={}", key))
                }
            },
        ).await?;

        Ok(serde_json::json!({"result": result.unwrap()}))
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    let processor = PaymentProcessor {
        api_key: std::env::var("API_KEY").unwrap_or_default(),
        max_retries: 3,
        environment: "production".to_string(),
    };
    durable_lambda_trait::run(processor).await
}
```

### All Operations

The `TraitContext` exposes the same 8 operations as every other API style:

```rust
use durable_lambda_trait::prelude::*;
use async_trait::async_trait;

struct MyHandler;

#[async_trait]
impl DurableHandler for MyHandler {
    async fn handle(
        &self,
        event: serde_json::Value,
        mut ctx: TraitContext,
    ) -> Result<serde_json::Value, DurableError> {
        // Step
        let result: Result<i32, String> = ctx.step("work", || async { Ok(42) }).await?;

        // Wait
        ctx.wait("pause", 5).await?;

        // Child Context
        let sub: i32 = ctx.child_context(
            "subflow",
            |mut child_ctx: DurableContext| async move {
                let r: Result<i32, String> = child_ctx.step("inner", || async { Ok(99) }).await?;
                Ok(r.unwrap())
            },
        ).await?;

        // Replay-safe logging
        ctx.log("all operations complete");

        Ok(serde_json::json!({"step": result.unwrap(), "sub": sub}))
    }
}
```

## Prelude

Import everything you need with a single line:

```rust
use durable_lambda_trait::prelude::*;
```

This re-exports `TraitContext`, `DurableHandler`, `DurableContext`, `DurableError`, `StepOptions`, `ExecutionMode`, and all other commonly used types.

## Testing

Test your handler implementation with [`durable-lambda-testing`](https://crates.io/crates/durable-lambda-testing) -- no AWS credentials needed:

```rust
use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_order_processor_replays() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("validate", r#"{"order_id": 42}"#)
        .with_step_result("charge", r#""tx-abc-123""#)
        .build()
        .await;

    // Test against the DurableContext directly (TraitContext wraps it)
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
| `DurableHandler` | Trait to implement on your handler struct |
| `TraitContext` | Wrapper context with all 8 durable operations |
| `run(handler)` | Entry point that wires up Lambda runtime and AWS backend |

Re-exported from `durable-lambda-core`:

| Type | Description |
|---|---|
| `DurableContext` | Core context type (used in parallel/map/child_context callbacks) |
| `DurableError` | SDK infrastructure error type |
| `StepOptions` | Step configuration (retries, backoff, timeout, retry_if) |
| `ExecutionMode` | Replaying or Executing |

Full API documentation: [docs.rs/durable-lambda-trait](https://docs.rs/durable-lambda-trait)

## License

Licensed under MIT. See [LICENSE-MIT](https://github.com/pgdad/durable-rust/blob/main/LICENSE-MIT) for details.

## Repository

[https://github.com/pgdad/durable-rust](https://github.com/pgdad/durable-rust)
