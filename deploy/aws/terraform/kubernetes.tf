# Kubernetes Resources for MEZ Zone
# Deploys all zone services to EKS

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
  }
}

# -----------------------------------------------------------------------------
# Secret for Database Credentials
# -----------------------------------------------------------------------------

resource "kubernetes_secret" "db_credentials" {
  metadata {
    name      = "db-credentials"
    namespace = kubernetes_namespace.mez.metadata[0].name
  }

  data = {
    DATABASE_URL = "postgresql://mez:${random_password.rds.result}@${aws_db_instance.mez.endpoint}/mez"
    REDIS_URL    = "rediss://${aws_elasticache_replication_group.mez.primary_endpoint_address}:6379"
  }

  type = "Opaque"
}

# -----------------------------------------------------------------------------
# Zone Authority Deployment
# -----------------------------------------------------------------------------

resource "kubernetes_deployment" "zone_authority" {
  metadata {
    name      = "zone-authority"
    namespace = kubernetes_namespace.mez.metadata[0].name

    labels = {
      app       = "zone-authority"
      component = "core"
    }
  }

  spec {
    replicas = var.environment == "prod" ? 3 : 2

    selector {
      match_labels = {
        app = "zone-authority"
      }
    }

    template {
      metadata {
        labels = {
          app       = "zone-authority"
          component = "core"
        }
      }

      spec {
        service_account_name = kubernetes_service_account.zone_authority.metadata[0].name

        container {
          name  = "zone-authority"
          image = "${var.ecr_registry}/mez-zone-authority:${var.image_tag}"

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
              name = kubernetes_secret.db_credentials.metadata[0].name
            }
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
              path = "/health/live"
              port = 8080
            }
            initial_delay_seconds = 30
            period_seconds       = 10
          }

          readiness_probe {
            http_get {
              path = "/health/ready"
              port = 8080
            }
            initial_delay_seconds = 5
            period_seconds       = 5
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

resource "kubernetes_service" "zone_authority" {
  metadata {
    name      = "zone-authority"
    namespace = kubernetes_namespace.mez.metadata[0].name
  }

  spec {
    selector = {
      app = "zone-authority"
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
# Entity Registry Deployment
# -----------------------------------------------------------------------------

resource "kubernetes_deployment" "entity_registry" {
  metadata {
    name      = "entity-registry"
    namespace = kubernetes_namespace.mez.metadata[0].name

    labels = {
      app       = "entity-registry"
      component = "registry"
    }
  }

  spec {
    replicas = var.environment == "prod" ? 3 : 2

    selector {
      match_labels = {
        app = "entity-registry"
      }
    }

    template {
      metadata {
        labels = {
          app       = "entity-registry"
          component = "registry"
        }
      }

      spec {
        container {
          name  = "entity-registry"
          image = "${var.ecr_registry}/mez-zone-authority:${var.image_tag}"
          args  = ["python", "-m", "tools.mez", "entity-registry", "serve", "--port", "8083"]

          port {
            container_port = 8083
            name          = "http"
          }

          env_from {
            config_map_ref {
              name = kubernetes_config_map.zone_config.metadata[0].name
            }
          }

          env_from {
            secret_ref {
              name = kubernetes_secret.db_credentials.metadata[0].name
            }
          }

          resources {
            requests = {
              cpu    = "250m"
              memory = "256Mi"
            }
            limits = {
              cpu    = "1000m"
              memory = "1Gi"
            }
          }

          liveness_probe {
            http_get {
              path = "/health"
              port = 8083
            }
            initial_delay_seconds = 30
            period_seconds       = 10
          }
        }
      }
    }
  }
}

resource "kubernetes_service" "entity_registry" {
  metadata {
    name      = "entity-registry"
    namespace = kubernetes_namespace.mez.metadata[0].name
  }

  spec {
    selector = {
      app = "entity-registry"
    }

    port {
      port        = 8083
      target_port = 8083
    }

    type = "ClusterIP"
  }
}

# -----------------------------------------------------------------------------
# License Registry Deployment
# -----------------------------------------------------------------------------

resource "kubernetes_deployment" "license_registry" {
  metadata {
    name      = "license-registry"
    namespace = kubernetes_namespace.mez.metadata[0].name

    labels = {
      app       = "license-registry"
      component = "registry"
    }
  }

  spec {
    replicas = var.environment == "prod" ? 3 : 2

    selector {
      match_labels = {
        app = "license-registry"
      }
    }

    template {
      metadata {
        labels = {
          app       = "license-registry"
          component = "registry"
        }
      }

      spec {
        container {
          name  = "license-registry"
          image = "${var.ecr_registry}/mez-zone-authority:${var.image_tag}"
          args  = ["python", "-m", "tools.licensepack", "serve", "--port", "8084"]

          port {
            container_port = 8084
            name          = "http"
          }

          env_from {
            config_map_ref {
              name = kubernetes_config_map.zone_config.metadata[0].name
            }
          }

          env_from {
            secret_ref {
              name = kubernetes_secret.db_credentials.metadata[0].name
            }
          }

          resources {
            requests = {
              cpu    = "250m"
              memory = "256Mi"
            }
            limits = {
              cpu    = "1000m"
              memory = "1Gi"
            }
          }
        }
      }
    }
  }
}

# -----------------------------------------------------------------------------
# Corridor Node Deployment
# -----------------------------------------------------------------------------

resource "kubernetes_deployment" "corridor_node" {
  metadata {
    name      = "corridor-node"
    namespace = kubernetes_namespace.mez.metadata[0].name

    labels = {
      app       = "corridor-node"
      component = "corridor"
    }
  }

  spec {
    replicas = var.environment == "prod" ? 3 : 1

    selector {
      match_labels = {
        app = "corridor-node"
      }
    }

    template {
      metadata {
        labels = {
          app       = "corridor-node"
          component = "corridor"
        }
      }

      spec {
        container {
          name  = "corridor-node"
          image = "${var.ecr_registry}/mez-corridor-node:${var.image_tag}"

          port {
            container_port = 8081
            name          = "http"
          }

          env_from {
            config_map_ref {
              name = kubernetes_config_map.zone_config.metadata[0].name
            }
          }

          env_from {
            secret_ref {
              name = kubernetes_secret.db_credentials.metadata[0].name
            }
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
        }
      }
    }
  }
}

# -----------------------------------------------------------------------------
# Watcher Deployment
# -----------------------------------------------------------------------------

resource "kubernetes_deployment" "watcher" {
  metadata {
    name      = "watcher"
    namespace = kubernetes_namespace.mez.metadata[0].name

    labels = {
      app       = "watcher"
      component = "corridor"
    }
  }

  spec {
    replicas = var.environment == "prod" ? 3 : 1

    selector {
      match_labels = {
        app = "watcher"
      }
    }

    template {
      metadata {
        labels = {
          app       = "watcher"
          component = "corridor"
        }
      }

      spec {
        container {
          name  = "watcher"
          image = "${var.ecr_registry}/mez-watcher:${var.image_tag}"

          port {
            container_port = 8082
            name          = "http"
          }

          env_from {
            config_map_ref {
              name = kubernetes_config_map.zone_config.metadata[0].name
            }
          }

          env_from {
            secret_ref {
              name = kubernetes_secret.db_credentials.metadata[0].name
            }
          }

          resources {
            requests = {
              cpu    = "250m"
              memory = "256Mi"
            }
            limits = {
              cpu    = "1000m"
              memory = "1Gi"
            }
          }
        }
      }
    }
  }
}

# -----------------------------------------------------------------------------
# Service Accounts with IRSA
# -----------------------------------------------------------------------------

resource "kubernetes_service_account" "zone_authority" {
  metadata {
    name      = "zone-authority"
    namespace = kubernetes_namespace.mez.metadata[0].name

    annotations = {
      "eks.amazonaws.com/role-arn" = aws_iam_role.zone_authority.arn
    }
  }
}

resource "aws_iam_role" "zone_authority" {
  name = "mez-${var.zone_id}-zone-authority"

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
            "${module.eks.oidc_provider}:sub" = "system:serviceaccount:mez:zone-authority"
          }
        }
      }
    ]
  })
}

resource "aws_iam_role_policy" "zone_authority_s3" {
  name = "s3-access"
  role = aws_iam_role.zone_authority.id

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
      "kubernetes.io/ingress.class"               = "alb"
      "alb.ingress.kubernetes.io/scheme"          = "internet-facing"
      "alb.ingress.kubernetes.io/target-type"     = "ip"
      "alb.ingress.kubernetes.io/healthcheck-path" = "/health"
      "alb.ingress.kubernetes.io/certificate-arn" = var.acm_certificate_arn
      "alb.ingress.kubernetes.io/ssl-policy"      = "ELBSecurityPolicy-TLS13-1-2-2021-06"
    }
  }

  spec {
    rule {
      host = var.zone_domain

      http {
        path {
          path      = "/api/zone/*"
          path_type = "Prefix"

          backend {
            service {
              name = kubernetes_service.zone_authority.metadata[0].name
              port {
                number = 8080
              }
            }
          }
        }

        path {
          path      = "/api/entities/*"
          path_type = "Prefix"

          backend {
            service {
              name = kubernetes_service.entity_registry.metadata[0].name
              port {
                number = 8083
              }
            }
          }
        }

        path {
          path      = "/api/licenses/*"
          path_type = "Prefix"

          backend {
            service {
              name = "license-registry"
              port {
                number = 8084
              }
            }
          }
        }

        path {
          path      = "/api/corridors/*"
          path_type = "Prefix"

          backend {
            service {
              name = "corridor-node"
              port {
                number = 8081
              }
            }
          }
        }
      }
    }
  }

  depends_on = [
    kubernetes_deployment.zone_authority,
    kubernetes_deployment.entity_registry,
    kubernetes_deployment.license_registry,
    kubernetes_deployment.corridor_node,
  ]
}

# -----------------------------------------------------------------------------
# Additional Variables
# -----------------------------------------------------------------------------

variable "ecr_registry" {
  description = "ECR registry URL"
  type        = string
  default     = ""
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
  description = "Domain name for the zone"
  type        = string
  default     = "zone.example.com"
}

# -----------------------------------------------------------------------------
# Outputs
# -----------------------------------------------------------------------------

output "ingress_hostname" {
  description = "Ingress hostname"
  value       = kubernetes_ingress_v1.mez.status[0].load_balancer[0].ingress[0].hostname
}
