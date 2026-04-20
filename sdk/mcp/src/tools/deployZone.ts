import { execFile } from "node:child_process";
import { promises as fs } from "node:fs";
import path from "node:path";
import { parse } from "yaml";

const DEFAULT_COMPOSE_FILE = "deploy/docker-compose.yaml";
const DEFAULT_MEZ_API_URL = "http://localhost:8080";
const DOCKER_COMPOSE_BIN = "docker-compose";
const EXEC_TIMEOUT_MS = 300_000;
const VERIFY_NEXT_STEP = "Run `verify_deploy({zone_path})`";

export type DeployMode = "dry_run" | "execute" | "instructions_only";

export interface DeployZoneParams {
  zone_path: string;
  mode: DeployMode;
  compose_file?: string;
  env_file?: string;
  detach?: boolean;
  build?: boolean;
}

export interface DeployZoneResult {
  mode: DeployMode;
  compose_file_resolved: string;
  rendered_compose?: string;
  command: string;
  execution_log?: string;
  exit_code: number | null;
  next_step: string;
}

export interface VerifyDeployParams {
  zone_path: string;
  mez_api_url?: string;
  timeout_ms?: number;
}

export interface VerifyDeployResult {
  reachable: boolean;
  zone_status?: unknown;
  error?: string;
  next_step: string;
}

type ExecResult = {
  stdout: string;
  stderr: string;
  exitCode: number;
};

type ExecFileError = Error & {
  code?: number | string;
};

export async function deployZone(params: DeployZoneParams): Promise<DeployZoneResult> {
  const zonePath = path.resolve(params.zone_path);
  const detach = params.detach ?? true;
  const build = params.build ?? false;

  await assertDirectory(zonePath, "zone_path");

  const composeFileResolved = await resolveExistingFile(
    zonePath,
    params.compose_file ?? DEFAULT_COMPOSE_FILE,
    "compose_file",
  );
  const envFileResolved = params.env_file
    ? await resolveExistingFile(zonePath, params.env_file, "env_file")
    : undefined;

  const { args, command } = buildComposeInvocation({
    composeFileResolved,
    envFileResolved,
    mode: params.mode,
    detach,
    build,
  });

  if (params.mode === "instructions_only") {
    return {
      mode: params.mode,
      compose_file_resolved: composeFileResolved,
      command,
      exit_code: null,
      next_step: VERIFY_NEXT_STEP,
    };
  }

  if (params.mode === "dry_run") {
    const result = await runDockerCompose(args, zonePath);
    if (result.exitCode !== 0) {
      throw new Error(`docker-compose config failed with exit code ${result.exitCode}`);
    }

    return {
      mode: params.mode,
      compose_file_resolved: composeFileResolved,
      rendered_compose: result.stdout.trimEnd(),
      command,
      exit_code: result.exitCode,
      next_step: VERIFY_NEXT_STEP,
    };
  }

  if (process.env["MASS_MCP_DEPLOY_AUTHORIZED"] !== "true") {
    throw new Error(
      "Execution refused: MASS_MCP_DEPLOY_AUTHORIZED=true is required for mode=execute",
    );
  }

  const result = await runDockerCompose(args, zonePath);

  return {
    mode: params.mode,
    compose_file_resolved: composeFileResolved,
    command,
    execution_log: formatExecutionLog(result.stdout, result.stderr),
    exit_code: result.exitCode,
    next_step: VERIFY_NEXT_STEP,
  };
}

export async function verifyDeploy(
  params: VerifyDeployParams,
): Promise<VerifyDeployResult> {
  const zonePath = path.resolve(params.zone_path);
  const timeoutMs = params.timeout_ms ?? 10_000;

  await assertDirectory(zonePath, "zone_path");

  const mezApiUrl = normalizeBaseUrl(
    params.mez_api_url ?? (await readApiUrlFromZoneYaml(zonePath)) ?? DEFAULT_MEZ_API_URL,
  );
  const requestUrl = `${mezApiUrl}/v1/zone`;

  try {
    const response = await fetch(requestUrl, {
      method: "GET",
      signal: AbortSignal.timeout(timeoutMs),
    });

    if (!response.ok) {
      return {
        reachable: false,
        error: `GET ${requestUrl} returned ${response.status} ${response.statusText}`.trim(),
        next_step: "Inspect the deploy logs, then rerun verify_deploy once the zone is healthy.",
      };
    }

    const payload = await response.json();

    return {
      reachable: true,
      zone_status: extractZoneStatus(payload),
      next_step: "Zone API reachable. Continue with runtime-side validation.",
    };
  } catch (error) {
    return {
      reachable: false,
      error: `GET ${requestUrl} failed: ${formatError(error)}`,
      next_step: "Inspect the deploy logs, then rerun verify_deploy once the zone is healthy.",
    };
  }
}

function buildComposeInvocation(input: {
  composeFileResolved: string;
  envFileResolved?: string;
  mode: DeployMode;
  detach: boolean;
  build: boolean;
}) {
  const args = ["-f", input.composeFileResolved];

  if (input.envFileResolved) {
    args.push("--env-file", input.envFileResolved);
  }

  if (input.mode === "dry_run") {
    args.push("config");
    return { args, command: toCommandString(DOCKER_COMPOSE_BIN, args) };
  }

  args.push("up");

  if (input.build) {
    args.push("--build");
  }

  if (input.detach) {
    args.push("-d");
  }

  return { args, command: toCommandString(DOCKER_COMPOSE_BIN, args) };
}

async function runDockerCompose(args: string[], cwd: string): Promise<ExecResult> {
  return await new Promise((resolve, reject) => {
    execFile(
      DOCKER_COMPOSE_BIN,
      args,
      {
        cwd,
        timeout: EXEC_TIMEOUT_MS,
        maxBuffer: 10 * 1024 * 1024,
      },
      (error, stdout, stderr) => {
        if (!error) {
          resolve({ stdout, stderr, exitCode: 0 });
          return;
        }

        const execError = error as ExecFileError;
        if (typeof execError.code === "number") {
          resolve({ stdout, stderr, exitCode: execError.code });
          return;
        }

        reject(new Error(`Failed to execute docker-compose: ${formatError(execError)}`));
      },
    );
  });
}

async function readApiUrlFromZoneYaml(zonePath: string): Promise<string | undefined> {
  const zoneYamlPath = path.join(zonePath, "zone.yaml");
  const zoneYamlStat = await safeStat(zoneYamlPath);
  if (!zoneYamlStat?.isFile()) {
    return undefined;
  }

  try {
    const raw = await fs.readFile(zoneYamlPath, "utf8");
    const parsed = parse(raw);
    if (!isRecord(parsed) || !isRecord(parsed["api"])) {
      return undefined;
    }

    const candidate = parsed["api"]["url"];
    return typeof candidate === "string" && candidate.trim() !== ""
      ? candidate.trim()
      : undefined;
  } catch {
    return undefined;
  }
}

async function assertDirectory(targetPath: string, label: string) {
  const stat = await safeStat(targetPath);
  if (!stat?.isDirectory()) {
    throw new Error(`${label} is not a directory: ${targetPath}`);
  }
}

async function resolveExistingFile(basePath: string, candidate: string, label: string) {
  const resolved = path.resolve(basePath, candidate);
  const stat = await safeStat(resolved);
  if (!stat?.isFile()) {
    throw new Error(`${label} not found: ${resolved}`);
  }

  return resolved;
}

async function safeStat(targetPath: string) {
  try {
    return await fs.stat(targetPath);
  } catch {
    return null;
  }
}

function extractZoneStatus(payload: unknown) {
  if (isRecord(payload) && "zone_status" in payload) {
    return payload["zone_status"];
  }

  return payload;
}

function formatExecutionLog(stdout: string, stderr: string) {
  const output = [
    stdout.trim() ? `stdout:\n${stdout.trim()}` : "",
    stderr.trim() ? `stderr:\n${stderr.trim()}` : "",
  ].filter(Boolean);

  return output.join("\n\n") || "(no output)";
}

function formatError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function toCommandString(command: string, args: string[]) {
  return [command, ...args.map(shellQuote)].join(" ");
}

function shellQuote(value: string) {
  return JSON.stringify(value);
}

function normalizeBaseUrl(value: string) {
  return value.replace(/\/+$/, "");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
