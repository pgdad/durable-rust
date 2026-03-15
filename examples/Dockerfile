# Dockerfile for building durable Lambda functions as container images.
#
# This multi-stage build compiles your Rust durable Lambda handler and
# packages it for deployment on AWS Lambda with the provided.al2023 runtime.
#
# Usage:
#   docker build -f examples/Dockerfile -t my-durable-lambda .
#   docker push <account>.dkr.ecr.<region>.amazonaws.com/my-durable-lambda:latest
#
# To build a different example or your own handler, change the -p flag:
#   docker build -f examples/Dockerfile --build-arg PACKAGE=my-handler -t my-handler .

# --------------------------------------------------------------------------
# Stage 1: Build the Rust binary
# --------------------------------------------------------------------------
FROM rust:1-bookworm AS builder

# Default package to build — override with --build-arg PACKAGE=your-crate
ARG PACKAGE=closure-style-example

WORKDIR /usr/src/app

# Copy the full workspace (Cargo.toml, Cargo.lock, crates/, examples/)
COPY . .

# Build the release binary for the target package
RUN cargo build --release -p "${PACKAGE}"

# --------------------------------------------------------------------------
# Stage 2: Lambda runtime image
# --------------------------------------------------------------------------
FROM public.ecr.aws/lambda/provided:al2023

# Default package name — must match Stage 1
ARG PACKAGE=closure-style-example

# Copy the compiled binary as the Lambda bootstrap entrypoint.
# provided.al2023 looks for a binary named "bootstrap" in LAMBDA_RUNTIME_DIR.
COPY --from=builder "/usr/src/app/target/release/${PACKAGE}" "${LAMBDA_RUNTIME_DIR}/bootstrap"

# The CMD value is the Lambda handler name. For lambda_runtime-based handlers
# this is ignored (the runtime handles routing internally), but it is required
# by the container image specification.
CMD ["handler"]
