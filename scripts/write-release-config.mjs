import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const pubkey = process.env.TAURI_UPDATER_PUBLIC_KEY?.trim();
const endpoint = process.env.LEY_UPDATER_ENDPOINT?.trim();

if (!pubkey) throw new Error("TAURI_UPDATER_PUBLIC_KEY is required");
if (!endpoint) throw new Error("LEY_UPDATER_ENDPOINT is required");

const parsedEndpoint = new URL(endpoint);
if (parsedEndpoint.protocol !== "https:") {
  throw new Error("LEY_UPDATER_ENDPOINT must use HTTPS");
}

const bundle = { createUpdaterArtifacts: true };
if (process.platform === "win32") {
  const certificateThumbprint = process.env.LEY_WINDOWS_CERT_THUMBPRINT?.trim();
  const timestampUrl = process.env.LEY_WINDOWS_TIMESTAMP_URL?.trim();
  if (!certificateThumbprint) {
    throw new Error("LEY_WINDOWS_CERT_THUMBPRINT is required on Windows");
  }
  if (!timestampUrl) {
    throw new Error("LEY_WINDOWS_TIMESTAMP_URL is required on Windows");
  }
  bundle.windows = {
    certificateThumbprint,
    digestAlgorithm: "sha256",
    timestampUrl,
  };
}

const destination = resolve(root, "src-tauri/generated/release.conf.json");
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(
  destination,
  `${JSON.stringify({
    bundle,
    plugins: { updater: { pubkey, endpoints: [endpoint] } },
  }, null, 2)}\n`,
);
console.log(destination);
