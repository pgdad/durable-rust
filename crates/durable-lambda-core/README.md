# durable-lambda-core

Core replay engine, types, and operation logic for AWS Lambda durable execution in Rust.

[![Docs.rs](https://docs.rs/durable-lambda-core/badge.svg)](https://docs.rs/durable-lambda-core)
[![Crates.io](https://img.shields.io/crates/v/durable-lambda-core.svg)](https://crates.io/crates/durable-lambda-core)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/pgdad/durable-rust#license)

## Overview

`durable-lambda-core` is the foundational crate of the [durable-rust](https://github.com/pgdad/durable-rust) SDK. It contains the replay engine, all 8 durable operations, type definitions, error types, and the backend abstraction layer.

**Most users should not depend on this crate directly.** Instead, choose one of the four ergonomic wrapper crates that provide a higher-level API:

| Crate | Style | Best for |
|---|---|---|
| [`durable-lambda-closure`](https://crates.io/crates/durable-lambda-closure) | Closure-native (recommended) | Simplest syntax, no traits or macros |
| [`durable-lambda-macro`](https://crates.io/crates/durable-lambda-macro) | Proc-macro | Zero boilerplate with `#[durable_execution]` |
| [`durable-lambda-trait`](https://crates.io/crates/durable-lambda-trait) | Trait-based | OOP pattern, shared state via struct fields |
| [`durable-lambda-builder`](https://crates.io/crates/durable-lambda-builder) | Builder-pattern | Most configurable, tracing/error hooks |

Use `durable-lambda-core` directly when you need access to internal types like `DurableContext`, `DurableBackend`, or the replay engine for custom integrations.

## Features

- **Replay engine** with deterministic state machine (Replaying -> Executing) that replays completed operations from cache and executes new ones
- **8 core durable operations:** Step, Wait, Callback, Invoke, Parallel, Map, Child Context, and Logging
- **Step options:** configurable retries, exponential backoff, per-step timeouts, and conditional retry predicates
- **Batch checkpoint mode** to reduce checkpoint API calls by up to 90% for sequential step workflows
- **Saga / compensation** support with `step_with_compensation()` for durable rollback
- **Deterministic operation IDs** using blake2b hashing, byte-for-byte compatible with the Python SDK
- **`DurableBackend` trait** abstracting all AWS API calls behind a single boundary (`RealBackend` for production, `MockBackend` for testing)
- **Full Python SDK compatibility** -- identical checkpoint protocol, operation IDs, and replay semantics

## Getting Started

Add to your `Cargo.toml`:

```toml
[dependencies]
durable-lambda-core = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
lambda_runtime = "1.1"
```

### Direct Usage with DurableContext

```rust
use durable_lambda_core::context::DurableContext;
use durable_lambda_core::error::DurableError;

async fn handler(
    event: serde_json::Value,
    mut ctx: DurableContext,
) -> Result<serde_json::Value, DurableError> {
    // Step: checkpointed work unit
    let order: Result<serde_json::Value, String> = ctx.step("validate", || async {
        Ok(serde_json::json!({"order_id": 42, "valid": true}))
    }).await?;

    // Wait: time-based suspension
    ctx.wait("cooldown", 10).await?;

    // Step with retries and backoff
    let payment: Result<String, String> = ctx.step_with_options(
        "charge",
        durable_lambda_core::types::StepOptions::new()
            .retries(3)
            .backoff_seconds(5),
        || async { Ok("tx-abc-123".to_string()) },
    ).await?;

    Ok(serde_json::json!({
        "order": order.unwrap(),
        "transaction": payment.unwrap(),
    }))
}
```

## Operations

### Step (checkpointed work)

The fundamental operation. Wraps a closure in a checkpoint -- on first execution the closure runs and the result is persisted; on replay the cached result is returned without executing the closure.

```rust
let result: Result<String, String> = ctx.step("validate", || async {
    Ok("valid".to_string())
}).await?;
```

### Step with Options

Configure retries, backoff, timeouts, and conditional retry predicates:

```rust
use durable_lambda_core::types::StepOptions;

let result: Result<i32, String> = ctx.step_with_options(
    "charge",
    StepOptions::new()
        .retries(3)
        .backoff_seconds(5)
        .timeout_seconds(30),
    || async { Ok(100) },
).await?;
```

### Wait (time-based suspension)

Suspends execution for a specified number of seconds. The wait is checkpointed -- on replay it completes immediately.

```rust
ctx.wait("cooldown", 30).await?;
```

### Callback (external signal coordination)

Creates a callback handle and suspends until an external system signals completion.

### Invoke (Lambda-to-Lambda)

Durably invokes another Lambda function with automatic checkpointing of the result.

### Parallel (concurrent fan-out)

Executes multiple branches concurrently, each with its own child `DurableContext`.

### Map (parallel collection processing)

Processes a collection in parallel with configurable batch sizes.

### Child Context (isolated subflow)

Runs an isolated subflow with its own checkpoint namespace.

### Replay-Safe Logging

All log methods are no-ops during replay, preventing duplicate log entries:

```rust
ctx.log("processing order");
ctx.log_with_data("order details", &serde_json::json!({"id": 42}));
```

## Replay Engine

The replay engine is the heart of durable execution. It maintains a `HashMap<String, Operation>` keyed by deterministic operation IDs (64 hex-character blake2b hashes).

**State machine:**

1. **Replaying** -- When the context is created with completed operations from a previous invocation, the engine starts in Replaying mode. Each `step()` call looks up its operation ID in the map and returns the cached result.
2. **Executing** -- Once all completed operations have been visited, the engine transitions to Executing mode. Subsequent `step()` calls execute the closure and checkpoint the result.

**Operation ID generation:**

- Root operations: `blake2b("{counter}")`
- Child operations: `blake2b("{parent_id}-{counter}")`

These IDs are byte-for-byte compatible with the Python Durable Lambda SDK, ensuring cross-language replay compatibility.

## DurableBackend Trait

The `DurableBackend` trait is the sole I/O boundary in the SDK. All AWS API calls flow through this trait:

- **`RealBackend`** -- Production implementation that calls the AWS Lambda durable execution APIs
- **`MockBackend`** -- Testing implementation that records calls without making network requests

To implement a custom backend (e.g., for a different cloud provider), implement the `DurableBackend` trait.

## API Reference

Key types and modules:

| Type | Description |
|---|---|
| `DurableContext` | Main context type with all 8 operations |
| `DurableError` | SDK infrastructure error type |
| `StepOptions` | Step configuration (retries, backoff, timeout, retry_if) |
| `ParallelOptions` | Parallel execution configuration |
| `MapOptions` | Map operation configuration (batch_size) |
| `CallbackOptions` | Callback configuration (timeout, heartbeat) |
| `DurableBackend` | Trait abstracting AWS API calls |
| `ExecutionMode` | Replaying or Executing state |

Full API documentation: [docs.rs/durable-lambda-core](https://docs.rs/durable-lambda-core)

## License

Licensed under either of [MIT](https://github.com/pgdad/durable-rust/blob/main/LICENSE-MIT) or [Apache-2.0](https://github.com/pgdad/durable-rust/blob/main/LICENSE-APACHE) at your option.

## Repository

[https://github.com/pgdad/durable-rust](https://github.com/pgdad/durable-rust)
