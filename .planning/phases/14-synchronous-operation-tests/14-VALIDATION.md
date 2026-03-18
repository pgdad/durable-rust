---
phase: 14
slug: synchronous-operation-tests
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 14 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | bash + AWS CLI (shell integration tests) |
| **Config file** | scripts/test-all.sh + scripts/test-helpers.sh |
| **Quick run command** | `bash scripts/test-all.sh closure-basic-steps` |
| **Full suite command** | `bash scripts/test-all.sh` |
| **Estimated runtime** | ~5 min (32 sync tests, combined_workflow ~35s each) |

---

## Sampling Rate

- **After every task commit:** Run `bash scripts/test-all.sh closure-basic-steps` (quick sanity)
- **After every plan wave:** Run `bash scripts/test-all.sh` (full suite)
- **Before `/gsd:verify-work`:** Full suite must show 32/32 PASS
- **Max feedback latency:** ~10 seconds per individual test

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 14-01-01 | 01 | 1 | OPTEST-01 | integration | `bash scripts/test-all.sh closure-basic-steps` | ✅ | ⬜ pending |
| 14-01-02 | 01 | 1 | OPTEST-02 | integration | `bash scripts/test-all.sh closure-step-retries` | ✅ | ⬜ pending |
| 14-01-03 | 01 | 1 | OPTEST-03 | integration | `bash scripts/test-all.sh closure-typed-errors` | ✅ | ⬜ pending |
| 14-01-04 | 01 | 1 | OPTEST-07 | integration | `bash scripts/test-all.sh closure-parallel` | ✅ | ⬜ pending |
| 14-01-05 | 01 | 1 | OPTEST-08 | integration | `bash scripts/test-all.sh closure-map` | ✅ | ⬜ pending |
| 14-01-06 | 01 | 1 | OPTEST-09 | integration | `bash scripts/test-all.sh closure-child-contexts` | ✅ | ⬜ pending |
| 14-01-07 | 01 | 1 | OPTEST-10 | integration | `bash scripts/test-all.sh closure-replay-safe-logging` | ✅ | ⬜ pending |
| 14-01-08 | 01 | 1 | OPTEST-11 | integration | `bash scripts/test-all.sh closure-combined-workflow` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements:
- `scripts/test-all.sh` — test runner with 32 stub functions (Phase 13)
- `scripts/test-helpers.sh` — invoke_sync, get_alias_arn, etc. (Phase 13)
- 48 Lambda functions deployed (Phase 11/12/16)

*No Wave 0 needed.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
