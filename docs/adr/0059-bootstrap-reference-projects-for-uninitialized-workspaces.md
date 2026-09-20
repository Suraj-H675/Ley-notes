# ADR 0059: Bootstrap reference projects for uninitialized workspaces

- Status: Accepted
- Date: 2026-09-20
- Extends: ADR 0058

## Context

ADR 0058 lets an uninitialized workspace consume an explicitly attached, exact approved
Specification without creating writable Ley project identity. `LEY.md` §30.6 also requires the
same pre-project journey to support an explicitly selected **reference project** as read-only
knowledge.

Normal Context Mounts are intentionally keyed to an initialized active `prj_` identity and carry
per-mount egress semantics. Generalizing that registry to path-like uninitialized targets would
weaken a boundary that is already correct for normal project work. A second independent bootstrap
registry would create a cross-file retirement/rollback transaction during initialization.

## Decision

Ley extends the existing owner-private bootstrap authority registry instead.

The registry schema advances to v2 while retaining the existing
`bootstrap-specifications-v1.json` filename for compatibility. Legacy schema-v1 records remain
readable and decode with no reference grants; the document upgrades to v2 only on a subsequent
write.

An explicit local-user command creates reference authority:

```text
ley bootstrap-ref attach SOURCE_PROJECT [WORKSPACE]
ley bootstrap-ref list [WORKSPACE]
ley bootstrap-ref detach GRANT_ID [WORKSPACE]
```

MCP and host hooks expose no authority-mutation route.

### Authority identity

The target uses the same `bsw_` canonical-directory generation from ADR 0058. A Bootstrap Reference
grant uses a deterministic `brg_` ID derived from the target `bsw_` identity and stable source
project ID. The private record stores only:

- stable source project ID; and
- attachment time.

It stores no source path, vault path, captured body, search result, prompt, or target project ID.

Reference attachment requires the source to be an initialized, currently bound Ley project. It is
idempotent. Merely attaching or listing a reference does not initialize, scan, capture, or write the
target workspace.

### Current captured-memory semantics

A Bootstrap Reference follows the source project's **current captured Ley memory** rather than
pinning artifact/graph snapshot IDs at attachment time. This matches ordinary Context Mount
semantics: authority selects a stable source project, while each read uses that source project's
currently valid captured projections and revision-freshness diagnostics.

The search path may inspect bounded live Git metadata as a freshness beacon. It does not read live
source file contents, refresh capture, or treat captured state as live current source.

### Egress and source-identity safety

The source project's project-level egress policy is checked before any captured reference memory is
searched for the agent target. This first slice deliberately adds no target/per-reference egress
override because an uninitialized target has no stable `prj_` owner for the existing Context Mount
policy key. Source-project revocation therefore remains the complete read-time reference egress
control for bootstrap mode.

Compilation holds authority in this order:

```text
target directory transition lock
  → bootstrap authority lock
    → egress snapshot lock
      → current source binding lock
        → captured-memory read locks
```

The captured-memory search receives the exact expected source project ID. It re-diagnoses the source
before and after search and verifies that the returned project ID still equals that expected ID. The
successful post-search revalidation is the read's linearization point: a delete/recreate or project
replacement observable before that point fails closed as `source-identity-changed`, and replacement
content is never substituted for the granted source. After that successful validation the compiler
may finish assembling the already-read captured snapshot without reading source files again; a later
filesystem mutation does not retroactively change which project produced that snapshot.

### Admission and precedence

Bootstrap References reuse the normal Context Compiler's deterministic memory admission rules rather
than returning raw search results. In particular:

- low-relevance candidates are rejected;
- unverified, contested, rejected, superseded, or stale learning guidance is rejected;
- divergent decision/revision/learning guidance is rejected;
- materially conflicted memory is rejected;
- direct captured evidence retains evidence authority rather than becoming instruction; and
- admitted items retain stable source IDs, session/learning handles, citations, trust, revision,
  ranking, and source authority.

When Bootstrap Specifications and Bootstrap References are both attached, whole exact
Specifications consume result/token budget first and remain higher-precedence human intent.
Conflicting reference **guidance** is withheld with `contradicts-human-intent` and the relevant
Specification IDs. Direct evidence may remain visible because a requirement/implementation mismatch
is useful evidence rather than replacement policy.

The combined compiler reports bounded reference scopes, admitted items, exclusions, coverage, and
omissions. Result/token limits are shared across Specifications first and reference evidence second.

### MCP-only delivery in this slice

An uninitialized workspace with at least one current Bootstrap Specification **or** Bootstrap
Reference may start the dedicated bootstrap MCP server. It still advertises exactly one read-only
tool:

```text
ley_compile_context
```

The result preserves the established Specification fields (`specifications`, `exclusions`,
`coverage`, `projectMemoryAvailable`) and adds `referenceScopes`, `references`,
`referenceExclusions`, `referenceCoverage`, and `referenceMemoryAuthorized`.

The server still exposes no resources and no normal project/session/search/graph/evidence/learning/
utility/write tools. `projectMemoryAvailable` remains `false` because the **target** has no Ley
project memory; any returned memory is explicitly attributed Bootstrap Reference evidence.

Automatic host injection of reference memory is deliberately **not** enabled. A reference-only
uninitialized workspace therefore remains `{}` for lifecycle hooks, including `UserPromptSubmit`.
ADR 0058's prompt-time automatic path remains Specification-only. This keeps a new evidence channel
from silently entering model context before its MCP result/diagnostic contract has independent
evaluation.

### Transition to normal project ownership

Specification and reference grants live in the same target-generation authority record. Every
public initialization path therefore retires both kinds in the same bootstrap-registry transaction
before `.ley` is created, and restores the whole record if initialization fails. Successful
initialization permanently retires both kinds; deleting `.ley` later does not resurrect either.

## Consequences

- An empty workspace can deliberately reuse another project's captured experience without becoming
  a Ley project first.
- The observed Project Catalog remains resolution metadata, never ambient authorization.
- Unattached projects cannot participate in bootstrap retrieval even if their text is more similar.
- Reference text remains lower-authority evidence and cannot silently become human intent or write
  authority.
- Source-project egress can revoke bootstrap reference delivery at read time.
- The target is not initialized, captured, assigned a session, or given write authority by reference
  compilation.
- Automatic prompt-time reference injection remains deferred.

## Verification

The implementation must prove:

- explicit/idempotent non-mutating `bootstrap-ref attach/list/detach` authority;
- backward-readable registry v1 → v2 upgrade only on write;
- Specification + reference authority retire atomically during normal initialization;
- only explicitly attached captured projects are searched; unrelated observed projects cannot leak;
- source-project `never-send` blocks search/return for cloud egress;
- Specification human intent excludes conflicting historical reference guidance;
- a source delete/recreate observable before post-search identity validation returns no reference
  memory and never substitutes replacement-project content;
- reference-only bootstrap MCP exposes exactly one read-only tool and zero resources;
- reference-only host hooks remain no-ops;
- agent-facing output contains no target/source/vault machine paths; and
- deterministic eval covers downstream marker recovery, unrelated-project isolation, egress
  revocation, zero target mutation, and initialization retirement.

## Deliberately deferred

- prompt-time automatic injection of Bootstrap Reference memory;
- target/per-reference egress overrides for an uninitialized workspace;
- pinned-at-attachment reference snapshot semantics;
- reference-project live source-content reads;
- general uninitialized Knowledge Scope, Policy Bundle, connector, folder/note/Canvas mounts;
- desktop management UI for bootstrap authority; and
- converting bootstrap reference outcomes into target project/session memory before explicit normal
  initialization.
