---
phase: 2
slug: boundary-replay-engine-tests
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-16
---

# Phase 2 — Validation Strategy

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
| 2-01-01 | 01 | 1 | TEST-12 | unit | `cargo test -p durable-lambda-core -- wait_zero_duration` | ❌ W0 | ⬜ pending |
| 2-01-02 | 01 | 1 | TEST-13 | unit | `cargo test -p durable-lambda-core -- map_batch_size` | ❌ W0 | ⬜ pending |
| 2-01-03 | 01 | 1 | TEST-14 | unit | `cargo test -p durable-lambda-core -- parallel_zero_branches` | ❌ W0 | ⬜ pending |
| 2-01-04 | 01 | 1 | TEST-15 | unit | `cargo test -p durable-lambda-core -- operation_name` | ❌ W0 | ⬜ pending |
| 2-01-05 | 01 | 1 | TEST-16 | unit | `cargo test -p durable-lambda-core -- negative_option` | ❌ W0 | ⬜ pending |
| 2-02-01 | 02 | 1 | TEST-17 | integration | `cargo test -p e2e-tests -- nested_child_context` | ❌ W0 | ⬜ pending |
| 2-02-02 | 02 | 1 | TEST-18 | integration | `cargo test -p e2e-tests -- parallel_in_child_in_parallel` | ❌ W0 | ⬜ pending |
| 2-03-01 | 03 | 1 | TEST-19 | unit | `cargo test -p durable-lambda-core -- deterministic_replay` | ❌ W0 | ⬜ pending |
| 2-03-02 | 03 | 1 | TEST-20 | unit | `cargo test -p durable-lambda-core -- duplicate_operation_ids` | ❌ W0 | ⬜ pending |
| 2-03-03 | 03 | 1 | TEST-21 | unit | `cargo test -p durable-lambda-core -- history_gap` | ❌ W0 | ⬜ pending |
| 2-03-04 | 03 | 1 | TEST-22 | unit | `cargo test -p durable-lambda-core -- checkpoint_token_evolution` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Test stubs for TEST-12 through TEST-16 in core operation test modules
- [ ] Test stubs for TEST-17, TEST-18 in `tests/e2e/tests/e2e_workflows.rs`
- [ ] Test stubs for TEST-19 through TEST-22 in `crates/durable-lambda-core/src/replay.rs`

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
