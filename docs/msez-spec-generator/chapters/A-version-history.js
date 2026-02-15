const { partHeading, chapterHeading, h2, p, p_runs, bold, table, spacer } = require("../lib/primitives");

module.exports = function build_appendixA() {
  return [
    ...partHeading("APPENDICES"),
    chapterHeading("Appendix A: Version History"),

    // --- Narrative: Evolution Trajectory ---
    h2("A.1 Evolution Trajectory"),
    p(
      "The SEZ Stack specification has evolved through four distinct phases over eighteen months. " +
      "Phase I (Sep 2024 - Mar 2025) established the foundational primitives: receipt chains with MMR checkpoints " +
      "for tamper-evident state tracking, and the compliance tensor with lawpack system for jurisdiction-aware " +
      "predicate evaluation. Phase II (Jul - Oct 2025) introduced the Smart Asset model and the initial " +
      "comprehensive specification covering core modules. Phase III (Nov 2025 - Jan 2026) expanded into " +
      "regulatory integration (regpacks, sanctions screening), dispute resolution (arbitration institutions), " +
      "autonomous operations (agentic framework with ZK L1 anchoring), and the Phoenix subsystem " +
      "(Compliance Tensor V2, Manifold path optimization, SAVM execution, Watcher Economy, and the " +
      "Migration/Bridge protocols). Phase IV (Feb 2026) represents the GENESIS milestone: the complete " +
      "separation of Mass primitives from SEZ Stack orchestration, the port from Python to pure Rust, " +
      "the licensepack system for live registry integration, the composition engine for multi-pack evaluation, " +
      "and the GovOS deployment architecture targeting sovereign jurisdictions."
    ),
    p(
      "Each version builds cumulatively. The specification has grown from a single-page draft to a " +
      "comprehensive system covering 20 compliance domains, multi-jurisdiction corridor operations, " +
      "verifiable credential issuance, and autonomous policy enforcement -- all orchestrated above the " +
      "five Mass programmable primitives (Entities, Ownership, Fiscal, Identity, Consent)."
    ),
    spacer(),

    // --- Version Table with Milestone Annotations ---
    h2("A.2 Version Log"),
    table(
      ["Version", "Date", "Milestone", "Changes"],
      [
        ["0.4.44", "Feb 2026", "GENESIS", "Licensepacks, Composition Engine, Corporate/Identity/Tax/Markets/Trade modules, One-click deployment, Rust migration, Mass/MSEZ separation, GovOS architecture, live corridors"],
        ["0.4.43", "Jan 2026", "PHOENIX", "Phoenix Ascendant: Compliance Tensor V2, Manifold, SAVM, Watcher Economy, Migration, Bridge"],
        ["0.4.42", "Jan 2026", "AGENTIC", "Agentic Ascension: Agentic Framework, ZK L1, enhanced arbitration"],
        ["0.4.41", "Dec 2025", "ARBITRATION", "Arbitration System: Institution registry, dispute filing, ruling enforcement"],
        ["0.4.40", "Nov 2025", "REGPACK", "RegPack Integration: Dynamic regulatory state, sanctions screening"],
        ["0.4.38", "Oct 2025", "SPEC-CORE", "Initial comprehensive specification, core modules"],
        ["0.4.0", "Jul 2025", "REDESIGN", "Architecture redesign, Smart Asset model"],
        ["0.3.0", "Mar 2025", "--", "Compliance tensor, lawpack system"],
        ["0.2.0", "Dec 2024", "--", "Receipt chain architecture, MMR checkpoints"],
        ["0.1.0", "Sep 2024", "INITIAL", "Initial specification draft"],
      ],
      [1000, 1100, 1400, 5860]
    ),
    spacer(),

    // --- Milestone Summary ---
    h2("A.3 Milestone Definitions"),
    p_runs([bold("INITIAL (0.1.0): "), "First formal specification draft establishing scope and terminology."]),
    p_runs([bold("REDESIGN (0.4.0): "), "Fundamental architecture shift to the Smart Asset model, replacing the prior document-centric approach with programmable objects carrying receipt chains."]),
    p_runs([bold("SPEC-CORE (0.4.38): "), "First comprehensive specification covering all core modules with formal definitions and theorem statements."]),
    p_runs([bold("REGPACK (0.4.40): "), "Introduction of dynamic regulatory state management, enabling real-time sanctions screening and calendar-aware filing deadlines."]),
    p_runs([bold("ARBITRATION (0.4.41): "), "Dispute resolution framework with institution registries, evidence packaging, and VC-based ruling enforcement."]),
    p_runs([bold("AGENTIC (0.4.42): "), "Autonomous operation layer with trigger taxonomy (20 types x 5 domains), policy evaluation, and ZK L1 anchoring for provable compliance."]),
    p_runs([bold("PHOENIX (0.4.43): "), "Second-generation compliance infrastructure: Tensor V2 with 20 compliance domains, Manifold for cross-corridor path optimization, SAVM for deterministic asset execution, Watcher Economy with bonding/slashing, and Migration/Bridge protocols."]),
    p_runs([bold("GENESIS (0.4.44): "), "Production-ready milestone. Complete Mass/SEZ separation, pure Rust codebase, licensepack system for live registry integration, composition engine for multi-pack evaluation, and GovOS architecture for sovereign deployment."]),
    spacer(),
  ];
};
