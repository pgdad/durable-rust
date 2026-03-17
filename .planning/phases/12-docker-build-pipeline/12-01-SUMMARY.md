---
phase: 12-docker-build-pipeline
plan: "01"
subsystem: infra
tags: [docker, cargo-chef, dockerfile, rust, lambda, ecr]

# Dependency graph
requires:
  - phase: 11-infrastructure
    provides: ECR repository (dr-examples-c351) where images will be pushed
provides:
  - cargo-chef three-stage Dockerfile with PACKAGE and BINARY_NAME ARGs
  - .dockerignore excluding target/, .git/, .planning/ from build context
affects:
  - 12-02-build-images-script
  - 11-03-deploy-lambdas

# Tech tracking
tech-stack:
  added:
    - "cargo-chef (lukemathwalker/cargo-chef:latest-rust-1) — Dockerfile base image for planner and builder stages"
  patterns:
    - "Three-stage cargo-chef build: planner generates recipe.json, builder cooks dependencies (cached layer) then compiles, runtime copies binary"
    - "PACKAGE ARG drives cargo build -p; BINARY_NAME ARG drives COPY in runtime stage — kept separate because crate names differ from binary names"
    - "Full workspace cook (no -p flag) avoids cross-crate dependency resolution failures when all crates share durable-lambda-core"

key-files:
  created:
    - .dockerignore
  modified:
    - examples/Dockerfile

key-decisions:
  - "Full workspace cargo chef cook (no -p flag) chosen over per-crate cook to avoid cross-crate dependency resolution issues; all 4 example crates share durable-lambda-core as a transitive dep"
  - "BINARY_NAME ARG added separately from PACKAGE to fix existing bug where Dockerfile assumed crate name equals binary name"
  - ".dockerignore excludes *.md globally but allows examples/**/*.md to preserve example READMEs"

patterns-established:
  - "Cargo-chef Dockerfile pattern: chef base → planner (recipe.json) → builder (cook + build) → runtime (copy binary)"
  - ".dockerignore at workspace root gates all docker build context"

requirements-completed: [BUILD-01]

# Metrics
duration: 3min
completed: 2026-03-17
---

# Phase 12 Plan 01: Docker Build Pipeline — Dockerfile Summary

**Three-stage cargo-chef Dockerfile with separate PACKAGE/BINARY_NAME ARGs and .dockerignore, verified to build closure-basic-steps Lambda image with CACHED dependency layer on second build**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-17T16:42:39Z
- **Completed:** 2026-03-17T16:45:21Z
- **Tasks:** 1 of 1
- **Files modified:** 2

## Accomplishments
- Rewrote examples/Dockerfile with four-stage cargo-chef build (chef/planner/builder/runtime), eliminating 60-minute cold recompiles on source-only changes
- Added BINARY_NAME ARG separate from PACKAGE, fixing the pre-existing bug where the runtime COPY used the crate name instead of the binary name
- Created .dockerignore at workspace root excluding target/, .git/, .planning/ (prevents hundreds of MB of artifacts from entering build context)
- Verified docker build succeeds end-to-end: /var/runtime/bootstrap present in image; second build after source touch shows cook step as CACHED

## Task Commits

Each task was committed atomically:

1. **Task 1: Create .dockerignore and rewrite Dockerfile with cargo-chef** - `9ec6e70` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified
- `examples/Dockerfile` - Four-stage cargo-chef build; PACKAGE drives cargo build -p, BINARY_NAME drives runtime COPY
- `.dockerignore` - Excludes target/, .git/, .planning/, infra/.terraform/, terraform state files, *.md (preserving examples/**/*.md)

## Decisions Made
- **Full workspace cook over per-crate cook:** All 4 example crates depend on durable-lambda-core; per-crate cook risks cross-crate dependency resolution failures. Full workspace cook accepted as the safer choice with a slightly larger cache layer.
- **BINARY_NAME default set to `closure-basic-steps`:** Aligns with the most commonly used test binary; always overridden by build-images.sh (plan 12-02).

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None. First cold docker build completed successfully (dependency compilation ~30s, source compilation ~28s — fast because the machine already had Rust toolchain layers cached from prior operations).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness
- Dockerfile is ready for plan 12-02 (build-images.sh script)
- Both PACKAGE and BINARY_NAME ARGs work as intended; build-images.sh can drive all 44 image builds
- cargo-chef layer caching verified working — subsequent builds will be fast
- .dockerignore reduces build context to source-only, speeding up `docker build` context transfer

---
*Phase: 12-docker-build-pipeline*
*Completed: 2026-03-17*
