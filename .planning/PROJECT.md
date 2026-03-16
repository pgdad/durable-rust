# durable-rust

## What This Is

An idiomatic Rust SDK for AWS Lambda Durable Execution, providing full feature parity with the official AWS Python Durable Lambda SDK. Targets teams migrating from Python to Rust for cost, performance, and type-safety benefits at scale.

## Core Value

Every durable operation behaves identically to the Python SDK — zero behavioral divergence — while delivering Rust-native performance and safety.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

- ✓ Replay engine with deterministic blake2b operation ID generation — v1.0
- ✓ Step operation with optional retries and backoff — v1.0
- ✓ Wait operation (time-based suspension) — v1.0
- ✓ Callback operation (external signal coordination) — v1.0
- ✓ Invoke operation (Lambda-to-Lambda) — v1.0
- ✓ Parallel operation (concurrent fan-out) — v1.0
- ✓ Map operation (parallel collection processing with batching) — v1.0
- ✓ Child Context operation (isolated subflows) — v1.0
- ✓ Replay-safe structured logging — v1.0
- ✓ 4 API styles: Closure, Macro, Trait, Builder — v1.0
- ✓ Full cross-approach behavioral parity — v1.0
- ✓ MockDurableContext testing framework with assertion helpers — v1.0
- ✓ Python-Rust compliance suite — v1.0
- ✓ 28 end-to-end workflow tests — v1.0
- ✓ 44 examples (11 per API style) — v1.0
- ✓ Migration guide and documentation — v1.0
- ✓ Container deployment targeting provided.al2023 — v1.0
- ✓ GSD planning infrastructure — v1.1
- ✓ BMAD tooling removed, clean repo — v1.1

### Active

<!-- Current scope. Building toward these. -->

(None — define in next milestone)

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- SDK feature work — deferred to next milestone
- CI/CD pipeline changes — separate concern

## Context

- Workspace with 7 crates: core, macro, closure, trait, builder, testing + compliance
- All crates depend on `durable-lambda-core`; no circular dependencies
- 16,699 lines of Rust code
- Project managed with GSD workflow (transitioned from BMAD in v1.1)
- Young dev team using AI coding assistants (Claude Code, Copilot)

## Constraints

- **Tech stack**: Rust stable 1.82.0+, tokio, aws-sdk-lambda 1.118+
- **Tooling**: GSD workflow for all project management

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Switch from BMAD to GSD | GSD better fits team workflow with Claude Code | ✓ Good |
| Capture existing SDK as v1.0 Validated | Establishes baseline without re-validating shipped work | ✓ Good |
| Remove BMAD artifacts in separate commits | Clean separation of concerns in git history | ✓ Good |
| 4 atomic commits for BMAD removal | Output → framework → skills → doc cleanup ordering | ✓ Good |

---
*Last updated: 2026-03-16 after v1.1 milestone*
