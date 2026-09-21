# ADR 0068: Derived acceptance criteria from approved Specifications

- Status: Accepted
- Date: 2026-09-21
- Extends: ADR 0031, ADR 0032, ADR 0058
- Supersedes: ADR 0031 only for its deferral of structured acceptance-criteria projection

## Context

Ley already treats an exact user-approved Markdown Specification revision as human-intent authority. The private `specifications-v1.json` registry is the approval/hash authority pin; the Markdown note remains the sole authoritative content body. ADR 0032 admits whole current approved Specifications into task context without flattening them into historical memory.

The remaining P0 gap is machine-legible acceptance criteria. Keeping criteria only as unstructured prose makes it harder for downstream agents and evaluators to distinguish an explicit criterion from neighboring requirement prose. Copying criteria into a private store or interpreting task-list checkboxes as completion would create a second authority/state channel and violate Ley's portable-knowledge model.

## Decision

Ley derives a read-only `acceptanceCriteria` projection from an exact current approved Specification revision. The projection is rebuilt from the approved Markdown source on every read/compile and is never persisted independently.

The parent Specification remains the authority. A criterion cannot outlive, override, broaden, or survive staleness/revocation of its parent revision.

### Parser profile

This first slice intentionally implements a narrow published Markdown profile rather than claiming full CommonMark parsing.

A matching section must use a container-free ATX heading whose normalized heading text is exactly `Acceptance criteria`, ASCII-case-insensitively.

- ATX levels 1 through 6 are recognized.
- The heading marker must begin in column 1. Indented ATX-looking text is treated as container content and cannot establish the section.
- A closing hash run is ignored only when it is separated from heading text by whitespace.
- Setext headings are not recognized in this slice.
- Headings inside YAML frontmatter, fenced code, blockquotes, or other containers do not count.
- Frontmatter is recognized only when the first line is `---` and is closed by `---` or `...`; an unclosed opening frontmatter delimiter fails closed to no projection.
- Backtick and tilde fenced blocks with at least three fence characters are excluded.
- Multiple matching Acceptance criteria headings make the projection `ambiguous`; Ley returns no criterion rows rather than guessing which section is authoritative.

Within the one accepted section:

- the section ends at the next ATX heading of equal or higher level;
- the first lower-level child heading ends direct-criterion collection for the rest of that parent section;
- a direct criterion must begin in column 1;
- unordered markers `-`, `*`, and `+` are supported when followed by whitespace;
- ordered markers use one to nine ASCII digits followed by `.` or `)` and whitespace;
- indented/nested list entries are never separate criteria;
- continuation/nested lines may remain inside the parent criterion's exact source slice;
- a top-level fenced block or blockquote terminates the current item rather than becoming criterion content;
- after a blank separator, a nonblank line indented fewer than four spaces terminates the current item.

Ley does not render Markdown, fetch links, execute embedded content, parse HTML semantics, or interpret task-list markers.

### Projection shape

Each returned criterion contains only:

- deterministic `criterionId`;
- exact raw Markdown source slice as `text`;
- 1-based inclusive `startLine`;
- 1-based inclusive `endLine`.

The criterion ID is revision-bound and content-addressed using a domain-separated SHA-256 over the Specification ID, exact approved content hash, matching heading line, criterion line range, occurrence ordinal, and exact raw criterion text. Duplicate identical criterion text therefore remains distinguishable by occurrence/range, and any approved-revision change necessarily changes IDs.

The parent `acceptanceCriteria` projection also reports:

- `state`;
- total/returned/omitted criterion counts;
- matching heading line when unambiguous;
- `sourceRevisionBound: true`;
- `statusInterpreted: false`;
- `persisted: false`;
- `authority: human-intent`;
- `sourceBoundary: derived-from-approved-specification`.

Supported states are:

- `available` — one unambiguous section with one or more returned criteria;
- `empty` — one unambiguous section with no direct criteria;
- `absent` — no valid matching section;
- `ambiguous` — multiple valid matching sections;
- `omitted-limit` — the section contains more than 64 direct criteria, so the projection is omitted atomically;
- `omitted-budget` — the complete projection would exceed spare output budget, so criterion rows are omitted atomically.

An empty criterion array is therefore never the only signal for why no criterion text was returned.

## Status and verification boundary

Task-list syntax such as `- [x]` or `- [ ]` is preserved literally inside `text`. Ley does not expose `checked`, `completed`, `verified`, `satisfied`, `remaining`, or equivalent fields from this projection.

This ADR implements only `Requirement -> Acceptance criteria` structure. It does not claim the later links in the North Star chain:

`Acceptance criteria -> Verification method -> Observed verification result -> Evidence`.

Those require separately evidence-bound designs. A criterion row is desired human intent, not proof of implementation or verification.

## Budget behavior

Whole-Specification admission semantics remain unchanged.

Acceptance-criteria text duplicates a slice already present in the returned whole Specification, so projection text may consume only spare output budget after existing authoritative/evidence output has been selected.

- `ley_project_specifications` first selects complete Specifications under its existing result/character budget, then spends only remaining character budget on criteria projections.
- normal Context Compiler output first performs all existing Specification/Policy Bundle/memory admission, diagnostics, mounted-reference admission, shared-knowledge admission, and egress accounting; only the final remaining token budget may be spent on active-project and Policy Bundle criteria projections.
- standalone bootstrap Specification compilation first performs existing whole-Specification selection, then spends only remaining token budget on criteria projections. Combined bootstrap context additionally admits Bootstrap References before criteria projections are fitted.

If the complete projection cannot fit, the parent Specification remains returned exactly as before and the projection becomes `omitted-budget`. Criteria never evict an already-admitted Specification, memory item, reference, or diagnostic.

The 64-criterion ceiling is atomic: Ley does not return a misleading partial list.

MCP keeps its existing 256 KiB serialized-result guard as a final transport boundary. In this ADR's original criteria-only surface, an oversized structured result atomically downgraded `available` acceptance-criteria projections to `omitted-budget` before the existing bounded retryable error. ADR 0071 extends that transport order without weakening this guarantee: optional `verificationMethods` are now dropped first, and the acceptance-criteria downgrade remains the next fallback while preserving the parent Specifications. This transport fallback does not alter logical Specification authority or create a second context-pack identity.

## Authority, egress, and privacy

Projection occurs only after the existing authority and source controls succeed:

1. fixed project/source binding resolution;
2. applicable agent-egress authorization;
3. stable no-follow source read;
4. exact approved content-hash verification.

Therefore blocked, missing, changed, revoked, or otherwise unavailable Specification revisions expose no criterion text, IDs, ranges, or parser state through agent-facing outputs.

No criterion is written to `.ley`, the approval registry, session memory, learnings, caches, Policy Bundle authority, or bootstrap authority. No new filesystem, network, tool, review, write, or egress permission is granted.

## Surface parity

The same shared parser/projection is used for:

- direct `SpecificationContextItem`;
- normal `CompiledSpecificationItem`;
- `BootstrapCompiledSpecification`;
- `CompiledPolicyBundleItem`.

This prevents divergent acceptance-criteria semantics across active-project, bootstrap, and policy-bundle context.

Automatic Codex/Claude hook rendering remains unchanged in this slice. Hooks continue to receive their existing bounded whole-Specification rendering; structured criteria are available through MCP/compiler outputs only until a separate whole-criterion rendering/budget design is evaluated.

## Consequences

- acceptance criteria stay ordinary readable Markdown;
- exact approved Markdown remains the sole authoritative content body;
- agents receive deterministic criterion boundaries without semantic extraction;
- checkbox/task syntax is never laundered into completion state;
- changed/reapproved revisions naturally replace criterion IDs;
- optional structure cannot reduce previously available whole-Specification context;
- parser limitations are explicit rather than hidden behind a claim of complete Markdown support.

## Verification

The slice must prove:

- exact raw Markdown slices and 1-based ranges, including CRLF preservation;
- deterministic IDs for the same approved revision and changed IDs across revisions;
- unordered, ordered, task-list, multiline, and nested-content behavior;
- exclusion of frontmatter, fenced-code, blockquote, list-container/indented, setext, malformed-heading, and child-subsection false positives;
- multiple matching sections fail closed as `ambiguous`;
- more than 64 criteria fail closed atomically as `omitted-limit`;
- direct character, final compiler/bootstrap token pressure, and MCP serialized-byte pressure produce `omitted-budget` without evicting the parent Specification or previously admitted reference context;
- normal Specification relevance/conflict semantics remain unchanged;
- Policy Bundle and bootstrap projections use the same parser;
- source-level egress blocks both whole source and derived criteria;
- stale/missing/revoked revisions expose no criteria;
- MCP output preserves the projection contract; ADR 0071 now inserts optional Verification-method omission before this criteria fallback, while the existing oversized-result error remains last;
- the existing P0 Specification authority/egress scenarios and bootstrap evaluation cover the projection rather than adding a weaker standalone metric.
