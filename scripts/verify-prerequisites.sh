#!/usr/bin/env bash
# scripts/verify-prerequisites.sh
# Verifies all tools are installed with minimum versions and ADFS credentials
# are valid for us-east-2. Run this before starting any downstream phase.
#
# Exit code 0: all checks passed
# Exit code 1: one or more checks failed (error count printed)
set -euo pipefail

PROFILE="adfs"
REGION="us-east-2"
ACCOUNT="REDACTED_ACCOUNT_ID"
ERRORS=0

# ---------------------------------------------------------------------------
# Helper: run a command silently; print [OK] or [FAIL] with name
# ---------------------------------------------------------------------------
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

# ---------------------------------------------------------------------------
# Helper: display version string without aborting on missing tool
# ---------------------------------------------------------------------------
show_version() {
  local tool="$1"
  local cmd="$2"
  if command -v "$tool" &>/dev/null; then
    eval "$cmd" 2>&1 || true
  else
    echo "  [FAIL] $tool not found"
    ERRORS=$((ERRORS + 1))
  fi
}

# ---------------------------------------------------------------------------
# Helper: compare version numbers (returns 0 if actual >= minimum)
# Usage: version_ge "1.14.6" "1.14.0"
# ---------------------------------------------------------------------------
version_ge() {
  local actual="$1"
  local minimum="$2"
  printf '%s\n%s\n' "$minimum" "$actual" | sort -V -C
}

# ---------------------------------------------------------------------------
# Section 1: Tool Versions
# ---------------------------------------------------------------------------
echo "=== Tool Versions ==="

show_version "terraform" "terraform --version | head -1"
show_version "aws"       "aws --version"
show_version "docker"    "docker --version"
show_version "docker"    "docker buildx version | head -1"
show_version "jq"        "jq --version"
show_version "rustc"     "rustc --version"

# ---------------------------------------------------------------------------
# Section 2: Minimum Version Checks
# ---------------------------------------------------------------------------
echo ""
echo "=== Minimum Version Checks ==="

# Terraform >= 1.14.0
if command -v terraform &>/dev/null; then
  TF_RAW=$(terraform --version 2>/dev/null | head -1 || true)
  # Matches "Terraform v1.14.6" or "OpenTofu v..."
  TF_VER=$(echo "$TF_RAW" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || true)
  TF_MIN="1.14.0"
  if [[ -n "$TF_VER" ]] && version_ge "$TF_VER" "$TF_MIN"; then
    echo "  [OK] terraform $TF_VER >= $TF_MIN"
  else
    echo "  [FAIL] terraform $TF_VER < minimum $TF_MIN (or version undetected)"
    ERRORS=$((ERRORS + 1))
  fi
else
  echo "  [FAIL] terraform not found — minimum $TF_MIN required"
  ERRORS=$((ERRORS + 1))
fi

# AWS CLI v2
if command -v aws &>/dev/null; then
  AWS_RAW=$(aws --version 2>&1 || true)
  # "aws-cli/2.27.7 Python/..." — extract major version
  AWS_MAJOR=$(echo "$AWS_RAW" | grep -oE 'aws-cli/[0-9]+' | grep -oE '[0-9]+' || true)
  if [[ "$AWS_MAJOR" == "2" ]]; then
    echo "  [OK] aws-cli v2 detected"
  else
    echo "  [FAIL] aws-cli major version is '${AWS_MAJOR:-unknown}', require v2.x"
    ERRORS=$((ERRORS + 1))
  fi
else
  echo "  [FAIL] aws not found — v2.x required"
  ERRORS=$((ERRORS + 1))
fi

# Docker >= 20.0.0
if command -v docker &>/dev/null; then
  DOCKER_RAW=$(docker --version 2>/dev/null || true)
  DOCKER_VER=$(echo "$DOCKER_RAW" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || true)
  DOCKER_MIN="20.0.0"
  if [[ -n "$DOCKER_VER" ]] && version_ge "$DOCKER_VER" "$DOCKER_MIN"; then
    echo "  [OK] docker $DOCKER_VER >= $DOCKER_MIN"
  else
    echo "  [FAIL] docker $DOCKER_VER < minimum $DOCKER_MIN (or version undetected)"
    ERRORS=$((ERRORS + 1))
  fi
else
  echo "  [FAIL] docker not found — minimum 20.0.0 required"
  ERRORS=$((ERRORS + 1))
fi

# jq >= 1.7 (output format is "jq-1.7", hyphen-prefixed)
if command -v jq &>/dev/null; then
  JQ_RAW=$(jq --version 2>/dev/null || true)
  # Strip leading "jq-" prefix
  JQ_VER=$(echo "$JQ_RAW" | sed 's/^jq-//' || true)
  JQ_MIN="1.7"
  # Pad to 3-part version for sort -V comparison
  JQ_VER_PAD="${JQ_VER}.0"
  JQ_MIN_PAD="${JQ_MIN}.0"
  if [[ -n "$JQ_VER" ]] && version_ge "$JQ_VER_PAD" "$JQ_MIN_PAD"; then
    echo "  [OK] jq $JQ_VER >= $JQ_MIN"
  else
    echo "  [FAIL] jq $JQ_VER < minimum $JQ_MIN (or version undetected)"
    ERRORS=$((ERRORS + 1))
  fi
else
  echo "  [FAIL] jq not found — minimum 1.7 required"
  ERRORS=$((ERRORS + 1))
fi

# Rust >= 1.70.0
if command -v rustc &>/dev/null; then
  RUST_RAW=$(rustc --version 2>/dev/null || true)
  RUST_VER=$(echo "$RUST_RAW" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || true)
  RUST_MIN="1.70.0"
  if [[ -n "$RUST_VER" ]] && version_ge "$RUST_VER" "$RUST_MIN"; then
    echo "  [OK] rustc $RUST_VER >= $RUST_MIN"
  else
    echo "  [FAIL] rustc $RUST_VER < minimum $RUST_MIN (or version undetected)"
    ERRORS=$((ERRORS + 1))
  fi
else
  echo "  [FAIL] rustc not found — minimum 1.70.0 required"
  ERRORS=$((ERRORS + 1))
fi

# ---------------------------------------------------------------------------
# Section 3: Connectivity Checks
# ---------------------------------------------------------------------------
echo ""
echo "=== Connectivity Checks ==="

check "ADFS credentials valid" \
  "aws sts get-caller-identity --profile $PROFILE --region $REGION"

check "ECR accessible (us-east-2)" \
  "aws ecr describe-repositories --profile $PROFILE --region $REGION --max-items 1"

check "Docker daemon responding" \
  "docker info --format '{{.ServerVersion}}'"

# ---------------------------------------------------------------------------
# Section 4: Summary
# ---------------------------------------------------------------------------
echo ""
if [[ $ERRORS -gt 0 ]]; then
  echo "ERROR: $ERRORS check(s) failed. Fix before proceeding."
  exit 1
fi

echo "All prerequisites satisfied."
exit 0
