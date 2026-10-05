# Ley architecture

Ley is a local-first continuity/context system for coding agents. The architecture is intentionally smaller than
its historical notebook and experimental memory surfaces: current source, user intent, and explicit local
authority remain primary; retained history is bounded evidence that can help a later session continue work.

Historical ADRs and research documents explain how Ley reached this shape. They are evidence, not requirements for
preserving old implementation structure.

## Product surfaces

Ley has two current product surfaces plus one migration utility:

1. **Ley Desktop** — the native control center for explicit project selection, capture, Brief preview, historical
   recall, session/learning inspection, approved-source authority, evidence, privacy/egress, export, and erasure.
2. **Public website** — static product information. It has no project-memory backend and does not mount the Desktop
   runtime.
3. **Legacy browser recovery page** — a separate same-origin inspect/export/erase utility for retired browser-local
   notebook data. It is not a browser edition of Ley and does not promote that data into current continuity.

The former PWA/browser workspace, note editor, Canvas, note graph, bookmarks, and related notebook runtime are not
current product surfaces.

## Build boundary

Website and Desktop share source where useful but have separate entrypoints and artifacts:

- `index.html` + `src/website-main.tsx` + `vite.config.ts` -> `dist/`;
- `desktop/index.html` + `desktop/desktop-main.tsx` + `vite.desktop.config.ts` -> `dist-desktop/`;
- `src-tauri/tauri.conf.json` points only at the Desktop build.

The native shell and local commands live in `src-tauri/`; the Rust continuity engine, CLI, and MCP server live under
`crates/`.

## Project identity and local state

Each initialized project has a small repo-local `.ley/` directory. It carries stable project identity and explicit
capture configuration only; it is not a conversation database or hidden memory folder.

Machine-managed continuity lives in owner-private operating-system state. The canonical store is SQLite
(`continuity.sqlite3`), currently schema v11, partitioned by stable project ID. Its durable core is:

- `projects` — project identity;
- `events` — append-only/versioned continuity events;
- `event_links` — explicit provenance, dependency, and supersession links;
- native approved-source, artifact-snapshot, artifact-file, project-observation, and migration state needed by the
  current engine.

Large immutable cited evidence may live in Ley-managed content-addressed storage. Rebuildable indexes and
presentation projections are not independent authority.

Some older owner-private JSON registries and filesystem-vault data still exist as migration/privacy compatibility
inputs. They are not a second canonical product model.

## Capture and evidence

Capture is explicit and project-scoped. A new project previews the default Structured plan before initialization.
The repo-local capture mode plus `.leyignore`, Git ignore rules, hard file/byte bounds, no-follow filesystem access,
and credential-pattern redaction define the local evidence boundary.

Current capture modes are:

- **Minimal** — paths/classifications/hashes without source blobs;
- **Structured** — bounded post-redaction UTF-8 source evidence and citations;
- **Full Evidence** — Structured plus explicitly consented original PNG/JPEG/WebP evidence.

Captured evidence is historical. Ley does not silently relabel it as a live-source check.

Fresh/native capture persists artifact snapshots, captured bytes where consent allows them, capture time, and
bounded Git revision metadata. It does not build or persist a source-code graph. The remaining graph parser,
snapshots, and history belong to finite legacy-vault compatibility and are not part of canonical native authority.

## Continuity model

Ley preserves the facts that change a later engineering decision: goals/handoffs, decisions, constraints, attempts
and outcomes, verification, unresolved work, corrections/supersession, and exact evidence references.

Sessions and learnings remain evidence-bearing structured records rather than executable instructions. Human-reviewed
or otherwise trusted state still does not outrank current user intent or current repository/runtime evidence.

Git applicability is recomputed from bounded local Git evidence. Divergent historical state is withheld
where the canonical admission rules require it rather than being rewritten as though branch history never happened.

## Retrieval and Brief compilation

Canonical native, transition, and retained legacy Search use deterministic lexical ranking plus explicit trust,
authority, revision, conflict, egress, and budget handling. The previously bundled Model2Vec experiment and its
derived semantic-index implementation have been removed rather than kept as dormant product complexity. A future
model-assisted retrieval path would need a controlled native-state ablation to earn its dependency and authority
surface again.

The task-conditioned Brief is a deterministic compiler over bounded Search/authority inputs. Admission, withholding,
provenance, budget, and egress remain inspectable rather than being delegated to an opaque model classifier.

## Agent interface

The canonical native MCP contract is deliberately small:

- `ley_brief` — task-conditioned active-project continuity;
- `ley_search` — bounded historical recall, optionally against one exact explicitly selected already-observed project;
- `ley_evidence` — citation-bound historical evidence;
- `ley_checkpoint` — structured session write, only when writes were explicitly enabled at process start.

An inactive ordinary workspace exposes no project-memory capabilities and is not initialized implicitly. Bootstrap
Specification access for an uninitialized workspace is a separate explicit read-only authority path.

Host adapters establish/reuse session identity and capture bounded/redacted lifecycle evidence. Desktop separately
probes documented Codex/Claude CLI/plugin state for detection and Ley-managed configuration. Those probes do not
attest hook trust or runtime health. Recorded integration activity remains retained provenance; the explicit smoke
check passes only when Ley actually observes a matching host-hook session for the selected project.

## Human authority and egress

Privileged source approval, correction/review, deletion, export, and privacy mutation are human/local control
surfaces rather than ambient agent authority.

The current project egress vocabulary is:

- `agent-ok`;
- `local-model-only`;
- `confirm-per-use` (fail-closed until a real retrieval-scoped confirmation flow exists);
- `never-send`.

A local egress target is an explicit host/user assertion, not provider attestation. Retained finer-grained legacy
ancestry can still make a derivative more restrictive while migration is incomplete; historical data must not
launder itself through a newer, broader project policy.

## Portability, erasure, and recovery

Portable continuity export is project-scoped. It contains the selected project's continuity database plus only the
Ley-managed evidence actually cited by retained events. Import validates the bundle before installing runtime state.

Erasure removes Ley-controlled continuity for the selected project/session while preserving user-owned repository
files and independent copies. Ley does not claim forensic deletion of backups, filesystem snapshots, SSD remnants,
or downstream provider copies.

Interruption recovery is read-only evidence plus ordinary re-verification/checkpointing. The former shape-specific
recovery-writer MCP family is retired from the canonical product.

## Compatibility boundaries still in force

Cleanup must not silently orphan real local state. Current finite compatibility obligations include:

- legacy binding/vault reconnect for pre-cutover projects;
- one-time legacy project-catalog migration into native observations;
- legacy session/learning and approved-source import where native authority is not yet complete;
- retained Context Mount / Knowledge Scope / Policy Bundle / External Connector ancestry needed for cleanup and
  fail-closed egress;
- current Bootstrap Specification grants for uninitialized workspaces, plus retained legacy Bootstrap Reference
  grants only until local list/detach or initialization cleanup retires them;
- explicit historical-host import while that user-facing import workflow remains supported.

Creation/growth and content-contribution paths for those historical products are retired. Only bounded
inspect/remove/detach readers plus the ancestry reads needed for fail-closed egress remain; those compatibility
paths should disappear only after persisted-state obligations are explicitly closed, not merely because the current
machine happens to have no corresponding file.

## Security and trust invariants

Current implementation changes must preserve these boundaries unless new evidence justifies a deliberate redesign:

- explicit project/file scope and stable identity;
- owner-private machine state;
- no-follow filesystem containment;
- bounded/redacted capture and output;
- historical evidence is not instruction or current truth;
- provenance survives derivation;
- user intent and privileged human authority cannot be self-granted by an agent;
- revision applicability and uncertainty remain visible;
- egress and erasure fail closed;
- no unnecessary absolute local paths or unrelated project data cross agent/portable boundaries.

## Verification

`docs/runtime-verification.md` is the current human/runtime release contract. Deterministic P0/P1/P2 eval lanes
exercise the focused continuity capabilities; unit/integration tests cover lower-level correctness and retained
migration/privacy obligations. Model-dependent studies remain separate evidence and must identify their model,
host/version, fixture, and limits.

A passing historical fixture or ADR is not a reason to preserve a subsystem. A retained subsystem should continue to
exist only while the current product or a finite migration/security obligation still needs it.
