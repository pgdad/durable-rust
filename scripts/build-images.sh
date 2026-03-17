#!/usr/bin/env bash
# scripts/build-images.sh
# Builds all 44 Lambda container images (4 crates x 11 binaries) and pushes
# them to ECR. The 4 crates are built concurrently as background jobs.
#
# Usage:
#   bash scripts/build-images.sh
#
# Prerequisites: ADFS credentials valid, Docker daemon running, ECR deployed.
# First run: 30-60 minutes (cold cargo-chef dependency layer compilation).
# Subsequent source-only runs: 5-10 minutes (cached dependency layer).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TF_DIR="$REPO_ROOT/infra"
PROFILE="adfs"
REGION="us-east-2"

echo "=== Building and pushing all 44 Lambda images to ECR ==="
echo "Repo root: $REPO_ROOT"
echo ""

# ---------------------------------------------------------------------------
# Prerequisites gate
# ---------------------------------------------------------------------------
"$SCRIPT_DIR/verify-prerequisites.sh" || exit 1

# ---------------------------------------------------------------------------
# Read ECR URL from Terraform outputs (never hardcoded)
# ---------------------------------------------------------------------------
echo ""
echo "=== Reading ECR URL from Terraform ==="
ECR_URL=$(terraform -chdir="$TF_DIR" output -raw ecr_repo_url)
echo "ECR URL: $ECR_URL"

# ---------------------------------------------------------------------------
# ECR login (once, before all builds — token valid for 12 hours)
# ---------------------------------------------------------------------------
echo ""
echo "=== Logging in to ECR ==="
aws ecr get-login-password --profile "$PROFILE" --region "$REGION" \
  | docker login --username AWS --password-stdin "$ECR_URL"
echo "ECR login successful"

# ---------------------------------------------------------------------------
# Pre-pull base images (prevents layer-store contention when 4 parallel
# builds simultaneously try to pull the same base layers — Research Pitfall 3)
# ---------------------------------------------------------------------------
echo ""
echo "=== Pre-pulling base images ==="
docker pull lukemathwalker/cargo-chef:latest-rust-1
docker pull public.ecr.aws/lambda/provided:al2023
echo "Base images ready"

# ---------------------------------------------------------------------------
# Crate-to-binary mapping (all 44 binary names hardcoded to match lambda.tf)
# ---------------------------------------------------------------------------
declare -A CRATE_BINS

CRATE_BINS["closure-style-example"]="closure-basic-steps closure-step-retries closure-typed-errors closure-waits closure-callbacks closure-invoke closure-parallel closure-map closure-child-contexts closure-replay-safe-logging closure-combined-workflow"

CRATE_BINS["macro-style-example"]="macro-basic-steps macro-step-retries macro-typed-errors macro-waits macro-callbacks macro-invoke macro-parallel macro-map macro-child-contexts macro-replay-safe-logging macro-combined-workflow"

CRATE_BINS["trait-style-example"]="trait-basic-steps trait-step-retries trait-typed-errors trait-waits trait-callbacks trait-invoke trait-parallel trait-map trait-child-contexts trait-replay-safe-logging trait-combined-workflow"

CRATE_BINS["builder-style-example"]="builder-basic-steps builder-step-retries builder-typed-errors builder-waits builder-callbacks builder-invoke builder-parallel builder-map builder-child-contexts builder-replay-safe-logging builder-combined-workflow"

# ---------------------------------------------------------------------------
# Build and push function: builds all 11 images for one crate sequentially,
# then pushes each. Runs inside a background job (one per crate).
# ---------------------------------------------------------------------------
build_and_push_crate() {
  local package="$1"
  local bins="$2"
  local count=0
  local total=11

  echo "[${package}] Starting — building ${total} images..."

  for bin_name in $bins; do
    count=$((count + 1))
    echo "[${package}] Building ${bin_name} (${count}/${total})..."
    docker build \
      -f "$REPO_ROOT/examples/Dockerfile" \
      --build-arg "PACKAGE=${package}" \
      --build-arg "BINARY_NAME=${bin_name}" \
      -t "${ECR_URL}:${bin_name}" \
      "$REPO_ROOT"
    docker push "${ECR_URL}:${bin_name}"
    echo "[${package}] Pushed ${bin_name} (${count}/${total})"
  done

  echo "[${package}] Complete -- 11 images pushed"
}

# Export variables and function so subshells (background jobs) can access them
export -f build_and_push_crate
export ECR_URL REPO_ROOT

# ---------------------------------------------------------------------------
# Parallel execution: 4 background jobs, one per crate
# PIDs collected individually so failures are detected per-job (Pitfall 5)
# ---------------------------------------------------------------------------
echo ""
echo "=== Starting parallel crate builds ==="
echo "(4 jobs running concurrently — log output will be interleaved)"
echo ""

PIDS=()
for package in "${!CRATE_BINS[@]}"; do
  build_and_push_crate "$package" "${CRATE_BINS[$package]}" &
  PIDS+=($!)
done

# Wait for all 4 jobs and count failures
FAILED=0
for pid in "${PIDS[@]}"; do
  wait "$pid" || FAILED=$((FAILED + 1))
done

if [[ $FAILED -gt 0 ]]; then
  echo ""
  echo "ERROR: $FAILED crate build job(s) failed. Check log output above."
  exit 1
fi

# ---------------------------------------------------------------------------
# Final summary and ECR verification
# ---------------------------------------------------------------------------
echo ""
echo "=== All 44 images pushed to ${ECR_URL} ==="
echo ""
echo "=== Verifying ECR image count ==="
IMAGE_COUNT=$(aws ecr list-images \
  --profile "$PROFILE" \
  --region "$REGION" \
  --repository-name "$(basename "$ECR_URL")" \
  --query 'imageIds[*].imageTag' \
  --output text | tr '\t' '\n' | grep -v '^$' | sort -u | wc -l)
echo "ECR image count: $IMAGE_COUNT (expected: 44)"
echo ""

if [[ "$IMAGE_COUNT" -ne 44 ]]; then
  echo "WARNING: Expected 44 images in ECR but found ${IMAGE_COUNT}."
  echo "Some pushes may have failed silently. Check ECR console."
  exit 1
fi

echo "Build pipeline complete. All 44 Lambda images are ready for deployment."
echo "Next step: run scripts/deploy-lambdas.sh (Phase 11 Plan 03)"
