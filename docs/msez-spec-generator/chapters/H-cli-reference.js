const { chapterHeading, p, table, spacer } = require("../lib/primitives");

module.exports = function build_appendixH() {
  return [
    chapterHeading("Appendix H: CLI Reference"),
    p("The msez binary provides a clap-derived CLI:"),
    table(
      ["Command", "Subcommand", "Description"],
      [
        ["msez init", "--profile <name> --jurisdiction <id>", "Initialize a new jurisdiction deployment"],
        ["msez pack", "import / verify / list / diff", "Pack Trilogy management"],
        ["msez deploy", "--target docker|aws|k8s", "Deploy infrastructure"],
        ["msez verify", "--all | --service <name>", "Verify deployment health"],
        ["msez corridor", "activate / status / sync", "Corridor management"],
        ["msez migrate", "up / down / status", "Database migrations"],
        ["msez artifact", "graph verify / bundle attest", "Artifact graph operations"],
        ["msez tensor", "evaluate / slice / commit", "Compliance tensor operations"],
        ["msez watcher", "register / bond / attest", "Watcher economy operations"],
        ["msez govos", "deploy / status / handover", "GovOS lifecycle management"],
      ],
      [1800, 3200, 4360]
    ),
    spacer(),
  ];
};
