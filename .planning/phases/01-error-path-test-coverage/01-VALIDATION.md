---
phase: 1
slug: error-path-test-coverage
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-16
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test framework via `cargo test` |
| **Config file** | `Cargo.toml` workspace with `[dev-dependencies]` per crate |
| **Quick run command** | `cargo test -p e2e-tests error_paths` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p e2e-tests error_paths`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | TEST-01 | unit | `cargo test -p e2e-tests error_paths::replay_mismatch` | -- W0 | pending |
| 01-01-02 | 01 | 1 | TEST-02 | unit | `cargo test -p e2e-tests error_paths::serialization_mismatch` | -- W0 | pending |
| 01-01-03 | 01 | 1 | TEST-03 | unit | `cargo test -p e2e-tests error_paths::checkpoint_failure` | -- W0 | pending |
| 01-01-04 | 01 | 1 | TEST-04 | unit | `cargo test -p e2e-tests error_paths::retry_exhaustion` | -- W0 | pending |
| 01-01-05 | 01 | 1 | TEST-05 | unit | `cargo test -p e2e-tests error_paths::callback_timeout` | -- W0 | pending |
| 01-01-06 | 01 | 1 | TEST-06 | unit | `cargo test -p e2e-tests error_paths::callback_failure` | -- W0 | pending |
| 01-01-07 | 01 | 1 | TEST-07 | unit | `cargo test -p e2e-tests error_paths::invoke_failure` | -- W0 | pending |
| 01-02-01 | 02 | 2 | TEST-08 | integration | `cargo test -p e2e-tests error_paths::parallel_all_fail` | -- W0 | pending |
| 01-02-02 | 02 | 2 | TEST-09 | integration | `cargo test -p e2e-tests error_paths::map_item_failures` | -- W0 | pending |
| 01-02-03 | 02 | 2 | TEST-11 | integration | `cargo test -p e2e-tests error_paths::parallel_branch_panic` | -- W0 | pending |
| 01-03-01 | 03 | 3 | TEST-10 | unit+fix | `cargo test -p e2e-tests error_paths::step_closure_panic` | -- W0 | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [ ] `tests/e2e/tests/error_paths.rs` — test file covering TEST-01 through TEST-11
- [ ] `FailingMockBackend` helper struct (inline in error_paths.rs or shared helper)
- [ ] Production fix in `crates/durable-lambda-core/src/operations/step.rs` for TEST-10 (panic catching)

*Existing test infrastructure in `tests/e2e/tests/e2e_workflows.rs` covers happy paths; this phase adds the error path complement.*

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
