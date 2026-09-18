# ADR 0041: Context Pack Inspector manifest over the actual compiler path

## Status

Accepted.

## Context

Ley's third P1 roadmap item is a Context Pack Inspector. The Context Compiler already exposes most of the raw diagnostics needed to understand a pack—authority, admission basis, exclusions, premise warnings, conflicts, retrieval mode, revision freshness, budgets, omissions, mounts, egress exclusions, and follow-up handles—but there is no stable identity for one compiled pack and no compact manifest that answers “why was this supplied?” without copying all supplied text again.

Persisting every full context pack would create another sensitive storage tier, with new deletion, retention, egress, and invalidation requirements. Conversely, independently re-running a different retrieval algorithm in an Inspector would make attribution unreliable: the diagnostic surface could disagree with what the agent actually received.

## Decision

Ley adds a logical `contextPackId` and `createdAtUnixMs` to every `CompiledContextPack`, then derives a non-persistent Inspector manifest from the **same finalized pack structure**.

`contextPackId` is `cpk_` plus a SHA-256 digest of the logical compiled pack after Specifications, active-project admission, mounted references, premise/revision diagnostics, and agent egress filtering have been finalized. The hash input clears the pack ID itself and excludes creation time, so recompiling unchanged logical context can reproduce the same ID. If included records, context text, authority/policy diagnostics, mounts, source snapshots, or other logical pack content changes, the ID changes.

The Inspector:

- uses the same agent-aware Context Compiler path, same task, same result/token limits, same Specification registry, same Context Mount registry, same egress registry, and same configured target;
- returns a compact manifest rather than another copy of context bodies/excerpts;
- lists included Specification, active-project, and mounted-reference records with stable IDs, project-relative citations/paths, authority, admission basis, inclusion reason, revision applicability, and estimated token contribution;
- exposes Specification, active-project, mounted-reference, and egress exclusions;
- preserves premise adjudication, conflicts, gaps, retrieval mode/fallback metadata, revision freshness, coverage, mount scopes, and follow-up handles;
- reports budget composition across Specifications, active-project items, mounted references, and diagnostic/structural overhead;
- keeps `liveSourceChecked: false` unless the compiler itself can truthfully claim otherwise;
- returns `persisted: false` and stores no Inspector manifest or copied pack body in this slice.

The MCP tool `ley_context_pack_inspect` accepts the same task/result/token parameters as `ley_compile_context` plus an optional `expectedContextPackId`. If the current recompilation matches that ID, the manifest can be attributed to the same logical pack content. If it does not match, Ley returns `matchesExpectedContextPack: false` and an explicit warning that the manifest describes the current recompilation only; it does not pretend to reconstruct the older supplied pack.

## Timestamp semantics

`ley_compile_context` returns `createdAtUnixMs` for the pack actually compiled in that call. Because the first Inspector slice deliberately persists no historical pack manifest, `ley_context_pack_inspect` cannot recover the original timestamp of a prior call. Its manifest therefore reports `recompiledAtUnixMs`, the time of the current recompilation.

This distinction is intentional. Logical pack identity is reproducible; historical invocation time is not retained.

## Privacy and egress

The Inspector runs the same agent-aware compiler path rather than a broad historical reader. Egress policy is therefore applied before the manifest is constructed. Blocked source text cannot be recovered through inspection; the manifest may disclose bounded policy/exclusion metadata that the compiler already exposes.

The manifest omits included Specification source text, active/mounted excerpts, session bodies, and learning guidance bodies. It may expose stable IDs, project-relative paths/citations, hashes, policy scope IDs, and diagnostic metadata needed for attribution. Absolute project/vault paths are not returned.

`contextPackId` is computed only after egress-filtered logical pack content is finalized for the caller. It must not be used as a capability, permission, authority signal, or proof of live-source freshness.

## Consequences

Benefits:

- gives each supplied logical pack a stable debuggable identity;
- attributes inclusion/exclusion to the same compiler semantics the agent actually used;
- avoids storing another copy of sensitive context text;
- makes budget, authority, retrieval, premise, revision, mount, and egress behavior inspectable;
- fails honestly when an older pack can no longer be reproduced.

Tradeoffs:

- the first slice cannot reconstruct an older mismatched pack after source/policy/retrieval state changes;
- original historical invocation timestamps are not retained by the Inspector;
- a full “pack history” UI would require an explicitly designed persisted manifest tier with retention, deletion, egress inheritance, and storage bounds;
- Inspector output diagnoses retrieval/context construction but cannot by itself prove whether a downstream model used the context correctly.
