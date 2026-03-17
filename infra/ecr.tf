resource "aws_ecr_repository" "examples" {
  name                 = "dr-examples-${local.suffix}"
  image_tag_mutability = "MUTABLE"
  force_delete         = true

  image_scanning_configuration {
    scan_on_push = false
  }
}

resource "aws_ecr_lifecycle_policy" "examples" {
  repository = aws_ecr_repository.examples.name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last 2 images per tag"
        selection = {
          tagStatus      = "tagged"
          tagPatternList = ["*"]
          countType      = "imageCountMoreThan"
          countNumber    = 2
        }
        action = { type = "expire" }
      },
      {
        rulePriority = 2
        description  = "Expire untagged images after 2 days"
        selection = {
          tagStatus   = "untagged"
          countType   = "sinceImagePushed"
          countUnit   = "days"
          countNumber = 2
        }
        action = { type = "expire" }
      }
    ]
  })
}
