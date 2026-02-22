# MEZ Zone - AWS Infrastructure
# Terraform configuration for production zone deployment
#
# This deploys a complete EZ-in-a-Box with:
# - EKS cluster for container orchestration
# - RDS PostgreSQL for persistence
# - ElastiCache Redis for caching/pub-sub
# - S3 for artifact storage
# - CloudWatch for observability
# - KMS for encryption
# - VPC with private subnets

terraform {
  required_version = ">= 1.5.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.25"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.12"
    }
  }

  backend "s3" {
    # Configure in backend.hcl:
    # bucket         = "mez-terraform-state"
    # key            = "zones/${zone_id}/terraform.tfstate"
    # region         = "us-east-1"
    # encrypt        = true
    # dynamodb_table = "mez-terraform-locks"
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "mez"
      Environment = var.environment
      ZoneId      = var.zone_id
      ManagedBy   = "terraform"
    }
  }
}

# -----------------------------------------------------------------------------
# Variables
# -----------------------------------------------------------------------------

variable "aws_region" {
  description = "AWS region for deployment"
  type        = string
  default     = "us-east-1"
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
  default     = "dev"
}

variable "zone_id" {
  description = "MEZ Zone identifier"
  type        = string
}

variable "zone_name" {
  description = "Human-readable zone name"
  type        = string
}

variable "jurisdiction_id" {
  description = "Primary jurisdiction identifier"
  type        = string
}

variable "profile" {
  description = "Zone profile (digital-financial-center, trade-corridor, etc.)"
  type        = string
  default     = "digital-financial-center"
}

variable "eks_node_instance_types" {
  description = "EC2 instance types for EKS nodes"
  type        = list(string)
  default     = ["t3.large", "t3.xlarge"]
}

variable "eks_node_desired_size" {
  description = "Desired number of EKS nodes"
  type        = number
  default     = 3
}

variable "rds_instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t3.medium"
}

variable "rds_allocated_storage" {
  description = "RDS allocated storage in GB"
  type        = number
  default     = 100
}

variable "redis_node_type" {
  description = "ElastiCache Redis node type"
  type        = string
  default     = "cache.t3.medium"
}

variable "enable_multi_az" {
  description = "Enable Multi-AZ deployment for high availability"
  type        = bool
  default     = true
}

variable "enable_encryption" {
  description = "Enable encryption at rest for all services"
  type        = bool
  default     = true
}

# -----------------------------------------------------------------------------
# Data Sources
# -----------------------------------------------------------------------------

data "aws_availability_zones" "available" {
  state = "available"
}

data "aws_caller_identity" "current" {}

# -----------------------------------------------------------------------------
# KMS Key for Encryption
# -----------------------------------------------------------------------------

resource "aws_kms_key" "mez" {
  description             = "MEZ Zone encryption key for ${var.zone_id}"
  deletion_window_in_days = 30
  enable_key_rotation     = true

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "Enable IAM User Permissions"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      },
      {
        Sid    = "Allow EKS to use the key"
        Effect = "Allow"
        Principal = {
          Service = "eks.amazonaws.com"
        }
        Action = [
          "kms:Encrypt",
          "kms:Decrypt",
          "kms:ReEncrypt*",
          "kms:GenerateDataKey*",
          "kms:DescribeKey"
        ]
        Resource = "*"
      }
    ]
  })

  tags = {
    Name = "mez-${var.zone_id}-key"
  }
}

resource "aws_kms_alias" "mez" {
  name          = "alias/mez-${var.zone_id}"
  target_key_id = aws_kms_key.mez.key_id
}

# -----------------------------------------------------------------------------
# VPC
# -----------------------------------------------------------------------------

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = "mez-${var.zone_id}-vpc"
  cidr = "10.0.0.0/16"

  azs             = slice(data.aws_availability_zones.available.names, 0, 3)
  private_subnets = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
  public_subnets  = ["10.0.101.0/24", "10.0.102.0/24", "10.0.103.0/24"]

  enable_nat_gateway     = true
  single_nat_gateway     = var.environment != "prod"
  enable_dns_hostnames   = true
  enable_dns_support     = true

  # Tags required for EKS
  public_subnet_tags = {
    "kubernetes.io/role/elb"                    = 1
    "kubernetes.io/cluster/mez-${var.zone_id}" = "shared"
  }

  private_subnet_tags = {
    "kubernetes.io/role/internal-elb"           = 1
    "kubernetes.io/cluster/mez-${var.zone_id}" = "shared"
  }
}

# -----------------------------------------------------------------------------
# EKS Cluster
# -----------------------------------------------------------------------------

module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 19.0"

  cluster_name    = "mez-${var.zone_id}"
  cluster_version = "1.29"

  vpc_id     = module.vpc.vpc_id
  subnet_ids = module.vpc.private_subnets

  cluster_endpoint_public_access  = var.environment != "prod"
  cluster_endpoint_private_access = true

  # Encryption
  cluster_encryption_config = var.enable_encryption ? {
    provider_key_arn = aws_kms_key.mez.arn
    resources        = ["secrets"]
  } : {}

  # Managed node groups
  eks_managed_node_groups = {
    mez_nodes = {
      name           = "mez-nodes"
      instance_types = var.eks_node_instance_types

      min_size     = 2
      max_size     = 10
      desired_size = var.eks_node_desired_size

      # Use latest Amazon Linux 2 EKS-optimized AMI
      ami_type = "AL2_x86_64"

      # Enable encryption on node volumes
      block_device_mappings = {
        xvda = {
          device_name = "/dev/xvda"
          ebs = {
            volume_size           = 100
            volume_type           = "gp3"
            encrypted             = var.enable_encryption
            kms_key_id           = var.enable_encryption ? aws_kms_key.mez.arn : null
            delete_on_termination = true
          }
        }
      }

      labels = {
        "mez.zone"    = var.zone_id
        "mez.profile" = var.profile
      }

      tags = {
        ZoneId = var.zone_id
      }
    }
  }

  # Enable IRSA for service accounts
  enable_irsa = true

  # Cluster addons
  cluster_addons = {
    coredns = {
      most_recent = true
    }
    kube-proxy = {
      most_recent = true
    }
    vpc-cni = {
      most_recent = true
    }
  }

  tags = {
    ZoneId = var.zone_id
  }
}

# -----------------------------------------------------------------------------
# RDS PostgreSQL
# -----------------------------------------------------------------------------

resource "aws_db_subnet_group" "mez" {
  name       = "mez-${var.zone_id}"
  subnet_ids = module.vpc.private_subnets

  tags = {
    Name = "mez-${var.zone_id}-db-subnet-group"
  }
}

resource "aws_security_group" "rds" {
  name        = "mez-${var.zone_id}-rds"
  description = "Security group for MEZ RDS"
  vpc_id      = module.vpc.vpc_id

  ingress {
    description     = "PostgreSQL from EKS"
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [module.eks.cluster_security_group_id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "mez-${var.zone_id}-rds-sg"
  }
}

resource "random_password" "rds" {
  length  = 32
  special = false
}

resource "aws_secretsmanager_secret" "rds_password" {
  name                    = "mez/${var.zone_id}/rds-password"
  recovery_window_in_days = 7
  kms_key_id             = aws_kms_key.mez.id
}

resource "aws_secretsmanager_secret_version" "rds_password" {
  secret_id     = aws_secretsmanager_secret.rds_password.id
  secret_string = random_password.rds.result
}

resource "aws_db_instance" "mez" {
  identifier = "mez-${var.zone_id}"

  engine         = "postgres"
  engine_version = "16.1"
  instance_class = var.rds_instance_class

  allocated_storage     = var.rds_allocated_storage
  max_allocated_storage = var.rds_allocated_storage * 2
  storage_type          = "gp3"
  storage_encrypted     = var.enable_encryption
  kms_key_id           = var.enable_encryption ? aws_kms_key.mez.arn : null

  db_name  = "mez"
  username = "mez"
  password = random_password.rds.result

  db_subnet_group_name   = aws_db_subnet_group.mez.name
  vpc_security_group_ids = [aws_security_group.rds.id]

  multi_az               = var.enable_multi_az
  publicly_accessible    = false

  backup_retention_period = 30
  backup_window          = "02:00-03:00"
  maintenance_window     = "Mon:04:00-Mon:05:00"

  deletion_protection = var.environment == "prod"
  skip_final_snapshot = var.environment != "prod"
  final_snapshot_identifier = var.environment == "prod" ? "mez-${var.zone_id}-final" : null

  performance_insights_enabled    = true
  performance_insights_kms_key_id = var.enable_encryption ? aws_kms_key.mez.arn : null

  # Enable IAM database authentication for credential-free pod access.
  iam_database_authentication_enabled = true

  # Export logs to CloudWatch for auditing and monitoring.
  enabled_cloudwatch_logs_exports = ["postgresql", "upgrade"]

  tags = {
    Name = "mez-${var.zone_id}-postgres"
  }
}

# -----------------------------------------------------------------------------
# ElastiCache Redis
# -----------------------------------------------------------------------------

resource "aws_elasticache_subnet_group" "mez" {
  name       = "mez-${var.zone_id}"
  subnet_ids = module.vpc.private_subnets
}

resource "aws_security_group" "redis" {
  name        = "mez-${var.zone_id}-redis"
  description = "Security group for MEZ Redis"
  vpc_id      = module.vpc.vpc_id

  ingress {
    description     = "Redis from EKS"
    from_port       = 6379
    to_port         = 6379
    protocol        = "tcp"
    security_groups = [module.eks.cluster_security_group_id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "mez-${var.zone_id}-redis-sg"
  }
}

resource "aws_elasticache_replication_group" "mez" {
  replication_group_id = "mez-${var.zone_id}"
  description          = "MEZ Zone Redis cluster"

  engine         = "redis"
  engine_version = "7.0"
  node_type      = var.redis_node_type
  port           = 6379

  num_cache_clusters         = var.enable_multi_az ? 2 : 1
  automatic_failover_enabled = var.enable_multi_az
  multi_az_enabled          = var.enable_multi_az

  subnet_group_name  = aws_elasticache_subnet_group.mez.name
  security_group_ids = [aws_security_group.redis.id]

  at_rest_encryption_enabled = var.enable_encryption
  transit_encryption_enabled = true
  kms_key_id                = var.enable_encryption ? aws_kms_key.mez.arn : null

  snapshot_retention_limit = 7
  snapshot_window         = "02:00-03:00"
  maintenance_window      = "sun:03:00-sun:04:00"

  tags = {
    Name = "mez-${var.zone_id}-redis"
  }
}

# -----------------------------------------------------------------------------
# S3 Bucket for Artifacts
# -----------------------------------------------------------------------------

resource "aws_s3_bucket" "artifacts" {
  bucket = "mez-${var.zone_id}-artifacts"

  tags = {
    Name = "mez-${var.zone_id}-artifacts"
  }
}

resource "aws_s3_bucket_versioning" "artifacts" {
  bucket = aws_s3_bucket.artifacts.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "artifacts" {
  bucket = aws_s3_bucket.artifacts.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = "aws:kms"
      kms_master_key_id = aws_kms_key.mez.arn
    }
    bucket_key_enabled = true
  }
}

resource "aws_s3_bucket_public_access_block" "artifacts" {
  bucket = aws_s3_bucket.artifacts.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Deny non-HTTPS access to the artifacts bucket.
resource "aws_s3_bucket_policy" "artifacts_https_only" {
  bucket = aws_s3_bucket.artifacts.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid       = "DenyInsecureTransport"
        Effect    = "Deny"
        Principal = "*"
        Action    = "s3:*"
        Resource = [
          aws_s3_bucket.artifacts.arn,
          "${aws_s3_bucket.artifacts.arn}/*"
        ]
        Condition = {
          Bool = {
            "aws:SecureTransport" = "false"
          }
        }
      }
    ]
  })
}

# -----------------------------------------------------------------------------
# Outputs
# -----------------------------------------------------------------------------

output "vpc_id" {
  description = "VPC ID"
  value       = module.vpc.vpc_id
}

output "eks_cluster_name" {
  description = "EKS cluster name"
  value       = module.eks.cluster_name
}

output "eks_cluster_endpoint" {
  description = "EKS cluster endpoint"
  value       = module.eks.cluster_endpoint
}

output "rds_endpoint" {
  description = "RDS endpoint"
  value       = aws_db_instance.mez.endpoint
}

output "redis_endpoint" {
  description = "Redis primary endpoint"
  value       = aws_elasticache_replication_group.mez.primary_endpoint_address
}

output "artifacts_bucket" {
  description = "S3 artifacts bucket name"
  value       = aws_s3_bucket.artifacts.id
}

output "kms_key_arn" {
  description = "KMS key ARN"
  value       = aws_kms_key.mez.arn
}

output "rds_secret_arn" {
  description = "Secrets Manager secret ARN for RDS password"
  value       = aws_secretsmanager_secret.rds_password.arn
}
