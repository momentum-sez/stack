# Terminology & Normative Language

This specification uses the key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** as described in RFC 2119 and RFC 8174.

## Core terms

- **Stack Spec**: The normative specification describing the module system, manifests, interfaces, and conformance rules.
- **Module**: A versioned, self-contained unit that produces one or more artifacts (legal texts, regulatory rules, schemas, APIs, forms, workflows).
- **Variant**: A named implementation of a module with different policy choices (e.g., `dispute.arbitration-first` vs `dispute.courts-first`).
- **Profile**: A bundle of modules + versions + parameters representing a deployable “style” (e.g., `digital-financial-center`).
- **Zone Node**: An instantiated deployment of a profile in a real jurisdiction (a “Mass network node” in the project context).
- **Corridor**: A configuration + institutional agreement pattern enabling cross-node interoperability (passporting, recognition, settlement).

- **Corridor Agreement VC**: A Verifiable Credential used to express participant-specific acceptance of a corridor definition and define activation thresholds.
- **Agreement-set digest**: A content-addressed SHA256 digest over (definition VC payload hash + agreement VC payload hashes) used to pin an activated corridor state deterministically.
- **Activation blockers**: A list of `<partyDid>:<commitment>` strings identifying non-affirmative commitments that prevent corridor activation.
- **Governance module**: A module that implements decision/consent mechanisms (voting, delegation, quadratic mechanisms) for zone governance workflows.

- **Verifiable Credential (VC)**: A digitally signed data structure (per the W3C VC model) used in MEZ to bind critical artifacts (e.g., corridor manifests) to an issuer identity (typically a DID) in a tamper-evident way.
- **Proof**: A cryptographic signature attached to a VC. MEZ supports multi-proof credentials for multi-party co‑signing.

- **MCF (Momentum Canonical Form)**: The deterministic JSON serialization used for all digest computation. MCF is RFC 8785 JSON Canonicalization Scheme (JCS) with two additional safety coercions: (1) float rejection — numeric values not representable as i64/u64 MUST be rejected; (2) datetime truncation — RFC 3339 timestamps are normalized to second precision. See `spec/40-corridors.md` §5 for the normative definition.
- **Content-Addressed Storage (CAS)**: An artifact storage convention where the storage key is the SHA-256 digest of the artifact's canonical bytes. Used for lawpacks, regpacks, licensepacks, rulesets, and other versioned artifacts. See `spec/97-artifacts.md`.
- **ArtifactRef**: A typed reference to a content-addressed artifact, containing `artifact_type`, `digest_sha256`, and optional metadata. Defined in `schemas/artifact-ref.schema.json`.
- **Receipt chain**: An append-only sequence of corridor receipts forming a hash-chain seeded from `genesis_root`. Each receipt commits to the previous state root (`prev_root == final_state_root`) and computes a self-commitment (`next_root = SHA256(MCF(payload_without_proof_and_next_root))`).
- **MMR (Merkle Mountain Range)**: An append-only authenticated data structure used alongside the receipt hash-chain to provide efficient inclusion proofs over corridor receipts.
- **Checkpoint**: A signed snapshot of corridor state at a given height, binding `genesis_root`, `final_state_root`, `receipt_count`, digest sets, and the MMR commitment. Used for verifier bootstrap and watcher attestation.
- **Watcher**: An independent observer that monitors a corridor's receipt chain, attests to observed head state, and participates in fork detection via signed attestation credentials.
