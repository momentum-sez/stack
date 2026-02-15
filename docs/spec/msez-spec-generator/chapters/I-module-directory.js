const { chapterHeading, codeBlock, spacer } = require("../lib/primitives");

module.exports = function build_appendixI() {
  return [
    chapterHeading("Appendix I: Module Directory Structure"),
    ...codeBlock(
      "crates/msez-modules/src/\n" +
      "\u251C\u2500\u2500 compliance/          # Tensor, manifold, ZK circuits\n" +
      "\u251C\u2500\u2500 corridors/           # State sync, bridge, multilateral\n" +
      "\u251C\u2500\u2500 governance/          # Constitutional, voting, delegation\n" +
      "\u251C\u2500\u2500 financial/           # Accounts, payments, custody, FX\n" +
      "\u251C\u2500\u2500 regulatory/          # KYC, AML, sanctions, reporting\n" +
      "\u251C\u2500\u2500 licensing/           # Applications, monitoring, portability\n" +
      "\u251C\u2500\u2500 legal/               # Contracts, disputes, arbitration\n" +
      "\u251C\u2500\u2500 operational/         # HR, procurement, facilities\n" +
      "\u251C\u2500\u2500 corporate/           # Formation, cap table, dissolution (v0.4.44)\n" +
      "\u251C\u2500\u2500 identity/            # DID, KYC tiers, credentials (v0.4.44)\n" +
      "\u251C\u2500\u2500 tax/                 # Regimes, fees, incentives (v0.4.44)\n" +
      "\u251C\u2500\u2500 capital_markets/     # Securities, trading, CSD (v0.4.44)\n" +
      "\u2514\u2500\u2500 trade/               # LCs, documents, SCF (v0.4.44)"
    ),
    spacer(),
  ];
};
