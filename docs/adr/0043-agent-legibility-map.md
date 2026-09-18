# ADR 0043: Source-bound Agent Legibility Map

## Status

Accepted.

## Context

Ley's fifth P1 roadmap item is Agent Legibility: a compact table of contents that helps an agent understand how to navigate and operate a project without manufacturing a second copy of the repository or a new authority layer.

The requested orientation spans several source classes with different semantics:

- captured repository artifacts and graph nodes are immutable captured evidence;
- recent checkpoint commands and plan items are historical structured memory, not canonical project policy or live issue-tracker truth;
- approved Specifications carry `human-intent` authority, but their bodies may be sensitive;
- Minimal capture can retain artifact metadata while intentionally omitting source bodies.

A single readiness or legibility score would collapse those distinctions and imply evidence Ley does not possess. Likewise, copying document bodies into a project map would create a stale second repository/manual and increase privacy exposure.

## Decision

Ley implements Agent Legibility as an **on-demand, non-persistent, source-bound table of contents** exposed by `compile_agent_legibility_map` and the read-only MCP tool `ley_agent_legibility`.

The first slice projects:

- architecture/design references selected by conservative captured-path patterns;
- captured top-level directory counts with optional conventional navigation-role labels;
- declared root `package.json` scripts when captured source text is retained;
- observed historical checkpoint and verification commands;
- project-policy references;
- schema/migration references;
- graph-derived primary API candidates;
- observability references;
- non-completed current plan items from bounded active/paused latest checkpoints;
- current approved Specification registry metadata only.

Every category exposes its `selectionBasis` or equivalent source provenance. Path/name classification is a navigation heuristic, never semantic authority.

## No score and no invented commands

The projection exposes `tableOfContentsNotScore: true` and has no `score` field. Missing categories become explicit `gaps`; they are not converted into a project-deficiency score.

Declared commands are emitted only from a retained captured root `package.json`. If Minimal capture omits that source text, Ley reports `declared-commands-not-detected` with the missing-source-text reason. It does not invent `npm test`, build commands, package-manager conventions, or framework defaults.

Observed checkpoint/verification commands remain historical evidence and are labeled separately from declared commands. An observed command is not automatically canonical project policy.

## Authority, privacy, and source boundaries

Agent Legibility is navigation, not authority. It does not replace the Context Compiler, approved Specifications, or live-source inspection. `liveSourceChecked` remains false.

The map returns only project-relative paths and bounded stable IDs/hashes/metadata. It does not copy source excerpts, Specification bodies, or prompt/response bodies. Specification entries are registry metadata only.

The MCP route applies the existing historical-memory egress gate before map construction. If a fine-grained restriction means historical derivatives cannot be proven independent, the projection fails closed rather than exposing a partial route around that policy.

## Bounds and identity

Callers can bound entries per section, inspected sessions, and copied command/plan text. Coverage reports candidate/returned/omitted counts truthfully; when the text budget is exhausted, later text-bearing entries are omitted rather than returned with empty placeholders.

The map carries a deterministic SHA-256 `mapFingerprint` over logical project/snapshot identity, revision freshness, returned sections, gaps, and coverage. Generation time is excluded, so unchanged logical state has the same fingerprint while changed captured source changes the fingerprint.

## Consequences

Benefits:

- gives agents a compact project-orientation table of contents without duplicating repository bodies;
- preserves declared-vs-observed command provenance;
- makes missing discovery explicit instead of filling gaps with guesses;
- reuses existing capture, graph, Specification, revision, privacy, and egress semantics;
- remains rebuildable after source/session erasure because no legibility cache is persisted.

Tradeoffs:

- conservative path/name heuristics can miss unconventional repository layouts;
- the first declared-command slice parses only the captured root `package.json`;
- a primary API candidate is not proof of public/runtime exposure;
- current plan items are structured session state, not issue-tracker truth;
- broader build-system parsing or a dedicated UI may be added later only with equally explicit source/provenance semantics.
