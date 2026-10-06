# Ley

Ley is being rebuilt as a **local-first Project Brain for software development**.

A repository tells an agent much of what exists now. Ley is for the project history and source context that
usually disappears or becomes ambiguous across sessions: what was required, decided, attempted, failed,
verified, superseded, left unresolved, and what evidence supports those conclusions.

The current contract is [`LEY.md`](LEY.md). The 2026-10-06 M0 audit maps the existing repository onto that
contract in [`docs/research/project-brain-m0-reconciliation-2026-10-06.md`](docs/research/project-brain-m0-reconciliation-2026-10-06.md).
Existing ADRs and older research remain useful implementation history, but they are not automatically current
requirements.

> **Migration status:** the repository still implements the preceding focused-continuity runtime while the
> Project Brain milestones are being built. This README distinguishes the target product from behavior that
> already ships; it does not claim the new Desktop IA or Brain data model is finished yet.

## Product surfaces

Ley now has exactly two intended surfaces:

- **Ley Desktop** — the actual local application. It owns project access, local agent integrations, privacy controls, review, search/recall, evidence inspection, and the continuity workflow.
- **Ley website** — a normal public marketing/showcase site. It does not run a reduced copy of Ley in the browser; detailed documentation lives in this repository.

The previous `/app` browser workspace, PWA, browser-folder mode, and browser-local notebook mode have been retired. A web page cannot provide the same local project/MCP boundary as the native product, and maintaining a parallel notebook implementation added substantial complexity without strengthening Ley's differentiated value.

## Project Brain direction

The target product gives each software project one durable logical Brain. A Brain may exist before code and
can contain retained sources/specifications, repository evidence, Codex/Claude sessions, supported observable
activity, decisions, failed attempts, solutions, verification, unresolved work, and evidence-backed knowledge.

The central rule is **store broadly, retrieve selectively**: retained history can be rich, while agent context
stays bounded, task-relevant, evidence-linked, privacy-aware, and explicit about stale or uncertain applicability.

The eventual Desktop shell is deliberately sparse:

- global: **Home / Projects / Settings**;
- inside a Brain: **Overview / Knowledge / History / Review**;
- optional **Project Contents** rail: **Sources / Repository**.

The user-facing coding-agent workflow centers on the Ley skill/invocation (`$ley` in Codex and the cleanest
native equivalent in other hosts). The local engine—not prompt text—enforces project scope, evidence,
idempotency/concurrency, human-authority, capture, and egress boundaries.

Deterministic local import is the core path. Optional Desktop-triggered Codex/Claude analysis is a later,
explicit egress action whose output remains candidate knowledge rather than truth.

Ley is not becoming a general-purpose note editor/PKM, IDE, terminal, ambient cross-project memory service, or
graph-demo homepage.

## Current implementation during the remake

Today the implementation still exposes the focused continuity foundation that the Project Brain will build on:

- owner-private transactional SQLite continuity state;
- bounded/redacted project capture and exact retained-evidence citations;
- revision/applicability evidence;
- project/session erasure and egress controls;
- a small native MCP surface (`ley_brief`, `ley_search`, `ley_evidence`, and opt-in `ley_checkpoint`);
- project-scoped Codex and Claude lifecycle integrations;
- the current Agent Memory Desktop surfaces.

Those names and the current UI are transition behavior, not permanent Project Brain ontology. General-purpose
note editing, backlinks, Canvas, daily notes, and the retired notebook runtime remain out of scope. Historical
browser-local notebook data is still handled only by the explicit recovery utility.

## Important principles

- current user intent and live project evidence outrank historical memory;
- historical agent text is context/evidence, not instruction or truth;
- durable memory preserves provenance;
- project and cross-project scope must be explicit;
- context is small and task-conditioned by default;
- privacy, egress, erasure, and export remain inspectable;
- Git revision/applicability stays lightweight and visible;
- derived indexes/views are rebuildable;
- advanced features must beat a simpler baseline in realistic agent-task evaluation before they earn their complexity.

See [`LEY.md`](LEY.md) for the Project Brain contract and [`docs/README.md`](docs/README.md) for current,
transition, and historical documentation boundaries.

## Install and first run

Ley's normal-user distribution is the native Desktop app. Release builds produce a macOS DMG, Windows NSIS/MSI
installers, and Linux DEB/RPM packages; Arch Linux is intended to use its native AUR/package-manager path. The
application carries the matching native Ley engine itself; normal users do **not** install Rust, Node.js, clone this
repository, run `cargo install`, or add `ley` to `PATH`.

AppImage is deliberately not a required v0.1 distribution target. Current Tauri/WebKitGTK AppImage tooling has active
cross-distro build/runtime reliability problems, including modern rolling-distribution failures. Ley prefers native
package-manager formats it can verify end to end rather than advertising a portable artifact we cannot currently
trust across supported Linux environments.

The **currently implemented** first launch remains deliberately small while the new onboarding milestone is pending:

1. choose a coding-project folder;
2. review the exact proposed capture boundary and exclusions;
3. enable Ley and create the first local snapshot;
4. optionally connect detected Codex or Claude Code installations from Ley Desktop;
5. restart/review the host when Ley says that host requires it, then use **Run smoke check** to verify that Ley has
   actually observed host-hook activity.

Ley Desktop is not a daemon. A configured coding agent starts Ley's app-owned helper directly when its hooks/MCP
server need it, so closing Desktop does not disable continuity.

The repository now contains a signed cross-platform release/update pipeline, but this README does **not** claim a
public release exists until that pipeline has actually run with the required Apple, Windows, and updater signing
credentials from a public release repository. See [`docs/native-release.md`](docs/native-release.md) for the exact
release trust boundary and prerequisites.

## Uninstall

Before removing Ley Desktop, disconnect Ley from each coding project that you no longer want connected:

1. open the project in Ley Desktop;
2. go to **Project settings → Coding agents**;
3. choose **Disconnect** for each Ley-managed Codex or Claude Code integration;
4. restart that coding agent if Ley reports that a restart is required.

Disconnect is deliberately project-scoped. For Codex, Ley removes only the exact Ley-owned project MCP binding and
plugin enablement from `.codex/config.toml`; unrelated settings and foreign `mcp_servers.ley` entries are preserved.
For Claude Code, Ley uninstalls `ley-memory@ley-desktop` only at project scope. Shared marketplace/package metadata
may remain because another Ley project on the machine can still depend on it.

If you also want Ley's retained continuity for a project removed, use **Project settings → Capture & privacy → Erase
memory…** before uninstalling. Application uninstall and project-memory erasure are intentionally different actions:
removing the app does not claim to erase user data, exported backups, or user-owned project files.

Then remove the application using the normal platform mechanism:

- **macOS:** quit Ley and remove `Ley.app` from Applications;
- **Windows:** **Settings → Apps → Installed apps → Ley → Uninstall**;
- **Debian/Ubuntu package:** `sudo apt remove ley`;
- **RPM-based distributions:** remove the installed Ley package with the distribution's normal package manager.
- **Arch/AUR:** remove the installed Ley package with `pacman` or the AUR helper used to install it.

Reinstalling Ley does not automatically reconnect a coding project; host setup remains an explicit user action.

## Code signing policy

Ley's public [code signing policy](CODE_SIGNING_POLICY.md) defines release authority, source/build provenance,
credential handling, platform/helper-signature expectations, privacy, and signing-incident response. Production
release tags must resolve to reviewed `main` history with successful ordinary CI before signing is allowed.

## License

Ley is dual-licensed under your choice of either the [MIT License](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE). The SPDX expression used by the package manifests is
`MIT OR Apache-2.0`: recipients may choose either license at their option.

## Repository map

```text
.
├── crates/               # Rust core, CLI, MCP, and supporting adapters
├── desktop/              # Dedicated native frontend HTML entry
├── docs/                 # Current docs plus historical ADR/research evidence
├── eval/                 # Deterministic and real-agent evaluation runners
├── integrations/         # Codex and Claude Code packages
├── src/
│   ├── app/              # Native desktop React composition
│   ├── features/         # Focused continuity desktop feature slices
│   ├── shared/           # Reusable UI/state/utilities
│   └── website/          # Public marketing site
└── src-tauri/            # Native shell and local filesystem/project commands
```

The website and desktop share reusable source code where useful, but they have **separate entrypoints and build artifacts**. A website deployment cannot expose the desktop application runtime.

## Development

Requirements today: Node.js **24.21.0** and Rust **1.98.1**, pinned by `.node-version` and
`rust-toolchain.toml`, plus the Tauri 2 platform prerequisites for your operating system. With `rustup`
installed, the repository toolchain file selects/installs the pinned Rust toolchain automatically.

```bash
npm install

npm run dev:website       # public marketing/showcase site
npm run build:website

npm run desktop           # Tauri development app
npm run build:desktop-ui  # desktop webview frontend only
npm run desktop:build     # native application bundle
npm run desktop:package:arch # local pacman package for production-like Arch testing

npm run typecheck
npm run lint
npm test
cargo check --locked --workspace
```

`npm run build` currently aliases the website production build.

Routine GitHub Actions CI runs on pushes to `main` and pull requests. It continuously checks the frontend
typecheck/lint/tests/website+desktop-UI builds/production dependency audit and the pinned Rust workspace
format/check/tests on Ubuntu. The manual six-lane portability/security workflow remains separate because
its Linux/macOS/Windows x64+ARM64 matrix is intentionally more expensive and evidence-oriented.

Native release tags use a separate fail-closed workflow. It refuses to publish when release versions disagree, the
GitHub release repository is not public, or required updater/platform signing material is absent. Successful lanes
build and verify Linux/macOS/Windows bundles on clean hosted runners, upload signed updater artifacts plus SHA-256
checksum manifests, and publish only after all required assets are present.

## Compatibility state during the remake

The retired notebook implementation has been removed. Remaining compatibility code exists only where it still
protects real migration, recovery, privacy/erasure, or historical local data; it is not a second product surface
and must not grow new authority. The Project Brain reset does not authorize deleting those paths until each real
persisted-state/privacy obligation has a verified migration or removal condition.

Work being retained or adapted includes:

- bounded credential redaction and evidence handling;
- stable project identity and idempotency patterns;
- structured sessions/handoffs;
- exact provenance/citation IDs;
- Git revision relation logic;
- privacy/egress/erasure tests;
- six-lane Linux/macOS/Windows x64/ARM64 portability evidence;
- host compatibility probes.

Canonical artifact/session/learning/approved-source continuity now lives in transactional SQLite. A smaller set of legacy JSON registries remains only where it still carries migration, cleanup, or privacy/egress ancestry; those records no longer expand the canonical agent context surface. Large immutable evidence may remain content-addressed files. Old browser IndexedDB bytes remain in the user's browser profile until the user chooses the migration-only same-origin recovery page to export or erase them; they are no longer part of the product runtime.

## Evaluation and Project Brain growth

The earlier Phase-0 comparison remains useful evidence, not authority over the Project Brain shape. A pinned
`gpt-6-luna` / `xhigh` study compared
no history, a human `HANDOFF.md`, a tiny historical brief, and current/full Ley across four controlled
continuity tasks. After fixing two evaluator artifacts and replacing the affected observations, the final
sample contains 48 valid attempts with three observations per task/arm cell and zero runner failures.

The result does **not** justify preserving the current architecture wholesale. It still supports keeping
structured verification/evidence, bounded event evidence, explicit selected-source context, and lightweight
revision applicability as strong foundations. The new milestones will test the broader Project Brain workflows
directly, especially failed-attempt avoidance and sources-first project creation.

See [`docs/research/phase0-frontier-agent-benchmark-2026-09-26.md`](docs/research/phase0-frontier-agent-benchmark-2026-09-26.md)
for methodology, corrected results, discarded evaluator artifacts, limits, and migration implications.
