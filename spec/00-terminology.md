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
