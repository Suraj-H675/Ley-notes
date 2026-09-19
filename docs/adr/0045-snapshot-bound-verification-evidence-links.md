# ADR 0045: Snapshot-bound verification evidence links

Status: Accepted

## Context

Ley checkpoints already retain bounded verification outcomes (`kind`, `status`, `summary`, and optional `command`) together with the checkpoint's captured project revision. The P1 roadmap next calls for richer test/runtime/verification evidence links. The North Star explicitly allows bounded summaries, hashes, and references while rejecting a design that turns Ley into a raw-log or observability warehouse.

Verification outcomes and repository evidence also have different semantics. A statement that a test passed is historical session memory. A captured artifact is immutable repository evidence with a project-relative path, artifact snapshot ID, content hash, and line range. Neither one proves that the live working tree still matches the captured state, and a `passed` status must not become authority merely because it has a citation.

The existing session event versions are deliberately type-reserved: v1 lifecycle events, v2 turn evidence, and v3 bound recovery checkpoints. Adding durable evidence links to a v1 checkpoint without a new event version would weaken Ley's explicit schema-evolution boundary.

## Decision

Checkpoint verification records may optionally cite artifacts from the current approved captured snapshot.

- The input field is `evidenceArtifactPaths`, containing project-relative paths only.
- Each path must already exist in Ley's current approved captured artifact manifest. Ley does not fall back to the live filesystem and does not accept an arbitrary path, URL, provider attachment, or log location.
- At write time Ley resolves each path into the existing immutable `SessionArtifactCitation`: project-relative path, artifact snapshot ID, SHA-256 content hash, and bounded captured line range.
- Verification records return those citations as `evidenceArtifacts`. The citation identifies original captured evidence; the verification summary remains historical interpretation of that evidence.
- The checkpoint's existing `projectRevision` remains the revision/snapshot context for the outcome. A citation does not make `liveSourceChecked` true.
- A verification may cite at most 20 evidence artifacts, with at most 200 verification-evidence citations across one checkpoint. Replay validation enforces the same limits and citation integrity as write-time normalization.
- `ley_session_get`, Current Project State, and Topic Dossiers preserve evidence citations when they expose verification. Topic Dossiers already fit the complete serialized projection to their token budget; Session Context and Current Project State each return at most 64 verification-evidence citations per projection and explicitly report per-record or coverage-level omissions rather than allowing structural metadata to exhaust the MCP result budget. Topic Dossiers may also surface a cited verification artifact in their bounded important-artifact set.
- Session Markdown includes the command and immutable evidence references for human audit without copying artifact bodies.

Durable schema evolution is explicit:

- Ordinary lifecycle checkpoints with no verification evidence links remain schema-v1 events.
- A checkpoint that carries at least one verification evidence link is stored as session event schema v4 and upgrades the rebuildable projection to `session-v4.json`.
- Existing v1/v2/v3 ledgers remain readable without read-time rewriting.
- A v4 event that is not a checkpoint with verification evidence links fails validation, and v1 is forbidden from carrying those links.

This slice deliberately does **not** add raw runtime-log storage, CI-provider APIs, screenshot/media capture, OCR/vision descriptions, benchmark warehouses, or arbitrary external references. If a text report is deliberately part of the captured project evidence, it can be cited; otherwise Ley reports no link rather than inventing or scraping one. Multimodal evidence remains a later roadmap item where original media and derived interpretation require separate provenance.

## Consequences

Agents can distinguish an unsupported historical claim such as "tests passed" from a verification outcome that points to immutable captured evidence, while still preserving the authority boundary: status, recency, and successful use do not become truth. Live file drift after capture does not rewrite the stored evidence hash, and agent-facing readers continue to report that live source was not checked.

The first slice is intentionally narrower than a general runtime evidence system. Test frameworks or CI systems that emit evidence outside the captured project cannot be linked directly yet. Future provider-specific or multimodal evidence must define bounded retention, content-addressing, redaction, egress, deletion, and original-vs-derived provenance before expanding this schema.
