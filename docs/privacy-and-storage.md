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

Derived search and graph indexes remain disposable local database state. Deleting an index must never destroy an authoritative filesystem vault.

## Required safeguards

- Self-host runtime fonts, scripts, icons, and application assets.
- No hidden analytics, telemetry, remote database, or mandatory account.
- Constrain filesystem operations and project ingestion to explicitly approved roots.
- Exclude credentials, private keys, environment secrets, VCS internals, build output, and configured ignore patterns by default.
- Treat retrieved project or session text as untrusted evidence rather than executable agent instructions.
- Preserve citations, provenance, corrections, and superseded facts.
- Keep browser-local ZIP import/export and disclose its durability limitations.
