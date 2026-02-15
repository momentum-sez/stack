const {
  chapterHeading, h2,
  p,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter47() {
  return [
    chapterHeading("Chapter 47: Production Hardening"),

    // --- 47.1 Validation Framework ---
    h2("47.1 Validation Framework"),
    p("The validation framework enforces structural and semantic correctness at every system boundary. All external inputs are validated against JSON Schemas (116 schemas in msez-schema) before processing. Internal types use newtype wrappers with validation on construction, ensuring that invalid states are unrepresentable. Validation errors carry full diagnostic context: the field that failed, the constraint that was violated, the value that was provided, and a human-readable explanation. This eliminates silent data corruption and provides actionable error messages to API consumers."),

    // --- 47.2 Thread Safety ---
    h2("47.2 Thread Safety"),
    p("All shared state uses parking_lot::RwLock, which is non-poisonable and provides deterministic behavior under contention. The Rust type system enforces Send and Sync bounds at compile time, eliminating data races by construction. The API server uses Axum's tower-based middleware stack, which processes requests concurrently without shared mutable state. State that must be shared across request handlers is wrapped in Arc<RwLock<T>> and accessed through typed extractors, ensuring that lock acquisition is always scoped and deadlock-free."),

    // --- 47.3 Cryptographic Utilities ---
    h2("47.3 Cryptographic Utilities"),
    p("Cryptographic operations are centralized in the msez-crypto crate. Ed25519 signing keys implement the Zeroize trait and are scrubbed from memory on drop. All signature verification uses constant-time comparison via the subtle crate, preventing timing side-channel attacks. SHA-256 digests are computed exclusively through the CanonicalBytes::new() path, ensuring consistent serialization before hashing. The Merkle Mountain Range (MMR) implementation provides append-only authenticated data structures for receipt chains. Content-addressable storage (CAS) enables deduplication and integrity verification for all persisted artifacts."),

    // --- 47.4 Rust Security Guarantees ---
    h2("47.4 Rust Security Guarantees"),
    table(
      ["Guarantee", "Mechanism", "Python-era Risk Eliminated"],
      [
        ["Memory Safety", "Ownership model, borrow checker", "Buffer overflows, use-after-free, dangling pointers"],
        ["Thread Safety", "Send/Sync traits, no shared mutable state", "Data races, TOCTOU bugs"],
        ["Type Safety", "Algebraic types, exhaustive match", "Type confusion, null pointer dereference"],
        ["Error Handling", "Result<T, E>, no exceptions", "Unhandled exceptions, silent failures"],
        ["No GC Pauses", "Deterministic destruction", "Latency spikes during garbage collection"],
        ["No Unsafe", "Application code policy", "Undefined behavior from unsafe operations"],
      ],
      [1800, 3200, 4360]
    ),
    spacer(),
  ];
};
