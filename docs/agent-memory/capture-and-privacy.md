# Capture and privacy

Every initialized project owns a tiny portable `.ley/capture.json` setting plus `.ley/.leyignore`. Current schema-2 `capture.json` stores only the selected capture mode; Ley derives the reviewed root/Git-ignore/byte-limit defaults in code. Durable continuity and retained evidence live in Ley's owner-private local store. Migrated legacy projects may still need their former vault while crossing compatibility boundaries. In the desktop app, open **Agent Memory → Capture & privacy** to inspect the effective boundary, preview it, and change evidence retention.

## Review the first capture before initialization

Choosing an uninitialized project folder in Ley Desktop performs a read-only preview before setup. Ley evaluates the same default Structured policy and default `.leyignore` rules it would create, then shows eligible file/byte totals, the effective project root, fixed safety limits, bounded path samples, hard-bound skips, and default exclusion categories. This step creates no project `.ley` metadata or private Agent Memory state.

The approval action is bound to the canonical selected project plus a deterministic capture-plan fingerprint covering
the reviewed policy, observed candidate paths/sizes, and hard-bound exclusions. If the project changes before first
capture finishes, Ley fails closed rather than silently capturing a broader plan. Once approved Desktop
initialization creates `.ley`, the project is registered native-born **before** capture; a later capture-plan mismatch
therefore returns to native **Needs capture** instead of falling into legacy-vault onboarding. A true initialized,
unbound pre-cutover project is a different compatibility state and can only reconnect existing validated historical
memory. Existing scoped file reads still reject a candidate whose size changes while ingestion is reading it.

The preview is not a content secret scanner. Default ignored credential-oriented paths plus post-capture pattern redaction remain defense in depth; project-specific exclusions belong in `.leyignore`. Historical schema-1 capture files with narrower custom roots or limits remain honored rather than being silently broadened. See [ADR 0056](../adr/0056-reviewed-first-capture-onboarding.md) and [ADR 0095](../adr/0095-simplify-repo-local-capture-config.md).

## Choose a mode

| Mode | Project evidence retained locally | Automatic turn bodies | Structured checkpoints |
| --- | --- | --- | --- |
| Minimal | Paths, classifications, and post-redaction hashes; no source blobs | Omitted with a body-free disclosure event | Yes |
| Structured | Minimal plus post-redaction UTF-8 source evidence and citations; supported image originals remain excluded | Bounded and pattern-redacted | Yes |
| Full Evidence | Structured evidence plus supported PNG/JPEG/WebP original image evidence | Bounded and pattern-redacted | Yes |

Structured is the recommended default. Full Evidence does not make Ley scrape chats. It is the explicit higher-sensitivity boundary required before Ley retains supported project image originals, so the desktop app requires a separate acknowledgement before enabling it. Current schema-2 config grants no raw-transcript permission in any mode, and current lifecycle adapters never read transcript paths. Any future transcript-capable adapter must introduce its own explicit consent boundary rather than inherit authorization from Full Evidence. Historical schema-1 files may still contain the old dormant `storeRawTranscripts` flag for compatibility, but no current adapter consumes it.

The explicit `ley session import codex-history` workflow is intentionally **not** raw-host-transcript capture and does not depend on Full Evidence or any raw-transcript permission. It reads only a user-supplied, bounded Codex global message-history JSONL source and one explicitly selected session UUID. Minimal stores body-free imported observations; Structured and Full Evidence apply the same bounded redaction/turn-retention rules used for ordinary submitted turn evidence. The importer does not scan Codex storage, parse rollout/session transcript files, or reconstruct assistant/tool/hidden-reasoning history.

Image evidence is intentionally different from UTF-8 evidence: Ley validates supported PNG/JPEG/WebP signatures and stores the exact original bytes in the private content-addressed artifact store, but it does not OCR, visually redact, caption, embed, or otherwise interpret those pixels. Minimal and Structured report such files as `media-requires-full-evidence`. Use `.leyignore`, preview, and Full Evidence consent as the primary privacy boundary for screenshots or images that may contain sensitive visual content. Historical schema-1 projects may additionally retain narrower custom capture roots.

## Inspect the boundary

The panel runs the same deterministic preview as `ley preview`. Preview reads candidate file metadata, not contents. It shows the effective project-relative root, eligible file and byte totals, safety limits, skipped symlinks, and whether Git and `.leyignore` rules apply. For a historical schema-1 project, the panel shows its preserved custom roots/limits if they differ from current defaults.

Applying a mode serializes local writers with a private project-specific OS lock, rechecks the project identity and visible mode, writes the repository-local setting atomically, and immediately runs the same redacted ingestion used by the CLI. The durable artifact snapshot still records the complete effective policy and fingerprint. A stale interface cannot overwrite a mode changed by another local client; reload the panel and decide again.

Current schema-2 mode changes have no hidden policy knobs to preserve. For historical schema-1 projects, mode changes conservatively preserve custom roots, ignore behavior, and byte limits; a default-equivalent schema-1 file may collapse to the mode-only schema during that explicit change. Minimal re-capture stops source blobs from appearing in the current snapshot, but does not erase historical evidence. Use **Erase this project’s Agent Memory** for the separate reviewed deletion workflow: after exact-name confirmation, Ley removes the project's Ley-controlled private continuity/evidence state while preserving source files and `.ley` metadata. Migrated legacy projects still require access to any old vault bytes Ley must honestly erase. The project returns to **Needs capture** when its retained local identity remains.

Erasure waits for current-version memory readers and writers through a project lifecycle lock. Stop connected agent sessions first, especially older Ley processes that do not implement that lock. Ley cannot securely wipe backups, filesystem snapshots, SSD remnants, or copies outside Ley-controlled storage.

To forget one session instead, open that session’s desktop inspector and choose **Erase session memory**. Ley requires the exact current session name and the inspected event version, waits for active memory operations, physically removes the session, and removes every learning that cites it. A learning that would otherwise point to an erased replacement is removed as well. Unrelated sessions, learnings, project captures, files, and capture settings remain.

Session erasure deliberately preserves ordinary Markdown handoffs and JSON Canvas documents because those are explicit user-owned copies rather than private Agent Memory projections. Delete those through the normal note or Canvas workflow when they should also be removed. Neither project nor session erasure can remove external copies, cloud-provider context, backups, storage snapshots, or device remnants.

The Artifact surface can also open retained original image evidence. Browsing the inventory does not eagerly load image bytes; **View original retained media** performs the bounded local read on demand. A media citation from session/activity history carries its exact artifact snapshot ID and content hash so the desktop opens that historical original rather than substituting today's file. The viewer labels it as original evidence and explicitly reports that no OCR/vision description or live-source check was performed. See [ADR 0051](../adr/0051-bounded-original-image-evidence.md).

## Cloud-agent boundary

Ley never uploads captured data independently. A cloud agent such as Claude or Codex may receive bounded context only when the user or host asks that agent to retrieve it. Capture mode controls local retention; it does not override the connected provider's handling of deliberately retrieved context.

## Export portable continuity

The Desktop **Capture & privacy** panel can export this project's Ley-owned continuity into a user-selected local
parent folder. Ley creates a new child directory containing the project-scoped portable SQLite bundle, including
any immutable imported approved-source snapshots stored in that database, plus only artifact-evidence blobs
referenced by durable citations. The chosen parent must be outside the project tree, so an export cannot silently
become project source, enter Git, or be captured again as ordinary evidence. Export is local only and does not
upload or automatically reveal the bundle, but the destination may itself be cloud-synced/shared by other software;
choose it as sensitive continuity data. Human-authored Markdown remains in the project and is edited with external
tools; current approved `.md`/`.mdx` project files can be opened from **Approved Sources** after Ley revalidates
their exact approved revision. See [ADR 0096](../adr/0096-desktop-portability-and-external-markdown.md).

See [ADR 0020](../adr/0020-reviewed-project-memory-erasure.md) and [ADR 0024](../adr/0024-reviewed-session-memory-erasure.md) for the erasure boundaries and concurrency contracts.
