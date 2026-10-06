# Code signing policy

Ley treats release signing as a provenance and user-trust boundary, not as a packaging checkbox.

## Scope

This policy applies to native Ley Desktop releases and the bundled `ley` helper distributed with them.
Release signing currently covers four distinct controls:

- the Tauri updater signature, which establishes update continuity for an installed Ley version;
- macOS Developer ID signing and notarization;
- Windows Authenticode signing, regardless of the eventual certificate/signing provider;
- release checksums and CI evidence tying published artifacts back to one repository revision.

Signing one layer does not substitute for another. In particular, the bundled helper is verified as its own signed
executable on macOS and Windows because Ley copies it into an app-owned location where coding-agent hosts can run it
independently of the Desktop process.

## Release authority and roles

Ley is currently maintained in the public repository `Suraj-H675/Ley-notes` by its repository owner/maintainer.
Until additional maintainers are explicitly added, that maintainer fills these release roles:

- **Committer / reviewer** — [`Suraj-H675`](https://github.com/Suraj-H675); may merge or push accepted source, build,
  CI, and release changes.
- **Release approver** — [`Suraj-H675`](https://github.com/Suraj-H675); decides whether a specific version is ready
  to receive a release tag and platform signatures.

These are distinct responsibilities even when one person currently fills both roles. Adding collaborators does not
implicitly make them release approvers; the policy must be updated when the release-approval set changes.

Every production signing operation requires an explicit release action. Ley does not sign arbitrary branch builds,
pull-request artifacts, or local developer builds with production release credentials. If a signing provider adds a
separate manual approval gate, that approval is mandatory in addition to Ley's repository-side release checks.

## Source and build provenance

Production releases are tag-driven and fail closed. A release tag must:

1. use Ley's `v<version>` tag format;
2. identify a commit that is contained in the repository's `main` branch;
3. match the version declared consistently by Ley's version-owning manifests;
4. identify a commit whose ordinary GitHub Actions CI run completed successfully;
5. build on clean GitHub-hosted release runners from the checked-in workflow and locked dependency state.

Release review includes CI configuration, package/build scripts, signing configuration, and dependency-lock changes,
not only application source. Build provenance must remain inspectable from the repository revision and GitHub Actions
run that produced the release.

## Signing credentials

Private signing material must never be committed to the repository.

- The long-lived Tauri updater private key and password are release credentials. Losing or replacing that identity
  breaks updater continuity for installations that trust the previous public key.
- Apple signing/notarization credentials remain external release credentials and are injected only into the hosted
  release job that needs them.
- Windows signing may use an imported certificate, a managed/HSM-backed provider, or another reviewed mechanism. The
  provider is an implementation choice; valid Authenticode signatures and signed-helper verification are the trust
  outcome Ley requires.

Repository automation must fail rather than publish an unsigned or partially signed release when required signing
material is absent.

## Artifact verification and publication

Before publication, Ley's release workflow verifies the artifacts appropriate to each platform, including the
bundled helper, updater signatures, and the generated third-party notice inventory. It publishes only after all
required platform lanes and release assets are present.

The first signed release is not considered full updater proof. Ley must also exercise an already-installed prior
version updating through the published signed updater channel before claiming that update path as production-proven.

## Privacy and network behavior

Ley is local-first. Ley Desktop does not transfer project memory or captured project content to other networked
systems unless the user explicitly requests a workflow that does so. The user-initiated **Check for updates** action
contacts Ley's configured public update endpoint but does not send project memory or captured project content.

Host integration setup may modify the selected local coding-agent configuration only after explicit user action; it
does not silently configure hosts on Desktop startup.

Ley provides corresponding project-scoped **Disconnect** controls and normal application uninstall instructions in
the repository README. Uninstalling the application and erasing retained project continuity are separate explicit
operations; uninstall does not silently claim deletion of user data or external copies.

## Incident response and key rotation

If a release credential is suspected to be exposed, signing must stop until the affected credential and published
trust path are assessed. A compromised platform certificate is revoked/replaced according to its provider's process.
An updater-key rotation is more disruptive because installed Ley versions embed the trusted updater public key; it
must be treated as an explicit migration rather than silently generating a replacement key.

Incorrect or untrusted release assets must not be "fixed" in place while retaining misleading provenance. Prefer a
new reviewed release and preserve enough public evidence to explain the correction.

## SignPath Foundation

Ley has evaluated SignPath Foundation as a possible Windows signing provider but does not currently claim to be
accepted or signed by SignPath Foundation. If that provider is adopted, this policy and Ley's release/download page
will be updated with SignPath Foundation's required attribution, current team-role links, and provider-specific manual
approval/origin-verification rules before the first SignPath-signed release.

See [`docs/native-release.md`](docs/native-release.md) for the operational release prerequisites and current provider
status.
