# MEZ Stack — Multi-Stage Rust Build
# Produces two binaries: mez-api (Axum HTTP server) and mez-cli (validation/management tool)
#
# Build:   docker build -t mez-api .
# Run:     docker run -p 8080:8080 mez-api
# CLI:     docker run --rm mez-api mez-cli validate --all-modules

# ============================================
# Stage 1: Build
# ============================================
FROM rust:1.77-bookworm AS builder

WORKDIR /app

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build release binaries
RUN cargo build --release -p mez-api -p mez-cli

# ============================================
# Stage 2: Runtime
# ============================================
FROM debian:bookworm-slim

LABEL org.opencontainers.image.title="MEZ API"
LABEL org.opencontainers.image.description="Momentum EZ Stack — Axum API server and CLI toolchain"
LABEL org.opencontainers.image.vendor="Momentum"
LABEL org.opencontainers.image.version="0.4.44"
LABEL org.opencontainers.image.source="https://github.com/momentum-ez/stack"

# Install minimal runtime dependencies (TLS certs for HTTPS, libpq for PostgreSQL)
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        libpq5 \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r mez && useradd -r -g mez -d /app -s /sbin/nologin mez

# Copy binaries from builder
COPY --from=builder /app/target/release/mez-api /usr/local/bin/
COPY --from=builder /app/target/release/mez-cli /usr/local/bin/

# Copy data files needed at runtime (module descriptors, schemas, jurisdictions)
COPY modules/ /app/modules/
COPY schemas/ /app/schemas/
COPY jurisdictions/ /app/jurisdictions/

# Create data directories
RUN mkdir -p /app/data /app/config && chown -R mez:mez /app

WORKDIR /app
USER mez

# Expose API port
EXPOSE 8080

# Health check against the liveness endpoint
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health/liveness || exit 1

# Default: run the API server
CMD ["mez-api"]
