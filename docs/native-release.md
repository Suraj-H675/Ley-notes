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

The GitHub repository used for the release must also be public. Ley's updater endpoint is the public GitHub
`releases/latest/download/latest.json` asset; publishing from a private repository would not be a normal-user update
channel, so release preflight rejects it.

Generate the Tauri updater key pair with the pinned project CLI, for example:

```bash
npm run tauri signer generate -- -w ~/.tauri/ley.key
```

Keep the private key and its password in protected release secrets/backups. The public key is safe to publish. Losing
the updater private key breaks the ability to ship trusted updates to already-installed Ley versions, so it is a
long-lived release credential rather than disposable CI state.

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
