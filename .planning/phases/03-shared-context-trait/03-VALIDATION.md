---
phase: 3
slug: shared-context-trait
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-16
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (tokio::test for async) |
| **Config file** | Cargo.toml workspace members |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | ARCH-01 | unit (compile check) | `cargo build -p durable-lambda-core` | ❌ W0 | ⬜ pending |
| 03-01-02 | 01 | 1 | ARCH-02 | unit (compile check) | `cargo build -p durable-lambda-closure` | ❌ W0 | ⬜ pending |
| 03-01-03 | 01 | 1 | ARCH-03 | unit (compile check) | `cargo build -p durable-lambda-trait` | ❌ W0 | ⬜ pending |
| 03-01-04 | 01 | 1 | ARCH-04 | unit (compile check) | `cargo build -p durable-lambda-builder` | ❌ W0 | ⬜ pending |
| 03-02-01 | 02 | 2 | ARCH-05 | integration | `cargo test -p parity-tests generic_handler` | ❌ W0 | ⬜ pending |
| 03-02-02 | 02 | 2 | ARCH-06 | unit + compile | `cargo build --workspace` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/durable-lambda-core/src/ops_trait.rs` — DurableContextOps trait with all 21 methods, plus impl for DurableContext (ARCH-01, ARCH-05)
- [ ] `impl DurableContextOps for ClosureContext` in `crates/durable-lambda-closure/src/context.rs` (ARCH-02)
- [ ] `impl DurableContextOps for TraitContext` in `crates/durable-lambda-trait/src/context.rs` (ARCH-03)
- [ ] `impl DurableContextOps for BuilderContext` in `crates/durable-lambda-builder/src/context.rs` (ARCH-04)
- [ ] Generic handler test in `tests/parity/tests/parity.rs` (ARCH-05)
- [ ] `parse_invocation()` + `InvocationData` in `crates/durable-lambda-core/src/event.rs` (ARCH-06)

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
