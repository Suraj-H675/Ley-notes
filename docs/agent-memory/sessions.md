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

The vault also contains `session.md` for review and a derived JSON projection for local tools. V1-only ledgers use `session-v1.json`; deliberate turn evidence advances the projection to v2, provenance-bound unresolved recovery checkpoints to v3, text verification-evidence checkpoints to v4, context-utility bind/observe events to v5, checkpoints with multimodal artifact citations to v6, explicitly imported host turns to v7, provenance-bound typed minimal Decision/Problem recovery checkpoints to v8, verifier-bound typed Task recovery checkpoints to v9, verifier-bound typed Plan recovery checkpoints to v10, atomic multi-claim recovery checkpoints with record-specific evidence bindings to v11, verifier-bound rich Problem recovery checkpoints with per-Problem/Attempt/Resolution evidence bindings to v12, atomic rich-Problem composite recovery checkpoints to v13, and deterministic supported host-tool observations to v14. Older immutable events remain readable without rewrite and older projections may remain beside the newest projection. Preserve the immutable event files when repairing or migrating memory.

Unresolved checkpoint entries remain durable strings for compatibility. Read-side session context additionally derives one deterministic `unr_...` record ID per returned unresolved item, aligned by index in `unresolvedRecordIds`. That ID is a citation handle only: it does not alter the stored checkpoint shape or grant trust/authority. For schema-v11 recovery, citing such a child preserves the child's exact recovery-evidence subset instead of attributing the full atomic batch evidence union.

Ley Desktop exposes the same event history in **Agent Memory → Sessions**. Opening a session uses the bounded shared context projection rather than trusting a mutable Markdown summary. It shows turn counts without loading bodies; **Captured turns** performs a separate bounded local read only when expanded and labels prompts/responses plus supported host-tool observations as untrusted history. Tool observations stay separate from checkpoint Commands and do not claim verification success. The inspector also shows recent checkpoints, decisions, tasks, problem attempts and outcomes, structured resolution root causes and verification, commands, handoff, unresolved work, snapshot-pinned artifact citations, captured project revisions, and naming history. Text citations open bounded captured excerpts. Media citations route to the Artifact surface and open the exact cited original snapshot/hash, explicitly labeled as original media with no OCR/vision description or live-source check. Selecting a captured revision opens the exact retained Project Graph view used by that checkpoint; it does not substitute today's graph. Older records and truncated text are disclosed instead of being presented as complete history.

The desktop **Projects** search also indexes those checkpoint revisions. Paste a full or partial captured Git SHA, a branch name, a graph snapshot ID, or an artifact snapshot ID to recover the owning session across explicitly observed and currently bound projects. Opening a revision result enters that session first so its checkpoint context remains visible.

### Link an inspected session into notes

Use **To notes** in the desktop session inspector to review a title and create a user-owned Markdown handoff under `Agent Memory/Sessions`. The accessible action remains **Link session to notes** when its visible label collapses in a narrow header. The note carries portable project/session IDs, status and event count at export, timestamps, and the `ley/session` tag. Its body preserves the inspected goal, outcome, handoff, unresolved work, visible checkpoints, verification, and artifact trail while quoting stored agent text beneath an evidence-not-instructions warning.

Ley first canonically verifies that the open notes vault is the project’s bound Agent Memory vault. If a project from another vault was opened through the multi-project catalog, the write is refused until that vault is opened. Repeating the action opens the existing linked note by project/session ID even after a rename or move; it never overwrites an unrelated title. The note is a bounded export, discloses omitted or clipped context, and does not silently synchronize later session events. The immutable session remains authoritative. See [ADR 0021](../adr/0021-vault-verified-agent-memory-note-links.md) and [ADR 0022](../adr/0022-checkpoint-project-revision-citations.md).

## Recover a missed checkpoint after an interruption

Bounded prompt/response observations and supported host-tool observations are source evidence, not structured memory. If a host crashes or an agent fails to checkpoint before stopping, a later resume can report a **Recovery signal** for that same Ley session without injecting retained bodies into startup context. Memory Compiler exposes post-checkpoint Bash observations separately as supporting provenance; their `toe_` IDs are not current recovery anchors, and `returned` means only that the host emitted its normal post-tool event—not that a command or test succeeded. When a retained Bash command is complete, ADR 0067 also permits a read-only `automaticCommandCandidates` projection that references that exact `toe_` row. The candidate keeps `exitCode: null`, persists nothing, cannot bind or write, and proves no outcome or Verification state.

Use the read-only MCP tool `ley_session_memory_compile` to inspect candidate-bound prompt/response observations whose immutable event sequence is later than the latest checkpoint. `reviewable-evidence` means the bounded post-checkpoint turns are paired and retained; `partial-evidence` means a turn is missing, unpaired, or truncated; `metadata-only` means observations exist but capture policy/capacity retained no prompt/response bodies; `no-unconsolidated-evidence` means there is no candidate-bound turn evidence later to recover. Post-checkpoint Bash observations may still appear separately in `supportingToolEvidence` without changing that recovery state or `totalUnconsolidatedEvidence`. Each returned tool row discloses `automaticCommandCandidateEligibility`; only complete retained commands can produce a matching `automaticCommandCandidates` row. Result truncation alone does not block a command-only candidate because Ley makes no result claim. `omittedAutomaticCommandCandidateSources` distinguishes eligible sources omitted by `maxResults`, while `suppressedAutomaticCommandCandidateSources` discloses selected sources whose command could not be returned completely under the requested compiler character budget.

Do not convert a request into a claimed result. For example, a retained prompt with no paired response proves that the request was observed, not that the requested work happened. Reconstruct only facts the evidence supports and keep unknown work in `unresolved`. The compiler itself writes nothing and creates no trusted learning.

Before writing reconstructed structure, use the verifier matching the recovery shape with the same `sessionEventCount`. Exactly one generic unresolved/Decision/minimal-Problem claim goes through `ley_session_memory_verify`; exactly one Plan or Task goes through `ley_session_memory_verify_typed` so its exact status is bound into the candidate fingerprint and overlap comparison; one complete debugging episode with Problem expected behavior, ordered Attempts/outcomes/evidence, and optional Resolution goes through `ley_session_memory_verify_problem`, which binds evidence separately for the Problem and every nested child. If that rich Problem shares its recovery window with one or more minimal unresolved/Decision/Problem/Task/Plan siblings, `ley_session_memory_verify_composite` verifies the complete episode+sibling set under one explicit checkpoint summary. When two or more minimal candidates share a window with no rich Problem, `ley_session_memory_verify_batch` verifies the minimal set. Candidates must cite exact recovery `recordId` values; every current post-checkpoint record must be cited or explicitly deferred. `review-required` means only that structural accounting and deterministic duplicate/revision checks passed **and no recovery evidence remains deferred**. It does not prove semantic faithfulness, live-source correctness, or authorize a write. `needs-revision` or `stale` means do not write the candidate; `deferred` means at least one current record was intentionally left unconsolidated, so do not advance the checkpoint boundary yet. For a single unresolved recovery claim, use `ley_session_memory_commit_unresolved`; for a single minimal Decision or Problem use `ley_session_memory_commit_structured`; for a single Task use `ley_session_memory_commit_task`; for a single Plan use `ley_session_memory_commit_plan`; for one rich Problem episode with no siblings use `ley_session_memory_commit_problem`; for one rich Problem plus minimal siblings use `ley_session_memory_commit_composite`; for a verified multi-claim minimal-only set use `ley_session_memory_commit_batch`. Each route re-verifies the current window under the writer lock, requires the exact matching verifier fingerprint and evidence IDs, derives the checkpoint inside Ley, and persists provenance in the immutable event. The batch route appends one schema-v11 checkpoint with a complete evidence union plus per-child subsets; the rich Problem route appends one schema-v12 checkpoint with exact evidence subsets for the Problem, each Attempt, and optional Resolution; the composite route appends one schema-v13 checkpoint with the complete union plus exact rich-component and sibling child subsets. Exact retries replay without reopening the closed recovery window. Ley does not invent a batch/composite checkpoint summary, Plan status/text, Task status/details, Problem expected behavior, Attempt outcomes/evidence, or Resolution semantics. Standalone attachment of an Attempt/Resolution to an existing historical Problem plus Command/Verification/Summary recovery remain review-only. Generic checkpoints are not a substitute for these bound routes, and one composite window must not be split into sequential writes.

For supported single-claim recovery, pass the compiler pack's `sessionEventCount` and matching verifier `candidateFingerprint` to the bound route: unresolved → `ley_session_memory_commit_unresolved`; minimal Decision/Problem → `ley_session_memory_commit_structured`; Task → `ley_session_memory_commit_task`; Plan → `ley_session_memory_commit_plan`; rich Problem with no siblings → `ley_session_memory_commit_problem`, with Plan/Task first checked through `ley_session_memory_verify_typed` and rich Problem checked through `ley_session_memory_verify_problem`. For a rich Problem plus one or more minimal siblings from the same window, carry the exact `ley_session_memory_verify_composite` fingerprint, checkpoint summary, rich Problem, and complete sibling set to `ley_session_memory_commit_composite`. For two or more minimal-only claims, use the exact `ley_session_memory_verify_batch` fingerprint, checkpoint summary, and complete candidate set with `ley_session_memory_commit_batch`. Do not advance a shared recovery boundary with sequential writes. Ley re-verifies the candidate/batch/composite and complete evidence window under the session writer lock before appending. If any newer prompt, response, supported tool observation, rename, checkpoint, or finish event arrived after inspection, the bound write fails and the caller must recompile and reverify. `toe_` tool observations are not accepted as current verifier evidence IDs. `ley_session_checkpoint` retains its optional `expectedEventCount` guard for ordinary deliberate checkpoints, but it is not candidate-bound recovery.

## Review native sessions at lifecycle boundaries for reusable guidance

Use the local Consolidation Inbox only when deliberately reviewing retained evidence after a meaningful pause/completion/abandonment boundary:

```bash
ley consolidation inbox /path/to/project --json
```

The inbox is an on-demand read-only view over native paused, completed, and abandoned sessions. It excludes active sessions and explicitly imported historical sessions, returns stable session/turn IDs plus bounded counts rather than prompt/response bodies, and does not invoke a model, start background work, persist state, reopen or mutate cited sessions, or claim semantic faithfulness. Re-running the unchanged view may surface the same evidence again because this slice has no durable "processed" marker.

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

Imported sessions do not become automatic Resume context and are not flagged by Memory Health
as a missed-checkpoint backlog. Use explicit session list/show/turn inspection, project-memory
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
