data "aws_caller_identity" "current" {}

resource "aws_iam_role" "lambda_exec" {
  name = "dr-lambda-exec-${local.suffix}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "lambda.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "durable_exec" {
  role       = aws_iam_role.lambda_exec.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicDurableExecutionRolePolicy"
}

# Required for invoke.rs and combined_workflow.rs handlers
resource "aws_iam_role_policy" "invoke_permission" {
  name = "dr-invoke-permission-${local.suffix}"
  role = aws_iam_role.lambda_exec.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Action = ["lambda:InvokeFunction"]
      Resource = [
        "arn:aws:lambda:us-east-2:${data.aws_caller_identity.current.account_id}:function:dr-order-enrichment-lambda-${local.suffix}",
        "arn:aws:lambda:us-east-2:${data.aws_caller_identity.current.account_id}:function:dr-fulfillment-lambda-${local.suffix}",
      ]
    }]
  })
}
