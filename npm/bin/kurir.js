#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function candidates() {
  const values = [];
  if (process.env.KURIR_BIN) values.push(process.env.KURIR_BIN);
  const local = path.join(__dirname, "..", "vendor", process.platform, process.arch, "kurir");
  values.push(process.platform === "win32" ? `${local}.exe` : local);
  const lookup = process.platform === "win32" ? "where" : "which";
  const found = spawnSync(lookup, ["kurir"], { encoding: "utf8" });
  if (found.status === 0) {
    for (const entry of found.stdout.split(/\r?\n/).map((value) => value.trim()).filter(Boolean)) {
      if (path.resolve(entry) !== path.resolve(process.argv[1])) values.push(entry);
    }
  }
  return values;
}

function executablePath() {
  for (const candidate of candidates()) {
    try {
      fs.accessSync(candidate, fs.constants.X_OK);
      return candidate;
    } catch {
      // Try the next candidate.
    }
  }
  return null;
}

const binary = executablePath();
if (!binary) {
  console.error("Kurir binary was not found.");
  console.error("Set KURIR_BIN or install a Kurir release package with its native binary.");
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`Could not start Kurir: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
