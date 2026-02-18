# Performance

This document describes the performance characteristics of the MEZ Stack,
covering the Rust workspace, cryptographic primitives, API server, Python
PHOENIX layer, and the performance test harness.

---

## Overview

The MEZ Stack is a dual-language system designed for sovereign digital
infrastructure at nation-state scale. The Rust workspace provides the
production runtime -- compiled binaries with no garbage collector, zero-cost
abstractions, and type-level enforcement of correctness invariants. The Python
PHOENIX layer serves as the specification-grade test harness and CLI toolchain
for validation, artifact management, and compliance evaluation.

Performance-sensitive operations -- digest computation, signature verification,
Merkle Mountain Range construction, and content-addressed storage lookups --
are implemented in Rust with careful attention to allocation patterns, cache
locality, and algorithmic complexity. The Python layer is optimized for
correctness and auditability rather than raw throughput.

---

## Rust Workspace Performance

The workspace comprises 14 crates organized by domain responsibility:

| Crate | Role |
|-------|------|
| `mez-core` | Canonical serialization, content digests, compliance domains, domain-primitive newtypes |
| `mez-crypto` | Ed25519, MMR, CAS, SHA-256 digest computation |
| `mez-vc` | Verifiable Credential issuance and verification |
| `mez-state` | Entity and corridor state machines |
| `mez-tensor` | Compliance tensor (multi-domain evaluation) |
| `mez-zkp` | Zero-knowledge proof types (Phase 2 stubs) |
| `mez-pack` | Lawpack and regpack compilation |
| `mez-corridor` | Corridor lifecycle, receipt chains, fork resolution |
| `mez-agentic` | Agentic policy engine |
| `mez-arbitration` | Dispute lifecycle management |
| `mez-schema` | JSON Schema validation utilities |
| `mez-api` | Axum HTTP API server |
| `mez-cli` | CLI binary (replaces Python `tools/mez.py` monolith) |
| `mez-integration-tests` | Cross-crate integration tests |

### Build Characteristics

The workspace uses Rust edition 2021 with a minimum supported Rust version
of 1.75 and the v2 dependency resolver. A full workspace build compiles all
14 crates along with their dependencies. Key third-party dependencies include:

- **serde / serde_json** for serialization (BTreeMap-backed maps ensure
  deterministic key ordering)
- **sha2** for SHA-256 digest computation
- **ed25519-dalek** for Ed25519 signing and verification
- **axum / tokio / tower** for the async HTTP server
- **chrono** for timestamp handling
- **proptest** for property-based testing

Incremental builds during development benefit from Cargo's crate-level
compilation caching -- changing a leaf crate like `mez-tensor` does not
trigger recompilation of unrelated crates.

### Test Execution

The workspace contains over 2,580 tests across all 14 crates, including
unit tests, integration tests, and property-based tests via proptest. The
full test suite is designed to complete in under 10 seconds on modern
hardware, enabling rapid iteration during development.

Property-based tests in `mez-core` verify structural invariants of the
canonicalization pipeline (determinism, idempotency, key sorting, float
rejection, UTF-8 validity, and data round-trip preservation) across
randomly generated inputs.

### Binary Outputs

The workspace produces two binaries:

- **`mez-api`** -- the Axum HTTP server serving the five programmable
  primitives API. Compiled from `mez/crates/mez-api/src/main.rs`.
- **`mez`** (from `mez-cli`) -- the CLI tool for zone validation, lockfile
  generation, corridor management, CAS operations, and Ed25519/VC signing.

Both binaries are statically linked Rust executables with no runtime
dependency on a garbage collector, interpreter, or VM. Memory management
is handled at compile time through Rust's ownership model, eliminating
GC pause latency entirely.

### Memory Safety

Rust's ownership and borrowing system provides memory safety without runtime
overhead. The MEZ Stack makes extensive use of:

- **Newtype wrappers** (`CanonicalBytes`, `ContentDigest`, `Ed25519Signature`)
  that enforce correctness invariants at the type level. For example,
  `CanonicalBytes` has a private inner `Vec<u8>` that can only be constructed
  through the canonicalization pipeline, making "wrong serialization path"
  defects structurally impossible.
- **Zero-copy deserialization** where applicable via serde's borrow
  capabilities.
- **No unsafe code** in the application layer -- all unsafe operations are
  confined to vetted third-party crates (`sha2`, `ed25519-dalek`).

---

## Cryptographic Operations

### SHA-256 Digest Computation via CanonicalBytes

All digest computation in the stack flows through a single path:

```
Data -> CanonicalBytes::new() -> sha256_digest() -> ContentDigest
```

`CanonicalBytes::new()` applies the Momentum type coercion pipeline before
serialization:

1. Rejects floats (amounts must be strings or integers)
2. Normalizes RFC 3339 datetime strings to UTC with `Z` suffix, truncated
   to seconds
3. Sorts object keys lexicographically (via serde_json's BTreeMap)
4. Produces compact JSON with no whitespace

The `sha256_digest()` function accepts only `&CanonicalBytes` -- not raw
`&[u8]` -- enforcing at the type level that every digest was computed from
properly canonicalized data. This eliminates the canonicalization split defect
class (where two modules could produce different digests for identical data).

The SHA-256 implementation uses the `sha2` crate, which provides optimized
implementations leveraging hardware acceleration (SHA-NI instructions on
x86_64) when available.

### Ed25519 Signing and Verification

Ed25519 operations use the `ed25519-dalek` crate (v2) with the following
type-level safety guarantees:

- **Signing** (`SigningKey::sign`) accepts only `&CanonicalBytes`, ensuring
  the signed payload was produced by the JCS canonicalization pipeline. This
  prevents signature malleability from non-canonical serialization.
- **Verification** (`VerifyingKey::verify`) similarly requires `&CanonicalBytes`,
  ensuring verification is performed against properly canonicalized data.
- The `SigningKey` type intentionally does not implement `Serialize` to prevent
  accidental private key leakage. Its `Debug` implementation displays only the
  corresponding public key.

Ed25519 signing is deterministic -- the same key and message always produce
the same signature, which simplifies testing and audit.

### Merkle Mountain Range Operations

The MMR implementation (`mez-crypto/src/mmr.rs`) supports:

- **Append** -- O(log n) per leaf insertion, with peak merging
- **Root computation** -- O(k) where k is the number of peaks (bounded by
  log2(n))
- **Inclusion proof construction** -- O(log n) path steps
- **Inclusion proof verification** -- O(log n) hash computations
- **Incremental append** -- extends an existing peak set without replaying
  history

Domain separation is enforced at the hash level:
- Leaf hash: `SHA256(0x00 || leaf_bytes)`
- Node hash: `SHA256(0x01 || left_hash || right_hash)`

The stateful `MerkleMountainRange` wrapper maintains internal peak state
across sequential appends, avoiding redundant recomputation. Cross-language
compatibility with the Python `tools/mmr.py` implementation is verified by
shared test vectors (9-receipt and 17-receipt fixtures with known roots).

### Content-Addressed Storage Lookups

The `ContentAddressedStore` provides filesystem-backed CAS with integrity
verification:

- **Store**: canonicalize, compute digest, write to
  `{base_dir}/{type}/{digest}.json`. Idempotent -- existing artifacts are
  not overwritten.
- **Resolve**: read file, re-canonicalize content, recompute digest, verify
  against filename. Corruption or tampering is detected at read time.
- **List**: directory scan with filename validation (O(n) in the number of
  artifacts of that type).

The resolve path performs a full integrity check on every read, trading a
small amount of read latency for tamper detection. For high-throughput
scenarios, the PHOENIX layer's tiered cache infrastructure can be placed
in front of the CAS.

---

## API Server

The `mez-api` crate implements an Axum HTTP server with the following
architecture:

### Async I/O with Tokio Runtime

The server runs on the Tokio async runtime (`#[tokio::main]` with full
features enabled), providing:

- Non-blocking I/O for all network operations
- Multi-threaded work-stealing scheduler (Tokio's default)
- Efficient handling of concurrent connections without per-connection threads
- Graceful shutdown via Tokio's signal handling

The server binds to a configurable port (default 8080) using
`tokio::net::TcpListener` and serves requests via `axum::serve`.

### Route Groups

The API surface is organized into 8 route groups serving the five
programmable primitives plus auxiliary services:

| Prefix | Module | Primitive |
|--------|--------|-----------|
| `/v1/entities/*` | `routes::entities` | ENTITIES |
| `/v1/ownership/*` | `routes::ownership` | OWNERSHIP |
| `/v1/fiscal/*` | `routes::fiscal` | FISCAL |
| `/v1/identity/*` | `routes::identity` | IDENTITY |
| `/v1/consent/*` | `routes::consent` | CONSENT |
| `/v1/corridors/*` | `routes::corridors` | Corridors |
| `/v1/assets/*` | `routes::smart_assets` | Smart Assets |
| `/v1/regulator/*` | `routes::regulator` | Regulator |

### Middleware Stack

Requests pass through a layered Tower middleware stack:

```
TraceLayer -> MetricsMiddleware -> RateLimitMiddleware -> AuthMiddleware
```

- **TraceLayer** (tower-http): structured request/response tracing with
  configurable log levels via the `RUST_LOG` environment variable
- **MetricsMiddleware**: request counting and latency tracking
- **RateLimitMiddleware**: configurable rate limiting per client
- **AuthMiddleware**: bearer token authentication with scope claims

Health probes (`/health/liveness` and `/health/readiness`) are mounted
outside the authentication middleware so they remain accessible to
Kubernetes probe controllers without credentials.

### OpenAPI

Auto-generated OpenAPI 3.1 specification is available at `/openapi.json`
via utoipa derive macros, enabling client SDK generation and interactive
API documentation.

---

## Python PHOENIX Layer

The Python PHOENIX layer (`tools/phoenix/`, 14,363 lines across 18 modules)
implements the specification-grade compliance engine, migration sagas,
corridor bridge routing, watcher economy, and supporting infrastructure
(circuit breakers, caching, event bus, observability).

### Performance-Oriented Test Harness

The `tests/perf/` directory contains performance tests that are **skipped by
default** so that CI and normal local test runs stay fast. These tests are
intended as a regression guard and tuning aid, not as strict benchmarks.

#### Receipt Chain Verification

`test_receipt_chain_verification_perf.py` measures end-to-end receipt chain
verification throughput:

1. Generates a configurable number of corridor state receipts, each with:
   - JCS-canonicalized payload digests
   - Ed25519 signatures via real cryptographic operations (no mocking)
   - Chained `prev_root` / `next_root` linkage
2. Times the full chain verification pass (`_corridor_state_build_chain`)
3. Reports receipts-per-second throughput to stdout

The test uses real Ed25519 key generation, real JCS canonicalization, and
real signature verification to ensure the benchmark reflects actual
production code paths.

#### Watcher Attestation Comparison

`test_watcher_compare_scaling_perf.py` measures watcher attestation
comparison scaling:

1. Generates a configurable number of watcher attestation Verifiable
   Credentials, each signed by a distinct Ed25519 key
2. Times the `cmd_corridor_state_watcher_compare` operation
3. Reports VCs-per-second analysis throughput

### Environment Variables for Tuning

| Variable | Default | Description |
|----------|---------|-------------|
| `MEZ_RUN_PERF` | (unset) | Set to `1` to enable performance tests |
| `MEZ_PERF_RECEIPTS` | `10000` | Number of receipts to verify in the chain test |
| `MEZ_PERF_WATCHERS` | `100` | Number of watcher attestations to compare |
| `MEZ_PERF_BUDGET_MS` | (unset) | Optional hard time budget in milliseconds; test fails if exceeded |

---

## Running Performance Tests

### Rust Workspace Tests

```bash
# Run all workspace tests (typically completes in under 10 seconds)
cd mez && cargo test --workspace

# Run tests for a specific crate
cargo test -p mez-crypto

# Run tests with output (for debugging)
cargo test --workspace -- --nocapture

# Release-mode tests (optimized, for accurate performance measurement)
cargo test --workspace --release
```

### Python Performance Tests

Enable and run the performance test harness:

```bash
# Run all perf tests with default workload sizes
MEZ_RUN_PERF=1 pytest -q tests/perf/

# Receipt chain verification with 100,000 receipts
MEZ_RUN_PERF=1 MEZ_PERF_RECEIPTS=100000 pytest -q -k receipt_chain_verification_time -s

# Watcher comparison with 500 attestations
MEZ_RUN_PERF=1 MEZ_PERF_WATCHERS=500 pytest -q -k watcher_compare_scaling -s

# With a hard time budget (fail if verification exceeds 5 seconds)
MEZ_RUN_PERF=1 MEZ_PERF_BUDGET_MS=5000 pytest -q -k receipt_chain_verification_time -s
```

### Stable Benchmarking

For reproducible benchmark results:

1. Pin CPU frequency scaling (`cpupower frequency-set -g performance`)
2. Run on a quiet machine with minimal background load
3. Use release-mode compilation for Rust (`--release`)
4. Run multiple iterations and report the median
5. Disable Turbo Boost if present for consistent clock speeds

---

## Optimization Guidelines

### Production Deployment

**Rust binaries:**

- Compile with `--release` for full optimizations (LTO, codegen-units=1
  if binary size is acceptable)
- Set `RUST_LOG=info` or `RUST_LOG=warn` in production to reduce tracing
  overhead
- Configure the Tokio runtime thread count via `TOKIO_WORKER_THREADS` to
  match available CPU cores
- Use connection pooling for database connections (sqlx with
  `runtime-tokio-rustls` and `postgres` features)

**API server:**

- Place behind a reverse proxy (nginx, envoy) for TLS termination and
  connection management
- Configure rate limiting thresholds appropriate for expected traffic patterns
- Monitor the `/health/readiness` probe to detect degraded state before
  routing traffic

**Content-addressed storage:**

- For high-throughput CAS operations, place the `dist/artifacts/` directory
  on an SSD with low-latency filesystem access
- The integrity verification on every resolve operation is intentional for
  security; do not bypass it in production
- Consider the PHOENIX layer's tiered cache (LRU + TTL) for frequently
  accessed artifacts to reduce filesystem I/O

**Cryptographic operations:**

- Ed25519 signing and verification benefit from hardware that supports
  constant-time 64-bit multiplication
- SHA-256 performance scales with SHA-NI instruction support on x86_64
  (available on Intel Goldmont+ and AMD Zen+)
- Merkle Mountain Range operations are CPU-bound; for very large receipt
  chains (millions of entries), incremental append from a checkpoint avoids
  full-chain replay

### Python Layer

- The Python layer is designed for correctness and auditability, not
  throughput. Production request serving is the responsibility of the Rust
  API server
- For batch operations (mass validation, bulk artifact processing), consider
  parallelizing across zones using Python's `multiprocessing` module
- The circuit breaker and bulkhead patterns in `tools/phoenix/resilience.py`
  prevent cascading failures when downstream services are slow; tune their
  thresholds based on observed latency distributions
- Cache infrastructure in `tools/phoenix/cache.py` provides O(1) get/set
  with configurable LRU eviction and TTL expiration; size limits should be
  tuned based on available memory

### Monitoring

- Use structured logging (`tracing-subscriber` with JSON output) for
  production log aggregation
- The metrics middleware tracks request counts and latencies per route group
- Set up alerts on the `/health/readiness` probe transitioning to non-200
  status
- Monitor CAS integrity violations (logged as errors) -- any occurrence
  indicates potential tampering or storage corruption
