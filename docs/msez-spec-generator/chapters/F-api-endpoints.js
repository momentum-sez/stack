const { chapterHeading, table, h2, p } = require("../lib/primitives");

module.exports = function build_appendixF() {
  return [
    chapterHeading("Appendix F: API Endpoint Reference"),

    h2("F.1 Mass API Endpoints (External, Live)"),
    p("The five Mass primitives are deployed Java/Spring Boot services. The SEZ Stack communicates with these exclusively through the msez-mass-client crate. Direct HTTP calls from any other crate are forbidden by architectural invariant (INV-2, Appendix E)."),
    table(
      ["API", "Base URL", "Swagger"],
      [
        ["Organization Info", "organization-info.api.mass.inc", "https://organization-info.api.mass.inc/organization-info/swagger-ui/index.html"],
        ["Investment Info", "investment-info-production.herokuapp.com", "https://investment-info-production-4f3779c81425.herokuapp.com/investment-info/swagger-ui/index.html"],
        ["Treasury Info", "treasury-info.api.mass.inc", "https://treasury-info.api.mass.inc/treasury-info/swagger-ui/index.html"],
        ["Consent Info", "consent.api.mass.inc", "https://consent.api.mass.inc/consent-info/swagger-ui/index.html"],
        ["Templating Engine", "templating-engine-prod.herokuapp.com", "https://templating-engine-prod-5edc768c1f80.herokuapp.com/templating-engine/swagger-ui/index.html"],
      ],
      [2000, 3400, 3960]
    ),

    h2("F.2 SEZ Stack API Routes (msez-api, Axum)"),
    p("The SEZ Stack exposes its own HTTP API via msez-api. These routes compose compliance evaluation, Mass API orchestration, credential issuance, and corridor management into jurisdiction-aware operations."),

    table(
      ["Route Group", "Method", "Path", "Description"],
      [
        ["Corridor Mgmt", "POST", "/api/v1/corridors", "Create a new trade corridor between two jurisdictions"],
        ["Corridor Mgmt", "GET", "/api/v1/corridors/:id", "Retrieve corridor state, receipt chain head, fork status"],
        ["Corridor Mgmt", "POST", "/api/v1/corridors/:id/activate", "Transition corridor from Defined to Active via FSM"],
        ["Corridor Mgmt", "POST", "/api/v1/corridors/:id/sync", "Synchronize corridor state, trigger netting reconciliation"],
        ["Corridor Mgmt", "GET", "/api/v1/corridors/:id/receipts", "List receipt chain entries with MMR inclusion proofs"],

        ["Asset Mgmt", "POST", "/api/v1/assets", "Register asset with jurisdiction-specific classification"],
        ["Asset Mgmt", "GET", "/api/v1/assets/:id", "Retrieve asset details and compliance attestation status"],
        ["Asset Mgmt", "POST", "/api/v1/assets/:id/transfer", "Initiate cross-border asset transfer with corridor binding"],

        ["Compliance", "POST", "/api/v1/compliance/evaluate", "Evaluate tensor across 20 domains for entity + jurisdiction"],
        ["Compliance", "GET", "/api/v1/compliance/tensor/:id", "Retrieve a committed compliance tensor snapshot"],
        ["Compliance", "POST", "/api/v1/compliance/manifold/path", "Compute optimal compliance path through the manifold"],

        ["Credentials", "POST", "/api/v1/credentials/issue", "Issue W3C Verifiable Credential with Ed25519 proof"],
        ["Credentials", "POST", "/api/v1/credentials/verify", "Verify VC signature and check revocation status"],
        ["Credentials", "GET", "/api/v1/credentials/:id", "Retrieve a previously issued credential by ID"],
        ["Credentials", "POST", "/api/v1/credentials/selective-disclose", "Generate BBS+ selective disclosure proof from a VC"],

        ["Agentic", "POST", "/api/v1/agentic/triggers", "Register autonomous trigger (20 types x 5 domains)"],
        ["Agentic", "GET", "/api/v1/agentic/triggers/:id", "Retrieve trigger config and execution history"],
        ["Agentic", "POST", "/api/v1/agentic/evaluate", "Evaluate policy engine and fire matching triggers"],
        ["Agentic", "DELETE", "/api/v1/agentic/triggers/:id", "Deactivate and archive a trigger"],

        ["Settlement", "POST", "/api/v1/settlement/initiate", "Initiate settlement with SWIFT pacs.008 adapter"],
        ["Settlement", "GET", "/api/v1/settlement/:id/status", "Check settlement status and reconciliation state"],

        ["Regulator", "GET", "/api/v1/regulator/dashboard", "Compliance overview across all entities in zone"],
        ["Regulator", "GET", "/api/v1/regulator/filings/:jurisdiction", "Filing calendar and submission status for jurisdiction"],

        ["Mass Proxy", "ANY", "/api/v1/mass/organizations/**", "Proxy to organization-info (transitional; target: orchestration)"],
        ["Mass Proxy", "ANY", "/api/v1/mass/investments/**", "Proxy to investment-info (transitional; target: orchestration)"],
        ["Mass Proxy", "ANY", "/api/v1/mass/treasury/**", "Proxy to treasury-info (transitional; target: orchestration)"],
        ["Mass Proxy", "ANY", "/api/v1/mass/consent/**", "Proxy to consent-info (transitional; target: orchestration)"],
      ],
      [1400, 700, 3000, 4260]
    ),

    p("Note: Mass Proxy routes are transitional. The target architecture replaces each passthrough with an orchestration endpoint that composes compliance evaluation, Mass API calls, VC issuance, and corridor state updates into a single atomic operation."),
  ];
};
