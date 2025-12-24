# Templating and Overlays

This document specifies how MSEZ bundles are *rendered* for a particular jurisdiction/zone, and how
jurisdiction-specific changes are applied without forking base modules.

## Goals

The Stack MUST support:

- **Deterministic builds**: given the same module versions, overlays, and parameter values, the
  output bundle is bit-for-bit reproducible.
- **Non-fork customization**: local legal teams and regulators can apply jurisdiction-specific
  adjustments as overlays, while still consuming upstream updates.
- **Machine-verifiable provenance**: a third party can re-build the bundle and verify that it
  matches the published `stack.lock`.

## Template placeholders

Modules MAY contain template placeholders in text formats.

The v0.2 reference tooling supports two placeholder syntaxes (both are common in existing modules):

- `{{ VAR_NAME }}` — typically used in Akoma Ntoso XML templates
- `${var_name}` — typically used in YAML manifests (e.g., corridor manifests)

During `msez build`, placeholders are rendered from the *resolved parameter context*.

### Parameter context

The parameter context is constructed (in order):

1. Zone-level context (when building from `zone.yaml`): `jurisdiction_id`, `zone_id`, `zone_name`
2. Module defaults from `module.yaml -> parameters -> <param>.default`
3. Profile overrides from `profile.yaml -> modules[].params`
4. Zone overrides from `zone.yaml -> params_overrides[module_id]`

For convenience, both the original parameter key and an UPPERCASE alias are available.
For example, `jurisdiction_name` is exposed as both `jurisdiction_name` and `JURISDICTION_NAME`.

### Missing values

Rendering is allowed to be *partial* during development.

- In non-strict mode, missing variables SHOULD remain as placeholders.
- In strict mode (`--strict-render`), missing variables MUST fail the build.

## Overlays

Overlays are optional patch sets declared in `zone.yaml`.

An overlay MUST specify a `module_id` and one or more patch files.
Patch files MUST be in unified diff format and SHOULD be applicable using `git apply`.

Overlays are applied in list order, and patches within an overlay are applied in list order.

## Lockfile linkage

The `stack.lock` file MUST record:

- The exact module versions and content digests
- The resolved parameter values
- The ordered list of overlays and their digests

This information MUST be sufficient to reproduce the build output.
