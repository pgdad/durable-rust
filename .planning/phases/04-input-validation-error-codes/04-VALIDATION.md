---
phase: 4
slug: input-validation-error-codes
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-16
---

# Phase 4 — Validation Strategy

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
| 4-01-01 | 01 | 1 | FEAT-01 | unit | `cargo test -p durable-lambda-core step_options_validation` | ❌ W0 | ⬜ pending |
| 4-01-02 | 01 | 1 | FEAT-02 | unit | `cargo test -p durable-lambda-core callback_options_validation` | ❌ W0 | ⬜ pending |
| 4-01-03 | 01 | 1 | FEAT-03 | unit | `cargo test -p durable-lambda-core map_options_validation` | ❌ W0 | ⬜ pending |
| 4-01-04 | 01 | 1 | FEAT-04 | unit | `cargo test -p durable-lambda-core options_panic_messages` | ❌ W0 | ⬜ pending |
| 4-02-01 | 02 | 1 | FEAT-05 | unit | `cargo test -p durable-lambda-core error_code` | ❌ W0 | ⬜ pending |
| 4-02-02 | 02 | 1 | FEAT-06 | unit | `cargo test -p durable-lambda-core all_variants_have_unique_codes` | ❌ W0 | ⬜ pending |
| 4-02-03 | 02 | 2 | FEAT-07 | unit | `cargo test -p durable-lambda-core retry_uses_error_codes` | ❌ W0 | ⬜ pending |
| 4-02-04 | 02 | 2 | FEAT-08 | unit | `cargo test -p durable-lambda-core checkpoint_token_none_handling` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Test stubs for FEAT-01 through FEAT-04 in `crates/durable-lambda-core/src/types.rs` (inline tests)
- [ ] Test stubs for FEAT-05 through FEAT-06 in `crates/durable-lambda-core/src/error.rs` (inline tests)
- [ ] Test stubs for FEAT-07 in `crates/durable-lambda-core/src/backend.rs` (inline tests)
- [ ] Test stubs for FEAT-08 across operation files in `crates/durable-lambda-core/src/operations/`

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
