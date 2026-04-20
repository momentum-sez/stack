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
 * Usage with Claude Code or Codex CLI:
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
import { z } from "zod";

import {
  addCorridor,
  configureZone,
  deployZone,
  initZone,
  verifyDeploy,
} from "./tools/index.js";
import type {
  AddCorridorParams,
  CarryingDomain,
  ConfigureZoneParams,
  DeployMode,
  DeployZoneParams,
  FrictionProfile,
  InitZoneParams,
  RecognitionBasis,
  VerifyDeployParams,
} from "./tools/index.js";

const RECOGNITION_BASES = [
  "passport-only",
  "passport-and-msa",
  "full-mutual",
  "asymmetric-read-only",
] as const;
const FRICTION_PROFILES = ["low", "medium", "high", "custom"] as const;
const COMPLIANCE_DOMAINS = [
  "aml",
  "kyc",
  "sanctions",
  "tax",
  "securities",
  "corporate",
  "custody",
  "data_privacy",
  "licensing",
  "banking",
  "payments",
  "clearing",
  "settlement",
  "digital_assets",
  "employment",
  "immigration",
  "ip",
  "consumer_protection",
  "arbitration",
  "trade",
  "insurance",
  "anti_bribery",
  "sharia",
] as const;
const DEPLOY_MODES = ["dry_run", "execute", "instructions_only"] as const;

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

function resolveZonePath(zonePath: string): string {
  const expanded = expandHome(zonePath);
  return path.isAbsolute(expanded) ? expanded : path.resolve(STACK_DEPLOY_DIR, expanded);
}

function jsonContent(payload: unknown) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(payload, null, 2) }],
  };
}

function errorContent(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  return {
    isError: true as const,
    content: [{ type: "text" as const, text: message }],
  };
}

async function runTool<T>(fn: () => Promise<T>) {
  try {
    return jsonContent(await fn());
  } catch (error) {
    return errorContent(error);
  }
}

// Wraps server.tool to bound the MCP-SDK's handler-arg inference depth.
// Without this, TS hits TS2589 inferring z.infer<z.ZodObject<Shape>> for
// multi-field shapes that combine z.enum and z.array unions. Runtime
// parsing is unaffected — the SDK still validates args against the shape.
function bindTool<Args>(
  name: string,
  description: string,
  shape: z.ZodRawShape,
  handler: (args: Args) => Promise<unknown>,
): void {
  (server.tool as unknown as (
    n: string,
    d: string,
    s: z.ZodRawShape,
    h: (a: Args) => Promise<unknown>,
  ) => void)(name, description, shape, handler);
}

const initZoneShape: z.ZodRawShape = {
  zone_id: z.string().describe('Zone identifier to scaffold, e.g. "sc-sez-001"'),
  jurisdiction_id: z
    .string()
    .describe('Jurisdiction identifier to stamp into zone.yaml, e.g. "sc-sez"'),
  zone_name: z.string().describe("Human-readable zone name."),
  operator_email: z.string().describe("Operator contact email written into zone.yaml."),
  api_port: z.number().int().describe("API port to expose, 1-65535."),
  region: z.string().describe('Deployment region label, e.g. "ap-southeast-1".'),
  template_dir: z
    .string()
    .optional()
    .describe("Optional template root override. Defaults to STACK_TEMPLATE_DIR."),
  deploy_dir: z
    .string()
    .optional()
    .describe("Optional deploy root override. Defaults to STACK_DEPLOY_DIR."),
};

const configureZoneShape: z.ZodRawShape = {
  zone_path: z.string().describe("Absolute path or STACK_DEPLOY_DIR-relative zone directory."),
  config_changes: z
    .record(z.unknown())
    .describe("Dot-notation config overrides to apply into zone.yaml."),
  target_file: z
    .string()
    .optional()
    .describe("Override target file inside zone_path. Defaults to zone.yaml."),
  validate_schema: z
    .boolean()
    .optional()
    .describe("Validate the updated manifest against schemas/. Default true."),
  dry_run: z
    .boolean()
    .optional()
    .describe("Return diff without writing the file. Default false."),
};

const addCorridorShape: z.ZodRawShape = {
  zone_path: z.string().describe("Absolute path or STACK_DEPLOY_DIR-relative zone directory."),
  partner_zone_id: z
    .string()
    .describe('Peer zone identifier, e.g. "org.momentum.mez.zone.partner"'),
  partner_jurisdiction_id: z.string().describe("Peer jurisdiction identifier."),
  partner_endpoint: z.string().describe("Peer kernel endpoint URL."),
  recognition_basis: z
    .enum(RECOGNITION_BASES)
    .describe(
      "Recognition basis R: passport-only, passport-and-msa, full-mutual, asymmetric-read-only.",
    ),
  friction_profile: z
    .enum(FRICTION_PROFILES)
    .describe("Friction profile phi: low, medium, high, or custom."),
  custom_friction: z
    .record(z.unknown())
    .optional()
    .describe("Custom friction payload. Required when friction_profile=custom."),
  carrying_domains: z
    .array(z.enum(COMPLIANCE_DOMAINS))
    .describe("Compliance domains carried across the corridor (subset of the 23-domain tensor)."),
  expiry_days: z
    .number()
    .int()
    .optional()
    .describe("Days until corridor expiry. Default 365."),
  cryptographic_epoch: z
    .string()
    .optional()
    .describe("PQ hybrid-signature epoch label. Default 'current'."),
};

const deployZoneShape: z.ZodRawShape = {
  zone_path: z.string().describe("Absolute path or STACK_DEPLOY_DIR-relative zone directory."),
  mode: z.enum(DEPLOY_MODES).describe("Deployment mode: instructions_only, dry_run, or execute."),
  compose_file: z
    .string()
    .optional()
    .describe("Compose file relative to zone_path. Default deploy/docker-compose.yaml."),
  env_file: z.string().optional().describe("Optional --env-file relative to zone_path."),
  detach: z.boolean().optional().describe("Pass -d on execute. Default true."),
  build: z.boolean().optional().describe("Pass --build on execute. Default false."),
};

const verifyDeployShape: z.ZodRawShape = {
  zone_path: z.string().describe("Absolute path or STACK_DEPLOY_DIR-relative zone directory."),
  mez_api_url: z
    .string()
    .optional()
    .describe("Override mez-api base URL. Default zone.yaml api.url or http://localhost:8080."),
  timeout_ms: z
    .number()
    .int()
    .optional()
    .describe("Per-request timeout in ms. Default 10000."),
};

interface InitZoneArgs {
  zone_id: string;
  jurisdiction_id: string;
  zone_name: string;
  operator_email: string;
  api_port: number;
  region: string;
  template_dir?: string;
  deploy_dir?: string;
}

interface ConfigureZoneArgs {
  zone_path: string;
  config_changes: Record<string, unknown>;
  target_file?: string;
  validate_schema?: boolean;
  dry_run?: boolean;
}

interface AddCorridorArgs {
  zone_path: string;
  partner_zone_id: string;
  partner_jurisdiction_id: string;
  partner_endpoint: string;
  recognition_basis: RecognitionBasis;
  friction_profile: FrictionProfile;
  custom_friction?: Record<string, unknown>;
  carrying_domains: CarryingDomain[];
  expiry_days?: number;
  cryptographic_epoch?: string;
}

interface DeployZoneArgs {
  zone_path: string;
  mode: DeployMode;
  compose_file?: string;
  env_file?: string;
  detach?: boolean;
  build?: boolean;
}

interface VerifyDeployArgs {
  zone_path: string;
  mez_api_url?: string;
  timeout_ms?: number;
}

const server = new McpServer({
  name: "stack",
  version: "0.1.0",
});

bindTool(
  "init_zone",
  `Bootstrap a new Mass zone from the open-source Stack template.

Copies the filesystem template tree into STACK_DEPLOY_DIR/<zone_id>,
then rewrites zone.yaml so the scaffold has the right identifiers
before any runtime deployment occurs.

Use this first when an agent needs a clean zone workspace for a
jurisdiction, operator, or pilot deployment. This is the template-side
companion to @mass/mcp, which operates against a live runtime.`,
  initZoneShape,
  async (rawArgs) => {
    const args = rawArgs as InitZoneArgs;
    const params: InitZoneParams = {
      zone_id: args.zone_id,
      jurisdiction_id: args.jurisdiction_id,
      zone_name: args.zone_name,
      operator_email: args.operator_email,
      api_port: args.api_port,
      region: args.region,
      template_dir: args.template_dir ?? STACK_TEMPLATE_DIR,
      deploy_dir: args.deploy_dir ?? STACK_DEPLOY_DIR,
    };
    return runTool(() => initZone(params));
  },
);

bindTool(
  "configure_zone",
  `Apply dot-notation overrides to a zone's zone.yaml manifest.

Takes a target zone path plus a map of configuration overrides such as
profile.version, key_management.rotation_interval_days, or
national_adapters.tax.endpoint_url. Validates against the zone schema
when available and writes atomically.`,
  configureZoneShape,
  async (rawArgs) => {
    const args = rawArgs as ConfigureZoneArgs;
    const params: ConfigureZoneParams = {
      zone_path: resolveZonePath(args.zone_path),
      config_changes: args.config_changes,
      target_file: args.target_file,
      validate_schema: args.validate_schema,
      dry_run: args.dry_run,
    };
    return runTool(() => configureZone(params));
  },
);

bindTool(
  "add_corridor",
  `Create a bilateral corridor definition for a partner zone.

Writes corridors/<partner_zone_id>.yaml with the recognition basis R
and friction profile phi that govern cross-zone recognition and
interoperability across the declared compliance domains.`,
  addCorridorShape,
  async (rawArgs) => {
    const args = rawArgs as AddCorridorArgs;
    const params: AddCorridorParams = {
      zone_path: resolveZonePath(args.zone_path),
      partner_zone_id: args.partner_zone_id,
      partner_jurisdiction_id: args.partner_jurisdiction_id,
      partner_endpoint: args.partner_endpoint,
      recognition_basis: args.recognition_basis,
      friction_profile: args.friction_profile,
      custom_friction: args.custom_friction,
      carrying_domains: args.carrying_domains,
      expiry_days: args.expiry_days,
      cryptographic_epoch: args.cryptographic_epoch,
    };
    return runTool(() => addCorridor(params));
  },
);

bindTool(
  "deploy_zone",
  `Prepare or execute the docker-compose deployment for a zone.

Modes:
- instructions_only: return the docker compose command for a human operator to run.
- dry_run: invoke 'docker-compose config' and return the rendered compose output.
- execute: run 'docker-compose up -d'. Requires MASS_MCP_DEPLOY_AUTHORIZED=true.`,
  deployZoneShape,
  async (rawArgs) => {
    const args = rawArgs as DeployZoneArgs;
    const params: DeployZoneParams = {
      zone_path: resolveZonePath(args.zone_path),
      mode: args.mode,
      compose_file: args.compose_file,
      env_file: args.env_file,
      detach: args.detach,
      build: args.build,
    };
    return runTool(() => deployZone(params));
  },
);

bindTool(
  "verify_deploy",
  `Check the deployed mez-api surface for a zone.

Calls GET /v1/zone against the configured kernel endpoint and returns the
zone-status payload when reachable, or a structured warning when the
runtime cannot be contacted within the timeout.`,
  verifyDeployShape,
  async (rawArgs) => {
    const args = rawArgs as VerifyDeployArgs;
    const params: VerifyDeployParams = {
      zone_path: resolveZonePath(args.zone_path),
      mez_api_url: args.mez_api_url,
      timeout_ms: args.timeout_ms,
    };
    return runTool(() => verifyDeploy(params));
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
