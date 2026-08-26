# ADR 0024: Reviewed session Agent Memory erasure

- Status: Accepted
- Date: 2026-07-18

## Context

A structured session can contain private prompts, outcomes, commands, problems, and handoff text that a user needs to forget without removing the project’s unrelated history. Deleting only the session directory is insufficient: proposed and corrected learnings copy bounded session-derived evidence into a separate immutable ledger. Keeping one of those learnings would retain derived content after Ley claimed to erase its source.

Tombstones are also the wrong default for a privacy action because they retain the deleted session’s identity. Supersession introduces a second dependency: a retained learning cannot point to a replacement learning that was erased with its cited session.

Ordinary Markdown handoffs and JSON Canvas cards are different. They are explicit user-owned copies in the normal vault, can be edited independently, and may have acquired links or annotations that are unrelated to the private session ledger.

## Decision

Ley provides a local user-authorized **Erase session memory** action from the desktop session inspector and an explicitly confirmed local CLI command. MCP tools and automatic Codex and Claude adapters receive no erasure authority.

Before erasing, the caller must provide the exact current session name and the event count it inspected. Ley acquires the project’s exclusive lifecycle lock, replays and validates the authoritative session and learning ledgers, then rechecks both values. A stale inspector or concurrent writer fails without deleting memory.

Erasure physically removes the complete session directory rather than appending a tombstone. It also physically removes every learning whose proposal or any later correction cites that session. If another learning is superseded by one of those removed learnings, Ley removes that dependent learning as well; the cascade continues until no retained supersession points to an erased record. Unrelated sessions, unrelated learnings, project artifacts, graph history, source files, repository-local `.ley` policy, vault binding, and project catalog entry remain.

Ley rebuilds the derived learning index and review Markdown from the remaining authoritative events using atomic file replacement. Corrupt or dangling learning history aborts before the session directory is removed.

User-owned Markdown session exports and Canvas documents remain untouched. The confirmation must state this explicitly: users who want those copies removed must delete them through the normal notes and Canvas workflows. Ley does not call the operation forensic secure erase and cannot remove backups, filesystem snapshots, provider-retained context, storage remnants, or copies outside the selected vault.

## Consequences

- A user can forget one captured agent session without destroying unrelated project continuity.
- Session-derived lessons do not survive under another record ID or through a dangling supersession chain.
- Privacy deletion intentionally sacrifices the append-only audit trail for the erased records; no internal tombstone retains their identity.
- Normal notes and Canvas documents remain stable and portable, but Ley must clearly disclose that they can contain user-created copies of the erased material.
- Current Ley readers and writers cannot race the operation because the exclusive lifecycle lock waits for their shared operations. Older processes predating that lock should still be stopped.
- Multi-file removal is not a cryptographic transaction or secure wipe. Fault-injection and recovery testing remain required for storage failures between durable filesystem operations.
