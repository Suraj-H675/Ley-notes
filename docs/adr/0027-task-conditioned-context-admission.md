# ADR 0027: Task-conditioned context admission

Status: accepted

## Context

Ley already has bounded fixed-project retrieval across captured artifacts, graph facts,
sessions, decisions, problems, and reviewed learnings. Hybrid retrieval can nominate
conceptually related records, but nomination is not permission to inject a record into an
agent prompt. Relative semantic rank also cannot distinguish a useful top result from the
least-bad item in an unrelated candidate set.

The North Star requires a Context Compiler that separates candidate retrieval from memory
admission, preserves authority distinctions, abstains when useful evidence is absent, and
explains why relevant-looking records were excluded.

## Decision

Ley adds `compile_project_context` and the read-only MCP tool `ley_compile_context` as the
first Context Compiler slice. It remains fixed to the project and vault selected when the MCP
process starts. It does not discover projects, refresh ingestion, read live source, install a
model, alter memory, mount reference scopes, or infer new authority.
Candidate generation deliberately reuses `search_project_memory` with its existing bounded
lexical, semantic, structured-memory, trust, freshness, and conflict projections. The compiler
then applies a separate admission gate.

The initial relevance rule is intentionally inspectable:

- any retained lexical match is eligible for further admission;
- a semantic-only candidate requires cosine similarity of at least `0.30` from the pinned local
  retrieval model;
- relative semantic rank alone is never sufficient.

The local semantic rank result therefore exposes its cosine similarity as an additional ranking
signal. This is diagnostic/admission data; it does not become evidence and does not change the
existing reciprocal-rank ordering.

The `0.30` threshold is a conservative first policy, not a universal truth. It was selected after
local probes with the pinned model separated representative direct/related coding-memory text
from weak and unrelated text. It must remain easy to evaluate and replace if realistic Ley
evaluations show a better policy.

Admission keeps authority and trust separate from relevance. Direct captured artifact/symbol/
dependency evidence is labelled `direct-evidence`; user-reviewed current learnings are labelled
`trusted-reviewed-knowledge`; sessions, revisions, decisions, and problems are labelled
`historical-project-memory` and are never promoted to current truth merely because they match.

A learning that is unverified, contested, superseded, rejected, or stale is excluded from normal
compiled context even when highly relevant. Durable content disagreements are also withheld from
automatic injection and surfaced as conflicts. The low-level search marks each affected result as
`contentConflicted` before conflict descriptions are output-fitted, so diagnostic truncation cannot
accidentally re-admit a known conflicting record. The existing lower-level tools remain available for
deliberate inspection of excluded history.

Compilation can return `good-evidence`, `partial-evidence`, `conflicting-evidence`,
`stale-evidence`, or `no-useful-evidence`. Zero admitted items is valid. The compiler must not fill
a budget with low-relevance memory simply because capacity remains.

The pack includes admitted items, separate ranking signals, authority, admission basis, conflicts,
bounded exclusions with reasons, gaps, retrieval/fallback metadata, and stable follow-up handles.
`liveSourceChecked` remains false. A captured snapshot is not represented as the current working
tree.

The initial compiler budget is a strict context-material estimate: 500–8000 tokens, default 1500,
with at most 20 admitted items and a default of 8. It accounts admitted titles/excerpts plus returned
diagnostic text and identifiers; it is not claimed to equal an exact provider-tokenizer count for the
serialized JSON schema. The compiler reserves diagnostic capacity before admitting excerpts, then fits
conflicts, gaps, exclusions, and follow-up handles into the remaining estimate. Coverage reports
returned and omitted diagnostic counts plus whether the underlying search was itself truncated.
Candidate generation remains independently bounded by the existing memory-search limits, and the MCP
256 KB serialized hard limit remains a final backstop.

## Consequences

- Hosts gain one high-level task-specific read operation without losing inspectable low-level tools.
- Semantic similarity can nominate memory but cannot override learning trust state or a disclosed
  durable conflict.
- Missing/corrupt semantic retrieval degrades safely to lexical admission with an explicit gap.
- Historical sessions can orient an agent while remaining visibly weaker authority than direct
  captured evidence or reviewed current knowledge.
- The first slice does not yet implement specification authority, reference mounts, egress policy,
  branch-lineage adjudication, or a live freshness beacon; later P0 work must extend the compiler
  rather than pretending those signals already exist.
- The admission threshold and evidence-state policy require realistic downstream and adversarial
  evaluation against raw search/resume baselines before further generalization.
