# Milestones: durable-rust

## Milestone History

<details>
<summary>v1.1 — GSD Tooling Transition (shipped 2026-03-16)</summary>

### v1.1 — GSD Tooling Transition

**Shipped:** 2026-03-16
**Phases:** 2 | **Plans:** 2 | **Tasks:** 5
**Timeline:** 2 days (2026-03-14 → 2026-03-16)
**Git stats:** 660 files changed, 2,915 insertions, 102,778 deletions

**Key accomplishments:**
- Established GSD planning infrastructure (PROJECT.md, MILESTONES.md, REQUIREMENTS.md, ROADMAP.md, STATE.md)
- Captured v1.0 SDK as shipped milestone with 20 validated capabilities and 7 design decisions
- Removed 545+ BMAD files across `_bmad/`, `_bmad-output/`, and `.claude/skills/bmad-*` in 4 atomic commits
- Cleaned all functional `_bmad` references from planning documents
- Verified zero Rust source changes throughout the tooling transition

**Phases:**
- Phase 1: GSD Infrastructure — verified all planning files, advanced state
- Phase 2: BMAD Cleanup — 4 atomic removal commits, reference cleanup, final verification

</details>

<details>
<summary>v1.0 — Initial SDK Release (shipped 2026-03-13)</summary>

### v1.0 — Initial SDK Release

**Shipped:** 2026-03-13
**Managed under:** BMAD workflow
**Scope:** Complete idiomatic Rust SDK for AWS Lambda Durable Execution, targeting feature parity with the official AWS Python Durable Lambda SDK.

### Delivered Capabilities

| # | Capability | Notes |
|---|-----------|-------|
| 1 | Replay engine with deterministic blake2b operation ID generation | blake2b chosen for collision resistance + speed at short input lengths; operation IDs are content-addressed from function name + cursor position |
| 2 | Step operation with optional retries and configurable backoff | Exponential and fixed backoff strategies; retry state is checkpointed so replays resume mid-retry correctly |
| 3 | Wait operation (time-based suspension) | Lambda suspends without consuming compute; checkpoint records the target resume timestamp |
| 4 | Callback operation (external signal coordination) | External systems signal via callback ID; SDK suspends until success, failure, or heartbeat |
| 5 | Invoke operation (Lambda-to-Lambda durable invocation) | Checkpoint captures invocation result; replay returns cached result without re-invoking target |
| 6 | Parallel operation (concurrent fan-out) | tokio::spawn-based; each branch has isolated checkpoint namespace; configurable completion criteria (all/any/N-of-M) |
| 7 | Map operation (parallel collection processing with batching) | Returns BatchResult<T>; batching config controls concurrency ceiling |
| 8 | Child Context operation (isolated subflows) | Fully owned child contexts share only Arc<LambdaService>; no checkpoint namespace collision with parent |
| 9 | Replay-safe structured logging | tracing integration with deduplication layer that suppresses duplicate log output during replay phase |
| 10 | Closure API style | Ergonomic closure-native approach; hides Send + 'static ownership requirements behind owned-data patterns |
| 11 | Macro API style (#[durable_execution]) | Proc-macro approach; least boilerplate for simple handlers |
| 12 | Trait API style | Trait-based approach; most explicit; best for complex handlers |
| 13 | Builder API style | Builder-pattern approach; most configurable |
| 14 | Full cross-approach behavioral parity | All 4 API styles expose identical operation semantics; compliance verified by test suite |
| 15 | MockDurableContext testing framework | Pre-loaded step results; no AWS credentials required; assertion helpers for operation sequence verification |
| 16 | Python-Rust compliance suite | Executes identical workflows in Python and Rust; compares outputs byte-for-byte |
| 17 | 28 end-to-end workflow tests | Cover all operation types and error paths |
| 18 | 44 examples (11 per API style) | Demonstrate every core feature across all 4 API styles |
| 19 | Migration guide and documentation | Maps Python SDK concepts to Rust equivalents with side-by-side code; all rustdoc examples compile and pass |
| 20 | Container deployment targeting provided.al2023 | Dockerfile template for Lambda container builds; lambda_runtime integration |

### Workspace Structure

7 crates: `durable-lambda-core`, `durable-lambda-macro`, `durable-lambda-closure`, `durable-lambda-trait`, `durable-lambda-builder`, `durable-lambda-testing`, `durable-lambda-compliance`

All approach crates depend only on `durable-lambda-core` — no circular dependencies, no cross-approach dependencies.

### Key Design Decisions (from v1.0 planning artifacts)

| Decision | Rationale |
|----------|-----------|
| blake2b for operation IDs | Content-addressed from function name + cursor position; collision-resistant at short inputs; faster than SHA-2 for small payloads |
| 4 independent API style crates | Teams adopting the SDK choose one style; mixing styles in a codebase would create inconsistency; isolation enforces this |
| MockDurableContext abstraction | Requires clean interface boundary between operation logic and AWS backend; enables 100% local testing without credentials |
| Send + 'static for parallel branches | tokio::spawn requires these bounds; owned-data pattern chosen over Arc<Mutex<>> to avoid deadlock footguns |
| serde for all checkpoint types | Single serialization mechanism; user types need only Serialize + DeserializeOwned; no custom derive needed |
| provided.al2023 container target | Smallest Lambda runtime image; best cold start for Rust binaries; avoids amazonlinux2 glibc version issues |
| Python compliance suite | Zero-divergence guarantee requires executable proof, not just assertion; same workflow inputs produce identical outputs |

### Tech Stack

- Rust stable 1.82.0+
- tokio (mandated by aws-sdk-lambda)
- aws-sdk-lambda 1.118+
- lambda_runtime
- serde
- thiserror
- tracing
- blake2 (operation ID hashing)

</details>

---

## Active Milestone

None — run `/gsd:new-milestone` to start next milestone.

---
*Last updated: 2026-03-16 after v1.1 completion*
