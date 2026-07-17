# ADR 0011: Bounded trusted-first project resume context

- Status: Accepted
- Date: 2026-07-18

## Context

Making every session and lesson queryable still leaves a new agent to decide what to load. Loading the entire project history wastes tokens, increases prompt-injection exposure, and makes old or unreviewed claims compete with current work. Returning only the newest completed session can also hide an older active or paused handoff.

Ley needs one predictable startup pack that helps an agent continue without claiming perfect memory or current-source knowledge.

## Decision

Ley exposes the same bounded resume projection through:

```bash
ley resume /path/to/project
```

and the default read-only MCP tool:

```text
ley_project_resume
```

The projection contains:

- fixed project and captured artifact/graph snapshot identity;
- active sessions first, then paused sessions, then completed and abandoned sessions by recency;
- each selected session’s goal, latest checkpoint summary, decisions, active/blocked tasks, unresolved problems/items, result, handoff, and remaining work;
- only lessons that are `verified`, `trusted`, and `current` against artifact citations from the latest approved ingestion.

Defaults are three sessions, ten lessons, and 16,000 text characters. Callers may request at most ten sessions, twenty lessons, and 32,000 text characters. The response reports total/omitted counts, character/token estimates, and truncation.

The pack never includes tentative, contested, rejected, superseded, stale, source-changed, or uncited lessons. These remain available through deliberate review/all queries, but cannot enter normal startup context. A correction immediately removes a previously trusted lesson until a user confirms the corrected claim. A later source hash change removes it again.

Every text field is labeled untrusted historical evidence. The pack states `liveSourceChecked: false`, carries stable session/record/learning IDs for deeper retrieval, omits absolute paths, and instructs the agent to inspect live files before changing them. It does not start a session or write memory.

## Consequences

- Agents have one low-friction, token-bounded continuity call.
- Active and paused work cannot be hidden by a newer completed session.
- Unreviewed or stale memory is structurally excluded from normal startup.
- The pack is a recall aid, not a guarantee against forgetting or hallucination.
- Deeper session, learning, graph, and artifact evidence remains on-demand.
- Project isolation, correction compliance, source staleness, and poisoning exclusion are covered by a two-project multi-session evaluation.
