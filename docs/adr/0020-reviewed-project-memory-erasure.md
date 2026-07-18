# ADR 0020: Reviewed whole-project Agent Memory erasure

Status: accepted

## Context

Ley retains immutable artifact captures, graph history, structured sessions, and learning events inside one vault-scoped project namespace. “Forget project” removes only the private observation-catalog entry; changing capture mode affects future/current retention but does not erase historical snapshots. Neither action can honestly be presented as deletion.

Removing individual immutable events would also break citations, replay, corrections, and provenance unless Ley first defines tombstones and dependent-record behavior. The trustworthy first deletion boundary is therefore the complete Agent Memory namespace for one project.

Deletion can race with a running MCP server, host hook, ingestion, or desktop read. Locks stored inside the directory being deleted are insufficient: a process can hold an open directory or lock-file descriptor after that namespace has been removed.

## Decision

- The desktop **Capture & privacy** surface provides a whole-project **Erase Agent Memory** action. It permanently removes captured artifacts and immutable history, the project graph, structured sessions, and learning ledgers for that project from the selected vault.
- Erasure preserves the user’s project files, ordinary notes and Canvas documents, repository-local `.ley` identity/capture policy, private vault binding, and observation-catalog entry. The project becomes **Needs capture**, so the user can deliberately recapture later.
- The confirmation names every deleted category, requires the exact project name, warns the user to stop Codex, Claude Code, and Gemini sessions, and states that Ley cannot securely wipe backups, filesystem snapshots, SSD remnants, or copies outside the vault.
- Every current-code memory reader and writer acquires a shared project lifecycle lock stored as a sibling of the erasable project directory. Erasure acquires the exclusive lock, waits for active operations, revalidates the no-follow project directory, and removes only that capability-scoped relative namespace.
- The small lifecycle lock file remains after erasure. It contains no memory content and prevents a recapture/deletion race.
- Erasure is desktop/user authority only. It is not exposed through MCP tools or automatic host adapters.
- This action is not described as cryptographic or forensic secure erase.

## Consequences

- Users can remove all Agent Memory retained by Ley for one project without deleting their work or losing the project-to-vault setup.
- Running current-version agents cannot write into an unlinked directory during deletion; erasure waits for their in-flight memory operation to finish.
- Old Ley processes that predate the lifecycle-lock protocol must still be stopped before erasure, which the UI states explicitly.
- Fine-grained session/learning deletion remains a separate future design because dependent citations and append-only provenance need explicit semantics.
- External backups and storage-layer remnants remain the user’s responsibility.
