import { copyFileSync, chmodSync, mkdirSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const root = resolve(scriptDir, "..");

function commandOutput(command, args) {
  return execFileSync(command, args, {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  }).trim();
}

function hostTargetTriple() {
  const output = commandOutput("rustc", ["-vV"]);
  const host = output
    .split(/\r?\n/)
    .find((line) => line.startsWith("host: "));
  if (!host) throw new Error("rustc did not report a host target triple");
  return host.slice("host: ".length);
}

const target = process.env.TAURI_ENV_TARGET_TRIPLE || hostTargetTriple();
const debug = process.env.TAURI_ENV_DEBUG === "true";
const profile = debug ? "debug" : "release";
const windows = target.includes("windows");
const executable = windows ? "ley.exe" : "ley";
const generated = windows ? "ley-helper.exe" : "ley-helper";

const cargoArgs = ["build", "--locked", "-p", "ley-cli", "--target", target];
if (!debug) cargoArgs.push("--release");

execFileSync("cargo", cargoArgs, { cwd: root, stdio: "inherit" });

const targetDir = resolve(root, process.env.CARGO_TARGET_DIR || "target");
const source = join(targetDir, target, profile, executable);
const destination = join(root, "src-tauri", "generated", generated);

mkdirSync(dirname(destination), { recursive: true });
copyFileSync(source, destination);
if (!windows) chmodSync(destination, 0o755);

signReleaseHelper(destination, target);

console.log(`Prepared Ley helper for ${target}: ${destination}`);

function signReleaseHelper(helperPath, targetTriple) {
  if (targetTriple.includes("apple-darwin")) {
    const identity = process.env.APPLE_SIGNING_IDENTITY?.trim();
    if (!identity) return;
    execFileSync(
      "codesign",
      [
        "--force",
        "--timestamp",
        "--options",
        "runtime",
        "--sign",
        identity,
        helperPath,
      ],
      { cwd: root, stdio: "inherit" },
    );
    execFileSync(
      "codesign",
      ["--verify", "--strict", "--verbose=2", helperPath],
      { cwd: root, stdio: "inherit" },
    );
    return;
  }

  if (targetTriple.includes("windows")) {
    const thumbprint = process.env.LEY_WINDOWS_CERT_THUMBPRINT?.trim();
    const timestampUrl = process.env.LEY_WINDOWS_TIMESTAMP_URL?.trim();
    if (!thumbprint && !timestampUrl) return;
    if (!thumbprint || !timestampUrl) {
      throw new Error(
        "Both LEY_WINDOWS_CERT_THUMBPRINT and LEY_WINDOWS_TIMESTAMP_URL are required to sign the bundled helper",
      );
    }
    const env = { ...process.env, LEY_HELPER_TO_SIGN: helperPath };
    const script = [
      "$certPath = 'Cert:\\CurrentUser\\My\\' + $env:LEY_WINDOWS_CERT_THUMBPRINT",
      "$certificate = Get-Item $certPath -ErrorAction Stop",
      "$signature = Set-AuthenticodeSignature -FilePath $env:LEY_HELPER_TO_SIGN -Certificate $certificate -HashAlgorithm SHA256 -TimestampServer $env:LEY_WINDOWS_TIMESTAMP_URL",
      "if ($signature.Status -ne 'Valid') { throw ('Bundled Ley helper signature is ' + $signature.Status) }",
    ].join("; ");
    execFileSync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-Command", script],
      { cwd: root, env, stdio: "inherit" },
    );
  }
}
