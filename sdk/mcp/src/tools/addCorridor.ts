import { promises as fs } from "node:fs";
import path from "node:path";
import { parse, stringify } from "yaml";

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

const ZONE_ID_RE = /^[a-z0-9]+(?:-[a-z0-9]+)*(?:\.[a-z0-9]+(?:-[a-z0-9]+)*)+$/;
const COMPLIANCE_DOMAIN_SET = new Set<string>(COMPLIANCE_DOMAINS);

export type RecognitionBasis = (typeof RECOGNITION_BASES)[number];
export type FrictionProfile = (typeof FRICTION_PROFILES)[number];
export type CarryingDomain = (typeof COMPLIANCE_DOMAINS)[number];

export interface AddCorridorParams {
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

export interface AddCorridorResult {
  corridor_path: string;
  yaml_written: boolean;
  validation: {
    ok: true;
    carrying_domains: CarryingDomain[];
    defaults_applied: {
      expiry_days: boolean;
      cryptographic_epoch: boolean;
    };
  };
  next_step: string;
}

export async function addCorridor(params: AddCorridorParams): Promise<AddCorridorResult> {
  const zonePath = path.resolve(params.zone_path);
  const zoneStat = await safeStat(zonePath);
  if (!zoneStat?.isDirectory()) {
    throw new Error(`zone_path is not a directory: ${zonePath}`);
  }

  const zoneYamlPath = path.join(zonePath, "zone.yaml");
  const zoneYamlStat = await safeStat(zoneYamlPath);
  if (!zoneYamlStat?.isFile()) {
    throw new Error(`zone.yaml not found in zone_path: ${zonePath}`);
  }

  if (!ZONE_ID_RE.test(params.partner_zone_id)) {
    throw new Error(
      `partner_zone_id must match label regex ${ZONE_ID_RE.source}: ${params.partner_zone_id}`,
    );
  }

  const invalidDomains = params.carrying_domains.filter((domain) => !COMPLIANCE_DOMAIN_SET.has(domain));
  if (invalidDomains.length > 0) {
    throw new Error(
      `carrying_domains must be a subset of the 23 compliance domains: ${invalidDomains.join(", ")}`,
    );
  }

  if (params.friction_profile === "custom" && !params.custom_friction) {
    throw new Error("custom_friction is required when friction_profile is custom");
  }

  const expiryDays = params.expiry_days ?? 365;
  if (!Number.isInteger(expiryDays) || expiryDays <= 0) {
    throw new Error(`expiry_days must be a positive integer: ${expiryDays}`);
  }

  const cryptographicEpoch = params.cryptographic_epoch ?? "current";
  if (cryptographicEpoch.trim().length === 0) {
    throw new Error("cryptographic_epoch must be a non-empty string");
  }

  const corridorsDir = path.join(zonePath, "corridors");
  await fs.mkdir(corridorsDir, { recursive: true });

  const corridorPath = path.join(corridorsDir, `${params.partner_zone_id}.yaml`);
  const corridorStat = await safeStat(corridorPath);
  if (corridorStat) {
    throw new Error(`corridor file already exists: ${corridorPath}`);
  }

  const zoneManifest = parse(await fs.readFile(zoneYamlPath, "utf8"));
  const localZoneId = getZoneId(zoneManifest) ?? path.basename(zonePath);
  const corridorId = `${localZoneId}--${params.partner_zone_id}`;

  const corridorDocument = {
    corridor_id: corridorId,
    peer: {
      zone_id: params.partner_zone_id,
      jurisdiction_id: params.partner_jurisdiction_id,
      endpoint_url: params.partner_endpoint,
    },
    recognition_basis: params.recognition_basis,
    friction_profile:
      params.friction_profile === "custom"
        ? {
            mode: "custom",
            custom: params.custom_friction!,
          }
        : {
            mode: params.friction_profile,
          },
    carrying_domains: params.carrying_domains,
    expiry_days: expiryDays,
    cryptographic_epoch: cryptographicEpoch,
  };

  await writeAtomically(corridorPath, stringify(corridorDocument, null, { indent: 2 }));

  return {
    corridor_path: corridorPath,
    yaml_written: true,
    validation: {
      ok: true,
      carrying_domains: params.carrying_domains,
      defaults_applied: {
        expiry_days: params.expiry_days === undefined,
        cryptographic_epoch: params.cryptographic_epoch === undefined,
      },
    },
    next_step:
      `Call mass.request_corridor_recognition for ${corridorId} ` +
      `to begin bilateral recognition with ${params.partner_zone_id}.`,
  };
}

function getZoneId(manifest: unknown): string | undefined {
  if (!isPlainObject(manifest)) {
    throw new Error("zone.yaml must parse to an object");
  }

  const zoneId = manifest["zone_id"];
  return typeof zoneId === "string" && zoneId.length > 0 ? zoneId : undefined;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

async function safeStat(targetPath: string) {
  try {
    return await fs.stat(targetPath);
  } catch {
    return undefined;
  }
}

async function writeAtomically(targetPath: string, contents: string) {
  const tempPath = `${targetPath}.tmp-${process.pid}-${Date.now()}`;
  await fs.writeFile(tempPath, contents, "utf8");
  await fs.rename(tempPath, targetPath);
}
