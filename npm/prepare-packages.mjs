#!/usr/bin/env node

import { chmod, cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const npmRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.dirname(npmRoot);
const [artifactsRoot, skillsRoot, outputRoot] = process.argv.slice(2).map((value) =>
  value ? path.resolve(value) : value
);
if (!artifactsRoot || !skillsRoot || !outputRoot) {
  throw new Error(
    "usage: node npm/prepare-packages.mjs <artifacts-dir> <skills-dir> <output-dir>"
  );
}

const cargoToml = await readFile(path.join(repoRoot, "Cargo.toml"), "utf8");
const version = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
if (!version) throw new Error("Cargo.toml [package].version not found");

const skillsManifest = JSON.parse(
  await readFile(path.join(skillsRoot, "manifest.json"), "utf8")
);
if (skillsManifest.cli !== version) {
  throw new Error(
    `skills manifest CLI version ${skillsManifest.cli} does not match ${version}`
  );
}
if (!Array.isArray(skillsManifest.files) || skillsManifest.files.length === 0) {
  throw new Error("skills manifest contains no files");
}
for (const file of skillsManifest.files) {
  if (typeof file.path !== "string") throw new Error("invalid skills manifest file path");
  const relative = path.normalize(file.path);
  if (path.isAbsolute(relative) || relative === ".." || relative.startsWith(`..${path.sep}`)) {
    throw new Error(`unsafe skills manifest file path: ${file.path}`);
  }
  await readFile(path.join(skillsRoot, relative));
}

const targets = [
  ["darwin-arm64", "aarch64-apple-darwin", "darwin", "arm64", "skz"],
  ["darwin-x64", "x86_64-apple-darwin", "darwin", "x64", "skz"],
  ["linux-arm64", "aarch64-unknown-linux-musl", "linux", "arm64", "skz"],
  ["linux-x64", "x86_64-unknown-linux-musl", "linux", "x64", "skz"],
  ["win32-x64", "x86_64-pc-windows-gnu", "win32", "x64", "skz.exe"]
];

await rm(outputRoot, { recursive: true, force: true });
await mkdir(outputRoot, { recursive: true });
const mainPackage = JSON.parse(await readFile(path.join(npmRoot, "package.json"), "utf8"));
mainPackage.version = version;
mainPackage.optionalDependencies = {};
const mainDir = path.join(outputRoot, "skz-quant-cli");
await mkdir(mainDir, { recursive: true });
await cp(path.join(npmRoot, "bin"), path.join(mainDir, "bin"), { recursive: true });
await cp(skillsRoot, path.join(mainDir, "bin", "skills"), { recursive: true });
await cp(path.join(npmRoot, "install.cjs"), path.join(mainDir, "install.cjs"));
await cp(path.join(repoRoot, "README.md"), path.join(mainDir, "README.md"));

for (const [platform, triple, os, cpu, binary] of targets) {
  const alias = `skz-quant-cli-${platform}`;
  const platformVersion = `${version}-${platform}`;
  mainPackage.optionalDependencies[alias] = `npm:skz-quant-cli@${platformVersion}`;

  const platformDir = path.join(outputRoot, alias);
  await mkdir(path.join(platformDir, "bin"), { recursive: true });
  const destination = path.join(platformDir, "bin", binary);
  await cp(path.join(artifactsRoot, triple, binary), destination);
  if (binary !== "skz.exe") await chmod(destination, 0o755);
  await writeFile(
    path.join(platformDir, "package.json"),
    `${JSON.stringify({
      name: "skz-quant-cli",
      version: platformVersion,
      description: `Native skz binary for ${platform}`,
      license: mainPackage.license,
      repository: mainPackage.repository,
      os: [os],
      cpu: [cpu],
      files: ["bin"]
    }, null, 2)}\n`
  );
}

await writeFile(path.join(mainDir, "package.json"), `${JSON.stringify(mainPackage, null, 2)}\n`);
console.log(`prepared skz-quant-cli ${version} and ${targets.length} platform versions`);
