---
phase: 15
slug: async-operation-tests
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 15 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | bash + AWS CLI (shell integration tests) |
| **Config file** | scripts/test-all.sh + scripts/test-helpers.sh |
| **Quick run command** | `bash scripts/test-all.sh closure-invoke` |
| **Full suite command** | `bash scripts/test-all.sh` |
| **Estimated runtime** | ~3 min (12 async tests, waits ~10s each, callbacks ~15s each, invoke ~5s each) |

---

## Sampling Rate

- **After every task commit:** Run `bash scripts/test-all.sh closure-invoke` (quick sanity for sync invoke)
- **After every plan wave:** Run full Phase 15 tests via test-all.sh
- **Before `/gsd:verify-work`:** Full suite must show 12/12 async PASS
- **Max feedback latency:** ~15 seconds per individual test

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 15-01-01 | 01 | 1 | OPTEST-04 | integration | `bash -n scripts/test-helpers.sh` (handler modification) | ✅ | ⬜ pending |
| 15-01-02 | 01 | 1 | OPTEST-04,05,06 | integration | `bash -n scripts/test-all.sh` (test stubs replaced) | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements:
- `scripts/test-all.sh` — test runner with 12 Phase 15 stub functions (Phase 13)
- `scripts/test-helpers.sh` — invoke_async, wait_for_terminal_status, extract_callback_id, send_callback_success (Phase 13)
- 48 Lambda functions deployed (Phase 11/12/16)
- order-enrichment-lambda stub deployed (Phase 11)

*No Wave 0 needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Full async test suite against live AWS | OPTEST-04,05,06 | Requires ADFS credentials + deployed Lambda | `bash scripts/test-all.sh` with valid ADFS creds |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
