const {
  chapterHeading, h2,
  p,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter50() {
  return [
    chapterHeading("Chapter 50: Docker Infrastructure (v0.4.44)"),

    // --- 50.1 Service Architecture ---
    h2("50.1 Service Architecture"),
    p("Docker Compose orchestrates twelve services with dependency ordering and health checks:"),
    table(
      ["Service", "Image", "Port", "Function"],
      [
        ["msez-api", "msez/api:0.4.44", "3100", "Axum HTTP server, primary API gateway"],
        ["msez-worker", "msez/worker:0.4.44", "—", "Background task processor, agentic triggers"],
        ["postgres", "postgres:16-alpine", "5432", "Primary database, corridor state, audit log"],
        ["redis", "redis:7-alpine", "6379", "Cache layer, rate limiting, session store"],
        ["nginx", "nginx:1.25-alpine", "443/80", "TLS termination, reverse proxy, static assets"],
        ["prometheus", "prom/prometheus:v2.50", "9090", "Metrics collection and time-series storage"],
        ["grafana", "grafana/grafana:10.3", "3000", "Monitoring dashboards and alerting"],
        ["loki", "grafana/loki:2.9", "3100", "Log aggregation and querying"],
        ["tempo", "grafana/tempo:2.3", "4317", "Distributed tracing backend"],
        ["minio", "minio/minio:latest", "9000", "S3-compatible object storage for CAS"],
        ["vault", "hashicorp/vault:1.15", "8200", "Secrets management, key storage, PKI"],
        ["watchtower", "containrrr/watchtower:latest", "—", "Automated container image updates"],
      ],
      [1800, 2200, 800, 4560]
    ),
    spacer(),

    // --- 50.2 Container Definitions ---
    h2("50.2 Container Definitions"),
    p("All application containers use multi-stage Docker builds. The build stage compiles Rust code with cargo build --release in a rust:1.77-slim image. The runtime stage copies only the compiled binary into an alpine:3.19 base image, producing final images under 50 MB. Each service defines health check commands, resource limits (CPU and memory), restart policies (unless-stopped), and explicit dependency ordering via depends_on with condition: service_healthy. Environment variables are injected from a shared .env file with service-specific overrides."),

    // --- 50.3 Database Initialization ---
    h2("50.3 Database Initialization"),
    p("PostgreSQL initialization is handled by SQLx migrations embedded in the msez-api binary. On first startup, the API server runs all pending migrations before accepting traffic. The migration system creates tables for corridor state, tensor snapshots, verifiable credential audit logs, agentic policy state, and audit event hash chains. A readiness probe on /healthz confirms that migrations have completed and the database connection pool is active. The postgres service mounts a named volume for data persistence across container restarts."),
  ];
};
