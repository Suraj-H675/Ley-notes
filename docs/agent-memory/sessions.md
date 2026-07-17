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

## Retry a write safely

Supply `--request-id req_01234567890123456789012345678901` when another process may repeat a start, compact checkpoint, or finish call. The same ID and content replay the original event. The same ID with different content fails instead of creating ambiguous history.

Generated request IDs are safe for interactive commands. Host adapters should persist their request ID until they receive a successful response.

## Understand the privacy boundary

Ley applies local credential-pattern redaction before it writes session text. It also bounds event size and collection counts, rejects symlinks and malformed history, and serializes concurrent writers.

Redaction cannot guarantee that arbitrary private text is safe. Record summaries and bounded evidence instead of full prompts, tool output, environment dumps, or transcripts. Ley does not send session data anywhere, but a cloud agent can receive context that you intentionally retrieve through that agent.
