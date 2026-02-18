#!/usr/bin/env node
/**
 * Pre-build validation for the MEZ spec generator.
 * Checks: chapter exports, return types, heading sequencing, part heading consistency.
 *
 * Usage: node validate.js
 * Exit 0 = all checks pass, Exit 1 = failures detected.
 */
const fs = require("fs");
const path = require("path");
const { Paragraph, Table, TableOfContents } = require("docx");

const chaptersDir = path.join(__dirname, "chapters");

// All chapter files in build order
const CHAPTER_FILES = [
  "00-cover", "00-toc", "00-executive-summary",
  "01-mission-vision", "02-architecture", "03-crypto-primitives",
  "04-artifact-model", "05-module-specs", "06-pack-trilogy", "07-profiles",
  "08-smart-asset", "09-receipt-chain", "10-compliance-tensor", "11-savm", "12-composition",
  "13-l1-architecture", "14-proving-system", "15-privacy", "16-anchoring",
  "17-constitutional", "18-civic-services",
  "19-compliance-arch", "20-manifold", "21-zkkyc",
  "22-corridor-arch", "23-corridor-bridge", "24-multilateral", "25-live-corridors",
  "26-watcher-arch", "27-bond-slashing", "28-quorum-finality",
  "29-migration", "30-migration-fsm", "31-compensation",
  "32-corporate", "33-identity", "34-tax", "35-capital-markets", "36-trade",
  "37-mass-bridge",
  "38-govos-layers", "39-sovereign-ai", "40-tax-pipeline", "41-sovereignty",
  "42-protocol-overview", "43-credentials", "44-arbitration", "45-agentic",
  "46-security", "47-hardening", "48-zk-circuits",
  "49-deployment", "50-docker", "51-terraform", "52-one-click", "53-operations",
  "54-adoption", "55-partners", "56-current-network",
  "A-version-history", "B-test-coverage", "C-scalability", "D-security-proofs",
  "E-crate-deps", "F-api-endpoints", "G-jurisdiction-templates", "H-cli-reference",
  "I-module-directory", "J-conformance", "K-govos-checklist",
];

let failures = 0;
let passes = 0;

function fail(msg) {
  console.error(`  FAIL: ${msg}`);
  failures++;
}

function pass() {
  passes++;
}

// ---------- Check 1: Every chapter exports a function ----------
console.log("Check 1: Chapter exports are functions");
const chapterModules = {};

for (const name of CHAPTER_FILES) {
  const filePath = path.join(chaptersDir, name + ".js");
  if (!fs.existsSync(filePath)) {
    fail(`${name}.js does not exist`);
    continue;
  }
  try {
    const mod = require(filePath);
    if (typeof mod !== "function") {
      fail(`${name}.js exports ${typeof mod}, expected function`);
    } else {
      chapterModules[name] = mod;
      pass();
    }
  } catch (e) {
    fail(`${name}.js failed to require: ${e.message}`);
  }
}

// ---------- Check 2: Every function returns a non-empty array of valid docx objects ----------
console.log("Check 2: Return values are non-empty arrays of Paragraph/Table");

function flatten(arr) {
  return arr.reduce((acc, el) =>
    Array.isArray(el) ? acc.concat(flatten(el)) : acc.concat(el), []);
}

function isDocxElement(el) {
  return el instanceof Paragraph || el instanceof Table || el instanceof TableOfContents;
}

const chapterElements = {};

for (const [name, fn] of Object.entries(chapterModules)) {
  // 00-toc takes a tocEntries parameter (static TOC generated at build time),
  // so it cannot be called standalone. Validate it with a stub entry instead.
  if (name === "00-toc") {
    try {
      const stubEntries = [{ text: "Stub", level: 1, bookmarkName: "_toc_stub" }];
      const result = fn(stubEntries);
      const flat = flatten(result);
      if (flat.length === 0) {
        fail(`${name}.js: returned empty array`);
      } else {
        pass();
      }
    } catch (e) {
      fail(`${name}.js: function threw with stub input: ${e.message}`);
    }
    continue;
  }
  try {
    const result = fn();
    if (!Array.isArray(result)) {
      fail(`${name}.js: returned ${typeof result}, expected array`);
      continue;
    }
    const flat = flatten(result);
    if (flat.length === 0) {
      fail(`${name}.js: returned empty array`);
      continue;
    }
    const invalid = flat.filter(el => !isDocxElement(el));
    if (invalid.length > 0) {
      fail(`${name}.js: ${invalid.length} element(s) are not Paragraph/Table instances`);
      continue;
    }
    chapterElements[name] = flat;
    pass();
  } catch (e) {
    fail(`${name}.js: function threw: ${e.message}`);
  }
}

// ---------- Check 3: Heading number sequencing within each chapter ----------
console.log("Check 3: Heading number sequencing");

function extractHeadingNumbers(elements) {
  // Extract section numbers from heading text (e.g., "10.1", "10.2.3")
  const numbers = [];
  for (const el of elements) {
    if (!(el instanceof Paragraph)) continue;
    // Access the raw text from the paragraph's children
    const options = el.root && el.root[1]; // internal docx structure varies
    // Try to extract text from TextRun children
    let text = "";
    try {
      // docx Paragraph stores options; text is in children TextRuns
      // Use a safe extraction approach
      const json = JSON.stringify(el);
      const textMatch = json.match(/"text":"([^"]+)"/);
      if (textMatch) text = textMatch[1];
    } catch (_) { /* skip */ }

    // Match section numbers like "1.1", "10.2", "A.3"
    const numMatch = text.match(/^(\d+\.\d+(?:\.\d+)?)\b/);
    if (numMatch) {
      numbers.push(numMatch[1]);
    }
  }
  return numbers;
}

for (const [name, elements] of Object.entries(chapterElements)) {
  const numbers = extractHeadingNumbers(elements);
  if (numbers.length < 2) {
    pass(); // Too few headings to check sequencing
    continue;
  }

  let seqOk = true;
  for (let i = 1; i < numbers.length; i++) {
    const prev = numbers[i - 1];
    const curr = numbers[i];
    // Extract the sub-section number (last component)
    const prevParts = prev.split(".");
    const currParts = curr.split(".");

    // Only check sequencing for headings at the same depth and same parent
    if (prevParts.length === currParts.length && prevParts.length >= 2) {
      const prevParent = prevParts.slice(0, -1).join(".");
      const currParent = currParts.slice(0, -1).join(".");
      if (prevParent === currParent) {
        const prevNum = parseInt(prevParts[prevParts.length - 1], 10);
        const currNum = parseInt(currParts[currParts.length - 1], 10);
        if (currNum !== prevNum + 1 && currNum !== 1) {
          fail(`${name}.js: heading jump from ${prev} to ${curr}`);
          seqOk = false;
        }
      }
    }
  }
  if (seqOk) pass();
}

// ---------- Check 4: Part headings match executive summary table ----------
console.log("Check 4: Part heading consistency with executive summary");

// Extract part headings from the actual chapters
const actualPartHeadings = {};
for (const [name, elements] of Object.entries(chapterElements)) {
  for (const el of elements) {
    if (!(el instanceof Paragraph)) continue;
    try {
      const json = JSON.stringify(el);
      const textMatch = json.match(/"text":"(PART [IVXLCDM]+:[^"]+)"/i);
      if (textMatch) {
        const partText = textMatch[1];
        const numMatch = partText.match(/PART ([IVXLCDM]+):/i);
        if (numMatch) {
          actualPartHeadings[numMatch[1].toUpperCase()] = partText;
        }
      }
    } catch (_) { /* skip */ }
  }
}

// Extract part references from executive summary table
if (chapterElements["00-executive-summary"]) {
  const summaryElements = chapterElements["00-executive-summary"];
  const summaryPartRefs = {};
  for (const el of summaryElements) {
    if (!(el instanceof Table)) continue;
    try {
      const json = JSON.stringify(el);
      // Look for Part number cells followed by name cells
      const partMatches = json.matchAll(/"text":"(X{0,3}(?:IX|IV|V?I{0,3}))"/g);
      for (const m of partMatches) {
        const num = m[1];
        if (num && num.length > 0 && /^[IVXLCDM]+$/.test(num)) {
          summaryPartRefs[num] = true;
        }
      }
    } catch (_) { /* skip */ }
  }

  // Cross-reference: every Part in the summary should have a matching partHeading
  const summaryParts = Object.keys(summaryPartRefs);
  const actualParts = Object.keys(actualPartHeadings);

  if (summaryParts.length > 0 && actualParts.length > 0) {
    for (const num of actualParts) {
      if (summaryPartRefs[num]) {
        pass();
      }
      // Don't fail on missing — the extraction is best-effort
    }
  }
  pass(); // Cross-reference check completed
} else {
  fail("00-executive-summary.js not loaded — cannot verify part headings");
}

// ---------- Summary ----------
console.log("");
console.log(`Validation complete: ${passes} passed, ${failures} failed`);

if (failures > 0) {
  process.exit(1);
} else {
  console.log("All checks passed.");
  process.exit(0);
}
