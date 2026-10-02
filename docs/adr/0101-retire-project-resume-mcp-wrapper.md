# ADR 0101: Retire the Project Resume MCP wrapper

**Status:** Accepted — 2026-10-02

## Context

Ley still needs the bounded trusted-first Project Resume projection for deliberate local inspection. It prioritizes
active/paused work, then recent terminal sessions, admits only current explicitly trusted learnings, withholds
positively divergent session history, and reports bounded omissions rather than loading full project history.

The model-facing `ley_project_resume` wrapper no longer has a distinct product role:

- canonical native MCP uses task-conditioned `ley_brief` for agent continuity;
- initialized host startup no longer injects a Resume pack automatically;
- `ley resume PROJECT --json` calls the same transition-aware core projection as the old MCP wrapper; and
- broad no-task Resume is useful for a human/local adapter, but it is not necessary in the focused agent tool set.

Keeping the MCP wrapper therefore duplicates a local inspection surface and encourages agents to load broad recent
history before they have a concrete task.

## Decision

Retire only the `ley_project_resume` MCP wrapper:

- remove its MCP parameter schema, handler, instructions, inventory expectations, and wrapper-specific test;
- keep `ProjectResumePack`, its transition-aware core implementation, and `ley resume` CLI unchanged;
- migrate evaluation coverage that tests Resume semantics to the local CLI projection; and
- keep canonical `ley_brief` as the model-facing task-conditioned continuity entry point.

## Consequences

- External legacy MCP clients that call `ley_project_resume` must use `ley resume --json` for deliberate local
  inspection or `ley_brief` for an agent task.
- No session, learning, artifact, trust, Git-applicability, or migration state is removed.
- Resume's active/paused ordering, trusted-learning admission, divergent-session withholding, bounds, and privacy
  behavior continue to be tested through the CLI/core surface.
- Agents lose a broad startup-style context route and retain the smaller task-scoped Brief path.

## Non-goals

This ADR does not remove Project Resume itself, session detail readers, learning detail readers, Search, Brief, or any
legacy migration storage.
