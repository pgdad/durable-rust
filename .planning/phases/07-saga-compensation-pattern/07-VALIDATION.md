---
phase: 7
slug: saga-compensation-pattern
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 7 — Validation Strategy

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
| 7-01-01 | 01 | 1 | FEAT-25 | unit | `cargo test -p durable-lambda-core -- step_with_compensation` | ❌ W0 | ⬜ pending |
| 7-01-02 | 01 | 1 | FEAT-26 | unit | `cargo test -p durable-lambda-core -- compensation_reverse_order` | ❌ W0 | ⬜ pending |
| 7-02-01 | 02 | 2 | FEAT-27 | integration | `cargo test -p e2e-tests -- compensation_checkpoint` | ❌ W0 | ⬜ pending |
| 7-02-02 | 02 | 2 | FEAT-28 | integration | `cargo test -p e2e-tests -- compensation_failure` | ❌ W0 | ⬜ pending |
| 7-02-03 | 02 | 2 | FEAT-28 | integration | `cargo test -p e2e-tests -- partial_rollback` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Test stubs for FEAT-25, FEAT-26 in core operation tests
- [ ] Test stubs for FEAT-27, FEAT-28 in `tests/e2e/`

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
