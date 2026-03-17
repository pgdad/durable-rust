---
phase: 10
slug: tooling-and-prerequisites
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | bash (verify-prerequisites.sh) |
| **Config file** | scripts/verify-prerequisites.sh |
| **Quick run command** | `bash scripts/verify-prerequisites.sh` |
| **Full suite command** | `bash scripts/verify-prerequisites.sh` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `bash scripts/verify-prerequisites.sh`
- **After every plan wave:** Run `bash scripts/verify-prerequisites.sh`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 10-01-01 | 01 | 1 | TOOL-01 | manual | `terraform version && aws --version && docker buildx version && jq --version` | ✅ | ⬜ pending |
| 10-01-02 | 01 | 1 | TOOL-02 | manual | `aws sts get-caller-identity --profile adfs --region us-east-2` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `scripts/verify-prerequisites.sh` — verification script for all tool checks

*Existing infrastructure covers most phase requirements — tools already installed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Tool versions correct | TOOL-01 | One-time setup verification | Run each tool version command, compare against minimum |
| ADFS auth works with us-east-2 | TOOL-02 | Requires active ADFS session | `aws sts get-caller-identity --profile adfs --region us-east-2` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
