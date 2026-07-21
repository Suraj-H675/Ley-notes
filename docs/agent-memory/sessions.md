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

The document must match [checkpoint-input.schema.json](../../schemas/agent-memory/checkpoint-input.schema.json). Include a stable `requestId` when a hook may retry delivery:

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

Artifact paths must exist in the current captured snapshot. Ley stores a snapshot-pinned citation instead of the unverified path alone.

Ley also derives a project revision for every new checkpoint. It pins the exact immutable graph and artifact snapshots used while accepting that checkpoint, plus the Git HEAD, branch, and captured tracked-change count when the approved capture came from a Git repository. Agents do not submit this metadata, and Ley does not read live Git during checkpoint recording. A checkpoint made after the source changes but before the next ingestion therefore continues to cite the earlier approved capture. Replaying the same request after a later ingestion returns the original event and revision.

## Inspect captured sessions

List sessions for a project:

```bash
ley session list /path/to/project
```

Read one reconstructed session:

```bash
ley session show ses_01234567890123456789012345678901 \
  /path/to/project --json
```

The vault also contains `session.md` for review and `session-v1.json` for local tools. Both files are derived. Preserve the immutable event files when repairing or migrating memory.

Ley Desktop exposes the same event history in **Agent Memory → Sessions**. Opening a session uses the bounded shared context projection rather than trusting a mutable Markdown summary. It shows recent checkpoints, decisions, tasks, problem attempts and outcomes, structured resolution root causes and verification, commands, handoff, unresolved work, snapshot-pinned artifact citations, captured project revisions, and naming history. Selecting a captured revision opens the exact retained Project Graph view used by that checkpoint; it does not substitute today's graph. Older checkpoints, older naming revisions, and truncated text are disclosed instead of being presented as complete history.

The desktop **Projects** search also indexes those checkpoint revisions. Paste a full or partial captured Git SHA, a branch name, a graph snapshot ID, or an artifact snapshot ID to recover the owning session across explicitly observed and currently bound projects. Opening a revision result enters that session first so its checkpoint context remains visible.

### Link an inspected session into notes

Use **To notes** in the desktop session inspector to review a title and create a user-owned Markdown handoff under `Agent Memory/Sessions`. The accessible action remains **Link session to notes** when its visible label collapses in a narrow header. The note carries portable project/session IDs, status and event count at export, timestamps, and the `ley/session` tag. Its body preserves the inspected goal, outcome, handoff, unresolved work, visible checkpoints, verification, and artifact trail while quoting stored agent text beneath an evidence-not-instructions warning.

Ley first canonically verifies that the open notes vault is the project’s bound Agent Memory vault. If a project from another vault was opened through the multi-project catalog, the write is refused until that vault is opened. Repeating the action opens the existing linked note by project/session ID even after a rename or move; it never overwrites an unrelated title. The note is a bounded export, discloses omitted or clipped context, and does not silently synchronize later session events. The immutable session remains authoritative. See [ADR 0021](../adr/0021-vault-verified-agent-memory-note-links.md) and [ADR 0022](../adr/0022-checkpoint-project-revision-citations.md).

## Retry a write safely

Supply `--request-id req_01234567890123456789012345678901` when another process may repeat a start, compact checkpoint, or finish call. The same ID and content replay the original event. The same ID with different content fails instead of creating ambiguous history.

Generated request IDs are safe for interactive commands. Host adapters should persist their request ID until they receive a successful response.

## Understand the privacy boundary

Ley applies local credential-pattern redaction before it writes session text. It also bounds event size and collection counts, rejects symlinks and malformed history, and serializes concurrent writers.

Redaction cannot guarantee that arbitrary private text is safe. Record summaries and bounded evidence instead of full prompts, tool output, environment dumps, or transcripts. Ley does not send session data anywhere, but a cloud agent can receive context that you intentionally retrieve through that agent.
