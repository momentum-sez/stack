const { chapterHeading, table, spacer } = require("../lib/primitives");

module.exports = function build_appendixJ() {
  return [
    chapterHeading("Appendix J: Conformance Levels"),
    table(
      ["Level", "Category", "Requirements"],
      [
        ["1", "Schema Conformance", "JSON Schema validation, Akoma Ntoso, W3C VC data model"],
        ["2", "Behavioral Conformance", "Module dependency resolution, deterministic outputs"],
        ["3", "Cryptographic Conformance", "Signature verification, ZK soundness, correct hashes"],
        ["4", "Corridor Integrity", "Definition VC binding, agreement binding, fork detection"],
        ["5", "Migration Integrity", "State machine transitions, compensation execution"],
      ],
      [800, 2600, 5960]
    ),
    spacer(),
  ];
};
