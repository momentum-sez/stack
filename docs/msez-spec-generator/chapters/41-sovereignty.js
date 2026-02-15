const {
  chapterHeading, h2,
  p, p_runs, bold,
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

    h2("Phase 1 — Foundation (Months 1-6)"),

    p("The Foundation phase establishes the core infrastructure and connects the primary national systems. This phase operates with a joint Momentum-Pakistan engineering team, with Momentum engineers leading and Pakistani counterparts embedded at every level."),

    table(
      ["Month", "Milestone", "Deliverable", "Exit Criteria"],
      [
        ["1", "Infrastructure provisioning", "On-premise GPU cluster operational (training + inference), Kubernetes cluster deployed, network fabric configured, zero-egress verified", "All hardware passes burn-in; AI training cluster achieves target FLOPS; network ACLs confirmed by independent audit"],
        ["2", "Mass primitive deployment", "All five Mass primitives deployed to Pakistani data center; Entities, Ownership, Fiscal, Identity, Consent APIs operational", "End-to-end entity formation test completes; all API health checks green; load test sustains 1000 req/s"],
        ["3", "FBR IRIS integration", "Bidirectional connection to FBR IRIS operational; real-time withholding reporting active; NTN validation live", "100 test withholding statements successfully submitted and reconciled with FBR; error rate below 0.1%"],
        ["4", "SBP Raast + NADRA", "SBP Raast payment initiation/confirmation live; NADRA CNIC verification bridge operational", "Successful end-to-end payment cycle via Raast; 1000 CNIC verifications completed with >99.5% match rate"],
        ["5", "AI Spine initial training", "Foundation model fine-tuned on 3 years of historical FBR data; initial tax gap model producing risk scores", "Model achieves >70% precision on known evasion cases from historical audit data; false positive rate below 30%"],
        ["6", "Pilot deployment", "GovOS Console live for FBR pilot group (50 officers); Tax & Revenue Dashboard active with real data", "FBR pilot officers complete 100 AI-assisted audit reviews; system uptime >99.5% for 30 consecutive days"],
      ],
      [600, 1800, 3600, 3360]
    ),

    h2("Phase 2 — Expansion (Months 7-12)"),

    table(
      ["Month", "Milestone", "Deliverable", "Exit Criteria"],
      [
        ["7", "Ministry onboarding (wave 1)", "Finance Division, Commerce Division, and Planning Commission onboarded; ministry-specific dashboards deployed", "Each ministry has 10+ active users; data feeds from ministry systems operational"],
        ["8", "Ministry onboarding (wave 2)", "Revenue Division, Industries Division, and Board of Investment onboarded; cross-ministry analytics operational", "Cross-ministry spend anomaly detection producing alerts; 6 ministries fully active"],
        ["9", "Corridor activation", "PAK-UAE and PAK-KSA trade corridors live; receipt chains operational; SWIFT pacs.008 adapter tested", "10 end-to-end corridor transactions completed per corridor; settlement within SLA"],
        ["10", "Licensing module deployment", "SECP, BOI, PTA, and PEMRA license management live; automated renewal notifications active", "500+ licenses migrated to digital management; automated renewal accuracy >99%"],
        ["11", "Provincial authority integration", "Punjab and Sindh revenue authorities connected; provincial sales tax integration live", "Provincial WHT collection flowing through pipeline; reconciliation with provincial FBR offices confirmed"],
        ["12", "Full ministry coverage", "All 40+ federal ministries onboarded; Citizen Tax & Services Portal launched for public beta", "40+ ministries active with minimum 5 users each; citizen portal handles 10,000+ tax filings in beta"],
      ],
      [600, 1800, 3600, 3360]
    ),

    h2("Phase 3 — Optimization (Months 13-18)"),

    table(
      ["Month", "Milestone", "Deliverable", "Exit Criteria"],
      [
        ["13", "AI model refinement (cycle 1)", "Tax gap model retrained with 6 months of live pipeline data; precision improvement measured", "Model precision >80% on evasion detection; false positive rate below 20%; measurable improvement over Month 5 baseline"],
        ["14", "Tax gap reduction measurement", "First official tax gap reduction report published; comparison against pre-GovOS baseline", "Identified additional revenue of PKR 50+ billion attributable to AI-assisted detection"],
        ["15", "Cross-department analytics", "Cross-ministry correlation engine operational; procurement irregularity detection live; PIMA alignment dashboard", "3+ cross-department fraud cases identified through correlation; PIMA score improvement documented"],
        ["16", "CPEC corridor planning", "China-Pakistan Economic Corridor digital trade lane designed; integration architecture with Chinese customs approved", "Technical specification signed off by both governments; sandbox environment operational"],
        ["17", "AI model refinement (cycle 2)", "Urdu NLP model deployed for citizen portal; natural language tax queries operational", "Citizen queries answered with >85% accuracy in Urdu; user satisfaction score >4.0/5.0"],
        ["18", "Performance optimization", "System-wide performance audit completed; capacity planning for standalone operations finalized", "All APIs meet p99 latency SLAs; capacity plan covers 3x projected growth; DR failover tested"],
      ],
      [600, 1800, 3600, 3360]
    ),

    h2("Phase 4 — Handover (Months 19-24)"),

    table(
      ["Month", "Milestone", "Deliverable", "Exit Criteria"],
      [
        ["19", "Knowledge transfer (infrastructure)", "Pakistani infrastructure team leads all deployment operations; Momentum engineers in advisory role only", "Pakistani team completes 3 independent deployment cycles without Momentum intervention"],
        ["20", "Knowledge transfer (application)", "Pakistani application engineering team leads all feature development; code review authority transferred", "Pakistani team delivers 2 features end-to-end; code quality metrics match Momentum baseline"],
        ["21", "Knowledge transfer (AI/ML)", "Pakistani AI/ML team leads model retraining cycles; training pipeline fully documented and independently operable", "Pakistani team completes 1 full model retraining cycle independently; model performance maintained"],
        ["22", "Operational documentation", "Complete operational runbooks, architecture decision records, incident response procedures, and capacity planning guides delivered", "Documentation review completed by Pakistani team; tabletop incident exercise passed"],
        ["23", "Support transition", "24/7 operations fully staffed by Pakistani team; Momentum available for escalation only", "Pakistani ops team handles 30 consecutive days of operations independently; SLA maintained"],
        ["24", "Full sovereignty transfer", "All credentials, access controls, and administrative authority transferred; Momentum retains advisory-only role", "Formal sovereignty certificate issued; all access audited and confirmed; advisory SLA signed"],
      ],
      [600, 1800, 3600, 3360]
    ),

    spacer(),

    h2("Key Personnel Requirements"),

    p("Successful execution of the 24-month handover requires dedicated personnel on both the Momentum and Pakistani sides. The following table specifies minimum staffing by role category and phase. Pakistani personnel are embedded with Momentum engineers from Phase 1, ensuring continuous knowledge transfer rather than a single handover event."),

    table(
      ["Role Category", "Phase 1 (Momentum / Pakistan)", "Phase 2 (Momentum / Pakistan)", "Phase 3 (Momentum / Pakistan)", "Phase 4 (Momentum / Pakistan)"],
      [
        ["Platform Engineering (Rust, distributed systems)", "6 / 4", "5 / 6", "3 / 8", "1 / 10"],
        ["Infrastructure / DevOps (Kubernetes, GPU, networking)", "4 / 3", "3 / 4", "2 / 5", "1 / 6"],
        ["AI / Machine Learning (model training, NLP)", "3 / 2", "3 / 3", "2 / 4", "1 / 5"],
        ["Integration Engineering (FBR, SBP, NADRA adapters)", "4 / 3", "3 / 4", "2 / 5", "0 / 6"],
        ["Security and Compliance (audit, pen testing, SOC)", "2 / 1", "2 / 2", "1 / 3", "0 / 3"],
        ["Product Management", "2 / 1", "1 / 2", "1 / 2", "0 / 2"],
        ["Project Management / Delivery", "1 / 1", "1 / 1", "1 / 1", "0 / 2"],
        ["Total", "22 / 15", "18 / 22", "12 / 28", "3 / 34"],
      ],
      [2200, 1800, 1800, 1800, 1760]
    ),

    p("By Phase 4, the Pakistani team comprises 34 engineers and operators, fully capable of independent operation. The 3 remaining Momentum personnel serve exclusively in an advisory capacity with no production access. Total Pakistani capacity build: 34 trained engineers across platform, infrastructure, AI, integration, and security disciplines."),

    spacer(),

    h2("Success Metrics"),

    p("The following metrics define success at each phase gate. Phase advancement requires all metrics for the current phase to be met. Metrics are measured independently by both Momentum and the Pakistani government, with discrepancies resolved through the project steering committee."),

    table(
      ["Metric", "Phase 1 Target", "Phase 2 Target", "Phase 3 Target", "Phase 4 Target"],
      [
        ["System uptime (monthly)", ">99.0%", ">99.5%", ">99.9%", ">99.9% (Pakistan-operated)"],
        ["API p99 latency", "<500ms", "<300ms", "<200ms", "<200ms (Pakistan-operated)"],
        ["Tax events processed per day", "10,000+", "100,000+", "500,000+", "500,000+ (Pakistan-operated)"],
        ["AI evasion detection precision", ">70%", ">75%", ">80%", ">80% (Pakistan-retrained)"],
        ["AI false positive rate", "<30%", "<25%", "<20%", "<20% (Pakistan-retrained)"],
        ["National systems integrated", "3 (FBR, SBP, NADRA)", "6 (+SECP, PSW, provincial)", "6 (optimized)", "6 (Pakistan-maintained)"],
        ["Active government users", "50 (FBR pilot)", "500+ across ministries", "2,000+ including citizens", "10,000+ (full rollout)"],
        ["Identified additional revenue (cumulative)", "Baseline established", "PKR 50+ billion", "PKR 150+ billion", "PKR 300+ billion"],
        ["Pakistani team independent operations", "Shadowing all operations", "Leading 30% of operations", "Leading 70% of operations", "Leading 100% of operations"],
        ["Documentation coverage", "Architecture docs complete", "Runbooks for all systems", "Incident response tested", "Full handover package accepted"],
        ["Security audit findings (critical/high)", "Zero critical, <5 high", "Zero critical, <3 high", "Zero critical, zero high", "Zero critical, zero high (Pakistan-audited)"],
        ["Disaster recovery RTO", "<4 hours", "<2 hours", "<1 hour", "<1 hour (Pakistan-tested)"],
      ],
      [2000, 1800, 1800, 1800, 1960]
    ),

    spacer(),
  ];
};
