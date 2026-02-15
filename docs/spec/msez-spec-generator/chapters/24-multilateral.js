const {
  chapterHeading, h2,
  p
} = require("../lib/primitives");

module.exports = function build_chapter24() {
  return [
    chapterHeading("Chapter 24: Multilateral Corridors"),

    // --- 24.1 Hub and Spoke Model ---
    h2("24.1 Hub and Spoke Model"),
    p("The hub-and-spoke model designates a single jurisdiction as the corridor hub through which all transactions between spoke jurisdictions are routed. The hub maintains a master state channel with each spoke and provides centralized compliance evaluation, netting, and settlement services. This model minimizes the number of bilateral agreements required: N jurisdictions need only N corridor agreements rather than N(N-1)/2. The hub jurisdiction assumes additional governance responsibilities including dispute arbitration, fee collection, and compliance monitoring. Hub designation is determined by treaty agreement and typically falls to the jurisdiction with the deepest capital markets or strongest regulatory framework in the corridor group."),

    // --- 24.2 Mesh Model ---
    h2("24.2 Mesh Model"),
    p("The mesh model establishes n(n-1)/2 bilateral corridors among N participating jurisdictions, enabling direct transfers without intermediary routing. Each bilateral corridor operates independently with its own governance, compliance baseline, and state channel. The mesh model provides maximum resilience: failure of any single corridor affects only the two connected jurisdictions. However, the quadratic scaling of bilateral agreements makes this model practical only for small groups of tightly integrated jurisdictions. Mesh corridors share a common multilateral governance overlay that coordinates policy updates, manages shared watchlists, and provides a forum for collective dispute resolution. The overlay does not process transactions directly but ensures consistency across the bilateral corridors through periodic policy synchronization and compliance attestation exchange."),
  ];
};
