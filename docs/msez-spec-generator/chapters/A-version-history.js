const { partHeading, chapterHeading, table, spacer } = require("../lib/primitives");

module.exports = function build_appendixA() {
  return [
    ...partHeading("APPENDICES"),
    chapterHeading("Appendix A: Version History"),
    table(
      ["Version", "Date", "Changes"],
      [
        ["0.4.44", "Feb 2026", "GENESIS: Licensepacks, Composition Engine, Corporate/Identity/Tax/Markets/Trade modules, One-click deployment, Rust migration, Mass/MSEZ separation, GovOS architecture, live corridors"],
        ["0.4.43", "Jan 2026", "Phoenix Ascendant: Compliance Tensor V2, Manifold, SAVM, Watcher Economy, Migration, Bridge"],
        ["0.4.42", "Jan 2026", "Agentic Ascension: Agentic Framework, ZK L1, enhanced arbitration"],
        ["0.4.41", "Dec 2025", "Arbitration System: Institution registry, dispute filing, ruling enforcement"],
        ["0.4.40", "Nov 2025", "RegPack Integration: Dynamic regulatory state, sanctions screening"],
        ["0.4.38", "Oct 2025", "Initial comprehensive specification, core modules"],
        ["0.4.0", "Jul 2025", "Architecture redesign, Smart Asset model"],
        ["0.3.0", "Mar 2025", "Compliance tensor, lawpack system"],
        ["0.2.0", "Dec 2024", "Receipt chain architecture, MMR checkpoints"],
        ["0.1.0", "Sep 2024", "Initial specification draft"],
      ],
      [1200, 1400, 6760]
    ),
    spacer(),
  ];
};
