# Capture structured agent sessions

Use `ley session` to preserve goals, decisions, verified work, problems, outcomes, and handoffs in the project's bound filesystem vault. Ley stores structured events, not a complete raw conversation.

## Before you capture a session

Initialize, bind, and ingest the project first:

```bash
ley init /path/to/project --capture structured
ley bind /path/to/project --vault /path/to/ley-vault
ley ingest /path/to/project
```

Ingestion establishes the approved artifact snapshot used by session citations. Session commands refuse an uninitialized, unbound, or un-ingested project.

## Start and finish a session

Start a session with a human-readable name and a concrete goal:

```bash
ley session start /path/to/project \
  --name "Implement offline search" \
  --goal "Add cited lexical retrieval and verify its limits"
```

The command prints a stable `ses_` ID. Keep that ID for later checkpoints. An adapter can add `--host codex --agent gpt-5` to record the capture source.

Record a compact checkpoint after a meaningful change:

```bash
ley session checkpoint ses_01234567890123456789012345678901 \
  /path/to/project \
  --summary "Implemented bounded lexical retrieval" \
  --touched src/search.ts \
  --command "npm run test" \
  --verification-passed "Search tests passed" \
  --unresolved "Add temporal reranking"
```

Finish the session with its outcome and handoff:

```bash
ley session finish ses_01234567890123456789012345678901 \
  /path/to/project \
  --status completed \
  --summary "Cited lexical retrieval is working" \
  --final-response "Implemented and verified the retrieval slice" \
  --handoff "Add temporal reranking next"
```

Use `paused` when another session should continue the work. Use `abandoned` when the approach should not continue.

## Rename a session without rewriting history

The agent-suggested name is not permanent. Rename a session from its desktop inspector, or use the manual CLI:

```bash
ley session rename ses_01234567890123456789012345678901 \
  /path/to/project \
  --name "Ship offline search" \
  --note "The completed scope is clearer than the original working title" \
  --expected-events 4
```

Ley appends the new name and required reason as another immutable event. The original name and every later naming revision remain visible, while the stable `ses_` ID, citations, learnings, and captured work do not change. Renaming also works after a session is completed, paused, or abandoned.

`--expected-events` is optional for deliberate CLI automation and recommended whenever the caller previously read the session. The desktop always sends it. If an agent or another process appends an event first, Ley rejects the stale rename and asks the caller to reload. MCP session-write permission does not include rename authority.

## Erase one session’s Agent Memory

The desktop session inspector exposes **Erase session memory** as a reviewed destructive action. It requires the exact current session name and rejects the operation if another writer appended an event after the inspector loaded.

Agent-facing Session Context/Turns responses keep two schema identities separate:
`projectionSchemaVersion` describes the reader JSON contract, while `schemaVersion` remains the
underlying durable session ledger schema. A later event-schema upgrade therefore does not silently
redefine the meaning of the reader payload. See ADR 0083.

When a session contains explicit context-utility bindings, the same inspector also shows a compact
**Context utility measurement** section. It reports bound, uniquely observed, and unobserved binding
counts plus at most the already-bounded unobserved binding metadata returned by Session Context. A
terminal unobserved row may state that the retained finish event is available as an exact observation
anchor. This is provenance/coverage only: the desktop does not infer context usage, helpfulness,
harmfulness, causation, trust, or ranking, and it exposes no automatic observation/repair action.

The equivalent local CLI command is intentionally explicit:

```bash
ley session erase ses_01234567890123456789012345678901 \
  /path/to/project \
  --confirm-name "Ship offline search" \
  --expected-events 4
```

Ley physically removes that structured session and every learning whose proposal or correction cited it. It also removes any learning whose supersession chain would otherwise point to one of those erased records. Unrelated sessions, unrelated learnings, captured project artifacts, graph history, source files, `.ley` policy, and the vault binding remain.

Ordinary Markdown handoffs and JSON Canvas documents are user-owned copies and remain. Delete those through the normal note or Canvas workflow if they should also be forgotten. Session erasure is not a forensic wipe and cannot remove backups, filesystem snapshots, provider-retained context, storage remnants, or external copies. MCP and automatic host adapters cannot erase sessions. See [ADR 0024](../adr/0024-reviewed-session-memory-erasure.md).

## Record decisions and problem outcomes

Pass a JSON document to record the complete checkpoint model:

```bash
ley session checkpoint ses_01234567890123456789012345678901 \
  /path/to/project \
  --data checkpoint.json
```

The document must match the versioned checkpoint input schema. Include a stable `requestId` when a hook may retry delivery:

```json
{
  "requestId": "req_01234567890123456789012345678901",
  "summary": "Fixed projection recovery",
  "decisions": [
    {
      "title": "Source of truth",
      "decision": "Replay immutable events",
      "rationale": "Derived files can be recreated"
    }
  ],
  "problems": [
    {
      "title": "Missing projection after interruption",
      "symptom": "session.md was absent",
      "attempts": [
        {
          "action": "Replay the event directory",
          "outcome": "helped",
          "evidence": "The complete session was reconstructed"
        }
      ],
      "resolution": {
        "rootCause": "The process stopped before projection replacement",
        "change": "Treat events as authoritative",
        "verification": "A read succeeds without either projection"
      }
    }
  ],
  "touchedArtifacts": ["src/session.ts"]
}
```

Artifact paths must exist in the current captured snapshot. Ley stores a snapshot-pinned citation instead of the unverified path alone. Text citations carry captured line ranges. Under explicit Full Evidence capture, supported PNG/JPEG/WebP originals may also be cited; those citations carry `mediaType` with `startLine: 0` and `endLine: 0` to state that the evidence is non-text. Ley does not OCR or describe the image automatically.

Ley also derives a project revision for every new checkpoint. It pins the exact immutable graph and artifact snapshots used while accepting that checkpoint, plus the Git HEAD, branch, and captured tracked-change count when the approved capture came from a Git repository. Agents do not submit this metadata, and Ley does not read live Git during checkpoint recording. A checkpoint made after the source changes but before the next ingestion therefore continues to cite the earlier approved capture. Replaying the same request after a later ingestion returns the original event and revision.

## Inspect captured sessions

List sessions for a project:

```bash
ley session list /path/to/project
```

Read one bounded reconstructed session context. This reports prompt/response
counts but does not return their bodies:

```bash
ley session show ses_01234567890123456789012345678901 \
  /path/to/project --json
```

The vault also contains `session.md` for review and a derived JSON projection for local tools. V1-only ledgers use `session-v1.json`; deliberate turn evidence advances the projection to v2, provenance-bound unresolved recovery checkpoints to v3, text verification-evidence checkpoints to v4, context-utility bind/observe events to v5, checkpoints with multimodal artifact citations to v6, explicitly imported host turns to v7, provenance-bound typed minimal Decision/Problem recovery checkpoints to v8, verifier-bound typed Task recovery checkpoints to v9, verifier-bound typed Plan recovery checkpoints to v10, atomic multi-claim recovery checkpoints with record-specific evidence bindings to v11, verifier-bound rich Problem recovery checkpoints with per-Problem/Attempt/Resolution evidence bindings to v12, atomic rich-Problem composite recovery checkpoints to v13, deterministic supported host-tool observations to v14, claim-bearing Procedure application observations to v15, and isolated verifier-bound observed-Command recovery checkpoints to v16. Older immutable events remain readable without rewrite and older projections may remain beside the newest projection. Preserve the immutable event files when repairing or migrating memory.

Unresolved checkpoint entries remain durable strings for compatibility. Read-side session context additionally derives one deterministic `unr_...` record ID per returned unresolved item, aligned by index in `unresolvedRecordIds`. That ID is a citation handle only: it does not alter the stored checkpoint shape or grant trust/authority. For schema-v11 recovery, citing such a child preserves the child's exact recovery-evidence subset instead of attributing the full atomic batch evidence union.

Ley Desktop exposes the same event history in **Agent Memory → Sessions**. Opening a session uses the bounded shared context projection rather than trusting a mutable Markdown summary. It shows turn counts without loading bodies; **Captured turns** performs a separate bounded local read only when expanded and labels prompts/responses plus supported host-tool observations as untrusted history. Tool observations stay separate from checkpoint Commands and do not claim verification success. The inspector also shows recent checkpoints, decisions, tasks, problem attempts and outcomes, structured resolution root causes and verification, commands, handoff, unresolved work, snapshot-pinned artifact citations, captured project revisions, and naming history. Text citations open bounded captured excerpts. Media citations route to the Artifact surface and open the exact cited original snapshot/hash, explicitly labeled as original media with no OCR/vision description or live-source check. Captured Git/revision metadata remains provenance; the retired Project Graph view is no longer a Desktop navigation target. Older records and truncated text are disclosed instead of being presented as complete history.

The desktop **Projects** search also indexes those checkpoint revisions. Paste a full or partial captured Git SHA, a branch name, a graph snapshot ID, or an artifact snapshot ID to recover the owning session across explicitly observed and currently bound projects. Opening a revision result enters that session first so its checkpoint context remains visible.

### External Markdown and historical note exports

The focused Desktop no longer writes inspected sessions or learnings into a Ley-managed notes vault or Canvas.
Human-authored Markdown belongs outside Ley's continuity authority, while Ley's own portable continuity
export/import path handles product-owned backup and migration. Historical Markdown/Canvas copies created by
older versions remain user-owned files; retiring the integration does not make those copies authoritative and
does not justify deleting them. ADR 0021 documents the retired note-link design as historical context; ADR 0022
still describes checkpoint revision citations.

The focused Desktop now exposes those two boundaries directly. **Approved Sources** can hand a current approved
project-file `.md`/`.mdx` source to the operating system's default external application after revalidating the exact
approved bytes and immediately rechecking the project-relative path for symlink substitution; the final OS handoff
is still a point-in-time pathname handoff, not an atomic retained-byte handle. Imported snapshots and stale/missing
sources are never opened as editable files. Once an external editor saves new bytes, the old approval becomes stale
until explicitly reviewed/reapproved. That editor's own backup, sync, extension, and cloud behavior is outside Ley's
privacy boundary. **Capture &
privacy → Export continuity bundle** asks for a local parent folder and writes a new portable continuity
bundle outside the project tree using the same validated exporter used by core migration tests. The export contains
Ley-owned continuity plus only cited immutable evidence required by that continuity; it is not a project-source or
Markdown export. See ADR 0096.

## Recover a missed checkpoint after an interruption

Bounded prompt/response observations and supported host-tool observations are source evidence, not structured memory. If a host crashes or an agent stops before checkpointing, Ley can retain that post-checkpoint evidence and later report a bounded recovery signal without injecting the retained bodies into ordinary startup context.

`ley_session_memory_compile` is now **read-only recovery inspection**. It reports the current session status, checkpoint boundary, bounded `tev_` prompt/response evidence, separately bounded supporting tool observations, omission/truncation counts, and whether the session is still active. `reviewable-evidence`, `partial-evidence`, and `metadata-only` describe evidence quality only; they do not authorize reconstruction writes. A normal host tool return is not proof that a command/test succeeded, and any automatic Command candidate/fingerprint exposed by the compiler is diagnostic historical metadata only. `automaticCommandWriteAllowed` remains false.

If current work depends on an interrupted claim, inspect the retained evidence and then verify the relevant repository/runtime state with the coding host's normal live tools. For an **active** current Ley session, record only newly supportable state through the ordinary checkpoint workflow (`ley_checkpoint` in the canonical MCP surface, or the local/session lifecycle equivalent). Do not convert a retained request into a claimed result, do not reconstruct structured Decisions/Tasks/Problems merely because candidate text looks plausible, and do not rewrite paused/completed/abandoned historical sessions. For deeper local inspection, use `ley session show` / `ley session turns`.

Historical schema-v3/v8–v16 recovery checkpoint events remain readable and strictly validated so older Ley data is not corrupted or laundered during migration. Their shape-specific verifier/commit MCP routes are retired from the current model-facing surface.

## Review native sessions at lifecycle boundaries for reusable guidance

Use the local Consolidation Inbox only when deliberately reviewing retained evidence after a meaningful pause/completion/abandonment boundary:

```bash
ley consolidation inbox /path/to/project --json
```

The inbox is an on-demand read-only view over native paused, completed, and abandoned sessions. It excludes active sessions and explicitly imported historical sessions, returns stable session/turn IDs plus bounded counts rather than prompt/response bodies, and does not invoke a model, start background work, persist state, reopen or mutate cited sessions, or claim semantic faithfulness. Schema v2 makes its bounds explicit: `inspectedSessionsWithUnconsolidatedEvidence` describes only the sessions actually examined, while `allEligibleSessionsInspected` and `sessionsOmitted` disclose whether older eligible lifecycle-boundary sessions remain outside the view. Re-running the unchanged view may surface the same evidence again because this slice has no durable "processed" marker.

When a returned item contains exact captured `tev_` prompt/response handles and the retained evidence genuinely supports reusable guidance, a separate `learning propose` action may cite those turn IDs directly. Body-free observations cannot support a learning proposal. Every such proposal still starts tentative/review-required, carries direct turn-evidence origin lineage, and requires the normal learning review workflow; the inbox itself grants no learning-write or trust authority.

## Retry a write safely

Supply `--request-id req_01234567890123456789012345678901` when another process may repeat a start, compact checkpoint, turn capture, or finish call. The same ID and retained content replay the original event. The same ID with different retained content fails instead of creating ambiguous history. Body-free Minimal/capacity disclosures deliberately retain no content fingerprint, so they can validate identity and metadata but cannot compare an omitted retry body.

Generated request IDs are safe for interactive commands. Host adapters should persist their request ID until they receive a successful response.

## Import selected historical Codex message history

Historical import is explicit and separate from automatic lifecycle capture:

```bash
ley session import codex-history /path/to/project \
  --source /path/to/history.jsonl \
  --host-session 11111111-1111-4111-8111-111111111111 \
  --json
```

The first supported format is Codex's global message-history JSONL: one object per line with
`session_id`, Unix-second `ts`, and user-message `text`. The user supplies both the file
and the exact session UUID. Ley does not search `~/.codex`, does not follow a supplied
symlink, and does not treat Codex rollout/session transcript JSONL as this format.

The import is intentionally narrow:

- at most 64 MiB of source file, 100,000 JSONL records, 1 MiB per line, and 512 messages for the selected session;
- selected user messages only — assistant responses, tools, hidden reasoning, model identity, and rollout metadata are not reconstructed;
- an opaque `hsi_` source reference is retained instead of the raw host UUID or source path;
- original message timestamps are retained as `sourceRecordedAtUnixMs`;
- Structured/Full Evidence apply normal per-turn redaction and retention; Minimal records body-free imported observations;
- imported turns are schema-v7 events with `origin: import` and `sourceBoundary: untrusted-imported-host-history`;
- the resulting Ley session is completed immediately and reports `liveSourceChecked: false`.

Re-importing the exact same selected-message snapshot is an idempotent replay. If the selected
Codex history changed, Ley creates a new immutable imported Ley session while preserving the
same opaque source reference. Deleting or changing the external history file later does not
mutate an already imported snapshot.

Imported sessions do not become automatic Resume context or an automatic missed-checkpoint recovery
target. Use explicit session list/show/turn inspection, project-memory
search, or the read-only per-session recovery compiler when the history is relevant. Imported
sessions are deliberately excluded from the local Consolidation Inbox. Existing explicit
inspection/proposal workflows remain separate; Ley does not sweep imported history for reusable
knowledge automatically. Memory Compiler keeps the imported-history boundary/source timestamps
and cannot checkpoint a completed imported session automatically.

The existing `session erase` workflow applies to an imported Ley session exactly like any other
private session memory. Removing the external Codex source file itself is outside Ley's erasure
authority.

## Understand the privacy boundary

Ley applies local credential-pattern redaction before it writes session text. It also bounds event size and collection counts, rejects symlinks and malformed history, and serializes concurrent writers.

Redaction cannot guarantee that arbitrary private text is safe. Use Minimal mode when automatic prompt/response/tool bodies or explicitly imported historical message bodies should not be retained. Structured and Full Evidence keep only bounded, pattern-redacted automatic-evidence bodies under one shared session capacity. Lifecycle adapters still never read a complete transcript automatically; the separate Codex history importer reads only the explicitly selected bounded message-history source described above. Ley does not send session data anywhere, but a cloud agent can receive context that you intentionally retrieve through that agent.
