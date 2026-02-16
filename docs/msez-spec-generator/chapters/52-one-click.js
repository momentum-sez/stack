const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter52() {
  return [
    chapterHeading("Chapter 52: One-Click Deployment (v0.4.44)"),

    // --- 52.1 Deployment Steps ---
    h2("52.1 Deployment Steps"),
    p("The msez CLI transforms zone configuration into running infrastructure through a five-step pipeline. Each step is idempotent and can be re-run safely."),
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

    h3("52.1.1 Step Details"),
    table(
      ["Step", "Command", "Duration", "Outputs", "Rollback"],
      [
        ["1. Init", "msez init", "< 5s", "zone.yaml, stack.lock", "Delete zone directory"],
        ["2. Pack Import", "msez pack import", "10–30s", "Lawpack, regpack, licensepack in CAS", "msez pack remove"],
        ["3. Deploy", "msez deploy", "2–18 min", "Running services, health endpoints", "msez deploy --destroy"],
        ["4. Verify", "msez verify", "30–60s", "Verification report (JSON)", "No state change"],
        ["5. Corridor", "msez corridor activate", "< 10s", "Corridor state in DRAFT", "msez corridor deactivate"],
      ],
      [1200, 1600, 1200, 2400, 2960]
    ),
    spacer(),

    // --- 52.2 Deployment Targets ---
    h2("52.2 Deployment Targets"),
    table(
      ["Target", "Command Flag", "Provisioning Time", "Prerequisites", "Use Case"],
      [
        ["Docker Compose", "--target docker", "~2 minutes", "Docker 24+, 16 GB RAM", "Development, testing, single-server production"],
        ["Kubernetes", "--target k8s", "~5 minutes", "kubectl configured, cluster running", "Multi-node production with existing cluster"],
        ["AWS", "--target aws", "~18 minutes", "AWS credentials, Terraform installed", "Full cloud deployment (VPC, EKS, RDS, S3)"],
        ["Bare Metal", "--target bare-metal", "~3 minutes", "SSH access, systemd", "Air-gapped sovereign deployments"],
      ],
      [1600, 1600, 1600, 2200, 2360]
    ),
    spacer(),

    h3("52.2.1 AWS Deployment Pipeline"),
    p("Running msez deploy --target aws --region me-south-1 executes the following pipeline: validate AWS credentials and check resource quotas, apply Terraform with auto-approve, wait for EKS node readiness, deploy MSEZ workloads via Helm, run the verification suite, and output the API endpoint URL. Rollback on failure is automatic: the script captures Terraform state before apply and reverts on any provisioning error."),
    table(
      ["Phase", "Action", "Duration", "Failure Behavior"],
      [
        ["1. Validate", "Check AWS credentials, resource quotas, region availability", "< 10s", "Abort with diagnostic message"],
        ["2. Provision", "Terraform apply: VPC, RDS, ElastiCache, EKS, S3", "~15 min", "Terraform destroy on failure"],
        ["3. Deploy", "Helm install: msez-api, msez-worker, monitoring stack", "~2 min", "Helm rollback"],
        ["4. Verify", "Health checks, connectivity tests, compliance readiness", "~1 min", "Report failures, do not tear down"],
        ["5. Output", "Print API endpoint, Grafana URL, kubectl context", "< 5s", "N/A"],
      ],
      [1200, 3200, 1200, 3760]
    ),
    spacer(),
  ];
};
