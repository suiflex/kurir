"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const test = require("node:test");

const BIN = path.resolve(__dirname, "..", "bin", "kurir.js");

test("launcher delegates to KURIR_BIN", () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "kurir-npm-"));
  const fake = path.join(directory, process.platform === "win32" ? "fake.cmd" : "fake");
  if (process.platform === "win32") {
    fs.writeFileSync(fake, "@echo off\necho fake-kurir %*\n");
  } else {
    fs.writeFileSync(fake, "#!/bin/sh\nprintf 'fake-kurir %s\\n' \"$*\"\n");
    fs.chmodSync(fake, 0o755);
  }
  const result = spawnSync(process.execPath, [BIN, "clients"], {
    env: { ...process.env, KURIR_BIN: fake },
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /fake-kurir/);
  fs.rmSync(directory, { recursive: true, force: true });
});
