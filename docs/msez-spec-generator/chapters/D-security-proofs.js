const { chapterHeading, table, spacer } = require("../lib/primitives");

module.exports = function build_appendixD() {
  return [
    chapterHeading("Appendix D: Security Proofs Summary"),
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
    spacer(),
  ];
};
