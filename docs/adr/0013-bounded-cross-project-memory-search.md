# ADR 0013: Bounded cross-project memory search

Status: accepted

## Context

Ley's private observation catalog makes explicitly opened projects discoverable on one device, but filtering that catalog by project name is not memory search. Users need to recover a decision, failed attempt, lesson, session, file, or symbol even when they no longer remember which project contains it.

Letting one MCP process enumerate projects would weaken the fixed-project protocol boundary. Scanning the device for repositories would violate explicit-root consent. Returning every event or source match would create an unbounded prompt-injection and resource-exhaustion surface.

## Decision

The desktop app exposes a separate local cross-project search command. It reads only the bounded private observation catalog. Every observed root is independently:

1. canonicalized and diagnosed;
2. checked against the catalog's stable project ID;
3. resolved through its own private vault binding; and
4. searched only through existing captured-memory projections.

Unavailable, unbound, identity-changed, or invalid projects are skipped and counted. They do not cause another project's binding or memory to be reused.

Search covers structured session names and goals, decisions, problems and outcomes, learning summaries, captured artifacts, symbols, and dependencies. Existing bounded projections remain the source of truth. Results carry project identity, local navigation metadata, optional session or learning identity, and immutable artifact citations where available.

The response is globally capped at 30 results by default and 50 by hard limit. Queries are capped at 256 visible characters. Per-project artifact and activity projections retain their own limits; the response reports truncation whenever a source or the global result set is incomplete. The native command runs on a blocking worker rather than freezing the webview.

This capability is desktop-only. It is not added to the fixed-project MCP server, and it does not create an agent-accessible project-enumeration tool. Result text is labeled `untrusted-local-memory`, live source is not claimed to have been checked, and the interface repeats the instruction boundary.

## Consequences

- Users can recover knowledge without first remembering its project.
- Project-card filtering remains a distinct lightweight navigation control.
- Search never expands the set of observed roots or refreshes capture implicitly.
- Missing projects and moved vaults degrade honestly instead of aborting all useful results.
- Results can be stale relative to live source and must preserve their captured-snapshot language and citations.
