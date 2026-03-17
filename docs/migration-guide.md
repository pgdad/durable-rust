# Python-to-Rust Migration Guide

Migrate your AWS Durable Lambda handlers from the Python SDK to the Rust SDK.

**Target audience:** Senior Python developer with some Rust exposure. This guide assumes you know Python durable Lambda patterns well and have done a Rust tutorial. It provides conceptual bridges — not Rust basics.

## Conceptual Mapping Table

| Python SDK | Rust SDK (closure-native) | Notes |
|---|---|---|
| `@durable_execution` decorator | `durable_lambda_closure::run()` | Entry point for handler registration |
| `context.call_activity("name", fn)` | `ctx.step("name", \|\| async { ... }).await?` | Checkpointed work unit |
| `context.call_activity` with retry config | `ctx.step_with_options("name", StepOptions::new().retries(3), \|\| async { ... }).await?` | Server-side retries |
| `context.create_wait("name", secs)` | `ctx.wait("name", secs).await?` | Time-based suspension |
| `context.create_callback("name", opts)` | `ctx.create_callback("name", CallbackOptions::new()).await?` | External signal coordination |
| `context.get_callback_result(handle)` | `ctx.callback_result(&handle)?` | Retrieve callback outcome |
| `context.invoke("name", fn, payload)` | `ctx.invoke("name", "fn-name", &payload).await?` | Lambda-to-Lambda invocation |
| `context.parallel("name", branches)` | `ctx.parallel("name", branches, ParallelOptions::new()).await?` | Concurrent fan-out |
| `context.map("name", items, fn)` | `ctx.map("name", items, MapOptions::new(), \|item, ctx\| async move { ... }).await?` | Parallel collection processing |
| `context.child_context("name", fn)` | `ctx.child_context("name", \|mut ctx\| async move { ... }).await?` | Isolated subflow |
| `context.logger.info("msg")` | `ctx.log("msg")` | Replay-safe logging (no-op during replay) |
| `MockContext()` | `MockDurableContext::new().build().await` | Testing mock |
| `import durable_execution` | `use durable_lambda_closure::prelude::*;` | Single import for everything |
| `requirements.txt` / `pip install` | `Cargo.toml` / `cargo build` | Dependency management |
| `python:3.x` Docker base | `rust:1.x` build + `al2023` runtime | Container deployment |

## Handler Registration

### Python

```python
from aws_durable_execution import durable_execution

@durable_execution
async def handler(event, context):
    result = await context.call_activity("validate", validate_order, event)
    return {"status": "ok", "result": result}
```

### Rust (closure-native — recommended default)

```rust
use durable_lambda_closure::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run(|event: serde_json::Value, mut ctx: ClosureContext| async move {
        let result: Result<String, String> = ctx.step("validate", || async {
            Ok("validated".to_string())
        }).await?;
        Ok(serde_json::json!({"status": "ok", "result": result.unwrap()}))
    }).await
}
```

### Other Rust API Styles

The Rust SDK offers 4 API approaches. All are behaviorally identical — choose based on preference:

| Approach | Crate | Best for |
|---|---|---|
| **Closure-native** (recommended) | `durable-lambda-closure` | Most Pythonic feel, inline handlers |
| Trait-based | `durable-lambda-trait` | Teams preferring `impl Handler` pattern |
| Builder-pattern | `durable-lambda-builder` | Fluent API fans |
| Proc-macro | `durable-lambda-macro` | Minimal boilerplate with `#[durable_execution]` |

## Core Operations

### Step (Basic)

**Python:**
```python
result = await context.call_activity("validate_order", validate_fn, order_data)
```

**Rust:**
```rust
let result: Result<OrderValidation, String> = ctx.step("validate_order", || async {
    // Your business logic here
    Ok(OrderValidation { order_id: 42, valid: true })
}).await?;

match result {
    Ok(validation) => println!("Valid: {}", validation.valid),
    Err(e) => println!("Failed: {e}"),
}
```

**Key difference:** Rust `step()` returns `Result<Result<T, E>, DurableError>`. The outer `Result` is for SDK errors (checkpoint failures). The inner `Result` is your business logic error — both `Ok` and `Err` values are checkpointed.

### Step with Retries

**Python:**
```python
result = await context.call_activity(
    "charge_payment",
    charge_fn,
    payment_data,
    retry_policy=RetryPolicy(max_retries=3, backoff_seconds=5)
)
```

**Rust:**
```rust
use durable_lambda_closure::prelude::*;

let result: Result<String, String> = ctx.step_with_options(
    "charge_payment",
    StepOptions::new().retries(3).backoff_seconds(5),
    || async {
        Ok("charged".to_string())
    },
).await?;
```

**Key difference:** When a retry is needed, the Rust SDK returns `Err(DurableError::StepRetryScheduled)`. Your handler must propagate this via `?` so the Lambda exits cleanly. The server re-invokes after the delay.

### Wait

**Python:**
```python
await context.create_wait("cooldown", duration_seconds=30)
# Execution continues here after 30 seconds
```

**Rust:**
```rust
ctx.wait("cooldown", 30).await?;
// Execution continues here after 30 seconds
```

**Key difference:** On first execution, `wait()` returns `Err(DurableError::WaitSuspended)`. Propagate with `?`. The server re-invokes after the timer. On replay, `wait()` returns `Ok(())` immediately.

### Callback

**Python:**
```python
handle = await context.create_callback("approval", timeout_seconds=300)
callback_id = handle.callback_id
# ... share callback_id with external system ...
result = await context.get_callback_result(handle)
```

**Rust:**
```rust
use durable_lambda_closure::prelude::*;

let handle = ctx.create_callback("approval", CallbackOptions::new().timeout_seconds(300)).await?;
println!("Share this with external system: {}", handle.callback_id);

// Check result — suspends if not yet signaled
let result: String = ctx.callback_result(&handle)?;
```

**Key difference:** `create_callback` never suspends. `callback_result` returns `Err(DurableError::CallbackSuspended)` if not yet signaled — propagate with `?` to exit. The server re-invokes when the callback is signaled.

### Invoke

**Python:**
```python
result = await context.invoke(
    "call_processor",
    function_name="payment-processor-lambda",
    payload={"order_id": 123}
)
```

**Rust:**
```rust
let result: serde_json::Value = ctx.invoke(
    "call_processor",
    "payment-processor-lambda",
    &serde_json::json!({"order_id": 123}),
).await?;
```

**Key difference:** Returns `Err(DurableError::InvokeSuspended)` while the target executes. Propagate with `?`. If the target completes instantly (detected via the double-check pattern), the result is returned directly.

### Parallel

**Python:**
```python
async def branch_a(ctx):
    return await ctx.call_activity("work_a", do_work_a)

async def branch_b(ctx):
    return await ctx.call_activity("work_b", do_work_b)

result = await context.parallel("fan_out", [branch_a, branch_b])
```

**Rust:**
```rust
use durable_lambda_closure::prelude::*;
use durable_lambda_core::context::DurableContext;
use std::pin::Pin;
use std::future::Future;

type BranchFn = Box<dyn FnOnce(DurableContext)
    -> Pin<Box<dyn Future<Output = Result<serde_json::Value, DurableError>> + Send>>
    + Send>;

let branches: Vec<BranchFn> = vec![
    Box::new(|mut ctx| Box::pin(async move {
        let r: Result<String, String> = ctx.step("work_a", || async {
            Ok("result_a".to_string())
        }).await?;
        Ok(serde_json::json!(r.unwrap()))
    })),
    Box::new(|mut ctx| Box::pin(async move {
        let r: Result<String, String> = ctx.step("work_b", || async {
            Ok("result_b".to_string())
        }).await?;
        Ok(serde_json::json!(r.unwrap()))
    })),
];

let result = ctx.parallel("fan_out", branches, ParallelOptions::new()).await?;
// result.results[0].result, result.results[1].result, etc.
```

**Key difference:** Branch closures receive an owned `DurableContext` (not `ClosureContext`) and must satisfy `Send + 'static` because they run via `tokio::spawn`. The type alias for `BranchFn` keeps signatures manageable.

### Map

**Python:**
```python
items = [1, 2, 3]
result = await context.map("process_items", items, process_fn)
```

**Rust:**
```rust
use durable_lambda_closure::prelude::*;
use durable_lambda_core::context::DurableContext;

let items = vec![1, 2, 3];
let result = ctx.map(
    "process_items",
    items,
    MapOptions::new().batch_size(10),
    |item: i32, mut child_ctx: DurableContext| async move {
        let r: Result<i32, String> = child_ctx.step("double", || async move {
            Ok(item * 2)
        }).await?;
        Ok(r.unwrap())
    },
).await?;

for batch_item in &result.results {
    println!("Item {}: {:?}", batch_item.index, batch_item.result);
}
```

**Key difference:** The closure must be `Clone` (applied to each item independently). `batch_size` controls concurrency — each batch completes before the next starts.

### Child Context

**Python:**
```python
result = await context.child_context("sub_workflow", sub_workflow_fn)
```

**Rust:**
```rust
use durable_lambda_core::context::DurableContext;

let result: i32 = ctx.child_context("sub_workflow", |mut child_ctx: DurableContext| async move {
    let r: Result<i32, String> = child_ctx.step("inner_step", || async {
        Ok(42)
    }).await?;
    Ok(r.unwrap())
}).await?;
```

**Key difference:** The closure receives an owned `DurableContext` with an isolated checkpoint namespace. Operations inside don't collide with parent or sibling contexts.

### Logging

**Python:**
```python
context.logger.info("Order processing started")
context.logger.warn("Inventory low")
context.logger.error("Payment failed")
context.logger.debug("Validating fields")
```

**Rust:**
```rust
ctx.log("Order processing started");
ctx.log_warn("Inventory low");
ctx.log_error("Payment failed");
ctx.log_debug("Validating fields");

// With structured data
ctx.log_with_data("Order processed", &serde_json::json!({"order_id": 42}));
```

**Key difference:** All log methods are no-ops during replay — no duplicate log output. Structured data variants (`log_with_data`, `log_error_with_data`, etc.) accept `serde_json::Value`.

## Testing

### Python

```python
from unittest.mock import MagicMock

mock_context = MagicMock()
mock_context.call_activity.return_value = {"validated": True}

result = await handler(event, mock_context)
assert result["status"] == "ok"
```

### Rust

```rust
use durable_lambda_testing::prelude::*;

#[tokio::test]
async fn test_handler() {
    let (mut ctx, calls, ops) = MockDurableContext::new()
        .with_step_result("validate_order", &serde_json::json!({"valid": true}))
        .build()
        .await;

    // Run your handler logic using ctx...
    let result: Result<serde_json::Value, String> = ctx.step("validate_order", || async {
        Ok(serde_json::json!({"valid": true}))
    }).await.unwrap();

    // Assert operation sequence
    assert_operations(&ops, &["step:validate_order"]).await;

    // Assert checkpoint count
    assert_checkpoint_count(&calls, 2).await; // START + SUCCEED
}
```

**Key difference:** `MockDurableContext` uses a builder pattern to pre-load replay data. The mock returns pre-configured results during replay without hitting AWS. `ops` records the operation sequence for verification.

### Assertion Helpers

| Python | Rust |
|---|---|
| `assert mock.call_activity.call_count == 2` | `assert_operation_count(&ops, 2).await` |
| `assert mock.call_activity.call_args_list[0][0] == "step_name"` | `assert_operation_names(&ops, &["step_name"]).await` |
| Custom assertions | `assert_operations(&ops, &["step:validate", "wait:cooldown"]).await` |

## Deployment

### Container Image

Both SDKs deploy as container images. The build process differs:

**Python Dockerfile:**
```dockerfile
FROM public.ecr.aws/lambda/python:3.12
COPY requirements.txt .
RUN pip install -r requirements.txt
COPY handler.py .
CMD ["handler.handler"]
```

**Rust Dockerfile:**
```dockerfile
# Build stage
FROM rust:1.82 AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

# Runtime stage
FROM public.ecr.aws/lambda/provided:al2023
COPY --from=builder /build/target/release/my-handler ${LAMBDA_RUNTIME_DIR}/bootstrap
CMD ["bootstrap"]
```

### Lambda Configuration Differences

| Setting | Python | Rust |
|---|---|---|
| Runtime | `python3.12` | `provided.al2023` |
| Handler | `handler.handler` | `bootstrap` (binary name) |
| Memory | 256-512 MB typical | 128-256 MB typical (lower due to compiled binary) |
| Cold start | 1-3 seconds | 50-200 ms |
| Package | ZIP or container | Container (recommended) |

## Gotchas

### 1. Determinism — Non-Durable Code Re-executes on Replay

Code outside durable operations runs on EVERY invocation (including replays). Non-deterministic code produces different values each time.

**Wrong:**
```rust
// BAD: SystemTime changes on each invocation
let timestamp = std::time::SystemTime::now();
ctx.step("process", || async move {
    process_with_timestamp(timestamp) // Different timestamp on replay!
}).await?;
```

**Right:**
```rust
// GOOD: Use event data or checkpoint the timestamp
let timestamp: Result<String, String> = ctx.step("get_timestamp", || async {
    Ok(chrono::Utc::now().to_rfc3339())
}).await?;

ctx.step("process", || async move {
    process_with_timestamp(&timestamp.unwrap())
}).await?;
```

**Rule:** If a value must be the same across replays, checkpoint it inside a `step()`.

### Python Determinism Anti-Patterns in Rust

Python durable execution silently serializes many non-deterministic values. Rust requires you to be explicit. These Python patterns cause replay failures in Rust:

| Python Pattern | Why It Works in Python | Rust Equivalent (Correct) |
|----------------|----------------------|--------------------------|
| `datetime.now()` outside activity | Python SDK sometimes serializes it automatically | `ctx.step("ts", \|\| async { Ok(Utc::now()) }).await?` |
| `uuid.uuid4()` outside activity | Python value happens to be deterministic per-session | `ctx.step("id", \|\| async { Ok(Uuid::new_v4()) }).await?` |
| `random.random()` outside activity | Python may checkpoint the value implicitly | `ctx.step("rng", \|\| async { Ok(rand::random::<f64>()) }).await?` |
| Branching on external env vars | Env vars stable per container instance in Python | Read env vars in a `step()` if they affect workflow branching |

**The rule:** If a value must be the same across all replays of a workflow execution, it must be produced inside a `ctx.step()` so it is checkpointed.

### 2. Send + 'static Bounds — Parallel/Map Closures Must Be Sendable

Branch closures in `parallel()` and item closures in `map()` run on separate Tokio tasks. They must satisfy `Send + 'static`.

**Wrong:**
```rust
let local_data = &some_parent_data; // borrowed reference

let branches = vec![
    Box::new(|mut ctx| Box::pin(async move {
        // ERROR: `local_data` is a reference, not owned — violates 'static
        process(local_data);
        Ok(1)
    })),
];
```

**Right:**
```rust
let owned_data = some_parent_data.clone(); // owned copy

let branches = vec![
    Box::new(move |mut ctx| Box::pin(async move {
        // OK: `owned_data` is moved into the closure — satisfies Send + 'static
        process(&owned_data);
        Ok(1)
    })),
];
```

**Rule:** Clone data into branch closures. Use `move` to transfer ownership.

### 3. Owned Data in Closures — Rust Ownership in Async Closures

Rust's ownership rules apply inside async closures. You can't borrow data across `.await` points in closures that need `Send`.

**Wrong:**
```rust
let order = get_order();
let order_ref = &order;

ctx.step("validate", || async {
    // ERROR: cannot borrow `order` across await — closure captures reference
    validate(order_ref).await
}).await?;
```

**Right:**
```rust
let order = get_order();
let order_clone = order.clone();

ctx.step("validate", || async move {
    // OK: `order_clone` is moved into the async block
    validate(&order_clone).await
}).await?;
```

**Rule:** Use `move` on async blocks and `clone()` values before passing them in.

### 4. Serde Bounds — Checkpoint Types Must Be Serializable

Every type passed through `step()`, `invoke()`, `parallel()`, `map()`, or `child_context()` must implement `Serialize` and `DeserializeOwned`. The SDK checkpoints these values as JSON.

**Wrong:**
```rust
struct Order {
    id: u64,
    // No Serialize/Deserialize derives!
}

let result: Result<Order, String> = ctx.step("validate", || async {
    Ok(Order { id: 42 }) // ERROR: `Order` does not implement `Serialize`
}).await?;
```

**Right:**
```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Order {
    id: u64,
}

let result: Result<Order, String> = ctx.step("validate", || async {
    Ok(Order { id: 42 }) // OK: Order implements Serialize + Deserialize
}).await?;
```

**Rule:** Add `#[derive(Serialize, Deserialize)]` to every type that flows through durable operations. Both success AND error types need it.

## Quick Reference Card

```
Python                              Rust (closure-native)
─────────────────────────────────   ─────────────────────────────────
@durable_execution                  run(|event, mut ctx| async move { ... })
context.call_activity("n", fn)      ctx.step("n", || async { ... }).await?
context.create_wait("n", secs)      ctx.wait("n", secs).await?
context.create_callback("n", o)     ctx.create_callback("n", opts).await?
context.get_callback_result(h)      ctx.callback_result(&h)?
context.invoke("n", fn, p)          ctx.invoke("n", "fn", &p).await?
context.parallel("n", branches)     ctx.parallel("n", branches, opts).await?
context.map("n", items, fn)         ctx.map("n", items, opts, |i, ctx| async move { ... }).await?
context.child_context("n", fn)      ctx.child_context("n", |mut ctx| async move { ... }).await?
context.logger.info("msg")          ctx.log("msg")
MockContext()                        MockDurableContext::new().build().await
pip install aws-durable-execution   cargo add durable-lambda-closure
```
