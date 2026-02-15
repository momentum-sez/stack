const {
  chapterHeading, h2,
  p,
  codeBlock, spacer
} = require("../lib/primitives");

module.exports = function build_chapter52() {
  return [
    chapterHeading("Chapter 52: One-Click Deployment (v0.4.44)"),

    // --- 52.1 Deployment Steps ---
    h2("52.1 Deployment Steps"),
    p("A single shell script transforms zone configuration into running infrastructure:"),
    ...codeBlock(
`# 1. Initialize a new zone with jurisdiction profile
msez init --zone my-zone --profile digital-financial-center --jurisdiction AE-ADGM

# 2. Import jurisdiction packs (lawpack, regpack, licensepack)
msez pack import --jurisdiction AE-ADGM --all

# 3. Deploy infrastructure (Docker or Kubernetes)
msez deploy --target docker    # Local / single-server
msez deploy --target k8s       # Kubernetes cluster

# 4. Verify deployment health and compliance readiness
msez verify --zone my-zone --checks health,compliance,connectivity

# 5. Activate corridors for cross-border operations
msez corridor activate --from AE-ADGM --to PK-PSEZ --corridor-type bilateral`
    ),
    spacer(),

    // --- 52.2 AWS Deployment ---
    h2("52.2 AWS Deployment"),
    p("For AWS deployments, the one-click script wraps Terraform with pre-configured modules. Running msez deploy --target aws --region me-south-1 provisions the full infrastructure stack (VPC, RDS, ElastiCache, EKS, S3, CloudFront) in approximately 18 minutes. The script validates AWS credentials, checks resource quotas, applies Terraform with auto-approve, waits for EKS node readiness, deploys MSEZ workloads via Helm, runs the verification suite, and outputs the API endpoint URL. Rollback on failure is automatic: the script captures the Terraform state before apply and reverts on any provisioning error. Total deploy time including pack import and corridor activation is under 25 minutes for a standard profile deployment."),
  ];
};
