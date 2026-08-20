#!/usr/bin/env node

const {
  chmodSync,
  copyFileSync,
  linkSync,
  readFileSync,
  statSync,
  unlinkSync,
  writeFileSync
} = require("node:fs");
const path = require("node:path");

const platforms = {
  "darwin-arm64": ["@shengkezhi-com/skz-quant-cli-darwin-arm64", "skz"],
  "darwin-x64": ["@shengkezhi-com/skz-quant-cli-darwin-x64", "skz"],
  "linux-arm64": ["@shengkezhi-com/skz-quant-cli-linux-arm64", "skz"],
  "linux-x64": ["@shengkezhi-com/skz-quant-cli-linux-x64", "skz"],
  "win32-x64": ["@shengkezhi-com/skz-quant-cli-win32-x64", "skz.exe"]
};

function placeBinary(source, destination) {
  const fallback = statSync(destination).size < 4096 ? readFileSync(destination) : null;
  try {
    unlinkSync(destination);
    linkSync(source, destination);
  } catch (linkError) {
    try {
      copyFileSync(source, destination);
    } catch (copyError) {
      if (fallback) writeFileSync(destination, fallback, { mode: 0o755 });
      throw new Error(`hardlink failed (${linkError.message}); copy failed (${copyError.message})`);
    }
  }
  if (process.platform !== "win32") chmodSync(destination, 0o755);
}

const key = `${process.platform}-${process.arch}`;
const target = platforms[key];
if (!target) throw new Error(`@shengkezhi-com/skz-quant-cli does not support ${key}`);

let source;
try {
  const packageJson = require.resolve(`${target[0]}/package.json`);
  source = path.join(path.dirname(packageJson), "bin", target[1]);
} catch {
  throw new Error(
    `native package ${target[0]} is missing; reinstall without --omit=optional`
  );
}

placeBinary(source, path.join(__dirname, "bin", "skz.exe"));
console.log(`installed native skz for ${key}`);
