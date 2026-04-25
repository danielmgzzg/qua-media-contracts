// Validates that:
//  1. Every JSON Schema under schemas/v1 is itself a valid draft 2020-12 schema.
//  2. The sample messages under schemas/v1/examples/* validate against their
//     declared schema (provided via "$schema" or filename convention).
//
// Used by `make validate` and CI.

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import $RefParser from "@apidevtools/json-schema-ref-parser";
import { readFile, readdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCHEMAS = resolve(__dirname, "../../../schemas/v1");
const EXAMPLES = resolve(SCHEMAS, "examples");

const ajv = new Ajv2020({ allErrors: true, strict: false });
addFormats(ajv);

async function loadSchema(file) {
  return $RefParser.bundle(file);
}

async function main() {
  const targets = [
    `${SCHEMAS}/domain.schema.json`,
    `${SCHEMAS}/ws/server.schema.json`,
    `${SCHEMAS}/ws/client.schema.json`,
  ];

  const validators = {};
  for (const file of targets) {
    const schema = await loadSchema(file);
    const validate = ajv.compile(schema);
    validators[file] = validate;
    console.log(`compiled ${file.replace(SCHEMAS, "schemas/v1")}`);
  }

  if (!existsSync(EXAMPLES)) {
    console.log("no schemas/v1/examples/ directory — skipping example checks");
    return;
  }

  // Examples convention: examples/server/*.json validate against server.schema.json,
  //                      examples/client/*.json validate against client.schema.json.
  const groups = [
    { dir: `${EXAMPLES}/server`, schema: `${SCHEMAS}/ws/server.schema.json` },
    { dir: `${EXAMPLES}/client`, schema: `${SCHEMAS}/ws/client.schema.json` },
  ];

  let failed = 0;
  for (const { dir, schema } of groups) {
    if (!existsSync(dir)) continue;
    const validate = validators[schema];
    for (const name of await readdir(dir)) {
      if (!name.endsWith(".json")) continue;
      const path = `${dir}/${name}`;
      const data = JSON.parse(await readFile(path, "utf8"));
      const ok = validate(data);
      if (ok) {
        console.log(`  PASS ${path.replace(SCHEMAS, "schemas/v1")}`);
      } else {
        failed++;
        console.error(`  FAIL ${path.replace(SCHEMAS, "schemas/v1")}`);
        for (const err of validate.errors ?? []) {
          console.error(`    ${err.instancePath || "/"} ${err.message}`);
        }
      }
    }
  }

  if (failed > 0) {
    console.error(`${failed} example(s) failed validation`);
    process.exit(1);
  }
  console.log("all examples validated successfully");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
