import { afterEach, describe, expect, it } from "vitest";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { configureZone } from "./configureZone";

const tempDirs: string[] = [];
const schema = {
  type: "object",
  properties: {
    operator: {
      type: "object",
      properties: { email: { type: "string" } },
      required: ["email"],
    },
    lawpacks: { type: "array", items: { type: "string" } },
    api: {
      type: "object",
      properties: { port: { type: "integer" } },
      required: ["port"],
    },
  },
  required: ["operator", "lawpacks", "api"],
};

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fs.rm(dir, { recursive: true, force: true })));
});

describe("configureZone", () => {
  it("sets an existing path and persists the file", async () => {
    const zonePath = await createZoneDir();
    const result = await configureZone({
      zone_path: zonePath,
      config_changes: { "operator.email": "new@example.com" },
    });

    expect(result.paths_updated).toEqual(["operator.email"]);
    expect(await fs.readFile(path.join(zonePath, "zone.yaml"), "utf8")).toContain("email: new@example.com");
  });

  it("sets an array index", async () => {
    const zonePath = await createZoneDir();
    const result = await configureZone({
      zone_path: zonePath,
      config_changes: { "lawpacks.0": "updated-lawpack" },
    });

    expect(result.paths_updated).toEqual(["lawpacks.0"]);
    expect(await fs.readFile(path.join(zonePath, "zone.yaml"), "utf8")).toContain("- updated-lawpack");
  });

  it("fails when an intermediate path is missing", async () => {
    const zonePath = await createZoneDir();
    const result = await configureZone({
      zone_path: zonePath,
      config_changes: { "nonexistent.deep.path": "value" },
    });

    expect(result.paths_updated).toEqual([]);
    expect(result.paths_failed).toContainEqual(
      expect.objectContaining({
        path: "nonexistent.deep.path",
        reason: expect.stringContaining("intermediate path missing: nonexistent"),
      }),
    );
  });

  it("reverts all changes when any path fails", async () => {
    const zonePath = await createZoneDir();
    const before = await fs.readFile(path.join(zonePath, "zone.yaml"), "utf8");
    const result = await configureZone({
      zone_path: zonePath,
      config_changes: { "operator.email": "atomic@example.com", "missing.value": "boom" },
    });

    expect(result.paths_updated).toEqual([]);
    expect(result.paths_failed).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ path: "missing.value" }),
        expect.objectContaining({
          path: "operator.email",
          reason: "reverted because another path failed",
        }),
      ]),
    );
    expect(await fs.readFile(path.join(zonePath, "zone.yaml"), "utf8")).toBe(before);
  });

  it("returns a diff without writing on dry run", async () => {
    const zonePath = await createZoneDir();
    const before = await fs.readFile(path.join(zonePath, "zone.yaml"), "utf8");
    const result = await configureZone({
      zone_path: zonePath,
      config_changes: { "api.port": 9000 },
      dry_run: true,
    });

    expect(result.diff?.before).toBe(before);
    expect(result.diff?.after).toContain("port: 9000");
    expect(await fs.readFile(path.join(zonePath, "zone.yaml"), "utf8")).toBe(before);
  });

  it("throws for a missing target file", async () => {
    const zonePath = await createZoneDir();
    await expect(
      configureZone({
        zone_path: zonePath,
        target_file: "operations/missing.yaml",
        config_changes: { "steps.0.id": "new-id" },
      }),
    ).rejects.toThrow("Target file not found");
  });
});

async function createZoneDir() {
  const zonePath = await fs.mkdtemp(path.join(os.tmpdir(), "configure-zone-"));
  tempDirs.push(zonePath);
  await fs.mkdir(path.join(zonePath, "schemas"), { recursive: true });
  await fs.writeFile(path.join(zonePath, "schemas", "zone.schema.json"), JSON.stringify(schema, null, 2));
  await fs.writeFile(
    path.join(zonePath, "zone.yaml"),
    ["operator:", "  email: old@example.com", "lawpacks:", "  - legacy", "api:", "  port: 8080", ""].join("\n"),
  );
  return zonePath;
}
