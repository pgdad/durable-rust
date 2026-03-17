---
phase: 12
slug: docker-build-pipeline
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 12 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | bash + docker + aws ecr |
| **Config file** | scripts/build-images.sh |
| **Quick run command** | `docker buildx version && terraform -chdir=infra output -raw ecr_repo_url` |
| **Full suite command** | `bash scripts/build-images.sh` |
| **Estimated runtime** | ~600 seconds (first build), ~120 seconds (cache hit) |

---

## Sampling Rate

- **After every task commit:** Run `docker buildx version` (fast check)
- **After every plan wave:** Run `bash scripts/build-images.sh --dry-run` (if available) or check ECR images
- **Before `/gsd:verify-work`:** Full build + push, verify all 44 tags in ECR
- **Max feedback latency:** 120 seconds (cached build)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 12-01-01 | 01 | 1 | BUILD-01 | automated | `docker build -f examples/Dockerfile --target chef-planner .` | ❌ W0 | ⬜ pending |
| 12-01-02 | 01 | 1 | BUILD-02,03,04 | manual | `bash scripts/build-images.sh` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `examples/Dockerfile` — modified with cargo-chef stages + BINARY_NAME arg
- [ ] `scripts/build-images.sh` — build + push script

*This phase creates build artifacts from scratch.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| All 44 images in ECR | BUILD-02,03 | Requires Docker build + ECR push | `aws ecr list-images --repository-name dr-examples-c351 --profile adfs --region us-east-2 \| jq '.imageIds \| length'` |
| Cached rebuild < 10 min | BUILD-01 | Requires timed re-run | `time bash scripts/build-images.sh` (second run) |
| 4 crates build concurrently | BUILD-04 | Visual from script output | Check for parallel job PIDs in output |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
