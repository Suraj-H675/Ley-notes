# ADR 0071: Derived Specification Verification methods

## Context

LEY.md defines the intended requirement-verification chain as:

`Requirement -> Acceptance criteria -> Verification method -> Observed verification result -> Evidence`.

ADR 0068 made Acceptance criteria machine-legible without creating completion state. ADR 0070 added
an exact read-only review between one current criterion and one historical Verification record while
deliberately leaving semantic coverage and criterion satisfaction unproven.

The remaining human-intent gap is Verification method structure. Keeping method text only inside the
whole Specification makes it harder for an agent to distinguish an explicit user-authored
verification instruction from nearby implementation prose. Inferring a method from historical
commands/tests would invert authority: observed agent/project history is evidence, not human intent.

## Decision

Ley derives a read-only `verificationMethods` projection from an exact current approved
Specification Markdown revision. The approved Markdown remains the sole authoritative content body;
the projection is rebuilt on every read/compile and is never persisted independently.

This slice does **not** derive a relation between a method and any Acceptance criterion or historical
Verification result. It only makes explicitly authored method rows addressable.

### Parser profile

The parser intentionally mirrors ADR 0068's narrow source-safe list profile.

A matching section must use a container-free ATX heading whose normalized heading text is exactly
`Verification method` or `Verification methods`, ASCII-case-insensitively.

- ATX levels 1 through 6 are recognized.
- The heading marker must begin in column 1.
- YAML frontmatter, fenced code, blockquotes, indented/list-contained headings, and Setext headings
  do not establish a section.
- An unclosed leading frontmatter delimiter fails closed to no projection.
- Backtick and tilde fenced blocks are excluded.
- More than one matching singular/plural heading makes the projection `ambiguous`; Ley returns no
  method rows rather than choosing one.

Within one accepted section:

- collection ends at the next equal/higher ATX heading;
- the first child subsection ends direct-method collection for the rest of the section;
- methods must be direct column-1 unordered or ordered list items;
- continuation/nested lines remain inside their parent method's exact source slice;
- nested list entries never become independent methods; and
- top-level fences/blockquotes and unindented prose after a blank separator terminate the current
  method using the same rules as ADR 0068.

Ley does not execute commands, render Markdown, fetch links, infer a test framework, interpret
checkboxes, or turn prose into a method semantically.

### Projection shape

Each method row contains only:

- deterministic `methodId`;
- exact raw Markdown source slice as `text`;
- 1-based inclusive `startLine`; and
- 1-based inclusive `endLine`.

The method ID uses prefix `vmd_` and a domain-separated SHA-256 over the Specification ID, exact
approved content hash, matching heading line, method line range, occurrence ordinal, and exact raw
method text. Any approved-revision change therefore changes method IDs.

The parent projection reports:

- `state`;
- total/returned/omitted method counts;
- matching heading line when unambiguous;
- `sourceRevisionBound: true`;
- `criterionBindingProven: false`;
- `observedResultBindingProven: false`;
- `statusInterpreted: false`;
- `persisted: false`;
- `authority: human-intent`; and
- `sourceBoundary: derived-from-approved-specification`.

Supported states are `available`, `empty`, `absent`, `ambiguous`, `omitted-limit`, and
`omitted-budget`. More than 64 direct methods is `omitted-limit` atomically.

## Relationship and outcome boundary

A Verification method is desired human-authored process, not proof that it was executed.

Ley does not infer:

- which `acr_` criterion a `vmd_` method verifies;
- which historical `ver_` record, command, or artifact resulted from the method;
- whether the method was run correctly;
- whether an observed result is still current;
- whether the criterion is satisfied; or
- whether the implementation currently meets the Specification.

List order, lexical similarity, matching command text, recency, and a historical `passed` status
are insufficient to create those links.

ADR 0072 later adds an optional exact `vmd_` to the existing read-only acceptance-criterion
Verification review. That extension keeps the relation caller-supplied and non-persistent; it does
not change this ADR's rule against automatic criterion/result inference.

## Budget behavior

Verification-method text duplicates source already contained in the whole Specification, so it may
consume only final spare output budget.

Compatibility ordering is strict:

1. whole Specification and all pre-existing context/reference/diagnostic admission;
2. existing ADR 0068 Acceptance-criteria projection; then
3. Verification-method projection.

Therefore this ADR cannot evict a parent Specification, existing memory/reference result,
diagnostic, Policy Bundle result, or an Acceptance-criteria row that previously fit.

- direct Specification context spends remaining character budget on Acceptance criteria first,
  Verification methods second;
- normal Context Compiler and Policy Bundle rows spend remaining token budget in that same order;
- standalone/combined Bootstrap context preserves existing whole-Specification and reference
  precedence, then Acceptance criteria, then Verification methods.

At the MCP 256 KiB transport boundary, Ley first downgrades available `verificationMethods`
projections to `omitted-budget` and retries serialization. Only if the result is still too large
does the pre-existing Acceptance-criteria fallback run. The existing retryable oversized-result
error remains the final fallback.

## Authority, egress, and privacy

Projection occurs only after the same source controls as the parent Specification:

1. project/source authority resolution;
2. applicable agent-egress authorization;
3. no-follow source read; and
4. exact approved content-hash verification.

Blocked, missing, changed, revoked, or otherwise unavailable Specification revisions expose no
method text, IDs, ranges, or parser state. The projection creates no filesystem/network/tool/review/
write/egress permission and is never copied into session memory, learnings, caches, approval
registries, Policy Bundle authority, or Bootstrap authority.

## Surface parity

The shared projection is returned on:

- direct `SpecificationContextItem`;
- normal `CompiledSpecificationItem`;
- `CompiledPolicyBundleItem`; and
- `BootstrapCompiledSpecification`.

Automatic lifecycle-hook rendering remains unchanged; hooks still render bounded whole
Specifications rather than duplicating optional structured rows.

## Consequences

- explicit human-authored Verification methods become stable, cited machine-readable handles;
- exact approved Markdown remains the sole authority;
- the method layer cannot be confused with observed execution/result state;
- automatic or persistent criterion/result linkage remains a separate evidence-bound problem;
- Acceptance-criteria budget behavior is preserved; and
- source egress restrictions automatically protect derived method rows.

## Verification

The slice must prove:

- exact raw slices/ranges and deterministic revision-bound `vmd_` IDs;
- singular/plural heading support with ambiguity fail-closed;
- frontmatter/fence/blockquote/indented/Setext false-positive resistance;
- >64 rows fail closed atomically;
- direct/compiler/bootstrap budgets preserve whole-source and Acceptance-criteria priority;
- Policy Bundle parity uses the same parser and egress boundary;
- MCP byte pressure drops methods before criteria;
- stale/missing/blocked revisions expose no method projection;
- the normal Specification-authority and Bootstrap evals require exact non-binding method rows; and
- the existing Specification egress canary protects a private marker inside a Verification method.
