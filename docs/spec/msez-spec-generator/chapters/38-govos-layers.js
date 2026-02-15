const {
  partHeading, chapterHeading,
  p, p_runs, bold,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter38() {
  return [
    ...partHeading("PART XIV: GovOS ARCHITECTURE"),

    p("GovOS is the emergent product when the full MSEZ Stack + Mass APIs are deployed for a sovereign government. It is not a separate product. It is what the Stack becomes at national scale. Pakistan serves as the reference architecture."),

    chapterHeading("Chapter 38: Four-Layer Model"),

    p("The GovOS architecture comprises four layers:"),

    table(
      ["Layer", "Name", "Function"],
      [
        ["01", "Experience", "Dashboards, portals, citizen-facing services, AI-powered interfaces"],
        ["02", "Platform Engine", "Five Mass primitives + supporting infrastructure + regulated organs"],
        ["03", "Jurisdictional Configuration", "MSEZ Pack Trilogy encoding national law in machine-readable format"],
        ["04", "National System Integration", "Connections to existing government systems (Mass enhances, never replaces)"],
      ],
      [800, 2400, 6160]
    ),

    spacer(),

    p_runs([bold("Layer 01 — Experience."), " The Experience Layer provides all citizen-facing and government-officer-facing interfaces. This includes the GovOS Console (administrative dashboards for ministry officials), citizen portals (tax filing, license applications, entity registration), and AI-powered interfaces (natural language queries against government data). The Experience Layer never contains business logic; it calls the Platform Engine for all operations."]),

    p_runs([bold("Layer 02 — Platform Engine."), " The Platform Engine is the five Mass primitives (Entities, Ownership, Fiscal, Identity, Consent) plus the regulated Organs (Center of Mass, Torque, Inertia) plus supporting infrastructure (templating engine, notification service, document storage). This layer provides the transactional capabilities that power all government operations."]),

    p_runs([bold("Layer 03 — Jurisdictional Configuration."), " The Jurisdictional Configuration layer is the MSEZ Pack Trilogy encoding Pakistani law, regulation, and licensing requirements in machine-readable format. Lawpacks encode the Income Tax Ordinance 2001, Sales Tax Act 1990, Companies Act 2017, and all relevant SROs. Regpacks encode SBP rates, FATF sanctions lists, and filing calendars. Licensepacks encode SECP, BOI, PTA, PEMRA, and provincial authority requirements."]),

    p_runs([bold("Layer 04 — National System Integration."), " The National System Integration layer connects GovOS to existing Pakistani government systems. This includes FBR IRIS (tax administration), SBP Raast (instant payments), NADRA (identity verification), SECP (company registration), SBP RTGS (large-value settlements), and Pakistan Single Window (trade facilitation). Mass enhances these systems; it never replaces them. Integration is additive and reversible."]),
  ];
};
