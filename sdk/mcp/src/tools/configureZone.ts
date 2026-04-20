import { promises as fs } from "node:fs";
import path from "node:path";
import { parse, stringify } from "yaml";

export type ConfigPath = string;

export interface ConfigureZoneParams {
  zone_path: string;
  config_changes: Record<ConfigPath, unknown>;
  target_file?: "zone.yaml" | string;
  validate_schema?: boolean;
  dry_run?: boolean;
}

export interface ConfigureZoneResult {
  zone_path: string;
  target_file: string;
  paths_updated: string[];
  paths_failed: { path: string; reason: string }[];
  diff?: { before: string; after: string };
  schema_validation?: { passed: boolean; errors: string[] };
}

type JsonSchema = Record<string, any>;
type SchemaNode = { fileName: string; schema: JsonSchema };

const INDEX_RE = /^\d+$/;

export async function configureZone(
  params: ConfigureZoneParams,
): Promise<ConfigureZoneResult> {
  const zonePath = path.resolve(params.zone_path);
  const targetFile = params.target_file ?? "zone.yaml";
  const validateSchema = params.validate_schema ?? true;
  const dryRun = params.dry_run ?? false;

  const zoneStat = await safeStat(zonePath);
  if (!zoneStat?.isDirectory()) {
    throw new Error(`zone_path is not a directory: ${zonePath}`);
  }

  const targetPath = path.resolve(zonePath, targetFile);
  const relativeTarget = path.relative(zonePath, targetPath);
  if (relativeTarget.startsWith("..") || path.isAbsolute(relativeTarget)) {
    throw new Error(`target_file resolves outside zone_path: ${targetFile}`);
  }

  const targetStat = await safeStat(targetPath);
  if (!targetStat?.isFile()) {
    throw new Error(`Target file not found: ${targetFile}`);
  }

  const before = await fs.readFile(targetPath, "utf8");
  const parsed = parse(before);
  if (!isContainer(parsed)) {
    throw new Error(`Target YAML must parse to an object or array: ${targetFile}`);
  }

  const workingCopy = structuredClone(parsed);
  const schemaContext = await loadSchemaContext(zonePath, targetFile);
  const updatedPaths: string[] = [];
  const nonBlockingFailures: ConfigureZoneResult["paths_failed"] = [];
  const blockingFailures: ConfigureZoneResult["paths_failed"] = [];

  for (const [configPath, value] of Object.entries(params.config_changes)) {
    const applied = setValueAtPath(workingCopy, configPath, value);
    if (!applied.ok) {
      blockingFailures.push({ path: configPath, reason: applied.reason });
      continue;
    }

    updatedPaths.push(configPath);

    const leafNodes = schemaContext.primarySchema
      ? resolvePathSchemaNodes(
          [{ fileName: schemaContext.primarySchemaName!, schema: schemaContext.primarySchema }],
          configPath.split("."),
          schemaContext.schemas,
        )
      : [];

    if (leafNodes.length === 0) {
      continue;
    }

    const leafErrors = validateAgainstNodes(value, leafNodes, schemaContext.schemas, configPath);
    if (leafErrors.length === 0) {
      continue;
    }

    const failure = { path: configPath, reason: leafErrors[0] };
    if (validateSchema) {
      blockingFailures.push(failure);
      continue;
    }

    nonBlockingFailures.push(failure);
  }

  let schemaValidation: ConfigureZoneResult["schema_validation"];

  if (blockingFailures.length === 0 && validateSchema) {
    if (schemaContext.primarySchema) {
      const schemaErrors = validateAgainstNodes(
        workingCopy,
        [{ fileName: schemaContext.primarySchemaName!, schema: schemaContext.primarySchema }],
        schemaContext.schemas,
        "$",
        schemaContext.allowUnknownTopLevel,
      );
      schemaValidation = { passed: schemaErrors.length === 0, errors: schemaErrors };
      if (schemaErrors.length > 0) {
        blockingFailures.push(
          ...updatedPaths
            .filter((configPath) => !nonBlockingFailures.some((failure) => failure.path === configPath))
            .map((configPath) => ({
              path: configPath,
              reason: `schema validation failed: ${schemaErrors[0]}`,
            })),
        );
      }
    } else {
      schemaValidation = {
        passed: true,
        errors: [`schema not found for ${targetFile}; validation skipped`],
      };
    }
  }

  if (blockingFailures.length > 0) {
    const reverted = updatedPaths
      .filter((configPath) => !blockingFailures.some((failure) => failure.path === configPath))
      .map((configPath) => ({
        path: configPath,
        reason: "reverted because another path failed",
      }));

    return {
      zone_path: zonePath,
      target_file: targetFile,
      paths_updated: [],
      paths_failed: [...blockingFailures, ...reverted],
      schema_validation: schemaValidation,
    };
  }

  const after = stringify(workingCopy, null, { indent: 2 });
  const pathsUpdated = updatedPaths.filter(
    (configPath) => !nonBlockingFailures.some((failure) => failure.path === configPath),
  );

  if (dryRun) {
    return {
      zone_path: zonePath,
      target_file: targetFile,
      paths_updated: pathsUpdated,
      paths_failed: nonBlockingFailures,
      diff: { before, after },
      schema_validation: schemaValidation,
    };
  }

  await writeAtomically(targetPath, after);

  return {
    zone_path: zonePath,
    target_file: targetFile,
    paths_updated: pathsUpdated,
    paths_failed: nonBlockingFailures,
    schema_validation: schemaValidation,
  };
}

function setValueAtPath(root: unknown, configPath: string, value: unknown) {
  const segments = configPath.split(".").filter(Boolean);
  if (segments.length === 0) {
    return { ok: false as const, reason: `invalid config path: ${configPath}` };
  }

  let current: any = root;
  for (let index = 0; index < segments.length - 1; index += 1) {
    const segment = segments[index];
    if (!isContainer(current)) {
      const prefix = segments.slice(0, index).join(".") || segment;
      return { ok: false as const, reason: missingIntermediate(prefix) };
    }

    const nextValue = Array.isArray(current) && INDEX_RE.test(segment)
      ? current[Number(segment)]
      : current[segment];
    if (nextValue === undefined || nextValue === null) {
      return {
        ok: false as const,
        reason: missingIntermediate(segments.slice(0, index + 1).join(".")),
      };
    }

    current = nextValue;
  }

  const leaf = segments[segments.length - 1];
  if (!isContainer(current)) {
    return {
      ok: false as const,
      reason: missingIntermediate(segments.slice(0, -1).join(".") || leaf),
    };
  }

  if (Array.isArray(current) && INDEX_RE.test(leaf)) {
    current[Number(leaf)] = value;
    return { ok: true as const };
  }

  current[leaf] = value;
  return { ok: true as const };
}

async function loadSchemaContext(zonePath: string, targetFile: string) {
  const schemas = new Map<string, JsonSchema>();
  for (const fileName of ["zone.schema.json", "operation.schema.json"]) {
    const schemaPath = path.join(zonePath, "schemas", fileName);
    const stat = await safeStat(schemaPath);
    if (stat?.isFile()) {
      schemas.set(fileName, JSON.parse(await fs.readFile(schemaPath, "utf8")));
    }
  }

  const primarySchemaName = targetFile.startsWith("operations/")
    ? "operation.schema.json"
    : "zone.schema.json";

  return {
    schemas,
    primarySchemaName,
    primarySchema: schemas.get(primarySchemaName),
    allowUnknownTopLevel: primarySchemaName === "zone.schema.json",
  };
}

function resolvePathSchemaNodes(nodes: SchemaNode[], segments: string[], schemas: Map<string, JsonSchema>) {
  let currentNodes = nodes;
  for (const segment of segments) {
    const nextNodes: SchemaNode[] = [];
    for (const node of expandNode(nodeList(currentNodes), schemas)) {
      const schema = node.schema;
      if ((schema.type === "array" || schema.items) && INDEX_RE.test(segment) && schema.items) {
        nextNodes.push({ fileName: node.fileName, schema: schema.items });
        continue;
      }

      const propertySchema = schema.properties?.[segment];
      if (propertySchema) {
        nextNodes.push({ fileName: node.fileName, schema: propertySchema });
        continue;
      }

      if (schema.additionalProperties && schema.additionalProperties !== false) {
        nextNodes.push({ fileName: node.fileName, schema: schema.additionalProperties });
      }
    }

    if (nextNodes.length === 0) {
      return [];
    }

    currentNodes = nextNodes;
  }

  return currentNodes;
}

function validateAgainstNodes(
  value: unknown,
  nodes: SchemaNode[],
  schemas: Map<string, JsonSchema>,
  location: string,
  allowUnknownTopLevel = false,
): string[] {
  const expanded = expandNode(nodes, schemas);
  if (expanded.length === 0) {
    return [];
  }

  const errorSets = expanded.map((node) => validateNode(value, node, schemas, location, allowUnknownTopLevel));
  return errorSets.find((errors) => errors.length === 0) ?? errorSets[0] ?? [];
}

function validateNode(
  value: unknown,
  node: SchemaNode,
  schemas: Map<string, JsonSchema>,
  location: string,
  allowUnknownTopLevel: boolean,
): string[] {
  const schema = node.schema;
  const errors: string[] = [];

  if (schema.type === "object" || schema.properties || schema.required || schema.additionalProperties !== undefined) {
    if (!isPlainObject(value)) {
      return [`${location}: expected object`];
    }

    for (const key of schema.required ?? []) {
      if (!(key in value)) {
        errors.push(`${location}: missing required property ${key}`);
      }
    }

    for (const [key, childValue] of Object.entries(value)) {
      const childSchema = schema.properties?.[key] ?? (schema.additionalProperties && schema.additionalProperties !== false
        ? schema.additionalProperties
        : undefined);
      if (!childSchema) {
        if (!(allowUnknownTopLevel && location === "$")) {
          errors.push(`${location}.${key}: additional property not allowed`);
        }
        continue;
      }

      errors.push(
        ...validateAgainstNodes(
          childValue,
          [{ fileName: node.fileName, schema: childSchema }],
          schemas,
          `${location}.${key}`,
        ),
      );
    }
  }

  if ((schema.type === "array" || schema.items) && !Array.isArray(value)) {
    errors.push(`${location}: expected array`);
  } else if (Array.isArray(value) && schema.items) {
    if (typeof schema.minItems === "number" && value.length < schema.minItems) {
      errors.push(`${location}: expected at least ${schema.minItems} items`);
    }
    if (schema.uniqueItems) {
      const seen = new Set<string>();
      for (const item of value) {
        const key = JSON.stringify(item);
        if (seen.has(key)) {
          errors.push(`${location}: duplicate array item`);
          break;
        }
        seen.add(key);
      }
    }
    value.forEach((item, index) => {
      errors.push(
        ...validateAgainstNodes(
          item,
          [{ fileName: node.fileName, schema: schema.items }],
          schemas,
          `${location}.${index}`,
        ),
      );
    });
  }

  if (schema.type === "string" && typeof value !== "string") {
    errors.push(`${location}: expected string`);
  }
  if (schema.type === "integer" && !Number.isInteger(value)) {
    errors.push(`${location}: expected integer`);
  }
  if (schema.type === "number" && typeof value !== "number") {
    errors.push(`${location}: expected number`);
  }
  if (schema.type === "boolean" && typeof value !== "boolean") {
    errors.push(`${location}: expected boolean`);
  }
  if (schema.type === "null" && value !== null) {
    errors.push(`${location}: expected null`);
  }
  if (typeof value === "string") {
    if (typeof schema.minLength === "number" && value.length < schema.minLength) {
      errors.push(`${location}: minimum length is ${schema.minLength}`);
    }
    if (typeof schema.maxLength === "number" && value.length > schema.maxLength) {
      errors.push(`${location}: maximum length is ${schema.maxLength}`);
    }
    if (schema.pattern && !(new RegExp(schema.pattern).test(value))) {
      errors.push(`${location}: value does not match pattern ${schema.pattern}`);
    }
    if (schema.format === "date" && !/^\d{4}-\d{2}-\d{2}$/.test(value)) {
      errors.push(`${location}: expected date in YYYY-MM-DD format`);
    }
  }
  if (typeof value === "number" && typeof schema.minimum === "number" && value < schema.minimum) {
    errors.push(`${location}: minimum value is ${schema.minimum}`);
  }
  if (schema.enum && !schema.enum.includes(value)) {
    errors.push(`${location}: value must be one of ${schema.enum.join(", ")}`);
  }

  return errors;
}

function expandNode(nodes: SchemaNode[], schemas: Map<string, JsonSchema>, seen = new Set<string>()): SchemaNode[] {
  return nodes.flatMap((node) => {
    if (node.schema.$ref) {
      const resolved = resolveRef(node.fileName, node.schema.$ref, schemas);
      if (!resolved) {
        return [];
      }

      const key = `${resolved.fileName}:${node.schema.$ref}`;
      if (seen.has(key)) {
        return [];
      }

      return expandNode([resolved], schemas, new Set(seen).add(key));
    }

    if (Array.isArray(node.schema.oneOf) && node.schema.oneOf.length > 0) {
      return expandNode(
        node.schema.oneOf.map((branch: JsonSchema) => ({ fileName: node.fileName, schema: branch })),
        schemas,
        seen,
      );
    }

    return [node];
  });
}

function resolveRef(fileName: string, ref: string, schemas: Map<string, JsonSchema>): SchemaNode | undefined {
  const [targetFile, pointer = ""] = ref.includes("#") ? ref.split("#", 2) : [ref, ""];
  const resolvedFile = targetFile || fileName;
  const schemaRoot = schemas.get(resolvedFile);
  if (!schemaRoot) {
    return undefined;
  }

  let current: any = schemaRoot;
  for (const segment of pointer.replace(/^\//, "").split("/").filter(Boolean)) {
    current = current?.[segment.replace(/~1/g, "/").replace(/~0/g, "~")];
  }

  return current ? { fileName: resolvedFile, schema: current } : undefined;
}

function nodeList(nodes: SchemaNode[]) {
  return nodes;
}

function missingIntermediate(prefix: string) {
  return `intermediate path missing: ${prefix} (set ${prefix} first)`;
}

function isContainer(value: unknown): value is Record<string, unknown> | unknown[] {
  return isPlainObject(value) || Array.isArray(value);
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
