"use strict";

const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const packageRoot = path.resolve(__dirname, "..");
const version = process.env.npm_package_version || require(path.join(packageRoot, "package.json")).version;
const vendorRoot = path.join(packageRoot, "vendor", process.platform, process.arch);
const binaryName = process.platform === "win32" ? "kurir.exe" : "kurir";
const binaryPath = path.join(vendorRoot, binaryName);

if (process.env.KURIR_SKIP_INSTALL === "1" || process.env.KURIR_BIN) process.exit(0);
if (fs.existsSync(binaryPath)) process.exit(0);

const target = targetName();
const base = `https://github.com/suiflex/kurir/releases/download/v${version}`;
const temp = fs.mkdtempSync(path.join(os.tmpdir(), "kurir-install-"));
const archive = path.join(temp, process.platform === "win32" ? "kurir.zip" : "kurir.tar.gz");
try {
  download(`${base}/kurir-${version}-${target}.${process.platform === "win32" ? "zip" : "tar.gz"}`, archive);
  fs.mkdirSync(vendorRoot, { recursive: true });
  if (process.platform === "win32") {
    execFileSync("powershell", ["-NoProfile", "-Command", `Expand-Archive -Force '${archive}' '${temp}/expanded'`], { stdio: "inherit" });
    fs.copyFileSync(path.join(temp, "expanded", "kurir.exe"), binaryPath);
  } else {
    execFileSync("tar", ["-xzf", archive, "-C", temp], { stdio: "inherit" });
    fs.copyFileSync(path.join(temp, "kurir"), binaryPath);
    fs.chmodSync(binaryPath, 0o755);
  }
} catch (error) {
  console.error(`Kurir binary installation failed: ${error.message}`);
  console.error("Set KURIR_BIN to a compatible binary or KURIR_SKIP_INSTALL=1 to skip the download.");
  process.exit(1);
} finally {
  fs.rmSync(temp, { recursive: true, force: true });
}

function targetName() {
  const arch = process.arch === "arm64" ? "aarch64" : process.arch === "x64" ? "x86_64" : process.arch;
  const platform = process.platform === "darwin" ? "darwin" : process.platform === "win32" ? "windows" : process.platform;
  if (!['darwin', 'linux', 'windows'].includes(platform) || !['aarch64', 'x86_64'].includes(arch)) {
    throw new Error(`unsupported platform ${process.platform}-${process.arch}`);
  }
  return `${platform}-${arch}`;
}

function download(url, destination) {
  const script = [
    "const fs = require('node:fs');",
    "const [url, destination] = process.argv.slice(1);",
    "fetch(url).then(async (response) => {",
    "  if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);",
    "  fs.writeFileSync(destination, Buffer.from(await response.arrayBuffer()));",
    "}).catch((error) => { console.error(error.message); process.exit(1); });",
  ].join(" ");
  execFileSync(process.execPath, ["-e", script, url, destination], { stdio: "inherit" });
}
