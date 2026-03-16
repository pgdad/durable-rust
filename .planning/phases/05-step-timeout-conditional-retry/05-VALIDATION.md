---
phase: 5
slug: step-timeout-conditional-retry
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-16
---

# Phase 5 — Validation Strategy

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
| 5-01-01 | 01 | 1 | FEAT-09 | unit | `cargo test -p durable-lambda-core -- step_options_timeout` | ❌ W0 | ⬜ pending |
| 5-01-02 | 01 | 1 | FEAT-13 | unit | `cargo test -p durable-lambda-core -- step_options_retry_if` | ❌ W0 | ⬜ pending |
| 5-02-01 | 02 | 2 | FEAT-10,11 | unit | `cargo test -p durable-lambda-core -- step_timeout` | ❌ W0 | ⬜ pending |
| 5-02-02 | 02 | 2 | FEAT-12 | unit | `cargo test -p durable-lambda-core -- timeout_within` | ❌ W0 | ⬜ pending |
| 5-02-03 | 02 | 2 | FEAT-14 | unit | `cargo test -p durable-lambda-core -- retry_if_predicate` | ❌ W0 | ⬜ pending |
| 5-02-04 | 02 | 2 | FEAT-15 | unit | `cargo test -p durable-lambda-core -- retry_default_all` | ❌ W0 | ⬜ pending |
| 5-02-05 | 02 | 2 | FEAT-16 | unit | `cargo test -p durable-lambda-core -- retry_if_transient` | ❌ W0 | ⬜ pending |
| 5-03-01 | 03 | 3 | TEST-23 | integration | `cargo test -p parity-tests -- step_timeout_parity` | ❌ W0 | ⬜ pending |
| 5-03-02 | 03 | 3 | TEST-24 | integration | `cargo test -p parity-tests -- conditional_retry_parity` | ❌ W0 | ⬜ pending |
| 5-03-03 | 03 | 3 | TEST-25 | integration | `cargo test -p e2e-tests -- batch_item_status` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Test stubs for FEAT-09 through FEAT-16 in core step operation tests
- [ ] Test stubs for TEST-23, TEST-24 in `tests/parity/tests/parity.rs`
- [ ] Test stubs for TEST-25 in `tests/e2e/tests/e2e_workflows.rs`

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
