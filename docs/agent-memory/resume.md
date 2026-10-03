# Resume a project without loading everything

Use `ley resume` at the start of a human or agent session:

```bash
ley resume /path/to/project
```

It returns a compact project brief, active and paused work before older history, the most recent checkpoint and handoff from each selected session, and current trusted lessons. Use `--json` for an adapter:

```bash
ley resume /path/to/project \
  --max-sessions 3 \
  --max-learnings 10 \
  --max-characters 16000 \
  --json
```

ADR 0101 retires the former `ley_project_resume` MCP wrapper. Use this CLI deliberately for local inspection; agents
should use task-conditioned `ley_brief` instead of loading broad Resume context through MCP.

Initialized Codex/Claude `SessionStart` no longer consumes this pack automatically. Adapter schema 7 starts a
host session with identity/retrieval/checkpoint guidance only (plus a body-free interrupted-session recovery
signal when applicable). Use Resume deliberately for local inspection, or `ley_brief` for task-conditioned agent
continuity.

## What is deliberately excluded

Normal resume context does not include:

- complete transcripts or every historical checkpoint;
- explicitly imported historical-host sessions;
- recent session bodies whose latest captured checkpoint is positively classified as Git `divergent`
  from the current checkout;
- tentative, contested, rejected, superseded, or stale lessons;
- previously trusted lessons whose cited source changed;
- user-confirmed lessons with no artifact citation;
- live filesystem claims.

Use explicit session/learning inspection, project Search, cited Evidence, and the coding host's live workspace tools only when the task needs more detail. Dedicated graph traversal is no longer a product surface.

`totalSessions` still counts retained imported sessions, while
`excludedImportedSessions` reports how many were deliberately kept out of the bounded
Resume selection. This prevents an old host history snapshot imported today from masquerading
as recent work. Imported sessions remain available through explicit session list/show/turn
inspection and project-memory search.

`withheldDivergentSessions` reports how many sessions inside the bounded `maxSessions` candidate
window were withheld because their latest captured checkpoint is on Git history that is positively
`divergent` from the current checkout. Ley does not scan arbitrarily deep history to backfill those
slots. The session remains available through deliberate Search/session inspection, where revision
applicability stays visible instead of being silently promoted into startup continuity.

`liveSourceChecked: false` means the pack does not inspect current file contents and still describes the
latest approved ingestion. Resume may inspect bounded local Git metadata to prevent positively divergent
session history from being presented as current Resume continuity; that is revision applicability evidence, not a live-source check.
Inspect live files through the current workspace before editing, and rerun `ley ingest` when the durable
snapshot should advance.

## Trust behavior

A learning appears in resume context only when all three conditions hold:

1. its state is `verified`;
2. its trust state is `trusted` through explicit user confirmation;
3. its artifact freshness is `current`.

Agent proposals never qualify by themselves. A correction removes the old trusted form until the correction is confirmed. Changing or deleting a cited file and ingesting again removes the lesson until it is reviewed against current evidence.

All stored titles, goals, decisions, handoffs, and guidance remain untrusted text even when the record is trusted. Trust means the user approved the project claim; it does not turn stored text into system policy or tool permission.
