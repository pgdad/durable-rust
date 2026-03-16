---
phase: 6
slug: observability-batch-checkpoint
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-16
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test -p durable-lambda-core` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p durable-lambda-core`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 6-01-01 | 01 | 1 | FEAT-17 | unit | `cargo test -p durable-lambda-core -- span` | ❌ W0 | ⬜ pending |
| 6-01-02 | 01 | 1 | FEAT-18 | unit | `cargo test -p durable-lambda-core -- nested_span` | ❌ W0 | ⬜ pending |
| 6-01-03 | 01 | 1 | FEAT-19 | unit | `cargo test -p durable-lambda-core -- span_enter_exit` | ❌ W0 | ⬜ pending |
| 6-01-04 | 01 | 1 | FEAT-20 | unit | `cargo test -p durable-lambda-core -- span_fields` | ❌ W0 | ⬜ pending |
| 6-02-01 | 02 | 2 | FEAT-21 | unit | `cargo test -p durable-lambda-core -- batch_checkpoint` | ❌ W0 | ⬜ pending |
| 6-02-02 | 02 | 2 | FEAT-22 | unit | `cargo test -p durable-lambda-core -- batch_opt_in` | ❌ W0 | ⬜ pending |
| 6-02-03 | 02 | 2 | FEAT-23 | unit | `cargo test -p durable-lambda-core -- batch_single_call` | ❌ W0 | ⬜ pending |
| 6-02-04 | 02 | 2 | FEAT-24 | integration | `cargo test -p e2e-tests -- batch_fewer_calls` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Test stubs for FEAT-17 through FEAT-20 in operation test modules
- [ ] Test stubs for FEAT-21 through FEAT-24 in core/e2e test modules

*Existing test infrastructure covers all framework needs.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
