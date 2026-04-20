#!/usr/bin/env node

/**
 * @mass/stack-mcp - MCP server for the Mass zone deployment template.
 *
 * Stack-MCP scaffolds Mass zones agent-natively from the open-source
 * filesystem template. It is the template-side control surface for
 * creating, shaping, and preparing a zone before runtime deployment.
 *
 * Configuration (environment variables):
 *   STACK_TEMPLATE_DIR - template root to copy from (default: ~/stack)
 *   STACK_DEPLOY_DIR   - directory where generated zones live (default: ./zones)
 *
 * Usage with Claude Code:
 *   {
 *     "mcpServers": {
 *       "stack": {
 *         "command": "node",
 *         "args": ["/absolute/path/to/sdk/mcp/dist/index.js"],
 *         "env": {
 *           "STACK_TEMPLATE_DIR": "/absolute/path/to/stack",
 *           "STACK_DEPLOY_DIR": "/absolute/path/to/zones"
 *         }
 *       }
 *     }
 *   }
 *
 * Usage with Codex CLI:
 *   {
 *     "mcpServers": {
 *       "stack": {
 *         "command": "node",
 *         "args": ["/absolute/path/to/sdk/mcp/dist/index.js"],
 *         "env": {
 *           "STACK_TEMPLATE_DIR": "/absolute/path/to/stack",
 *           "STACK_DEPLOY_DIR": "/absolute/path/to/zones"
 *         }
 *       }
 *     }
 *   }
 *
 * For runtime-side operations against a live zone, use the Mass-MCP
 * companion package at @mass/mcp.
 */

import os from "node:os";
import path from "node:path";

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import YAML from "yaml";
import { z } from "zod";

const STACK_TEMPLATE_DIR = expandHome(process.env["STACK_TEMPLATE_DIR"] ?? "~/stack");
const STACK_DEPLOY_DIR = path.resolve(expandHome(process.env["STACK_DEPLOY_DIR"] ?? "./zones"));

function expandHome(value: string): string {
  if (value === "~") {
    return os.homedir();
  }

  if (value.startsWith("~/")) {
    return path.join(os.homedir(), value.slice(2));
  }

  return value;
}

function normalizePath(value: string): string {
  return path.resolve(expandHome(value));
}

function shellQuote(value: string): string {
  return JSON.stringify(value);
}

function jsonContent(payload: unknown) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(payload, null, 2) }],
  };
}

function yamlPreview(payload: unknown): string {
  return YAML.stringify(payload).trim();
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function setDotPath(target: Record<string, unknown>, dotPath: string, value: unknown): void {
  const segments = dotPath.split(".").filter(Boolean);
  if (segments.length === 0) {
    return;
  }

  let cursor = target;
  for (let index = 0; index < segments.length; index += 1) {
    const segment = segments[index];
    const isLeaf = index === segments.length - 1;

    if (isLeaf) {
      cursor[segment] = value;
      return;
    }

    const current = cursor[segment];
    if (!isRecord(current)) {
      cursor[segment] = {};
    }

    cursor = cursor[segment] as Record<string, unknown>;
  }
}

function dotNotationOverlay(input: Record<string, unknown>): Record<string, unknown> {
  const overlay: Record<string, unknown> = {};
  for (const [dotPath, value] of Object.entries(input)) {
    setDotPath(overlay, dotPath, value);
  }
  return overlay;
}

function makeZoneManifestPreview(zoneId: string, jurisdictionId: string): Record<string, unknown> {
  return {
    zone_id: zoneId,
    jurisdiction_id: jurisdictionId,
    zone_name: zoneId,
    profile: {
      profile_id: "org.momentum.mez.profile.minimal-mvp",
      version: "0.4.44",
    },
    jurisdiction_stack: [jurisdictionId],
    compliance_domains: ["aml", "corporate", "kyc", "sanctions"],
    corridors: [],
  };
}

const server = new McpServer({
  name: "stack",
  version: "0.1.0",
});

server.tool(
  "init_zone",
  `Bootstrap a new Mass zone from the open-source Stack template.

Copies the filesystem template tree into STACK_DEPLOY_DIR/<zone_id>,
then rewrites zone.yaml so the scaffold has the right identifiers
before any runtime deployment occurs.

Use this first when an agent needs a clean zone workspace for a
jurisdiction, operator, or pilot deployment. This is the template-side
companion to @mass/mcp, which operates against a live runtime.`,
  {
    zone_id: z.string().describe(
      'Zone identifier to scaffold, e.g. "org.momentum.mez.zone.seychelles"',
    ),
    jurisdiction_id: z.string().describe(
      'Jurisdiction identifier to stamp into zone.yaml, e.g. "sc-sez"',
    ),
    template_dir: z.string().optional().describe(
      "Optional template root override. Defaults to STACK_TEMPLATE_DIR or ~/stack.",
    ),
  },
  async ({ zone_id, jurisdiction_id, template_dir }) => {
    const sourceTemplateDir = normalizePath(template_dir ?? STACK_TEMPLATE_DIR);
    const zonePath = path.join(STACK_DEPLOY_DIR, zone_id);

    return jsonContent({
      tool: "init_zone",
      status: "stub",
      template_dir: sourceTemplateDir,
      deploy_dir: STACK_DEPLOY_DIR,
      zone_path: zonePath,
      planned_actions: [
        `Copy the template tree from ${sourceTemplateDir} to ${zonePath}`,
        `Rewrite ${path.join(zonePath, "zone.yaml")} with the provided zone identifiers`,
        "Prepare the zone for follow-on configuration and deployment steps",
      ],
      zone_manifest_preview: yamlPreview(makeZoneManifestPreview(zone_id, jurisdiction_id)),
      note: "Filesystem copy and YAML substitution are scaffolded here and deferred to ST-2.",
    });
  },
);

server.tool(
  "configure_zone",
  `Apply dot-notation overrides to a zone's zone.yaml manifest.

Takes a target zone path plus a map of configuration overrides such as
profile.version, key_management.rotation_interval_days, or
national_adapters.tax.endpoint_url.

Use this after init_zone when an agent needs to specialize the generic
template for a real deployment. The live file-edit implementation is
deferred; this scaffold returns the normalized update plan.`,
  {
    zone_path: z.string().describe("Absolute or relative path to the zone root directory."),
    config_changes: z.record(z.unknown()).describe(
      "Dot-notation config overrides to apply into zone.yaml.",
    ),
  },
  async ({ zone_path, config_changes }) => {
    const normalizedZonePath = normalizePath(zone_path);
    const zoneYamlPath = path.join(normalizedZonePath, "zone.yaml");
    const overlay = dotNotationOverlay(config_changes);

    return jsonContent({
      tool: "configure_zone",
      status: "stub",
      zone_path: normalizedZonePath,
      zone_yaml_path: zoneYamlPath,
      config_changes,
      overlay_preview: yamlPreview(overlay),
      planned_actions: [
        `Load ${zoneYamlPath}`,
        "Apply dot-notation overrides into the parsed YAML object",
        "Write the updated zone manifest back to disk",
      ],
      note: "In-place YAML editing is deferred to ST-3.",
    });
  },
);

server.tool(
  "add_corridor",
  `Create a bilateral corridor definition for a partner zone.

Writes corridors/<partner_zone_id>.yaml with the recognition basis R
and friction profile phi that govern cross-zone recognition and
interoperability.

Use this when an operator needs to declare how this zone will recognize
entities, attestations, or workflows from another zone before any
runtime corridor exchange is attempted.`,
  {
    zone_path: z.string().describe("Absolute or relative path to the zone root directory."),
    partner_zone_id: z.string().describe(
      'Peer zone identifier, e.g. "org.momentum.mez.zone.partner"',
    ),
    recognition_basis: z.union([z.string(), z.record(z.unknown())]).describe(
      "Recognition basis R for the corridor. Accepts a symbolic name or structured payload.",
    ),
    friction_profile: z.union([z.string(), z.record(z.unknown())]).describe(
      "Friction profile phi for the corridor. Accepts a symbolic name or structured payload.",
    ),
  },
  async ({ zone_path, partner_zone_id, recognition_basis, friction_profile }) => {
    const normalizedZonePath = normalizePath(zone_path);
    const corridorPath = path.join(normalizedZonePath, "corridors", `${partner_zone_id}.yaml`);
    const corridorDocument = {
      corridor_id: `${path.basename(normalizedZonePath)}--${partner_zone_id}`,
      peer: {
        zone_id: partner_zone_id,
      },
      recognition_basis,
      friction_profile,
    };

    return jsonContent({
      tool: "add_corridor",
      status: "stub",
      zone_path: normalizedZonePath,
      corridor_path: corridorPath,
      corridor_preview: yamlPreview(corridorDocument),
      planned_actions: [
        `Create ${corridorPath}`,
        "Serialize the bilateral corridor parameters as YAML",
        "Make the new corridor available for deployment planning",
      ],
      note: "Corridor file creation is deferred to ST-4.",
    });
  },
);

server.tool(
  "deploy_zone",
  `Prepare the docker-compose deployment flow for a zone.

In dry_run mode this tool will eventually render the compose config for
inspection. In execute mode it will return the exact docker compose
command for a human operator to run.

This scaffold never executes deployment itself. That boundary is
intentional: agents can plan and explain the deployment, but the final
runtime launch remains an explicit operator action.`,
  {
    zone_path: z.string().describe("Absolute or relative path to the zone root directory."),
    mode: z.enum(["dry_run", "execute"]).describe(
      'Deployment preparation mode: "dry_run" previews config, "execute" returns the launch command.',
    ),
  },
  async ({ zone_path, mode }) => {
    const normalizedZonePath = normalizePath(zone_path);
    const composeFile = path.join(normalizedZonePath, "deploy", "docker-compose.yaml");
    const envFile = path.join(normalizedZonePath, ".env");
    const dryRunCommand = `docker compose -f ${shellQuote(composeFile)} --env-file ${shellQuote(envFile)} config`;
    const executeCommand = `docker compose -f ${shellQuote(composeFile)} --env-file ${shellQuote(envFile)} up -d`;

    return jsonContent({
      tool: "deploy_zone",
      status: "stub",
      zone_path: normalizedZonePath,
      mode,
      compose_file: composeFile,
      dry_run_command: dryRunCommand,
      execute_command: executeCommand,
      behavior:
        mode === "dry_run"
          ? "Would print the resolved docker-compose config without launching containers."
          : "Would return the docker compose up command for the operator to run manually.",
      note: "Compose rendering and deployment handoff are deferred to ST-5. No command was executed.",
    });
  },
);

server.tool(
  "verify_deploy",
  `Check the deployed mez-api surface for a zone.

Calls the zone endpoint exposed by a running deployment and returns the
response payload when reachable, or a warning when the runtime cannot
be contacted.

Use this after deployment to confirm that the scaffolded zone has come
up cleanly and is answering on the expected API surface. This scaffold
currently returns the verification plan instead of making the live call.`,
  {
    zone_path: z.string().describe("Absolute or relative path to the zone root directory."),
    health_check_url: z.string().optional().describe(
      "Optional override for the verification URL. Defaults to http://localhost:8080/v1/zone.",
    ),
  },
  async ({ zone_path, health_check_url }) => {
    const normalizedZonePath = normalizePath(zone_path);
    const resolvedHealthCheckUrl = health_check_url ?? "http://localhost:8080/v1/zone";

    return jsonContent({
      tool: "verify_deploy",
      status: "stub",
      zone_path: normalizedZonePath,
      health_check_url: resolvedHealthCheckUrl,
      planned_actions: [
        `GET ${resolvedHealthCheckUrl}`,
        "Parse the zone metadata response when reachable",
        "Return a warning payload if the endpoint is unreachable or malformed",
      ],
      warning: "Live runtime verification is deferred to ST-5 in this scaffold.",
    });
  },
);

async function main() {
  console.error(`[stack-mcp] template root: ${STACK_TEMPLATE_DIR}`);
  console.error(`[stack-mcp] deploy root: ${STACK_DEPLOY_DIR}`);

  const transport = new StdioServerTransport();
  await server.connect(transport);

  console.error("[stack-mcp] server running on stdio");
}

main().catch((error) => {
  console.error("[stack-mcp] fatal:", error);
  process.exit(1);
});
