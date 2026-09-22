# ADR 0034: Task-conditioned mounted project reference admission

- Status: Accepted
- Date: 2026-09-17

Later extension: ADR 0074 adds a narrow active-project-current-state conflict rule for mounted
historical Decision/Learning guidance. Exact reviewed/current active-project Learnings may withhold
explicitly contradictory mounted history while mounted direct evidence remains visible.

## Context

ADR 0033 established explicit read-only project Context Mount authority before any agent retrieval path consumed it. The next P0 slice must let an active project reuse captured knowledge from deliberately attached projects without turning Ley's observed-project catalog into ambient MCP search, changing write scope, or flattening reference history into active-project authority.

A software upgrade must also not make mounts created by the earlier authority-only release suddenly agent-visible. Mount authorization and model egress eligibility must advance only through an explicit user action.

## Decision

The Context Mount registry advances to schema v2 while keeping the existing `context-mounts-v1.json` filename. Each entry now records `agentContextEnabled`. Legacy schema-v1 entries decode with this flag disabled and are invisible to MCP/Context Compiler until the user explicitly runs `ley mount add` again. New or explicitly re-added mounts set the flag true.

`ley_compile_context` may resolve only agent-enabled mounts attached to its fixed active project. Resolution happens while holding the mount-registry lock, revalidates source project identity and binding through Ley's existing private Project Catalog and Binding Registry, and searches only already-captured project memory. It does not read live source, ingest, enumerate unmounted projects, or mutate mount authority.

Compiler precedence is structural:

1. exact current approved Specifications (`human-intent`);
2. admitted active-project context and its diagnostics;
3. mounted reference context using only the remaining shared result/token budget.

Mounted items retain `mountId`, source project ID/name, source record kind/identity, citation/ranking/trust signals, and the source record's evidence authority. Their top-level authority is `mounted-reference` and source boundary is `untrusted-mounted-project-memory`. Source trust never grants active-project authority or write permission.

Mounted candidates pass the same relevance/trust/conflict admission rules as active memory. Non-current learnings and materially conflicted records are withheld. Historical mounted guidance that conservatively contradicts admitted human intent is excluded with the exact Specification IDs; mounted direct captured evidence remains visible because a requirement/current-implementation mismatch is useful evidence rather than guidance.

The pack exposes `referencePrecedence: active-project-over-mounted-reference`, `mountedReferenceScopes`, `mountedReferences`, `mountedReferenceExclusions`, and bounded coverage/omission counts. Scope metadata, high-value rejections, returned reference items, and lower-value exclusions are all fitted inside the compiler's existing context-material estimate. An unavailable or uncaptured mounted source fails closed and is diagnostic only.

The mount registry lock remains held across authorization resolution, bounded source search, and reference assembly. A concurrent unmount therefore has a clear linearization point: if unmount wins first, the compiler cannot use the mount; if compilation wins first, unmount waits until that bounded read completes.

MCP gains no mount-management or ambient cross-project tool. Mounted session/learning IDs and citations are provenance only in this slice; the active-project low-level readers and writers do not switch scope when given them. Session and learning writes remain bound to the active project.

## Consequences

- Explicit mounts now produce useful task-conditioned reference context without weakening fixed active-project write isolation.
- Observed-but-unmounted projects remain invisible to MCP compilation.
- Legacy authority-only mounts do not gain agent egress merely because Ley was upgraded; explicit re-add is required.
- Reference provenance and omissions remain inspectable instead of being merged anonymously into active memory.
- The strict compiler budget remains truthful even when many mounts or rejected mounted candidates exist.

## Deliberately deferred

Per-source egress classes beyond this explicit mount opt-in, mounted Specification/note/folder/Canvas/bundle sources, mount-specific deep evidence/session readers, live-source freshness across normal initialized-project references, branch-aware reference adjudication, and any write capability to mounted projects remain separate roadmap work. ADR 0058 separately adds the narrower empty-workspace case for exact already-approved Specifications, and ADR 0059 extends that bootstrap authority with explicitly attached captured reference projects. Neither bootstrap path generalizes normal Context Mounts or grants ambient project discovery.
