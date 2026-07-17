# ADR 0003: Private vault binding registry

- Status: Accepted
- Date: 2026-07-17

## Context

An initialized project has a stable `projectId`, while durable agent memory belongs in a user-selected Ley filesystem vault. Storing an absolute vault path in the repository would disclose private machine layout, make `.ley/` non-portable, and create noisy per-machine changes. Deriving a vault from a project path would break after moves and could select the wrong location without consent.

The CLI, desktop app, and later local MCP server must resolve the same binding without an account, network service, running GUI, or duplicated configuration.

## Decision

Store bindings in a private OS-local registry named `bindings-v1.json` under Ley's application configuration directory. The CLI uses the same application identifier as Tauri, `app.leynotes.desktop`, so both runtimes resolve the same directory:

- Linux: the XDG configuration directory, normally `~/.config/app.leynotes.desktop/`;
- macOS: the user's Application Support directory;
- Windows: the user's roaming application-data directory.

The registry contains only:

```json
{
  "schemaVersion": 1,
  "bindings": {
    "prj_0123456789abcdef0123456789abcdef": "/canonical/path/to/vault"
  }
}
```

It never stores a project root, project name, note, captured content, transcript, credential, agent identity, or provider data. Entries are serialized in project-ID order for deterministic inspection.

`ley bind [project] --vault <vault>` creates or explicitly replaces one binding. `ley binding [project]` resolves it. `ley unbind [project]` explicitly removes it. `ley binding [project] --vault <vault>` validates and returns a temporary override without changing the registry. Future commands that consume a vault will use the same resolver.

Both project and vault must already be directories. The project is discovered through its validated `.ley/` identity. Vault paths are canonicalized before storage. Moving a project preserves its binding because the stable project ID is the key. Moving a vault makes resolution fail with a rebind instruction rather than guessing a replacement.

Every reader and writer takes an advisory cross-process lock on a separate private lock file. Mutations read, validate, update, and atomically replace the registry while holding that lock, preventing lost concurrent updates and partial JSON. The registry rejects unknown fields, unsupported versions, invalid project IDs, relative/non-UTF-8 vault paths, non-regular files, symlinks, and files over 1 MiB. Newly created configuration directories use owner-only permissions on Unix; registry and lock files use owner read/write permissions, and Ley refuses either file if group or other access is later enabled.

## Evidence

- Obsidian keeps vault data in local folders while application-global state lives in the operating system's application-data location: <https://obsidian.md/help/data-storage>.
- The XDG Base Directory specification defines per-user configuration storage and requires absolute paths: <https://specifications.freedesktop.org/basedir-spec/latest/>.
- Tauri derives its app configuration directory from the platform configuration directory and bundle identifier: <https://v2.tauri.app/reference/javascript/api/namespacepath/#appconfigdir>.
- Microsoft's Known Folder system defines per-user roaming application data: <https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid>.
- Apple defines Application Support as the location for app-created support files: <https://developer.apple.com/documentation/foundation/url/applicationsupportdirectory>.
- Rust's standard `File` locks are cross-platform advisory locks: <https://doc.rust-lang.org/std/fs/struct.File.html#method.lock>.
- `atomic-write-file` provides atomic replacement on supported Unix and Windows filesystems: <https://docs.rs/atomic-write-file/0.3.0/atomic_write_file/>.

## Consequences

- Repository-local `.ley/` remains portable and path-free.
- CLI, desktop, hooks, and MCP share one implementation and one binding.
- A copied project identity intentionally resolves to the same binding. Duplicate-identity detection needs a separate local observation index; the registry will not weaken privacy by storing project roots.
- Application config is private by operating-system access controls, not encrypted at rest.
- Deleting or corrupting the registry loses only machine-local bindings, not vault content. Users can recover by explicitly rebinding.
- Browser-only PWA mode cannot expose this registry to external local agents. Agent integration requires the desktop/local engine and a real filesystem vault.
