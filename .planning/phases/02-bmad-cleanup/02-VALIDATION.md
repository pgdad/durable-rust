---
phase: 2
slug: bmad-cleanup
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-16
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Shell commands (file existence + grep checks — no code changes in this phase) |
| **Config file** | none |
| **Quick run command** | `test ! -d _bmad-output && test ! -d _bmad && echo "PASS"` |
| **Full suite command** | `test ! -d _bmad-output && test ! -d _bmad && test -z "$(find .claude/skills -name 'bmad-*' -maxdepth 1 2>/dev/null)" && ! grep -r "_bmad" . --include="*.md" --include="*.yaml" --include="*.json" --exclude-dir=.git 2>/dev/null && echo "ALL PASS"` |
| **Estimated runtime** | ~2 seconds |

---

## Sampling Rate

- **After every task commit:** Run quick command for that task's target
- **After every plan wave:** Run full suite command
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 2 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 2-01-01 | 01 | 1 | BMAD-01 | dir check | `test ! -d _bmad-output && echo "REMOVED"` | ✅ | ⬜ pending |
| 2-01-02 | 01 | 1 | BMAD-02 | dir check | `test ! -d _bmad && echo "REMOVED"` | ✅ | ⬜ pending |
| 2-01-03 | 01 | 1 | BMAD-03 | dir check | `test -z "$(find .claude/skills -name 'bmad-*' -maxdepth 1 2>/dev/null)" && echo "REMOVED"` | ✅ | ⬜ pending |
| 2-01-04 | 01 | 1 | BMAD-03 | grep check | `! grep -r "_bmad" . --include="*.md" --exclude-dir=.git 2>/dev/null && echo "CLEAN"` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. This phase removes files only — no test framework needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Rust source untouched | Safety | Verify no changes to crates/, tests/, examples/, docs/ | `git diff --name-only HEAD~4 HEAD -- crates/ tests/ examples/ docs/` should return empty |

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 2s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
