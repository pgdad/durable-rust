# Phase 10: Tooling and Prerequisites - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Install and configure all required tools on the developer machine so that downstream phases (Terraform, Docker, AWS CLI) can execute without tool-related blockers. Most tools are already installed — this phase verifies versions and fixes configuration gaps.

</domain>

<decisions>
## Implementation Decisions

### Region configuration
- ADFS profile stays at us-east-1 (untouched for other work)
- All test and deploy scripts explicitly pass `--region us-east-2`
- Terraform provider block hardcodes `region = "us-east-2"` — self-contained, no environment dependency
- AWS CLI calls in test harness scripts use `--region us-east-2 --profile adfs`

### Terraform version
- v1.14.6 is installed, v1.14.7 is latest — either works with AWS provider >= 6.25.0
- No forced upgrade needed; if user wants latest, update is trivial (`sudo apt-get update && sudo apt-get install terraform`)

### Tool verification
- All tools already installed and functional:
  - Terraform v1.14.6
  - AWS CLI v2.27.7
  - Docker v28.4.0 + Buildx v0.23.0
  - jq v1.7
  - Rust v1.94.0
- ADFS profile authenticated, account REDACTED_ACCOUNT_ID

### Claude's Discretion
- Whether to upgrade Terraform from 1.14.6 to 1.14.7 (both work)
- Any additional Docker Buildx builder configuration if needed

</decisions>

<specifics>
## Specific Ideas

- ADFS profile region must NOT be modified — it's shared with other projects using us-east-1
- All AWS region references in this project should be explicit (`--region us-east-2` or `region = "us-east-2"`)

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `examples/Dockerfile`: Multi-stage build for Lambda containers — already targets x86_64
- `.github/workflows/ci.yml`: Existing CI runs fmt/clippy/build/test — no AWS integration

### Established Patterns
- Workspace Cargo.toml manages all dependency versions centrally
- aws-sdk-lambda 1.118.0 and aws-config 1.8.15 already in workspace deps

### Integration Points
- Terraform will live in `infra/` directory (new)
- Build/test scripts will live in `scripts/` directory (new)
- Both will use `--profile adfs --region us-east-2` consistently

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 10-tooling-and-prerequisites*
*Context gathered: 2026-03-17*
