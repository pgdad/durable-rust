---
phase: 8
slug: macro-builder-improvements
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) + trybuild |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test -p durable-lambda-macro && cargo test -p durable-lambda-builder` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p durable-lambda-macro` or `cargo test -p durable-lambda-builder`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 8-01-01 | 01 | 1 | FEAT-29 | unit | `cargo test -p durable-lambda-macro -- validate` | ✅ exists | ⬜ pending |
| 8-01-02 | 01 | 1 | FEAT-30 | unit | `cargo test -p durable-lambda-macro -- validate_return` | ❌ W0 | ⬜ pending |
| 8-01-03 | 01 | 1 | FEAT-31 | trybuild | `cargo test -p durable-lambda-macro -- trybuild` | ✅ exists | ⬜ pending |
| 8-02-01 | 02 | 1 | FEAT-32 | unit | `cargo test -p durable-lambda-builder -- with_tracing` | ❌ W0 | ⬜ pending |
| 8-02-02 | 02 | 1 | FEAT-33 | unit | `cargo test -p durable-lambda-builder -- with_error_handler` | ❌ W0 | ⬜ pending |
| 8-02-03 | 02 | 1 | FEAT-34 | unit | `cargo test -p durable-lambda-builder -- config_takes_effect` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Test stubs for FEAT-30 in macro expand.rs tests
- [ ] Test stubs for FEAT-32, FEAT-33, FEAT-34 in builder handler.rs tests

*Existing trybuild infrastructure covers FEAT-31.*

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
