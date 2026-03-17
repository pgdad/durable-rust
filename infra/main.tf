terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.25"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.0"
    }
  }
}

provider "aws" {
  region  = "us-east-2"
  profile = "adfs"

  default_tags {
    tags = {
      Project   = "durable-rust"
      Milestone = "v1.1"
      ManagedBy = "terraform"
    }
  }
}

resource "random_id" "suffix" {
  byte_length = 2 # 2 bytes = 4 hex chars
}

locals {
  suffix = random_id.suffix.hex # e.g. "a3f2"
}
