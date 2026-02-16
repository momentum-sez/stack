const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  table, codeBlock
} = require("../lib/primitives");

module.exports = function build_chapter47() {
  return [
    chapterHeading("Chapter 47: Production Hardening"),

    // --- 47.1 Validation Framework ---
    h2("47.1 Validation Framework"),
    p("The validation framework enforces structural and semantic correctness at every system boundary. All external inputs are validated against JSON Schemas (116 schemas in msez-schema) before processing. Internal types use newtype wrappers with validation on construction, ensuring that invalid states are unrepresentable."),
    table(
      ["Boundary", "Validation Method", "Error Handling", "Coverage"],
      [
        ["API request body", "JSON Schema (msez-schema, 116 schemas)", "400 with field-level diagnostic", "All POST/PUT endpoints"],
        ["Path/query params", "Newtype parsing with FromStr", "400 with expected format", "All parameterized routes"],
        ["Mass API responses", "Typed deserialization (serde)", "502 with upstream error context", "All msez-mass-client calls"],
        ["Pack content", "Schema + digest verification", "PackError with content hash mismatch", "All pack import/load paths"],
        ["Corridor state transitions", "FSM guard predicates", "409 with current state and attempted transition", "All corridor write operations"],
        ["Credential payloads", "W3C VC schema + signature verification", "CredentialError with verification failure reason", "All VC issuance and verification"],
      ],
      [2000, 2800, 2400, 2160]
    ),

    // --- 47.2 Thread Safety ---
    h2("47.2 Thread Safety"),
    p("All shared state uses parking_lot::RwLock, which is non-poisonable and provides deterministic behavior under contention. The Rust type system enforces Send and Sync bounds at compile time, eliminating data races by construction."),
    table(
      ["Pattern", "Mechanism", "Guarantee"],
      [
        ["Request concurrency", "Axum tower middleware stack; no shared mutable state per request", "Requests processed concurrently without data races"],
        ["Shared app state", "Arc<RwLock<T>> via typed Axum extractors", "Lock acquisition always scoped; deadlock-free by construction"],
        ["Background workers", "Tokio tasks with message passing (mpsc channels)", "No shared mutable state between worker tasks"],
        ["Database access", "SQLx connection pool with async checkout", "Pool manages contention; no application-level locking"],
        ["Cache access", "Redis client with connection pooling", "Atomic Redis operations; no application-level locks"],
      ],
      [2200, 3800, 3360]
    ),

    // --- 47.3 Cryptographic Utilities ---
    h2("47.3 Cryptographic Hardening"),
    table(
      ["Measure", "Implementation", "Crate", "Threat Mitigated"],
      [
        ["Key zeroization", "Zeroize + ZeroizeOnDrop on Ed25519 SigningKey", "msez-crypto", "Key material persistence in memory after use"],
        ["Constant-time comparison", "subtle::ConstantTimeEq for all secret comparisons", "msez-api auth", "Timing side-channel on bearer token validation"],
        ["Canonical hashing", "CanonicalBytes::new() → SHA-256 for all digests", "msez-core", "Non-deterministic serialization producing different hashes"],
        ["Secret redaction", "SecretString with [REDACTED] Debug/Display", "msez-api", "Secret leakage in log output"],
        ["Auth token lifecycle", "Zeroizing<String> from env read to process shutdown", "msez-mass-client", "Token persistence in memory after config load"],
        ["MMR integrity", "Append-only Merkle Mountain Range for receipt chains", "msez-crypto", "Receipt chain tampering or reordering"],
      ],
      [2000, 2800, 1600, 2960]
    ),

    // --- 47.4 Rust Security Guarantees ---
    h2("47.4 Rust Security Guarantees"),
    table(
      ["Guarantee", "Mechanism", "Risk Eliminated"],
      [
        ["Memory Safety", "Ownership model, borrow checker", "Buffer overflows, use-after-free, dangling pointers"],
        ["Thread Safety", "Send/Sync traits, no shared mutable state", "Data races, TOCTOU bugs"],
        ["Type Safety", "Algebraic types, exhaustive match", "Type confusion, null pointer dereference"],
        ["Error Handling", "Result<T, E>, no exceptions", "Unhandled exceptions, silent failures"],
        ["No GC Pauses", "Deterministic destruction", "Latency spikes during garbage collection"],
        ["No Unsafe", "Application code policy (zero unsafe blocks)", "Undefined behavior from unsafe operations"],
      ],
      [1800, 3200, 4360]
    ),
  ];
};
