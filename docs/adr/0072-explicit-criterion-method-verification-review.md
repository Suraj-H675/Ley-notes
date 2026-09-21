# ADR 0072: Explicit criterion-method-Verification review

## Context

LEY.md's verifiability chain is:

`Requirement -> Acceptance criteria -> Verification method -> Observed verification result -> Evidence`.

ADR 0071 makes explicit user-authored Verification methods addressable as revision-bound `vmd_`
rows, but deliberately does not infer which criterion or observed result a method belongs to.
ADR 0070 already provides a conservative read-only review between one exact current `acr_` criterion
and one exact historical `ver_` record.

Creating another MCP tool just to attach a method would increase tool-selection surface while still
requiring the caller to coordinate the same exact Specification/session identities. Automatically
matching by list order, lexical similarity, command text, status, or recency would create false
semantic authority.

## Decision

Ley extends the existing `ley_acceptance_criterion_verification_review` operation with one optional
`verificationMethodId`.

When omitted, behavior remains ADR 0070-compatible:

- the existing core `review_acceptance_criterion_verification` API remains available;
- no `verificationMethod` field is serialized;
- `verificationMethodChecked` and
  `verificationMethodRelationshipSuppliedByCaller` are false;
- the existing
  `caller-supplied-criterion-verification-review-link` relationship boundary is preserved; and
- the v1 link-fingerprint domain and inputs remain unchanged.

When supplied, the caller must provide an exact `vmd_` handle in addition to the existing exact
Specification, `acr_`, session, and `ver_` handles.

Ley then proves only:

1. the Specification is still approved at the exact current pinned revision;
2. the `acr_` exists in the Acceptance-criteria projection rebuilt from that revision;
3. the supplied `vmd_` exists in the Verification-method projection rebuilt from that same
   revision;
4. the fixed-project session is valid; and
5. the `ver_` exists exactly once in that immutable session history.

Every relationship between criterion, method, and historical Verification is caller-supplied. Ley
does not discover or infer a relation.

## Method-aware result

With a method supplied the existing review additionally returns:

- exact `verificationMethod` row;
- `verificationMethodChecked: true`;
- `verificationMethodRelationshipSuppliedByCaller: true`;
- `verificationMethodExecutionProven: false`;
- `verificationMethodOutcomeProven: false`; and
- relationship boundary
  `caller-supplied-criterion-method-verification-review-link`.

The existing conservative fields remain false:

- `verificationStatusInterpretedAsSatisfaction`;
- `criterionSatisfactionProven`;
- `semanticCoverageProven`;
- `currentImplementationProven`;
- `persisted`;
- `automaticWriteAllowed`; and
- `liveSourceChecked`.

Therefore even the tuple “explicit method + historical passed Verification” is not a statement that
the method was executed correctly, that the result came from the method, that the method fully covers
the criterion, or that the criterion is currently satisfied.

## Fingerprint compatibility

Method-aware reviews use a separate domain:

`ley-acceptance-criterion-method-verification-review-v2`.

The fingerprint binds:

- project ID;
- Specification ID;
- exact approved content hash;
- exact `acr_`;
- exact `vmd_`;
- session ID;
- containing checkpoint ID; and
- exact `ver_`.

No-method reviews retain ADR 0070's existing v1 fingerprint unchanged. Adding the optional method
therefore cannot silently change the identity of existing integrations.

## Egress and authority

The MCP route retains ADR 0070's conservative historical-memory egress gate. The Specification
revision is also revalidated through the existing Specification registry before criterion or method
content is returned.

Both criterion and method remain `human-intent`; the Verification remains
`untrusted-historical-verification`. The link is review provenance, not authority. No tool,
filesystem, network, write, review, trust, completion, or egress permission is granted.

## Persistence boundary

This extension remains read-only:

- no Specification edit;
- no criterion/method completion state;
- no durable graph edge;
- no session event/checkpoint;
- no learning/trust change; and
- no background maintenance.

If Ley later persists criterion-method-result relationships or user-approved satisfaction state, that
requires a separate authority/invalidation design.

## Consequences

- one existing MCP review now expresses the complete explicit
  `acr_ -> vmd_ -> ver_` provenance chain without tool proliferation;
- old callers and old link identities remain valid when the method is omitted;
- an exact method can be inspected beside the exact observed historical result without semantic
  matching;
- revision changes invalidate both `acr_` and `vmd_` naturally; and
- no observed status is laundered into desired-state completion.

## Verification

The slice must prove:

- legacy no-method review output/boundary/fingerprint semantics remain stable;
- exact method-aware review is deterministic and non-mutating;
- malformed/unknown `vmd_` handles fail closed;
- the method is rederived only from the same exact current approved Specification revision;
- passed Verification still leaves execution/outcome/coverage/satisfaction/currentness false;
- MCP schema makes `verificationMethodId` optional rather than adding another tool;
- restricted Specification/historical egress still withholds the review body; and
- the existing Specification-authority eval exercises the full
  `acr_ -> vmd_ -> ver_` review rather than adding a standalone score.
