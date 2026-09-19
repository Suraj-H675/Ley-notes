# ADR 0052: Explicit reusable team and organization knowledge scopes

Status: Accepted

## Context

Ley's P0 Context Mounts deliberately authorize one active project to reuse one other captured project as lower-precedence read-only reference context. That is the right primitive for an isolated reference, but it becomes repetitive when the same bounded set of project knowledge should be reused across several active projects for a team or organization.

The reusable form must not become ambient whole-vault search, cloud collaboration, shared write authority, or a hidden project-discovery shortcut. It also must not weaken Ley's existing agent-egress rule: local retention of a source project is not permission to send that source or derivatives to a configured model target.

## Decision

Ley adds an owner-private reusable **Knowledge Scope** authority with two descriptive kinds: `team` and `organization`.

The first slice is deliberately project-reference-only and read-only:

- a scope has one stable `ksc_` ID, a kind, a human-readable name, and an immutable set of stable source project IDs;
- source membership is bounded to 16 projects, the registry is bounded to 32 scopes, and one active project may attach at most 8 scopes;
- creating the same kind/name/source set is idempotent; changing membership means creating a new immutable scope rather than silently mutating the old one;
- `ley scope create/list/attach/attached/detach` is the local user-controlled authority surface;
- source projects must already be explicitly initialized and persistently bound;
- the registry stores no project or vault paths. Source locations are resolved through Ley's existing private Project Catalog and Binding Registry and revalidated before use;
- scope authority is never inferred from observed projects and MCP exposes no create/list/attach/detach mutation surface.

Attachments are explicit per active project. Detaching removes current reference authority, but Ley retains bounded stable scope-ID/source-project-ID ancestry for that active project. The retained relation contains no source text or machine path and exists so later source-project egress restrictions can still constrain historical derivatives that may have been influenced while the scope was attached.

## Context Compiler semantics

Attached scope sources reuse the existing captured-memory search and admission machinery. Ley does not create a second retrieval/trust system.

Compiler precedence is:

1. current approved Specifications as human intent;
2. active-project context;
3. explicit one-project Context Mount references;
4. attached team/organization shared references.

The existing `referencePrecedence: active-project-over-mounted-reference` contract remains unchanged. The reusable layer adds `sharedKnowledgePrecedence: explicit-mount-over-shared-knowledge`.

Shared scope output is explicit:

- `sharedKnowledgeScopes`;
- `sharedKnowledgeReferences`;
- `sharedKnowledgeExclusions`;
- `sharedKnowledgeCoverage`.

Returned references carry the stable scope ID and source project ID/name, `authority: shared-knowledge-reference`, and `sourceBoundary: untrusted-shared-project-memory`. They grant no write authority to the source project and never redirect active-project session or learning writes.

If the same source is also an explicit Context Mount, the explicit mount wins and the lower-precedence shared source is not searched a second time. Overlapping attached scopes similarly deduplicate a source project. Scope/reference metadata and diagnostics consume the same bounded compiler budget.

Context Pack Inspector schema v2 includes shared-scope IDs, source IDs, precedence, coverage, exclusions, and included-record metadata while continuing to omit the supplied shared reference bodies.

ADR 0053 later extends the current Inspector contract to schema v3 for Policy Bundle attribution; the schema-v2 statement above records this ADR's original Knowledge Scope slice.

## Egress and non-laundering

Each shared source inherits its source project's agent-egress ceiling. Ley checks that project policy **before** searching the source project's captured memory. A blocked source contributes no text and cannot steer ranking/admission indirectly.

When an attached or historically attached scope source is blocked for the configured target, Ley applies the same conservative derivative rule already used for other finer-grained authority:

- unproven active-project session/decision/problem/learning derivatives are withheld from the Context Compiler;
- broad historical MCP readers fail closed;
- host SessionStart withholds historical startup context;
- reviewed runbook Skill export fails closed.

An explicit local target can still use a `local-model-only` source. Detaching the scope does not erase the retained ancestry, so detach cannot become an egress-laundering bypass.

## Persistence and safety

`knowledge-scopes-v1.json` uses the same owner-private locked atomic persistence pattern as Ley's other authority registries. It rejects symlink/non-regular/private-permission violations, unsupported/corrupt schema, invalid IDs, duplicate sources, self-source attachments, and configured bounds.

Missing projects, changed project identity, and unavailable vault bindings remain visible as typed unavailable source states rather than silently granting or dropping authority.

## Deliberately deferred

This ADR deliberately did **not** implement the second half of the roadmap item: reusable team/organization **policy bundles**. That follow-up is now implemented separately by [ADR 0053](0053-explicit-reusable-team-organization-policy-bundles.md), preserving this ADR's original scope and rationale. This ADR still does not add:

- shared writable sessions/learnings or cross-project mutation;
- remote accounts, synchronization, membership/role management, or collaboration servers;
- automatic project enumeration or auto-attachment;
- shared Specifications/notes/folders/Canvas bundles beyond captured project-reference scopes;
- a dedicated desktop scope manager.

Policy Bundles therefore compose this explicit scope model rather than turning retrieved scope content into policy. Remote collaboration, shared writes, automatic project enumeration, and dedicated desktop scope management remain outside this ADR.

## Evaluation

P2 coverage includes the deterministic `team-organization-knowledge-scope` scenario through the real CLI and stdio MCP surfaces. Passing requires:

- no shared context before explicit attachment;
- bounded multi-source shared retrieval after attachment;
- no unrelated observed-project access or machine-path leakage;
- stable read-only scope/source provenance and Inspector attribution;
- source-project `local-model-only` enforcement before cloud search while local-target access still works;
- detach removing current shared context;
- retained ancestry withholding historical derivatives and broad historical readers for a disallowed target;
- zero privacy-canary violations.

Focused core, CLI, host, runbook, MCP, locking, permission, corruption, and availability regressions cover the same authority boundary independently.
