const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  table, bulletItem
} = require("../lib/primitives");

module.exports = function build_chapter46() {
  return [
    ...partHeading("PART XVI: SECURITY AND HARDENING"),

    chapterHeading("Chapter 46: Security Architecture"),

    // --- 46.1 Threat Model ---
    h2("46.1 Threat Model"),
    p("The security architecture addresses five threat categories:"),
    bulletItem("External adversaries: network-level attacks, API abuse, credential theft"),
    bulletItem("Insider threats: compromised administrators, rogue watchers, collusion"),
    bulletItem("State-level adversaries: jurisdiction-level censorship, forced key disclosure, traffic analysis"),
    bulletItem("Cryptographic threats: quantum computing, side-channel attacks, implementation flaws"),
    bulletItem("Systemic threats: cascade failures, consensus deadlocks, economic attacks on watcher bonds"),
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

    // --- 46.3 Audit System ---
    h2("46.3 Audit System"),
    p("Every state-changing operation in the SEZ Stack produces an AuditEvent. Audit events are structured records containing: event_id (unique identifier), timestamp (UTC with nanosecond precision), actor (authenticated principal), action (the operation performed), target (the resource affected), outcome (success or failure with error context), and previous_event_digest (SHA-256 hash of the prior audit event, forming a hash chain). The hash chain ensures that audit log tampering is detectable: any modification to a historical event invalidates all subsequent digests. Audit events are persisted to Postgres with write-through semantics and replicated to an append-only external store for regulatory retention."),

    // --- 46.4 πpriv Constraint Analysis ---
    h2("46.4 \u03C0priv Constraint Analysis"),
    p("The privacy-preserving proof system (\u03C0priv) enables confidential compliance verification without revealing underlying transaction data. Each proof demonstrates that a transaction satisfies jurisdictional compliance requirements while preserving commercial confidentiality. The circuit is decomposed into eight constraint groups (C1 through C8), each enforcing a specific invariant."),
    table(
      ["Constraint", "Description", "R1CS Constraints", "Purpose"],
      [
        ["C1", "Value commitment opening", "~4,000", "Proves the prover knows the opening (value, blinding factor) to a Pedersen commitment. Ensures committed values are well-formed without revealing the transaction amount."],
        ["C2", "Nullifier computation", "~3,000", "Derives a unique nullifier from the input note and spending key. Prevents double-spending by ensuring each note can only be consumed once, without linking to the original deposit."],
        ["C3", "Merkle tree membership", "~8,000", "Proves the input note exists in the global state tree via a Merkle authentication path. The tree depth determines the constraint count; 32 levels yield approximately 8,000 constraints."],
        ["C4", "Signature verification", "~6,000", "Verifies an Ed25519 signature inside the circuit, binding the proof to an authorized signer. This is the most expensive per-operation constraint group due to elliptic curve arithmetic in R1CS."],
        ["C5", "Range proof", "~4,000", "Ensures all output values are non-negative and within the valid range [0, 2^64). Prevents overflow attacks that could create value from nothing via modular arithmetic."],
        ["C6", "Balance conservation", "~2,000", "Enforces that the sum of input values equals the sum of output values plus fees. This is the fundamental soundness constraint: no value is created or destroyed."],
        ["C7", "Compliance predicate", "~3,000", "Evaluates a jurisdiction-specific compliance predicate: sanctions list non-membership, transaction limit adherence, and withholding tax computation. The predicate is parameterized by the compliance tensor."],
        ["C8", "Output commitment", "~4,000", "Constructs Pedersen commitments for each output note. Mirrors C1 for outputs, ensuring recipients can later prove ownership and spend the received notes."],
      ],
      [800, 1800, 1200, 5560]
    ),
    p_runs([bold("Total Circuit Size."), " The complete \u03C0priv circuit comprises approximately 34,000 R1CS constraints. At current proving speeds (Groth16 on commodity hardware), proof generation takes 2-4 seconds and verification takes under 10 milliseconds. The proof size is constant at 192 bytes (two G1 points and one G2 point in BN254), making it practical for on-chain verification and credential embedding."]),
    p_runs([bold("Constraint Optimization."), " C3 (Merkle membership) and C4 (signature verification) together account for over 40% of the circuit. Two optimization paths are under evaluation: (1) replacing SHA-256 in the Merkle tree with a SNARK-friendly hash (Poseidon), which would reduce C3 to approximately 3,000 constraints, and (2) using Schnorr signatures instead of Ed25519 inside circuits, reducing C4 to approximately 2,500 constraints. These optimizations are tracked in the msez-zkp crate roadmap."]),
    p_runs([bold("Security Level."), " The circuit targets 128-bit security. The BN254 curve provides approximately 126 bits of security against discrete log attacks. For sovereign deployments requiring higher security margins, BLS12-381 (approximately 128 bits) is supported as an alternative curve, at the cost of approximately 20% larger constraints due to the wider field."]),
  ];
};
