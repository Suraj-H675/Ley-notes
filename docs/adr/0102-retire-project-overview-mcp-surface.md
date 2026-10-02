# ADR 0102: Retire the Project Overview MCP surface

**Status:** Accepted — 2026-10-02

## Context

Ley's historical MCP server exposed the same captured-project Overview twice:

- the `ley_project_overview` tool; and
- the single `ley://project/<project-id>/overview` JSON resource.

The Overview projection remains useful locally. It reports capture mode, artifact/graph snapshot identity, retained
and skipped file counts, graph counts/diagnostics, captured Git metadata, revision freshness, and privacy metadata.
Desktop still uses the transition-aware Overview projection for project inspection.

The MCP exposures no longer fit the focused agent surface:

- canonical native MCP already exposes no resources and omits `ley_project_overview`;
- `ley_search` exposes project identity, artifact/graph snapshot IDs, capture time, revision freshness, privacy
  boundary, freshness, and `liveSourceChecked: false` alongside the evidence an agent actually requested;
- `ley_brief` is the task-conditioned authority-aware entry point; and
- the legacy MCP tool/resource call the legacy-vault-only `project_memory_overview(...)` path directly, while current
  Desktop/native continuity use transition-aware/native authority. Keeping them can therefore present older legacy
  snapshot diagnostics after native artifact authority has advanced.

The remaining unique fields—capture mode and file/graph diagnostic counts—are local inspection/debug information,
not evidence an agent needs injected through a broad model-facing route.

## Decision

Retire the complete model-facing Overview surface:

- remove the `ley_project_overview` MCP tool;
- stop advertising MCP resource capability;
- remove `ley://project/<project-id>/overview` listing/reading; and
- keep `MemoryOverview`, `project_memory_overview`, `project_memory_overview_with_continuity_transition`, all
  Desktop consumers, and local diagnostic behavior unchanged.

Current agents use `ley_brief` for task context and `ley_search` for bounded inspection/freshness. Overview remains a
local product projection, not an agent API.

## Consequences

- Legacy MCP clients using the tool or resource must migrate to canonical Brief/Search for agent work or local
  Desktop/core diagnostics for overview inspection.
- The normal Ley MCP server is tools-only in both canonical and legacy-compatibility modes.
- No captured artifacts, graph data, Git provenance, migration state, or Overview data model is deleted.
- Retiring the stale legacy-only MCP read removes a possible disagreement with newer native continuity state.

## Non-goals

This ADR does not remove project diagnosis, Desktop project overview, capture statistics, revision freshness,
`MemoryOverview`, or any artifact/graph migration machinery.
