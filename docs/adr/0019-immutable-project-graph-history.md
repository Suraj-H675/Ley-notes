# ADR 0019: Immutable project graph history and cited source inspection

Status: accepted

## Context

Ley already retained immutable artifact and project-graph snapshots, but the desktop graph exposed only the current pointer. A user could not compare captures, distinguish a historical relationship from the working tree, or inspect the exact captured source behind an edge. Reading a current file for an old graph would be actively misleading.

Graph filters also need to be applied by the shared engine. Filtering only rendered nodes in React would make counts, bounds, and connectedness disagree with the result the user sees.

## Decision

- Each changed ingestion appends the graph snapshot ID, matching artifact snapshot ID, capture time, counts, and bounded Git identity to a compact private `history-v1.json` index. Identical ingestion remains a no-op and does not create a duplicate capture.
- History contains at most 512 entries and 512 KiB. It is an index over already immutable snapshots, not a second copy of graph or source data.
- Historical reads select an exact graph snapshot and its recorded artifact snapshot under the normal shared project lock. Both snapshots, immutable files, project identity, and cross-snapshot references are verified before use.
- Node-kind, edge-kind, and provenance filters are validated and applied in the Rust projection before search, graph bounding, degree calculation, and response counts.
- Source inspection accepts an exact graph snapshot and an exact citation already attached to the selected node or edge. Ley rejects a forged, broadened, or unrelated citation.
- The excerpt is read from the selected immutable artifact, never from the live project. It is redacted, line-numbered, bounded to 200 lines and 8 KiB, and labeled with its capture boundary. Minimal captures return an honest unavailable-evidence result because they contain no source blob.
- The desktop graph makes current versus historical state explicit, provides capture and semantic filters, and opens evidence in a nonmodal inspector so the graph remains spatially available.

## Consequences

- A user can reopen the evidence Ley actually used, even after the working tree changes or deletes the file.
- Historical results still do not claim that live source was checked. Re-capture is the only way to update the current snapshot.
- The history index is rebuildable from retained immutable graph snapshots in a future repair workflow, but v1 does not scan the store on every read.
- Switching capture mode does not erase earlier immutable evidence. Retention and secure deletion remain a separate user-authority design.
- Agents continue to receive bounded cited projections; this desktop inspection surface does not expand MCP authority.
