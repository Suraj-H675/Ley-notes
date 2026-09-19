# ADR 0054: Explicit Codex message-history import

Status: Accepted

## Context

Ley's roadmap includes explicit imports from supported historical host data, but the product explicitly rejects transcript hoarding and automatic parsing of unstable host-owned transcript files.

The first useful import slice therefore needs to recover some earlier host context without:

- scanning ambient host storage;
- binding Ley to a private rollout/transcript format;
- fabricating assistant/tool history the source does not contain;
- making an old conversation appear newly current because it was imported today;
- laundering imported text into policy or trusted memory;
- retaining machine paths or raw host identifiers unnecessarily.

Codex exposes a distinct global message-history file at `$CODEX_HOME/history.jsonl`. The open-source Codex implementation documents each line as a JSON object containing:

- `session_id` — UUID of the originating Codex session;
- `ts` — Unix seconds;
- `text` — the user's submitted message.

This message-history store is distinct from richer Codex rollout/session JSONL files. Source: <https://github.com/openai/codex/blob/main/codex-rs/message-history/src/lib.rs>.

## Decision

Ley adds one explicit local-user import command:

```text
ley session import codex-history PROJECT \
  --source FILE \
  --host-session SESSION_UUID
```

The user must supply both the source file and exact host session UUID. Ley does not discover `$CODEX_HOME`, scan `~/.codex`, infer which session should be imported, or invoke this path from lifecycle hooks.

The importer accepts only the documented Codex global message-history shape above. It is **not** a generic Codex transcript importer and does not parse rollout/session transcript files.

## Imported evidence semantics

The selected historical user messages become one completed Ley session with:

- `SessionSourceKind::Import`;
- `host: codex`;
- no claimed agent/model identity;
- one opaque `hsi_` source reference derived from the external Codex session UUID;
- no retained raw source path;
- no retained raw Codex session UUID;
- no fabricated assistant responses;
- no tool activity, hidden reasoning, rollout metadata, or environment history.

Each selected message becomes a bounded prompt observation with:

- `origin: import`;
- original source timestamp in `sourceRecordedAtUnixMs`;
- `sourceBoundary: untrusted-imported-host-history` on explicit read/Memory Compiler projections;
- ordinary Ley turn redaction and retention semantics.

Imported turn events use session schema v7 and rebuild to `session-v7.json`. Older session/event versions remain readable and are not rewritten merely because v7 exists.

The import receipt explicitly reports `assistantMessagesImported: 0`, `sourcePathRetained: false`, `rawHostSessionIdRetained: false`, `liveSourceChecked: false`, and `authority: untrusted-historical-evidence`.

## Capture modes

Historical message import is explicit bounded evidence submission, not raw-host-transcript capture. It therefore does not require Full Evidence transcript permission.

- **Minimal** records body-free imported observations.
- **Structured** retains bounded pattern-redacted selected user-message bodies.
- **Full Evidence** uses the same bounded turn semantics for this source; it does not broaden the importer into a rollout/transcript parser.

The normal per-turn and aggregate session retention limits continue to apply.

## Source/read bounds

The explicit source read fails closed unless it remains within these limits:

- 64 MiB source file;
- 100,000 JSONL records;
- 1 MiB per JSONL record;
- 512 selected messages for one import.

The path must identify a regular file. Ley checks metadata and opens with no-follow semantics; a supplied symlink is rejected. Malformed JSONL, invalid UUIDs, zero/unrepresentable timestamps, empty/unsafe selected text, or no matching selected session fail the import.

Ley currently validates every non-empty record in the supplied source rather than silently skipping malformed unrelated lines. This keeps the supported input contract deterministic and fail-closed.

## Snapshot identity and idempotency

The opaque `hsi_` source reference identifies the selected external Codex session without exposing its raw UUID.

A separate import snapshot digest covers:

- the opaque source reference;
- each selected message's original timestamp;
- each selected message's normalized, recognized-secret-redacted source text before the normal per-turn retention/truncation transform.

Deterministic Ley request IDs are derived from that snapshot digest.

Recognized secret values therefore do not influence durable import/request identity. Two otherwise
identical selected-message snapshots that differ only in a recognized secret value replay the same
redacted Ley snapshot rather than creating a secret-dependent durable hash.

Consequences:

- repeating the exact same selected-message snapshot replays the existing Ley import idempotently;
- if the selected Codex history changes, Ley creates a new immutable imported session;
- changed snapshots from the same external Codex session keep the same opaque `hsi_` source identity;
- deleting or changing the external history file later does not mutate already imported Ley evidence.

## Temporal semantics

Import time is not source time.

An old Codex session imported today must not become “recent work” merely because Ley wrote its event ledger today.

Therefore:

- imported sessions are excluded from automatic `ley resume` / `ley_project_resume` session selection;
- Resume reports `excludedImportedSessions` while `totalSessions` still counts the retained session;
- fixed-project Memory Search uses the newest imported source timestamp as the imported session's temporal signal instead of the Ley import timestamp;
- Memory Health does not flag an intentionally imported session merely because its raw turn evidence has no structured checkpoint.

Explicit session list/show/turn inspection, fixed-project search, and read-only Memory Compiler inspection remain available.

## Memory Compiler and authority

Imported history is evidence, not trusted structure.

The existing Memory Compiler may expose imported turns explicitly, preserving `untrusted-imported-host-history` and original source timestamps. Because the imported session is completed, `canCheckpoint` is false for that recovery projection; the importer itself never creates a decision, task, problem, resolution, verification result, learning, Specification, or policy.

If historical material should later become reusable project knowledge, it must go through Ley's ordinary explicit structured/review workflows. Transformation does not increase authority.

Imported text never grants tool, filesystem, network, review, write, or egress permission.

## Automatic host adapters remain unchanged

This ADR does not relax ADR 0018 or ADR 0025.

Codex and Claude Code lifecycle adapters continue to ignore host `transcript_path` fields and use documented stable lifecycle payload fields for automatic bounded turn capture. No automatic rollout/transcript parser is introduced.

Explicit historical import is a separate local CLI workflow. MCP exposes no import mutation route, and packaged skills instruct agents not to scan host storage or initiate import unless the user explicitly asks for it.

## Privacy and erasure

The durable imported Ley session lives in the same private per-project session namespace as other Agent Memory.

Ley does not retain:

- the external history file path;
- the raw Codex session UUID;
- unrelated sessions from the supplied history file;
- assistant/tool/hidden-reasoning records absent from the supported source;
- the original unredacted form of recognized secrets when retained bodies are enabled.

The ordinary session erasure workflow applies to the imported Ley session and any Ley-managed derivatives covered by that workflow. Ley does not claim to delete the external Codex history file, filesystem backups, provider-retained copies, or other independent user-owned copies.

## Deliberately deferred

This first slice does not add:

- Claude Code historical-history import;
- Codex rollout/session transcript parsing;
- assistant response import;
- tool-call/tool-output import;
- hidden-reasoning import;
- automatic `$CODEX_HOME` / `~/.codex` discovery;
- automatic migration of all historical sessions;
- lifecycle-triggered background import;
- background/local semantic consolidation of imported history;
- a desktop historical-import manager;
- generic arbitrary-host transcript adapters.

Each additional source requires its own stable supported contract, privacy boundary, provenance model, and evaluation before it can ship.

## Evaluation

P2 coverage includes deterministic `explicit-codex-message-history-import` through the real local CLI plus existing read-only MCP/session surfaces.

Passing requires:

- exact selected-session isolation;
- unrelated-session and secret canaries absent from returned/durable evidence;
- zero fabricated assistant messages;
- schema-v7 imported turns with `origin: import`, Codex host, original timestamps, and `untrusted-imported-host-history`;
- opaque `hsi_` provenance with no raw source path/host UUID disclosure;
- read-only Memory Compiler visibility with no automatic checkpoint authority;
- exclusion from automatic Resume;
- project-memory search temporal ranking by source time rather than import time;
- exact retry idempotency;
- changed selected history creating a new immutable Ley session with the same opaque source identity;
- retained imported evidence remaining readable after the external source file is removed;
- zero privacy-canary violations.

Focused core and CLI tests additionally cover Minimal body omission, malformed/missing session rejection, source symlink rejection, redaction, projection generation, and vault scans for raw identifiers/source-path leakage.
