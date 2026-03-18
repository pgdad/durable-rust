---
phase: quick-fix
plan: 1
type: execute
wave: 1
depends_on: []
files_modified: []
autonomous: true
must_haves:
  truths:
    - "macro-basic-steps Lambda returns a valid durable response instead of Runtime.ExitError"
    - "All 11 stale Lambda functions have live aliases pointing to musl-compiled image versions"
    - "No Lambda functions in the deployment have GLIBC mismatch errors"
  artifacts: []
  key_links:
    - from: "ECR image (musl-compiled)"
      to: "Lambda live alias"
      via: "publish-version + update-alias"
      pattern: "aws lambda update-function-code.*publish-version.*update-alias"
---

<objective>
Fix 11 Lambda functions (macro/trait/builder styles) whose `live` aliases point to pre-musl
image versions, causing GLIBC_2.38/2.39 "not found" crashes at runtime startup.

Purpose: Phase 16-02 pushed musl-compiled images to ECR and ran `terraform apply`, but
Terraform did not detect image digest changes for 11 of the 44 functions because the
image tag stayed the same. Those functions' `live` aliases still point to old versions
containing dynamically-linked (glibc-dependent) binaries that crash on the al2023 runtime.

Output: All 11 stale functions updated to use the current musl-compiled ECR images via
`update-function-code` + `publish-version` + `update-alias`.
</objective>

<execution_context>
@/home/esa/.claude/get-shit-done/workflows/execute-plan.md
@/home/esa/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
Root cause confirmed via CloudWatch Logs:
```
/var/runtime/bootstrap: /lib64/libc.so.6: version `GLIBC_2.38' not found (required by /var/runtime/bootstrap)
/var/runtime/bootstrap: /lib64/libc.so.6: version `GLIBC_2.39' not found (required by /var/runtime/bootstrap)
```

The `$LATEST` version of each function already has the correct musl image (pushed 2026-03-18),
but the published version that the `live` alias points to still uses the old pre-musl image
(pushed 2026-03-17). Need to force Lambda to pull the new image digest, publish a new version,
and update the alias.

Stale functions (11 total):
- macro-basic-steps, macro-parallel
- trait-typed-errors, trait-invoke, trait-map, trait-child-contexts, trait-replay-safe-logging
- builder-step-retries, builder-callbacks, builder-invoke, builder-map

Infrastructure naming: dr-{name}-c351 (suffix from Terraform random_id)
ECR repo: dr-examples-c351
AWS profile: adfs, region: us-east-2
</context>

<tasks>

<task type="auto">
  <name>Task 1: Update stale Lambda functions and publish new versions</name>
  <files></files>
  <action>
For each of the 11 stale Lambda functions, run the following AWS CLI sequence to force
Lambda to pull the current ECR image digest, publish a new version, and update the live alias:

```bash
STALE_FUNCTIONS=(
  macro-basic-steps macro-parallel
  trait-typed-errors trait-invoke trait-map trait-child-contexts trait-replay-safe-logging
  builder-step-retries builder-callbacks builder-invoke builder-map
)
ECR_URL=$(terraform -chdir=infra output -raw ecr_repo_url)
SUFFIX="c351"

for fn in "${STALE_FUNCTIONS[@]}"; do
  FULL_NAME="dr-${fn}-${SUFFIX}"
  IMAGE_URI="${ECR_URL}:${fn}"

  # Force Lambda to re-resolve the image tag to its current digest
  aws lambda update-function-code \
    --function-name "$FULL_NAME" \
    --image-uri "$IMAGE_URI" \
    --profile adfs --region us-east-2

  # Wait for update to complete (Lambda image updates are async)
  aws lambda wait function-updated-v2 \
    --function-name "$FULL_NAME" \
    --profile adfs --region us-east-2

  # Publish a new version from $LATEST
  NEW_VERSION=$(aws lambda publish-version \
    --function-name "$FULL_NAME" \
    --profile adfs --region us-east-2 \
    --query 'Version' --output text)

  # Update the live alias to point to the new version
  aws lambda update-alias \
    --function-name "$FULL_NAME" \
    --name live \
    --function-version "$NEW_VERSION" \
    --profile adfs --region us-east-2

  echo "Updated ${fn}: live -> v${NEW_VERSION}"
done
```

After all 11 are updated, verify that the `$LATEST` CodeSha256 matches the live alias
CodeSha256 for every function (not just the 11 stale ones, but all 44).
  </action>
  <verify>
Invoke macro-basic-steps via its live alias and confirm it no longer returns Runtime.ExitError:

```bash
aws lambda invoke \
  --function-name "arn:aws:lambda:us-east-2:REDACTED_ACCOUNT_ID:function:dr-macro-basic-steps-c351:live" \
  --payload '{"order_id":"verify-fix"}' \
  --cli-binary-format raw-in-base64-out \
  --profile adfs --region us-east-2 \
  /tmp/verify-fix.json && cat /tmp/verify-fix.json
```

Expected: Response contains `"Status": "SUCCEEDED"` or a durable execution response (not
`"errorType":"Runtime.ExitError"`). The function may return a parse error since the payload
lacks durable execution fields -- that is acceptable and proves the binary starts correctly.

Also spot-check one trait and one builder function to confirm they also work.
  </verify>
  <done>
All 11 stale Lambda functions have new published versions with musl-compiled images, and
their live aliases point to these new versions. macro-basic-steps invocation no longer
produces "Runtime exited with error: exit status 1" / GLIBC mismatch errors.
  </done>
</task>

<task type="auto">
  <name>Task 2: Run Terraform apply to sync state</name>
  <files></files>
  <action>
After the CLI updates, Terraform state is out of sync (it thinks the functions are on older
versions). Run `terraform apply -parallelism=5` to reconcile state. Terraform will detect
the version/alias drift and update its state to match.

If Terraform wants to revert any alias to an older version, that means the Terraform config
needs adjustment. In that case, run `terraform apply -refresh-only` first to import the
current state without making changes, then do a normal `terraform apply`.

The key concern is that `publish = true` in lambda.tf means Terraform will try to
publish yet another version on apply. This is fine -- it will just create a version
identical to what we just published. The alias will follow `aws_lambda_function.examples[key].version`.
  </action>
  <verify>
```bash
terraform -chdir=infra plan -parallelism=5 2>&1 | tail -5
```
Expected: "No changes" or only non-destructive changes. No Lambda functions should show
as needing code updates.
  </verify>
  <done>
Terraform state is synchronized with the actual AWS state. `terraform plan` shows no
pending changes for any Lambda function code or alias versions.
  </done>
</task>

</tasks>

<verification>
1. Invoke macro-basic-steps via live alias -- must NOT return Runtime.ExitError
2. Invoke trait-invoke via live alias -- must NOT return Runtime.ExitError
3. Invoke builder-invoke via live alias -- must NOT return Runtime.ExitError
4. `terraform plan` shows no changes (or only non-destructive changes)
5. CloudWatch logs for macro-basic-steps show no GLIBC errors on new invocations
</verification>

<success_criteria>
- macro-basic-steps Lambda responds without GLIBC/Runtime.ExitError
- All 11 previously-stale functions have live aliases on musl-compiled versions
- Terraform state is in sync with AWS
</success_criteria>

<output>
After completion, create `.planning/quick/1-fix-macro-basic-steps-lambda-runtime-exi/1-SUMMARY.md`
</output>
