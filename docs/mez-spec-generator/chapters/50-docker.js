const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter50() {
  return [
    chapterHeading("Chapter 50: Docker Infrastructure (v0.4.44)"),

    // --- 50.1 Service Architecture ---
    h2("50.1 Service Architecture"),
    p("Docker Compose orchestrates four services with dependency ordering and health checks. The architecture consolidates all five primitive API services into a single mez-api binary (Axum), with PostgreSQL for persistence and Prometheus/Grafana for observability. This replaces the prior nine-service Python layout where individual services could not start because their CLI subcommands did not exist."),
    table(
      ["Service", "Image", "Port", "Function"],
      [
        ["mez-api", "Built from deploy/docker/Dockerfile", "8080", "Single Axum HTTP server exposing all endpoints: corridor operations, smart asset lifecycle, compliance evaluation, credentials, identity, tax, GovOS, settlement, agentic, and regulator console"],
        ["postgres", "postgres:16-alpine", "5432", "Primary data store with SQLx runtime migrations. Seven databases for domain isolation (corridor_state, tensor_snapshots, vc_audit, agentic_policy, audit_events, pack_registry, migration_log)"],
        ["prometheus", "prom/prometheus:v2.51.0", "9090", "Metrics collection from mez-api /metrics endpoint. 30-day retention with lifecycle management enabled"],
        ["grafana", "grafana/grafana:10.4.1", "3000", "Monitoring dashboards and alerting. Admin credentials injected via environment variables (no defaults)"],
      ],
      [1800, 2800, 800, 4960]
    ),

    // --- 50.1.1 Credential Security ---
    h3("50.1.1 Credential Security"),
    p("No default passwords exist in the compose file. POSTGRES_PASSWORD and GRAFANA_PASSWORD are required environment variables that must be set before deployment. Docker Compose will refuse to start if they are unset, using the ${VAR:?message} syntax for fail-fast validation. JWT signing keys are mounted from a volume rather than injected as environment strings."),

    // --- 50.2 Database Initialization ---
    h2("50.2 Database Initialization"),
    p("PostgreSQL initialization is handled by an init-db.sql script mounted into the postgres container\u2019s /docker-entrypoint-initdb.d/ directory. This script creates seven databases to enforce domain isolation. Runtime schema migrations are handled by SQLx embedded in the mez-api binary, which runs all pending migrations before accepting traffic. A health check on /health/liveness confirms that migrations have completed and the database connection pool is active."),

    h3("50.2.1 Database Schema"),
    table(
      ["Database", "Purpose"],
      [
        ["corridor_state", "Corridor lifecycle state machines, receipt chains, fork resolution records, bilateral agreements, and netting ledgers"],
        ["tensor_snapshots", "Point-in-time compliance tensor snapshots (20 domains \u00d7 N jurisdictions) for audit trail and historical queries"],
        ["vc_audit", "Verifiable credential audit log: issuance, verification, and revocation events (credential payloads live in CAS)"],
        ["agentic_policy", "Agentic trigger definitions, policy evaluation history, and autonomous action execution records"],
        ["audit_events", "Append-only audit event hash chain with SHA-256 linkage forming a tamper-evident log"],
        ["pack_registry", "Registry of installed lawpacks, regpacks, and licensepacks with version, jurisdiction, and effective dates"],
        ["migration_log", "Schema migration history used by SQLx to determine pending migrations on startup"],
      ],
      [2000, 7360]
    ),

    // --- 50.3 Dockerfile Structure ---
    h2("50.3 Dockerfile Structure"),
    p("The Dockerfile uses a two-stage build: compilation in a rust:1.77-bookworm image and runtime on debian:bookworm-slim. Both the mez-api server and mez-cli tool are compiled and included in the final image."),
    ...codeBlock(
`# Stage 1: Build — compile the workspace
FROM rust:1.77-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
RUN cargo build --release -p mez-api -p mez-cli

# Stage 2: Runtime — minimal production image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \\
    ca-certificates libssl3 libpq5 curl && rm -rf /var/lib/apt/lists/*
RUN groupadd -r mez && useradd -r -g mez -d /app -s /sbin/nologin mez
COPY --from=builder /app/target/release/mez-api /usr/local/bin/
COPY --from=builder /app/target/release/mez-cli /usr/local/bin/
COPY modules/ /app/modules/
COPY schemas/ /app/schemas/
COPY jurisdictions/ /app/jurisdictions/
RUN mkdir -p /app/data /app/config && chown -R mez:mez /app
WORKDIR /app
USER mez
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \\
    CMD curl -f http://localhost:8080/health/liveness || exit 1
CMD ["mez-api"]`
    ),
    p_runs([bold("Layer Caching. "), "Cargo manifests are copied before source code, allowing Docker to cache dependency compilation across source-only changes. Future optimization with cargo-chef can further reduce incremental build times."]),
    p_runs([bold("Runtime Dependencies. "), "The final image includes CA certificates (for TLS to Mass APIs), libssl3 and libpq5 (for PostgreSQL via SQLx), and curl (for health check probes). Module descriptors, JSON schemas, and jurisdiction templates are copied from the repository for runtime validation."]),
    p_runs([bold("Security Hardening. "), "The runtime container runs as a non-root user (mez). The HEALTHCHECK directive uses curl against /health/liveness to detect unresponsive containers. Resource limits are enforced at the Docker Compose level to allow per-deployment tuning."]),
  ];
};
