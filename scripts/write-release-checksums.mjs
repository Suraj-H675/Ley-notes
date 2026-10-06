import { createHash } from "node:crypto";
import {
  createReadStream,
  mkdirSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const target = resolve(root, "target");
const label = process.env.LEY_RELEASE_CHECKSUM_LABEL?.trim();
if (!label) throw new Error("LEY_RELEASE_CHECKSUM_LABEL is required");

const allowed = [".deb", ".rpm", ".dmg", ".msi", ".exe", ".zip", ".tar.gz"];
const files = walk(target)
  .filter((path) => path.includes(`${sep}bundle${sep}`))
  .filter((path) => {
    const suffix = path.split(`${sep}bundle${sep}`)[1];
    return suffix?.split(sep).length === 2;
  })
  .filter((path) => allowed.some((suffix) => path.endsWith(suffix)))
  .filter((path) => !path.endsWith(".sig"))
  .sort();

if (files.length === 0) {
  throw new Error("No release bundle files were found for checksumming");
}

const lines = [];
for (const path of files) {
  lines.push(`${await sha256(path)}  ${basename(path)}`);
}

const output = resolve(root, "release-checksums", `SHA256SUMS-${label}.txt`);
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, `${lines.join("\n")}\n`);
console.log(`${relative(root, output)} (${files.length} files)`);

function walk(directory) {
  if (!statSync(directory, { throwIfNoEntry: false })?.isDirectory()) return [];
  const files = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...walk(path));
    else if (entry.isFile()) files.push(path);
  }
  return files;
}

async function sha256(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest("hex");
}
