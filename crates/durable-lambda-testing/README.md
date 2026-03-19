# durable-lambda-testing

MockDurableContext and assertion helpers for testing durable Lambda handlers without AWS credentials.

[![Docs.rs](https://docs.rs/durable-lambda-testing/badge.svg)](https://docs.rs/durable-lambda-testing)
[![Crates.io](https://img.shields.io/crates/v/durable-lambda-testing.svg)](https://crates.io/crates/durable-lambda-testing)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/pgdad/durable-rust#license)

## Overview

**No AWS credentials needed.** `durable-lambda-testing` provides `MockDurableContext` and a suite of assertion helpers so you can test durable Lambda handlers entirely in-memory, without any AWS configuration or network calls.

This crate is part of the [durable-rust](https://github.com/pgdad/durable-rust) SDK. It is battle-tested -- the SDK's own test suites (28 end-to-end tests, cross-approach parity tests, and Python-Rust compliance tests) all use `MockDurableContext` exclusively.

## Features

- **`MockDurableContext` builder** for creating pre-loaded durable contexts with step results, errors, waits, callbacks, and invokes
- **Two testing modes:**
  - **Replay testing** -- pre-load results, verify closures are NOT executed, assert no checkpoints
  - **Execute testing** -- empty context, verify closures ARE executed, assert operation sequence
- **5 assertion helpers** for verifying checkpoint calls and operation sequences
- **Deterministic operation IDs** using the same blake2b algorithm as the production engine
- **Batch mode testing** support via `build_with_batch_counter()`
- **Zero external dependencies** beyond `durable-lambda-core`

## Getting Started

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
durable-lambda-testing = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

Import everything via the prelude:

```rust
use durable_lambda_testing::prelude::*;
```

The prelude re-exports `MockDurableContext`, all assertion helpers, `DurableContext`, `DurableError`, `StepOptions`, `ExecutionMode`, checkpoint recorders, and operation recorders.

## MockDurableContext Builder

`MockDurableContext` uses a builder pattern to pre-load completed operations. When you call `.build()`, it returns a tuple of `(DurableContext, CheckpointRecorder, OperationRecorder)`.

### Builder Methods

```rust
MockDurableContext::new()
    // Pre-load a successful step result (JSON string)
    .with_step_result("validate", r#"{"valid": true}"#)

    // Pre-load a failed step (error type + JSON error data)
    .with_step_error("charge", "PaymentError", r#""insufficient_funds""#)

    // Pre-load a completed wait
    .with_wait("cooldown")

    // Pre-load a completed callback (callback_id + JSON result)
    .with_callback("approval", "cb-123", r#""approved""#)

    // Pre-load a completed invoke (JSON result)
    .with_invoke("call_processor", r#"{"status": "ok"}"#)

    // Build and get context + recorders
    .build()
    .await;
```

### How It Works

- **With pre-loaded operations:** The context starts in **Replaying** mode. Step closures are NOT executed -- cached results are returned instead.
- **Without pre-loaded operations:** The context starts in **Executing** mode. Step closures are executed and their results are recorded.

Operation IDs are generated deterministically using blake2b, matching the core engine. The nth `with_step_result()` call corresponds to the nth `ctx.step()` call in your handler.

## Testing Patterns

### Replay Testing (verify cached results)

Pre-load results and verify that your handler correctly processes replayed data without re-executing closures:

```rust
use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_handler_replays_correctly() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_result("validate", r#"{"order_id": 42, "valid": true}"#)
        .with_step_result("charge", r#""tx-abc-123""#)
        .build()
        .await;

    // During replay, closures are NOT executed
    let order: Result<serde_json::Value, String> = ctx
        .step("validate", || async { panic!("not executed during replay") })
        .await
        .unwrap();
    assert_eq!(order.unwrap()["order_id"], 42);

    let payment: Result<String, String> = ctx
        .step("charge", || async { panic!("not executed during replay") })
        .await
        .unwrap();
    assert_eq!(payment.unwrap(), "tx-abc-123");

    // Verify no checkpoint API calls were made (pure replay)
    assert_no_checkpoints(&calls).await;
}
```

### Execute Testing (verify new execution)

Create an empty context and verify that closures execute and produce the expected operation sequence:

```rust
use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_handler_executes_correctly() {
    let (mut ctx, _calls, ops) = MockDurableContext::new()
        .build()
        .await;

    // No pre-loaded results -- closures ARE executed
    let result: Result<i32, String> = ctx
        .step("validate", || async { Ok(42) })
        .await
        .unwrap();
    assert_eq!(result.unwrap(), 42);

    let result: Result<String, String> = ctx
        .step("charge", || async { Ok("tx-123".to_string()) })
        .await
        .unwrap();
    assert_eq!(result.unwrap(), "tx-123");

    // Verify the operation sequence
    assert_operations(&ops, &["step:validate", "step:charge"]).await;
}
```

### Error Replay Testing

Verify that your handler correctly processes replayed errors:

```rust
use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_handler_replays_error() {
    let (mut ctx, calls, _ops) = MockDurableContext::new()
        .with_step_error("charge", "PaymentError", r#""insufficient_funds""#)
        .build()
        .await;

    let result: Result<i32, String> = ctx
        .step("charge", || async { panic!("not executed") })
        .await
        .unwrap();

    // The error is replayed from cache
    assert_eq!(result.unwrap_err(), "insufficient_funds");
    assert_no_checkpoints(&calls).await;
}
```

### Mixed Replay + Execute Testing

Pre-load some operations and let the handler execute past the replay boundary:

```rust
use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_handler_transitions_replay_to_execute() {
    let (mut ctx, _calls, ops) = MockDurableContext::new()
        .with_step_result("validate", r#"true"#)
        .build()
        .await;

    // This step replays from cache
    let _: Result<bool, String> = ctx
        .step("validate", || async { panic!("not executed") })
        .await
        .unwrap();

    // This step executes (no pre-loaded result)
    let result: Result<i32, String> = ctx
        .step("charge", || async { Ok(100) })
        .await
        .unwrap();
    assert_eq!(result.unwrap(), 100);

    // Only the executed step produces an operation record
    assert_operations(&ops, &["step:charge"]).await;
}
```

## Assertion Helpers

### `assert_no_checkpoints(calls)`

Verify that no checkpoint API calls were made. Use in replay tests to confirm pure replay behavior.

```rust
assert_no_checkpoints(&calls).await;
```

### `assert_checkpoint_count(calls, n)`

Verify the exact number of checkpoint API calls.

```rust
assert_checkpoint_count(&calls, 2).await;
```

### `assert_operations(ops, expected)`

Verify the exact operation sequence using `"type:name"` format strings.

```rust
assert_operations(&ops, &["step:validate", "step:charge"]).await;
```

### `assert_operation_names(ops, expected)`

Verify operation names only, ignoring operation types.

```rust
assert_operation_names(&ops, &["validate", "charge"]).await;
```

### `assert_operation_count(ops, n)`

Verify the total number of recorded operations.

```rust
assert_operation_count(&ops, 3).await;
```

## Batch Mode Testing

For testing batch checkpoint behavior, use `build_with_batch_counter()`:

```rust
use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_batch_mode() {
    let (mut ctx, _calls, _ops, batch_counter) = MockDurableContext::new()
        .build_with_batch_counter()
        .await;

    ctx.enable_batch_mode();

    let _: Result<i32, String> = ctx.step("s1", || async { Ok(1) }).await.unwrap();
    let _: Result<i32, String> = ctx.step("s2", || async { Ok(2) }).await.unwrap();

    ctx.flush_batch().await.unwrap();

    assert_eq!(*batch_counter.lock().await, 1);
}
```

## API Reference

### Types

| Type | Description |
|---|---|
| `MockDurableContext` | Builder for creating mock contexts with pre-loaded results |
| `CheckpointRecorder` | `Arc<Mutex<Vec<CheckpointCall>>>` -- records checkpoint API calls |
| `OperationRecorder` | `Arc<Mutex<Vec<OperationRecord>>>` -- records executed operations |
| `BatchCallCounter` | `Arc<Mutex<usize>>` -- counts batch checkpoint calls |
| `CheckpointCall` | Details of a single checkpoint API call |
| `OperationRecord` | Details of a single executed operation |

### Re-exported from `durable-lambda-core`

| Type | Description |
|---|---|
| `DurableContext` | The context type your handler receives |
| `DurableError` | SDK infrastructure error type |
| `StepOptions` | Step configuration (retries, backoff, timeout) |
| `ExecutionMode` | `Replaying` or `Executing` |

Full API documentation: [docs.rs/durable-lambda-testing](https://docs.rs/durable-lambda-testing)

## License

Licensed under either of [MIT](https://github.com/pgdad/durable-rust/blob/main/LICENSE-MIT) or [Apache-2.0](https://github.com/pgdad/durable-rust/blob/main/LICENSE-APACHE) at your option.

## Repository

[https://github.com/pgdad/durable-rust](https://github.com/pgdad/durable-rust)
