# Authoring corridor modules

Corridors are modules of kind `corridor`.

## Required files

- `module.yaml`
- `corridor.yaml` (machine-readable manifest)
- `required-attestations.yaml`
- `docs/tradeoffs.md`
- `docs/playbook.md`

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

