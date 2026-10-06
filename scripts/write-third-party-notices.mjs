import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outputPath = join(
  repoRoot,
  "src-tauri",
  "generated",
  "THIRD_PARTY_NOTICES.txt",
);
const noticeName =
  /^(licen[cs]e|unlicense|copying|notice|copyrights?|patents?)([._-].*)?$/i;

function run(command, args) {
  return execFileSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    stdio: ["ignore", "pipe", "inherit"],
  });
}

function hostTriple() {
  if (process.env.TAURI_ENV_TARGET_TRIPLE)
    return process.env.TAURI_ENV_TARGET_TRIPLE;
  const match = run("rustc", ["-vV"]).match(/^host:\s+(.+)$/m);
  if (!match) throw new Error("Could not determine Rust host target triple");
  return match[1].trim();
}

function readableNoticeFiles(packageRoot, explicitLicenseFile) {
  const files = new Set();
  if (explicitLicenseFile) {
    const fullPath = resolve(packageRoot, explicitLicenseFile);
    if (existsSync(fullPath) && statSync(fullPath).isFile())
      files.add(fullPath);
  }
  for (const name of readdirSync(packageRoot)) {
    const fullPath = join(packageRoot, name);
    if (noticeName.test(name) && statSync(fullPath).isFile())
      files.add(fullPath);
  }
  return [...files].sort();
}

function normalizeRepository(repository) {
  if (!repository) return "";
  if (typeof repository === "string") return repository;
  return repository.url ?? "";
}

function rustPackages(targetTriple) {
  const metadata = JSON.parse(
    run("cargo", [
      "metadata",
      "--locked",
      "--format-version",
      "1",
      "--filter-platform",
      targetTriple,
    ]),
  );
  const workspace = new Set(metadata.workspace_members);
  const packages = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const roots = metadata.packages
    .filter((pkg) => pkg.name === "ley-cli" || pkg.name === "ley-desktop")
    .map((pkg) => pkg.id);
  if (roots.length !== 2) {
    throw new Error(
      `Expected ley-cli and ley-desktop Cargo roots, found ${roots.length}`,
    );
  }

  const seen = new Set(roots);
  const pending = [...roots];
  while (pending.length > 0) {
    const id = pending.pop();
    const node = nodes.get(id);
    if (!node) continue;
    for (const dependency of node.deps) {
      const kinds = dependency.dep_kinds ?? [];
      if (kinds.length > 0 && kinds.every((kind) => kind.kind === "dev"))
        continue;
      if (!seen.has(dependency.pkg)) {
        seen.add(dependency.pkg);
        pending.push(dependency.pkg);
      }
    }
  }

  return [...seen]
    .filter((id) => !workspace.has(id))
    .map((id) => {
      const pkg = packages.get(id);
      if (!pkg) throw new Error(`Cargo metadata omitted package ${id}`);
      if (!pkg.license && !pkg.license_file) {
        throw new Error(
          `Rust dependency ${pkg.name} ${pkg.version} has no declared license`,
        );
      }
      const packageRoot = dirname(pkg.manifest_path);
      return {
        ecosystem: "Rust crate",
        name: pkg.name,
        version: pkg.version,
        license: pkg.license ?? `SEE ${pkg.license_file}`,
        source: pkg.repository ?? pkg.homepage ?? pkg.source ?? "",
        files: readableNoticeFiles(packageRoot, pkg.license_file),
      };
    });
}

function npmPackages() {
  const installed = run("npm", ["ls", "--omit=dev", "--all", "--parseable"])
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && resolve(line) !== repoRoot);
  const entries = [];
  const seen = new Set();

  for (const packageRoot of installed) {
    const manifestPath = join(packageRoot, "package.json");
    if (!existsSync(manifestPath)) {
      throw new Error(
        `Installed npm dependency is missing package.json: ${packageRoot}`,
      );
    }
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
    const name = manifest.name;
    const version = manifest.version;
    if (!name || !version)
      throw new Error(
        `Installed npm dependency has incomplete metadata: ${packageRoot}`,
      );
    const key = `${name}@${version}`;
    if (seen.has(key)) continue;
    seen.add(key);

    const license = manifest.license;
    if (!license)
      throw new Error(`npm dependency ${key} has no declared license`);
    entries.push({
      ecosystem: "npm package",
      name,
      version,
      license: typeof license === "string" ? license : JSON.stringify(license),
      source:
        normalizeRepository(manifest.repository) || manifest.homepage || "",
      files: readableNoticeFiles(
        packageRoot,
        manifest.licenseFile ?? manifest.license_file,
      ),
    });
  }

  return entries;
}

function renderEntry(entry, noticeIds) {
  const lines = [
    "=".repeat(80),
    `${entry.ecosystem}: ${entry.name} ${entry.version}`,
    `License: ${entry.license}`,
  ];
  if (entry.source) lines.push(`Source: ${entry.source}`);

  if (entry.files.length === 0) {
    lines.push(
      "Package-supplied notice text: none found at the package root; the declared license metadata above is retained.",
    );
    return lines.join("\n");
  }

  for (const file of entry.files) {
    const text = readFileSync(file, "utf8").trim();
    const hash = createHash("sha256").update(text).digest("hex");
    lines.push(
      `Notice text: ${noticeIds.get(hash)} (${file.split(/[\\/]/).pop()})`,
    );
  }
  return lines.join("\n");
}

const targetTriple = hostTriple();
const entries = [...rustPackages(targetTriple), ...npmPackages()].sort((a, b) =>
  [a.ecosystem, a.name, a.version]
    .join("\0")
    .localeCompare([b.ecosystem, b.name, b.version].join("\0")),
);

const noticeTexts = new Map();
for (const entry of entries) {
  for (const file of entry.files) {
    const text = readFileSync(file, "utf8").trim();
    const hash = createHash("sha256").update(text).digest("hex");
    const reference = `${entry.ecosystem}: ${entry.name} ${entry.version} (${file.split(/[\\/]/).pop()})`;
    const existing = noticeTexts.get(hash);
    if (existing) {
      existing.references.push(reference);
    } else {
      noticeTexts.set(hash, { text, references: [reference] });
    }
  }
}

const orderedNotices = [...noticeTexts.entries()].sort(([left], [right]) =>
  left.localeCompare(right),
);
const noticeIds = new Map(
  orderedNotices.map(([hash], index) => [
    hash,
    `T${String(index + 1).padStart(3, "0")}`,
  ]),
);

const output = [
  "LEY THIRD-PARTY NOTICES",
  "",
  `Target: ${targetTriple}`,
  "Generated from Cargo.lock/package-lock.json and installed locked package contents.",
  "Rust scope: non-dev dependency closure of the shipped ley-desktop and ley-cli binaries.",
  "npm scope: installed non-dev packages used by the shipped frontend.",
  "",
  "This file is an attribution and license-evidence inventory. It preserves each package's declared license",
  "metadata and exact package-supplied license/notice text when that text is present. It does not choose or",
  "declare Ley's own project license and is not a substitute for legal review of release obligations.",
  "",
  `Packages: ${entries.length}`,
  `Unique package-supplied notice texts: ${orderedNotices.length}`,
  "",
  ...entries.map((entry) => renderEntry(entry, noticeIds)),
  "",
  "# Package-supplied license and notice texts",
  "",
  ...orderedNotices.map(([hash, notice]) =>
    [
      "=".repeat(80),
      `${noticeIds.get(hash)} — SHA-256 ${hash}`,
      "Referenced by:",
      ...notice.references.sort().map((reference) => `- ${reference}`),
      "",
      notice.text,
    ].join("\n"),
  ),
  "",
].join("\n");

mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, output, "utf8");
console.log(
  `Wrote ${entries.length} third-party dependency notices to ${outputPath}`,
);
