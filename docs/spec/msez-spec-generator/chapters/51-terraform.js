const {
  chapterHeading, h2,
  p
} = require("../lib/primitives");

module.exports = function build_chapter51() {
  return [
    chapterHeading("Chapter 51: AWS Terraform Infrastructure (v0.4.44)"),

    // --- 51.1 Core Infrastructure ---
    h2("51.1 Core Infrastructure"),
    p("The Terraform configuration provisions a complete AWS environment for production MSEZ deployments. Core resources include: a dedicated VPC with public and private subnets across three availability zones, NAT gateways for private subnet egress, RDS PostgreSQL 16 with Multi-AZ deployment and automated backups (30-day retention), ElastiCache Redis 7 cluster for caching and rate limiting, S3 buckets for CAS object storage with versioning and server-side encryption (AES-256), CloudFront distribution for static asset delivery, ACM certificates for TLS, Route 53 hosted zones for DNS management, and CloudWatch log groups with 90-day retention. All resources are tagged with environment, project, and cost-center labels for operational tracking."),

    // --- 51.2 Kubernetes Resources ---
    h2("51.2 Kubernetes Resources"),
    p("An EKS cluster (Kubernetes 1.29) runs the MSEZ workloads with managed node groups. Terraform provisions: Deployments for msez-api and msez-worker with rolling update strategy (maxSurge: 1, maxUnavailable: 0), Services with internal load balancers for inter-service communication, Horizontal Pod Autoscalers (HPA) scaling on CPU (target 70%) and custom Prometheus metrics (request latency p99), Pod Disruption Budgets (PDB) ensuring at least one replica remains available during voluntary disruptions, ConfigMaps for jurisdiction-specific pack configurations and feature flags, Secrets managed via AWS Secrets Manager with external-secrets-operator synchronization, Ingress resources with AWS ALB controller for external traffic routing with WAF integration, and NetworkPolicies restricting inter-pod communication to explicitly allowed paths."),
  ];
};
