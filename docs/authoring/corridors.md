# Authoring corridor modules

Corridors are modules of kind `corridor`.

## Required files

- `module.yaml`
- `corridor.yaml` (machine-readable manifest)
- `corridor.vc.json` (Corridor Definition VC: cryptographic binding)
- `trust-anchors.yaml`
- `key-rotation.yaml`
- `required-attestations.yaml`
- `docs/tradeoffs.md`
- `docs/playbook.md`

## Optional files

- `corridor.agreement.vc.json` (Corridor Agreement VC: participant acceptance + activation thresholds)

## Required content

Your corridor manifest MUST specify:

- settlement type and limits
- recognition / passporting scope (licenses, entities)
- required attestations and trust anchors
- dispute and supervisory escalation

Your tradeoffs doc MUST explain:

- why choose this corridor vs alternatives
- operational, regulatory, and reputational risks
- dependency footprint (banks, networks, issuers)

## Cryptographic binding

### Corridor Definition VC (required)

A corridor module MUST include a signed Corridor Definition VC referenced by `definition_vc_path` in `corridor.yaml`.

You can verify it with:

```bash
python -m tools.msez corridor verify modules/corridors/<name>
```

You can sign a VC with:

```bash
python -m tools.msez vc sign path/to/unsigned.vc.json --key path/to/private.jwk --out corridor.vc.json
```

### Corridor Agreement VC (optional; v0.3+ direction)

A Corridor Agreement VC makes the corridor **operationally activatable**, not just tamper‑evident.

It is participant‑specific and defines:

- **who** the participants are (DIDs + roles)
- **what** corridor definition they are agreeing to (binds to the Corridor Definition VC payload hash)
- **when** it is effective (optional)
- **how many signatures** are required before activation (threshold rules)

In `corridor.yaml`, set `agreement_vc_path`:

- as a single path to one multi‑signed VC file, OR
- as an array of VC paths (per‑participant VCs), if you want each party to publish their own agreement VC

#### Participant-specific agreement VCs (per-party files)

When using multiple agreement VC files, each party SHOULD set:

- `credentialSubject.party.id` = that party DID
- `credentialSubject.party.role` = role that MUST match its entry in `credentialSubject.participants`
- (optional) `credentialSubject.party_terms` for party-specific carveouts
- (optional) `credentialSubject.commitment` = `agree` (default) for an affirmative commitment.
  Any other verb (e.g., `withdraw`, `suspend`) represents a **non‑affirmative** status and will **block activation**
  unless that verb is included in `credentialSubject.activation.accept_commitments` (default: `["agree"]`).

Each agreement VC file MUST be signed by that party. Validators count the party’s signature toward activation thresholds.

Status lock (v0.3+): `agreement_vc_path` MUST contain **at most one current agreement VC per party DID**.
If the same `party.id` appears twice, validation fails. This makes party status updates explicit: replace the party’s VC
file (or update `agreement_vc_path` to point to the party’s latest VC) rather than accumulating conflicting versions.

Authoring helpers (v0.3+):

```bash
# Generate a new dev Ed25519 key (writes a private JWK, prints the derived did:key)
python -m tools.msez vc keygen --out keys/my-dev.ed25519.jwk

# Scaffold a corridor definition VC from corridor.yaml + artifact hashes
python -m tools.msez corridor vc-init-definition <corridor-dir> \
  --issuer did:key:z... \
  --out corridor.vc.unsigned.json

# Scaffold a participant-specific corridor agreement VC (party defaults issuer)
python -m tools.msez corridor vc-init-agreement <corridor-dir> \
  --party did:key:z... \
  --role zone_authority \
  --out agreement.zone-a.unsigned.json
```

Example flow:

```bash
# Zone A signs its own agreement VC
python -m tools.msez vc sign agreement.zone-a.unsigned.json \
  --key docs/examples/keys/zone-a.ed25519.jwk \
  --out corridor.agreement.zone-a.vc.json

# Zone B signs its own agreement VC
python -m tools.msez vc sign agreement.zone-b.unsigned.json \
  --key docs/examples/keys/zone-b.ed25519.jwk \
  --out corridor.agreement.zone-b.vc.json
```

Check activation status (human-readable):

```bash
python -m tools.msez corridor status <corridor-dir>
```

Use `--json` to emit machine-readable status.

To co‑sign a single agreement VC file (recommended):

```bash
# signer 1
python -m tools.msez vc sign docs/examples/vc/unsigned.corridor-agreement.json \
  --key docs/examples/keys/dev.ed25519.jwk \
  --out corridor.agreement.vc.json

# signer 2 appends a second proof to the same file
python -m tools.msez vc sign corridor.agreement.vc.json \
  --key path/to/second-signer.jwk \
  --out corridor.agreement.vc.json
```


To compute the binding hash for a Corridor Definition VC:

```bash
python -m tools.msez vc payload-hash path/to/corridor.vc.json
```

Then verify that the agreement is valid **and activation thresholds are met**:

```bash
python -m tools.msez corridor verify <corridor-dir>
```
