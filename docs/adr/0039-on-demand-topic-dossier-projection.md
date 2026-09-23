# ADR 0039: On-demand Topic Dossier projection

## Status

Accepted.

## Context

Ley's first P1 roadmap item is Topic Dossiers: compact topic-oriented views for areas that are revisited across sessions, such as authentication, deployment, billing, or editor synchronization. The North Star requires dossiers to remain derived intelligence views rather than authority: they must be evidence-linked, source-bound, rebuildable, bounded, and disposable.

Persisting an automatically maintained topic document immediately would add a new invalidation, egress-inheritance, and deletion/purge tier before its retrieval value is proven. It would also risk turning a fluent summary into a competing truth store. The existing fixed-project memory search, structured sessions, captured artifact citations, learning trust state, and Git revision applicability already provide enough deterministic structure for a useful first dossier slice.

## Decision

Ley implements Topic Dossiers first as an **on-demand, non-persistent projection** exposed by `compile_topic_dossier` and the read-only MCP tool `ley_topic_dossier`.

The projection:

- searches only the fixed active project's already-captured evidence and structured memory;
- preserves stable evidence IDs, captured artifact citations, learning kind/trust state, conflicts, and revision applicability;
- expands only the small set of supporting structured sessions nominated by topic-relevant evidence in order to recover open tasks/problems/unresolved work and recent verification records;
- uses the same deterministic `unr_...` child identity as Session Context for unresolved open items, so dossier `itemId` values remain stable citation/follow-up handles rather than display-only synthetic IDs;
- deduplicates important artifacts by captured snapshot/path/content hash;
- categorizes trusted-current procedure/pitfall learnings without promoting unreviewed or stale learning into current knowledge;
- treats session and checkpoint-revision rows as secondary evidence because supporting-session metadata and top-level revision freshness already represent those dimensions;
- admits conflicts, recent verification, open work, important artifacts, and supporting-session references before those redundant secondary rows when the strict token budget is tight;
- reports truthful returned/omitted counts and truncation rather than silently pretending the dossier is complete;
- carries `liveSourceChecked: false`; live Git may contribute only the same lightweight metadata freshness signal already used elsewhere;
- computes a SHA-256 source fingerprint over the selected evidence/content/trust/revision state and dossier structure while excluding transient retrieval-ranking scores, so an unchanged source state does not receive a different fingerprint merely because time-based ranking contributions moved;
- never writes a dossier file/cache in this slice.

The MCP dossier reader uses the same historical-memory egress gate as project resume, memory/activity search, session/turn inspection, and learning readers. A project-level denial or finer-grained historical egress ceiling therefore cannot be bypassed through a dossier.

## Authority and trust

A dossier is a **derived navigation/progressive-disclosure view**, not a new authority type. It does not outrank Specifications, reviewed learnings, live source, or the Context Compiler's task-specific admission logic. Stored text remains untrusted evidence and never grants capabilities, write authority, review authority, or policy.

For a concrete current task, agents should still prefer `ley_compile_context`. A dossier is appropriate when the user/agent needs a compact map of a repeatedly revisited topic before drilling into exact evidence.

## Budget behavior

The serialized dossier response itself is kept within the requested 800–8000 token budget using the same conservative four-characters-per-token approximation used by existing bounded projections. Coverage fields disclose omitted evidence, artifacts, supporting sessions, open items, verification records, and conflicts.

Schema v2 also carries the bounded source-search boundary into dossier coverage:

- `sourceSearchCandidateLimit`;
- `sourceSearchCollectedCandidates`;
- `sourceSearchOmittedCandidates`;
- `sourceSearchSourceTruncated`;
- the existing `sourceSearchResults` / `sourceSearchOmittedResults`.

This separates two different loss points. `sourceSearchOmittedCandidates` means potentially relevant
memory never reached ranking/result fitting because the lower-level search candidate cap was reached;
`sourceSearchOmittedResults` means bounded candidates survived collection but did not fit the search
result response. The dossier's top-level `truncated` remains the aggregate warning, but callers no
longer need to infer which source-search stage lost evidence.

Schema v3 preserves the captured media boundary on `importantArtifacts`. Each candidate is enriched
only from the already-loaded captured artifact manifest when its `artifactSnapshotId`, project-relative
path, and content hash all match the immutable captured record. Supported image artifacts then expose
optional `mediaType` (`png`, `jpeg`, or `webp`) even when the artifact was nominated directly by source
search rather than through a supporting-session verification row. Ley does not infer media from an
extension, read the live file, generate a description, or increase the artifact's authority.

When budget pressure exists, dossier-specific state is more valuable than duplicate navigation metadata. Session and revision search rows are therefore secondary to conflicts, verification, open state, artifact references, and supporting-session identity. They may still be returned when budget remains.

## Deletion and rebuildability

Because dossiers are rebuilt on demand and are not persisted, session/project erasure does not require a new dossier purge transaction in this slice. The end-to-end deletion-fidelity scenario explicitly rebuilds `ley_topic_dossier` after erasure and verifies that erased canaries remain unrecoverable. Any future persisted/background dossier cache must add dependency-aware invalidation and erasure before it is acceptable.

## Consequences

Benefits:

- proves topic-oriented retrieval value without creating another durable truth store;
- reuses P0 trust, provenance, revision, privacy, and egress semantics rather than duplicating them;
- makes open work and verification visible alongside architecture/decision/history evidence;
- supports progressive disclosure through stable IDs/citations;
- keeps deletion semantics simple and testable.

Tradeoffs:

- dossier generation is query-time work rather than a background-maintained cache;
- there is no automatic topic discovery/consolidation/refresh scheduler yet;
- verification/open-state relevance is inherited from the supporting session/checkpoint selected by topic evidence rather than independently semantically ranked;
- this slice does not claim a continuously maintained canonical topic document.

Those are deliberate P1 boundaries. Persistent/background maintenance should only follow after downstream usefulness, maintenance cost, invalidation, erasure, and egress propagation are demonstrated by evaluation.
