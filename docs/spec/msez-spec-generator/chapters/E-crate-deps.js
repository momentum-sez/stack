const { chapterHeading, codeBlock, spacer } = require("../lib/primitives");

module.exports = function build_appendixE() {
  return [
    chapterHeading("Appendix E: Rust Crate Dependency Graph"),
    ...codeBlock(
      "msez-cli\n" +
      "  \u251C\u2500\u2500 msez-govos\n" +
      "  \u2502   \u251C\u2500\u2500 msez-mass-bridge\n" +
      "  \u2502   \u2502   \u251C\u2500\u2500 msez-pack\n" +
      "  \u2502   \u2502   \u251C\u2500\u2500 msez-tensor\n" +
      "  \u2502   \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u2502   \u251C\u2500\u2500 msez-modules\n" +
      "  \u2502   \u2514\u2500\u2500 msez-corridor\n" +
      "  \u251C\u2500\u2500 msez-vm\n" +
      "  \u2502   \u251C\u2500\u2500 msez-tensor\n" +
      "  \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u251C\u2500\u2500 msez-migration\n" +
      "  \u2502   \u251C\u2500\u2500 msez-corridor\n" +
      "  \u2502   \u251C\u2500\u2500 msez-watcher\n" +
      "  \u2502   \u2514\u2500\u2500 msez-tensor\n" +
      "  \u251C\u2500\u2500 msez-watcher\n" +
      "  \u2502   \u2514\u2500\u2500 msez-core\n" +
      "  \u2514\u2500\u2500 msez-governance\n" +
      "      \u2514\u2500\u2500 msez-core\n" +
      "\n" +
      "Shared dependencies: serde, tokio, chrono, ed25519-dalek, arkworks, halo2"
    ),
    spacer(),
  ];
};
