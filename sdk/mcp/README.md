# Stack-MCP

`@mass/stack-mcp` is the MCP server for the open-source Mass Stack template.
It lets an AI agent scaffold and prepare a filesystem-based zone deployment
without touching proprietary kernel internals.

This package is the template-side companion to `@mass/mcp`:

- `@mass/stack-mcp` works on `zone.yaml`, `corridors/`, `operations/`, and
  `deploy/docker-compose.yaml`
- `@mass/mcp` works against a live Mass runtime over the API surface

Current status: this is a scaffold release. The tools are registered and typed,
but they currently return placeholder JSON previews instead of mutating files or
calling a live deployment.

## Tools

- `init_zone` - preview a new zone scaffold under `STACK_DEPLOY_DIR`
- `configure_zone` - preview dot-notation edits to `zone.yaml`
- `add_corridor` - preview a bilateral corridor definition
- `deploy_zone` - preview docker compose config or the launch command
- `verify_deploy` - preview a runtime health check against `/v1/zone`

## Build

```bash
cd sdk/mcp
npm install
npm run build
```

Node 20+ is required.

## Environment

- `STACK_TEMPLATE_DIR` - template source directory, defaults to `~/stack`
- `STACK_DEPLOY_DIR` - generated zone directory, defaults to `./zones`

## Claude Code

Add this to your MCP configuration:

```json
{
  "mcpServers": {
    "stack": {
      "command": "node",
      "args": ["/absolute/path/to/stack/sdk/mcp/dist/index.js"],
      "env": {
        "STACK_TEMPLATE_DIR": "/absolute/path/to/stack",
        "STACK_DEPLOY_DIR": "/absolute/path/to/zones"
      }
    }
  }
}
```

## Codex CLI

Add the same server block to your Codex CLI MCP configuration:

```json
{
  "mcpServers": {
    "stack": {
      "command": "node",
      "args": ["/absolute/path/to/stack/sdk/mcp/dist/index.js"],
      "env": {
        "STACK_TEMPLATE_DIR": "/absolute/path/to/stack",
        "STACK_DEPLOY_DIR": "/absolute/path/to/zones"
      }
    }
  }
}
```

## Example Sessions

Example: deploy a Seychelles zone

```text
Create a new zone with init_zone using zone_id org.momentum.mez.zone.seychelles
and jurisdiction_id sc-sez.
```

```text
Configure the zone so profile.version is 0.4.44, jurisdiction_stack.0 is
sc-sez, and compliance_domains includes aml, kyc, corporate, and sanctions.
```

```text
Add a corridor from the Seychelles zone to org.momentum.mez.zone.partner with a
recognition basis of mutual-recognition and a friction profile of low-friction.
```

```text
Prepare the deployment in dry_run mode and show me the docker compose config you
would render.
```

```text
Verify the deployment at http://localhost:8080/v1/zone.
```

Because this release is scaffold-only, each tool returns a structured preview of
the action rather than executing it.

## Runtime Pairing

Use `@mass/stack-mcp` to create and shape a zone. Once the zone is deployed and
`mez-api` is reachable, switch to `@mass/mcp` for runtime-side operations such
as discovering operations, planning workflows, and inspecting entity state.
