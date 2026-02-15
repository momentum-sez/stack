const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter46() {
  return [
    ...partHeading("PART XVI: SECURITY AND HARDENING"),

    chapterHeading("Chapter 46: Security Architecture"),

    // --- 46.1 Threat Model ---
    h2("46.1 Threat Model"),
    p("The security architecture addresses five threat categories: external adversaries (network-level attacks, API abuse, credential theft), insider threats (compromised administrators, rogue watchers, collusion), state-level adversaries (jurisdiction-level censorship, forced key disclosure, traffic analysis), cryptographic threats (quantum computing, side-channel attacks, implementation flaws), and systemic threats (cascade failures, consensus deadlocks, economic attacks on watcher bonds)."),
    table(
      ["Threat", "Mitigation", "Detection"],
      [
        ["Credential theft", "Ed25519 with hardware key support, key rotation policy", "Anomalous signing patterns, geographic impossibility"],
        ["API abuse", "Rate limiting, authentication-first middleware, input validation", "Request pattern analysis, threshold alerts"],
        ["Man-in-the-middle", "TLS 1.3 mandatory, certificate pinning, mutual TLS for inter-service", "Certificate transparency monitoring"],
        ["Replay attacks", "Nonce-based request signing, monotonic sequence numbers", "Duplicate nonce detection, sequence gap alerts"],
        ["Watcher collusion", "Threshold signatures (t-of-n), economic bonding, slashing", "Bond monitoring, attestation divergence analysis"],
        ["Sanctions evasion", "Real-time sanctions screening, compliance tensor enforcement", "Transaction pattern analysis, network graph analysis"],
        ["Data exfiltration", "Encryption at rest, field-level encryption, access logging", "Access pattern anomalies, bulk query detection"],
        ["Consensus manipulation", "DAG-based consensus with jurisdictional awareness, finality gadget", "Fork detection, attestation timing analysis"],
        ["Side-channel attacks", "Constant-time comparison (subtle crate), key zeroization", "Timing analysis on cryptographic operations"],
        ["Quantum threats", "Post-quantum algorithm readiness, hybrid signing scheme design", "Cryptographic agility monitoring, algorithm deprecation tracking"],
      ],
      [2400, 3600, 3360]
    ),
    spacer(),

    p_runs([bold("Defense in Depth."), " Every security boundary implements multiple independent controls. Authentication is enforced before rate limiting. Cryptographic operations use constant-time implementations. Key material is zeroized on drop. All inter-service communication uses mutual TLS. No single control failure compromises the system."]),
    p_runs([bold("Byzantine Fault Tolerance."), " The watcher economy assumes up to f Byzantine watchers in a set of 3f+1. Threshold signatures require t-of-n attestations where t > 2n/3. Economic bonds ensure that collusion costs exceed potential gains. Slashing conditions are enforced automatically by the corridor state machine."]),

    // --- 46.2 Security Boundaries ---
    h2("46.2 Security Boundaries"),
    table(
      ["Boundary", "Scope", "Guarantees", "Enforcement Mechanism"],
      [
        ["API Gateway", "External clients", "Authentication, authorization, rate limiting", "Axum middleware stack, JWT/bearer token validation"],
        ["Mass Bridge", "Mass API integration", "Type-safe requests, response validation", "msez-mass-client with typed HTTP methods"],
        ["Cryptographic", "Key material, signatures", "Key isolation, zeroization, constant-time ops", "msez-crypto crate, Zeroize trait, subtle crate"],
        ["Corridor", "Cross-border operations", "Bilateral verification, receipt integrity", "MMR proofs, watcher attestations, threshold sigs"],
        ["Compliance", "Regulatory enforcement", "Tensor evaluation, pack verification", "msez-tensor + msez-pack composition"],
        ["Persistence", "State durability", "Write-through caching, migration safety", "SQLx + PgPool, transactional writes"],
      ],
      [2000, 1600, 3000, 2760]
    ),
    spacer(),

    // --- 46.3 Audit System ---
    h2("46.3 Audit System"),
    p("Every state-changing operation in the SEZ Stack produces an AuditEvent. Audit events are structured records containing: event_id (unique identifier), timestamp (UTC with nanosecond precision), actor (authenticated principal), action (the operation performed), target (the resource affected), outcome (success or failure with error context), and previous_event_digest (SHA-256 hash of the prior audit event, forming a hash chain). The hash chain ensures that audit log tampering is detectable: any modification to a historical event invalidates all subsequent digests. Audit events are persisted to Postgres with write-through semantics and replicated to an append-only external store for regulatory retention."),
  ];
};
