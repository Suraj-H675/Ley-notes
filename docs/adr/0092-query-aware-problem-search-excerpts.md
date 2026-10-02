# ADR 0092: Query-aware Problem excerpts in canonical Search

Status: accepted

## Context

Canonical `ley_search` ranks structured Problem records using the complete bounded debugging episode: Problem
title/symptom/expected state, ordered attempt action/evidence, and resolution root-cause/change/verification
fields. Session name and checkpoint summary also contribute surrounding search context.

The returned Problem preview did not reflect that ranking surface. Resolved Problems always returned only
`resolution.change`; unresolved Problems returned only `symptom`. A query could therefore match an exact failed
attempt or root cause strongly enough to rank the Problem while the returned excerpt omitted the matched fact.
With granular session inspection outside the canonical four-tool workflow, that made canonical Search needlessly
opaque for one of Ley's core continuity jobs: avoiding known failed approaches and recalling why a fix worked.

A focused regression reproduced the gap: Search returned the correct Problem for a query matching one failed
attempt plus the root cause, but the excerpt contained neither matching marker.

## Decision

Keep `ProjectMemorySearchResult` unchanged and make only structured Problem excerpts query-aware:

1. Build labeled excerpt parts directly from the stored Problem record: root cause, attempts with typed outcome and
   evidence, change, verification, symptom, and expected state.
2. Score each Problem part with the same deterministic lexical signal already used by Search. Exact and term-
   matching parts sort ahead of fallback episode facts.
3. For resolved Problems, fallback order is root cause → ordered attempts → change → verification → symptom →
   expected state. For unresolved Problems, preserve the historical symptom-first fallback, followed by attempts
   and expected state.
4. Bound each selected Problem field independently and center oversized fields around the matching query/term so a
   late match in an up-to-8,000-character field is not lost merely because it occurs after the prefix.
5. Keep the existing 720-character Search-result excerpt ceiling and normal response token fitting. If any Problem
   field is shortened, retain `truncated: true` even when the final concatenated excerpt itself is under 720
   characters.

No model summarizes or rewrites the episode. The excerpt contains only stored text plus deterministic labels and
truncation markers.

## Trust and provenance

This is a visibility change, not an authority change:

- Problem results remain `trustedForReuse: false`.
- Existing revision applicability, conflict disclosure, project selection, egress policy, citations, captured
  freshness, `liveSourceChecked: false`, and the `untrusted-project-memory` boundary are unchanged.
- A checkpoint-level citation remains checkpoint provenance; it is not upgraded into proof that an individual
  displayed attempt/root-cause sentence is verified by that artifact.

## Bounds and limitations

Problem fields can be much larger than a Search response. Per-part match-centered windows prevent one large field
from monopolizing the preview and reduce transient allocation before the existing result fitter runs.

Search ranking may still receive surrounding session/checkpoint text that is not itself rendered as a Problem
excerpt part. If a Problem ranks only because of that surrounding context, Search returns the deterministic
fallback episode preview rather than claiming that one Problem field matched. Do not add another response field or
broaden the canonical surface without evidence that this remaining distinction causes material downstream errors.

## Evidence

- Core regression: a resolved Problem with query markers buried deep inside oversized failed-attempt evidence and
  root cause must expose both markers plus the typed `no-effect` outcome under a 500-token Search budget, stay at
  or below the existing 720-character excerpt bound, and report truncation truthfully.
- Canonical MCP regression: `ley_search` must expose matching root-cause and failed-attempt facts at 500 tokens
  while leaving `trustedForReuse` false.
- The existing project-memory Search suite continues to cover deterministic ranking, revision filtering,
  conflicts, canonical native continuity, citations, and token fitting.
