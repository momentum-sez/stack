const {
  partHeading, chapterHeading, h2,
  p,
  codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter49() {
  return [
    ...partHeading("PART XVII: DEPLOYMENT AND OPERATIONS"),

    chapterHeading("Chapter 49: Deployment Architecture"),

    // --- 49.1 Infrastructure Requirements ---
    h2("49.1 Infrastructure Requirements"),
    table(
      ["Component", "Minimum", "Recommended"],
      [
        ["Compute", "4 vCPU, 16 GB RAM", "8 vCPU, 32 GB RAM"],
        ["Storage", "100 GB SSD", "500 GB NVMe SSD"],
        ["Network", "100 Mbps", "1 Gbps"],
        ["Database", "PostgreSQL 15+", "PostgreSQL 16 with pgvector"],
        ["Container Runtime", "Docker 24+", "containerd 1.7+ with Kubernetes 1.29+"],
      ],
      [2000, 3200, 4160]
    ),
    spacer(),

    // --- 49.2 Deployment Profiles ---
    h2("49.2 Deployment Profiles"),
    table(
      ["Profile", "Services", "Resources", "Use Case"],
      [
        ["minimal", "Core MSEZ + single jurisdiction", "4 vCPU / 16 GB", "Development, testing"],
        ["standard", "Full MSEZ + 3 jurisdictions + corridors", "8 vCPU / 32 GB", "Single-zone production"],
        ["enterprise", "Full MSEZ + 10+ jurisdictions + full corridors", "32 vCPU / 128 GB", "Multi-zone production"],
        ["sovereign-govos", "Full MSEZ + GovOS + AI + national integration", "64+ vCPU / 256+ GB + GPU", "National deployment"],
      ],
      [1600, 3000, 2200, 2560]
    ),
    spacer(),

    // --- 49.3 Rust Binary Deployment ---
    h2("49.3 Rust Binary Deployment"),
    p("The msez CLI is a single statically-linked binary compiled from the msez-cli crate. No runtime dependencies beyond libc. Container images use Alpine Linux with the msez binary, producing images under 50 MB."),
    ...codeBlock(
`# Build the release binary
cargo build --release --bin msez

# Binary is at: target/release/msez
# Deploy directly or via container:
FROM alpine:3.19
COPY target/release/msez /usr/local/bin/msez
ENTRYPOINT ["/usr/local/bin/msez"]`
    ),
    spacer(),
  ];
};
