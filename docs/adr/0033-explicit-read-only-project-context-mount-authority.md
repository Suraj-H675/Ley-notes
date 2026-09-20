# ADR 0033: Explicit read-only project Context Mount authority

- Status: Accepted
- Date: 2026-09-16
- Extended by: ADR 0034 (task-conditioned mounted project reference admission)

## Context

Ley's fixed-project MCP process is an important isolation boundary. P0 Context Mounts must allow deliberate cross-project reference reuse without turning the observed-project catalog into ambient agent search or granting writes to another project.

The authorization object must therefore exist before retrieval scope expands. It also should not duplicate project/vault filesystem paths already owned by Ley's private project catalog and binding registry.

## Decision

Ley stores project-reference Context Mount authority in a private `context-mounts-v1.json` registry. A mount contains only a stable mount ID, active project ID, source project ID, and creation time. The only permission in this first slice is `read-only`.

Users create, inspect, and remove mounts explicitly with local CLI commands. Mount creation requires both projects to be initialized and persistently bound. Self-mounts are rejected, mounting the same source twice is idempotent, and one active project may authorize at most 16 reference projects.
Project and vault paths are not stored in the mount registry. Resolution goes through the existing private Project Catalog and Binding Registry each time mounts are inspected. A source project move survives after legitimate re-observation; a different project occupying the recorded location fails closed as `source-identity-changed`. Missing projects and unavailable vault bindings remain visible as unavailable mounts rather than being silently removed.

The registry uses the same owner-private, locked, atomic JSON persistence pattern as Ley's other authority registries and rejects symlink/non-regular/private-permission violations.

## Consequences

- Mount authority is explicit, scoped to one active project, inspectable, revocable, and stable across ordinary source-project moves.
- The observed-project catalog remains discovery metadata, not authorization.
- No MCP tool can create, remove, or enumerate mount authority in this slice.
- No agent retrieval path consumes mounted projects yet. Cross-project agent access remains impossible until a later slice explicitly resolves only authorized mount IDs.
- New session and learning writes remain scoped to the active project; this ADR grants no write capability to references.

## Deliberately deferred

The next slice may expose bounded read-only reference retrieval through the Context Compiler after revalidating mount identity and binding. General empty-workspace **Context Mount / captured reference-project** bootstrapping, note/folder/Canvas/bundle mounts, egress policy, and richer mount UI remain separate roadmap work. ADR 0058 separately implements the narrower uninitialized-workspace case for exact already-approved Specifications only; it does not generalize Context Mount authority.
