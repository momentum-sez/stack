const { chapterHeading, table, spacer, pageBreak, p, h2 } = require("../lib/primitives");

module.exports = function build_appendixK() {
  return [
    chapterHeading("Appendix K: GovOS Deployment Checklist"),
    table(
      ["Phase", "Milestone", "Verification"],
      [
        ["1.1", "Core infrastructure deployed (compute, storage, network)", "msez verify --all passes"],
        ["1.2", "National identity system connected (NADRA for Pakistan)", "Identity API handshake confirmed"],
        ["1.3", "Tax authority connected (FBR IRIS for Pakistan)", "Test tax event round-trip"],
        ["1.4", "Central bank connected (SBP Raast for Pakistan)", "Test payment round-trip"],
        ["1.5", "Corporate registry connected (SECP for Pakistan)", "Test entity formation round-trip"],
        ["2.1", "Pack Trilogy imported for national law", "msez pack verify --all passes"],
        ["2.2", "All ministry accounts provisioned (40+ for Pakistan)", "Ministry dashboard access confirmed"],
        ["2.3", "First corridor activated", "Cross-border test transaction succeeds"],
        ["3.1", "AI models trained on national data", "Revenue projection accuracy > 85%"],
        ["3.2", "Tax gap reduction measured", "Baseline vs. 6-month comparison report"],
        ["4.1", "Pakistani engineering team certified", "All runbook procedures demonstrated"],
        ["4.2", "Full operational handover", "Momentum advisory-only SLA signed"],
      ],
      [800, 4400, 4160]
    ),
    spacer(),

    h2("K.1 Success Criteria and KPIs"),
    p("Each checklist milestone has quantifiable success criteria. The following table defines the key performance indicators (KPIs) that must be met before a milestone is considered complete."),
    spacer(),

    table(
      ["Phase", "KPI", "Target", "Measurement Method"],
      [
        ["1.1", "Infrastructure availability", ">= 99.9% uptime over 72-hour burn-in", "Synthetic health checks every 30s; downtime = any period where msez verify --all fails"],
        ["1.1", "API response latency (p99)", "<= 200ms for all SEZ Stack endpoints", "Load test with 100 concurrent users for 1 hour; measure p99 from Axum access logs"],
        ["1.1", "Database failover recovery", "<= 30s automatic failover", "Simulate primary Postgres failure; measure time until write availability restored"],

        ["1.2", "NADRA CNIC verification round-trip", "<= 5s end-to-end", "Submit 100 test CNIC numbers; measure time from API call to verified response"],
        ["1.2", "Identity match accuracy", ">= 99.5% true positive rate", "Verify against 1,000 known-good CNIC records; count false negatives"],
        ["1.2", "Error handling for offline NADRA", "Graceful degradation with queued retry", "Disconnect NADRA endpoint; verify requests are queued and retried when connection restores"],

        ["1.3", "Tax event creation latency", "<= 3s per event", "Create 500 test tax events via FBR IRIS integration; measure mean and p99 latency"],
        ["1.3", "Withholding calculation accuracy", "100% match against FBR published rates", "Compute withholding for 50 test scenarios; compare against manual FBR rate table lookup"],
        ["1.3", "NTN binding success rate", ">= 99% for valid NTNs", "Submit 200 valid NTN binding requests; count failures"],

        ["1.4", "Raast payment initiation", "<= 2s to acknowledgment", "Initiate 100 test payments via SBP Raast; measure time to ACK"],
        ["1.4", "Payment reconciliation accuracy", "100% balance match", "After 500 test transactions, verify treasury-info balances match Raast settlement records"],
        ["1.4", "FX rate feed freshness", "<= 15 minutes stale", "Monitor SBP exchange rate feed; alert if last update exceeds 15 minutes"],

        ["1.5", "Entity formation end-to-end", "<= 10s from request to SECP confirmation", "Form 50 test entities; measure time from API call to SECP registration number returned"],
        ["1.5", "Beneficial ownership disclosure", "100% of entities have BO recorded", "Audit all formed entities; verify beneficial ownership data present and complete"],
        ["1.5", "VC issuance on formation", "100% of formations produce a signed formation VC", "Verify every entity formation triggers VC issuance; validate VC signature"],

        ["2.1", "Pack Trilogy completeness", "All active legislation encoded", "Cross-reference lawpack against FBR gazette of active SROs; coverage >= 95%"],
        ["2.1", "Schema validation pass rate", "100% of pack documents valid", "Run msez pack verify --all; zero validation failures"],
        ["2.1", "Composition engine correctness", "Composite output matches manual legal review", "Legal team reviews 10 randomly selected composite outputs; 100% agreement"],

        ["2.2", "Ministry account provisioning", "40+ accounts created and accessible", "Each ministry logs in and confirms dashboard access; list maintained in deployment manifest"],
        ["2.2", "Role-based access control", "Zero unauthorized access in pen test", "Security team attempts cross-ministry access; zero successful privilege escalations"],

        ["2.3", "First corridor activation", "PAK-UAE corridor in Active state", "msez corridor status --id PAK-UAE returns Active; definition and agreement VCs verified"],
        ["2.3", "Cross-border test transaction", "End-to-end settlement in <= 60s", "Execute 10 test transactions through the corridor; all settle within 60 seconds"],
        ["2.3", "Receipt chain integrity", "100% of receipts verify", "Verify MMR inclusion proofs for all receipts in the test corridor"],

        ["3.1", "Revenue projection accuracy", ">= 85% within 10% margin", "Compare model projections against actual FBR collections for the test period"],
        ["3.1", "Tax gap identification rate", ">= 70% of known gaps detected", "Seed 100 synthetic tax gap scenarios; measure model detection rate"],

        ["3.2", "Tax gap reduction", ">= 15% reduction vs. baseline", "Compare pre-deployment vs. 6-month post-deployment tax collection for SEZ entities"],
        ["3.2", "Filing compliance rate", ">= 90% on-time filing", "Measure percentage of SEZ entities filing returns by deadline vs. pre-deployment baseline"],

        ["4.1", "Team certification completeness", "100% of runbook procedures demonstrated", "Each procedure executed live by Pakistani engineer with Momentum observer; checklist signed off"],
        ["4.1", "Incident response drill", "<= 15 min to diagnose and begin remediation", "Simulate 3 failure scenarios; measure time from alert to correct diagnosis and remediation start"],

        ["4.2", "Handover documentation", "All runbooks, architecture docs, and access credentials transferred", "Signed handover manifest listing every artifact transferred; countersigned by both parties"],
        ["4.2", "Advisory-only SLA execution", "SLA signed with defined response times", "Momentum SLA specifies advisory-only role with 4-hour response time for P0, 24-hour for P1"],
      ],
      [600, 2600, 2800, 3360]
    ),
    spacer(),

    pageBreak(),
    p("End of Specification", { bold: true, size: 28 }),
    spacer(200),
    p("Momentum Open Source SEZ Stack"),
    p("Version 0.4.44 \u2014 GENESIS"),
    p("February 2026"),
    spacer(200),
    p("For questions or feedback, contact:"),
    p("Momentum"),
    p("https://github.com/momentum-sez/stack"),
    p("research@momentum.inc"),
  ];
};
