---
phase: 9
slug: documentation-overhaul
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 9 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test --doc (rustdoc tests) + manual review |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test --doc -p durable-lambda-core` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --doc -p durable-lambda-core`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 9-01-01 | 01 | 1 | DOCS-01 | manual | `grep -c "Determinism Rules" README.md` | ✅ exists | ⬜ pending |
| 9-01-02 | 01 | 1 | DOCS-02 | manual | `grep -c "Ok(Ok" README.md` | ✅ exists | ⬜ pending |
| 9-01-03 | 01 | 1 | DOCS-03 | manual | `grep -c "Troubleshooting" README.md` | ✅ exists | ⬜ pending |
| 9-01-04 | 01 | 1 | DOCS-04 | manual | `grep -c "project-context" README.md` | ✅ exists | ⬜ pending |
| 9-02-01 | 02 | 1 | DOCS-05 | manual | `grep -c "determinism" docs/migration-guide.md` | ✅ exists | ⬜ pending |
| 9-02-02 | 02 | 1 | DOCS-06 | doctest | `cargo test --doc -p durable-lambda-core -- BatchResult` | ✅ exists | ⬜ pending |
| 9-02-03 | 02 | 1 | DOCS-07 | manual | `grep -c "BranchFn" README.md` | ✅ exists | ⬜ pending |
| 9-02-04 | 02 | 1 | DOCS-08 | manual | `grep -c "DurableContextOps" CLAUDE.md` | ✅ exists | ⬜ pending |
| 9-02-05 | 02 | 1 | DOCS-09 | manual | `grep -c "callback_id" README.md` | ✅ exists | ⬜ pending |
| 9-02-06 | 02 | 1 | DOCS-10 | build | `cargo metadata --format-version 1 2>&1 \| head -1` | ✅ exists | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing files cover all phase requirements. No new test infrastructure needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| README sections read clearly | DOCS-01..04 | Content quality | Read each section for accuracy and clarity |
| Migration guide anti-patterns helpful | DOCS-05 | Content quality | Verify examples match actual codebase patterns |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
