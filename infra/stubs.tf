data "archive_file" "order_enrichment" {
  type        = "zip"
  output_path = "${path.module}/stubs/order_enrichment.zip"
  source {
    content  = file("${path.module}/stubs/order_enrichment.py")
    filename = "lambda_function.py"
  }
}

resource "aws_lambda_function" "order_enrichment" {
  function_name    = "dr-order-enrichment-lambda-${local.suffix}"
  role             = aws_iam_role.lambda_exec.arn
  handler          = "lambda_function.lambda_handler"
  runtime          = "python3.13"
  filename         = data.archive_file.order_enrichment.output_path
  source_code_hash = data.archive_file.order_enrichment.output_base64sha256
  publish          = true
  timeout          = 30
  memory_size      = 128
}

resource "aws_lambda_alias" "order_enrichment_live" {
  name             = "live"
  function_name    = aws_lambda_function.order_enrichment.function_name
  function_version = aws_lambda_function.order_enrichment.version
}

data "archive_file" "fulfillment" {
  type        = "zip"
  output_path = "${path.module}/stubs/fulfillment.zip"
  source {
    content  = file("${path.module}/stubs/fulfillment.py")
    filename = "lambda_function.py"
  }
}

resource "aws_lambda_function" "fulfillment" {
  function_name    = "dr-fulfillment-lambda-${local.suffix}"
  role             = aws_iam_role.lambda_exec.arn
  handler          = "lambda_function.lambda_handler"
  runtime          = "python3.13"
  filename         = data.archive_file.fulfillment.output_path
  source_code_hash = data.archive_file.fulfillment.output_base64sha256
  publish          = true
  timeout          = 30
  memory_size      = 128
}

resource "aws_lambda_alias" "fulfillment_live" {
  name             = "live"
  function_name    = aws_lambda_function.fulfillment.function_name
  function_version = aws_lambda_function.fulfillment.version
}
