const { chapterHeading, h2, h3, p, p_runs, bold, definition, theorem, table } = require("../lib/primitives");

module.exports = function build_appendixD() {
  return [
    chapterHeading("Appendix D: Security Proofs Summary"),

    // --- Theorem Summary Table (existing, preserved) ---
    h2("D.1 Theorem Index"),
    table(
      ["Theorem", "Statement"],
      [
        ["9.1 (Object Survivability)", "Receipt chains maintain integrity during offline operation"],
        ["10.1 (Compliance Soundness)", "Compliance proofs demonstrate predicate satisfaction; false claims are computationally infeasible"],
        ["28.1 (Watcher Accountability)", "Dishonest attestations result in provable collateral loss"],
        ["29.1 (Identity Immutability)", "Smart Asset identity is established at genesis and cannot be modified"],
        ["29.2 (Non-Repudiation)", "Authorized state transitions cannot be repudiated"],
        ["30.1 (Migration Atomicity)", "Migration completes fully or compensation returns asset to original state"],
        ["31.1 (Unlinkability)", "Private transactions are unlinkable without viewing keys"],
        ["32.1 (Double-Spend Resistance)", "Each record can be spent exactly once via nullifier mechanism"],
      ],
      [2800, 6560]
    ),

    // --- Formal Definitions ---
    h2("D.2 Formal Definitions"),

    definition(
      "Definition D.1 (Receipt Chain).",
      "A receipt chain C is an ordered sequence of receipts (r_0, r_1, ..., r_n) where " +
      "r_0 is the genesis receipt and each r_i (i > 0) contains: (a) a payload hash " +
      "H(payload_i), (b) the digest of the previous receipt H(r_{i-1}), and (c) an MMR root " +
      "root_i computed over all receipts (r_0, ..., r_i). The chain is valid iff for all i in [1, n], " +
      "the back-link H(r_{i-1}) in r_i matches the actual hash of r_{i-1}, and root_i is the " +
      "correct MMR root over the prefix."
    ),

    definition(
      "Definition D.2 (Compliance Tensor).",
      "A compliance tensor C is a function C: AssetID x JurisdictionID x ComplianceDomain x " +
      "TimeQuantum -> ComplianceState, where ComplianceState is a discrete 7-state lattice with " +
      "ordering NonCompliant < Expired < Suspended < Unknown < Pending < {Compliant, Exempt} " +
      "(Compliant and Exempt form an incomparable top pair). For a given asset a, jurisdiction j, " +
      "domain d, and time quantum t, C(a, j, d, t) is computed by composing evaluations from the " +
      "pack trilogy: C(a, j, d, t) = compose(lawpack_d(j, t), regpack_d(j, t), licensepack_d(a, j, t)). " +
      "An asset is fully compliant in jurisdiction j at time t iff for all d in ComplianceDomain, " +
      "C(a, j, d, t) in {Compliant, Exempt}."
    ),

    definition(
      "Definition D.3 (Migration Protocol).",
      "A migration M from jurisdiction j_src to j_dst for asset A is an 8-phase saga: " +
      "(1) Initiation, (2) Compliance pre-check on j_dst, (3) Asset freeze in j_src, " +
      "(4) State snapshot and cryptographic commitment, (5) State transfer to j_dst, " +
      "(6) Compliance post-check on j_dst, (7) Activation in j_dst, (8) Tombstone in j_src. " +
      "The migration is atomic: it reaches phase 8 (complete) or executes compensation " +
      "that reverses all effects and restores A to its pre-migration state in j_src."
    ),

    definition(
      "Definition D.4 (Watcher Bond).",
      "A watcher w posts bond B_w as collateral for honest attestation. For each attestation a " +
      "produced by w, the bond is subject to slashing if a fraud proof pi is produced such that " +
      "Verify(pi, a) = true, demonstrating that a contradicts the canonical chain state. " +
      "Slashing transfers a fraction f of B_w to the fraud proof submitter and the remainder to " +
      "the protocol treasury."
    ),

    definition(
      "Definition D.5 (Nullifier).",
      "For a record r with secret key sk, the nullifier is nf = H(sk || r.id) where H is SHA-256. " +
      "Publishing nf marks r as spent. The nullifier set NS is append-only. A record r is spendable " +
      "iff its nullifier nf is not in NS. Double-spend is prevented because spending r requires " +
      "publishing nf, and a second spend attempt would produce the same nf, which is already in NS."
    ),

    // --- Proof Sketches ---
    h2("D.3 Proof Sketches"),

    h3("D.3.1 Theorem 9.1: Object Survivability"),
    theorem(
      "Theorem 9.1.",
      "Let A be a Smart Asset with receipt chain C = (r_0, ..., r_n). If A operates offline " +
      "for any duration t, appending receipts (r_{n+1}, ..., r_{n+k}) locally, then upon " +
      "reconnection the offline receipts can be verified and merged without loss of integrity."
    ),
    p_runs([bold("Proof sketch. "), "The proof proceeds in three steps."]),
    p(
      "Step 1 (Local chain validity). Each offline receipt r_{n+i} contains the back-link " +
      "H(r_{n+i-1}) and a locally computed MMR root. Because SHA-256 is collision-resistant, " +
      "the local chain (r_{n+1}, ..., r_{n+k}) forms a valid extension of C: any tampering " +
      "with a receipt r_{n+i} would break the back-link in r_{n+i+1}, and any tampering with " +
      "the MMR root would be detected during verification against the receipt sequence."
    ),
    p(
      "Step 2 (Reconnection verification). Upon reconnection, the verifier checks that " +
      "r_{n+1}.back_link = H(r_n), confirming the offline chain extends the last known online " +
      "receipt. The verifier then walks the offline chain, confirming each back-link and " +
      "recomputing MMR roots. If all checks pass, the offline chain is accepted."
    ),
    p(
      "Step 3 (Fork detection). If a conflicting chain (r_{n+1}', ..., r_{n+m}') exists " +
      "(i.e., a fork at position n+1), the system detects this because r_{n+1}.back_link = " +
      "r_{n+1}'.back_link = H(r_n) but H(r_{n+1}) != H(r_{n+1}'). Fork resolution applies " +
      "the corridor-specific policy (longest chain, highest compliance score, or manual arbitration). " +
      "In all cases, neither fork can be silently discarded -- both are recorded for audit. QED."
    ),

    h3("D.3.2 Theorem 10.1: Compliance Soundness"),
    theorem(
      "Theorem 10.1.",
      "Given a compliance tensor evaluation where C(a, j, d, t) in {Compliant, Exempt} for all " +
      "domains d, the corresponding compliance proof pi is sound: no polynomial-time adversary can " +
      "produce a proof pi' that causes a verifier to accept a non-compliant asset as compliant."
    ),
    p_runs([bold("Proof sketch. "), "By reduction to the binding property of cryptographic commitments."]),
    p(
      "The compliance proof pi contains: (a) a commitment to the pack trilogy state (lawpack hash, " +
      "regpack hash, licensepack hash), (b) the tensor evaluation result (a vector of ComplianceState " +
      "values across all 20 domains), and (c) a signature by the evaluating node's Ed25519 key. " +
      "To forge pi', an adversary must either: (i) find a collision in SHA-256 to substitute a " +
      "different pack state under the same commitment (infeasible by collision resistance), (ii) forge " +
      "the Ed25519 signature (infeasible by EUF-CMA security), or (iii) manipulate the tensor " +
      "evaluation function to produce Compliant or Exempt for a non-compliant asset (prevented by " +
      "deterministic evaluation from committed pack state, where each domain's ComplianceState is " +
      "computed from the lattice meet of the lawpack, regpack, and licensepack evaluations). Since " +
      "all three attack vectors are computationally infeasible, the proof is sound. QED."
    ),

    h3("D.3.3 Theorem 28.1: Watcher Accountability"),
    theorem(
      "Theorem 28.1.",
      "If a watcher w produces a dishonest attestation a that contradicts the canonical chain state, " +
      "then a fraud proof pi can be constructed such that Verify(pi, a) = true, triggering slashing " +
      "of w's bond B_w."
    ),
    p_runs([bold("Proof sketch. "), "By construction of the fraud proof."]),
    p(
      "An attestation a by watcher w asserts that state S is the canonical state at height h. " +
      "If a is dishonest, then the actual canonical state at height h is S' != S. A fraud proof " +
      "pi consists of: (a) the attestation a signed by w, (b) the canonical receipt chain up to " +
      "height h demonstrating state S', and (c) a Merkle proof that S' is committed in the MMR root " +
      "at height h. The verifier checks w's signature on a (confirming w made the claim), verifies " +
      "the canonical chain, and confirms S != S'. Since the MMR root is deterministic from the " +
      "receipt chain and SHA-256 is collision-resistant, w cannot claim the canonical chain was " +
      "ambiguous. The slashing contract transfers the prescribed fraction of B_w. QED."
    ),

    h3("D.3.4 Theorem 30.1: Migration Atomicity"),
    theorem(
      "Theorem 30.1.",
      "A migration M for asset A either completes all 8 phases (reaching Tombstone in j_src and " +
      "Activation in j_dst) or the compensation mechanism restores A to its exact pre-migration " +
      "state in j_src. No intermediate state is observable after migration completes or aborts."
    ),
    p_runs([bold("Proof sketch. "), "By induction on the migration phase."]),
    p(
      "Base case: At phase 1 (Initiation), no state has been modified. Abort is trivially safe. " +
      "Inductive step: Assume that at phase k, the compensation mechanism can reverse all effects " +
      "of phases 1 through k. At phase k+1, the saga records a compensating action C_{k+1} before " +
      "executing the forward action F_{k+1}. If F_{k+1} fails, the saga executes C_{k+1}, then " +
      "C_k, ..., C_1 in reverse order. Each compensating action is idempotent (can be safely " +
      "re-executed) and commutative with respect to concurrent operations on other assets (the " +
      "asset freeze at phase 3 ensures no concurrent modifications to A)."
    ),
    p(
      "Critical phases: Phase 3 (Asset freeze) establishes an exclusive lock on A in j_src, " +
      "preventing concurrent modifications. Phase 4 (State snapshot) creates a cryptographic " +
      "commitment to A's state, providing the restoration target. Phase 7 (Activation in j_dst) " +
      "is the commit point -- once j_dst activates the asset, phase 8 (Tombstone in j_src) must " +
      "complete. If phase 8 fails, it is retried (tombstoning is idempotent). The migration duration " +
      "bound S10 ensures that if the saga has not reached phase 7 within the allotted time, automatic " +
      "rollback is triggered, executing all recorded compensating actions. QED."
    ),

    h3("D.3.5 Theorem 32.1: Double-Spend Resistance"),
    theorem(
      "Theorem 32.1.",
      "Given a record r and the append-only nullifier set NS, r can be spent exactly once. " +
      "Any attempt to spend r a second time will be rejected."
    ),
    p_runs([bold("Proof sketch. "), "By the determinism of the nullifier function and the append-only property of NS."]),
    p(
      "Spending r requires computing nf = H(sk || r.id) and publishing nf to NS. The first spend " +
      "succeeds because nf is not in NS. Any subsequent spend of r produces the same nf (SHA-256 is " +
      "deterministic), which is now in NS. The membership check nf in NS returns true, and the spend " +
      "is rejected. An adversary attempting to produce a different nullifier nf' for the same record r " +
      "would need to find sk' such that H(sk' || r.id) != H(sk || r.id) while still satisfying the " +
      "ownership proof -- this is prevented by the binding between sk and the ownership credential. " +
      "The append-only property of NS ensures that once a nullifier is published, it cannot be removed. QED."
    ),
  ];
};
