const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  table
} = require("../lib/primitives");

module.exports = function build_chapter53() {
  return [
    chapterHeading("Chapter 53: Operations Management"),

    // --- 53.1 Monitoring and Alerting ---
    h2("53.1 Monitoring and Alerting"),
    p("The MEZ Stack exposes Prometheus metrics on /metrics for all operational dimensions. Grafana dashboards visualize these metrics with pre-configured panels for API health, corridor throughput, compliance evaluation, and Mass API connectivity."),

    h3("53.1.1 Metric Families"),
    table(
      ["Metric", "Type", "Labels", "Description"],
      [
        ["mez_api_request_duration_seconds", "Histogram", "route, method, status", "API request latency distribution"],
        ["mez_corridor_state_transitions_total", "Counter", "corridor, from_state, to_state", "Corridor FSM transition count"],
        ["mez_tensor_evaluation_duration_seconds", "Histogram", "jurisdiction, domain_count", "Compliance tensor evaluation latency"],
        ["mez_pack_check_results", "Counter", "pack_type, outcome", "Pack validation pass/fail count"],
        ["mez_mass_client_request_duration_seconds", "Histogram", "primitive, endpoint, status", "Mass API call latency"],
        ["mez_vc_issuance_total", "Counter", "credential_type, issuer", "Verifiable Credential issuance count"],
        ["mez_db_pool_connections", "Gauge", "state (active/idle/waiting)", "Database connection pool utilization"],
        ["mez_receipt_chain_length", "Gauge", "corridor", "Current receipt chain depth per corridor"],
      ],
      [3200, 1200, 2200, 2760]
    ),

    h3("53.1.2 Alert Rules"),
    table(
      ["Alert", "Condition", "Severity", "Response Window"],
      [
        ["API Latency Spike", "p99 latency > 500ms for 5 minutes", "SEV-2", "30 minutes"],
        ["Corridor Stall", "No state transition for > 5 minutes on active corridor", "SEV-2", "30 minutes"],
        ["Mass API Errors", "Error rate > 1% over 5-minute window", "SEV-2", "30 minutes"],
        ["DB Pool Exhaustion", "Active connections > 80% of pool for 1 minute", "SEV-1", "15 minutes"],
        ["Certificate Expiry", "TLS certificate expires within 14 days", "SEV-3", "2 hours"],
        ["Disk Usage", "Any volume > 80% capacity", "SEV-2", "30 minutes"],
        ["Failed VC Verification", "Signature verification failure rate > 0.1%", "SEV-1", "15 minutes"],
      ],
      [2200, 3200, 1200, 2760]
    ),

    // --- 53.2 Incident Response ---
    h2("53.2 Incident Response"),
    table(
      ["Severity", "Criteria", "Response Time", "Resolution Target", "Escalation"],
      [
        ["SEV-1 (Critical)", "Complete outage, data breach, security compromise", "15 minutes", "4 hours", "On-call \u2192 Engineering Lead \u2192 CTO"],
        ["SEV-2 (Major)", "Degraded service, corridor delays, compliance failures", "30 minutes", "8 hours", "On-call \u2192 Engineering Lead"],
        ["SEV-3 (Minor)", "Isolated feature failure, single-user impact", "2 hours", "24 hours", "On-call \u2192 ticket queue"],
        ["SEV-4 (Low)", "Cosmetic issues, documentation gaps, optimizations", "Next business day", "Sprint cycle", "Ticket queue"],
      ],
      [1400, 2400, 1200, 1400, 2960]
    ),
    p("Each incident produces a post-incident review (PIR) within 48 hours of resolution. The PIR documents the timeline, root cause, impact assessment, remediation steps, and preventive actions. PIR findings feed back into alert rule tuning, runbook updates, and capacity planning."),

    // --- 53.3 Change Management ---
    h2("53.3 Change Management"),
    p("All production changes follow a staged rollout process with automated gates at each stage."),
    table(
      ["Stage", "Traffic %", "Duration", "Gate Criteria", "Rollback Trigger"],
      [
        ["Staging", "0% (mirror)", "Full test suite", "All integration tests pass, contract tests green", "Any test failure"],
        ["Canary", "5%", "15 minutes", "Error rate < 0.5%, p99 latency stable", "Error rate > 0.5%"],
        ["Partial", "25%", "15 minutes", "Error rate < 0.3%, no new alerts", "p99 increase > 50%"],
        ["Majority", "50%", "15 minutes", "All metrics stable", "Any SEV-1 or SEV-2 alert"],
        ["Full", "100%", "Continuous", "Monitoring continues post-deploy", "Manual trigger only"],
      ],
      [1200, 1200, 1400, 2800, 2760]
    ),
    p("Infrastructure changes (Terraform) require plan output review and explicit approval before apply. Database migrations run in a maintenance window with pre-migration backup verification."),
  ];
};
