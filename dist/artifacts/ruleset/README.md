# ruleset artifacts

This directory is a **content-addressed store** for ruleset descriptor JSON files.

Convention:

```text
dist/artifacts/ruleset/<digest>.*
```

Ruleset digests used by the stack are defined as:

```
SHA256(JCS(ruleset_descriptor_json))
```

Populate from the declared rulesets registry:

```bash
python -m tools.msez artifact index-rulesets
```
