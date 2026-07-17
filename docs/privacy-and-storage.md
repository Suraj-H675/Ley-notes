# Local storage and data boundaries

Ley is local-first. The website may serve static HTML, CSS, JavaScript, fonts, icons, and application updates, but Ley has no knowledge-data backend.

## Storage contract

| Surface | Authoritative user data | Limitation |
| --- | --- | --- |
| Desktop app | User-selected folder containing Markdown, attachments, Canvas, and agent memory | Files depend on the user's normal device backup policy |
| Browser app with folder access | User-selected folder through a permissioned directory handle | Browser support and renewed permission vary |
| Browser-local compatibility mode | IndexedDB in the current browser profile | Clearing site data removes the vault; external local agents cannot open it |
| Public website | No knowledge data | Serves product information and static application assets only |

Browser-local mode requests persistent browser storage when available, but ZIP backup remains necessary. Filesystem vaults are the recommended durable mode.

## Agent boundary

Ley does not independently upload notes, project scans, sessions, or indexes. When a user intentionally asks a cloud agent such as Claude, Codex, or Gemini to retrieve Ley context, that selected context becomes visible to the agent provider as part of the agent request. A future local-model integration can keep both storage and inference on-device.

## Vault metadata

A vault-level `.ley/` directory is reserved for local Ley workspace and agent-memory files. An initialized code repository may also contain a minimal `.ley/` folder holding only its stable project identity, capture policy, and ignore rules. Repository-local metadata must not contain conversations, generated memories, credentials, embeddings, machine-specific vault paths, or raw transcripts.

The project-to-vault association lives outside both locations in Ley's private operating-system application configuration directory. Its versioned registry stores only stable project IDs and canonical vault paths. It contains no project path/name or knowledge content, is updated under a cross-process lock with atomic replacement, and can be deleted without deleting either project or vault data. A moved vault must be rebound explicitly; Ley does not guess another folder.

After an explicit ingestion command, deterministic project evidence lives under `<vault>/.ley/agent-memory/projects/<project-id>/`. Structured and Full Evidence modes retain allowed UTF-8 source text only after local credential redaction; Minimal mode retains artifact metadata and post-redaction hashes without source blobs. Binary/non-UTF-8 files, ignored paths, and detected raw values are not copied. Redaction is a safety layer, not a guarantee that every possible secret is recognizable, so preview and project-specific ignore rules remain important.

The durable agent project graph is derived locally from that same post-redaction boundary. It stores project-relative file/symbol citations, syntax-level imports/calls/inheritance, declared dependencies, and bounded tracked Git state. Git cannot introduce untracked or excluded paths. It stores no project root or vault path and does not run compilers, build scripts, package managers, language servers, or models.

`ley mcp` is a local stdio child process, not a network service. Each process is fixed to one explicitly initialized project and its resolved vault binding. Its current tools are read-only: they verify and query an existing captured snapshot, do not rescan live source, do not create a missing memory store, and do not expose project or vault absolute paths in results. A host may receive a bounded cited excerpt only after calling a retrieval tool; that host determines whether the excerpt stays local or is sent to its model provider.

Ordinary note search and visualization indexes remain disposable local database state. The versioned agent graph is a rebuildable but intentionally durable cited projection so agents can inspect immutable historical snapshots; deleting it must never delete source artifacts or the authoritative filesystem vault.

## Required safeguards

- Self-host runtime fonts, scripts, icons, and application assets.
- No hidden analytics, telemetry, remote database, or mandatory account.
- Constrain filesystem operations and project ingestion to explicitly approved roots.
- Exclude credentials, private keys, environment secrets, VCS internals, build output, and configured ignore patterns by default.
- Treat retrieved project or session text as untrusted evidence rather than executable agent instructions.
- Preserve citations, provenance, corrections, and superseded facts.
- Keep browser-local ZIP import/export and disclose its durability limitations.
