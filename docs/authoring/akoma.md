# Writing Akoma Ntoso for MEZ modules

Akoma Ntoso (LegalDocML) is the canonical source format for legal texts in this stack.

## Rules

1. Every clause MUST have a stable `eId` so it can be referenced by:
   - policy-as-code rules
   - policy-to-code mapping
   - diffs and upgrades

2. Do not bake jurisdiction-specific values directly into core modules.
   Use parameters + overlays.

3. Keep definitions in a dedicated `definitions` section with stable anchors.

## Templates

Use `modules/legal/akn-templates/src/akn/` as the starting point:

- `act.template.xml`
- `regulation.template.xml`
- `bylaw.template.xml`

## Common placeholders

- `{{JURISDICTION_NAME}}`
- `{{ZONE_NAME}}`
- `{{AUTHORITY_NAME}}`
- `{{EFFECTIVE_DATE}}`

These placeholders are expanded by build tooling in deployments (optional).

