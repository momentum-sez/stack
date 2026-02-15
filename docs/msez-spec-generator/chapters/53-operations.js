const {
  chapterHeading, h2,
  p
} = require("../lib/primitives");

module.exports = function build_chapter53() {
  return [
    chapterHeading("Chapter 53: Operations Management"),

    // --- 53.1 Monitoring and Alerting ---
    h2("53.1 Monitoring and Alerting"),
    p("The MSEZ Stack exposes Prometheus metrics on /metrics for all operational dimensions. Key metric families include: msez_api_request_duration_seconds (histogram, by route and status), msez_corridor_state_transitions_total (counter, by corridor and transition type), msez_tensor_evaluation_duration_seconds (histogram, by jurisdiction and domain count), msez_pack_check_results (counter, by pack type and outcome), msez_mass_client_request_duration_seconds (histogram, by Mass primitive and endpoint), msez_vc_issuance_total (counter, by credential type), and msez_db_pool_connections (gauge, active/idle/waiting). Alerting rules fire on: API p99 latency exceeding 500ms, corridor state machine stuck for more than 5 minutes, Mass API error rate above 1%, database connection pool exhaustion above 80%, and certificate expiration within 14 days."),

    // --- 53.2 Incident Response ---
    h2("53.2 Incident Response"),
    p("Incidents are classified into four severity levels. SEV-1 (Critical): complete service outage, data integrity breach, or security compromise requiring immediate response within 15 minutes and resolution within 4 hours. SEV-2 (Major): degraded service affecting multiple users, corridor processing delays, or compliance evaluation failures requiring response within 30 minutes and resolution within 8 hours. SEV-3 (Minor): isolated feature failures, single-user impact, or non-critical monitoring gaps requiring response within 2 hours and resolution within 24 hours. SEV-4 (Low): cosmetic issues, documentation gaps, or optimization opportunities addressed during normal development cycles. Each severity level has defined escalation paths, communication templates, and post-incident review requirements."),

    // --- 53.3 Change Management ---
    h2("53.3 Change Management"),
    p("All production changes follow a staged rollout process. Changes are first deployed to a staging environment that mirrors production topology. Automated integration tests validate API compatibility, corridor state machine transitions, tensor evaluation correctness, and Mass API client behavior against contract tests. After staging validation, production deployment uses a canary strategy: 5% of traffic is routed to the new version for 15 minutes while error rates, latency, and compliance evaluation results are monitored. If canary metrics remain within thresholds, traffic is gradually shifted (25%, 50%, 100%) over 45 minutes. Rollback is automatic if error rate exceeds 0.5% or p99 latency increases by more than 50% during any canary phase. Infrastructure changes (Terraform) require plan review and approval before apply."),
  ];
};
