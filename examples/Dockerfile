# Dockerfile for building durable Lambda functions as container images.
#
# This multi-stage build compiles your Rust durable Lambda handler using
# cargo-chef for dependency-layer caching, then packages the result for
# deployment on AWS Lambda with the provided.al2023 runtime.
#
# cargo-chef separates the dependency compilation (slow, cached) from source
# compilation (fast, always runs), so source-only changes rebuild in minutes
# rather than 60+ minutes.
#
# Build ARGs:
#   PACKAGE     — Cargo crate name to build (e.g. closure-style-example)
#   BINARY_NAME — Compiled binary to copy as the Lambda bootstrap
#                 (e.g. closure-basic-steps)
#
# Usage:
#   docker build -f examples/Dockerfile \
#     --build-arg PACKAGE=closure-style-example \
#     --build-arg BINARY_NAME=closure-basic-steps \
#     -t my-durable-lambda .
#
#   docker push <account>.dkr.ecr.<region>.amazonaws.com/my-repo:closure-basic-steps

# --------------------------------------------------------------------------
# Stage 0: chef base — Rust toolchain + cargo-chef pre-installed
# --------------------------------------------------------------------------
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /usr/src/app

# --------------------------------------------------------------------------
# Stage 1: Planner — generates recipe.json from workspace Cargo files only.
# recipe.json encodes just the dependency metadata, not source code, so this
# layer is stable across source-only changes.
# --------------------------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --------------------------------------------------------------------------
# Stage 2: Builder — cooks the dependency layer (cached), then builds the crate
# --------------------------------------------------------------------------
FROM chef AS builder

# Default crate to build — override with --build-arg PACKAGE=your-crate
ARG PACKAGE=closure-style-example

COPY --from=planner /usr/src/app/recipe.json recipe.json

# Cook the full workspace dependency layer. No -p flag here: all 4 example
# crates share durable-lambda-core as a transitive dependency, so scoping
# the cook per-crate can cause cross-crate dependency resolution failures.
# This layer is cached as long as Cargo.lock / the dependency tree is unchanged.
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source and build the specified crate (produces all its binaries)
COPY . .
RUN cargo build --release -p "${PACKAGE}"

# --------------------------------------------------------------------------
# Stage 3: Lambda runtime image
# --------------------------------------------------------------------------
FROM public.ecr.aws/lambda/provided:al2023

# BINARY_NAME is separate from PACKAGE because crate names do not match binary
# names in this workspace (e.g. crate closure-style-example produces binaries
# named closure-basic-steps, closure-step-retries, etc.).
ARG BINARY_NAME=closure-basic-steps

# Copy the compiled binary as the Lambda bootstrap entrypoint.
# provided.al2023 looks for a binary named "bootstrap" in LAMBDA_RUNTIME_DIR.
COPY --from=builder "/usr/src/app/target/release/${BINARY_NAME}" "${LAMBDA_RUNTIME_DIR}/bootstrap"

# The CMD value is the Lambda handler name. For lambda_runtime-based handlers
# this is ignored (the runtime handles routing internally), but it is required
# by the container image specification.
CMD ["handler"]
