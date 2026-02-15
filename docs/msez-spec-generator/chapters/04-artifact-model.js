const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  definition, codeBlock, table,
  spacer, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter04() {
  return [
    ...partHeading("PART III: CONTENT-ADDRESSED ARTIFACT MODEL"),
    chapterHeading("Chapter 4: Artifact Architecture"),

    // --- 4.1 Digest Type ---
    h2("4.1 Digest Type"),
    p("Every artifact in the MSEZ Stack is content-addressed. The artifact identifier is its cryptographic digest. This provides integrity (modification changes the identifier), deduplication (identical content shares an identifier), and auditability (any party can verify artifact integrity)."),
    definition("Definition 4.1 (Artifact Reference).", "An artifact reference contains artifact_type (indicating interpretation), digest_sha256 (canonical identifier), and uri_hints (suggestions for retrieval). The digest provides the canonical identifier; uri_hints do not affect identity."),
    p_runs([bold("Stability Invariant."), " For all valid JSON serializations j1, j2 of the same logical object A: Digest(j1) = Digest(j2). This is guaranteed by JCS canonicalization."]),
    ...codeBlock(
      "/// Every artifact is content-addressed via its canonical digest.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct Artifact {\n" +
      "    pub artifact_type: ArtifactType,\n" +
      "    pub digest: Digest,\n" +
      "    pub payload: Vec<u8>,\n" +
      "    pub metadata: ArtifactMetadata,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub enum ArtifactType {\n" +
      "    Lawpack, Regpack, Licensepack, Schema,\n" +
      "    VerifiableCredential, Receipt, Checkpoint,\n" +
      "    ProofKey, TransitionType, Blob,\n" +
      "}"
    ),
    spacer(),

    // --- 4.2 Artifact Type Registry ---
    h2("4.2 Artifact Type Registry"),
    table(
      ["Type", "MIME Type", "Description"],
      [
        ["lawpack", "application/vnd.momentum.lawpack+zip", "Legal corpus with Akoma Ntoso documents"],
        ["regpack", "application/vnd.momentum.regpack+zip", "Dynamic regulatory state container"],
        ["licensepack", "application/vnd.momentum.licensepack+zip", "Live license registry snapshot"],
        ["genesis", "application/vnd.momentum.genesis+json", "Smart Asset genesis document"],
        ["receipt", "application/vnd.momentum.receipt+json", "State transition receipt"],
        ["checkpoint", "application/vnd.momentum.checkpoint+json", "MMR checkpoint"],
        ["vc", "application/vc+json", "W3C Verifiable Credential"],
        ["proof", "application/vnd.momentum.proof+bin", "ZK proof bytes"],
        ["corridor-def", "application/vnd.momentum.corridor+json", "Corridor definition VC"],
        ["tensor", "application/vnd.momentum.tensor+json", "Compliance tensor definition"],
        ["attestation", "application/vnd.momentum.attestation+json", "Compliance attestation"],
      ],
      [1800, 4200, 3360]
    ),
    spacer(),

    // --- 4.3 Artifact Closure and Availability ---
    h2("4.3 Artifact Closure and Availability"),
    definition("Definition 4.3 (Artifact Closure).", "The transitive closure of artifact A is the set of all artifacts reachable by following references: Closure(A) = {A} \u222A \u222A{Closure(resolve(r)) : r \u2208 refs(A)}."),
    p_runs([bold("Axiom 4.1 (Availability Enforcement)."), " A proof is valid only if all artifacts in its artifact_bundle_root are retrievable by authorized parties. Enforcement levels: Best-Effort (S13 off) where provers commit to bundle root and auditors fetch out-of-band, and Enforced (S13 on) where DA committees verify retrievability before block acceptance."]),
    p("The CLI provides artifact graph operations:"),
    ...codeBlock(
      "# Verify artifact graph closure with strict digest recomputation\n" +
      "msez artifact graph verify transition-types <digest> --strict --json\n" +
      "\n" +
      "# Generate witness bundle for offline transfer\n" +
      "msez artifact graph verify transition-types <digest> --bundle /tmp/witness.zip\n" +
      "\n" +
      "# Attest (sign) a witness bundle for provenance\n" +
      "msez artifact bundle attest /tmp/witness.zip \\\n" +
      "  --issuer did:example:watcher \\\n" +
      "  --sign --key keys/dev.ed25519.jwk \\\n" +
      "  --out /tmp/witness.attestation.vc.json"
    ),

    pageBreak()
  ];
};
