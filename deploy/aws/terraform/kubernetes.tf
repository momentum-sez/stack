# Kubernetes Resources for MEZ Zone
#
# Deploys a single mez-api binary to EKS. All five Mass primitive APIs,
# corridor operations, compliance evaluation, and watcher economy are
# served from this one process (ADR-001: Single Binary Over Microservices).
#
# Architecture:
#   1x mez-api deployment (2-3 replicas) + RDS Postgres + ElastiCache Redis
#   All routes: /v1/entities, /v1/corridors, /v1/compliance, /health, /metrics

provider "kubernetes" {
  host                   = module.eks.cluster_endpoint
  cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

  exec {
    api_version = "client.authentication.k8s.io/v1beta1"
    command     = "aws"
    args        = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
  }
}

provider "helm" {
  kubernetes {
    host                   = module.eks.cluster_endpoint
    cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

    exec {
      api_version = "client.authentication.k8s.io/v1beta1"
      command     = "aws"
      args        = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
    }
  }
}

# -----------------------------------------------------------------------------
# Namespace
# -----------------------------------------------------------------------------

resource "kubernetes_namespace" "mez" {
  metadata {
    name = "mez"

    labels = {
      "app.kubernetes.io/name"       = "mez"
      "app.kubernetes.io/managed-by" = "terraform"
      "mez.zone/id"                 = var.zone_id
    }
  }
}

# -----------------------------------------------------------------------------
# ConfigMap for Zone Configuration
# -----------------------------------------------------------------------------

resource "kubernetes_config_map" "zone_config" {
  metadata {
    name      = "zone-config"
    namespace = kubernetes_namespace.mez.metadata[0].name
  }

  data = {
    MEZ_ZONE_ID      = var.zone_id
    MEZ_JURISDICTION = var.jurisdiction_id
    MEZ_PROFILE      = var.profile
    MEZ_ENVIRONMENT  = var.environment
    MEZ_LOG_LEVEL    = var.environment == "prod" ? "info" : "debug"
    MEZ_HOST         = "0.0.0.0"
    MEZ_PORT         = "8080"
    SOVEREIGN_MASS   = "true"
  }
}

# -----------------------------------------------------------------------------
# Secret for Database & Auth Credentials
# -----------------------------------------------------------------------------

resource "kubernetes_secret" "mez_credentials" {
  metadata {
    name      = "mez-credentials"
    namespace = kubernetes_namespace.mez.metadata[0].name
  }

  data = {
    DATABASE_URL   = "postgresql://mez:${random_password.rds.result}@${aws_db_instance.mez.endpoint}/mez"
    REDIS_URL      = "rediss://${aws_elasticache_replication_group.mez.primary_endpoint_address}:6379"
    AUTH_TOKEN     = var.auth_token
    MASS_API_TOKEN = var.mass_api_token
  }

  type = "Opaque"
}

# -----------------------------------------------------------------------------
# mez-api Deployment — single binary serving all routes
# -----------------------------------------------------------------------------

resource "kubernetes_deployment" "mez_api" {
  metadata {
    name      = "mez-api"
    namespace = kubernetes_namespace.mez.metadata[0].name

    labels = {
      app       = "mez-api"
      component = "core"
    }
  }

  spec {
    replicas = var.environment == "prod" ? 3 : 2

    strategy {
      type = "RollingUpdate"
      rolling_update {
        max_unavailable = 0
        max_surge       = 1
      }
    }

    selector {
      match_labels = {
        app = "mez-api"
      }
    }

    template {
      metadata {
        labels = {
          app       = "mez-api"
          component = "core"
        }

        annotations = {
          "prometheus.io/scrape" = "true"
          "prometheus.io/port"   = "8080"
          "prometheus.io/path"   = "/metrics"
        }
      }

      spec {
        service_account_name = kubernetes_service_account.mez_api.metadata[0].name

        container {
          name  = "mez-api"
          image = "${var.ecr_registry}/mez-api:${var.image_tag}"

          port {
            container_port = 8080
            name          = "http"
          }

          env_from {
            config_map_ref {
              name = kubernetes_config_map.zone_config.metadata[0].name
            }
          }

          env_from {
            secret_ref {
              name = kubernetes_secret.mez_credentials.metadata[0].name
            }
          }

          # In sovereign mode, Mass client URLs point to self
          env {
            name  = "MASS_ORG_INFO_URL"
            value = "http://localhost:8080"
          }
          env {
            name  = "MASS_TREASURY_INFO_URL"
            value = "http://localhost:8080"
          }
          env {
            name  = "MASS_CONSENT_INFO_URL"
            value = "http://localhost:8080"
          }
          env {
            name  = "MASS_INVESTMENT_INFO_URL"
            value = "http://localhost:8080"
          }

          resources {
            requests = {
              cpu    = "500m"
              memory = "512Mi"
            }
            limits = {
              cpu    = "2000m"
              memory = "2Gi"
            }
          }

          liveness_probe {
            http_get {
              path = "/health/liveness"
              port = 8080
            }
            initial_delay_seconds = 15
            period_seconds       = 10
            failure_threshold    = 3
          }

          readiness_probe {
            http_get {
              path = "/health/readiness"
              port = 8080
            }
            initial_delay_seconds = 5
            period_seconds       = 5
            failure_threshold    = 3
          }

          startup_probe {
            http_get {
              path = "/health/liveness"
              port = 8080
            }
            initial_delay_seconds = 5
            period_seconds       = 5
            failure_threshold    = 12
          }

          security_context {
            run_as_non_root             = true
            run_as_user                = 1000
            read_only_root_filesystem  = true
            allow_privilege_escalation = false
          }
        }
      }
    }
  }
}

# -----------------------------------------------------------------------------
# Service
# -----------------------------------------------------------------------------

resource "kubernetes_service" "mez_api" {
  metadata {
    name      = "mez-api"
    namespace = kubernetes_namespace.mez.metadata[0].name
  }

  spec {
    selector = {
      app = "mez-api"
    }

    port {
      port        = 8080
      target_port = 8080
      protocol    = "TCP"
    }

    type = "ClusterIP"
  }
}

# -----------------------------------------------------------------------------
# Service Account with IRSA
# -----------------------------------------------------------------------------

resource "kubernetes_service_account" "mez_api" {
  metadata {
    name      = "mez-api"
    namespace = kubernetes_namespace.mez.metadata[0].name

    annotations = {
      "eks.amazonaws.com/role-arn" = aws_iam_role.mez_api.arn
    }
  }
}

resource "aws_iam_role" "mez_api" {
  name = "mez-${var.zone_id}-api"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = {
          Federated = module.eks.oidc_provider_arn
        }
        Action = "sts:AssumeRoleWithWebIdentity"
        Condition = {
          StringEquals = {
            "${module.eks.oidc_provider}:sub" = "system:serviceaccount:mez:mez-api"
          }
        }
      }
    ]
  })
}

resource "aws_iam_role_policy" "mez_api_s3" {
  name = "s3-access"
  role = aws_iam_role.mez_api.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:ListBucket"
        ]
        Resource = [
          aws_s3_bucket.artifacts.arn,
          "${aws_s3_bucket.artifacts.arn}/*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt",
          "kms:Encrypt",
          "kms:GenerateDataKey"
        ]
        Resource = [aws_kms_key.mez.arn]
      }
    ]
  })
}

# -----------------------------------------------------------------------------
# Ingress (ALB)
# -----------------------------------------------------------------------------

resource "kubernetes_ingress_v1" "mez" {
  metadata {
    name      = "mez-ingress"
    namespace = kubernetes_namespace.mez.metadata[0].name

    annotations = {
      "kubernetes.io/ingress.class"                = "alb"
      "alb.ingress.kubernetes.io/scheme"           = "internet-facing"
      "alb.ingress.kubernetes.io/target-type"      = "ip"
      "alb.ingress.kubernetes.io/healthcheck-path" = "/health/liveness"
      "alb.ingress.kubernetes.io/certificate-arn"  = var.acm_certificate_arn
      "alb.ingress.kubernetes.io/ssl-policy"       = "ELBSecurityPolicy-TLS13-1-2-2021-06"
    }
  }

  spec {
    rule {
      host = var.zone_domain

      http {
        # Single backend — mez-api serves all routes
        path {
          path      = "/"
          path_type = "Prefix"

          backend {
            service {
              name = kubernetes_service.mez_api.metadata[0].name
              port {
                number = 8080
              }
            }
          }
        }
      }
    }
  }

  depends_on = [
    kubernetes_deployment.mez_api,
  ]
}

# -----------------------------------------------------------------------------
# Variables
# -----------------------------------------------------------------------------

variable "ecr_registry" {
  description = "ECR registry URL (e.g., 123456789.dkr.ecr.us-east-1.amazonaws.com)"
  type        = string
}

variable "image_tag" {
  description = "Docker image tag to deploy"
  type        = string
  default     = "latest"
}

variable "acm_certificate_arn" {
  description = "ACM certificate ARN for HTTPS"
  type        = string
  default     = ""
}

variable "zone_domain" {
  description = "Domain name for the zone (e.g., pk-sifc.zone.momentum.inc)"
  type        = string
}

variable "auth_token" {
  description = "Bearer token for API authentication"
  type        = string
  sensitive   = true
}

variable "mass_api_token" {
  description = "Token for Mass API authentication"
  type        = string
  sensitive   = true
}

# -----------------------------------------------------------------------------
# Outputs
# -----------------------------------------------------------------------------

output "ingress_hostname" {
  description = "ALB hostname for the zone"
  value       = kubernetes_ingress_v1.mez.status[0].load_balancer[0].ingress[0].hostname
}

output "api_endpoint" {
  description = "Full API endpoint URL"
  value       = "https://${var.zone_domain}"
}
