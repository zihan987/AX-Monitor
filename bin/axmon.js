#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const platformMap = {
  linux: "linux",
};
const archMap = {
  arm64: "arm64",
};

const platform = platformMap[process.platform];
const arch = archMap[process.arch];
const bundledBinary =
  platform && arch ? path.join(root, "prebuilt", `${platform}-${arch}`, "axmon") : null;
const devBinary = path.join(root, "target", "release", "axmon");

function run(command, args, options = {}) {
  return spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    ...options,
  });
}

let binary = bundledBinary && fs.existsSync(bundledBinary) ? bundledBinary : null;
if (!binary && fs.existsSync(devBinary)) {
  binary = devBinary;
}

if (!binary) {
  console.error(`axmon does not include a binary for ${process.platform}/${process.arch}.`);
  console.error("Build from source with:");
  console.error("  cargo build --release");
  process.exit(1);
}

const child = run(binary, process.argv.slice(2));
if (child.error) {
  console.error(child.error.message);
  process.exit(1);
}
process.exit(child.status ?? 0);
