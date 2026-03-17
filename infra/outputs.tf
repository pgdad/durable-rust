output "ecr_repo_url" {
  value       = aws_ecr_repository.examples.repository_url
  description = "ECR repository URL for pushing images"
}

output "suffix" {
  value       = local.suffix
  description = "4-char hex suffix used in all resource names"
}

output "alias_arns" {
  value       = { for k, v in aws_lambda_alias.live : k => v.arn }
  description = "Map of binary name to live alias ARN for test harness"
}

output "stub_alias_arns" {
  value = {
    "order-enrichment-lambda" = aws_lambda_alias.order_enrichment_live.arn
    "fulfillment-lambda"      = aws_lambda_alias.fulfillment_live.arn
  }
  description = "Alias ARNs for callee stub functions"
}

output "function_names" {
  value       = { for k, v in aws_lambda_function.examples : k => v.function_name }
  description = "Map of binary name to Lambda function name for test harness"
}
