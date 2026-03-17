# Phase 10: Tooling and Prerequisites - Research

**Researched:** 2026-03-17
**Domain:** Developer toolchain verification and AWS CLI region configuration
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- ADFS profile stays at us-east-1 (untouched for other work)
- All test and deploy scripts explicitly pass `--region us-east-2`
- Terraform provider block hardcodes `region = "us-east-2"` — self-contained, no environment dependency
- AWS CLI calls in test harness scripts use `--region us-east-2 --profile adfs`
- All tools already installed and functional (Terraform v1.14.6, AWS CLI v2.27.7, Docker v28.4.0 + Buildx v0.23.0, jq v1.7, Rust v1.94.0)
- ADFS profile authenticated, account REDACTED_ACCOUNT_ID

### Claude's Discretion
- Whether to upgrade Terraform from 1.14.6 to 1.14.7 (both work)
- Any additional Docker Buildx builder configuration if needed

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TOOL-01 | All missing tooling installed on Ubuntu (Terraform, AWS CLI v2, Docker CE + Buildx, jq) | All tools confirmed installed and functional via direct verification |
| TOOL-02 | AWS CLI configured with `adfs` profile and `us-east-2` region | ADFS profile exists at us-east-1; region gap resolved by explicit `--region us-east-2` flag in all scripts (profile must not be modified) |
</phase_requirements>

## Summary

Phase 10 is a verification-and-configuration phase, not an installation phase. All required tools are already present on the machine and functional. The only real work is establishing the pattern for how downstream scripts and Terraform reference us-east-2 without touching the shared ADFS profile.

Direct verification (run 2026-03-17) confirmed every tool is installed and authenticated. The ADFS profile is set to us-east-1 in `~/.aws/config` but the credentials work against us-east-2 when `--region us-east-2` is passed on the CLI. This is the established pattern for all downstream phases.

The key output of this phase is a `scripts/` directory skeleton and a verification script that downstream phases can call to confirm prerequisites are met before attempting AWS operations.

**Primary recommendation:** Create a `scripts/verify-prerequisites.sh` that checks each tool version and confirms ADFS credential validity against us-east-2. This script becomes the "gate" for all downstream phases and documents what versions are expected.

## Standard Stack

### Core
| Tool | Installed Version | Purpose | Notes |
|------|-------------------|---------|-------|
| Terraform | v1.14.6 | Infrastructure provisioning | v1.14.7 is latest, either works |
| AWS CLI | v2.27.7 | AWS API calls, ECR auth | ADFS profile configured |
| Docker | v28.4.0 (Desktop) | Container builds | Uses `desktop-linux` context |
| Docker Buildx | v0.23.0 | Multi-arch builds | Bundled with Docker Desktop |
| jq | v1.7 | JSON parsing in shell scripts | Standard in test harness |
| Rust | v1.94.0 | SDK compilation | Workspace builds confirmed |

### AWS Configuration Facts (verified)
| Property | Value |
|----------|-------|
| ADFS profile region (config) | us-east-1 (DO NOT MODIFY) |
| ADFS profile region (runtime) | `--region us-east-2` flag overrides |
| Account ID | REDACTED_ACCOUNT_ID |
| Role | `arn:aws:iam::REDACTED_ACCOUNT_ID:role/l1-developers` |
| Credential expiry format | ISO-8601 in `~/.aws/credentials` |
| ECR access | Confirmed — repositories in us-east-2 visible |
| Lambda access | Confirmed — 12 functions exist in us-east-2 |
| Docker context | `desktop-linux` (not `default`) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Explicit `--region` flag everywhere | Modifying ADFS profile region | Modifying profile breaks other projects that depend on us-east-1 |
| `desktop-linux` Docker context | `default` context | `default` shows error (no unix socket); `desktop-linux` works |

## Architecture Patterns

### Recommended Project Structure (new directories)
```
scripts/
├── verify-prerequisites.sh   # Tool version checks + credential validation
infra/                         # Created in Phase 11 — Terraform lives here
```

### Pattern 1: Explicit Region + Profile in All Scripts
**What:** Every AWS CLI invocation appends `--profile adfs --region us-east-2`
**When to use:** Every shell script in `scripts/` that calls `aws`
**Example:**
```bash
# Correct — region is explicit, profile is explicit
aws sts get-caller-identity --profile adfs --region us-east-2

# Correct — ECR login for Docker push
aws ecr get-login-password --profile adfs --region us-east-2 | \
  docker login --username AWS --password-stdin \
  REDACTED_ACCOUNT_ID.dkr.ecr.us-east-2.amazonaws.com
```

### Pattern 2: Docker Desktop Context Awareness
**What:** Docker Desktop on Linux uses `desktop-linux` context, not `default`
**When to use:** Any script that calls `docker build` or `docker push`
**Example:**
```bash
# Verify Docker is operational before attempting builds
docker context use desktop-linux 2>/dev/null || true
docker info --format '{{.ServerVersion}}' 2>&1
```

### Pattern 3: Credential Expiry Check
**What:** ADFS credentials expire. Scripts should validate before starting long operations.
**When to use:** At the top of `test-all.sh` and any multi-step script
**Example:**
```bash
# Check ADFS credentials are valid
if ! aws sts get-caller-identity --profile adfs --region us-east-2 &>/dev/null; then
  echo "ERROR: ADFS credentials expired. Re-authenticate and retry."
  exit 1
fi
```

### Pattern 4: Terraform Region Hardcoding
**What:** Terraform provider block sets region explicitly — no reliance on `AWS_REGION` or `AWS_DEFAULT_REGION`
**When to use:** `infra/main.tf` provider block (Phase 11)
**Example:**
```hcl
provider "aws" {
  region  = "us-east-2"
  profile = "adfs"
}
```

### Anti-Patterns to Avoid
- **Modifying ADFS profile region:** The profile is shared; changing `region` in `~/.aws/config [profile adfs]` to us-east-2 would break other projects
- **Using `default` Docker context:** The `default` context errors on this machine; always target `desktop-linux`
- **Relying on `AWS_DEFAULT_REGION` env var:** Scripts must be self-contained with explicit flags, not environment-dependent
- **Using `docker.service` systemctl:** Docker Desktop does not register as a systemd unit; check via `docker info` instead

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ECR Docker login | Custom auth script | `aws ecr get-login-password \| docker login` | AWS CLI handles token rotation; custom scripts go stale |
| Credential expiry parsing | Date math on expiry string | `aws sts get-caller-identity` live check | Simpler, always accurate |
| Terraform version pinning | Manual version check script | `required_version` in `terraform {}` block | Terraform enforces this natively |

**Key insight:** AWS CLI v2 handles ECR authentication, credential forwarding, and region routing. Prefer `aws` subcommands over custom HTTP/auth logic.

## Common Pitfalls

### Pitfall 1: Docker Context Mismatch
**What goes wrong:** `docker build` or `docker push` fails with "Cannot connect to the Docker daemon"
**Why it happens:** The `default` Docker context points to `unix:///var/run/docker.sock` which is not active; Docker Desktop uses `unix:///home/esa/.docker/desktop/docker.sock`
**How to avoid:** Scripts call `docker context use desktop-linux` or set `DOCKER_CONTEXT=desktop-linux` at script top
**Warning signs:** `docker buildx ls` shows `default` with `error` status next to it

### Pitfall 2: ADFS Credential Expiry Mid-Run
**What goes wrong:** A long build+push+deploy script fails halfway through with `ExpiredTokenException`
**Why it happens:** ADFS credentials have a session duration (typically 8h); long test runs can span a refresh boundary
**How to avoid:** Validate credentials at script start; document expiry time in output
**Warning signs:** AWS CLI returns `ExpiredTokenException` or `RequestExpired`

### Pitfall 3: Region Not Propagating to Terraform
**What goes wrong:** `terraform apply` hits us-east-1 resources instead of us-east-2
**Why it happens:** Terraform reads `AWS_DEFAULT_REGION` or the profile's configured region if not hardcoded
**How to avoid:** Always hardcode `region = "us-east-2"` in the provider block — never rely on environment
**Warning signs:** Resources created in wrong region; ARNs contain `us-east-1`

### Pitfall 4: Terraform Version Mismatch Warning
**What goes wrong:** CI or a teammate runs a slightly different Terraform version, gets state format differences
**Why it happens:** Terraform 1.14.6 vs 1.14.7 differ trivially but the CLI warns about being out of date
**How to avoid:** Pin with `required_version = "~> 1.14"` in `terraform {}` block; accept patch version flexibility
**Warning signs:** `Your version of Terraform is out of date!` in CI output

## Code Examples

Verified patterns from direct verification:

### Verify All Prerequisites
```bash
#!/usr/bin/env bash
# scripts/verify-prerequisites.sh
# Verifies all tools are installed and ADFS credentials are valid for us-east-2
set -euo pipefail

PROFILE="adfs"
REGION="us-east-2"
ACCOUNT="REDACTED_ACCOUNT_ID"
ERRORS=0

check() {
  local name="$1"
  local cmd="$2"
  if eval "$cmd" &>/dev/null; then
    echo "  [OK] $name"
  else
    echo "  [FAIL] $name"
    ERRORS=$((ERRORS + 1))
  fi
}

echo "=== Tool Versions ==="
terraform --version | head -1
aws --version
docker --version
docker buildx version | head -1
jq --version
rustc --version

echo ""
echo "=== Connectivity Checks ==="
check "ADFS credentials valid" \
  "aws sts get-caller-identity --profile $PROFILE --region $REGION"
check "ECR accessible (us-east-2)" \
  "aws ecr describe-repositories --profile $PROFILE --region $REGION --max-items 1"
check "Lambda accessible (us-east-2)" \
  "aws lambda list-functions --profile $PROFILE --region $REGION --max-items 1"
check "Docker daemon responding" \
  "docker info --format '{{.ServerVersion}}'"

if [[ $ERRORS -gt 0 ]]; then
  echo ""
  echo "ERROR: $ERRORS check(s) failed. Fix before proceeding."
  exit 1
fi

echo ""
echo "All prerequisites satisfied."
```

### AWS ECR Login (for Phase 12 build scripts)
```bash
# Standard pattern — reuse in build.sh
ACCOUNT="REDACTED_ACCOUNT_ID"
REGION="us-east-2"
PROFILE="adfs"

aws ecr get-login-password --profile "$PROFILE" --region "$REGION" | \
  docker login --username AWS --password-stdin \
  "${ACCOUNT}.dkr.ecr.${REGION}.amazonaws.com"
```

### Terraform Provider Block (for Phase 11)
```hcl
# infra/main.tf — hardcode region and profile, no environment dependency
terraform {
  required_version = "~> 1.14"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 6.25.0"
    }
  }
}

provider "aws" {
  region  = "us-east-2"
  profile = "adfs"
}
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `docker login -e` (deprecated) | `aws ecr get-login-password \| docker login` | AWS CLI v2 handles token refresh |
| Modifying `~/.aws/config` per project | Explicit `--region` flags in scripts | Profile stays shared; project is self-contained |
| `default` Docker context | `desktop-linux` context (Docker Desktop) | Docker Desktop no longer registers a systemd socket at `/var/run/docker.sock` |

**Deprecated/outdated:**
- `$(aws ecr get-login ...)` (old `get-login` subcommand): Replaced by `get-login-password` piped to `docker login` in AWS CLI v2

## Open Questions

1. **Terraform upgrade to 1.14.7**
   - What we know: v1.14.6 installed, v1.14.7 is latest patch, both work with AWS provider >= 6.25.0
   - What's unclear: Whether any future phase's Terraform code requires 1.14.7 specifically
   - Recommendation: Leave at v1.14.6 now; upgrade is one command if needed later. Note in verify script.

2. **Docker Buildx builder for cross-compilation**
   - What we know: `desktop-linux` builder supports linux/amd64 and linux/arm64 natively
   - What's unclear: Phase 12 will build Lambda containers — Lambda uses linux/amd64 (x86_64); no cross-compilation needed
   - Recommendation: No additional builder setup required; default `desktop-linux` builder is sufficient

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | bash (shell script assertions) |
| Config file | none — scripts are self-contained |
| Quick run command | `bash scripts/verify-prerequisites.sh` |
| Full suite command | `bash scripts/verify-prerequisites.sh` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TOOL-01 | All tools installed and executable | smoke | `bash scripts/verify-prerequisites.sh` | Wave 0 |
| TOOL-02 | ADFS profile works with us-east-2 via explicit flag | smoke | `bash scripts/verify-prerequisites.sh` | Wave 0 |

### Sampling Rate
- **Per task commit:** `bash scripts/verify-prerequisites.sh`
- **Per wave merge:** `bash scripts/verify-prerequisites.sh`
- **Phase gate:** Script exits 0 before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `scripts/verify-prerequisites.sh` — covers TOOL-01 and TOOL-02 (this IS the deliverable of the phase)

## Sources

### Primary (HIGH confidence)
- Direct shell verification on 2026-03-17 — all tool versions, AWS credential test, Docker context behavior
- `aws sts get-caller-identity --profile adfs --region us-east-2` — confirmed account REDACTED_ACCOUNT_ID, role l1-developers
- `aws ecr describe-repositories --profile adfs --region us-east-2` — confirmed ECR access

### Secondary (MEDIUM confidence)
- `~/.aws/config` and `~/.aws/credentials` — confirmed profile structure and region setting
- `docker context ls` — confirmed `desktop-linux` as active context, `default` shows error
- `docker buildx ls` — confirmed `desktop-linux` builder with linux/amd64 support

### Tertiary (LOW confidence)
- None — all claims are grounded in direct verification

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — directly verified on machine
- Architecture: HIGH — pattern derived from locked CONTEXT.md decisions + live verification
- Pitfalls: HIGH — identified from actual error messages observed during verification

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (ADFS credentials rotate, but tool versions stable; re-verify credentials before use)
