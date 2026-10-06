import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const generated = join(repoRoot, "src-tauri", "generated");

mkdirSync(generated, { recursive: true });
for (const file of ["LICENSE-MIT", "LICENSE-APACHE"]) {
  copyFileSync(join(repoRoot, file), join(generated, file));
}

console.log("Prepared Ley license resources for native bundles");
