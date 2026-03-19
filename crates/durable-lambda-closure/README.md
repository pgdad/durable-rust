# durable-lambda-closure

Closure-native API style for AWS Lambda durable execution workflows.

[![Docs.rs](https://docs.rs/durable-lambda-closure/badge.svg)](https://docs.rs/durable-lambda-closure)
[![Crates.io](https://img.shields.io/crates/v/durable-lambda-closure.svg)](https://crates.io/crates/durable-lambda-closure)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/pgdad/durable-rust/blob/main/LICENSE-MIT)

## Overview

`durable-lambda-closure` is the **recommended default** API style for the [durable-rust](https://github.com/pgdad/durable-rust) SDK. It provides a closure-based API for writing durable Lambda functions with the simplest possible syntax -- no traits to implement, no macros to learn, no builder chains to configure.

Write a plain async function, call `durable_lambda_closure::run(handler)`, and you're done.

### API Style Comparison

All four API styles produce identical runtime behavior. They differ only in ergonomics:

| Crate | Style | Boilerplate | Configuration | Best for |
|---|---|---|---|---|
| **`durable-lambda-closure`** | **Closure-native** | **Minimal** | **None** | **Getting started, most use cases** |
| [`durable-lambda-macro`](https://crates.io/crates/durable-lambda-macro) | Proc-macro | Lowest | None | Zero-boilerplate preference |
| [`durable-lambda-trait`](https://crates.io/crates/durable-lambda-trait) | Trait-based | Moderate | Via struct fields | Complex handlers with shared state |
| [`durable-lambda-builder`](https://crates.io/crates/durable-lambda-builder) | Builder-pattern | Moderate | `.with_tracing()`, `.with_error_handler()` | Production deployments needing hooks |

Choose `durable-lambda-closure` when you want the most straightforward API with no ceremony.

## Features

- **Simple async function handler** -- no traits, no macros, no builders
- **`ClosureContext`** wrapping `DurableContext` with all 8 durable operations
- **`durable_lambda_closure::run(handler)`** single entry point handling all runtime wiring
- **`prelude` module** re-exporting all types for single-line imports
- **Full access to all durable operations:** Step, Wait, Callback, Invoke, Parallel, Map, Child Context, Logging
- **Advanced features:** step timeout, conditional retry, batch checkpoint, saga/compensation

## Getting Started

Add to your `Cargo.toml`:

```toml
[dependencies]
durable-lambda-closure = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## Usage

### Basic Handler

```rust
use durable_lambda_closure::prelude::*;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Step: checkpointed work unit
    let order: Result<serde_json::Value, String> = ctx.step("validate", || async {
        Ok(serde_json::json!({"order_id": 42, "valid": true}))
    }).await?;

    // Step with retries
    let payment: Result<String, String> = ctx.step_with_options(
        "charge",
        StepOptions::new().retries(3).backoff_seconds(5),
        || async { Ok("tx-abc-123".to_string()) },
    ).await?;

    Ok(serde_json::json!({
        "order": order.unwrap(),
        "transaction": payment.unwrap(),
    }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
```

### All 8 Operations

```rust
use durable_lambda_closure::prelude::*;
use durable_lambda_core::types::{CallbackOptions, ParallelOptions, MapOptions};
use std::pin::Pin;
use std::future::Future;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // 1. Step -- checkpointed work
    let result: Result<i32, String> = ctx.step("work", || async { Ok(42) }).await?;

    // 2. Wait -- time-based suspension
    ctx.wait("pause", 10).await?;

    // 3. Callback -- external signal coordination
    let handle = ctx.create_callback("approval", CallbackOptions::new()
        .timeout_seconds(300)
    ).await?;
    let approval: String = ctx.callback_result(&handle)?;

    // 4. Invoke -- Lambda-to-Lambda
    let invoked: serde_json::Value = ctx.invoke(
        "process",
        "other-lambda-function",
        &serde_json::json!({"input": "data"}),
    ).await?;

    // 5. Parallel -- concurrent fan-out
    type BranchFn = Box<dyn FnOnce(DurableContext)
        -> Pin<Box<dyn Future<Output = Result<i32, DurableError>> + Send>> + Send>;
    let branches: Vec<BranchFn> = vec![
        Box::new(|mut ctx| Box::pin(async move {
            let r: Result<i32, String> = ctx.step("a", || async { Ok(10) }).await?;
            Ok(r.unwrap())
        })),
    ];
    let parallel_result = ctx.parallel("fan_out", branches, ParallelOptions::new()).await?;

    // 6. Map -- parallel collection processing
    let items = vec![1, 2, 3];
    let map_result = ctx.map(
        "double",
        items,
        MapOptions::new().batch_size(2),
        |item: i32, mut child_ctx: DurableContext| async move {
            let r: Result<i32, String> = child_ctx.step("calc", || async move {
                Ok(item * 2)
            }).await?;
            Ok(r.unwrap())
        },
    ).await?;

    // 7. Child Context -- isolated subflow
    let sub_result: i32 = ctx.child_context(
        "subflow",
        |mut child_ctx: DurableContext| async move {
            let r: Result<i32, String> = child_ctx.step("inner", || async { Ok(99) }).await?;
            Ok(r.unwrap())
        },
    ).await?;

    // 8. Logging -- replay-safe
    ctx.log("handler complete");
    ctx.log_with_data("summary", &serde_json::json!({"total": result.unwrap()}));

    Ok(serde_json::json!({"status": "done"}))
}
```

### Step Timeout

Enforce per-step deadlines:

```rust
let result: Result<String, String> = ctx.step_with_options(
    "external_call",
    StepOptions::new().timeout_seconds(10),
    || async { Ok("response".to_string()) },
).await?;
```

### Conditional Retry

Only retry when a predicate returns true:

```rust
let result: Result<String, String> = ctx.step_with_options(
    "api_call",
    StepOptions::new()
        .retries(3)
        .retry_if(|e: &String| e.contains("timeout")),
    || async { Ok("success".to_string()) },
).await?;
```

### Batch Checkpoint

Reduce checkpoint calls for sequential steps:

```rust
ctx.enable_batch_mode();

let a: Result<i32, String> = ctx.step("step_a", || async { Ok(1) }).await?;
let b: Result<i32, String> = ctx.step("step_b", || async { Ok(2) }).await?;
let c: Result<i32, String> = ctx.step("step_c", || async { Ok(3) }).await?;
```

### Saga / Compensation

Register rollback closures alongside forward operations:

```rust
ctx.step_with_compensation(
    "charge_payment",
    || async { Ok::<_, String>("tx-123".to_string()) },
    || async { Ok::<_, String>(()) },
).await?;

// On failure, roll back in reverse order
ctx.run_compensations().await?;
```

## Prelude

Import everything you need with a single line:

```rust
use durable_lambda_closure::prelude::*;
```

This re-exports `ClosureContext`, `DurableContext`, `DurableError`, `StepOptions`, `ExecutionMode`, and all other commonly used types.

## Testing

Test your handlers with [`durable-lambda-testing`](https://crates.io/crates/durable-lambda-testing) -- no AWS credentials needed:

```rust
use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_handler_replays() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("validate", r#"{"order_id": 42}"#)
        .with_step_result("charge", r#""tx-abc-123""#)
        .build()
        .await;

    // Your handler logic works with DurableContext directly
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
| `ClosureContext` | Wrapper context with all 8 durable operations |
| `run(handler)` | Entry point that wires up Lambda runtime and AWS backend |

Re-exported from `durable-lambda-core`:

| Type | Description |
|---|---|
| `DurableContext` | Core context type (used in parallel/map/child_context callbacks) |
| `DurableError` | SDK infrastructure error type |
| `StepOptions` | Step configuration (retries, backoff, timeout, retry_if) |
| `ExecutionMode` | Replaying or Executing |

Full API documentation: [docs.rs/durable-lambda-closure](https://docs.rs/durable-lambda-closure)

## License

Licensed under MIT. See [LICENSE-MIT](https://github.com/pgdad/durable-rust/blob/main/LICENSE-MIT) for details.

## Repository

[https://github.com/pgdad/durable-rust](https://github.com/pgdad/durable-rust)
