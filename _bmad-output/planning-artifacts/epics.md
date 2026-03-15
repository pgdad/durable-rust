---
stepsCompleted: [step-01-validate-prerequisites, step-02-design-epics, step-03-create-stories, step-04-final-validation]
inputDocuments:
  - '_bmad-output/planning-artifacts/prd.md'
  - '_bmad-output/planning-artifacts/architecture.md'
---

# durable-rust - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for durable-rust, decomposing the requirements from the PRD and Architecture into implementable stories.

## Requirements Inventory

### Functional Requirements

FR1: The SDK can load the complete execution history for a durable function invocation from AWS on startup
FR2: The SDK can distinguish between replay mode and execution mode based on history cursor position
FR3: The SDK can return cached results from history during replay without re-executing the operation
FR4: The SDK can execute new operations and checkpoint their results to AWS during execution mode
FR5: The SDK can advance a positional cursor through the history log as each durable operation is encountered
FR6: The SDK can serialize and deserialize checkpoint values using serde (Serialize + DeserializeOwned)
FR7: The SDK can serialize and deserialize step errors using serde (Serialize + DeserializeOwned)
FR8: Developers can define a named step that executes a closure and checkpoints the result
FR9: Developers can configure retry behavior for a step (retry count, backoff strategy)
FR10: Developers can return typed errors from steps that are checkpointed and replayed as the exact same typed error
FR11: The SDK can skip step execution during replay and return the previously checkpointed result
FR12: Developers can suspend a durable function for a specified duration without consuming compute
FR13: The SDK can resume execution after the wait duration has elapsed
FR14: Developers can register a callback and receive a callback ID for external systems to signal
FR15: The SDK can suspend execution until a callback signal (success or failure) is received
FR16: External systems can send callback success, failure, or heartbeat signals using the callback ID
FR17: Developers can durably invoke another Lambda function and receive its result
FR18: The SDK can checkpoint the invocation result so it replays without re-invoking the target function
FR19: Developers can execute multiple branches concurrently within a single Lambda invocation
FR20: Developers can configure completion criteria for parallel operations (e.g., all, any, N-of-M)
FR21: Each parallel branch can execute durable operations independently with its own checkpoint namespace
FR22: The SDK can run parallel branches as tokio::spawn tasks with Send + 'static bounds
FR23: Developers can process a collection of items in parallel using a closure applied to each item
FR24: Developers can configure batching for map operations
FR25: The SDK can return a BatchResult<T> containing the results of all map operations
FR26: Developers can create isolated subflows with their own checkpoint namespace
FR27: Child contexts can execute any durable operation independently from the parent context
FR28: The SDK can create fully owned child contexts that share only the AWS client (Arc<LambdaService>)
FR29: Developers can emit structured log messages that are deduplicated across replays
FR30: The SDK can integrate with the tracing crate for structured logging
FR31: The SDK can suppress duplicate log output during the replay phase
FR32: Developers can use a proc-macro approach (#[durable_execution]) to define durable Lambda handlers
FR33: Developers can use a trait-based approach to define durable Lambda handlers
FR34: Developers can use a closure-native approach to define durable Lambda handlers
FR35: Developers can use a builder-pattern approach to define durable Lambda handlers
FR36: All 4 API approaches expose the same 8 core operations with identical behavior
FR37: Developers can create a MockDurableContext with pre-loaded step results for local testing
FR38: Developers can run tests against MockDurableContext without AWS credentials
FR39: Developers can verify the sequence and names of durable operations executed during a test
FR40: The compliance suite can execute identical workflows in both Python and Rust and compare outputs
FR41: Every public type, method, trait, and function has rustdoc with at least one inline example
FR42: All doc examples compile and pass via cargo test --doc
FR43: Standalone examples demonstrate every core feature across all 4 API styles
FR44: A migration guide maps Python SDK concepts to Rust equivalents with side-by-side code
FR45: Developers can package durable Lambda functions as container images
FR46: The SDK provides a Dockerfile template for Lambda container builds
FR47: The SDK integrates with the lambda_runtime crate for Lambda handler registration
FR48: The SDK can report errors via a typed DurableError enum with thiserror derives
FR49: The SDK can propagate AWS SDK errors through the DurableError type
FR50: The SDK can propagate serialization errors through the DurableError type

### NonFunctional Requirements

NFR1: SDK overhead per durable operation (replay or execute) must add < 1ms latency beyond the AWS API call itself
NFR2: Eager history loading must complete within a single get_durable_execution_history call — no pagination loops during handler execution
NFR3: Lambda memory baseline with the SDK loaded must remain under 32MB for a minimal handler (targeting 4-8x reduction vs Python's ~128MB)
NFR4: Cold start time for a Rust durable Lambda container must be under 100ms (excluding AWS container initialization)
NFR5: The replay engine must produce identical behavior to the Python SDK for identical inputs and execution sequences — zero tolerance for divergence
NFR6: Checkpoint operations must be atomic — a partially written checkpoint must not corrupt the execution history
NFR7: The SDK must handle AWS API transient failures (throttling, timeouts) with appropriate retries before surfacing errors
NFR8: Each API approach crate must depend only on durable-lambda-core, not on other approach crates — clean dependency boundaries
NFR9: Adding a new core operation to the SDK must require changes only in durable-lambda-core and each approach crate — no cross-cutting changes
NFR10: 100% test coverage across all SDK crates, measured via cargo tarpaulin or llvm-cov
NFR11: A developer following SDK patterns must encounter zero ownership/borrowing/trait-bound compiler errors related to the SDK's API surface
NFR12: Compiler error messages caused by incorrect SDK usage must be actionable — the developer must be able to identify and fix the issue without reading SDK internals
NFR13: AI coding tools (Claude Code, Copilot) must generate compilable code on first or second attempt when following SDK patterns and doc examples
NFR14: cargo build for the complete workspace must complete in under 60 seconds on a standard development machine (clean build)
NFR15: The SDK must compile on the latest stable Rust toolchain
NFR16: The SDK must be compatible with the latest stable release of aws-sdk-lambda
NFR17: The SDK must be compatible with the latest stable release of lambda_runtime
NFR18: Container images must be deployable to AWS Lambda with the provided.al2023 runtime
NFR19: The SDK must use the 9 aws-sdk-lambda durable execution API operations as its sole interface with AWS — no direct HTTP calls or undocumented APIs
NFR20: The SDK must integrate with the tracing ecosystem for logging — no custom logging framework

### Additional Requirements

- Custom Cargo Workspace with virtual manifest — no starter template exists; hand-crafted workspace initialization required
- Workspace-level [workspace.dependencies] for shared dependency version management
- crates/ directory with flat layout housing all 6 crates
- DurableBackend trait as the I/O boundary between replay engine and AWS/mock backends
- Positional Vec<HistoryEntry> with usize cursor for replay — matches Python SDK sequential model
- JSON checkpoint serialization via serde_json — must match Python SDK format exactly for compliance
- Thin wrapper pattern: approach crates provide ergonomic sugar over core's DurableContext
- Each approach crate provides a run() entry point that wires up lambda_runtime + DurableContext internally
- #[durable_execution] attribute macro design (mirrors #[tokio::main] pattern)
- User step error type E requires only Serialize + DeserializeOwned bounds (not std::error::Error)
- DurableError flat enum with constructor methods — never raw struct construction
- GitHub Actions CI/CD platform
- CI pipeline: cargo fmt --check, cargo clippy -- -D warnings, cargo build --workspace, cargo test --workspace, cargo test --doc, cargo llvm-cov with threshold, example container builds
- Container base image: public.ecr.aws/lambda/provided:al2023
- cargo-llvm-cov for coverage (preferred over tarpaulin for macOS compatibility)
- Module organization: lib.rs = re-exports only, logic in dedicated files
- One file per operation in operations/ module
- Parameter ordering convention: name first, options second (when applicable), closure last
- Prelude module in each approach crate for single-import pattern: use durable_lambda_closure::prelude::*
- Compliance suite in top-level compliance/ directory with Python + Rust implementations
- Test naming convention: test_{operation}_{behavior}_{condition}
- Python SDK as behavioral reference: aws/aws-durable-execution-sdk-python

### UX Design Requirements

No UX Design document — this is a library/SDK project with no user interface.

### FR Coverage Map

FR1: Epic 1 - Load complete execution history from AWS on startup
FR2: Epic 1 - Distinguish between replay mode and execution mode
FR3: Epic 1 - Return cached results from history during replay
FR4: Epic 1 - Execute new operations and checkpoint results to AWS
FR5: Epic 1 - Advance positional cursor through history log
FR6: Epic 1 - Serialize/deserialize checkpoint values via serde
FR7: Epic 1 - Serialize/deserialize step errors via serde
FR8: Epic 1 - Define named step with closure and checkpoint
FR9: Epic 1 - Configure retry behavior for steps
FR10: Epic 1 - Return typed errors from steps (checkpointed and replayed)
FR11: Epic 1 - Skip step execution during replay, return cached result
FR12: Epic 2 - Suspend durable function for specified duration
FR13: Epic 2 - Resume execution after wait duration elapsed
FR14: Epic 2 - Register callback and receive callback ID
FR15: Epic 2 - Suspend execution until callback signal received
FR16: Epic 2 - External systems send callback signals via callback ID
FR17: Epic 2 - Durably invoke another Lambda function
FR18: Epic 2 - Checkpoint invocation result for replay
FR19: Epic 3 - Execute multiple branches concurrently
FR20: Epic 3 - Configure completion criteria for parallel operations
FR21: Epic 3 - Parallel branches with independent checkpoint namespaces
FR22: Epic 3 - Run parallel branches as tokio::spawn tasks
FR23: Epic 3 - Process collection items in parallel via closure
FR24: Epic 3 - Configure batching for map operations
FR25: Epic 3 - Return BatchResult<T> from map operations
FR26: Epic 3 - Create isolated subflows with own checkpoint namespace
FR27: Epic 3 - Child contexts execute durable operations independently
FR28: Epic 3 - Fully owned child contexts sharing only Arc<LambdaService>
FR29: Epic 3 - Emit structured log messages deduplicated across replays
FR30: Epic 3 - Integrate with tracing crate for structured logging
FR31: Epic 3 - Suppress duplicate log output during replay phase
FR32: Epic 4 - Proc-macro approach (#[durable_execution])
FR33: Epic 4 - Trait-based approach
FR34: Epic 1 - Closure-native approach (first approach delivered)
FR35: Epic 4 - Builder-pattern approach
FR36: Epic 4 - All 4 approaches expose identical operations and behavior
FR37: Epic 1 - MockDurableContext with pre-loaded step results
FR38: Epic 1 - Run tests against MockDurableContext without AWS credentials
FR39: Epic 5 - Verify sequence and names of durable operations in tests
FR40: Epic 5 - Compliance suite compares Python and Rust outputs
FR41: Epic 6 - Rustdoc on every public type/method/trait/function with examples
FR42: Epic 6 - All doc examples compile and pass via cargo test --doc
FR43: Epic 6 - Standalone examples across all 4 API styles
FR44: Epic 6 - Migration guide mapping Python SDK to Rust equivalents
FR45: Epic 1 - Package durable Lambda functions as container images
FR46: Epic 1 - Dockerfile template for Lambda container builds
FR47: Epic 1 - Integrate with lambda_runtime for handler registration
FR48: Epic 1 - DurableError enum with thiserror derives
FR49: Epic 1 - Propagate AWS SDK errors through DurableError
FR50: Epic 1 - Propagate serialization errors through DurableError

## Epic List

### Epic 1: First Working Durable Lambda
A developer can create, test locally, and deploy a basic durable Lambda with checkpointed steps using the closure-native approach. Covers workspace initialization, core replay engine, step operations (with retries and typed errors), closure-native API approach, basic MockDurableContext, container deployment, and error handling foundations.
**FRs covered:** FR1-FR11, FR34, FR37-FR38, FR45-FR50

### Epic 2: Suspension & External Coordination
A developer can build durable Lambdas that wait for time delays, receive external callback signals, and invoke other Lambda functions durably.
**FRs covered:** FR12-FR18

### Epic 3: Concurrent Execution & Observability
A developer can fan out parallel branches, process collections concurrently, create isolated subflows, and debug durable functions with replay-safe logging.
**FRs covered:** FR19-FR31

### Epic 4: API Approach Variety
A developer can choose between 4 distinct API styles (proc-macro, trait-based, closure-native, builder-pattern), all exposing identical operations — enabling team evaluation and standardization.
**FRs covered:** FR32-FR33, FR35-FR36

### Epic 5: Comprehensive Testing & Behavioral Compliance
A developer can verify operation sequences in tests, and a tech lead can validate that the Rust SDK produces identical behavior to the Python SDK via the compliance suite.
**FRs covered:** FR39-FR40

### Epic 6: Documentation, Examples & Developer Onboarding
A developer can onboard within 2 days using complete rustdoc, standalone examples in all 4 API styles, and a Python-to-Rust migration guide.
**FRs covered:** FR41-FR44

## Epic 1: First Working Durable Lambda

A developer can create, test locally, and deploy a basic durable Lambda with checkpointed steps using the closure-native approach. Covers workspace initialization, core replay engine, step operations (with retries and typed errors), closure-native API approach, basic MockDurableContext, container deployment, and error handling foundations.

### Story 1.1: Project Workspace Initialization

As a developer,
I want a properly structured cargo workspace with all 6 crate skeletons and CI pipeline,
So that I have the foundation to build and test SDK components incrementally.

**Acceptance Criteria:**

**Given** a fresh clone of the durable-rust repository
**When** I run `cargo build --workspace`
**Then** the workspace compiles successfully with all 6 crates: durable-lambda-core, durable-lambda-macro, durable-lambda-trait, durable-lambda-closure, durable-lambda-builder, durable-lambda-testing
**And** the root Cargo.toml is a virtual manifest with `[workspace.dependencies]` for shared dependency versions

**Given** the workspace is initialized
**When** I examine the crate dependency graph
**Then** each approach crate (macro, trait, closure, builder) depends only on durable-lambda-core
**And** durable-lambda-testing depends only on durable-lambda-core
**And** no circular dependencies exist

**Given** the workspace is initialized
**When** I examine each crate's src/ directory
**Then** each crate has a lib.rs containing only `pub use` and `pub mod` statements (no logic)
**And** the core crate has the canonical module structure: context.rs, backend.rs, replay.rs, operations/, types.rs, error.rs

**Given** the workspace is initialized
**When** I push a commit
**Then** GitHub Actions CI runs: cargo fmt --check, cargo clippy -- -D warnings, cargo build --workspace, cargo test --workspace

### Story 1.2: Core Types & Error Foundation

As a developer,
I want typed SDK errors and core data types,
So that all SDK components share a consistent type system and error handling from the start.

**Acceptance Criteria:**

**Given** the durable-lambda-core crate
**When** I examine the types module
**Then** it exports HistoryEntry (representing a single checkpoint record), ExecutionMode (Replaying { history, cursor } | Executing), and CheckpointResult<T, E> (Ok(T) | Err(E))
**And** all types implement Serialize + DeserializeOwned via serde

**Given** the durable-lambda-core crate
**When** I examine the error module
**Then** it exports a DurableError enum with thiserror derives
**And** DurableError has variants for replay mismatch, checkpoint failure, serialization errors, and AWS SDK errors
**And** each variant is constructed via constructor methods (e.g., DurableError::replay_mismatch(...)), never raw struct construction
**And** variants wrapping underlying errors use #[from] or source() for error chain propagation

**Given** the DurableError type
**When** an AWS SDK error occurs
**Then** it is propagated through DurableError with full context preserved (FR49)

**Given** the DurableError type
**When** a serde serialization/deserialization error occurs
**Then** it is propagated through DurableError with type name and source error (FR50)

**Given** all public types and error variants
**When** I run `cargo test --doc -p durable-lambda-core`
**Then** all rustdoc examples compile and pass

### Story 1.3: DurableBackend Trait & Replay Engine

As a developer,
I want a replay engine that loads execution history and distinguishes replay from execution mode,
So that durable operations can correctly return cached results or execute new work.

**Acceptance Criteria:**

**Given** the durable-lambda-core crate
**When** I examine the backend module
**Then** it exports a DurableBackend async trait covering all 9 AWS durable execution API operations
**And** it exports a RealBackend struct that implements DurableBackend using aws-sdk-lambda

**Given** a RealBackend connected to AWS
**When** a durable function is invoked
**Then** the complete execution history is loaded in a single get_durable_execution_history call (FR1, NFR2)
**And** history is stored as a Vec<HistoryEntry> in the replay engine

**Given** a loaded execution history
**When** the replay engine initializes
**Then** it creates a usize cursor starting at position 0
**And** it sets ExecutionMode to Replaying when history entries exist, or Executing when history is empty (FR2)

**Given** the replay engine is in Replaying mode
**When** a durable operation is encountered at the current cursor position
**Then** the cached result from history is returned without re-executing the operation (FR3)
**And** the cursor advances to the next position (FR5)

**Given** the replay engine has advanced past all history entries
**When** the next durable operation is encountered
**Then** the execution mode transitions to Executing (FR2)
**And** the operation executes and checkpoints its result to AWS (FR4)

**Given** any checkpoint value
**When** it is serialized for storage or deserialized from history
**Then** serde_json is used, matching the Python SDK's JSON format exactly (FR6)

**Given** AWS API transient failures (throttling, timeouts)
**When** the RealBackend encounters them
**Then** appropriate retries are performed before surfacing errors via DurableError (NFR7)

### Story 1.4: Step Operation Implementation

As a developer,
I want to define named steps that checkpoint results and replay from cache,
So that my durable Lambda functions can safely resume after interruption.

**Acceptance Criteria:**

**Given** a DurableContext in Executing mode
**When** I call `ctx.step("validate_order", || async { Ok(validated_order) })`
**Then** the closure executes and its result is checkpointed to AWS via DurableBackend (FR8, FR4)
**And** the step name "validate_order" is used as the checkpoint key

**Given** a DurableContext in Replaying mode with a cached result for step "validate_order"
**When** I call `ctx.step("validate_order", || async { ... })`
**Then** the closure is NOT executed (FR11)
**And** the previously checkpointed result is deserialized and returned (FR3)
**And** the cursor advances past this entry (FR5)

**Given** a step closure that returns `Result<T, E>` where T and E implement Serialize + DeserializeOwned
**When** the step executes successfully
**Then** Ok(T) is serialized and checkpointed as JSON

**Given** the step operation is implemented in core
**When** I examine the file structure
**Then** step logic lives in `crates/durable-lambda-core/src/operations/step.rs`
**And** it follows the parameter ordering convention: name first, closure last

**Given** multiple sequential steps
**When** they execute in order
**Then** each step advances the cursor by one position
**And** SDK overhead per step adds < 1ms latency beyond the AWS API call (NFR1)

### Story 1.5: Step Retries & Typed Errors

As a developer,
I want to configure retry behavior and return typed errors from steps,
So that my steps handle transient failures and propagate domain-specific errors correctly.

**Acceptance Criteria:**

**Given** a step with retry configuration
**When** I call `ctx.step_with_options("charge", StepOptions::new().retries(3), || async { ... })`
**Then** the parameter ordering is name, options, closure
**And** if the closure fails, it retries up to the configured count before checkpointing the final error

**Given** a step closure that returns `Err(MyDomainError)` where MyDomainError implements Serialize + DeserializeOwned
**When** the step executes and fails with the domain error
**Then** the typed error is serialized and checkpointed as JSON (FR7, FR10)

**Given** a DurableContext in Replaying mode with a cached typed error for a step
**When** the step is replayed
**Then** the exact same typed error is deserialized and returned (FR10)
**And** the error type E requires only Serialize + DeserializeOwned bounds, not std::error::Error

**Given** a step with retries configured
**When** all retries are exhausted
**Then** the final error is checkpointed
**And** subsequent replays return that checkpointed error without re-executing

### Story 1.6: Closure-Native API Approach & Lambda Integration

As a developer,
I want to write durable Lambda handlers using a closure-native API with a single run() entry point,
So that I can build durable functions with minimal boilerplate and no direct lambda_runtime wiring.

**Acceptance Criteria:**

**Given** the durable-lambda-closure crate
**When** I write a durable Lambda handler
**Then** I can use a single import: `use durable_lambda_closure::prelude::*` (FR34)
**And** all core types (DurableError, StepOptions, etc.) are re-exported through the prelude

**Given** the closure-native API
**When** I define a handler function
**Then** I call `durable_lambda_closure::run(my_handler).await` as the entry point
**And** this internally wires up lambda_runtime handler registration and DurableContext creation (FR47)
**And** I never need to interact with lambda_runtime or DurableContext construction directly

**Given** the closure-native context wrapper
**When** I use it inside my handler
**Then** I can call `ctx.step(...)` and `ctx.step_with_options(...)` with the same signatures as core
**And** the wrapper is a thin layer over DurableContext with no additional logic

**Given** the crate structure
**When** I examine durable-lambda-closure/src/
**Then** it contains lib.rs (re-exports only), handler.rs, context.rs, and prelude.rs
**And** durable-lambda-closure depends only on durable-lambda-core (NFR8)

**Given** a complete closure-native durable Lambda handler
**When** it compiles and runs
**Then** a developer following SDK patterns encounters zero ownership/borrowing/trait-bound compiler errors related to the SDK's API surface (NFR11)

### Story 1.7: MockDurableContext & Local Testing

As a developer,
I want a MockDurableContext with pre-loaded step results for local testing,
So that I can write and run tests without AWS credentials or deployment.

**Acceptance Criteria:**

**Given** the durable-lambda-testing crate
**When** I create a MockDurableContext
**Then** I can pre-load step results as a sequence of JSON values (FR37)
**And** the mock uses a MockBackend that implements DurableBackend without any AWS dependency at runtime

**Given** a MockDurableContext with pre-loaded results for steps "validate" and "charge"
**When** my handler calls `ctx.step("validate", ...)` and `ctx.step("charge", ...)`
**Then** the pre-loaded results are returned in order without executing the closures (FR37)
**And** no AWS credentials or network access are required (FR38)

**Given** a test using MockDurableContext
**When** I run `cargo test`
**Then** the test executes locally in milliseconds
**And** it verifies the handler's logic with deterministic, pre-loaded data

**Given** the durable-lambda-testing crate
**When** I examine its structure
**Then** it contains mock_backend.rs (MockBackend implementing DurableBackend), mock_context.rs (MockDurableContext), assertions.rs (test helpers), and prelude.rs
**And** it depends only on durable-lambda-core, not on aws-sdk-lambda at runtime

**Given** the testing prelude
**When** I import `use durable_lambda_testing::prelude::*`
**Then** I have access to MockDurableContext and all test assertion helpers

### Story 1.8: Container Deployment Template

As a developer,
I want a Dockerfile template and container build example for Lambda deployment,
So that I can package and deploy my durable Lambda as a container image.

**Acceptance Criteria:**

**Given** the examples/ directory
**When** I examine the Dockerfile
**Then** it uses `public.ecr.aws/lambda/provided:al2023` as the base image (FR46, NFR18)
**And** it builds a Rust binary with the Lambda runtime
**And** it produces a container image deployable to AWS Lambda

**Given** a durable Lambda handler written with the closure-native approach
**When** I run `docker build -f examples/Dockerfile -t my-durable-lambda .`
**Then** the container image builds successfully (FR45)
**And** the image contains the compiled Lambda handler binary

**Given** a built container image
**When** deployed to AWS Lambda with the provided.al2023 runtime
**Then** the durable Lambda function executes correctly
**And** the cold start time is under 100ms excluding AWS container initialization (NFR4)
**And** the memory baseline remains under 32MB for a minimal handler (NFR3)

## Epic 2: Suspension & External Coordination

A developer can build durable Lambdas that wait for time delays, receive external callback signals, and invoke other Lambda functions durably.

### Story 2.1: Wait Operation

As a developer,
I want to suspend a durable function for a specified duration without consuming compute,
So that I can implement time-based delays in my workflows (e.g., retry cooldowns, scheduled follow-ups).

**Acceptance Criteria:**

**Given** a DurableContext in Executing mode
**When** I call `ctx.wait(Duration::from_secs(30))`
**Then** the function suspends execution without consuming compute (FR12)
**And** the wait is checkpointed to AWS via DurableBackend

**Given** a suspended durable function with an active wait
**When** the wait duration has elapsed
**Then** the function resumes execution from the point after the wait call (FR13)
**And** the cursor advances past the wait entry

**Given** a DurableContext in Replaying mode with a completed wait in history
**When** the replay engine encounters the wait entry
**Then** the wait is skipped without re-suspending
**And** execution continues immediately to the next operation

**Given** the wait operation
**When** I examine the implementation
**Then** the logic lives in `crates/durable-lambda-core/src/operations/wait.rs`
**And** the closure-native approach crate exposes `ctx.wait(duration)` through its context wrapper

### Story 2.2: Callback Operation

As a developer,
I want to register a callback and suspend execution until an external system signals completion,
So that I can coordinate with external workflows (e.g., human approvals, third-party webhooks).

**Acceptance Criteria:**

**Given** a DurableContext in Executing mode
**When** I call `ctx.callback()`
**Then** a unique callback ID is generated and returned to the caller (FR14)
**And** the function suspends execution until a signal is received (FR15)

**Given** an active callback with a known callback ID
**When** an external system sends a success signal with payload using the callback ID
**Then** the function resumes with the success payload available to subsequent operations (FR16)

**Given** an active callback with a known callback ID
**When** an external system sends a failure signal using the callback ID
**Then** the function resumes with the failure information available (FR16)

**Given** an active callback with a known callback ID
**When** an external system sends a heartbeat signal using the callback ID
**Then** the callback timeout is extended without resuming execution (FR16)

**Given** a DurableContext in Replaying mode with a completed callback in history
**When** the replay engine encounters the callback entry
**Then** the cached callback result is returned without re-suspending
**And** the cursor advances past the callback entry

**Given** the callback operation
**When** I examine the implementation
**Then** the logic lives in `crates/durable-lambda-core/src/operations/callback.rs`
**And** the closure-native approach crate exposes `ctx.callback()` through its context wrapper

### Story 2.3: Invoke Operation

As a developer,
I want to durably invoke another Lambda function and receive its result,
So that I can compose durable workflows across multiple Lambda functions with guaranteed exactly-once invocation semantics.

**Acceptance Criteria:**

**Given** a DurableContext in Executing mode
**When** I call `ctx.invoke("target-function-name", payload)`
**Then** the target Lambda function is invoked via DurableBackend (FR17)
**And** the invocation result is checkpointed to AWS

**Given** a DurableContext in Replaying mode with a cached invoke result
**When** the replay engine encounters the invoke entry
**Then** the cached result is returned without re-invoking the target function (FR18)
**And** the target Lambda is NOT called again

**Given** the invoke operation follows parameter ordering convention
**When** I call `ctx.invoke("function_name", payload)`
**Then** function name is the first parameter and payload is the last

**Given** the invoke operation
**When** the target Lambda function returns an error
**Then** the error is checkpointed and replayed as-is on subsequent invocations

**Given** the invoke operation
**When** I examine the implementation
**Then** the logic lives in `crates/durable-lambda-core/src/operations/invoke.rs`
**And** the closure-native approach crate exposes `ctx.invoke(name, payload)` through its context wrapper

## Epic 3: Concurrent Execution & Observability

A developer can fan out parallel branches, process collections concurrently, create isolated subflows, and debug durable functions with replay-safe logging.

### Story 3.1: Parallel Operation

As a developer,
I want to execute multiple branches concurrently within a single Lambda invocation with configurable completion criteria,
So that I can fan out work (e.g., notify multiple reviewers, check multiple services) and control when the parallel block completes.

**Acceptance Criteria:**

**Given** a DurableContext in Executing mode
**When** I call `ctx.parallel("fan_out", branches)` with a collection of branch closures
**Then** all branches execute concurrently as tokio::spawn tasks (FR19, FR22)
**And** each branch result is checkpointed independently

**Given** a parallel operation
**When** I configure completion criteria (all, any, N-of-M)
**Then** the parallel block completes according to the specified criteria (FR20)
**And** remaining branches are handled appropriately based on the criteria

**Given** parallel branches executing concurrently
**When** each branch executes durable operations (steps, waits, etc.)
**Then** each branch operates in its own checkpoint namespace (FR21)
**And** branch checkpoint keys do not collide with each other or the parent context

**Given** branch closures passed to parallel
**When** they are compiled
**Then** they satisfy `Send + 'static` bounds required by tokio::spawn (FR22)
**And** the "pit of success" API patterns make satisfying these bounds natural (owned data, no shared mutable state)

**Given** a DurableContext in Replaying mode with cached parallel results
**When** the replay engine encounters the parallel entry
**Then** cached branch results are returned without re-executing any branches
**And** the cursor advances past all parallel-related entries

**Given** the parallel operation follows parameter ordering
**When** I call `ctx.parallel("fan_out", branches)`
**Then** name is first and branches collection is last

**Given** the parallel operation
**When** I examine the implementation
**Then** the logic lives in `crates/durable-lambda-core/src/operations/parallel.rs`

### Story 3.2: Map Operation

As a developer,
I want to process a collection of items in parallel using a closure applied to each item,
So that I can efficiently handle batch workloads (e.g., processing a list of orders, transforming a dataset).

**Acceptance Criteria:**

**Given** a DurableContext in Executing mode
**When** I call `ctx.map("process_items", items, || async { ... })`
**Then** the closure is applied to each item in the collection in parallel (FR23)
**And** each item's result is checkpointed independently

**Given** a map operation with batching configured
**When** I configure batch size
**Then** items are processed in batches of the specified size (FR24)
**And** each batch completes before the next batch begins

**Given** a map operation that completes
**When** all items have been processed
**Then** a BatchResult<T> is returned containing the results of all map operations (FR25)
**And** results maintain their correspondence to input items

**Given** a DurableContext in Replaying mode with cached map results
**When** the replay engine encounters the map entry
**Then** the cached BatchResult is returned without re-executing any item closures

**Given** map closures
**When** they are compiled
**Then** they satisfy `Send + 'static` bounds required for concurrent execution
**And** the API patterns make this natural with owned data

**Given** the map operation follows parameter ordering
**When** I call `ctx.map("process_items", items, closure)`
**Then** name is first, items second, closure last

**Given** the map operation
**When** I examine the implementation
**Then** the logic lives in `crates/durable-lambda-core/src/operations/map.rs`

### Story 3.3: Child Context Operation

As a developer,
I want to create isolated subflows with their own checkpoint namespace,
So that I can compose complex workflows from independent sub-workflows without checkpoint key collisions.

**Acceptance Criteria:**

**Given** a DurableContext in Executing mode
**When** I call `ctx.child_context("sub_workflow", || async { ... })`
**Then** an isolated child DurableContext is created with its own checkpoint namespace (FR26)
**And** the child context can execute any durable operation independently from the parent (FR27)

**Given** a child context
**When** it executes durable operations (steps, waits, parallel, etc.)
**Then** its checkpoint keys are namespaced and do not collide with the parent or sibling child contexts

**Given** a child context
**When** it is created
**Then** it is a fully owned context that shares only the AWS client (Arc<dyn DurableBackend>) with the parent (FR28)
**And** no shared mutable state exists between parent and child

**Given** a child context namespacing strategy
**When** implemented
**Then** it matches the Python SDK's namespacing approach for behavioral compliance (NFR5)

**Given** a DurableContext in Replaying mode with cached child context results
**When** the replay engine encounters the child context entry
**Then** the child's operations are replayed from its namespaced history entries

**Given** the child context operation
**When** I examine the implementation
**Then** the logic lives in `crates/durable-lambda-core/src/operations/child_context.rs`

### Story 3.4: Replay-Safe Logging

As a developer,
I want to emit structured log messages that are automatically deduplicated across replays,
So that I can debug durable functions without log noise from replayed operations.

**Acceptance Criteria:**

**Given** a DurableContext in Executing mode
**When** I call `ctx.log("order processed", structured_data)`
**Then** the structured log message is emitted via the tracing crate (FR29, FR30)
**And** the log entry is recorded so it can be identified during replay

**Given** a DurableContext in Replaying mode
**When** the replay engine encounters a log operation that was already emitted
**Then** the duplicate log output is suppressed (FR31)
**And** no repeated log entries appear in CloudWatch or local output

**Given** the logging integration
**When** I examine the implementation
**Then** it uses the tracing crate ecosystem — no custom logging framework (NFR20)
**And** the log operation integrates with the user's existing tracing subscriber configuration

**Given** a durable function with multiple steps and log calls
**When** it replays after a suspension
**Then** only log messages from newly executing operations appear
**And** previously emitted log messages are not duplicated

**Given** the log operation
**When** I examine the implementation
**Then** the logic lives in `crates/durable-lambda-core/src/operations/log.rs`

## Epic 4: API Approach Variety

A developer can choose between 4 distinct API styles (proc-macro, trait-based, closure-native, builder-pattern), all exposing identical operations — enabling team evaluation and standardization.

### Story 4.1: Proc-Macro API Approach

As a developer,
I want to define durable Lambda handlers using a `#[durable_execution]` attribute macro,
So that I can write handlers with minimal boilerplate in the most idiomatic Rust style (mirroring #[tokio::main]).

**Acceptance Criteria:**

**Given** the durable-lambda-macro crate
**When** I annotate an async function with `#[durable_execution]`
**Then** the macro generates lambda_runtime handler registration and DurableContext setup (FR32)
**And** the function receives a context with access to all 8 core operations

**Given** the proc-macro crate
**When** I examine the structure
**Then** it contains lib.rs (attribute macro entry point with `proc-macro = true`) and expand.rs (code generation logic)
**And** it depends on syn, quote, and proc-macro2 for macro expansion

**Given** the generated code from the macro
**When** I run `cargo expand` on a handler using `#[durable_execution]`
**Then** the expanded code is readable and debuggable
**And** it produces the same runtime behavior as the closure-native approach

**Given** the macro-generated handler
**When** a developer uses it with AI coding tools
**Then** the attribute macro pattern is reliably generated by Claude Code and Copilot (NFR13)

**Given** the durable-lambda-macro crate
**When** I examine its dependencies
**Then** it depends only on durable-lambda-core (plus proc-macro tooling: syn, quote, proc-macro2) (NFR8)

### Story 4.2: Trait-Based API Approach

As a developer,
I want to define durable Lambda handlers by implementing a DurableHandler trait,
So that I can use a structured, object-oriented approach with explicit method signatures for my handlers.

**Acceptance Criteria:**

**Given** the durable-lambda-trait crate
**When** I implement the DurableHandler trait on my struct
**Then** I can define my handler logic in the trait's method (FR33)
**And** I receive a context with access to all 8 core operations

**Given** the trait-based approach
**When** I call `durable_lambda_trait::run(MyHandler).await`
**Then** lambda_runtime is wired up internally with DurableContext creation
**And** I never need to interact with lambda_runtime directly

**Given** the durable-lambda-trait crate
**When** I import `use durable_lambda_trait::prelude::*`
**Then** I have access to the DurableHandler trait, context wrapper, and all core types

**Given** the crate structure
**When** I examine durable-lambda-trait/src/
**Then** it contains lib.rs (re-exports only), handler.rs (DurableHandler trait definition), context.rs (trait-specific context wrapper), and prelude.rs
**And** it depends only on durable-lambda-core (NFR8)

### Story 4.3: Builder-Pattern API Approach

As a developer,
I want to construct durable Lambda handlers using a builder pattern,
So that I can configure complex handlers with explicit, step-by-step construction and rich configuration options.

**Acceptance Criteria:**

**Given** the durable-lambda-builder crate
**When** I construct a handler using the builder pattern
**Then** I can configure the handler step-by-step before building and running it (FR35)
**And** the built handler has access to all 8 core operations

**Given** the builder-pattern approach
**When** I call the builder's run method
**Then** lambda_runtime is wired up internally with DurableContext creation
**And** I never need to interact with lambda_runtime directly

**Given** the durable-lambda-builder crate
**When** I import `use durable_lambda_builder::prelude::*`
**Then** I have access to the builder types, context wrapper, and all core types

**Given** the crate structure
**When** I examine durable-lambda-builder/src/
**Then** it contains lib.rs (re-exports only), handler.rs (builder-pattern handler construction), context.rs (builder-specific context wrapper), and prelude.rs
**And** it depends only on durable-lambda-core (NFR8)

### Story 4.4: Cross-Approach Behavioral Parity Verification

As a tech lead,
I want to verify that all 4 API approaches expose the same 8 core operations with identical behavior,
So that I can confidently select any approach knowing the team gets the same guarantees regardless of style.

**Acceptance Criteria:**

**Given** all 4 approach crates (macro, trait, closure, builder)
**When** the same durable workflow is implemented in each approach
**Then** all 4 produce identical checkpoint sequences for identical inputs (FR36)
**And** all 4 return identical results for identical execution histories

**Given** a shared behavioral test suite
**When** it runs against each approach crate
**Then** all 4 approaches pass the same test cases
**And** the tests cover all 8 core operations: step, wait, callback, invoke, parallel, map, child_context, log

**Given** each approach crate's context wrapper
**When** I compare the operation signatures
**Then** all 4 follow the same parameter ordering convention (name, options, closure)
**And** all 4 expose identical operation method names

**Given** the behavioral parity verification
**When** a new core operation is added to durable-lambda-core
**Then** it must be exposed by all 4 approach crates to maintain parity (NFR9)

## Epic 5: Comprehensive Testing & Behavioral Compliance

A developer can verify operation sequences in tests, and a tech lead can validate that the Rust SDK produces identical behavior to the Python SDK via the compliance suite.

### Story 5.1: Operation Sequence Verification

As a developer,
I want to verify the exact sequence and names of durable operations executed during a test,
So that I can assert my handler calls operations in the correct order and catch regressions in workflow logic.

**Acceptance Criteria:**

**Given** a MockDurableContext used in a test
**When** my handler executes a series of durable operations (steps, waits, callbacks, etc.)
**Then** I can retrieve the recorded sequence of operation names and types (FR39)

**Given** the assertion helpers in durable-lambda-testing
**When** I call an assertion like `assert_operations(mock, &["step:validate", "step:charge", "wait", "step:confirm"])`
**Then** the test passes if the handler executed exactly those operations in that order
**And** the test fails with a clear diff if the sequence doesn't match

**Given** the operation sequence recorder
**When** nested operations occur (e.g., steps inside a child context or parallel branches)
**Then** the recorded sequence captures the nesting structure accurately

**Given** the assertions module
**When** I examine the implementation
**Then** the logic lives in `crates/durable-lambda-testing/src/assertions.rs`
**And** assertion helpers are available via `use durable_lambda_testing::prelude::*`

### Story 5.2: Python/Rust Compliance Suite

As a tech lead,
I want a compliance suite that executes identical workflows in both Python and Rust and compares outputs,
So that I can verify zero behavioral divergence between the Rust SDK and the Python reference implementation.

**Acceptance Criteria:**

**Given** the compliance/ directory
**When** I examine its structure
**Then** it contains python/workflows/ with 3-5 Python reference implementations, rust/src/ with matching Rust implementations, and tests/ with comparison logic (FR40)

**Given** the compliance workflows
**When** I examine the Python and Rust implementations
**Then** they implement the same business logic: at minimum order processing (multi-step), parallel fanout, and callback-based approval workflows
**And** each workflow exercises multiple core operations

**Given** identical inputs and execution sequences
**When** the compliance test runs both Python and Rust implementations
**Then** both produce identical checkpoint sequences and final outputs (NFR5)
**And** any divergence is reported as a test failure with a clear diff showing expected vs actual

**Given** the compliance suite
**When** I run the comparison tests
**Then** the test harness executes both Python and Rust workflows against the same mock/recorded history
**And** serialized checkpoint formats (JSON) match exactly between Python and Rust

**Given** the compliance suite documentation
**When** I examine compliance/README.md
**Then** it explains how to add new compliance workflows, how to run the suite, and how results are compared

## Epic 6: Documentation, Examples & Developer Onboarding

A developer can onboard within 2 days using complete rustdoc, standalone examples in all 4 API styles, and a Python-to-Rust migration guide.

### Story 6.1: Rustdoc Coverage

As a developer,
I want every public type, method, trait, and function to have rustdoc with inline examples,
So that I can learn the SDK directly from the API documentation without needing external resources.

**Acceptance Criteria:**

**Given** all public items across all 6 SDK crates
**When** I examine their rustdoc comments
**Then** every public type, method, trait, and function has rustdoc with at least one inline example (FR41)
**And** summary lines use imperative mood ("Execute a named step", not "Executes")

**Given** all durable operation methods
**When** I read their rustdoc
**Then** each documents replay vs execution behavior explicitly
**And** each includes an `# Examples` section and an `# Errors` section (for methods returning Result)
**And** no `# Panics` section exists — the SDK should never panic

**Given** all doc examples across the workspace
**When** I run `cargo test --doc`
**Then** all doc examples compile and pass (FR42)
**And** zero doc test failures

**Given** the rustdoc for approach crate preludes
**When** I read the prelude module documentation
**Then** it shows the single-import pattern (`use durable_lambda_closure::prelude::*`) with a complete minimal handler example

### Story 6.2: Standalone Examples Across All API Styles

As a developer,
I want standalone examples demonstrating every core feature in all 4 API styles,
So that I can copy-paste a working pattern for any operation in my preferred API approach.

**Acceptance Criteria:**

**Given** the examples/ directory
**When** I examine its structure
**Then** it contains subdirectories for each API style: closure-style/, macro-style/, trait-style/, builder-style/ (FR43)
**And** each subdirectory contains the same set of example files

**Given** each API style's example directory
**When** I examine the example files
**Then** they cover: basic steps, step retries, typed errors, waits, callbacks, invoke, parallel, map, child contexts, replay-safe logging, and a combined end-to-end workflow
**And** each example is a complete, compilable, standalone file

**Given** all standalone examples
**When** I run `cargo build --examples` (or equivalent)
**Then** all examples compile successfully

**Given** a developer new to the SDK
**When** they open any example file
**Then** the example is self-contained with clear comments explaining what each section does
**And** the example can be used as a starting point for a new durable Lambda handler

### Story 6.3: Python-to-Rust Migration Guide

As a senior developer migrating Python durable Lambdas to Rust,
I want a migration guide with conceptual mappings and side-by-side code,
So that I can translate my existing Python patterns to Rust equivalents within 1 week.

**Acceptance Criteria:**

**Given** the docs/migration-guide.md file
**When** I read the conceptual mapping section
**Then** it provides a table mapping every Python SDK concept to its Rust equivalent (FR44)
**And** it covers all 8 core operations plus handler registration, testing, and deployment

**Given** each core operation in the migration guide
**When** I read its section
**Then** it shows side-by-side Python and Rust code for the same operation
**And** the Rust code uses the closure-native approach (recommended default)

**Given** the migration guide
**When** I read the gotchas section
**Then** it documents: determinism requirements (non-durable code re-executes), Send + 'static bounds for parallel/map closures, owned data in closures, and serde bounds on checkpoint types
**And** each gotcha includes a concrete "wrong" and "right" code example

**Given** a senior Python developer with some Rust exposure
**When** they follow the migration guide end-to-end
**Then** they have enough information to migrate an existing Python durable Lambda to Rust without reading SDK internals
