# Profile system (normative)

A **profile** is a deployable bundle that selects:

- modules
- variants
- parameter values

Profiles MUST be validated against `schemas/profile.schema.json`.

Profiles SHOULD be published as stable style baselines such as:

- minimal-mvp
- digital-native-free-zone
- digital-financial-center
- charter-city

Profiles SHOULD pin module versions. Deployments MUST lock a resolved profile in `stack.lock`.

