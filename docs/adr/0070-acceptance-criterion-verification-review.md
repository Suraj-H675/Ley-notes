# ADR 0070: Acceptance-criterion Verification review

## Context

ADR 0068 derives stable, revision-bound acceptance-criterion IDs from exact current approved
Specification Markdown. It intentionally stops before the later North Star chain:

`Acceptance criterion -> Verification method -> Observed verification result -> Evidence`.

Ley already has durable historical `VerificationRecord` entries inside immutable session
checkpoints. Those records preserve an observed status such as `passed`, `failed`, `skipped`, or
`unknown`, an optional command, a summary, and bounded captured evidence citations.

The existence of both structures is not enough to infer a relationship between them. A historical
test may be irrelevant to a criterion, may cover only part of it, may have run against an older live
working tree, or may have passed before later changes. Treating a matching-looking or merely
`passed` Verification as criterion completion would launder historical evidence into human-intent
state.

## Decision

Ley adds a read-only review operation:

- core: `review_acceptance_criterion_verification`;
- MCP: `ley_acceptance_criterion_verification_review`.

The caller must supply four exact stable handles:

- one approved Specification ID;
- one revision-bound `acr_` acceptance-criterion ID;
- one fixed-project session ID; and
- one durable `ver_` Verification-record ID from that session.

Ley then verifies only mechanically provable facts:

1. the Specification is still approved at the exact current pinned revision;
2. the supplied `acr_` ID is present in the acceptance-criteria projection rebuilt from that exact
   revision;
3. the supplied session belongs to the same fixed project; and
4. the supplied `ver_` record exists exactly once in that immutable session history.

The relationship itself is explicitly caller-supplied. Ley does not search for a Verification by
text similarity, command, status, recency, or artifact overlap.

## Review result

The result returns the exact criterion, exact historical Verification record, containing checkpoint
identity/time, and a deterministic link fingerprint. It also states:

- `specificationSourceRevisionChecked: true`;
- `verificationRecordChecked: true`;
- `relationshipSuppliedByCaller: true`;
- `verificationStatusInterpretedAsSatisfaction: false`;
- `criterionSatisfactionProven: false`;
- `semanticCoverageProven: false`;
- `currentImplementationProven: false`;
- `persisted: false`;
- `automaticWriteAllowed: false`; and
- `liveSourceChecked: false`.

A `passed` historical Verification therefore remains an observed historical status. It is not
rewritten as `satisfied`, `complete`, `verified criterion`, or equivalent human-intent state.

## Link identity

The `ley-acceptance-criterion-verification-review-v1` fingerprint binds:

- project ID;
- Specification ID;
- exact approved Specification content hash;
- exact `acr_` criterion ID;
- session ID;
- containing checkpoint ID; and
- exact `ver_` Verification-record ID.

Changing/reapproving the Specification naturally invalidates the old criterion relationship because
the old `acr_` no longer belongs to the current approved revision. The review never mutates the
historical Verification record.

## Egress and authority

The MCP route uses Ley's conservative historical-memory egress gate. If the project or any relevant
finer-grained historical source cannot be safely supplied to the configured agent target, the review
is withheld before the historical Verification body is returned.

The criterion remains `human-intent`; the Verification remains
`untrusted-historical-verification`; and the relationship is labeled
`caller-supplied-criterion-verification-review-link`. None of those boundaries grants filesystem,
network, tool, review, write, trust, or egress authority.

## Persistence boundary

This ADR adds no session event, checkpoint field, registry entry, completion state, or background
maintenance. It does not alter schema-v1/v4/v6 checkpoint meaning and does not create a durable
criterion-to-Verification edge.

ADR 0072 later extends this same read-only review with an optional exact current `vmd_`
Verification-method handle. It preserves this ADR's no-persistence/no-satisfaction boundary and does
not infer the method relationship automatically. A future persistent Verification-method/result model
would still need a separate design for authoring authority, current-vs-historical applicability,
semantic coverage, invalidation after implementation changes, and whether/how a human may mark a
criterion satisfied. This review primitive is deliberately not that model.

## Consequences

- agents can inspect one exact criterion and one exact historical Verification together without
  relying on brittle text matching;
- changed Specification revisions fail closed before the old criterion can be reviewed as current;
- a passing historical test cannot silently become acceptance state;
- no new durable graph or session schema is introduced; and
- the primitive creates a provenance-safe basis for later explicit Verification-method/result work.

## Verification

The slice must prove:

- the exact current approved revision and exact `acr_` are revalidated on every review;
- the exact session/`ver_` record is required and cross-project substitution fails;
- the link fingerprint is deterministic and revision-bound;
- a `passed` Verification still reports satisfaction/semantic/current-implementation proof as
  false;
- changed Specification revisions and unknown criterion/Verification handles fail closed;
- restricted Specification/historical egress returns no Verification summary;
- the MCP route is read-only, idempotent, closed-world, and available through the official protocol;
- no durable session/Specification state changes during review; and
- the existing Specification-authority evaluation exercises the review path instead of introducing
  a weaker standalone metric.
