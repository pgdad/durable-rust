---
stepsCompleted: [step-01-init, step-02-discovery, step-02b-vision, step-02c-executive-summary, step-03-success, step-04-journeys, step-05-domain, step-06-innovation, step-07-project-type, step-08-scoping, step-09-functional, step-10-nonfunctional, step-11-polish]
inputDocuments:
  - '_bmad-output/planning-artifacts/product-brief-durable-rust-2026-03-13.md'
  - '_bmad-output/brainstorming/brainstorming-session-2026-03-13-1257.md'
workflowType: 'prd'
briefCount: 1
researchCount: 0
brainstormingCount: 1
projectDocsCount: 0
classification:
  projectType: developer_tool
  domain: general
  complexity: medium
  projectContext: greenfield
---

# Product Requirements Document - durable-rust

**Author:** Esa
**Date:** 2026-03-13

## Executive Summary

durable-rust is an idiomatic Rust SDK for AWS Lambda Durable Functions, providing full feature parity with the official AWS Python Durable Lambda SDK. It targets organizations running durable Lambda workloads at massive scale — billions of daily invocations across multiple departments — where Rust's 4-8x lower memory footprint and order-of-magnitude faster cold starts translate to millions in annual compute savings.

The SDK implements all 8 core operations (steps, waits, callbacks, invoke, parallel, map, child contexts, replay-safe logging) as a cargo workspace with a shared replay engine and 4 distinct API styles (proc-macro, trait-based, closure-native, builder-pattern) for team evaluation. A dedicated testing crate provides local development without AWS credentials. Container-based Lambda deployment is the target packaging model.

Primary users are developers with 2-3 years of experience and minimal Rust knowledge, supported by AI coding assistants (Claude Code, Copilot). The SDK must enable a working durable Lambda within 2 days of onboarding. Secondary users include senior developers establishing team patterns, tech leads selecting the API approach, and engineering leadership tracking cost reduction.

The project is initially internal with documentation designed for future open-sourcing, positioning to influence or contribute to a potential official AWS Rust Durable Lambda SDK.

### What Makes This Special

The "pit of success" design philosophy inverts Rust's typical learning curve. Where most Rust libraries expose ownership, borrowing, and trait bound complexity to users, durable-rust abstracts these behind pattern-based interfaces where the simplest code to write is also correct. This is explicitly optimized for AI coding tools — enabling reliable code generation on first or second attempt — which is critical for a junior-heavy development team.

The economic case compounds at scale: Rust's ~16-32MB memory baseline vs Python's ~128MB, combined with dramatically faster cold starts, directly reduces per-invocation costs across billions of daily executions. No other path to these savings exists — AWS provides no Rust durable SDK, Step Functions are prohibited by corporate standards, and external frameworks (Temporal, Restate) require leaving the Lambda ecosystem.

The 12-18 month window before AWS potentially releases an official Rust SDK creates a strategic opportunity: production-validated API design and patterns from durable-rust could inform or become the official implementation.

## Project Classification

- **Project Type:** Developer Tool (SDK/library — Rust crate workspace)
- **Domain:** Cloud Infrastructure / Serverless Computing
- **Complexity:** Medium — technically nuanced replay-with-memoization execution model with behavioral compliance requirements against the Python SDK
- **Project Context:** Greenfield — no existing Rust durable Lambda SDK exists

## Success Criteria

### User Success

- **2-day onboarding:** A developer with 2-3 years of experience and minimal Rust knowledge builds a working durable Lambda function (steps, waits, parallel) within 2 days using AI coding assistants
- **1-week migration:** A senior developer migrates an existing Python durable Lambda to Rust within 1 week
- **Pit of success:** Developers following SDK patterns rarely encounter compiler errors related to ownership, borrowing, or trait bounds
- **AI-generated correctness:** Claude Code and Copilot consistently generate correct, compilable durable Lambda code on first or second attempt when following SDK patterns
- **Local testability:** Developers can write and run tests locally using `MockDurableContext` without AWS credentials or deployment

### Business Success

- **Cost reduction:** Minimum 10% aggregate reduction in Lambda compute costs (purely compute, excluding other AWS charges) when durable workloads run in Rust
- **3-month adoption:** At least 1 production durable Lambda function running in Rust
- **12-month adoption:** At least 10 production durable Lambda functions running in Rust across departments
- **Zero incidents:** No increase in incident rate for Rust durable Lambdas compared to equivalent Python implementations

### Technical Success

- **Full feature parity:** All 8 core operations functional in all 4 API approaches
- **100% test coverage:** Unit + integration tests across all SDK crates, measured via cargo tarpaulin / llvm-cov
- **100% live coverage:** All core operations verified via integration test suite against live AWS Lambda
- **Behavioral compliance:** Zero regressions — compliance Lambda suite (3-5 workflows in both Python and Rust) produces identical results for identical inputs and execution sequences
- **Container deployment:** All Lambda functions package and deploy as container images

### Measurable Outcomes

| Outcome | Target | Measurement |
|---|---|---|
| Aggregate compute cost reduction | >= 10% | AWS Cost Explorer before/after |
| Production Rust durable Lambdas (3mo) | >= 1 | Deployment inventory |
| Production Rust durable Lambdas (12mo) | >= 10 | Deployment inventory |
| SDK test coverage | 100% | cargo tarpaulin / llvm-cov |
| Live AWS test coverage | 100% of core ops | Integration test suite |
| Behavioral compliance | 0 regressions | Python/Rust compliance suite |
| Developer onboarding | <= 2 days | Pilot team measurement |
| Migration time per function | <= 1 week | First migration measurement |
| Incident rate | No increase | Rust vs Python comparison |

## Product Scope & Phased Development

### MVP Strategy

**Approach:** Full-capability SDK — all 8 core operations, all 4 API approaches, testing crate, compliance suite, and examples ship together as a single release. A partial SDK would force developers back to Python for unsupported operations, undermining the cost savings case.

**Resource Requirements:** Development team with Rust expertise, access to AWS Lambda for integration testing, CI/CD pipeline with container image builds.

### MVP Feature Set (Phase 1)

**Core User Journeys Supported:**
- Alex (junior dev) — builds first durable Lambda in 2 days
- Jordan (senior dev) — evaluates 4 API approaches, establishes team patterns
- Morgan (tech lead) — validates compliance and cost savings
- CI/CD pipeline — builds, tests, deploys container images

**Must-Have Capabilities:**
- All 8 core operations in all 4 API approach crates
- Shared replay engine (`durable-lambda-core`)
- `MockDurableContext` for local testing (`durable-lambda-testing`)
- Compliance Lambda suite (3-5 workflows in Python + Rust)
- Rustdoc with examples on every public API
- Standalone examples demonstrating every core feature in all 4 styles
- Container-based Lambda deployment with Dockerfile
- Migration guide (Python-to-Rust conceptual mapping)

### Phase 2 — Growth

- API approach consolidation — converge on recommended approach based on team evaluation
- Open-source release to crates.io
- Additional example workflows (AI workflows, data pipelines, multi-step approvals)
- Detailed migration guides with pattern-by-pattern recipes
- Community documentation and contribution guidelines

### Phase 3 — Expansion

- Position for AWS official Rust SDK adoption
- Community contributions and ecosystem tooling
- Cross-department scaling as organizational standard
- Advanced patterns library based on production experience

### Risk Mitigation Strategy

**Technical Risks:**

| Risk | Impact | Mitigation |
|---|---|---|
| `aws-sdk-lambda` durable execution API changes | Breaking changes to core operations | Pin SDK version, test against multiple versions, maintain compatibility layer in core |
| Replay engine correctness | Silent data corruption if replay diverges from Python SDK behavior | Compliance suite with identical-output verification; extensive property-based testing |
| `Send + 'static` bounds surprise junior devs | Compiler errors in parallel/map closures | Clear error messages in docs, lint-like guidance in examples, "pit of success" patterns that naturally satisfy bounds |
| Proc-macro debugging difficulty | Macro approach harder to troubleshoot | Macro crate generates readable code; `cargo expand` documented; other approaches available as alternatives |

**Market Risks:**

| Risk | Impact | Mitigation |
|---|---|---|
| AWS releases official Rust durable SDK | Reduces value of internal SDK | Open-source strategy — contribute patterns and API design to official SDK; production-validated codebase becomes community asset |
| Team Rust adoption resistance | Developers prefer staying with Python | 2-day onboarding target with AI assistance; pit of success API; no forced migration — new projects first |

**Resource Risks:**

| Risk | Impact | Mitigation |
|---|---|---|
| Fewer Rust-experienced developers than planned | Slower development velocity | SDK designed for AI-assisted development — same principle applies to SDK development itself |
| AWS durable functions API instability | Rework needed if API changes | Core crate isolates AWS API surface; approach crates depend only on core abstractions |

## User Journeys

### Journey 1: Alex Builds a First Durable Lambda (Junior Developer — Happy Path)

**Alex**, 2 years into a Python backend role, has been assigned to build a new order-processing durable Lambda in Rust. Alex has never written Rust beyond a tutorial.

**Opening Scene:** Alex opens the durable-rust repo README. The examples folder shows the same order-processing workflow in 4 styles. Alex's tech lead has already selected the closure-native approach. Alex opens the closure-style example, reads the doc comments, and asks Claude Code: "Help me build a durable Lambda that validates an order, charges payment, and sends confirmation."

**Rising Action:** Claude Code generates a complete handler using `ctx.step("validate", || ...)`, `ctx.step("charge", || ...)`, and `ctx.wait(Duration::from_secs(5))`. It compiles on the first try. Alex writes a test using `MockDurableContext`, pre-loading step results — the test runs locally in milliseconds without AWS credentials. Alex modifies the workflow to add parallel inventory checks using `ctx.parallel(...)`, and the pattern is the same — closures with owned data, no borrow checker fights.

**Climax:** Alex deploys the container image to AWS Lambda. The durable function executes, suspends during a wait, and resumes correctly — replaying previous steps from history and continuing from the new checkpoint. The order completes end-to-end.

**Resolution:** Alex has a production-ready durable Lambda in under 2 days. The patterns are copy-paste-able for the next function. Alex's confidence with Rust has grown from "intimidating" to "I can do this with AI help."

### Journey 2: Alex Hits a Replay Bug (Junior Developer — Edge Case)

**Opening Scene:** Alex's durable Lambda works in tests but behaves unexpectedly in production. A step that calls an external API returns different data on replay than it did on the original execution — the API response changed between invocations.

**Rising Action:** Alex doesn't understand why the function produces wrong results. The replay-safe logging shows the step name and sequence position. Alex reviews the SDK documentation on replay semantics: "The function replays from the top. Durable operations return cached results. Non-durable code re-executes and must be deterministic or wrapped in a step."

**Climax:** Alex realizes the external API call was outside a `ctx.step()` — it wasn't checkpointed. Moving it inside a step fixes the issue: the result is cached on first execution and replayed identically on subsequent invocations.

**Resolution:** Alex now understands the fundamental rule: anything that must return the same value across replays must be inside a durable operation. Alex documents the gotcha for the team and adds a test case using `MockDurableContext` to verify the fix.

### Journey 3: Jordan Evaluates API Approaches and Establishes Team Patterns (Senior Developer)

**Opening Scene:** Jordan, a senior Python developer with some Rust exposure, is tasked with evaluating which of the 4 API approaches the team should standardize on. Jordan clones the repo and examines the same order-processing workflow implemented in all 4 styles.

**Rising Action:** Jordan implements a more complex workflow — a multi-step approval process with parallel reviewer notifications, callback waits for approvals, and child contexts for isolated sub-workflows — in all 4 approaches. Jordan evaluates each on: how naturally it reads, how well Claude Code generates it, how easy it is for Alex-level developers to modify, and how error handling feels. Jordan runs the compliance suite against the Python implementation to verify identical behavior.

**Climax:** Jordan writes a comparison document with code samples and recommendations. The closure-native approach wins for simplicity; the builder approach wins for complex configurations. Jordan recommends closure-native as the default with builder for advanced use cases.

**Resolution:** Jordan publishes internal team guidelines with approved patterns, common gotchas (non-deterministic code outside steps, Send + 'static requirements for parallel closures), and a template project. The team has a clear, opinionated path forward.

### Journey 4: Morgan Approves SDK Adoption (Tech Lead / Architect)

**Opening Scene:** Morgan needs to present the Rust durable Lambda business case to Sam (leadership). Morgan reviews Jordan's evaluation, the compliance suite results, and the cost projections.

**Rising Action:** Morgan deploys one of the compliance Lambdas in both Python and Rust to a staging environment with production-like load. Morgan compares memory usage (Python ~128MB vs Rust ~24MB), cold start times (Python ~800ms vs Rust ~50ms), and per-invocation cost via AWS Cost Explorer. Morgan validates that both implementations produce identical outputs for identical inputs.

**Climax:** Morgan presents to Sam: "10%+ aggregate compute cost reduction is achievable. Zero behavioral regressions. Junior developers are productive in 2 days. We recommend starting all new durable Lambdas in Rust."

**Resolution:** Sam approves. Morgan publishes the selected API approach, the internal SDK documentation, and the team onboarding guide. The first production Rust durable Lambda ships within the month.

### Journey 5: CI/CD Pipeline Builds, Tests, and Deploys (Automated Consumer)

**Opening Scene:** A developer pushes a commit to the durable Lambda repository. The CI pipeline triggers.

**Rising Action:** The pipeline runs `cargo build` across the workspace — all 6 crates compile. `cargo test` executes unit and integration tests using `MockDurableContext` — no AWS credentials needed in CI. `cargo tarpaulin` reports 100% coverage. The pipeline builds a container image using the project's Dockerfile with the Lambda runtime.

**Climax:** On merge to main, the CD pipeline pushes the container image to ECR and updates the Lambda function configuration. A post-deployment smoke test invokes the durable Lambda with a test payload, verifying end-to-end execution including checkpoint, suspend, and resume.

**Resolution:** The deployment is complete. CloudWatch metrics confirm the function is operating within expected memory and latency bounds. The pipeline posts a summary to the team channel.

### Journey Requirements Summary

| Journey | Key Capabilities Revealed |
|---|---|
| Alex — Happy Path | SDK ergonomics, AI code generation, MockDurableContext, doc comments, examples, container deployment |
| Alex — Edge Case | Replay-safe logging, clear error messages, documentation on replay semantics, determinism guidance |
| Jordan — Evaluation | 4 API approaches with identical examples, compliance suite, code readability, AI tool compatibility |
| Morgan — Adoption | Cost comparison (via AWS Cost Explorer), compliance verification, performance benchmarks (staging), onboarding guides |
| CI/CD Pipeline | Cargo workspace builds, MockDurableContext for CI testing, container image builds, tarpaulin coverage, deployment automation |

## Developer Tool Specific Requirements

### Technical Architecture

**Language & Runtime:**
- Rust only, latest stable toolchain
- Async runtime: tokio (required by `aws-sdk-lambda` and `lambda_runtime`)
- No MSRV policy — developers use latest stable

**Package Distribution:**
- Phase 1 (internal): Git dependency in `Cargo.toml`
- Phase 2 (open-source): Publish to crates.io as 6 crates with matching version numbers

**Cargo Workspace Structure:**
- `durable-lambda-core` — shared replay engine, AWS API wrapper, types, errors
- `durable-lambda-macro` — proc-macro API approach (depends on core)
- `durable-lambda-trait` — trait-based API approach (depends on core)
- `durable-lambda-closure` — closure-native API approach (depends on core)
- `durable-lambda-builder` — builder-pattern API approach (depends on core)
- `durable-lambda-testing` — `MockDurableContext`, test utilities (depends on core)

### API Surface

**Core Types (from `durable-lambda-core`):**
- `DurableContext` — main context passed to handler functions
- `ExecutionMode` — `Replaying { history, cursor }` | `Executing`
- `HistoryEntry` — deserialized checkpoint record
- `CheckpointResult<T, E>` — `Ok(T)` | `Err(E)` for serialized step results
- `DurableError` — SDK error enum with `thiserror` derives
- `BatchResult<T>` — results from parallel/map operations

**8 Core Operations (exposed by each API approach crate):**
1. `step(name, closure)` — checkpointed work with retries
2. `wait(duration)` — time-based suspension
3. `callback()` — wait for external signal via callback ID
4. `invoke(function, payload)` — durable Lambda-to-Lambda invocation
5. `parallel(branches)` — fan-out with completion criteria
6. `map(items, closure)` — parallel collection processing
7. `child_context(name, closure)` — isolated subflow with own checkpoint namespace
8. `log()` — replay-safe, deduplicated structured logging via `tracing`

### Documentation & Examples

**Documentation Requirements:**
- Rustdoc on every public type, method, trait, and function with at least one inline example
- All doc examples compile and pass via `cargo test --doc`
- Standalone examples in `examples/` demonstrating every core feature across all 4 API styles

**Example Coverage:**

| Feature | Example Coverage |
|---|---|
| Steps (basic) | Simple step with checkpoint and replay |
| Steps (with retries) | Step with retry configuration and error handling |
| Steps (typed errors) | Step returning `Result<T, E>` with serializable error |
| Waits | Time-based suspension and resume |
| Callbacks | Registering callback, receiving external signal |
| Invoke | Calling another durable Lambda function |
| Parallel | Fan-out with multiple branches, completion criteria |
| Map | Processing a collection in parallel with batching |
| Child contexts | Isolated subflow with own checkpoint namespace |
| Replay-safe logging | Structured logging that deduplicates across replays |
| Combined workflow | End-to-end workflow using multiple operations together |

**Migration Guide:**
- Python-to-Rust conceptual mapping table
- Side-by-side Python and Rust code for each core operation
- Gotchas: determinism requirements, `Send + 'static` bounds for parallel, owned data in closures

### Implementation Considerations

- **Serde bounds everywhere** — all checkpoint values require `Serialize + DeserializeOwned`; no custom serialization layer
- **Send + 'static for parallel** — branch closures must be `Send + 'static` for `tokio::spawn`; owned child contexts eliminate shared mutable state
- **Eager history loading** — single `get_durable_execution_history` call at startup; replay is cursor-based over `Vec<HistoryEntry>`
- **Container deployment** — Lambda functions packaged as container images; Dockerfile provided in examples

## Functional Requirements

### Core Replay Engine

- FR1: The SDK can load the complete execution history for a durable function invocation from AWS on startup
- FR2: The SDK can distinguish between replay mode and execution mode based on history cursor position
- FR3: The SDK can return cached results from history during replay without re-executing the operation
- FR4: The SDK can execute new operations and checkpoint their results to AWS during execution mode
- FR5: The SDK can advance a positional cursor through the history log as each durable operation is encountered
- FR6: The SDK can serialize and deserialize checkpoint values using serde (`Serialize + DeserializeOwned`)
- FR7: The SDK can serialize and deserialize step errors using serde (`Serialize + DeserializeOwned`)

### Durable Operations — Steps

- FR8: Developers can define a named step that executes a closure and checkpoints the result
- FR9: Developers can configure retry behavior for a step (retry count, backoff strategy)
- FR10: Developers can return typed errors from steps that are checkpointed and replayed as the exact same typed error
- FR11: The SDK can skip step execution during replay and return the previously checkpointed result

### Durable Operations — Waits

- FR12: Developers can suspend a durable function for a specified duration without consuming compute
- FR13: The SDK can resume execution after the wait duration has elapsed

### Durable Operations — Callbacks

- FR14: Developers can register a callback and receive a callback ID for external systems to signal
- FR15: The SDK can suspend execution until a callback signal (success or failure) is received
- FR16: External systems can send callback success, failure, or heartbeat signals using the callback ID

### Durable Operations — Invoke

- FR17: Developers can durably invoke another Lambda function and receive its result
- FR18: The SDK can checkpoint the invocation result so it replays without re-invoking the target function

### Durable Operations — Parallel

- FR19: Developers can execute multiple branches concurrently within a single Lambda invocation
- FR20: Developers can configure completion criteria for parallel operations (e.g., all, any, N-of-M)
- FR21: Each parallel branch can execute durable operations independently with its own checkpoint namespace
- FR22: The SDK can run parallel branches as `tokio::spawn` tasks with `Send + 'static` bounds

### Durable Operations — Map

- FR23: Developers can process a collection of items in parallel using a closure applied to each item
- FR24: Developers can configure batching for map operations
- FR25: The SDK can return a `BatchResult<T>` containing the results of all map operations

### Durable Operations — Child Contexts

- FR26: Developers can create isolated subflows with their own checkpoint namespace
- FR27: Child contexts can execute any durable operation independently from the parent context
- FR28: The SDK can create fully owned child contexts that share only the AWS client (`Arc<LambdaService>`)

### Replay-Safe Logging

- FR29: Developers can emit structured log messages that are deduplicated across replays
- FR30: The SDK can integrate with the `tracing` crate for structured logging
- FR31: The SDK can suppress duplicate log output during the replay phase

### API Approaches

- FR32: Developers can use a proc-macro approach (`#[durable_execution]`) to define durable Lambda handlers
- FR33: Developers can use a trait-based approach to define durable Lambda handlers
- FR34: Developers can use a closure-native approach to define durable Lambda handlers
- FR35: Developers can use a builder-pattern approach to define durable Lambda handlers
- FR36: All 4 API approaches expose the same 8 core operations with identical behavior

### Testing

- FR37: Developers can create a `MockDurableContext` with pre-loaded step results for local testing
- FR38: Developers can run tests against `MockDurableContext` without AWS credentials
- FR39: Developers can verify the sequence and names of durable operations executed during a test
- FR40: The compliance suite can execute identical workflows in both Python and Rust and compare outputs

### Documentation & Examples

- FR41: Every public type, method, trait, and function has rustdoc with at least one inline example
- FR42: All doc examples compile and pass via `cargo test --doc`
- FR43: Standalone examples demonstrate every core feature across all 4 API styles
- FR44: A migration guide maps Python SDK concepts to Rust equivalents with side-by-side code

### Deployment

- FR45: Developers can package durable Lambda functions as container images
- FR46: The SDK provides a Dockerfile template for Lambda container builds
- FR47: The SDK integrates with the `lambda_runtime` crate for Lambda handler registration

### Error Handling

- FR48: The SDK can report errors via a typed `DurableError` enum with `thiserror` derives
- FR49: The SDK can propagate AWS SDK errors through the `DurableError` type
- FR50: The SDK can propagate serialization errors through the `DurableError` type

## Non-Functional Requirements

### Performance

- NFR1: SDK overhead per durable operation (replay or execute) must add < 1ms latency beyond the AWS API call itself
- NFR2: Eager history loading must complete within a single `get_durable_execution_history` call — no pagination loops during handler execution
- NFR3: Lambda memory baseline with the SDK loaded must remain under 32MB for a minimal handler (targeting 4-8x reduction vs Python's ~128MB)
- NFR4: Cold start time for a Rust durable Lambda container must be under 100ms (excluding AWS container initialization)

### Reliability

- NFR5: The replay engine must produce identical behavior to the Python SDK for identical inputs and execution sequences — zero tolerance for divergence
- NFR6: Checkpoint operations must be atomic — a partially written checkpoint must not corrupt the execution history
- NFR7: The SDK must handle AWS API transient failures (throttling, timeouts) with appropriate retries before surfacing errors

### Maintainability

- NFR8: Each API approach crate must depend only on `durable-lambda-core`, not on other approach crates — clean dependency boundaries
- NFR9: Adding a new core operation to the SDK must require changes only in `durable-lambda-core` and each approach crate — no cross-cutting changes
- NFR10: 100% test coverage across all SDK crates, measured via cargo tarpaulin or llvm-cov

### Developer Experience

- NFR11: A developer following SDK patterns must encounter zero ownership/borrowing/trait-bound compiler errors related to the SDK's API surface
- NFR12: Compiler error messages caused by incorrect SDK usage must be actionable — the developer must be able to identify and fix the issue without reading SDK internals
- NFR13: AI coding tools (Claude Code, Copilot) must generate compilable code on first or second attempt when following SDK patterns and doc examples
- NFR14: `cargo build` for the complete workspace must complete in under 60 seconds on a standard development machine (clean build)

### Compatibility

- NFR15: The SDK must compile on the latest stable Rust toolchain
- NFR16: The SDK must be compatible with the latest stable release of `aws-sdk-lambda`
- NFR17: The SDK must be compatible with the latest stable release of `lambda_runtime`
- NFR18: Container images must be deployable to AWS Lambda with the `provided.al2023` runtime

### Integration

- NFR19: The SDK must use the 9 `aws-sdk-lambda` durable execution API operations as its sole interface with AWS — no direct HTTP calls or undocumented APIs
- NFR20: The SDK must integrate with the `tracing` ecosystem for logging — no custom logging framework
