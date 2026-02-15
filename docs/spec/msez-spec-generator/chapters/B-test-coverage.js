const { chapterHeading, table, spacer } = require("../lib/primitives");

module.exports = function build_appendixB() {
  return [
    chapterHeading("Appendix B: Test Coverage Summary"),
    table(
      ["Test Category", "Count", "Coverage"],
      [
        ["MASS Protocol Primitives", "62", "100%"],
        ["RegPack/Arbitration", "36", "100%"],
        ["Agentic Framework", "18", "100%"],
        ["Smart Asset Lifecycle", "45", "100%"],
        ["Corridor Operations", "32", "100%"],
        ["Receipt Chain", "28", "100%"],
        ["Compliance Tensor V2", "22", "100%"],
        ["Compliance Manifold", "18", "100%"],
        ["Migration Protocol", "24", "100%"],
        ["Watcher Economy", "20", "100%"],
        ["Smart Asset VM", "28", "100%"],
        ["Corridor Bridge", "16", "100%"],
        ["L1 Anchoring", "14", "100%"],
        ["Composition Engine", "45", "100%"],
        ["Licensepacks", "55", "100%"],
        ["Corporate Modules", "65", "100%"],
        ["Identity Modules", "40", "100%"],
        ["Integration Tests", "82", "100%"],
        ["Total", "650", "100%"],
      ],
      [4000, 1200, 4160]
    ),
    spacer(),
  ];
};
