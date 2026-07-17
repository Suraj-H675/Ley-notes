# ADR 0012: Private project observation catalog

- Status: Accepted
- Date: 2026-07-18

## Context

Ley's vault-binding registry deliberately maps stable project IDs to filesystem vaults without storing project roots or names. That keeps repository locations private, preserves project moves, and prevents the binding mechanism from becoming a device scanner. It also means the registry cannot power the required Projects → project overview experience.

Storing a recent project path in webview `localStorage` supports only one project, cannot be shared safely with the CLI, and provides no duplicate-identity protection. Silently searching the device for `.ley/` folders would violate Ley's explicit-root boundary. A copied repository can also contain the same `.ley/project.json` identity as its source; allowing both live roots to resolve one vault namespace risks cross-project memory leakage.

## Decision

Maintain a separate private OS-local catalog named `projects-v1.json` beside, but not inside, `bindings-v1.json`. The catalog contains only initialized projects that the user, desktop app, CLI, hook, or MCP process explicitly opens through Ley:

```json
{
  "schemaVersion": 1,
  "projects": {
    "prj_0123456789abcdef0123456789abcdef": {
      "rootPath": "/canonical/path/to/project",
      "lastOpenedAtUnixMs": 1784320000000
    }
  }
}
```

It stores no project name, vault path, note, source content, session text, agent identity, provider data, or generated memory. Project names and capture modes are read from the selected project's validated `.ley/` metadata when the desktop Projects view is opened. Memory counts come from the already bound local vault and remain project-scoped.

All binding operations claim the project's identity in this catalog before resolving memory. If the same stable project ID is already claimed by another available initialized root, Ley refuses the second root and asks for a new `.ley` identity instead of allowing two folders to share one memory namespace. A moved project can replace an observation once its previous root no longer claims that identity. A reinitialized folder replaces its stale observation by canonical path.

The catalog is bounded to 200 returned entries and sorted by most recent explicit use. Missing project folders remain visible as unavailable instead of being guessed or silently deleted. Removing a project from the Projects view deletes only this device-local observation; it does not unbind the project, alter `.ley/`, or delete vault memory. Selecting the project again restores the observation.

The catalog uses its own cross-process advisory lock, strict schema validation, deterministic project-ID ordering, atomic replacement, a 1 MiB limit, symlink/non-regular-file rejection, and owner-only directory/file permissions on Unix. The binding registry remains path-free and keeps its existing format.

## Consequences

- Ley gains a real multi-project entry point without scanning neighboring folders.
- Desktop, CLI, hooks, and MCP contribute to one recent-project history through the shared engine.
- Project-root paths are explicit private device metadata and are disclosed as such in the interface and privacy documentation.
- Copied initialized repositories fail closed before their agent-memory namespaces can collide.
- Deleting the catalog loses only the recent Projects list. Repository identity, vault binding, captured evidence, and session memory remain intact.
- Browser/PWA mode cannot access the catalog and continues to show the honest desktop-only Agent Memory boundary.
