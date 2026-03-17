---
phase: 16
slug: advanced-feature-tests
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 16 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Bash integration tests in `scripts/test-all.sh` + `scripts/test-helpers.sh` |
| **Config file** | None — sourced from `scripts/test-helpers.sh` |
| **Quick run command** | `bash scripts/test-all.sh closure-saga-compensation` |
| **Full suite command** | `bash scripts/test-all.sh` |
| **Estimated runtime** | ~120 seconds (4 Lambda invocations + polling) |

---

## Sampling Rate

- **After every task commit:** Run `bash scripts/test-all.sh closure-saga-compensation` (single handler, quickest feedback)
- **After every plan wave:** Run `bash scripts/test-all.sh` (full suite)
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 16-01-01 | 01 | 1 | ADV-01 | integration | `bash scripts/test-all.sh closure-saga-compensation` | ❌ W0 | ⬜ pending |
| 16-01-02 | 01 | 1 | ADV-02 | integration | `bash scripts/test-all.sh closure-step-timeout` | ❌ W0 | ⬜ pending |
| 16-01-03 | 01 | 1 | ADV-03 | integration | `bash scripts/test-all.sh closure-conditional-retry` | ❌ W0 | ⬜ pending |
| 16-01-04 | 01 | 1 | ADV-04 | integration | `bash scripts/test-all.sh closure-batch-checkpoint` | ❌ W0 | ⬜ pending |
| 16-02-01 | 02 | 2 | ADV-01..04 | infra | `terraform plan` (4 new functions) | ❌ W0 | ⬜ pending |
| 16-02-02 | 02 | 2 | ADV-01..04 | build | `bash scripts/build-images.sh` (48 images) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `examples/closure-style/src/saga_compensation.rs` — ADV-01 handler
- [ ] `examples/closure-style/src/step_timeout.rs` — ADV-02 handler
- [ ] `examples/closure-style/src/conditional_retry.rs` — ADV-03 handler
- [ ] `examples/closure-style/src/batch_checkpoint.rs` — ADV-04 handler
- [ ] `examples/closure-style/Cargo.toml` — 4 new `[[bin]]` entries
- [ ] `infra/lambda.tf` — 4 new entries in `locals.handlers`
- [ ] `scripts/build-images.sh` — extend CRATE_BINS + update IMAGE_COUNT to 48
- [ ] `scripts/test-all.sh` — 4 new test functions + BINARY_TO_TEST entries + Phase 16 run_all_tests section

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Batch checkpoint produces fewer API calls | ADV-04 | Execution history may not distinguish batched vs individual calls | Compare `get-durable-execution-history` event counts between batch and non-batch invocations; if identical, verify via CloudWatch metrics |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
