const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  table, codeBlock
} = require("../lib/primitives");

module.exports = function build_chapter51() {
  return [
    chapterHeading("Chapter 51: AWS Terraform Infrastructure (v0.4.44)"),

    // --- 51.1 Core Infrastructure ---
    h2("51.1 Core Infrastructure"),
    p("The Terraform configuration provisions a complete AWS environment for production MSEZ deployments. All resources are tagged with environment, project, and cost-center labels for operational tracking."),
    table(
      ["Resource", "Service", "Configuration", "Purpose"],
      [
        ["VPC", "Amazon VPC", "3 AZs, public + private subnets, NAT gateways", "Network isolation with private subnet egress"],
        ["Database", "RDS PostgreSQL 16", "Multi-AZ, 30-day automated backups, encrypted at rest", "Primary persistence for corridor state, tensor snapshots, audit log"],
        ["Cache", "ElastiCache Redis 7", "Cluster mode, encryption in transit", "Rate limiting, session store, tensor evaluation cache"],
        ["Object Storage", "S3", "Versioning, AES-256 SSE, lifecycle policies", "Content-addressed store (CAS) for artifacts and pack content"],
        ["CDN", "CloudFront", "Distribution with S3 origin, TLS 1.3", "Static asset delivery, geographic edge caching"],
        ["TLS", "ACM", "Auto-renewing certificates", "TLS termination for API and CDN endpoints"],
        ["DNS", "Route 53", "Hosted zones with health checks", "DNS management, failover routing"],
        ["Logging", "CloudWatch", "Log groups with 90-day retention", "Centralized logging for all services"],
      ],
      [1400, 1800, 2800, 3360]
    ),

    h3("51.1.1 Network Architecture"),
    p("The VPC spans three availability zones for fault tolerance. Public subnets host the Application Load Balancer and NAT gateways. Private subnets host EKS worker nodes, RDS instances, and ElastiCache clusters. No application workload has a public IP address; all egress routes through NAT gateways with Elastic IPs for consistent source addresses in firewall allowlists."),
    ...codeBlock(
`# VPC CIDR allocation
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  cidr = "10.0.0.0/16"
  azs  = ["me-south-1a", "me-south-1b", "me-south-1c"]

  private_subnets  = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
  public_subnets   = ["10.0.101.0/24", "10.0.102.0/24", "10.0.103.0/24"]
  database_subnets = ["10.0.201.0/24", "10.0.202.0/24", "10.0.203.0/24"]

  enable_nat_gateway   = true
  single_nat_gateway   = false  # One per AZ for HA
  enable_dns_hostnames = true
}`
    ),

    h3("51.1.2 Database Configuration"),
    p("RDS PostgreSQL uses a dedicated subnet group across three AZs. The instance class scales per deployment profile: db.t3.medium for standard, db.r6g.xlarge for enterprise, db.r6g.4xlarge for sovereign-govos. Automated backups retain 30 days with point-in-time recovery. Performance Insights is enabled for query analysis."),
    table(
      ["Parameter", "Standard", "Enterprise", "Sovereign-GovOS"],
      [
        ["Instance class", "db.t3.medium", "db.r6g.xlarge", "db.r6g.4xlarge"],
        ["Storage", "100 GB gp3", "500 GB io2", "2 TB io2"],
        ["IOPS", "3,000 (baseline)", "10,000 (provisioned)", "40,000 (provisioned)"],
        ["Multi-AZ", "Yes", "Yes", "Yes + read replica"],
        ["Backup retention", "30 days", "30 days", "35 days"],
        ["Encryption", "AES-256 (AWS KMS)", "AES-256 (CMK)", "AES-256 (CMK + HSM)"],
      ],
      [2000, 1800, 2200, 3360]
    ),

    // --- 51.2 Kubernetes Resources ---
    h2("51.2 Kubernetes Resources"),
    p("An EKS cluster (Kubernetes 1.29) runs MSEZ workloads with managed node groups. Terraform provisions the cluster, node groups, and all Kubernetes resources via the Kubernetes and Helm providers."),
    table(
      ["Resource", "Type", "Configuration", "Purpose"],
      [
        ["msez-api", "Deployment", "Rolling update (maxSurge: 1, maxUnavailable: 0)", "Primary API server"],
        ["msez-worker", "Deployment", "Rolling update, no external ports", "Background task processor"],
        ["Services", "ClusterIP / LoadBalancer", "Internal LB for inter-service; ALB for external", "Network routing"],
        ["HPA", "HorizontalPodAutoscaler", "CPU target 70%, custom Prometheus metrics (p99 latency)", "Auto-scaling"],
        ["PDB", "PodDisruptionBudget", "minAvailable: 1 per deployment", "Availability during disruptions"],
        ["ConfigMaps", "ConfigMap", "Jurisdiction packs, feature flags", "Runtime configuration"],
        ["Secrets", "ExternalSecret", "AWS Secrets Manager via external-secrets-operator", "Credential management"],
        ["Ingress", "Ingress (ALB)", "AWS ALB controller, WAF integration, TLS termination", "External traffic routing"],
        ["NetworkPolicy", "NetworkPolicy", "Deny-all default, explicit allow per service pair", "Pod-to-pod isolation"],
      ],
      [1600, 1800, 2800, 3160]
    ),

    h3("51.2.1 Node Group Sizing"),
    table(
      ["Profile", "Instance Type", "Min / Max Nodes", "Disk", "Labels"],
      [
        ["standard", "m6i.xlarge", "2 / 4", "100 GB gp3", "workload=msez, tier=standard"],
        ["enterprise", "m6i.2xlarge", "3 / 8", "200 GB gp3", "workload=msez, tier=enterprise"],
        ["sovereign-govos", "m6i.4xlarge", "4 / 16", "500 GB io2", "workload=msez, tier=sovereign"],
        ["gpu (AI Spine)", "g5.2xlarge", "1 / 4", "200 GB gp3", "workload=ai, accelerator=gpu"],
      ],
      [1600, 1600, 1600, 1600, 2960]
    ),
  ];
};
