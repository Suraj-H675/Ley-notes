# ADR 0051: Bounded original image evidence

Status: Accepted

## Context

Ley already stores snapshot-bound text evidence and lets structured verification records cite captured artifacts. The next P2 slice needs screenshots and other project-local images to remain inspectable evidence without turning Ley into a binary warehouse, observability platform, or automatic vision system.

Images differ materially from retained UTF-8 source. Ley cannot truthfully apply its text secret-redaction pass to pixels, and an OCR/vision description would be a derived interpretation rather than the original observation. Silently retaining image bytes under Structured capture would therefore broaden the privacy contract that users previously accepted.

## Decision

Ley supports a first bounded original-image evidence slice with these limits:

- only explicitly selected **Full Evidence** capture retains supported project-local image originals;
- supported media are PNG, JPEG, and WebP, and the extension must match the expected file signature before the bytes are accepted;
- Minimal and Structured capture do not retain those originals and report `media-requires-full-evidence` instead of silently widening retention;
- capture still obeys the existing approved roots, Git/`.leyignore` exclusions, symlink confinement, per-file/total limits, private vault binding, and immutable content-addressed snapshot model;
- image bytes are not OCRed, visually redacted, described, embedded into graph/text search, or promoted into deterministic project facts;
- original media is labeled `original-media` beneath the `untrusted-project-evidence` boundary. `derivedDescriptionIncluded` is false and `liveSourceChecked` is false.

The core cited-media read requires the exact project-relative path, retained artifact snapshot ID, and SHA-256 content hash. It resolves only retained immutable artifact history and never substitutes the current live file. Core/desktop reads are bounded to at most 1 MiB per requested image.

## Session and learning provenance

Text and media citations have different durable semantics.

- Existing text verification citations remain session schema v4 and keep their positive captured line range.
- A checkpoint containing an image citation uses session schema v6 and a rebuildable `session-v6.json` projection.
- A media citation includes `mediaType` and uses `startLine: 0`, `endLine: 0` to state explicitly that no text range exists. Consumers must not render `0–0` as a source line range.
- Existing v1–v5 immutable events remain readable without rewrite. Schema v6 is reserved for checkpoints that actually contain multimodal artifact citations.
- Learning evidence may preserve those same media citations and origin identities. That provenance does not increase the learning’s authority and does not create a generated visual claim.

## Agent read boundary

`ley_read_media_evidence` is a fixed-project, read-only MCP tool. The caller must supply the exact path/snapshot/hash from a Ley citation. A successful result contains:

- compact structured provenance metadata; and
- one native MCP image content block containing the exact retained original bytes.

The MCP path uses the same active-project egress gate as other direct captured evidence. It performs no network request and exposes no project/vault path. To preserve Ley’s existing 256 KiB serialized MCP-result ceiling, one MCP media delivery is capped at 180,000 source bytes and the completed protocol result is checked again before return. Larger retained images remain inspectable through the local desktop/core boundary but are not sent through this MCP tool in one call.

The tool supplies no OCR, caption, visual conclusion, or other model-authored description. A host/model may inspect the returned image, but any conclusion it forms is derived interpretation and must not be confused with the immutable original.

## Human inspection boundary

The desktop Artifact surface can inspect retained original media from the same immutable path/snapshot/hash read. Browsing an artifact inventory does not eagerly load image bytes; the user explicitly opens **View original retained media**. A session/activity citation may deep-link directly to its exact historical snapshot/hash. The viewer labels the image as original evidence and states that no OCR/vision description or live-source check was performed.

## Storage, privacy, and erasure

This slice creates no new media database. Original bytes live in the existing private content-addressed artifact store and follow the existing project/session citation, lifecycle-lock, and project-memory erasure semantics. Session erasure removes the session ledger that cites an image but does not erase a still-retained project artifact snapshot merely because that session referenced it; whole-project Agent Memory erasure removes the Ley-managed captured artifact history. User-owned exports/backups remain outside that guarantee as already documented.

Full Evidence is therefore the explicit sensitivity boundary for pixels. Ley does not claim visual secret redaction. Users must use capture roots, ignore rules, preview, and deliberate Full Evidence consent to keep private screenshots or images out of capture.

## Deferred scope

This first multimodal slice does **not** add:

- PDFs, diagrams requiring parsing, audio/video, arbitrary binary attachments, or CI/provider attachments;
- OCR, image embeddings, vision-generated summaries, or automatic visual indexing;
- provider/network screenshot collection, browser automation, or runtime screenshot capture;
- a general raw-log or observability store.

Those require separate retention, provenance, redaction/egress, evaluation, and deletion designs.

## Evaluation

P2 coverage includes a deterministic real-binary `multimodal-original-image-evidence` scenario. It captures a PNG under Full Evidence, records a verification citation, mutates the live image afterward, and calls the real MCP media reader. Passing requires schema-v6 non-text citation semantics, the exact original bytes from the cited immutable snapshot, `derivedDescriptionIncluded: false`, `liveSourceChecked: false`, no local-path leakage, and zero privacy-canary violations.

Core, MCP, learning, desktop, export, and host-package regressions additionally cover Full-Evidence-only retention, extension/signature validation, immutable hash binding, output bounds, original-vs-derived labeling, and historical desktop inspection.
