import { execFile as execFileCallback } from "node:child_process";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";
import { parse, stringify } from "yaml";

const execFile = promisify(execFileCallback);
const EXCLUDED_NAMES = new Set([".claude", ".git", "dist", "node_modules"]);

export const ZONE_ID_PATTERN = /^[a-z0-9][a-z0-9-]{2,30}[a-z0-9]$/;

export interface InitZoneParams {
  zone_id: string;
  jurisdiction_id: string;
  template_dir?: string;
  deploy_dir?: string;
  zone_name: string;
  operator_email: string;
  api_port: number;
  region: string;
}

export interface InitZoneResult {
  zone_path: string;
  files_copied: number;
  template_version: string;
  parameters_applied: {
    zone_id: string;
    jurisdiction_id: string;
    zone_name: string;
    operator_email: string;
    api_port: number;
    region: string;
  };
  next_steps: string[];
}

type ZoneDocument = Record<string, unknown>;

export async function initZone(params: InitZoneParams): Promise<InitZoneResult> {
  validateParams(params);

  const templateDir = normalizePath(params.template_dir ?? "~/stack");
  const deployDir = normalizePath(params.deploy_dir ?? "./zones");
  const zonePath = path.join(deployDir, params.zone_id);

  const templateStat = await safeStat(templateDir);
  if (!templateStat?.isDirectory()) {
    throw new Error(`template_dir is not a directory: ${templateDir}`);
  }

  if (await safeStat(zonePath)) {
    throw new Error(`zone already exists: ${zonePath}`);
  }

  await fs.mkdir(deployDir, { recursive: true });
  await fs.cp(templateDir, zonePath, {
    recursive: true,
    filter: (source) => !EXCLUDED_NAMES.has(path.basename(source)),
  });

  const zoneYamlPath = path.join(zonePath, "zone.yaml");
  const manifestStat = await safeStat(zoneYamlPath);
  if (!manifestStat?.isFile()) {
    throw new Error(`zone.yaml not found in copied template: ${zoneYamlPath}`);
  }

  const manifest = parse(await fs.readFile(zoneYamlPath, "utf8"));
  if (!isRecord(manifest)) {
    throw new Error(`zone.yaml must parse to an object: ${zoneYamlPath}`);
  }

  applyZoneManifest(manifest, params);
  await fs.writeFile(zoneYamlPath, stringify(manifest, null, { indent: 2 }));

  const filesCopied = await countFiles(zonePath);
  const templateVersion = await resolveTemplateVersion(templateDir);
  const parametersApplied = {
    zone_id: params.zone_id,
    jurisdiction_id: params.jurisdiction_id,
    zone_name: params.zone_name,
    operator_email: params.operator_email,
    api_port: params.api_port,
    region: params.region,
  };

  return {
    zone_path: zonePath,
    files_copied: filesCopied,
    template_version: templateVersion,
    parameters_applied: parametersApplied,
    next_steps: [
      `Review ${path.join(zonePath, "zone.yaml")} for jurisdiction-specific settings.`,
      `Run configure_zone against ${zonePath} for any additional manifest overrides.`,
      `Use ${path.join(zonePath, "deploy")} when the scaffold is ready to launch.`,
    ],
  };
}

function validateParams(params: InitZoneParams): void {
  if (!ZONE_ID_PATTERN.test(params.zone_id)) {
    throw new Error(
      `zone_id must match ${ZONE_ID_PATTERN.source}: ${params.zone_id}`,
    );
  }

  for (const [key, value] of [
    ["jurisdiction_id", params.jurisdiction_id],
    ["zone_name", params.zone_name],
    ["operator_email", params.operator_email],
    ["region", params.region],
  ] as const) {
    if (value.trim().length === 0) {
      throw new Error(`${key} is required`);
    }
  }

  if (!Number.isInteger(params.api_port) || params.api_port < 1 || params.api_port > 65535) {
    throw new Error(`api_port must be an integer between 1 and 65535: ${params.api_port}`);
  }
}

function applyZoneManifest(manifest: ZoneDocument, params: InitZoneParams): void {
  setPreferredRootKey(manifest, ["zone_id", "id"], params.zone_id);
  setPreferredRootKey(manifest, ["zone_name", "name"], params.zone_name);
  setPreferredRootKey(manifest, ["jurisdiction_id", "jurisdiction"], params.jurisdiction_id);
  setPath(manifest, ["api", "port"], params.api_port);
  setPath(manifest, ["operator", "email"], params.operator_email);
  setPath(manifest, ["region"], params.region);
}

function setPreferredRootKey(target: ZoneDocument, keys: string[], value: unknown): void {
  const existingKey = keys.find((key) => key in target);
  target[existingKey ?? keys[0]] = value;
}

function setPath(target: ZoneDocument, segments: string[], value: unknown): void {
  let cursor: ZoneDocument = target;
  for (let index = 0; index < segments.length - 1; index += 1) {
    const segment = segments[index];
    const current = cursor[segment];
    if (!isRecord(current)) {
      cursor[segment] = {};
    }
    cursor = cursor[segment] as ZoneDocument;
  }

  cursor[segments[segments.length - 1]] = value;
}

async function resolveTemplateVersion(templateDir: string): Promise<string> {
  try {
    const { stdout } = await execFile("git", ["rev-parse", "HEAD"], { cwd: templateDir });
    return stdout.trim();
  } catch {
    return "unknown";
  }
}

async function countFiles(root: string): Promise<number> {
  let count = 0;
  for (const entry of await fs.readdir(root, { withFileTypes: true })) {
    if (EXCLUDED_NAMES.has(entry.name)) {
      continue;
    }

    const entryPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      count += await countFiles(entryPath);
      continue;
    }

    if (entry.isFile()) {
      count += 1;
    }
  }

  return count;
}

function normalizePath(value: string): string {
  return path.resolve(expandHome(value));
}

function expandHome(value: string): string {
  if (value === "~") {
    return os.homedir();
  }

  if (value.startsWith("~/")) {
    return path.join(os.homedir(), value.slice(2));
  }

  return value;
}

function isRecord(value: unknown): value is ZoneDocument {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

async function safeStat(targetPath: string) {
  try {
    return await fs.stat(targetPath);
  } catch {
    return undefined;
  }
}
