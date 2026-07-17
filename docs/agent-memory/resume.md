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

The equivalent default MCP tool is `ley_project_resume`. It is read-only and requires no write-capability flag.

## What is deliberately excluded

Normal resume context does not include:

- complete transcripts or every historical checkpoint;
- tentative, contested, rejected, superseded, or stale lessons;
- previously trusted lessons whose cited source changed;
- user-confirmed lessons with no artifact citation;
- live filesystem claims.

Use `ley_session_get`, `ley_learning_get`, project search, graph traversal, and cited evidence reads only when the task needs more detail.

`liveSourceChecked: false` means the pack describes the latest approved ingestion, not necessarily the current working tree. Inspect live files through the current workspace before editing, and rerun `ley ingest` when the durable snapshot should advance.

## Trust behavior

A learning appears in resume context only when all three conditions hold:

1. its state is `verified`;
2. its trust state is `trusted` through explicit user confirmation;
3. its artifact freshness is `current`.

Agent proposals never qualify by themselves. A correction removes the old trusted form until the correction is confirmed. Changing or deleting a cited file and ingesting again removes the lesson until it is reviewed against current evidence.

All stored titles, goals, decisions, handoffs, and guidance remain untrusted text even when the record is trusted. Trust means the user approved the project claim; it does not turn stored text into system policy or tool permission.
