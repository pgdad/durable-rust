---
phase: 11
slug: infrastructure
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 11 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | terraform + bash |
| **Config file** | infra/*.tf |
| **Quick run command** | `cd infra && terraform plan` |
| **Full suite command** | `cd infra && terraform apply -auto-approve -parallelism=5 && terraform plan -detailed-exitcode` |
| **Estimated runtime** | ~120 seconds (plan), ~300 seconds (apply) |

---

## Sampling Rate

- **After every task commit:** Run `cd infra && terraform validate`
- **After every plan wave:** Run `cd infra && terraform plan`
- **Before `/gsd:verify-work`:** Full apply + plan -detailed-exitcode (exit 0 = no changes)
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 11-01-01 | 01 | 1 | INFRA-07 | automated | `cd infra && terraform init && terraform validate` | ❌ W0 | ⬜ pending |
| 11-01-02 | 01 | 1 | INFRA-02 | automated | `cd infra && terraform validate` | ❌ W0 | ⬜ pending |
| 11-01-03 | 01 | 1 | INFRA-01 | automated | `cd infra && terraform validate` | ❌ W0 | ⬜ pending |
| 11-02-01 | 02 | 2 | INFRA-03,04,06 | manual | `cd infra && terraform apply -auto-approve -parallelism=5` | ❌ W0 | ⬜ pending |
| 11-02-02 | 02 | 2 | INFRA-05 | manual | `aws lambda invoke --function-name dr-order-enrichment-lambda-{suffix}:live` | ❌ W0 | ⬜ pending |
| 11-02-03 | 02 | 2 | INFRA-08 | manual | `cd infra && terraform destroy -auto-approve` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `infra/` directory with Terraform files — created by Phase 11 plans

*This phase creates infrastructure files from scratch — no pre-existing test infrastructure.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Lambda functions have durable_config | INFRA-03 | Requires terraform apply + AWS API check | `aws lambda get-function-configuration --function-name dr-closure-basic-steps-{suffix} --profile adfs --region us-east-2 \| jq .DurableConfig` |
| Lambda aliases exist and are qualified | INFRA-04 | Requires terraform apply + AWS API check | `aws lambda get-alias --function-name dr-closure-basic-steps-{suffix} --name live --profile adfs --region us-east-2` |
| Tags visible in AWS | INFRA-06 | Requires terraform apply + AWS console/API | `aws lambda list-tags --resource {function-arn} --profile adfs --region us-east-2` |
| terraform destroy is clean | INFRA-08 | Destructive test | `cd infra && terraform destroy -auto-approve && terraform plan` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
