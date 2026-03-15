# Story 1.8: Container Deployment Template

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want a Dockerfile template and container build example for Lambda deployment,
So that I can package and deploy my durable Lambda as a container image.

## Acceptance Criteria

1. **Given** the examples/ directory **When** I examine the Dockerfile **Then** it uses `public.ecr.aws/lambda/provided:al2023` as the base image (FR46, NFR18) **And** it builds a Rust binary with the Lambda runtime **And** it produces a container image deployable to AWS Lambda

2. **Given** a durable Lambda handler written with the closure-native approach **When** I run `docker build -f examples/Dockerfile -t my-durable-lambda .` **Then** the container image builds successfully (FR45) **And** the image contains the compiled Lambda handler binary

3. **Given** a closure-style example handler **When** I examine `examples/closure-style/` **Then** it contains a complete, compilable durable Lambda handler using `durable_lambda_closure::prelude::*` **And** it demonstrates basic step operations with checkpointing

4. **Given** all new files added in this story **When** I run `cargo build --workspace` **Then** the workspace still compiles cleanly **And** the example handler compiles as a standalone binary

## Tasks / Subtasks

- [x] Task 1: Create the Dockerfile template in `examples/Dockerfile` (AC: #1, #2)
  - [x] 1.1: Created `examples/` directory
  - [x] 1.2: Multi-stage Dockerfile: `rust:1-bookworm` builder → `public.ecr.aws/lambda/provided:al2023` runtime
  - [x] 1.3: Builder uses `rust:1-bookworm`, builds for native target (Lambda x86_64 Linux)
  - [x] 1.4: Runtime copies binary to `${LAMBDA_RUNTIME_DIR}/bootstrap`
  - [x] 1.5: Comments explain each stage, `ARG PACKAGE` allows customization via `--build-arg`
  - [x] 1.6: Docker build not verified (Docker not available in sandbox) — Dockerfile syntax verified manually

- [x] Task 2: Create closure-style example handler in `examples/closure-style/` (AC: #3)
  - [x] 2.1: `examples/closure-style/Cargo.toml` — binary crate with path dep on durable-lambda-closure + workspace deps
  - [x] 2.2: `examples/closure-style/src/main.rs` — 3-step order processing: validate_order, charge_payment (with retries), send_confirmation
  - [x] 2.3: Root `Cargo.toml` updated: added `"examples/closure-style"` to workspace members
  - [x] 2.4: `cargo build -p closure-style-example` compiles cleanly

- [x] Task 3: Verify workspace integration (AC: #4)
  - [x] 3.1: `cargo build --workspace` — passes
  - [x] 3.2: `cargo clippy --workspace -- -D warnings` — no warnings
  - [x] 3.3: `cargo fmt --check` — passes
  - [x] 3.4: `cargo test --workspace` — all existing tests pass (no regressions)

## Dev Notes

### Critical Architecture Constraints

- **Base image**: `public.ecr.aws/lambda/provided:al2023` — official AWS Lambda runtime image. This is the ONLY supported base image per architecture doc.
- **Binary name**: The compiled binary MUST be named `bootstrap` and placed at `/var/runtime/bootstrap` — this is how provided.al2023 finds the Lambda handler.
- **Target platform**: Lambda runs on `x86_64-unknown-linux-gnu` (or `aarch64-unknown-linux-gnu` for ARM). The Dockerfile must cross-compile for the correct target.
- **lib.rs = re-exports only**: The example's source code should demonstrate the pattern users will follow — import from prelude, define handler, call `run()`.

### Dockerfile Multi-Stage Pattern

```dockerfile
# Stage 1: Build the Rust binary
FROM rust:1-bookworm AS builder

WORKDIR /usr/src/app
COPY . .

# Build for Lambda's target
RUN cargo build --release -p closure-style-example

# Stage 2: Runtime
FROM public.ecr.aws/lambda/provided:al2023

# Copy the compiled binary as the Lambda bootstrap
COPY --from=builder /usr/src/app/target/release/closure-style-example ${LAMBDA_RUNTIME_DIR}/bootstrap

CMD ["handler"]
```

Key points:
- `LAMBDA_RUNTIME_DIR` is pre-set in the provided.al2023 image (usually `/var/runtime`)
- The binary name in the COPY step must match the package name from Cargo.toml
- `CMD ["handler"]` is the Lambda handler name — for `lambda_runtime` this is ignored (the runtime handles routing internally), but it's required by the image

### Example Handler Pattern

The example should be minimal but complete — demonstrating the "pit of success" pattern:

```rust
use durable_lambda_closure::prelude::*;

async fn handler(
    event: serde_json::Value,
    mut ctx: ClosureContext,
) -> Result<serde_json::Value, DurableError> {
    // Step 1: Validate the input
    let validated = ctx.step("validate", || async {
        Ok::<_, String>(serde_json::json!({"status": "valid"}))
    }).await?;

    // Step 2: Process with retry
    let processed = ctx.step_with_options(
        "process",
        StepOptions::new().retries(3),
        || async {
            Ok::<_, String>(serde_json::json!({"processed": true}))
        },
    ).await?;

    Ok(serde_json::json!({
        "validated": validated,
        "processed": processed,
    }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    durable_lambda_closure::run(handler).await
}
```

### Example Cargo.toml Pattern

```toml
[package]
name = "closure-style-example"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "closure-style-example"
path = "src/main.rs"

[dependencies]
durable-lambda-closure = { path = "../../crates/durable-lambda-closure" }
serde_json = { workspace = true }
tokio = { workspace = true }
lambda_runtime = { workspace = true }
```

### Workspace Integration

The root `Cargo.toml` workspace members list must include the example:
```toml
members = [
    "crates/durable-lambda-core",
    # ... other crates ...
    "examples/closure-style",
]
```

### What Exists vs What Needs to Be Added

**Already exists (from Stories 1.1–1.7):**
- `durable-lambda-closure` crate with `run()` entry point and `ClosureContext`
- `durable-lambda-core` with full step operation support
- `durable-lambda-testing` with MockDurableContext
- Root `Cargo.toml` workspace definition

**Needs to be added (this story):**
- `examples/` directory
- `examples/Dockerfile` — multi-stage container build template
- `examples/closure-style/Cargo.toml` — example binary crate
- `examples/closure-style/src/main.rs` — minimal durable Lambda handler
- Root `Cargo.toml` updated to include example in workspace members

### Architecture Doc Discrepancies (IMPORTANT — Inherited)

From previous stories — always follow Python SDK / actual implementation over architecture doc:
1. **Data structure**: Uses `HashMap<String, Operation>` keyed by operation ID, NOT `Vec` with cursor
2. **Handler signature**: Takes owned `ClosureContext` (not `&mut`), receives `(event, ctx)` — matches Python SDK pattern

New for this story:
3. **Architecture shows `examples/closure-style/src/basic_steps.rs`** etc. but for this story we only need a single `main.rs` demonstrating the basic pattern. The full per-feature examples are Epic 6 scope.

### Previous Story Intelligence (Story 1.7)

- `aws-sdk-lambda` and `aws-smithy-types` needed as direct deps for AWS SDK types
- AWS SDK builder `.build()` quirks: some return Result, some return direct type
- Clippy enforces no redundant closures, proper formatting
- Review clean pass on story 1.7 — code quality bar is high
- Story 1.6 review flagged: unused `tracing` dep, silent defaulting in parsers — avoid repeating

### NFR Considerations

- **NFR3**: Memory baseline under 32MB — Rust Lambda typically uses ~16-24MB, well within target
- **NFR4**: Cold start under 100ms — Rust on al2023 typically 20-50ms, well within target
- **NFR18**: provided.al2023 runtime — must be the base image

These NFRs are inherent to Rust + al2023 and don't require special implementation — they're satisfied by default. The Dockerfile just needs to be correct.

### Testing Approach

- This story is primarily about file creation (Dockerfile, example handler) — no unit tests needed
- Verification is compilation: `cargo build --workspace` must succeed including the example
- Docker build can be verified manually if Docker is available, but is NOT required for story completion
- clippy and fmt checks on the example code

### Project Structure Notes

Files to create:
```
examples/
  Dockerfile                     — multi-stage container build
  closure-style/
    Cargo.toml                   — binary crate depending on durable-lambda-closure
    src/
      main.rs                    — minimal durable Lambda handler
```

Files to modify:
```
Cargo.toml                       — add "examples/closure-style" to workspace members
```

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.8 — acceptance criteria]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements — FR45, FR46]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements — NFR3, NFR4, NFR18]
- [Source: _bmad-output/planning-artifacts/architecture.md#Container Base Image — provided.al2023]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure — examples/ directory]
- [Source: _bmad-output/planning-artifacts/architecture.md#Development Workflow — docker build command]
- [Source: _bmad-output/implementation-artifacts/1-6-closure-native-api-approach-and-lambda-integration.md — handler signature, run() pattern]
- [Source: _bmad-output/implementation-artifacts/1-7-mockdurablecontext-and-local-testing.md — AWS SDK builder patterns]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- Multi-stage Dockerfile in `examples/Dockerfile`: `rust:1-bookworm` builder → `public.ecr.aws/lambda/provided:al2023` runtime
- `ARG PACKAGE=closure-style-example` allows building any handler via `--build-arg`
- Binary copied to `${LAMBDA_RUNTIME_DIR}/bootstrap` (Lambda's expected entrypoint)
- Closure-style example: 3-step order processing (validate, charge with retries, confirm)
- Example demonstrates `run()`, `step()`, `step_with_options()`, owned event capture pattern
- Workspace members updated to include `examples/closure-style`
- All workspace checks pass: build, clippy, fmt, tests (no regressions)

### File List

- examples/Dockerfile (new — multi-stage container build template)
- examples/closure-style/Cargo.toml (new — binary crate for example handler)
- examples/closure-style/src/main.rs (new — minimal durable Lambda handler)
- Cargo.toml (modified — added "examples/closure-style" to workspace members)

### Change Log

- 2026-03-14: Story 1.8 implemented — Dockerfile template, closure-style example handler, workspace integration. Clippy clean, fmt clean, all workspace tests pass.
