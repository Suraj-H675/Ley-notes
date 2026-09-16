# ADR 0032: Task-conditioned Specification admission

- Status: Accepted
- Date: 2026-09-16

## Context

ADR 0031 established exact-revision user-approved Specifications as a separate human-intent authority boundary. Agents could inspect that boundary with `ley_project_specifications`, but normal task context still required the host to manually merge Specifications with historical memory. That left authority ordering as a host convention rather than a Context Compiler invariant.

Ley needs one normal task-conditioned operation that can combine human intent and project history without flattening them into one relevance score. A highly similar old decision must not outrank a current user-approved requirement merely because retrieval ranked it first.

## Decision

`ley_compile_context` resolves exact current approved Specifications before historical memory. Specification relevance uses the same deterministic lexical scoring shape as fixed-project memory search, but filters common grammatical and generic coding-task words before nomination. Exact task phrases still count. Changed and missing approved revisions never contribute requirement text; because Ley no longer retains their approved body, it does not claim that those unavailable revisions are relevant to the current task. Current exact approvals with no meaningful task match are reported as low-relevance rather than loaded merely because budget is available.

The compiler holds the Specification registry's authority lock across task scanning and context assembly. Approval, reapproval, and revocation therefore serialize against an in-flight compile; a revoke that completes first cannot be followed by a later compile returning the revoked text.

Relevant Specifications are returned whole in a distinct `specifications` section with `authority: human-intent`, exact approval hash/path/ID, and relevance signals. They consume the same result/token budget before historical memory. The pack reports `authorityPrecedence: human-intent-over-historical-memory` plus separate Specification coverage and exclusions.

The compiler still treats direct captured source evidence differently from historical guidance. A Specification expresses desired behavior; direct evidence may legitimately show that the implementation does not satisfy it. Therefore Specification intent never suppresses captured artifact/symbol/dependency evidence.

Historical sessions/decisions/problems and trusted learnings are checked against admitted Specification text using a deliberately conservative deterministic conflict rule. Ley only auto-withholds memory when a high-overlap clause differs by explicit negation. The exclusion records the exact Specification IDs and emits a human-intent conflict gap. This is a safety floor, not a general natural-language contradiction classifier.

The compiler retains `evidenceState` for project evidence. A pack may therefore contain useful human intent while reporting `no-useful-evidence` for historical memory. That state must not be interpreted as “ignore the admitted Specification.” `liveSourceChecked: false` still refers to the project working tree; exact Specification revision checking is separate.

## Consequences

- Hosts no longer need to orchestrate a separate Specification call before normal task compilation.
- `ley_project_specifications` remains available for explicit bounded inspection.
- Similarity cannot promote historical memory above human intent.
- Tight budgets prefer whole relevant Specifications before memory rather than truncating requirements.
- Changed/missing approved notes are visible authority gaps and require explicit user reapproval. Their relevance to the current task is unknown until a current approved revision exists.
- Direct evidence remains visible when it disagrees with intent so agents can see implementation drift.
- The compiler reads only the fixed project's bound Specification registry and vault; it cannot approve/revoke authority or enumerate other projects.

## Deliberately deferred

This slice does not add semantic Specification embeddings, model-based contradiction judging, section-level proprietary requirement schemas, empty-workspace/reference mounts, branch-lineage adjudication, or per-source egress policy. Those need their own evaluation and authority design rather than being hidden inside the first admission rule.