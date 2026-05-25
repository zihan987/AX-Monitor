#!/usr/bin/env node

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

if (platform && arch) {
  const binary = path.join(root, "prebuilt", `${platform}-${arch}`, "axmon");
  if (fs.existsSync(binary)) {
    fs.chmodSync(binary, 0o755);
    process.exit(0);
  }
}

const devBinary = path.join(root, "target", "release", "axmon");
if (fs.existsSync(devBinary)) {
  fs.chmodSync(devBinary, 0o755);
  process.exit(0);
}

console.error(`axmon install failed: no binary for ${process.platform}/${process.arch}.`);
console.error("This package currently ships linux/arm64 only.");
console.error("Build from source with: cargo build --release");
process.exit(1);
