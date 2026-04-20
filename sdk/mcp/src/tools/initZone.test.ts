import { afterEach, describe, expect, it } from "vitest";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { parse } from "yaml";
import { initZone } from "./initZone";

const tempDirs: string[] = [];

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fs.rm(dir, { recursive: true, force: true })));
});

describe("initZone", () => {
  it.each(["BAD", "a", "with spaces", `a${"b".repeat(31)}c`])(
    "rejects invalid zone_id: %s",
    async (zoneId) => {
      await expect(
        initZone({
          zone_id: zoneId,
          jurisdiction_id: "sc",
          template_dir: await createTemplateDir(),
          deploy_dir: await createDeployDir(),
          zone_name: "Seychelles Zone",
          operator_email: "ops@example.com",
          api_port: 8080,
          region: "indian-ocean",
        }),
      ).rejects.toThrow("zone_id must match");
    },
  );

  it("refuses to overwrite an existing zone directory", async () => {
    const templateDir = await createTemplateDir();
    const deployDir = await createDeployDir();
    await fs.mkdir(path.join(deployDir, "test-zone"), { recursive: true });

    await expect(
      initZone({
        zone_id: "test-zone",
        jurisdiction_id: "sc",
        template_dir: templateDir,
        deploy_dir: deployDir,
        zone_name: "Seychelles Zone",
        operator_email: "ops@example.com",
        api_port: 8080,
        region: "indian-ocean",
      }),
    ).rejects.toThrow("zone already exists");
  });

  it("copies the template, counts files, and rewrites zone.yaml", async () => {
    const result = await initZone({
      zone_id: "test-zone",
      jurisdiction_id: "sc",
      template_dir: await createTemplateDir(),
      deploy_dir: await createDeployDir(),
      zone_name: "Seychelles Zone",
      operator_email: "ops@example.com",
      api_port: 8080,
      region: "indian-ocean",
    });

    expect(result.files_copied).toBe(3);
    expect(result.parameters_applied).toEqual({
      zone_id: "test-zone",
      jurisdiction_id: "sc",
      zone_name: "Seychelles Zone",
      operator_email: "ops@example.com",
      api_port: 8080,
      region: "indian-ocean",
    });

    const zoneYaml = parse(await fs.readFile(path.join(result.zone_path, "zone.yaml"), "utf8")) as Record<string, any>;
    expect(zoneYaml["zone_id"]).toBe("test-zone");
    expect(zoneYaml["jurisdiction_id"]).toBe("sc");
    expect(zoneYaml["zone_name"]).toBe("Seychelles Zone");
    expect(zoneYaml["api"]["port"]).toBe(8080);
    expect(zoneYaml["operator"]["email"]).toBe("ops@example.com");
    expect(zoneYaml["region"]).toBe("indian-ocean");
  });

  it("excludes git, claude, node_modules, and dist artifacts from the copy", async () => {
    const deployDir = await createDeployDir();
    const result = await initZone({
      zone_id: "test-zone",
      jurisdiction_id: "sc",
      template_dir: await createTemplateDir(),
      deploy_dir: deployDir,
      zone_name: "Seychelles Zone",
      operator_email: "ops@example.com",
      api_port: 8080,
      region: "indian-ocean",
    });

    for (const excluded of [".git", ".claude", "node_modules", "dist"]) {
      await expect(fs.stat(path.join(result.zone_path, excluded))).rejects.toThrow();
    }
  });
});

async function createTemplateDir() {
  const templateDir = await fs.mkdtemp(path.join(os.tmpdir(), "init-zone-template-"));
  tempDirs.push(templateDir);

  await fs.mkdir(path.join(templateDir, "operations", "entity"), { recursive: true });
  await fs.mkdir(path.join(templateDir, ".git"), { recursive: true });
  await fs.mkdir(path.join(templateDir, ".claude"), { recursive: true });
  await fs.mkdir(path.join(templateDir, "node_modules", "left-pad"), { recursive: true });
  await fs.mkdir(path.join(templateDir, "dist"), { recursive: true });

  await fs.writeFile(
    path.join(templateDir, "zone.yaml"),
    [
      "zone_id: placeholder-zone",
      "jurisdiction_id: placeholder",
      "zone_name: Placeholder Zone",
      "api:",
      "  port: 7000",
      "operator:",
      "  email: old@example.com",
      "region: placeholder-region",
      "",
    ].join("\n"),
  );
  await fs.writeFile(path.join(templateDir, "README.md"), "# Template\n");
  await fs.writeFile(path.join(templateDir, "operations", "entity", "incorporate.yaml"), "operation: test\n");
  await fs.writeFile(path.join(templateDir, ".git", "config"), "[core]\n");
  await fs.writeFile(path.join(templateDir, ".claude", "settings.json"), "{}\n");
  await fs.writeFile(path.join(templateDir, "node_modules", "left-pad", "index.js"), "module.exports = {};\n");
  await fs.writeFile(path.join(templateDir, "dist", "bundle.js"), "console.log('build');\n");

  return templateDir;
}

async function createDeployDir() {
  const deployDir = await fs.mkdtemp(path.join(os.tmpdir(), "init-zone-deploy-"));
  tempDirs.push(deployDir);
  return deployDir;
}
