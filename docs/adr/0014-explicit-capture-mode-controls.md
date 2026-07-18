# ADR 0014: Explicit capture-mode controls

Status: accepted

## Context

Ley initializes projects with Structured capture and already validates Minimal and Full Evidence policies, but desktop users cannot inspect or change that choice. Merely displaying a mode badge is not meaningful privacy control. Editing `.ley/capture.json` by hand is error-prone, gives no preview, and can leave the durable artifact snapshot out of sync until a later ingestion.

The three modes must also be described according to what Ley implements today. Project-evidence retention and structured session capture are related but distinct. Full Evidence is permission for a transcript-capable adapter; it is not permission for Ley to scrape complete chats automatically.

## Decision

Add a first-class Capture & privacy surface to each ready desktop project. It reads the validated repository-local policy and deterministic metadata-only capture preview on demand. The view exposes:

- Minimal, Structured, and Full Evidence retention choices;
- approved project-relative roots and Git/`.leyignore` behavior;
- eligible file/byte totals and size/symlink exclusions;
- current retained-source count and configured byte ceilings;
- the capture-policy fingerprint and local/cloud boundary.

Changing mode updates only `mode` and `storeRawTranscripts`. Custom approved roots, ignore behavior, and byte limits are preserved. A caller must send the mode it observed. Ley serializes writers with a project-specific cross-process lock in its private operating-system configuration directory, then re-reads the project identity and policy while holding that lock. A stale expected mode fails instead of overwriting a newer privacy decision.

Enabling Full Evidence when raw-evidence permission is not already active requires an explicit consent flag. The desktop interface obtains it through a dedicated warning checkbox. Selecting another mode immediately clears that pending consent. Moving away from Full Evidence clears `storeRawTranscripts`.

After an accepted change, the native command runs deterministic ingestion against the already resolved project/vault binding and returns a refreshed Agent Memory dashboard. Minimal snapshots retain artifact metadata, hashes, graph structure, and structured session memory but omit project source blobs. Structured and Full Evidence snapshots retain redacted source evidence. Full Evidence additionally permits a compatible adapter to submit versioned raw host evidence; no current Ley process reads a complete host transcript automatically.

The policy write is atomic and revalidates the `.ley` directory immediately before replacement. The lock contains no knowledge data and uses owner-only permissions on Unix. A mode conflict, changed project identity, or missing consent fails before mutation. If re-ingestion fails after a successful policy write, Ley reports that the policy was saved and directs the user to retry snapshot refresh rather than claiming the old snapshot matches the new policy.

## Consequences

- Privacy modes become inspectable, reversible product behavior rather than initialization-only labels.
- Users see the exact local boundary before applying a change.
- Minimal mode materially reduces retained project content without deleting structured continuity records.
- Full Evidence remains an explicit high-sensitivity permission and does not imply automatic transcript collection.
- Editing approved roots, ignore rules, byte ceilings, and fine-grained retention remain separate future controls. [ADR 0020](0020-reviewed-project-memory-erasure.md) now defines reviewed whole-project Agent Memory erasure.
