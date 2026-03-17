# Phase 12: Docker Build Pipeline - Research

**Researched:** 2026-03-17
**Domain:** Docker multi-stage builds with cargo-chef, ECR push automation, parallel Bash scripting
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

None — all implementation decisions deferred to Claude's discretion.

### Claude's Discretion

- Build strategy: how to build 44 images efficiently (per-crate vs per-binary, Docker layer reuse)
- cargo-chef Dockerfile integration (recipe.json caching for dependency layer)
- Build script interface: flags, progress output, error handling, selective rebuild
- How to handle PACKAGE vs BINARY_NAME args in Dockerfile (research identified gap)
- Parallel build implementation (e.g., 4 crates × 11 binaries, background jobs)
- ECR login handling in build script (aws ecr get-login-password)
- Image tagging convention when pushing to ECR

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BUILD-01 | Dockerfile updated with cargo-chef for fast dependency-layer caching | cargo-chef 0.1.77 three-stage pattern with planner/builder/runtime stages; existing Dockerfile needs BINARY_NAME ARG + chef stages |
| BUILD-02 | Build script builds all 44 Docker images from the 4 example crates | Per-crate build strategy: 1 cargo build per crate, 11 docker build+tag per crate — verified from Cargo.toml inspection |
| BUILD-03 | Build script pushes all 44 images to ECR with per-binary tags | ECR URL from terraform output; tag = binary name (e.g., `closure-basic-steps`); aws ecr get-login-password pattern established in existing scripts |
| BUILD-04 | Build supports parallel execution (4 crates built concurrently) | Bash background jobs (`&` + `wait`) or `xargs -P 4`; per-crate parallelism is the natural boundary |
</phase_requirements>

---

## Summary

Phase 12 converts the 4 example crates (44 binaries total) into container images and pushes them to the ECR repository (`dr-examples-c351`) that Phase 11 already provisioned. The two deliverables are: (1) an updated `examples/Dockerfile` that adds cargo-chef layer caching, and (2) a new `scripts/build-images.sh` that drives the entire build-and-push workflow.

The build strategy is **per-crate**: run `cargo build --release -p <crate>` once per crate (4 times total), then build 11 Docker images per crate by varying only the `BINARY_NAME` ARG. This means the expensive Rust compilation happens 4 times (not 44), and Docker layer caching via cargo-chef means subsequent builds after a source change recompile only changed crates. The existing `examples/Dockerfile` needs exactly one structural change: replacing the single `ARG PACKAGE`/`COPY` pattern with a three-stage cargo-chef pipeline plus a separate `ARG BINARY_NAME` for the final copy.

The build script follows the pattern established by `scripts/deploy-ecr.sh`: gate on `verify-prerequisites.sh`, read ECR URL from terraform output, perform ECR login once, then build/push. Parallel execution targets the crate level — 4 background jobs, one per crate, each building and pushing its 11 images sequentially within the job.

**Primary recommendation:** Three-stage cargo-chef Dockerfile + per-crate parallel build script; ECR login once before all builds.

---

## Standard Stack

### Core

| Library / Tool | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| cargo-chef | 0.1.77 (latest) | Rust dependency layer caching in Docker | Only tool that correctly isolates Cargo.lock dependencies as a cacheable layer; prevents 60-min full workspace recompile on source changes |
| `lukemathwalker/cargo-chef:latest-rust-1` | latest-rust-1 tag | Base image combining Rust toolchain + cargo-chef | Eliminates manual installation step; same Rust version across all stages (required for cache validity) |
| `public.ecr.aws/lambda/provided:al2023` | al2023 | Lambda runtime base image | AWS-maintained, glibc 2.34, correct for x86_64 Lambda functions |
| Docker CE + Buildx | 20.0.0+ | Multi-stage image builds | Already verified present in verify-prerequisites.sh |
| AWS CLI v2 | 2.27+ | `aws ecr get-login-password`, ECR push | Already verified; v1 does not support `get-login-password` |
| Bash 5.x | system | Build orchestration script | Zero additional dependencies; consistent with existing scripts |

### Supporting

| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `docker buildx` | included with Docker CE 20+ | Multi-platform builds | Not needed for this phase (x86_64 only), but available if arm64 builds added later |
| `terraform output` | 1.14.7 | Read ECR URL after infra deploy | ECR URL must not be hardcoded — read from `terraform -chdir=infra output -raw ecr_repo_url` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| per-crate Docker build | per-binary Docker build (44 `docker build` invocations) | Per-binary build ignores the fact that cargo-chef dependency layer is per-workspace, not per-binary; still does 44 separate dependency compilations |
| Background jobs (`&` + `wait`) for parallelism | `xargs -P 4` | xargs is harder to read with multi-line per-job logic; background jobs with `wait` are idiomatic for this use case |
| `cargo chef cook --release -p <crate>` | Full workspace cook | Per-crate cook narrows the cached layer to only the crate's dependencies; slightly less caching efficiency but simpler reasoning |

**Installation (cargo-chef is used only inside Docker, not installed on host):**

```bash
# No host installation needed — cargo-chef runs inside the Docker build stages
# The lukemathwalker/cargo-chef:latest-rust-1 base image provides it
```

---

## Architecture Patterns

### Recommended Project Structure

```
examples/
└── Dockerfile          # MODIFIED: add cargo-chef stages + ARG BINARY_NAME

scripts/
├── verify-prerequisites.sh   # EXISTING: unchanged
├── deploy-ecr.sh             # EXISTING: unchanged
└── build-images.sh           # NEW: build + push all 44 images
```

### Pattern 1: Three-Stage cargo-chef Dockerfile

**What:** Splits the build into planner (generates recipe.json), builder (cooks dependencies then compiles), runtime (copies binary). The dependency layer is cached separately from source code.

**When to use:** Any multi-binary Rust workspace where source changes are more frequent than dependency changes.

**The key insight for this project:** `PACKAGE` identifies which crate to build (affects the cargo chef cook and cargo build commands). `BINARY_NAME` identifies which binary within that crate to copy to the Lambda runtime stage. These are two separate ARGs.

**Example:**
```dockerfile
# Source: cargo-chef README (https://github.com/LukeMathWalker/cargo-chef) + existing Dockerfile

# --------------------------------------------------------------------------
# Stage 0: chef base — Rust toolchain + cargo-chef installed
# --------------------------------------------------------------------------
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /usr/src/app

# --------------------------------------------------------------------------
# Stage 1: Planner — computes recipe.json from workspace Cargo files
# --------------------------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --------------------------------------------------------------------------
# Stage 2: Builder — cooks dependencies (cached layer), then builds crate
# --------------------------------------------------------------------------
FROM chef AS builder
ARG PACKAGE=closure-style-example

COPY --from=planner /usr/src/app/recipe.json recipe.json
# This layer is cached as long as Cargo.lock / dependency tree is unchanged
RUN cargo chef cook --release -p "${PACKAGE}" --recipe-path recipe.json

# Copy source and build the full crate (produces all 11 binaries)
COPY . .
RUN cargo build --release -p "${PACKAGE}"

# --------------------------------------------------------------------------
# Stage 3: Lambda runtime image
# --------------------------------------------------------------------------
FROM public.ecr.aws/lambda/provided:al2023

ARG PACKAGE=closure-style-example
ARG BINARY_NAME=closure-basic-steps

COPY --from=builder "/usr/src/app/target/release/${BINARY_NAME}" "${LAMBDA_RUNTIME_DIR}/bootstrap"

CMD ["handler"]
```

**Why `BINARY_NAME` is separate from `PACKAGE`:**
The existing Dockerfile uses `COPY --from=builder ".../target/release/${PACKAGE}"` which only works when the crate name matches the binary name. In this workspace, crate `closure-style-example` produces binaries named `closure-basic-steps`, `closure-step-retries`, etc. — not `closure-style-example`. The fix: keep `PACKAGE` to drive `cargo build -p`, add `BINARY_NAME` to select which compiled binary to copy.

### Pattern 2: Per-Crate Parallel Build Script

**What:** One background job per crate. Each job: docker build for all 11 binaries in that crate (reuses the builder layer across the 11 tags), then docker push all 11.

**When to use:** When you have N crates × M binaries and want parallelism at the crate level without overloading the build host.

**Example:**
```bash
# Source: established pattern from scripts/deploy-ecr.sh + verify-prerequisites.sh

#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TF_DIR="$REPO_ROOT/infra"
PROFILE="adfs"
REGION="us-east-2"

# Verify prerequisites (credentials, Docker daemon, etc.)
"$SCRIPT_DIR/verify-prerequisites.sh" || exit 1

# Read ECR URL from Terraform outputs (never hardcode)
ECR_URL=$(terraform -chdir="$TF_DIR" output -raw ecr_repo_url)

# Login to ECR once — token valid for 12 hours
aws ecr get-login-password --profile "$PROFILE" --region "$REGION" \
  | docker login --username AWS --password-stdin "$ECR_URL"

# Define all 4 crates and their 11 binaries
declare -A CRATE_BINS
CRATE_BINS["closure-style-example"]="closure-basic-steps closure-step-retries closure-typed-errors closure-waits closure-callbacks closure-invoke closure-parallel closure-map closure-child-contexts closure-replay-safe-logging closure-combined-workflow"
CRATE_BINS["macro-style-example"]="macro-basic-steps macro-step-retries macro-typed-errors macro-waits macro-callbacks macro-invoke macro-parallel macro-map macro-child-contexts macro-replay-safe-logging macro-combined-workflow"
CRATE_BINS["trait-style-example"]="trait-basic-steps trait-step-retries trait-typed-errors trait-waits trait-callbacks trait-invoke trait-parallel trait-map trait-child-contexts trait-replay-safe-logging trait-combined-workflow"
CRATE_BINS["builder-style-example"]="builder-basic-steps builder-step-retries builder-typed-errors builder-waits builder-callbacks builder-invoke builder-parallel builder-map builder-child-contexts builder-replay-safe-logging builder-combined-workflow"

build_and_push_crate() {
  local package="$1"
  local bins="$2"
  echo "[${package}] Starting build..."

  for bin_name in $bins; do
    echo "[${package}] Building ${bin_name}..."
    docker build \
      -f "$REPO_ROOT/examples/Dockerfile" \
      --build-arg "PACKAGE=${package}" \
      --build-arg "BINARY_NAME=${bin_name}" \
      -t "${ECR_URL}:${bin_name}" \
      "$REPO_ROOT"
    docker push "${ECR_URL}:${bin_name}"
    echo "[${package}] Pushed ${bin_name}"
  done

  echo "[${package}] Done — 11 images pushed"
}

# Launch 4 parallel jobs, one per crate
PIDS=()
for package in "${!CRATE_BINS[@]}"; do
  build_and_push_crate "$package" "${CRATE_BINS[$package]}" &
  PIDS+=($!)
done

# Wait for all jobs and collect exit codes
FAILED=0
for pid in "${PIDS[@]}"; do
  wait "$pid" || FAILED=$((FAILED + 1))
done

if [[ $FAILED -gt 0 ]]; then
  echo "ERROR: $FAILED crate build job(s) failed."
  exit 1
fi

echo "=== All 44 images built and pushed to ${ECR_URL} ==="
```

### Anti-Patterns to Avoid

- **Running 44 separate `cargo build` commands:** Each would trigger a full Rust compilation. The per-crate approach (4 builds) is 10x faster.
- **Building images without cargo-chef:** Any source change triggers a full 60-min workspace compile. cargo-chef is mandatory, not optional.
- **Hardcoding the ECR URL:** The URL contains the account ID and suffix. Always read from `terraform output -raw ecr_repo_url`.
- **Calling `aws ecr get-login-password` 44 times:** Login once; the token is valid for 12 hours.
- **Using `latest` as the image tag:** Phase 11 Terraform references images by binary name (e.g., `:closure-basic-steps`). The tag must match exactly.
- **Logging credentials to stdout:** `docker login --password-stdin` takes the token on stdin; never `echo "$TOKEN"` to a log.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Rust dependency layer caching | Manual `COPY Cargo.toml Cargo.lock && RUN cargo fetch` | cargo-chef cook stage | The naive approach still rebuilds dependencies when any source file changes because Rust checks all paths; cargo-chef generates a minimal recipe that is truly stable across source changes |
| ECR authentication | Custom curl-based token fetching | `aws ecr get-login-password --profile adfs --region us-east-2` | AWS CLI handles SigV4 signing, token refresh, and output formatting; the token format is AWS-specific |
| Parallel job management | Custom async queuing | Bash `&` + `wait` | Sufficient for 4 jobs; zero dependencies |

**Key insight:** The cargo-chef dependency caching problem is deceptively hard to solve correctly with vanilla Dockerfile tricks. The naive `COPY Cargo.toml` approach works for single-crate projects but breaks for workspaces because Rust's build graph spans crate boundaries. cargo-chef correctly handles workspace member manifests.

---

## Common Pitfalls

### Pitfall 1: PACKAGE copies wrong binary name

**What goes wrong:** `COPY --from=builder ".../target/release/${PACKAGE}"` fails because `closure-style-example` is the crate name but the compiled binary is `closure-basic-steps`. The copy silently produces an empty or missing bootstrap file.

**Why it happens:** The existing Dockerfile was written assuming crate name = binary name. In this workspace, all crates use explicit `[[bin]]` sections with different names.

**How to avoid:** Use two separate ARGs: `PACKAGE` (drives `cargo build -p`) and `BINARY_NAME` (drives the `COPY` in the runtime stage). Both must be passed as `--build-arg` during `docker build`.

**Warning signs:** Docker build succeeds but `docker run` immediately exits; Lambda invocation returns "Runtime exited without providing a reason."

### Pitfall 2: cargo-chef cook stage uses wrong package scope

**What goes wrong:** `cargo chef cook --release --recipe-path recipe.json` without `-p ${PACKAGE}` cooks ALL workspace dependencies. This creates a very large cached layer but also means any change to any workspace crate invalidates the cache.

**Why it happens:** The cook command defaults to the full workspace.

**How to avoid:** Pass `-p "${PACKAGE}"` to `cargo chef cook` so the cached layer is scoped to that crate's transitive dependencies. For 4 separate crate images, this gives 4 independent cache entries — a change to `closure-style-example` doesn't invalidate the `macro-style-example` builder cache.

**Warning signs:** Build times do not improve after the first build; any code change forces full recompile.

### Pitfall 3: Parallel builds sharing Docker layer cache coherently

**What goes wrong:** 4 crate builds run simultaneously and all try to pull/push the same base image layers, causing Docker layer-store contention or `manifest unknown` errors.

**Why it happens:** Docker's layer store is process-safe but concurrent pulls of the same base layer can race.

**How to avoid:** The planner stage (`COPY . . && cargo chef prepare`) can run once before parallelism begins. Alternatively, accept Docker's built-in locking — it handles concurrent access correctly but may serialize initial pulls. Pre-pull the base images before launching background jobs:

```bash
docker pull lukemathwalker/cargo-chef:latest-rust-1
docker pull public.ecr.aws/lambda/provided:al2023
```

**Warning signs:** `Error response from daemon: Get "...": context deadline exceeded` during parallel build phase.

### Pitfall 4: ECR image tag mismatch with Terraform

**What goes wrong:** Build script pushes images with tags like `closure-style-example-basic-steps` but Terraform references `closure-basic-steps`. Plan 11-03 fails with `InvalidImageUri` or image pull error at Lambda invocation time.

**Why it happens:** Tag naming convention not locked down before building.

**How to avoid:** Tags are exactly the binary names from Cargo.toml `[[bin]]` sections: `closure-basic-steps`, `macro-invoke`, etc. Cross-check against `infra/lambda.tf` `image_uri` expressions before first push.

**Warning signs:** `terraform apply` in plan 11-03 fails with `ResourceNotFoundException` on Lambda creation.

### Pitfall 5: Background job error surfacing

**What goes wrong:** One crate build fails (e.g., Docker daemon OOM mid-build) but the script exits 0 because `wait` without explicit PID tracking doesn't capture individual exit codes in all Bash versions.

**Why it happens:** `wait` alone returns the exit code of the last process waited; preceding failures are lost.

**How to avoid:** Capture PIDs into an array and call `wait $pid` individually, collecting failures:

```bash
PIDS=()
for ...; do some_job & PIDS+=($!); done
FAILED=0
for pid in "${PIDS[@]}"; do wait "$pid" || FAILED=$((FAILED+1)); done
[[ $FAILED -eq 0 ]] || exit 1
```

**Warning signs:** Script reports "All 44 images pushed" but ECR shows fewer than 44 tags.

---

## Code Examples

Verified patterns from official sources and existing codebase:

### ECR Login (from existing scripts/deploy-ecr.sh pattern)
```bash
# Source: AWS CLI v2 docs; pattern consistent with existing scripts/deploy-ecr.sh
ECR_URL=$(terraform -chdir="$TF_DIR" output -raw ecr_repo_url)
aws ecr get-login-password --profile adfs --region us-east-2 \
  | docker login --username AWS --password-stdin "$ECR_URL"
```

### Docker Build with PACKAGE and BINARY_NAME ARGs
```bash
# Source: Dockerfile ARG pattern from cargo-chef README + existing Dockerfile structure
docker build \
  -f examples/Dockerfile \
  --build-arg "PACKAGE=closure-style-example" \
  --build-arg "BINARY_NAME=closure-basic-steps" \
  -t "REDACTED_ACCOUNT_ID.dkr.ecr.us-east-2.amazonaws.com/dr-examples-c351:closure-basic-steps" \
  .
```

### Verify all 44 tags exist in ECR after push
```bash
# Source: AWS CLI v2 docs for list-images
aws ecr list-images \
  --profile adfs \
  --region us-east-2 \
  --repository-name dr-examples-c351 \
  --query 'imageIds[*].imageTag' \
  --output text \
  | tr '\t' '\n' \
  | sort \
  | wc -l
# Should output: 44
```

### Read suffix from Terraform (for documentation/verification)
```bash
SUFFIX=$(terraform -chdir=infra output -raw suffix)
ECR_URL=$(terraform -chdir=infra output -raw ecr_repo_url)
# ECR_URL = REDACTED_ACCOUNT_ID.dkr.ecr.us-east-2.amazonaws.com/dr-examples-c351
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `COPY Cargo.toml && RUN cargo fetch` | cargo-chef planner+cook stages | cargo-chef 0.1.x (~2021), now 0.1.77 (2026) | Correct workspace-aware dependency caching; prevents full recompile on source changes |
| Separate Rust install in Dockerfile | `lukemathwalker/cargo-chef:latest-rust-1` base image | ~0.1.50 | Eliminates installation boilerplate; guarantees same Rust version across stages |
| `docker build` once with `CMD` swap | Separate `--build-arg BINARY_NAME` per image | N/A — project-specific | Each Lambda function needs its own image because the bootstrap binary differs |

**Deprecated/outdated:**
- `rust:1-bookworm` base image without cargo-chef: still valid for simple cases, but insufficient for caching in this workspace. Replace with `lukemathwalker/cargo-chef:latest-rust-1` in the planner and builder stages; keep `public.ecr.aws/lambda/provided:al2023` for the runtime stage.

---

## Complete Binary Manifest

All 44 binary names (from Cargo.toml inspection), grouped by crate:

**closure-style-example (11):**
`closure-basic-steps`, `closure-step-retries`, `closure-typed-errors`, `closure-waits`, `closure-callbacks`, `closure-invoke`, `closure-parallel`, `closure-map`, `closure-child-contexts`, `closure-replay-safe-logging`, `closure-combined-workflow`

**macro-style-example (11):**
`macro-basic-steps`, `macro-step-retries`, `macro-typed-errors`, `macro-waits`, `macro-callbacks`, `macro-invoke`, `macro-parallel`, `macro-map`, `macro-child-contexts`, `macro-replay-safe-logging`, `macro-combined-workflow`

**trait-style-example (11):**
`trait-basic-steps`, `trait-step-retries`, `trait-typed-errors`, `trait-waits`, `trait-callbacks`, `trait-invoke`, `trait-parallel`, `trait-map`, `trait-child-contexts`, `trait-replay-safe-logging`, `trait-combined-workflow`

**builder-style-example (11):**
`builder-basic-steps`, `builder-step-retries`, `builder-typed-errors`, `builder-waits`, `builder-callbacks`, `builder-invoke`, `builder-parallel`, `builder-map`, `builder-child-contexts`, `builder-replay-safe-logging`, `builder-combined-workflow`

---

## Open Questions

1. **cargo chef cook scope: per-crate vs full workspace**
   - What we know: `-p <crate>` narrows the cache layer; without it, the full workspace is cooked
   - What's unclear: Whether `-p` on `cargo chef cook` is fully supported for workspace crates with cross-crate dependencies (all 4 example crates depend on `durable-lambda-core`)
   - Recommendation: Use full workspace cook (`cargo chef cook --release`) in the Dockerfile to avoid potential cross-crate dependency resolution issues. The PACKAGE ARG still scopes the final `cargo build --release -p` step. Accept slightly larger cache layer in exchange for correctness.

2. **Docker build context size**
   - What we know: `COPY . .` in the planner stage sends the entire workspace (including `target/`) to the Docker build context
   - What's unclear: Whether `.dockerignore` exists to exclude `target/`
   - Recommendation: Add a `.dockerignore` at workspace root excluding `target/`, `.git/`, and `.planning/` to avoid sending hundreds of MB to every build. Check if it already exists before creating.

3. **First-build time estimate**
   - What we know: Cold workspace build without cache takes 60+ minutes; with cargo-chef after warm cache, much faster
   - What's unclear: Exact time for the first build (cold cargo-chef cook of all dependencies)
   - Recommendation: Document expected cold-build time as "30-60 minutes for first build; 5-10 minutes for subsequent source-only changes." Plan 12 should not set a 5-minute CI timeout.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | None for build scripts — validation is manual CLI verification |
| Config file | none |
| Quick run command | `bash -n scripts/build-images.sh` (syntax check only) |
| Full suite command | `bash scripts/build-images.sh && aws ecr list-images --profile adfs --region us-east-2 --repository-name dr-examples-c351 --query 'length(imageIds)'` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BUILD-01 | Dockerfile builds a valid Lambda bootstrap binary | smoke | `docker build -f examples/Dockerfile --build-arg PACKAGE=closure-style-example --build-arg BINARY_NAME=closure-basic-steps -t test-closure-basic-steps . && docker inspect test-closure-basic-steps` | ❌ Wave 0 |
| BUILD-02 | build-images.sh builds all 44 images | integration | `bash scripts/build-images.sh` (full run) | ❌ Wave 0 |
| BUILD-03 | All 44 images visible in ECR with correct tags | smoke | `aws ecr list-images --profile adfs --region us-east-2 --repository-name dr-examples-c351 --query 'length(imageIds)' --output text` → expect `44` | ❌ Wave 0 |
| BUILD-04 | 4 crate jobs run concurrently | manual | Inspect build log for 4 interleaved `[closure-...] / [macro-...] / [trait-...] / [builder-...]` log lines | manual-only |

### Sampling Rate

- **Per task commit:** `bash -n scripts/build-images.sh` (syntax check)
- **Per wave merge:** Single crate build smoke test against ECR
- **Phase gate:** Full `scripts/build-images.sh` run + ECR count check = 44 before moving to plan 11-03

### Wave 0 Gaps

- [ ] No `scripts/build-images.sh` yet — core deliverable of this phase
- [ ] `examples/Dockerfile` needs cargo-chef stages and `ARG BINARY_NAME` — Wave 0 task
- [ ] `.dockerignore` at workspace root — check existence, create if missing

*(These are implementation tasks, not missing test infrastructure)*

---

## Sources

### Primary (HIGH confidence)
- `examples/Dockerfile` (direct file read) — existing Dockerfile structure, PACKAGE ARG, stage layout
- `examples/closure-style/Cargo.toml`, `macro-style/Cargo.toml`, `trait-style/Cargo.toml`, `builder-style/Cargo.toml` (direct file read) — all 44 binary names confirmed
- `scripts/deploy-ecr.sh` (direct file read) — established ECR login + terraform output pattern
- `scripts/verify-prerequisites.sh` (direct file read) — established prerequisite gate pattern
- `.planning/phases/12-docker-build-pipeline/12-CONTEXT.md` (direct file read) — ECR URL, suffix, all user decisions
- [cargo-chef GitHub README](https://github.com/LukeMathWalker/cargo-chef) — three-stage Dockerfile pattern, version 0.1.77 current
- [Luca Palmieri — 5x Faster Rust Docker Builds with cargo-chef](https://lpalmieri.com/posts/fast-rust-docker-builds/) — original cargo-chef article documenting the pattern

### Secondary (MEDIUM confidence)
- `.planning/research/SUMMARY.md` (direct file read) — project-level research conclusions on cargo-chef requirement and Docker strategy

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions verified from cargo-chef GitHub (0.1.77), existing scripts inspected directly
- Architecture: HIGH — Dockerfile pattern from official cargo-chef README; binary names from direct Cargo.toml inspection; ECR/build patterns from existing scripts
- Pitfalls: HIGH — BINARY_NAME gap identified from direct Dockerfile inspection; background job error handling is standard Bash knowledge

**Research date:** 2026-03-17
**Valid until:** 2026-04-16 (30 days — cargo-chef is stable; AWS CLI ECR auth pattern is stable)
