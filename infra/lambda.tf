locals {
  handlers = {
    # Closure style
    "closure-basic-steps"         = { style = "closure", package = "closure-style-example" }
    "closure-step-retries"        = { style = "closure", package = "closure-style-example" }
    "closure-typed-errors"        = { style = "closure", package = "closure-style-example" }
    "closure-waits"               = { style = "closure", package = "closure-style-example" }
    "closure-callbacks"           = { style = "closure", package = "closure-style-example" }
    "closure-invoke"              = { style = "closure", package = "closure-style-example" }
    "closure-parallel"            = { style = "closure", package = "closure-style-example" }
    "closure-map"                 = { style = "closure", package = "closure-style-example" }
    "closure-child-contexts"      = { style = "closure", package = "closure-style-example" }
    "closure-replay-safe-logging" = { style = "closure", package = "closure-style-example" }
    "closure-combined-workflow"   = { style = "closure", package = "closure-style-example" }
    # Closure style — advanced features (Phase 16)
    "closure-saga-compensation"   = { style = "closure", package = "closure-style-example" }
    "closure-step-timeout"        = { style = "closure", package = "closure-style-example" }
    "closure-conditional-retry"   = { style = "closure", package = "closure-style-example" }
    "closure-batch-checkpoint"    = { style = "closure", package = "closure-style-example" }
    # Macro style
    "macro-basic-steps"         = { style = "macro", package = "macro-style-example" }
    "macro-step-retries"        = { style = "macro", package = "macro-style-example" }
    "macro-typed-errors"        = { style = "macro", package = "macro-style-example" }
    "macro-waits"               = { style = "macro", package = "macro-style-example" }
    "macro-callbacks"           = { style = "macro", package = "macro-style-example" }
    "macro-invoke"              = { style = "macro", package = "macro-style-example" }
    "macro-parallel"            = { style = "macro", package = "macro-style-example" }
    "macro-map"                 = { style = "macro", package = "macro-style-example" }
    "macro-child-contexts"      = { style = "macro", package = "macro-style-example" }
    "macro-replay-safe-logging" = { style = "macro", package = "macro-style-example" }
    "macro-combined-workflow"   = { style = "macro", package = "macro-style-example" }
    # Trait style
    "trait-basic-steps"         = { style = "trait", package = "trait-style-example" }
    "trait-step-retries"        = { style = "trait", package = "trait-style-example" }
    "trait-typed-errors"        = { style = "trait", package = "trait-style-example" }
    "trait-waits"               = { style = "trait", package = "trait-style-example" }
    "trait-callbacks"           = { style = "trait", package = "trait-style-example" }
    "trait-invoke"              = { style = "trait", package = "trait-style-example" }
    "trait-parallel"            = { style = "trait", package = "trait-style-example" }
    "trait-map"                 = { style = "trait", package = "trait-style-example" }
    "trait-child-contexts"      = { style = "trait", package = "trait-style-example" }
    "trait-replay-safe-logging" = { style = "trait", package = "trait-style-example" }
    "trait-combined-workflow"   = { style = "trait", package = "trait-style-example" }
    # Builder style
    "builder-basic-steps"         = { style = "builder", package = "builder-style-example" }
    "builder-step-retries"        = { style = "builder", package = "builder-style-example" }
    "builder-typed-errors"        = { style = "builder", package = "builder-style-example" }
    "builder-waits"               = { style = "builder", package = "builder-style-example" }
    "builder-callbacks"           = { style = "builder", package = "builder-style-example" }
    "builder-invoke"              = { style = "builder", package = "builder-style-example" }
    "builder-parallel"            = { style = "builder", package = "builder-style-example" }
    "builder-map"                 = { style = "builder", package = "builder-style-example" }
    "builder-child-contexts"      = { style = "builder", package = "builder-style-example" }
    "builder-replay-safe-logging" = { style = "builder", package = "builder-style-example" }
    "builder-combined-workflow"   = { style = "builder", package = "builder-style-example" }
  }
}

resource "aws_lambda_function" "examples" {
  for_each = local.handlers

  function_name = "dr-${each.key}-${local.suffix}"
  role          = aws_iam_role.lambda_exec.arn
  package_type  = "Image"
  image_uri     = "${aws_ecr_repository.examples.repository_url}:${each.key}"
  publish       = true # REQUIRED: makes .version return a real version number for alias

  timeout     = 900 # Lambda invocation timeout; durable_config.execution_timeout governs durable lifecycle
  memory_size = 256

  durable_config {
    execution_timeout = 840 # 14 min — must be <= 900s (Lambda timeout) to allow synchronous invocation in integration tests
    retention_period  = 7   # days to retain checkpoint state
  }

  tags = {
    Style = each.value.style
  }
}

resource "aws_lambda_alias" "live" {
  for_each = local.handlers

  name             = "live"
  function_name    = aws_lambda_function.examples[each.key].function_name
  function_version = aws_lambda_function.examples[each.key].version
}
