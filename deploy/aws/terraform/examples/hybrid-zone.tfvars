# Example: Hybrid Zone Configuration
# NY civic + Delaware corporate + ADGM financial/digital assets
#
# Deploy with:
#   terraform init
#   terraform plan -var-file=examples/hybrid-zone.tfvars
#   terraform apply -var-file=examples/hybrid-zone.tfvars

# AWS Configuration
aws_region = "us-east-1"
environment = "prod"

# Zone Identity
zone_id        = "momentum.hybrid.nyc-de-adgm"
zone_name      = "NYC-Delaware-ADGM Hybrid Zone"
jurisdiction_id = "ae-abudhabi-adgm"  # Primary financial jurisdiction
profile        = "digital-financial-center"

# EKS Configuration
eks_node_instance_types = ["t3.xlarge", "t3.2xlarge"]
eks_node_desired_size   = 5

# Database Configuration
rds_instance_class   = "db.r6g.large"
rds_allocated_storage = 200

# Redis Configuration
redis_node_type = "cache.r6g.large"

# High Availability
enable_multi_az  = true
enable_encryption = true

# Container Images
ecr_registry = "123456789012.dkr.ecr.us-east-1.amazonaws.com"
image_tag    = "v0.4.44"

# Domain & SSL
zone_domain         = "hybrid.momentum.zone"
acm_certificate_arn = "arn:aws:acm:us-east-1:123456789012:certificate/xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
