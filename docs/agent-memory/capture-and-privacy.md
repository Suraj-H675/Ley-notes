# Capture and privacy

Every initialized project owns a small, portable `.ley/capture.json` policy. Durable evidence still lives in the bound filesystem vault. In the desktop app, open **Agent Memory → Capture & privacy** to inspect the policy, preview its current filesystem boundary, and change evidence retention.

## Choose a mode

| Mode | Project evidence retained in the vault | Automatic turn bodies | Structured checkpoints | Raw host transcript permission |
| --- | --- | --- | --- | --- |
| Minimal | Paths, classifications, post-redaction hashes, deterministic graph; no source blobs | Omitted with a body-free disclosure event | Yes | No |
| Structured | Minimal plus post-redaction UTF-8 source evidence and citations | Bounded and pattern-redacted | Yes | No |
| Full Evidence | Structured evidence | Bounded and pattern-redacted | Yes | Explicitly permitted for a future, separately invoked compatible adapter |

Structured is the recommended default. Full Evidence does not make Ley scrape chats. It allows a future, explicit transcript-capable adapter to submit raw evidence, so the desktop app requires a separate acknowledgement before enabling it. Current lifecycle adapters never read transcript paths in any mode.

## Inspect the boundary

The panel runs the same deterministic preview as `ley preview`. Preview reads candidate file metadata, not contents. It shows approved project-relative roots, eligible file and byte totals, configured limits, skipped symlinks, and whether Git and `.leyignore` rules apply.

Applying a mode serializes local writers with a private project-specific OS lock, rechecks the project identity and visible mode, writes the repository-local policy atomically, and immediately runs the same redacted ingestion used by the CLI. The durable artifact manifest records the complete policy and fingerprint. A stale interface cannot overwrite a mode changed by another local client; reload the panel and decide again.

Mode changes preserve custom roots, ignore behavior, and byte limits. Minimal re-capture stops source blobs from appearing in the current snapshot, but does not erase historical evidence. Use **Erase this project’s Agent Memory** for the separate reviewed deletion workflow: after exact-name confirmation, Ley removes that project’s captured artifacts, graph history, sessions, and lessons from the bound vault while preserving project files, notes, `.ley` policy, and the private binding. The project returns to **Needs capture**.

Erasure waits for current-version memory readers and writers through a project lifecycle lock. Stop connected agent sessions first, especially older Ley processes that do not implement that lock. Ley cannot securely wipe backups, filesystem snapshots, SSD remnants, or copies outside the selected vault.

To forget one session instead, open that session’s desktop inspector and choose **Erase session memory**. Ley requires the exact current session name and the inspected event version, waits for active memory operations, physically removes the session, and removes every learning that cites it. A learning that would otherwise point to an erased replacement is removed as well. Unrelated sessions, learnings, project captures, graph history, files, policy, and binding remain.

Session erasure deliberately preserves ordinary Markdown handoffs and JSON Canvas documents because those are explicit user-owned copies rather than private Agent Memory projections. Delete those through the normal note or Canvas workflow when they should also be removed. Neither project nor session erasure can remove external copies, cloud-provider context, backups, storage snapshots, or device remnants.

The desktop project graph indexes changed immutable graph/artifact pairs so an earlier capture remains inspectable. Selecting a historical capture never reads today's working tree. A node or relationship can reveal only an exact citation already present in that selected graph, from its matching retained artifact snapshot. Excerpts stay redacted and bounded. Minimal captures can still show graph structure and citations, but source inspection is unavailable because that mode intentionally retains no source blob.

## Cloud-agent boundary

Ley never uploads captured data independently. A cloud agent such as Claude, Codex, or Gemini may receive bounded context only when the user or host asks that agent to retrieve it. Capture mode controls local retention; it does not override the connected provider's handling of deliberately retrieved context.

See [ADR 0020](../adr/0020-reviewed-project-memory-erasure.md) and [ADR 0024](../adr/0024-reviewed-session-memory-erasure.md) for the erasure boundaries and concurrency contracts.
