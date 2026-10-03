# Runtime verification contract

Builds and unit tests do not prove that Ley is usable. This document is the current focused-product
runtime contract. The former notebook/PWA verification diary is preserved as
`archive/runtime-verification-notebook-legacy.md`; it is historical evidence, not a release checklist.

## Release baseline

Before treating a change as release-ready, run the checks proportional to the affected surface:

- frontend typecheck, lint, and tests;
- website and Desktop UI production builds;
- Rust formatting/check/tests for changed crates;
- the focused deterministic eval lanes (`--p0-coverage`, `--p1-coverage`, and `--p2-coverage`) when the
  continuity contract, retrieval, provenance, privacy, migration, or host-facing behavior changes;
- a native Desktop bundle when packaging or Tauri configuration changes.

Do not substitute the historical full eval corpus for these focused gates. Retired compatibility fixtures may
remain intentionally outside the current product matrix.

## Desktop launch and project boundary

- `npm run desktop` must start the Desktop development build without an unresolved module or webview load error.
- A fresh isolated Ley state opens the Projects hub without scanning neighboring folders or creating projects.
- Adding a project happens only after explicit folder selection.
- Removing a project from the Projects list removes only the device observation; it must not erase the project's
  continuity state or source files.
- A missing, moved, or identity-changed project fails closed and gives the user an explicit recovery/removal path.

## Project onboarding and capture

- A new project shows the bounded capture preview before initialization.
- Initialization writes only Ley's small repo-local identity/config plus owner-private application state; it must
  not create a new legacy vault.
- Refresh captures only files allowed by the active capture policy and ignore rules. Secret-like content,
  symlinks, binaries, oversized files, and excluded paths must preserve their existing fail-closed behavior.
- A moved legacy vault is accepted only through the reconnect/migration path after validating that it belongs to
  the exact Ley project. Migration compatibility must not silently widen capture or authority.

## Brief, Search, and Evidence

- Opening Overview must not compile a Brief automatically. Brief compilation starts only after the user supplies
  a task.
- Desktop Brief output must use the same canonical compiler contract as `ley_brief`: task/project identity,
  target, admitted items and Specifications, budget, premise/evidence state, gaps, egress exclusions, warnings,
  and citations must remain consistent.
- Cloud versus Local is an explicit egress target. Local is a host/user assertion, not proof that the downstream
  model is actually local.
- `confirm-per-use` remains fail-closed until Ley has a real confirmation boundary; the UI must not simulate one
  with an unrelated settings prompt.
- Project Search remains bounded, revision-aware, and explicit about stale/conflicting/untrusted history.
- Evidence reads must follow an exact Ley citation and revalidate project, snapshot/path/hash, and egress. Text
  citations should resolve historical text evidence; supported image citations should return the original bounded
  historical bytes rather than a generated description.

## Sessions, decisions, problems, and lessons

- Session views preserve the recorded project/revision provenance and never imply that retained history is live
  source truth.
- Decisions and Problems remain historical structured evidence. Divergent Git history must remain withheld where
  the canonical admission rules require it.
- Session rename and erasure use optimistic/current-state guards so a stale Desktop action cannot silently mutate
  newer state.
- Learning review/correction/supersession must preserve trust, freshness, evidence lineage, and replacement
  identity. Concurrent mutation of either side of a supersession must fail closed.
- Erasing Ley memory must not delete user-owned project files or unrelated Markdown/Canvas/archive copies.

## Approved sources and human authority

- Source approval/reapproval/revocation is a human-only authority surface.
- A changed or unavailable source must not be silently treated as still approved for its previous revision.
- Legacy source approvals may be migrated only through the bounded transition logic; old note-vault approvals that
  cannot be proven equivalent remain issues rather than becoming authority.
- Agent-facing tools cannot create privileged approved-source authority on their own.

## Capture, privacy, export, and erasure

- Desktop project egress policy and `ley egress list` must agree.
- A stale project identity or stale expected policy must reject a Desktop policy mutation before weakening the
  current boundary.
- More restrictive retained source-specific ancestry must continue to constrain derivatives during migration.
- Portable export contains only the selected project's continuity database plus evidence actually cited by those
  events; unrelated project state and uncited blobs must not leak into the bundle.
- Project-memory erasure removes Ley-controlled continuity state for the selected project without deleting the
  source repository. Do not describe this as forensic deletion of backups, filesystem remnants, or provider copies.

## Integrations and MCP

- Canonical native MCP discovery exposes only `ley_brief`, `ley_search`, and `ley_evidence`, plus
  `ley_checkpoint` when session writes were explicitly enabled.
- An inactive ordinary workspace exposes no project-memory tools and performs no implicit initialization or scan.
- Bootstrap Specification access for an uninitialized workspace remains explicit and read-only.
- Retained Bootstrap Reference, Context Mount, Knowledge Scope, Policy Bundle, and External Connector state remains
  inspect/remove/detach compatibility only. Release checks must not expose new growth paths, provider refresh/fetch,
  or retired cross-project content compilation while those records still constrain migration/privacy ancestry.
- Host lifecycle capture must preserve stable Ley session identity, bounded/redacted prompt/response evidence, and
  the guidance-only startup contract.
- Desktop's recorded integration activity is historical Ley evidence only. It must not claim that Codex, Claude,
  or another host is currently installed, trusted, connected, or healthy unless Ley actually verifies that state.

## Public website and legacy browser recovery

- The public website is static product information. It must not load the Desktop workspace, project-memory APIs,
  IndexedDB continuity state, or local-agent transports.
- The website must scroll normally at representative desktop and narrow widths without horizontal overflow.
- The separate legacy-recovery page performs no old IndexedDB access until the user explicitly requests it.
- Legacy browser recovery is archival inspect/export/erase only; it must not imply that the retired notebook data
  automatically becomes current Ley continuity.

## Accessibility and interaction

- Primary controls must remain keyboard reachable with visible focus treatment.
- Controls that behave as filters should use the keyboard/accessibility model for filters rather than claiming tab
  semantics unless an associated tab panel and arrow-key behavior exist.
- Loading and failure states that change the user's next action should be announced through appropriate status or
  alert semantics.
- Destructive actions must be visually and interactively subordinate to the primary continuation task and retain
  their existing explicit confirmations where data is actually erased.
- Reduced-motion and reduced-transparency preferences must preserve all required interactions and readability.

## Native packaging

- `npm run desktop:build` must produce the expected native bundle(s) for the current host platform.
- Inspect package identity/version and payload structure rather than assuming a successful build means the package
  is installable or signed.
- Signed distribution, updater behavior, and cross-platform installation/launch are separate release claims and
  must be verified before the product or website promises them.

## Evidence discipline

Record release evidence with the exact source revision, platform/toolchain, command or scenario, and relevant
limitations. A passing fixture proves its bounded contract; it is not evidence for arbitrary environments or
future host/model versions. Historical runtime evidence belongs in dated research/archival material rather than
being accumulated indefinitely in this current checklist.
