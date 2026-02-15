const {
  chapterHeading, h2,
  p, p_runs, bold,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter24() {
  return [
    chapterHeading("Chapter 24: Multilateral Corridors"),

    // --- 24.1 Hub and Spoke Model ---
    h2("24.1 Hub and Spoke Model"),
    p("The hub-and-spoke model designates a single jurisdiction as the corridor hub through which all transactions between spoke jurisdictions are routed. The hub maintains a master state channel with each spoke and provides centralized compliance evaluation, netting, and settlement services. This model minimizes the number of bilateral agreements required: N jurisdictions need only N corridor agreements rather than N(N-1)/2."),
    p("The hub jurisdiction assumes additional governance responsibilities including dispute arbitration, fee collection, and compliance monitoring. Hub designation is determined by treaty agreement and typically falls to the jurisdiction with the deepest capital markets or strongest regulatory framework in the corridor group. In the current deployment, UAE/ADGM serves as hub for the GCC corridor group due to its established financial center infrastructure and existing bilateral corridors with Pakistan, Saudi Arabia, and other regional participants."),
    table(
      ["Characteristic", "Hub-and-Spoke", "Mesh"],
      [
        ["Agreements Required", "N (one per spoke)", "N(N-1)/2"],
        ["Single Point of Failure", "Yes (hub)", "No"],
        ["Settlement Latency", "Higher (two hops for cross-spoke)", "Lower (direct bilateral)"],
        ["Compliance Cost", "Lower (hub evaluates)", "Higher (each pair evaluates)"],
        ["Netting Efficiency", "High (centralized)", "Lower (bilateral only)"],
      ],
      [2400, 3200, 3760]
    ),
    spacer(),

    // --- 24.2 Mesh Model ---
    h2("24.2 Mesh Model"),
    p("The mesh model establishes N(N-1)/2 bilateral corridors among N participating jurisdictions, enabling direct transfers without intermediary routing. Each bilateral corridor operates independently with its own governance, compliance baseline, and state channel. The mesh model provides maximum resilience: failure of any single corridor affects only the two connected jurisdictions."),
    p("The quadratic scaling of bilateral agreements makes the pure mesh model practical only for small groups of tightly integrated jurisdictions. Mesh corridors share a common multilateral governance overlay that coordinates policy updates, manages shared watchlists, and provides a forum for collective dispute resolution. The overlay does not process transactions directly but ensures consistency across the bilateral corridors through periodic policy synchronization and compliance attestation exchange."),

    // --- 24.3 Hybrid Topology ---
    h2("24.3 Hybrid Topology"),
    p("Production deployments use a hybrid topology that combines hub-and-spoke for low-volume corridors with direct bilateral corridors for high-volume pairs. The PAK\u2194UAE corridor operates as a direct bilateral due to $10.1B annual volume, while lower-volume corridors (e.g., PAK\u2194KSA at $5.4B) may route through a hub when direct bilateral infrastructure is not yet established. The PathRouter automatically selects the optimal path based on current topology, fees, settlement latency, and compliance overhead."),

    // --- 24.4 Multilateral Netting ---
    h2("24.4 Multilateral Netting"),
    p("Multilateral netting reduces gross settlement obligations across the corridor group. At each netting cycle (configurable per corridor group, typically daily), the netting engine computes net positions across all participants. A five-party corridor group with 100 gross transactions may reduce to 5-10 net settlement obligations, reducing liquidity requirements and settlement risk. The \u03C0net circuit (approximately 20,000 constraints) produces a ZK proof that the net positions correctly represent the underlying gross transactions, enabling privacy-preserving netting verification without exposing individual transaction details."),
  ];
};
