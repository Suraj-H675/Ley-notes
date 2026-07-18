# ADR 0023: User-directed Agent Memory session links into JSON Canvas

- Status: Accepted
- Date: 2026-07-18

## Context

An inspected agent session can already become a user-owned Markdown handoff. Linking that handoff into Canvas makes project continuity spatial and connects agent evidence with normal notes, but an automatic destination would remove user agency and could create noisy project maps. Writing a private Agent Memory node type would also make the result Ley-specific instead of interoperable JSON Canvas.

The operation spans two durable files: the promoted Markdown note and a Canvas document. A retry after either write must not create duplicate notes or cards, and a project bound to another vault must never leak into the currently open notes vault.

## Decision

The session inspector exposes a separate **To Canvas** interaction. The user reviews the handoff note title and explicitly chooses an existing Canvas in the open vault or names a new one. Ley does not infer a destination or silently create a project Canvas.

Before either file changes, the desktop adapter canonically verifies that the open notes vault is the project’s private Agent Memory binding. An existing destination is also confirmed before note promotion so a Canvas removed between inspection and submission does not leave a new orphan note.

Ley creates or reuses the stable session promotion defined by ADR 0021, then appends one standard JSON Canvas `file` node pointing to that Markdown path. A card with the same file path is reused on retries. New Canvas filenames follow the normal Canvas naming contract; if the generated path already exists, Ley reuses it and discloses that behavior in the review surface.

An existing destination is parsed before promotion. Invalid JSON is never normalized and overwritten by this workflow: Ley leaves the file byte-for-byte unchanged and asks the user to repair it or choose another Canvas.

After success, the Agent Memory inspector closes and Ley opens the exact destination Canvas rather than whichever Canvas happens to sort first. If the note write succeeds but the Canvas write fails, retrying resolves the session note by project/session identity and adds at most one file card.

## Consequences

- Users retain control over whether and where a session enters their spatial knowledge system.
- Obsidian and other JSON Canvas tools can read the result without understanding Ley Agent Memory.
- Session promotion, Canvas creation, and file-card insertion are independently retry-safe.
- A Canvas link remains an editable user-owned projection, not a second session ledger or a live synchronization channel.
- Canvas deletion, note deletion, or later note movement follows normal vault behavior; Ley does not hide a private relationship database behind the files.
