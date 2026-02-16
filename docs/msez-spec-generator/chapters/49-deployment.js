const {
  partHeading, chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table, bulletItem
} = require("../lib/primitives");

module.exports = function build_chapter49() {
  return [
    ...partHeading("PART XVII: DEPLOYMENT AND OPERATIONS"),

    chapterHeading("Chapter 49: Deployment Architecture"),

    // --- 49.1 Infrastructure Requirements ---
    h2("49.1 Infrastructure Requirements"),
    table(
      ["Component", "Minimum", "Recommended"],
      [
        ["Compute", "4 vCPU, 16 GB RAM", "8 vCPU, 32 GB RAM"],
        ["Storage", "100 GB SSD", "500 GB NVMe SSD"],
        ["Network", "100 Mbps", "1 Gbps"],
        ["Database", "PostgreSQL 15+", "PostgreSQL 16 with pgvector"],
        ["Container Runtime", "Docker 24+", "containerd 1.7+ with Kubernetes 1.29+"],
      ],
      [2000, 3200, 4160]
    ),

    // --- 49.2 Deployment Profiles ---
    h2("49.2 Deployment Profiles"),
    table(
      ["Profile", "Services", "Resources", "Use Case"],
      [
        ["minimal", "Core MSEZ + single jurisdiction", "4 vCPU / 16 GB", "Development, testing"],
        ["standard", "Full MSEZ + 3 jurisdictions + corridors", "8 vCPU / 32 GB", "Single-zone production"],
        ["enterprise", "Full MSEZ + 10+ jurisdictions + full corridors", "32 vCPU / 128 GB", "Multi-zone production"],
        ["sovereign-govos", "Full MSEZ + GovOS + AI + national integration", "64+ vCPU / 256+ GB + GPU", "National deployment"],
      ],
      [1600, 3000, 2200, 2560]
    ),

    // --- 49.2.1 Rust Binary Deployment ---
    h3("49.2.1 Rust Binary Deployment"),
    p("The msez CLI is a single statically-linked binary compiled from the msez-cli crate. No runtime dependencies beyond libc. Container images use Alpine Linux with the msez binary, producing images under 50 MB."),
    ...codeBlock(
`# Build the release binary
cargo build --release --bin msez

# Binary is at: target/release/msez
# Deploy directly or via container:
FROM alpine:3.19
COPY target/release/msez /usr/local/bin/msez
ENTRYPOINT ["/usr/local/bin/msez"]`
    ),

    // --- 49.2.2 Deployment Topology ---
    h3("49.2.2 Deployment Topology"),
    p("Each deployment profile defines a specific service topology. The topology determines how the msez-api server, PostgreSQL, Redis, observability stack, and external integrations connect. All profiles share the same binary artifacts; the difference is in replica count, resource allocation, and network segmentation."),
    table(
      ["Profile", "API Instances", "Database", "Cache", "Observability", "Network"],
      [
        ["minimal", "1 msez-api", "1 PostgreSQL (shared)", "1 Redis", "Logs to stdout only", "Single Docker network, no TLS between services"],
        ["standard", "2 msez-api behind nginx", "1 PostgreSQL with streaming replica", "1 Redis with AOF persistence", "Prometheus + Grafana + Loki", "Internal mTLS, external TLS via nginx"],
        ["enterprise", "4+ msez-api behind load balancer", "PostgreSQL primary + 2 read replicas + pgBouncer", "Redis Sentinel (3 nodes)", "Full stack: Prometheus, Grafana, Loki, Tempo", "Network segmentation: DMZ, app tier, data tier"],
        ["sovereign-govos", "8+ msez-api across availability zones", "PostgreSQL HA cluster (Patroni) + dedicated analytics replica", "Redis Cluster (6 nodes)", "Full stack + external SIEM integration", "Air-gapped option, HSM integration, dedicated VPC per tier"],
      ],
      [1200, 1400, 2000, 1600, 1800, 1360]
    ),
    p_runs([bold("Service Connectivity."), " In all profiles, msez-api is the sole ingress point for external traffic. It connects to PostgreSQL for state persistence, Redis for caching and rate limiting, and Mass APIs (organization-info, treasury-info, consent, investment-info) via msez-mass-client over HTTPS. The worker service (msez-worker) shares the same database and Redis instances but has no external-facing ports. Vault provides secrets to all application services at startup via environment variable injection or the Vault agent sidecar."]),

    // --- 49.3 Resource Scaling Guidelines ---
    h2("49.3 Resource Scaling Guidelines"),
    p("Resource allocation scales with three primary drivers:"),
    bulletItem("Jurisdictional breadth: number of active jurisdictions and their regulatory complexity"),
    bulletItem("Corridor throughput: transactions per second across all active corridors"),
    bulletItem("Credential volume: VCs issued and verified per day"),
    table(
      ["Scaling Dimension", "Metric", "Threshold", "Action"],
      [
        ["API compute", "p99 latency > 200ms", "Sustained 5 minutes", "Add msez-api replica; each replica handles approximately 500 req/s"],
        ["Database connections", "Active connections > 80% of pool", "Sustained 1 minute", "Increase PgBouncer pool size or add read replica for query offload"],
        ["Database storage", "Disk usage > 70%", "Trending to 80% within 7 days", "Expand volume; audit event and tensor snapshot tables grow fastest"],
        ["Redis memory", "Memory usage > 75%", "Sustained 10 minutes", "Increase maxmemory or add Redis node; rate-limit keys are highest cardinality"],
        ["Corridor throughput", "Corridor receipt processing backlog > 1000", "Sustained 5 minutes", "Add msez-worker replica; workers are stateless and horizontally scalable"],
        ["Tensor evaluation", "Tensor evaluation time > 50ms per entity", "Sustained across 100 evaluations", "Review jurisdiction pack complexity; consider tensor snapshot caching"],
        ["Certificate/VC issuance", "VC signing queue depth > 500", "Sustained 2 minutes", "Add msez-api replica or optimize Ed25519 batch verification"],
      ],
      [1600, 2200, 2200, 3360]
    ),
    p_runs([bold("Vertical vs. Horizontal."), " The msez-api and msez-worker services scale horizontally (add replicas). PostgreSQL scales vertically first (more CPU, RAM, faster storage) and then horizontally via read replicas. Redis scales vertically for single-instance profiles and horizontally via Cluster mode for enterprise and sovereign-govos. The compliance tensor evaluation is CPU-bound and benefits most from vertical scaling (faster cores), while corridor receipt processing is I/O-bound and benefits from horizontal scaling (more workers)."]),
  ];
};
