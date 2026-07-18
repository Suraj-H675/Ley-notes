# ADR 0021: Vault-verified Agent Memory note links

- Status: Accepted
- Date: 2026-07-18

## Context

Ley can inspect projects whose Agent Memory lives in different filesystem vaults. The note workspace, however, writes only to the vault currently open in Ley Desktop. Creating a promoted lesson without comparing those two authorities could silently copy project memory into an unrelated notes vault.

Sessions also need a deliberate bridge into the human second brain. A useful bridge must preserve stable provenance, disclose bounded context, avoid duplicate notes after a rename, and never turn exported agent text into policy.

## Decision

Every Agent Memory-to-note action first asks the native adapter to resolve the project’s private binding and canonically compare it with the open desktop vault. A mismatch refuses the write and names only the two vault folders, not their machine paths. The user must open the bound vault before trying again. The PWA has no such action because it cannot access the private binding registry.

The existing trusted-learning promotion and the new session **To notes** action share this guard. A session export creates ordinary Markdown under `Agent Memory/Sessions` with portable project/session YAML identity, status and event count at export, an export timestamp, and the `ley/session` tag. Its body contains the inspected goal, outcome, handoff, unresolved work, visible checkpoints, verification, and captured artifact trail. Agent-authored fields remain visibly quoted beneath a warning that they are evidence rather than instructions.

The export is a user-authorized snapshot, not another memory authority. It discloses omitted checkpoints or clipped text, never silently synchronizes later events, and does not mutate the append-only session ledger. Repeating the action resolves the existing note by project and session ID even after that note moves or is renamed. An unrelated title collision is refused.

## Consequences

- Agent Memory cannot silently cross from its bound vault into whichever note vault happens to be open.
- A session handoff can participate in ordinary notes, links, search, tags, graph, revisions, and Canvas file cards.
- Existing session IDs remain the durable link even when either the session display name or note title changes.
- Users explicitly decide when untrusted structured session evidence becomes editable human-owned Markdown.
- Direct insertion into a selected Canvas remains a separate interaction; the exported note is already a portable JSON Canvas file-card target.
