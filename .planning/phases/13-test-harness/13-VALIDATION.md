---
phase: 13
slug: test-harness
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 13 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | bash (test-all.sh) |
| **Config file** | scripts/test-all.sh |
| **Quick run command** | `bash -n scripts/test-all.sh && echo "Syntax OK"` |
| **Full suite command** | `bash scripts/test-all.sh --dry-run` (if available) |
| **Estimated runtime** | ~5 seconds (syntax), ~300 seconds (full run with Lambda invocations) |

---

## Sampling Rate

- **After every task commit:** Run `bash -n scripts/test-all.sh`
- **After every plan wave:** Run syntax check + verify script structure
- **Before `/gsd:verify-work`:** Full test run against live AWS
- **Max feedback latency:** 5 seconds (syntax check)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 13-01-01 | 01 | 1 | TEST-02,03 | automated | `bash -n scripts/test-helpers.sh` | ❌ W0 | ⬜ pending |
| 13-01-02 | 01 | 1 | TEST-01,04,05,06 | automated | `bash -n scripts/test-all.sh` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `scripts/test-helpers.sh` — polling + callback helpers
- [ ] `scripts/test-all.sh` — test runner framework

*This phase creates test infrastructure from scratch.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Polling waits for SUCCEEDED | TEST-02 | Requires live Lambda | Invoke a basic-steps function, poll until SUCCEEDED |
| Callback signal works | TEST-03 | Requires live Lambda + callback flow | Invoke callback handler, extract callback_id, send signal |
| Credential check gates tests | TEST-04 | Requires expired credentials | Expire ADFS session, run test-all.sh, verify early exit |
| Individual test filter | TEST-06 | Requires live Lambda | `scripts/test-all.sh closure-basic-steps` runs only that test |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
