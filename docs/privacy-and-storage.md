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

The project-to-vault association lives outside both locations in Ley's private operating-system application configuration directory. Its versioned binding registry stores only stable project IDs and canonical vault paths. It contains no project path/name or knowledge content, is updated under a cross-process lock with atomic replacement, and can be deleted without deleting either project or vault data. A moved vault must be rebound explicitly; Ley does not guess another folder.

A separate owner-private device catalog remembers only initialized project roots that the user or a local Ley client explicitly opens. It stores stable project IDs, canonical project paths, and last-opened times—never names, vault paths, source, notes, sessions, or generated memory. This catalog powers the desktop Projects view without searching the device. Missing folders remain visibly unavailable, removal affects only the device list, and two simultaneously available roots cannot claim the same copied `.ley` identity. The PWA cannot read this OS-local catalog.

Cross-project memory search reads that bounded catalog without adding entries. Each root must still present its recorded project identity and resolve its own available binding before Ley searches the corresponding captured vault namespace. Unavailable scopes are skipped, no neighboring folders are scanned, and searching does not ingest current source. Results can therefore be older than the working tree and retain captured-snapshot citations.

After an explicit ingestion command, deterministic project evidence lives under `<vault>/.ley/agent-memory/projects/<project-id>/`. Structured and Full Evidence modes retain allowed UTF-8 source text only after local credential redaction; Minimal mode retains artifact metadata and post-redaction hashes without source blobs. Binary/non-UTF-8 files, ignored paths, and detected raw values are not copied. Redaction is a safety layer, not a guarantee that every possible secret is recognizable, so preview and project-specific ignore rules remain important.

The durable agent project graph is derived locally from that same post-redaction boundary. It stores project-relative file/symbol citations, syntax-level imports/calls/inheritance, declared dependencies, and bounded tracked Git state. Git cannot introduce untracked or excluded paths. It stores no project root or vault path and does not run compilers, build scripts, package managers, language servers, or models. A bounded private history index references retained immutable graph/artifact pairs. Historical source inspection reads only an exact citation belonging to the selected graph, from that graph's matching redacted artifact; it never falls through to the live project. Minimal captures retain the relationship and citation metadata but no inspectable source text.

Structured session events live under `<vault>/.ley/agent-memory/projects/<project-id>/sessions/`. They contain bounded summaries, decisions, tasks, problem outcomes, commands, verification, handoffs, and citations that the user or an approved adapter submits. Ley does not capture a raw transcript automatically. It redacts recognized credential patterns before each immutable write, but users and adapters must still avoid environment dumps, complete tool logs, and unrelated private text. The derived JSON and Markdown session views can be rebuilt from the event sequence.

Desktop Capture & privacy controls change repository-evidence retention without changing the approved root implicitly. Minimal current snapshots omit source blobs; Structured retains post-redaction source; Full Evidence retains the same source and explicitly permits a compatible adapter to submit raw host evidence. Full Evidence requires a separate acknowledgement and still does not cause Ley to scrape transcripts. Mode changes preserve custom roots, ignore behavior, and byte limits, then refresh the bound local snapshot. Switching to Minimal is not secure deletion of earlier snapshots.

The separate exact-name **Erase Agent Memory** action removes the entire per-project memory namespace—artifacts and history, graph, sessions, and lessons—from the bound vault. It preserves project files, ordinary vault notes/Canvases, `.ley` metadata, the private binding, and the observation entry so the user can recapture deliberately. A sibling lifecycle lock serializes erasure against current-version memory readers and writers. This is logical local deletion, not a forensic wipe of backups, filesystem snapshots, SSD remnants, or external copies.

Evidence-backed project lessons live under the sibling `learnings/` directory. Their immutable events contain bounded claims, actor/provenance, confidence, references to existing session records, corrections, and review feedback. Agent proposals are not trusted automatically. Only an explicit user confirmation can establish trust; source changes remain visible through citation-hash freshness. Recognized credentials are redacted before persistence, but the same redaction limitations apply. Derived JSON and Markdown review views can be rebuilt from the event ledger.

`ley mcp` is a local stdio child process, not a network service. A ready process is fixed to one explicitly initialized project and its resolved vault binding. Its default tools are read-only: they verify and query existing project snapshots and immutable session history, do not rescan live source, do not create a missing memory store, and do not expose project or vault absolute paths in results. When a globally installed host package starts Ley in an ordinary or unavailable workspace, Ley stays protocol-valid but advertises zero capabilities, tools, and resources; it performs no discovery or writes. A host may receive a bounded cited excerpt or session handoff only after calling a retrieval tool; that host determines whether the result stays local or is sent to its model provider.

The optional `--allow-session-writes` launch flag adds append-only start, checkpoint, and finish tools for that process. The independent `--allow-learning-proposals` flag adds one agent-only, review-required proposal route. Neither enables deletion or raw transcript capture, and the proposal flag grants no review authority. Writes stay inside the bound filesystem vault and use the same local redaction and citation rules as the CLI. The host controls its own per-call approval interface.

Ordinary note search and visualization indexes remain disposable local database state. The versioned agent graph is a rebuildable but intentionally durable cited projection so agents can inspect immutable historical snapshots; deleting it must never delete source artifacts or the authoritative filesystem vault.

## Required safeguards

- Self-host runtime fonts, scripts, icons, and application assets.
- No hidden analytics, telemetry, remote database, or mandatory account.
- Constrain filesystem operations and project ingestion to explicitly approved roots.
- Exclude credentials, private keys, environment secrets, VCS internals, build output, and configured ignore patterns by default.
- Treat retrieved project or session text as untrusted evidence rather than executable agent instructions.
- Preserve citations, provenance, corrections, and superseded facts.
- Keep browser-local ZIP import/export and disclose its durability limitations.
