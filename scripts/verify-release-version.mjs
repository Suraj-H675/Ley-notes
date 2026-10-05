import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const packageVersion = JSON.parse(
  readFileSync(resolve(root, "package.json"), "utf8"),
).version;
const tauriVersion = JSON.parse(
  readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8"),
).version;
const cargo = readFileSync(resolve(root, "src-tauri/Cargo.toml"), "utf8");
const cargoPackage = cargo.match(
  /\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
);
const cargoVersion = cargoPackage?.[1];

if (!packageVersion || !tauriVersion || !cargoVersion) {
  throw new Error("Could not resolve every Ley release version");
}
if (new Set([packageVersion, tauriVersion, cargoVersion]).size !== 1) {
  throw new Error(
    `Release versions disagree: package=${packageVersion}, tauri=${tauriVersion}, cargo=${cargoVersion}`,
  );
}

const tag = process.env.LEY_RELEASE_TAG?.trim();
if (tag && tag !== `v${packageVersion}`) {
  throw new Error(`Release tag ${tag} does not match Ley v${packageVersion}`);
}

console.log(`Ley release version ${packageVersion} is consistent.`);
