# ADR 0096: Focused Desktop export and external Markdown handoff

**Status:** Accepted — 2026-10-02

## Context

The reset deliberately removed Ley's general-purpose note editor and Canvas workspace. Human-authored Markdown
belongs in the user's project and ordinary editor; Ley should not rebuild an editor merely to make approved intent
usable. The focused Desktop also needs a concrete portability action rather than describing export as a backend-only
capability.

Two already-proven primitives exist:

- native Approved Sources validate exact project-file authority with no-follow filesystem access; and
- portable continuity export creates a validated project-scoped SQLite/evidence bundle with only cited immutable
  evidence required by durable continuity.

The missing work is a narrow Desktop bridge over those primitives.

## Decision

### Open approved Markdown externally

The Approved Sources panel may open a source externally only when all of these are true at action time:

1. it is native approved-source authority for the selected project;
2. the source is a `project-file`, not an imported immutable snapshot;
3. its exact approved bytes are still current;
4. its project-relative path is Markdown (`.md` or `.mdx`); and
5. every path component is rechecked immediately before handoff and remains non-symlink, with a regular file at
   the final component.

The frontend sends only the project and stable source ID. It never receives a general "open arbitrary path"
capability. The Rust host revalidates authority and uses the pinned cross-platform `open` crate to hand the validated
file to the operating system's default application in a detached process. Editing the file naturally makes its
approval stale until the user reviews/reapproves the new revision.

This remains a point-in-time pathname handoff: no portable desktop API can atomically bind the already-validated
file descriptor to an arbitrary external editor. Ley therefore minimizes the race window and refuses symlinked
components, but does not claim the launched editor receives an immutable copy. The editor's own sync, backup,
extension, or cloud behavior is outside Ley's privacy boundary.

Changed, missing, non-Markdown, and imported-snapshot sources are not open-in-editor targets. Imported snapshots are
historical authority/evidence, not mutable source files.

### Export continuity bundle

The Capture & privacy surface gains an explicit **Export continuity bundle** action. The user selects an existing
local parent directory through the already-permitted native directory chooser. Rust then:

- re-resolves the selected project's native/legacy transition access;
- rechecks project identity;
- rejects a destination parent inside the project, so private continuity cannot be accidentally recaptured or
  committed with source;
- creates a new uniquely named child directory; and
- delegates bundle creation and validation to the existing core portable exporter.

The result reports the destination plus event, cited-snapshot, and evidence-blob counts. Ley does not upload the
bundle or automatically open/reveal it afterward. The SQLite bundle may itself contain immutable imported
approved-source snapshot bytes in addition to continuity records, and the user-selected parent directory may be
synced/shared by unrelated software. The UI therefore describes the export as sensitive local continuity data
rather than promising the destination is private on every filesystem/platform.

## Dependency boundary

Ley uses `open = 5.4.4`, pinned and called only from the validated Rust command. The crate opens a path using the
desktop's configured default application across Linux, macOS, Windows, and WSL and keeps launcher arguments
separated from launcher options where the platform permits. Ley does not register a generic Tauri opener plugin,
ship its JavaScript binding, or grant an `open arbitrary path` capability to the frontend. This keeps the dependency
and capability surface smaller while preserving the same local handoff behavior.

## Consequences

- Human intent remains ordinary project Markdown editable with the user's existing tools.
- Ley gains useful editor handoff without a new editor preference, process launcher, shell-string surface, or
  arbitrary-path frontend permission.
- Portable backup/migration becomes reachable from the focused Desktop while retaining the proven core bundle
  format and privacy filtering.
- Exporting into the project tree is refused by design.
- Open-in-editor follows the operating system's file association; Ley does not choose or configure a specific
  editor in this slice.

## Non-goals

This ADR does not restore a Markdown workspace, add in-app editing, export human-authored project files, expose
portable import in the Desktop, open immutable imported snapshots as editable files, or add a generic shell/file
launcher.
