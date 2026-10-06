# Native release and update trust boundary

Ley's normal-user release is a signed native application, not a source-install recipe. The release workflow lives at
`.github/workflows/release.yml` and runs only for `v*` tags whose version exactly matches `package.json`,
`src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`.

## What the workflow produces

- Linux x86_64 on Ubuntu 22.04: AppImage, DEB, and RPM;
- macOS Apple Silicon and Intel: signed/notarized DMGs plus signed updater archives;
- Windows x86_64: Authenticode-signed NSIS and MSI installers plus signed updater archives;
- Tauri `latest.json` update metadata and platform updater signatures;
- one SHA-256 manifest per build lane.

The Linux lane intentionally uses Ubuntu 22.04, an older Tauri v2/WebKitGTK 4.1 baseline, so AppImage compatibility
is not defined by a rolling developer workstation. Each release lane is a clean hosted runner. Linux additionally
installs the produced DEB and executes the packaged Ley helper, inspects the RPM payload, and extracts the AppImage;
macOS signs and directly verifies the helper before it is copied into the signed/notarized app, then verifies code
signing/Gatekeeper plus that bundled helper; Windows Authenticode-signs the helper itself in addition to the
installers, then administratively extracts the MSI and verifies/executes that helper.

## External prerequisites

The workflow fails before creating a public release unless all of the following are configured. These are external
trust assets and must never be committed to the repository.

Ley's public [`CODE_SIGNING_POLICY.md`](../CODE_SIGNING_POLICY.md) defines who may approve releases, the provenance
requirements that must hold before signing, credential handling, and incident/key-rotation expectations. The release
workflow enforces two of those provenance gates directly before any build lane starts: the tagged commit must be
contained in `main`, and that exact commit must already have at least one successful completed ordinary CI run.

GitHub Actions **secrets**:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- `APPLE_CERTIFICATE` — base64 `.p12`
- `APPLE_CERTIFICATE_PASSWORD`
- `KEYCHAIN_PASSWORD`
- `APPLE_ID`
- `APPLE_PASSWORD` — Apple app-specific password
- `APPLE_TEAM_ID`
- `WINDOWS_CERTIFICATE` — base64 `.pfx`
- `WINDOWS_CERTIFICATE_PASSWORD`

GitHub Actions **variables**:

- `TAURI_UPDATER_PUBLIC_KEY`
- `APPLE_SIGNING_IDENTITY` — production Developer ID Application identity
- `WINDOWS_TIMESTAMP_URL` — the timestamp service required/recommended by the Windows certificate provider

### Windows signing provider checkpoint

The current workflow's `WINDOWS_CERTIFICATE` / `WINDOWS_CERTIFICATE_PASSWORD` path is a **provider-specific PFX
fallback**, not a statement that exportable PFX certificates are the preferred modern Windows trust model. Current
Tauri guidance explicitly limits its simple OV/PFX instructions to certificates acquired before June 1, 2023 and
documents Azure Artifact Signing plus custom signing commands for modern providers:

- <https://v2.tauri.app/distribute/sign/windows/>
- <https://learn.microsoft.com/windows/apps/package-and-deploy/code-signing-options>

For a public open-source Ley release, SignPath Foundation is a promising no-cost alternative because it integrates
with GitHub-hosted builds, verifies build origin, and supports deep Authenticode signing of an MSI together with
nested executables. That can preserve Ley's stronger requirement that the bundled `ley-helper.exe` itself is signed,
not merely the outer installer:

- <https://signpath.org/>
- <https://signpath.org/terms>
- <https://docs.signpath.io/trusted-build-systems/github>
- <https://docs.signpath.io/artifact-configuration/examples>

Do not add speculative SignPath workflow code until Ley has an approved SignPath project and the concrete project /
artifact-configuration / signing-policy identifiers needed to validate the integration. SignPath Foundation also
requires, among other things, an OSI-approved license, an already released project in the form to be signed, a public
code-signing policy, and its OSS process rules. Ley currently has no repository license declaration and no published
release, so that path is not yet eligible. Selecting an open-source license is a product/legal decision and must not
be silently made by release automation.

Microsoft Store MSIX distribution is another valid zero-certificate option: Microsoft re-signs Store-submitted MSIX
packages and provides Store-managed updates. It is not a drop-in replacement for Ley's current NSIS/MSI + GitHub
updater architecture, though; it introduces a Store-specific MSIX package identity, Partner Center submission, and a
separate update channel. Keep it as an evaluated alternative rather than adding a second packaging stack before Ley
has a concrete Store identity/distribution decision.

### Open-source licensing checkpoint

Public repository visibility is not itself an open-source license. As of the 2026-10-05 release review, Ley has no
top-level `LICENSE` file and no project license declaration in its root package manifests. A resolved dependency
metadata audit found no missing Rust/npm license declarations and no GPL/AGPL-only dependency in the current graphs;
the notable non-permissive-at-a-glance entries are MPL-2.0 packages (including Servo CSS dependencies and
`lightningcss`) and `caniuse-lite` under CC-BY-4.0. That inventory is useful evidence, not a legal conclusion.

Before calling Ley an OSS release, choose the project's own license explicitly and review/package the third-party
notices required by the dependencies actually distributed in the native application. Do not infer a project license
from dependency licenses or from the repository being public.

Ley now generates `generated/THIRD_PARTY_NOTICES.txt` from the locked, non-development dependency closures used by
the shipped Desktop binary, bundled `ley` helper, and production frontend. The file preserves package/version/license/
source metadata for every included dependency and deduplicates exact package-supplied license/notice texts by content
hash. Packages that publish a license expression without a standalone top-level notice file remain visible as such;
the generator does not fabricate missing copyright text. Normal CI proves the inventory can be regenerated from a
clean checkout, and every native release lane verifies that the non-empty notice file is present inside the packaged
application beside the helper. This improves attribution evidence but still does not choose Ley's own license or turn
metadata inventory into a legal conclusion.

The GitHub repository used for the release must also be public. Ley's updater endpoint is the public GitHub
`releases/latest/download/latest.json` asset; publishing from a private repository would not be a normal-user update
channel, so release preflight rejects it.

Generate the Tauri updater key pair with the project-local CLI under a restrictive umask, for example:

```bash
install -d -m 700 ~/.tauri
umask 077
npx tauri signer generate -w ~/.tauri/ley.key
```

Keep the private key and its password in protected release secrets/backups. The public key is safe to publish. Losing
the updater private key breaks the ability to ship trusted updates to already-installed Ley versions, so it is a
long-lived release credential rather than disposable CI state.

Register that identity with the release repository without putting secret material in Git or shell arguments:

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/ley.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD
gh variable set TAURI_UPDATER_PUBLIC_KEY --body "$(cat ~/.tauri/ley.key.pub)"
```

The password command prompts for the same password used when the key was generated. Do not replace this keypair after
a release has shipped unless Ley is deliberately performing an updater trust-key rotation; installed builds trust the
public key embedded when they were released.

Apple Developer ID signing/notarization and Windows code-signing certificates are separate from Tauri's updater key.
An updater signature proves update continuity to Ley; platform signatures/notarization establish OS/download trust.
Neither substitutes for the other.

## Release procedure

1. Update Ley's version consistently in the three version-owning manifests and run the normal test/build suite.
2. Confirm the release secrets/variables above and that the release repository is public.
3. Create and push `v<version>`.
4. The workflow runs preflight, builds/signs every platform lane into one draft release, verifies the native artifacts,
   uploads updater metadata/signatures and SHA-256 manifests, and publishes only after every required asset exists.
5. On an installed prior Ley version, use **Check for updates** and verify the published update before considering the
   release fully exercised.

Development builds contain no fabricated updater endpoint/public key. The update network request is user initiated;
Ley does not silently check for updates at startup.
