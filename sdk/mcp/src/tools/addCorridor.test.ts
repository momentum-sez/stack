import { afterEach, describe, expect, it } from "vitest";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { parse } from "yaml";
import { addCorridor } from "./addCorridor";

const tempDirs: string[] = [];

afterEach(async () => {
  await Promise.all(tempDirs.splice(0).map((dir) => fs.rm(dir, { recursive: true, force: true })));
});

describe("addCorridor", () => {
  it("writes a corridor YAML document with defaults", async () => {
    const zonePath = await createZoneDir();
    const result = await addCorridor({
      zone_path: zonePath,
      partner_zone_id: "org.momentum.mez.zone.partner",
      partner_jurisdiction_id: "partner-jurisdiction",
      partner_endpoint: "https://partner.example/v1/corridor",
      recognition_basis: "passport-and-msa",
      friction_profile: "medium",
      carrying_domains: ["aml", "kyc", "sharia"],
    });

    const written = parse(await fs.readFile(result.corridor_path, "utf8")) as Record<string, unknown>;
    expect(result.yaml_written).toBe(true);
    expect(result.validation).toEqual(
      expect.objectContaining({
        ok: true,
        defaults_applied: { expiry_days: true, cryptographic_epoch: true },
      }),
    );
    expect(written["expiry_days"]).toBe(365);
    expect(written["cryptographic_epoch"]).toBe("current");
    expect(result.next_step).toContain("mass.request_corridor_recognition");
  });

  it("throws when custom friction is missing", async () => {
    const zonePath = await createZoneDir();
    await expect(
      addCorridor({
        zone_path: zonePath,
        partner_zone_id: "org.momentum.mez.zone.partner",
        partner_jurisdiction_id: "partner-jurisdiction",
        partner_endpoint: "https://partner.example/v1/corridor",
        recognition_basis: "full-mutual",
        friction_profile: "custom",
        carrying_domains: ["aml"],
      }),
    ).rejects.toThrow("custom_friction is required");
  });

  it("throws when the corridor file already exists", async () => {
    const zonePath = await createZoneDir();
    await fs.mkdir(path.join(zonePath, "corridors"), { recursive: true });
    await fs.writeFile(path.join(zonePath, "corridors", "org.momentum.mez.zone.partner.yaml"), "existing: true\n");

    await expect(
      addCorridor({
        zone_path: zonePath,
        partner_zone_id: "org.momentum.mez.zone.partner",
        partner_jurisdiction_id: "partner-jurisdiction",
        partner_endpoint: "https://partner.example/v1/corridor",
        recognition_basis: "passport-only",
        friction_profile: "low",
        carrying_domains: ["aml"],
      }),
    ).rejects.toThrow("corridor file already exists");
  });

  it("throws when carrying_domains includes an unknown domain", async () => {
    const zonePath = await createZoneDir();
    await expect(
      addCorridor({
        zone_path: zonePath,
        partner_zone_id: "org.momentum.mez.zone.partner",
        partner_jurisdiction_id: "partner-jurisdiction",
        partner_endpoint: "https://partner.example/v1/corridor",
        recognition_basis: "passport-only",
        friction_profile: "low",
        carrying_domains: ["aml", "not-a-domain" as never],
      }),
    ).rejects.toThrow("carrying_domains must be a subset");
  });

  it("throws when zone_path is invalid", async () => {
    await expect(
      addCorridor({
        zone_path: path.join(os.tmpdir(), "missing-zone-path"),
        partner_zone_id: "org.momentum.mez.zone.partner",
        partner_jurisdiction_id: "partner-jurisdiction",
        partner_endpoint: "https://partner.example/v1/corridor",
        recognition_basis: "passport-only",
        friction_profile: "low",
        carrying_domains: ["aml"],
      }),
    ).rejects.toThrow("zone_path is not a directory");
  });

  it("throws when zone.yaml is missing", async () => {
    const zonePath = await fs.mkdtemp(path.join(os.tmpdir(), "add-corridor-no-manifest-"));
    tempDirs.push(zonePath);

    await expect(
      addCorridor({
        zone_path: zonePath,
        partner_zone_id: "org.momentum.mez.zone.partner",
        partner_jurisdiction_id: "partner-jurisdiction",
        partner_endpoint: "https://partner.example/v1/corridor",
        recognition_basis: "passport-only",
        friction_profile: "low",
        carrying_domains: ["aml"],
      }),
    ).rejects.toThrow("zone.yaml not found");
  });
});

async function createZoneDir() {
  const zonePath = await fs.mkdtemp(path.join(os.tmpdir(), "add-corridor-"));
  tempDirs.push(zonePath);
  await fs.writeFile(
    path.join(zonePath, "zone.yaml"),
    ["zone_id: org.momentum.mez.zone.origin", "jurisdiction_id: origin", "zone_name: Origin Zone", ""].join("\n"),
  );
  return zonePath;
}
