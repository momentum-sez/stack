import { afterEach, describe, expect, it, vi } from "vitest";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";

const execFileMock = vi.hoisted(() => vi.fn());

vi.mock("node:child_process", () => ({
  execFile: execFileMock,
}));

import { deployZone, verifyDeploy } from "./deployZone";

const tempDirs: string[] = [];
type ExecFileCallback = (error: Error | null, stdout: string, stderr: string) => void;

afterEach(async () => {
  delete process.env["MASS_MCP_DEPLOY_AUTHORIZED"];
  execFileMock.mockReset();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  await Promise.all(tempDirs.splice(0).map((dir) => fs.rm(dir, { recursive: true, force: true })));
});

describe("deployZone", () => {
  it("returns rendered compose output in dry_run mode", async () => {
    const zonePath = await createZoneDir();
    execFileMock.mockImplementation(
      (
        _file: string,
        _args: string[],
        _options: Record<string, unknown>,
        callback: ExecFileCallback,
      ) => {
      callback(null, "services:\n  mez-api:\n    image: test\n", "");
      return {} as never;
      },
    );

    const result = await deployZone({ zone_path: zonePath, mode: "dry_run" });

    expect(execFileMock).toHaveBeenCalledOnce();
    expect(result.rendered_compose).toContain("mez-api");
    expect(result.command).toContain("config");
    expect(result.exit_code).toBe(0);
    expect(result.next_step).toBe("Run `verify_deploy({zone_path})`");
  });

  it("builds an instructions_only command without touching docker", async () => {
    const zonePath = await createZoneDir({ withEnvFile: true });

    const result = await deployZone({
      zone_path: zonePath,
      mode: "instructions_only",
      env_file: ".env",
      detach: false,
      build: true,
    });

    expect(execFileMock).not.toHaveBeenCalled();
    expect(result.command).toContain("--env-file");
    expect(result.command).toContain("--build");
    expect(result.command).not.toContain(" -d");
    expect(result.exit_code).toBeNull();
  });

  it("refuses execute mode unless MASS_MCP_DEPLOY_AUTHORIZED=true", async () => {
    const zonePath = await createZoneDir();

    await expect(
      deployZone({
        zone_path: zonePath,
        mode: "execute",
      }),
    ).rejects.toThrow("MASS_MCP_DEPLOY_AUTHORIZED=true");

    expect(execFileMock).not.toHaveBeenCalled();
  });
});

describe("verifyDeploy", () => {
  it("reports unreachable when the zone endpoint returns a non-200 response", async () => {
    const zonePath = await createZoneDir();
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 503,
      statusText: "Service Unavailable",
    });
    vi.stubGlobal("fetch", fetchMock);

    const result = await verifyDeploy({ zone_path: zonePath, timeout_ms: 25 });

    expect(fetchMock).toHaveBeenCalledWith(
      "http://localhost:8080/v1/zone",
      expect.objectContaining({ method: "GET" }),
    );
    expect(result.reachable).toBe(false);
    expect(result.error).toContain("503");
  });

  it("parses zone.yaml api.url before verifying the deploy", async () => {
    const zonePath = await createZoneDir({ apiUrl: "http://127.0.0.1:18080" });
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: vi.fn().mockResolvedValue({
        zone_status: {
          zone_id: "org.momentum.mez.zone.test",
          status: "healthy",
        },
      }),
    });
    vi.stubGlobal("fetch", fetchMock);

    const result = await verifyDeploy({ zone_path: zonePath });

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:18080/v1/zone",
      expect.objectContaining({ method: "GET" }),
    );
    expect(result).toMatchObject({
      reachable: true,
      zone_status: {
        zone_id: "org.momentum.mez.zone.test",
        status: "healthy",
      },
    });
  });
});

async function createZoneDir(options: { apiUrl?: string; withEnvFile?: boolean } = {}) {
  const zonePath = await fs.mkdtemp(path.join(os.tmpdir(), "deploy-zone-"));
  tempDirs.push(zonePath);

  await fs.mkdir(path.join(zonePath, "deploy"), { recursive: true });
  await fs.writeFile(
    path.join(zonePath, "deploy", "docker-compose.yaml"),
    ["services:", "  mez-api:", "    image: busybox", ""].join("\n"),
  );

  const zoneYaml = options.apiUrl
    ? ["zone_id: org.momentum.mez.zone.test", "api:", `  url: ${options.apiUrl}`, ""].join("\n")
    : ["zone_id: org.momentum.mez.zone.test", ""].join("\n");
  await fs.writeFile(path.join(zonePath, "zone.yaml"), zoneYaml);

  if (options.withEnvFile) {
    await fs.writeFile(path.join(zonePath, ".env"), "POSTGRES_PASSWORD=test\n");
  }

  return zonePath;
}
