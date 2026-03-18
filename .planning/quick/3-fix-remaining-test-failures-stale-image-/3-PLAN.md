---
phase: quick-fix
plan: 3
type: execute
wave: 1
depends_on: []
files_modified:
  - examples/closure-style/src/invoke.rs
  - examples/macro-style/src/invoke.rs
  - examples/trait-style/src/invoke.rs
  - examples/builder-style/src/invoke.rs
  - examples/closure-style/src/combined_workflow.rs
  - examples/macro-style/src/combined_workflow.rs
  - examples/trait-style/src/combined_workflow.rs
  - examples/builder-style/src/combined_workflow.rs
  - infra/lambda.tf
  - scripts/test-helpers.sh
autonomous: true
must_haves:
  truths:
    - "closure-replay-safe-logging no longer returns Runtime.ExitError (stale image fixed)"
    - "All 8 invoke/combined_workflow tests pass (AccessDeniedException resolved via env var function names)"
    - "All 8 waits/callbacks tests pass (invoke_async returns exec_arn after CLI upgrade)"
    - "AWS CLI version is >= 2.32.8 (minimum for durable execution support)"
  artifacts:
    - path: "examples/closure-style/src/invoke.rs"
      provides: "Env-var-driven callee function name for invoke"
      contains: "ENRICHMENT_FUNCTION"
    - path: "examples/closure-style/src/combined_workflow.rs"
      provides: "Env-var-driven callee function name for combined_workflow"
      contains: "FULFILLMENT_FUNCTION"
    - path: "infra/lambda.tf"
      provides: "Environment blocks injecting ENRICHMENT_FUNCTION and FULFILLMENT_FUNCTION"
      contains: "environment"
  key_links:
    - from: "infra/lambda.tf environment block"
      to: "stubs.tf function names"
      via: "Terraform interpolation of stub function names"
      pattern: "dr-order-enrichment-lambda.*suffix"
    - from: "invoke.rs ENRICHMENT_FUNCTION env var"
      to: "infra/lambda.tf environment block"
      via: "Lambda runtime environment"
      pattern: "std::env::var.*ENRICHMENT_FUNCTION"
---

<objective>
Fix the remaining 17 test failures from live AWS testing across three distinct root causes:
1. One stale GLIBC image (closure-replay-safe-logging) -- republish with update-function-code
2. Eight AccessDeniedException failures (invoke + combined_workflow) -- handlers hardcode bare function names but AWS uses prefixed names; fix by injecting correct names via env vars
3. Eight missing exec_arn failures (waits + callbacks) -- AWS CLI 2.27.7 lacks durable execution support; upgrade to >= 2.32.8

Purpose: After this fix, all non-XFAIL tests in the integration suite should pass.
Output: Updated Rust handler source code, Terraform config with env vars, upgraded CLI, redeployed functions.
</objective>

<execution_context>
@/home/esa/.claude/get-shit-done/workflows/execute-plan.md
@/home/esa/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@infra/lambda.tf
@infra/stubs.tf
@infra/iam.tf
@scripts/test-helpers.sh
@scripts/test-all.sh
@examples/closure-style/src/invoke.rs
@examples/closure-style/src/combined_workflow.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Upgrade AWS CLI and fix stale image</name>
  <files></files>
  <action>
**Part A: Upgrade AWS CLI to >= 2.32.8**

The current AWS CLI version is 2.27.7 which does not have durable execution support
(no `get-durable-execution`, `get-durable-execution-history`, `send-durable-execution-callback-success`
commands, and no `--durable-execution-name` flag on `invoke`). These were added in CLI 2.32.8.

Upgrade the CLI using the official installer:

```bash
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "/tmp/awscliv2.zip"
unzip -o /tmp/awscliv2.zip -d /tmp/
sudo /tmp/aws/install --update
aws --version  # Must show >= 2.32.8
```

After upgrade, verify durable commands exist:

```bash
aws lambda get-durable-execution help 2>&1 | head -3
# Should show help text, not "Invalid choice"

aws lambda invoke help 2>&1 | grep -i "durable-execution-name"
# Should find the flag
```

**Part B: Fix stale closure-replay-safe-logging image**

Same pattern as quick fixes 1 and 2. The `closure-replay-safe-logging` function's live alias
still points to a pre-musl image version.

```bash
SUFFIX="c351"
ECR_URL=$(terraform -chdir=infra output -raw ecr_repo_url)
FN="closure-replay-safe-logging"
FULL_NAME="dr-${FN}-${SUFFIX}"

aws lambda update-function-code \
  --function-name "$FULL_NAME" \
  --image-uri "${ECR_URL}:${FN}" \
  --profile adfs --region us-east-2

aws lambda wait function-updated-v2 \
  --function-name "$FULL_NAME" \
  --profile adfs --region us-east-2

NEW_VERSION=$(aws lambda publish-version \
  --function-name "$FULL_NAME" \
  --profile adfs --region us-east-2 \
  --query 'Version' --output text)

aws lambda update-alias \
  --function-name "$FULL_NAME" \
  --name live \
  --function-version "$NEW_VERSION" \
  --profile adfs --region us-east-2

echo "Updated ${FN}: live -> v${NEW_VERSION}"
```

Verify with a quick invocation:

```bash
source scripts/test-helpers.sh
check_credentials
load_tf_outputs
assert_replay_safe_logging "closure-replay-safe-logging"
```
  </action>
  <verify>
```bash
aws --version 2>&1 | grep -E "2\.(3[2-9]|[4-9][0-9])" && echo "CLI OK" || echo "CLI TOO OLD"
aws lambda get-durable-execution help 2>&1 | head -1 | grep -v "Invalid" && echo "Durable commands OK"
bash scripts/test-all.sh closure-replay-safe-logging
```
  </verify>
  <done>
AWS CLI upgraded to >= 2.32.8 with durable execution support. closure-replay-safe-logging
passes its integration test (no more Runtime.ExitError).
  </done>
</task>

<task type="auto">
  <name>Task 2: Fix invoke AccessDeniedException via env var function names and redeploy</name>
  <files>
    examples/closure-style/src/invoke.rs
    examples/macro-style/src/invoke.rs
    examples/trait-style/src/invoke.rs
    examples/builder-style/src/invoke.rs
    examples/closure-style/src/combined_workflow.rs
    examples/macro-style/src/combined_workflow.rs
    examples/trait-style/src/combined_workflow.rs
    examples/builder-style/src/combined_workflow.rs
    infra/lambda.tf
  </files>
  <action>
The root cause: `ctx.invoke()` receives a `function_name` parameter that the durable execution
service uses to invoke the target Lambda. The handlers hardcode bare names like
`"order-enrichment-lambda"` but the actual deployed function names include prefix and suffix:
`"dr-order-enrichment-lambda-c351"`. The IAM policy allows invoke on the full names, but the
durable service tries to invoke by the bare name which does not exist.

**Step 1: Add environment variables to Terraform**

In `infra/lambda.tf`, add `environment` blocks to the Lambda function resource. Only the
invoke and combined_workflow functions need the env vars, but since all functions share the
same `for_each` resource block, use a conditional approach.

Add a locals block computing per-handler environment variables:

```hcl
locals {
  # Environment variables for handlers that use ctx.invoke()
  invoke_env = {
    ENRICHMENT_FUNCTION = aws_lambda_function.order_enrichment.function_name
    FULFILLMENT_FUNCTION = aws_lambda_function.fulfillment.function_name
  }

  # Handlers that need invoke env vars
  invoke_handlers = toset([
    "closure-invoke", "macro-invoke", "trait-invoke", "builder-invoke",
    "closure-combined-workflow", "macro-combined-workflow", "trait-combined-workflow", "builder-combined-workflow",
  ])

  handler_env = {
    for key, _ in local.handlers : key => (
      contains(local.invoke_handlers, key)
      ? local.invoke_env
      : {}
    )
  }
}
```

Then add an `environment` block to the `aws_lambda_function.examples` resource using a
`dynamic` block:

```hcl
  dynamic "environment" {
    for_each = length(local.handler_env[each.key]) > 0 ? [1] : []
    content {
      variables = local.handler_env[each.key]
    }
  }
```

**Step 2: Update all 4 invoke.rs files**

Replace the hardcoded `"order-enrichment-lambda"` with `std::env::var("ENRICHMENT_FUNCTION")`.

For all 4 styles (closure/macro/trait/builder), change the `ctx.invoke()` call from:

```rust
ctx.invoke("enrich_order", "order-enrichment-lambda", &payload).await?;
```

to:

```rust
let enrichment_fn = std::env::var("ENRICHMENT_FUNCTION")
    .unwrap_or_else(|_| "order-enrichment-lambda".to_string());
ctx.invoke("enrich_order", &enrichment_fn, &payload).await?;
```

Keep the fallback default for local testing / mock contexts.

Files to update:
- `examples/closure-style/src/invoke.rs` line 19: change `"order-enrichment-lambda"` to env var
- `examples/macro-style/src/invoke.rs` line 20: same change
- `examples/trait-style/src/invoke.rs` line 20: same change
- `examples/builder-style/src/invoke.rs` line 14: same change

**Step 3: Update all 4 combined_workflow.rs files**

Replace the hardcoded `"fulfillment-lambda"` with `std::env::var("FULFILLMENT_FUNCTION")`.

For all 4 styles, change:

```rust
ctx.invoke("start_fulfillment", "fulfillment-lambda", &payload).await?;
```

to:

```rust
let fulfillment_fn = std::env::var("FULFILLMENT_FUNCTION")
    .unwrap_or_else(|_| "fulfillment-lambda".to_string());
ctx.invoke("start_fulfillment", &fulfillment_fn, &payload).await?;
```

Files to update:
- `examples/closure-style/src/combined_workflow.rs` line 57: change `"fulfillment-lambda"` to env var
- `examples/macro-style/src/combined_workflow.rs` line 55: same change
- `examples/trait-style/src/combined_workflow.rs` line 61: same change
- `examples/builder-style/src/combined_workflow.rs` line 55: same change

**Step 4: Verify Rust code compiles**

```bash
cargo build --workspace
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
```

**Step 5: Rebuild and push affected Docker images**

Only 8 binaries need rebuilding (invoke and combined_workflow across 4 styles). But the Docker
build system builds all binaries for a package at once. Since all 4 packages are affected,
all images need rebuilding. Use `bash scripts/build-images.sh` for the full rebuild.

Alternatively, to save time, build only the affected binaries:

```bash
ECR_URL=$(terraform -chdir=infra output -raw ecr_repo_url)

# ECR login
aws ecr get-login-password --profile adfs --region us-east-2 \
  | docker login --username AWS --password-stdin "$ECR_URL"

# Pre-pull base images (avoid contention)
docker pull lukemathwalker/cargo-chef:latest-rust-1
docker pull public.ecr.aws/lambda/provided:al2023

# Build affected binaries
AFFECTED_BINS=(
  closure-invoke closure-combined-workflow
  macro-invoke macro-combined-workflow
  trait-invoke trait-combined-workflow
  builder-invoke builder-combined-workflow
)
# Also rebuild waits (Phase 15-01 modified waits.rs but never redeployed)
AFFECTED_BINS+=(
  closure-waits macro-waits trait-waits builder-waits
)

# Map binary to package
declare -A BIN_PACKAGE
BIN_PACKAGE[closure-invoke]="closure-style-example"
BIN_PACKAGE[closure-combined-workflow]="closure-style-example"
BIN_PACKAGE[closure-waits]="closure-style-example"
BIN_PACKAGE[macro-invoke]="macro-style-example"
BIN_PACKAGE[macro-combined-workflow]="macro-style-example"
BIN_PACKAGE[macro-waits]="macro-style-example"
BIN_PACKAGE[trait-invoke]="trait-style-example"
BIN_PACKAGE[trait-combined-workflow]="trait-style-example"
BIN_PACKAGE[trait-waits]="trait-style-example"
BIN_PACKAGE[builder-invoke]="builder-style-example"
BIN_PACKAGE[builder-combined-workflow]="builder-style-example"
BIN_PACKAGE[builder-waits]="builder-style-example"

for bin_name in "${AFFECTED_BINS[@]}"; do
  PACKAGE="${BIN_PACKAGE[$bin_name]}"
  echo "Building $bin_name (package: $PACKAGE)..."
  docker build \
    -f examples/Dockerfile \
    --build-arg "PACKAGE=$PACKAGE" \
    --build-arg "BINARY_NAME=$bin_name" \
    --provenance=false \
    -t "${ECR_URL}:${bin_name}" \
    .
  docker push "${ECR_URL}:${bin_name}"
  echo "Pushed $bin_name"
done
```

NOTE: The Docker cargo-chef layer caches workspace dependencies, so after the first image
rebuild the others will be much faster (only the final stage re-copies the specific binary).

**Step 6: Deploy via Terraform**

```bash
terraform -chdir=infra apply -parallelism=5 -auto-approve
```

This will:
- Detect the environment variable additions on the 8 invoke/combined_workflow functions
- Detect image digest changes on all rebuilt functions
- Publish new versions and update live aliases

**Step 7: Force-update any functions Terraform missed**

After terraform apply, verify the live alias image digest matches $LATEST for all rebuilt
functions. If any are stale (Terraform tag-reuse issue), run the update-function-code +
publish-version + update-alias loop for them (same pattern as quick fix 1).

```bash
SUFFIX="c351"
for fn in "${AFFECTED_BINS[@]}"; do
  FULL_NAME="dr-${fn}-${SUFFIX}"
  LATEST_SHA=$(aws lambda get-function --function-name "$FULL_NAME" --qualifier '$LATEST' \
    --profile adfs --region us-east-2 --query 'Code.ImageUri' --output text 2>/dev/null)
  LIVE_VERSION=$(aws lambda get-alias --function-name "$FULL_NAME" --name live \
    --profile adfs --region us-east-2 --query 'FunctionVersion' --output text 2>/dev/null)
  LIVE_SHA=$(aws lambda get-function --function-name "$FULL_NAME" --qualifier "$LIVE_VERSION" \
    --profile adfs --region us-east-2 --query 'Code.ImageUri' --output text 2>/dev/null)

  if [[ "$LATEST_SHA" != "$LIVE_SHA" ]]; then
    echo "STALE: $fn (live=$LIVE_SHA, latest=$LATEST_SHA) -- forcing update..."
    aws lambda update-function-code \
      --function-name "$FULL_NAME" --image-uri "${ECR_URL}:${fn}" \
      --profile adfs --region us-east-2
    aws lambda wait function-updated-v2 --function-name "$FULL_NAME" --profile adfs --region us-east-2
    NEW_VER=$(aws lambda publish-version --function-name "$FULL_NAME" \
      --profile adfs --region us-east-2 --query 'Version' --output text)
    aws lambda update-alias --function-name "$FULL_NAME" --name live --function-version "$NEW_VER" \
      --profile adfs --region us-east-2
    echo "  Updated $fn: live -> v${NEW_VER}"
  else
    echo "OK: $fn (digests match)"
  fi
done
```
  </action>
  <verify>
Run the 8 invoke tests (these are synchronous and don't need the durable async flow):

```bash
bash scripts/test-all.sh closure-invoke
bash scripts/test-all.sh macro-invoke
bash scripts/test-all.sh trait-invoke
bash scripts/test-all.sh builder-invoke
bash scripts/test-all.sh closure-combined-workflow
bash scripts/test-all.sh macro-combined-workflow
bash scripts/test-all.sh trait-combined-workflow
bash scripts/test-all.sh builder-combined-workflow
```

All 8 should report [PASS].
  </verify>
  <done>
All 4 invoke handlers read ENRICHMENT_FUNCTION from env var. All 4 combined_workflow handlers
read FULFILLMENT_FUNCTION from env var. Terraform injects the correct full function names
(dr-order-enrichment-lambda-c351 and dr-fulfillment-lambda-c351). All 8 tests pass without
AccessDeniedException.
  </done>
</task>

<task type="auto">
  <name>Task 3: Verify waits and callbacks tests with upgraded CLI</name>
  <files>scripts/test-helpers.sh</files>
  <action>
With the CLI upgraded (Task 1) and waits images rebuilt (Task 2 rebuilt waits binaries), the
waits and callbacks tests should now work because:

1. `aws lambda invoke --durable-execution-name` flag is now recognized
2. The response from Event invocation with durable-execution-name should include `DurableExecutionArn`
3. `aws lambda get-durable-execution` command now exists
4. `aws lambda get-durable-execution-history` command now exists
5. `aws lambda send-durable-execution-callback-success` command now exists

**Step 1: Test one waits handler manually first**

```bash
source scripts/test-helpers.sh
check_credentials
load_tf_outputs

# Get the closure-waits ARN
WAITS_ARN=$(get_alias_arn "closure-waits")
echo "Invoking: $WAITS_ARN"

# Try invoke_async
EXEC_ARN=$(invoke_async "$WAITS_ARN" '{"wait_seconds":5}')
echo "exec_arn: $EXEC_ARN"
```

If `invoke_async` returns an exec_arn, the CLI upgrade fixed Issue 3.

If `invoke_async` still fails (DurableExecutionArn not in response), investigate the response
format of the upgraded CLI:

```bash
aws lambda invoke \
  --profile adfs --region us-east-2 \
  --function-name "$WAITS_ARN" \
  --invocation-type Event \
  --cli-binary-format raw-in-base64-out \
  --durable-execution-name "manual-test-$(date +%s)" \
  --payload '{"wait_seconds":5}' \
  /tmp/dr-manual-test.json 2>&1
```

Examine the raw response. If the DurableExecutionArn is in a different location or format,
update `invoke_async` in `scripts/test-helpers.sh` accordingly.

Possible adjustments to `invoke_async` if the response format differs:
- If exec_arn is in the output file instead of metadata: read from `$out_file` instead of `$meta`
- If exec_arn field name differs: update the jq path
- If the function needs qualified ARN (`:live` suffix): already handled -- test uses alias ARN

**Step 2: If invoke_async works, confirm get_execution_output Output field**

```bash
# After a waits execution succeeds:
aws lambda get-durable-execution \
  --profile adfs --region us-east-2 \
  --durable-execution-arn "$EXEC_ARN" 2>&1
```

Examine the raw response to confirm the `Output` field name is correct. If the field is named
differently (e.g., `Result`, `Payload`), update `get_execution_output()` in test-helpers.sh
to use the correct `--query` path.

**Step 3: Run all waits and callbacks tests**

```bash
bash scripts/test-all.sh closure-waits
bash scripts/test-all.sh macro-waits
bash scripts/test-all.sh trait-waits
bash scripts/test-all.sh builder-waits
bash scripts/test-all.sh closure-callbacks
bash scripts/test-all.sh macro-callbacks
bash scripts/test-all.sh trait-callbacks
bash scripts/test-all.sh builder-callbacks
```

**Step 4: If any test-helpers.sh adjustments were needed, update and commit**

If the response format required changes to `invoke_async`, `get_execution_output`,
`extract_callback_id`, or `send_callback_success` in test-helpers.sh, make those fixes
and verify all 8 tests pass.

Common adjustments that may be needed:
- `invoke_async`: The `DurableExecutionArn` may be in the output file for Event invocations
  rather than the stdout metadata. Check both locations.
- `get_execution_output`: The `--query 'Output'` path was marked as provisional in STATE.md.
  If the actual field is different, update the query.
- `send_callback_success`: The `--result` parameter might need different quoting or format.
  </action>
  <verify>
Run all 8 waits + callbacks tests:

```bash
for test in closure-waits macro-waits trait-waits builder-waits \
            closure-callbacks macro-callbacks trait-callbacks builder-callbacks; do
  bash scripts/test-all.sh "$test"
done
```

All 8 should report [PASS].
  </verify>
  <done>
All 4 waits tests pass (invoke_async returns exec_arn, poll reaches SUCCEEDED, output has
started/completed status). All 4 callbacks tests pass (invoke_async returns exec_arn,
callback_id extracted, approval signal sent, outcome.approved=true). Any test-helpers.sh
adjustments for response format are committed.
  </done>
</task>

</tasks>

<verification>
Run the full integration test suite to confirm all non-XFAIL tests pass:

```bash
bash scripts/test-all.sh
```

Expected results:
- Phase 14 sync tests: 20 PASS (8 basic/retries/typed_errors/logging + 12 XFAIL parallel/map/child_contexts + 4 combined_workflow PASS now)
- Phase 15 async tests: 12 PASS (4 waits + 4 callbacks + 4 invoke)
- Phase 16 advanced tests: 4 PASS
- Total: 48 tests, 36 PASS, 12 XFAIL (Context ops unsupported by service)
</verification>

<success_criteria>
- AWS CLI >= 2.32.8 installed with durable execution command support
- closure-replay-safe-logging test passes (stale image fixed)
- All 8 invoke/combined_workflow tests pass (env var function names, no AccessDeniedException)
- All 8 waits/callbacks tests pass (CLI supports --durable-execution-name and durable execution APIs)
- `cargo build --workspace` and `cargo clippy --workspace -- -D warnings` pass
- Full test suite: 36 PASS + 12 XFAIL (Context ops)
</success_criteria>

<output>
After completion, create `.planning/quick/3-fix-remaining-test-failures-stale-image-/3-SUMMARY.md`
</output>
