# ADR 0076: Current Project State Specification authority handles

- Status: Accepted
- Date: 2026-09-22

## Context

ADR 0040 introduced Current Project State as a non-persistent P1 projection over captured project
memory, structured sessions, reviewed learnings, revision freshness, and verification. That first slice
intentionally avoided turning recent historical decisions into current truth.

However, the projection omitted the Authority Plane's highest-precedence active-project human intent:
current user-approved Specifications. An agent asking "what matters now?" could therefore see reviewed
learned guidance and historical work while missing the exact approved requirement revision that should
outrank them.

Copying Specification bodies into Current Project State would create a second human-intent text copy,
consume the historical state text budget, and weaken progressive disclosure. Large but valid approved
Specifications could also disappear if the state reused the body-returning Specification context
budget.

## Decision

Current Project State schema v2 adds compact, metadata-only Specification authority.

The projection may receive an explicit `SpecificationAuthorityList` that is already scoped to the
same fixed project. The generic core `current_project_state(...)` entry point remains valid and performs
no implicit global Specification-registry read; the agent-facing MCP route explicitly lists authority
from its configured fixed-project registry and passes that list to
`current_project_state_with_specification_authority(...)`.

For each exact current approved revision, `authoritativeSpecifications` exposes only:

- stable Specification ID;
- vault-relative path;
- exact approved content hash;
- approval timestamp;
- `state: current`;
- `exactApprovedRevisionAvailable: true`;
- `authority: human-intent`;
- `sourceBoundary: user-approved-specification`;
- `sourceIncluded: false`; and
- `followupTool: ley_project_specifications`.

The approved Markdown body, requirement prose, Acceptance criteria text, and Verification method text
are not copied into Current Project State. Agents follow the existing Specification reader when the
authoritative body is needed.

Changed or missing approvals move to `specificationAttention`. Those rows keep stable identity,
approved hash, optional current hash, approval timestamp, typed `changed`/`missing` state,
`exactApprovedRevisionAvailable: false`, `sourceIncluded: false`, and the same follow-up tool.
They are not presented as active human intent.

The projection discloses `specificationAuthorityPrecedence:
human-intent-over-historical-memory`.

## Bounds and fingerprint

Current Project State returns at most 12 current Specification handles and 12 Specification-attention
rows. Coverage records approved/current/changed/missing totals plus returned/omitted counts. Omitted
Specification metadata marks the projection truncated.

These metadata rows consume no `maxCharacters` text budget because they copy no Specification body.
They do participate in the logical `stateFingerprint`, so approval/revision state changes invalidate
the prior state projection even when captured project/session memory is unchanged.

## Authority and privacy boundary

- Specification metadata does not become project evidence or live-source proof.
- Changed/missing approvals do not remain active human intent.
- Current Project State does not interpret Acceptance criteria completion or Verification outcomes.
- A Specification handle grants no filesystem, tool, network, review, or write permission.
- The existing conservative historical-memory MCP egress gate remains in force. A disallowed
  fine-grained Specification source blocks the broad project-state read rather than exposing it through
  metadata.
- The projection remains `persisted: false` and `liveSourceChecked: false`.

## Evaluation

Focused core coverage approves a Specification containing a private body canary and requires:

- one current metadata-only human-intent handle;
- no canary in serialized Current Project State;
- a stable follow-up handle to `ley_project_specifications`; and
- after mutating the ordinary vault note, removal from `authoritativeSpecifications`, one typed
  `changed` attention row, and a different state fingerprint.

The MCP regression uses the same exact approved Specification: cloud access returns the body-free
handle while allowed, setting that exact source to `local-model-only` makes cloud
`ley_project_state` fail closed, and the local target sees the handle again without the body.

The existing real-binary `current-project-state-storage` P1 scenario is strengthened with the same
current→changed revision transition and requires zero Specification-body/path privacy leakage.
