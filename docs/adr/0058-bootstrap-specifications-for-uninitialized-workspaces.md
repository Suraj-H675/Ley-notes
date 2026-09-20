# ADR 0058: Bootstrap Specifications for uninitialized workspaces

- Status: Accepted
- Date: 2026-09-20

## Context

Ley's ordinary agent boundary is intentionally fixed to one initialized project identity and one
private project-to-vault binding. That is the correct boundary for captured project memory,
sessions, learnings, graph evidence, and writes, but it leaves a distinct product journey from
`LEY.md` unresolved: a user may want an empty or unrelated workspace to consume already-reviewed
requirements before deciding whether that workspace should become a Ley project at all.

Initializing the target merely to obtain reference context would conflate read-only human intent
with writable project memory. Generalizing Context Mounts would also be too broad for the first
slice because mounts carry captured historical project memory rather than only reviewed
Specifications.

## Decision

Ley adds a separate **Bootstrap Specification** authority for uninitialized workspaces.

The authority is created only by an explicit local-user command:

```text
ley bootstrap-spec attach SOURCE_PROJECT SPECIFICATION_ID [WORKSPACE]
```

`list` and `detach` are likewise local-user operations. MCP and lifecycle hooks expose no mutation
route for bootstrap authority.

The source Specification must already be an exact current user-approved Specification in an
initialized and bound source project. Bootstrap attachment does not create or approve human intent.

### Target identity without `.ley`

An uninitialized workspace has no `prj_` identity, so the bootstrap registry must not pretend that
it does. The owner-private registry keys authority to a deterministic `bsw_` identity derived from:

- the canonical target directory path; and
- a Unix filesystem directory generation made from device, inode, and filesystem creation time.

The private record retains the canonical target path only so Ley can bind authority to the same
local directory generation. Agent-facing responses never expose that path. Deleting and recreating
a directory at the same pathname produces a different generation and does not inherit the grant.
If the platform/filesystem cannot expose that complete Unix generation, bootstrap authority fails
closed; normal Ley project functionality remains available. This first slice deliberately does not
claim an equivalent Windows ACL/file-ID implementation.

Each `bsg_` grant stores only:

- the stable source project ID;
- the stable Specification ID;
- the exact approved SHA-256 content hash; and
- attachment time.

Specification bodies remain in the source project's ordinary bound vault.

### Read-time source resolution and egress

After attachment, callers cannot select a source path or vault path. Ley resolves the stable source
project through the private Project Catalog, revalidates the live `.ley` identity, and resolves its
private binding. The source project ID used for project/Specification egress remains the expected
identity for the approved-source read: immediately before consulting the approval registry/body,
Ley re-diagnoses the source path and rejects any project-ID replacement instead of accepting content
from a different project that appeared at the same pathname.

The configured agent egress target is checked against both:

1. the source project's project policy; and
2. the exact source-Specification policy.

Those checks occur before the Specification note is opened for agent delivery or task scoring.
`confirm-per-use` remains fail-closed until Ley has a real confirmation flow. A changed, missing,
revoked, reapproved-at-a-different-hash, unavailable, identity-replaced, or unbound source returns
only bounded exclusion metadata and no source text. One missing approved note does not abort other
valid grants; it is reported as `SpecificationUnavailable` while unrelated whole Specifications may
still compile. Reapproval does not silently update an existing bootstrap hash pin; the user must
explicitly attach again.

### Bootstrap compiler

Bootstrap task compilation is deliberately narrower than the normal Context Compiler:

- exact attached approved Specifications are the only context source;
- task relevance uses the existing bounded Specification lexical nomination rules;
- Specifications consume the familiar result/token limits;
- a Specification is returned **whole or omitted whole**; bootstrap never clips human intent;
- there is no active-project memory, session state, learning, graph, mounted-reference memory,
  connector content, or inferred historical context;
- results state `projectMemoryAvailable: false`, `liveSourceChecked: false`,
  `persisted: false`, and `automaticWriteAllowed: false`.

The target workspace itself is never scanned or captured by bootstrap compilation.

### Bootstrap-only MCP

`ley mcp WORKSPACE` still prefers the ordinary initialized-project/binding path. Only a true
uninitialized workspace with a current explicit bootstrap grant starts the bootstrap MCP server.

That server advertises exactly one read-only tool:

```text
ley_compile_context
```

It advertises no resources and no session, learning, graph, search, evidence, utility-feedback, or
write tools. An initialized-but-unbound project does **not** fall back to bootstrap mode; it remains
an inactive normal project until the user deliberately binds/captures it.

### Lifecycle hooks

For an explicitly bootstrapped uninitialized workspace:

- `SessionStart`, `Stop`, and other non-prompt lifecycle events remain no-ops;
- `UserPromptSubmit` may inject `# Ley bootstrap task context (automatic)`;
- no Ley session is created;
- the prompt is not retained as Ley turn evidence;
- the raw prompt is not repeated in injected context;
- the automatic block remains under Ley's strict host byte bound;
- each Specification body is admitted atomically, never partially clipped;
- if a whole relevant Specification cannot fit, the block reports that omission and points to the
  bootstrap MCP `ley_compile_context` tool for the complete approved document.

Ordinary uninitialized workspaces with no explicit bootstrap grant remain quiet no-ops.

### Transition to a real Ley project

Bootstrap authority must not silently become writable project authority.

Every public project initialization path serializes with bootstrap attach/compile through a
non-persistent advisory lock on the target directory, then takes the private bootstrap-registry
lock in the same order. If bootstrap authority exists, initialization retires the current-generation
grant before creating `.ley` and restores the grant if initialization fails. Successful
initialization permanently retires that bootstrap authority. Removing `.ley` afterward does not
resurrect it. If bootstrap generation cannot be established, ordinary project initialization still
proceeds; only the bootstrap feature fails closed. If initialization fails **and** the private
registry cannot restore the retired grant, Ley returns an explicit restoration-failed error
requiring local authority review rather than silently reporting only the initialization failure.

Normal initialization consent/capture review remains unchanged; bootstrap context never initializes
the target itself.

The private inner initializer that actually creates `.ley` is bootstrap-agnostic; it is not part of
the public API. Public `initialize_project`, CLI setup, and desktop setup all pass through the same
coordinated transition so an attach cannot be persisted between the initialization check and
project creation.

## Consequences

- Ley can now serve already-reviewed human intent to a genuinely uninitialized workspace without
  creating project memory or widening agent write authority.
- Source project and Specification egress remain independently revocable at read time.
- On supported Unix filesystems, a target-path delete/recreate does not inherit stale bootstrap
  authority; unsupported generation semantics fail closed.
- Host integrations can use small whole Specifications automatically and fall back to the single
  MCP compiler tool for larger complete documents.
- Bootstrap output cannot be mistaken for a normal project session or historical memory surface.
- Transitioning into normal Ley project ownership is explicit and one-way with respect to the old
  bootstrap grant.

## Verification

The slice is covered by:

- core grant/generation/revision/egress/budget/private-permission/initialization regressions,
  including full delete/recreate, exact target-lock attach-vs-init and compile-vs-init serialization,
  missing-note-with-surviving-grant behavior, current-binding/rebind, unsupported-generation normal
  initialization, deterministic source delete/recreate after egress-subject resolution, and
  initialization-restoration failure cases;
- core lifecycle-hook tests proving no session or prompt capture and whole-document host omission;
- MCP router plus real stdio protocol tests proving exactly one read-only tool, zero resources,
  path-safe output, and live egress revocation;
- real CLI lifecycle/startup tests covering attach → hook context → initialize → permanent
  retirement, ordinary zero-mutation inactive workspaces, and path-free malformed-registry errors;
  and
- the deterministic `empty-workspace-bootstrap-specification` eval covering real MCP/hooks, privacy,
  source egress revocation, and normal initialization transition. On a platform/filesystem that
  explicitly cannot establish Ley's bootstrap directory generation, this positive bootstrap
  scenario is reported as unsupported/SKIP rather than a false product failure; ordinary
  uninitialized no-op and normal-initialization coverage remain active cross-platform.

## Deliberately deferred

- bootstrap access to captured reference-project sessions, learnings, graph, or artifacts;
- general empty-workspace Context Mounts or Knowledge Scope/Policy Bundle attachment;
- desktop UI for selecting/managing Bootstrap Specifications;
- remote/shared bootstrap authority or collaboration roles;
- automatic bootstrap discovery, attachment, initialization, capture, or scanning;
- converting bootstrap outcomes into project/session memory before explicit normal initialization.
