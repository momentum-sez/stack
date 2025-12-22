# Licensing scaffolding pack

The licensing pack ships:

- **application forms** (JSON Schema + example YAML/JSON)
- **OPA/Rego rules** for eligibility + ongoing compliance checks
- **reporting templates** for periodic returns and incident reporting

Each license module under `modules/licensing/` follows the same structure:

```
module.yaml
README.md
forms/
  application.schema.json
  application.example.json
policy/
  eligibility.rego
  ongoing.rego
reporting/
  periodic-return.schema.json
  periodic-return.example.json
  incident-report.schema.json
```

License modules MUST also include `src/policy-to-code/map.yaml` so requirements map to attestations.

