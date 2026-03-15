---
stepsCompleted: [1, 2, 3, 4, 5]
inputDocuments:
  - '_bmad-output/brainstorming/brainstorming-session-2026-03-13-1257.md'
date: 2026-03-13
author: Esa
---

# Product Brief: durable-rust

## Executive Summary

durable-rust is an idiomatic Rust SDK for AWS Lambda Durable Functions, providing full feature parity with the official AWS Python Durable Lambda SDK. It enables development teams to build resilient, long-running Lambda workflows in Rust — achieving significant compute cost reductions through Rust's lower CPU and memory footprint compared to Python and Java runtimes.

At billions of daily Lambda invocations across multiple departments, Rust's typical 4-8x memory reduction over Python (e.g., ~16-32MB vs ~128MB baseline) translates to millions in annual infrastructure savings. Combined with dramatically faster cold start times, the economic case compounds across both active compute and initialization overhead.

The SDK prioritizes developer accessibility: a team with 2-3 years of general development experience and minimal Rust knowledge should be able to build a simple durable Lambda function within 2 days using AI coding assistants, and migrate an existing Python durable Lambda to Rust within 1 week. The API is designed as a "pit of success" — hiding Rust's ownership complexity so that the simplest thing to write is also correct.

The project is initially internal with documentation and architecture designed for future open-sourcing. The AWS Rust SDK's maturity reaching production-readiness is the trigger for this initiative.

---

## Core Vision

### Problem Statement

Organizations running durable Lambda workloads at massive scale (billions of daily invocations) in Python and Java pay a significant premium in compute costs due to higher memory and CPU requirements of these runtimes. There is no Rust SDK for AWS Lambda Durable Functions, preventing teams from leveraging Rust's lower resource footprint for durable workflows despite the AWS Rust SDK being production-ready.

### Problem Impact

- **Direct cost impact:** At billions of daily invocations, Rust's typical 4-8x memory reduction over Python and significantly lower CPU utilization translate to millions in estimated annual savings. These savings scale linearly with invocation growth.
- **Cold start overhead:** Python and Java Lambda cold starts add latency and cost across high-volume workloads. Rust cold starts are an order of magnitude faster, reducing both user-facing latency and billed compute time.
- **Workload diversity:** Durable functions range from multi-day long-running processes to few-hour coordination workflows — all accumulating unnecessary compute costs during active execution phases.
- **No migration path exists:** Without a Rust Durable Lambda SDK, teams cannot incrementally adopt Rust for durable workloads even when the economic case is clear.

### Why Existing Solutions Fall Short

- **AWS only provides SDKs for Python, TypeScript, and Java** — no official Rust support exists for durable Lambda functions. AWS typically lags 12-18 months on new language SDKs after the initial launch languages, creating a window for this initiative.
- **AWS Step Functions** are not permitted under corporate technology standards, eliminating the primary alternative for durable workflows.
- **Other durable execution frameworks (Temporal, Restate)** require infrastructure outside the Lambda ecosystem, which violates the hard requirement to stay within AWS Lambda.
- **No community Rust crate** currently provides durable Lambda function support with feature parity to the official SDKs.
- **AWS official Rust SDK risk:** AWS could release an official Rust Durable Lambda SDK in the future. This is mitigated by the open-source strategy — production-validated API design and patterns could influence or contribute to an eventual official SDK. In the interim, this initiative fills a gap that may persist for 12-18+ months.

### Proposed Solution

A Rust SDK for AWS Lambda Durable Functions built on `aws-sdk-lambda`, providing all 8 core operations from the Python SDK: steps (checkpointed work with retries), waits (time-based suspension), callbacks (external signal coordination), invoke (durable function composition), parallel (fan-out with completion criteria), map (parallel collection processing), child contexts (isolated subflows), and replay-safe logging.

The SDK will be implemented as a cargo workspace with a shared core replay engine and 4 distinct API styles (proc-macro, trait-based, closure-native, builder-pattern) to be evaluated by the development team. A dedicated testing crate (`durable-lambda-testing`) is a critical component — not optional — providing a `MockDurableContext` for local development and testing without AWS credentials. Without local testability, the "2 days to a working durable Lambda" target is unachievable.

The API is designed as a "pit of success" — Rust's ownership model, borrow checker complexity, and async trait bounds are abstracted behind simple, pattern-based interfaces that AI coding tools can generate reliably. Developers should rarely fight the compiler when following the SDK's patterns.

Rollout strategy: new durable Lambda applications built in Rust first, with migration of existing Python/Java functions when project schedules permit.

### Key Differentiators

- **Cost reduction at scale:** Rust's 4-8x lower memory footprint and reduced CPU utilization directly cut per-invocation costs across billions of daily executions.
- **Cold start performance:** Rust Lambda cold starts are dramatically faster than Python/Java — a compounding advantage at high invocation volumes with diverse workload patterns.
- **Full feature parity:** All capabilities of the official Python SDK, not a subset — ensuring no functional compromises when adopting Rust.
- **AI-assistance optimized:** API design explicitly prioritizes patterns that AI coding tools (Claude Code, Copilot) can generate reliably, enabling junior developers to be productive quickly.
- **Pit of success API:** Hides Rust's ownership and borrow checker complexity so the simplest code to write is also correct. Developers with minimal Rust experience can be productive without deep language expertise.
- **Designed for production scale:** Architecture decisions (eager history loading, owned child contexts, always-parallel concurrency model) are informed by production workloads at billions of daily invocations.
- **Open-source ready:** Documentation and architecture designed from day one for eventual public release, positioning to influence or contribute to a potential future AWS official Rust SDK.

## Target Users

### Primary Users

**Developer — "Alex" (Junior/Mid-Level)**
- **Profile:** 2-3 years of development experience, primarily in Python or Java. Minimal or no Rust experience. Comfortable with AWS Lambda concepts but new to durable functions.
- **Context:** Building new durable Lambda functions from scratch — durable functions are a new AWS feature with limited existing production code to reference.
- **Motivations:** Wants to be productive quickly, relies heavily on AI coding assistants (Claude Code, Copilot) to learn patterns and generate code. Values clear examples and predictable API patterns over flexibility.
- **Pain points:** Rust's ownership model and borrow checker are unfamiliar. Complex trait bounds and generic type errors are discouraging. Lack of existing Rust durable Lambda examples means no code to copy from.
- **Success moment:** Writes a working durable Lambda function with steps, waits, and parallel operations in under 2 days using AI assistance, without fighting the compiler.

**Developer — "Jordan" (Senior)**
- **Profile:** 5+ years of development experience, strong in Python or Java. May have some Rust exposure but not deep expertise. Experienced with AWS Lambda and now learning durable functions.
- **Context:** Also building from scratch — evaluating how to structure durable workflows, establishing patterns that junior developers will follow.
- **Motivations:** Wants an SDK that produces maintainable, well-structured code. Cares about error handling, testability, and production reliability. Needs confidence that the SDK handles edge cases correctly.
- **Pain points:** Needs to validate that the Rust SDK behaves identically to the Python SDK for the same workflow logic. Wants to establish team patterns quickly without extensive Rust expertise.
- **Success moment:** Builds a production-ready durable workflow, writes comprehensive tests locally using the testing crate, and establishes reusable patterns that junior developers can follow.

### Secondary Users

**Tech Lead / Architect — "Morgan"**
- **Profile:** Engineering leader responsible for evaluating and standardizing SDK adoption. Reviews and approves durable Lambda code produced by development teams. Designs workflow patterns and best practices.
- **Context:** Evaluates which of the 4 API approaches to adopt as the team standard. Defines internal patterns and guidelines for durable Lambda development.
- **Motivations:** Wants a single, clear API approach that minimizes team friction and support burden. Needs confidence in correctness, performance characteristics, and maintainability. Cares about open-source readiness of documentation.
- **Pain points:** Must justify the Rust investment to leadership with concrete cost savings data. Needs to ensure the chosen API approach works well with AI coding assistants across the team.
- **Success moment:** Selects one API approach, publishes internal guidelines, and sees the team consistently producing correct durable Lambda functions without escalations.

**Engineering Leadership — "Sam"**
- **Profile:** Decision-maker who approves the adoption of the Rust Durable Lambda SDK across departments.
- **Context:** Focused on the cost reduction business case at scale — billions of daily invocations across multiple departments.
- **Motivations:** Measurable cost savings, reduced operational risk, and team productivity maintained or improved despite language change.
- **Pain points:** Risk of productivity loss during Rust adoption. Needs evidence that junior developers can be effective with the new SDK.
- **Success moment:** Sees the first production Rust durable Lambda delivering measurable cost savings with no increase in incident rate or development cycle time.

### User Journey

1. **Discovery:** Engineering leadership identifies Lambda compute cost as a significant line item. Architects evaluate Rust as a lower-cost runtime alternative. The durable-rust SDK is identified as the enabler for durable workloads.
2. **Evaluation:** Tech leads compare the 4 API approaches using the provided examples. Team runs the same order-processing workflow in all 4 styles, evaluates with AI coding assistants, and selects one approach.
3. **Onboarding:** Developers start with the SDK's example projects and documentation. Using AI coding assistants, they build their first simple durable Lambda function within 2 days. The testing crate enables rapid local iteration without AWS deployment.
4. **Production adoption:** First durable Lambda functions deployed to production for new workloads. Cost savings measured and reported. Patterns stabilize into internal best practices.
5. **Scaling:** Additional teams adopt the SDK. Existing Python/Java durable functions are migrated when project schedules permit. Cost savings compound across departments.

## Success Metrics

### User Success Metrics

- **Developer onboarding speed:** A developer with 2-3 years of experience and minimal Rust knowledge builds a working durable Lambda function (with steps, waits, and parallel operations) within 2 days using AI coding assistants.
- **Migration velocity:** An existing Python durable Lambda function is successfully migrated to Rust within 1 week by a senior developer.
- **Developer friction:** Developers following the SDK's patterns rarely encounter compiler errors related to ownership, borrowing, or trait bounds — the API is a "pit of success."
- **AI assistant effectiveness:** AI coding tools (Claude Code, Copilot) consistently generate correct, compilable durable Lambda code on first or second attempt when following SDK patterns.

### Business Objectives

- **Cost reduction:** Minimum 10% aggregate reduction in Lambda compute costs (purely compute, excluding other AWS charges) when durable workloads are migrated from Python/Java to Rust.
- **Production adoption (3 months):** At least 1 production durable Lambda function running in Rust.
- **Production adoption (12 months):** At least 10 production durable Lambda functions running in Rust across departments.
- **Reliability:** Zero regressions in durable function behavior compared to equivalent Python implementations. Rust functions must produce identical results for identical inputs and execution sequences.

### Key Performance Indicators

| KPI | Target | Measurement Method |
|---|---|---|
| Aggregate Lambda compute cost reduction | ≥ 10% | AWS Cost Explorer comparison before/after Rust adoption |
| Production Rust durable Lambdas (3 months) | ≥ 1 | Deployment inventory |
| Production Rust durable Lambdas (12 months) | ≥ 10 | Deployment inventory |
| SDK test coverage (unit + integration) | 100% | cargo tarpaulin / llvm-cov on SDK crates |
| SDK test coverage (real AWS deployment) | 100% of core operations | Integration test suite against live AWS Lambda |
| Behavioral compliance | 0 regressions | Compliance Lambda suite — 3-5 representative workflows implemented in both Python and Rust, outputs verified identical |
| Developer onboarding time | ≤ 2 days | Measured during pilot team onboarding |
| Migration time (per function) | ≤ 1 week | Measured during first Python-to-Rust migrations |
| Incident rate | No increase | Comparison of incident rate for Rust vs Python durable Lambdas |

### Compliance Validation Strategy

A suite of 3-5 representative durable Lambda functions will be implemented in both Python (using the official SDK) and Rust (using durable-rust). Each function will exercise different combinations of core operations (steps, waits, callbacks, invoke, parallel, map, child contexts, logging). The outputs of both implementations will be compared for identical results given identical inputs and execution sequences, serving as the primary compliance benchmark.

## MVP Scope

### Core Features

- **Full SDK implementation** — all 8 core operations from day one: steps, waits, callbacks, invoke, parallel, map, child contexts, and replay-safe logging
- **4 API approaches** — proc-macro, trait-based, closure-native, and builder-pattern, implemented as sibling crates in a cargo workspace for team evaluation and comparison
- **Shared core replay engine** (`durable-lambda-core`) — eager history loading, cursor-based replay, serde-only serialization, typed/serializable errors, owned child contexts
- **Testing crate** (`durable-lambda-testing`) — `MockDurableContext` for local development and testing without AWS credentials
- **Compliance Lambda suite** — 3-5 representative workflows implemented in both Python and Rust, with output comparison for behavioral verification
- **Container-based deployment** — Lambda functions packaged and deployed as container images
- **Comparison examples** — identical order-processing workflow implemented in all 4 API styles
- **Open-source-ready documentation** — comprehensive doc comments, examples, and guides suitable for eventual public release

### Out of Scope for MVP

- **AWS Step Functions** — not a permitted technology per corporate standards; no compatibility or integration
- **Performance benchmarking tooling** — cost savings measured via AWS Cost Explorer, not custom tooling
- **Migration automation tooling** — Python-to-Rust migrations are manual, guided by documentation and examples
- **Non-container deployment models** — ZIP-based Lambda packaging not supported; container images only
- **Non-Lambda deployment targets** — SDK is purpose-built for AWS Lambda durable functions

### MVP Success Criteria

- All 8 core operations functional and tested (unit + integration + live AWS deployment)
- All 4 API approaches implement the same feature set and pass the same test suite
- Compliance Lambda suite confirms zero behavioral regressions vs Python SDK
- A developer with 2-3 years experience builds a working durable Lambda within 2 days using AI assistance
- At least 1 production durable Lambda deployed within 3 months of SDK completion
- 100% test coverage across all SDK crates

### Future Vision

- **AWS adoption** — position the SDK for potential adoption as the official AWS Rust Durable Lambda SDK, or as a significant influence on its design
- **Open-source release** — publish to crates.io once production-validated internally
- **API approach consolidation** — after team evaluation, converge on a single recommended API approach while maintaining the others as alternatives
- **Ecosystem growth** — community contributions, additional examples, and integration patterns as adoption scales across departments
