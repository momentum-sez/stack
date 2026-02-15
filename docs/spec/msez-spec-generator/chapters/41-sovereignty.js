const {
  chapterHeading,
  p,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter41() {
  return [
    chapterHeading("Chapter 41: Sovereignty Handover"),

    p("The GovOS deployment follows a 24-month sovereignty handover framework. At the end of month 24, full operational control transfers to Pakistani engineers and administrators."),

    table(
      ["Phase", "Timeline", "Milestones"],
      [
        ["1. Foundation", "Months 1-6", "Core infrastructure deployment, FBR IRIS integration, SBP Raast connection, NADRA identity bridge, initial AI training on Pakistani tax data"],
        ["2. Expansion", "Months 7-12", "All 40+ ministry onboarding, corridor activation (PAK-KSA, PAK-UAE), license module deployment across SECP, BOI, PTA, provincial authorities"],
        ["3. Optimization", "Months 13-18", "AI model refinement, tax gap reduction measurement, cross-department analytics, CPEC corridor planning"],
        ["4. Handover", "Months 19-24", "Knowledge transfer to Pakistani engineering team, operational documentation, support transition, full sovereignty transfer"],
      ],
      [1600, 1600, 6160]
    ),

    spacer(),

    p("Phase 1 (Months 1-6): Deploy core Mass primitives, connect national systems, begin AI training. Phase 2 (Months 7-12): Scale to all ministries, activate bilateral corridors, deploy full licensing. Phase 3 (Months 13-18): Optimize AI models, measure tax collection improvements, launch analytics dashboards. Phase 4 (Months 19-24): Transfer complete operational control to Pakistani engineers. Momentum retains advisory role only."),
  ];
};
