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

    // --- 50.1.1 Container Definitions ---
    h3("50.1.1 Container Definitions"),
    p("All application containers use multi-stage Docker builds. The build stage compiles Rust code with cargo build --release in a rust:1.77-slim image. The runtime stage copies only the compiled binary into an alpine:3.19 base image, producing final images under 50 MB. Each service defines health check commands, resource limits (CPU and memory), restart policies (unless-stopped), and explicit dependency ordering via depends_on with condition: service_healthy. Environment variables are injected from a shared .env file with service-specific overrides."),

    // --- 50.2 Database Initialization ---
    h2("50.2 Database Initialization"),
    p("PostgreSQL initialization is handled by SQLx migrations embedded in the msez-api binary. On first startup, the API server runs all pending migrations before accepting traffic. The migration system creates tables for corridor state, tensor snapshots, verifiable credential audit logs, agentic policy state, and audit event hash chains. A readiness probe on /healthz confirms that migrations have completed and the database connection pool is active. The postgres service mounts a named volume for data persistence across container restarts."),

    // --- 50.2.1 Database Schema: init-db.sql ---
    h3("50.2.1 Database Schema: init-db.sql"),
    p("The init-db.sql script (mounted into the postgres container's /docker-entrypoint-initdb.d/) creates seven databases to enforce domain isolation. Each database corresponds to a distinct bounded context within the SEZ Stack, preventing cross-domain query coupling and enabling independent backup and scaling policies."),
    table(
      ["Database", "Owner", "Purpose"],
      [
        ["corridor_state", "msez_app", "Corridor lifecycle state machines, receipt chains, fork resolution records, bilateral agreements, and netting ledgers. Primary write target for corridor operations."],
        ["tensor_snapshots", "msez_app", "Point-in-time snapshots of the compliance tensor (20 domains x N jurisdictions). Used for audit trail, historical compliance queries, and tensor diff computation."],
        ["vc_audit", "msez_app", "Verifiable credential audit log: every VC issued, verified, or revoked. Stores credential metadata, issuer, subject, proof type, and issuance timestamp. Does not store credential payloads (those live in CAS)."],
        ["agentic_policy", "msez_app", "Agentic trigger definitions, policy evaluation history, autonomous action logs. Stores the 20 trigger types x 5 domain configurations and their execution records."],
        ["audit_events", "msez_app", "Append-only audit event hash chain. Each record contains event metadata and the SHA-256 digest of the previous event, forming a tamper-evident log. Oldest and highest-volume table."],
        ["pack_registry", "msez_app", "Registry of installed lawpacks, regpacks, and licensepacks. Tracks pack version, jurisdiction, effective dates, and composition rules. Updated on pack installation or regulatory change."],
        ["migration_log", "msez_app", "Schema migration history across all databases. Records migration ID, applied timestamp, checksum, and execution duration. Used by SQLx to determine pending migrations on startup."],
      ],
      [1600, 1200, 6560]
    ),
    p_runs([bold("Isolation Rationale."), " Separate databases (rather than schemas within a single database) enable independent pg_dump/pg_restore for each domain, separate connection pool limits per domain in PgBouncer, and the ability to place high-write databases (audit_events, corridor_state) on faster storage while keeping lower-write databases (pack_registry, migration_log) on standard storage."]),

    // --- 50.3 Dockerfile Structure ---
    h2("50.3 Dockerfile Structure"),
    p("All application images (msez-api, msez-worker) use an identical multi-stage build pattern. The build is split into three stages to maximize layer caching and minimize final image size."),
    ...codeBlock(
`# Stage 1: Chef — dependency caching layer
FROM rust:1.77-slim AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: Builder — compile the workspace
FROM chef AS builder
COPY recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin msez-api

# Stage 3: Runtime — minimal production image
FROM alpine:3.19 AS runtime
RUN apk add --no-cache ca-certificates libgcc
COPY --from=builder /app/target/release/msez-api /usr/local/bin/msez-api
ENV RUST_LOG=info
EXPOSE 3100
HEALTHCHECK --interval=10s --timeout=3s --retries=3 \\
  CMD wget -qO- http://localhost:3100/healthz || exit 1
ENTRYPOINT ["/usr/local/bin/msez-api"]`
    ),
    p_runs([bold("Build Caching."), " The cargo-chef pattern (Stage 1) pre-computes a dependency recipe from Cargo.toml and Cargo.lock, then builds all dependencies in a cached layer before copying source code. This means source-only changes do not re-download or re-compile dependencies, reducing incremental build times from 10+ minutes to under 2 minutes."]),
    p_runs([bold("Image Size."), " The final runtime image contains only the statically-linked binary, CA certificates (for TLS to Mass APIs and Vault), and libgcc (required by the Rust allocator on musl). The resulting image is typically 35-45 MB, compared to 1.5+ GB for the build stage. No compiler, source code, or build artifacts are present in the production image."]),
    p_runs([bold("Security Hardening."), " The runtime container runs as a non-root user (uid 1000). No shell is included in the final image beyond what Alpine provides. The HEALTHCHECK directive enables Docker and orchestrators to detect unresponsive containers and trigger automatic restarts. Resource limits (CPU and memory) are enforced at the Docker Compose level, not in the Dockerfile, to allow per-profile tuning."]),
  ];
};
