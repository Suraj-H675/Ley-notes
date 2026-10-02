# ADR 0097: Retire browser IndexedDB through explicit same-origin recovery

**Status:** Accepted — 2026-10-02

## Context

Ley's retired browser notebook once stored human-authored pages, attachments, and sparse revision history in the
`ley-notes` IndexedDB database. The focused product no longer has a browser workspace. Before this ADR, the only
remaining Dexie code was an unreferenced schema/opening island plus one compatibility test; neither the website nor
Desktop imported it in production.

Keeping that source did not actually preserve user data. IndexedDB persists independently in the browser profile,
while the dead code merely kept a schema dependency in the repository. Removing the code without a recovery path,
however, would leave historical same-origin data inaccessible to users who still need it.

Historical vault-isolation code from July 2026 establishes the narrow authority rule needed for recovery:

- `pages`, `assets`, and `revisions` were the authoritative browser-local set while browser-local mode was active;
- when switching to a filesystem-backed vault, Ley stashed those three stores into `browserLocalPages`,
  `browserLocalAssets`, and `browserLocalRevisions` before the active stores became a filesystem projection; and
- blocks, links, and tags were rebuilt/disposable projections rather than part of the browser-local stash.

Human-authored notebook data is not agent continuity. Importing it into the new continuity SQLite store would grant
the wrong semantics and authority merely to make migration convenient.

## Decision

### Separate recovery page

The public build includes `/legacy-recovery.html` as a separate Vite entry and JavaScript chunk. The normal landing
page may link to it for discoverability, but does not import its code. The recovery page performs no IndexedDB access
on load; the user must explicitly choose **Inspect legacy browser data**.

This is a migration-only exception to the normal website storage boundary. It reads only same-origin local browser
state and never uploads it.

### Non-creating, non-upgrading inspection

Recovery uses the browser's raw IndexedDB API rather than Dexie. It never supplies a schema version. When supported,
`indexedDB.databases()` first proves whether `ley-notes` exists. On browsers/privacy modes where enumeration is not
available, a fallback open aborts the version-change transaction if `oldVersion == 0`, so inspecting an origin with
no Ley database does not create one. Existing databases are opened at their current version without upgrading them.

### Authority selection

Recovery interprets the old `active-data-kind` marker conservatively:

1. `browser-local` + non-empty active stores: active `pages/assets/revisions` are the selected proven
   browser-local data because they may contain edits made after a previous snapshot was restored.
2. `filesystem:*`: a non-empty `browserLocal*` snapshot is selected; active tables are a retired filesystem cache and
   are not mislabeled or exported as browser-local authority. If no snapshot exists, the UI reports
   `filesystem-cache-only` rather than fabricating recoverable notebook data.
3. Missing/legacy marker + non-empty snapshot: the dedicated snapshot is retained. If active stores are also
   non-empty, the archive records **both** candidates and marks authority `ambiguous`; it does not merge them,
   silently discard one, or tell a future importer which is newer.
4. Missing/legacy marker + active data only: active stores are exported as `legacy-active-unknown` with ambiguous
   authority, preserving old pre-isolation data without claiming stronger provenance than Ley can prove.
5. Any present marker other than `browser-local` or `filesystem:*` fails closed. Recovery does not guess the meaning
   of a future/unknown authority mode.

Marker plus candidate stores are read in one readonly IndexedDB transaction so the authority decision and records
come from one database snapshot rather than separate racing reads. Version-1 databases simply lack the snapshot
stores; recovery treats those absent stores as empty and never upgrades the database to create them.

### Export format

The recovery page downloads schema-v1 JSON with:

- all selected Page records, including soft-deletion and frontmatter fields;
- all selected Revision records;
- all selected Asset metadata, the Blob MIME type separately, exact byte length, and exact binary Blob bytes encoded
  as base64;
- database version, old active-data marker, source-selection label, and export timestamp; and
- an `authority` label plus one or more explicitly sourced datasets when historical state is ambiguous.

The archive is labeled `ley-legacy-browser-local-archive` and explicitly states that it contains human-owned retired
notebook data, not trusted Ley continuity. There is no automatic import into current Ley. A future converter may be
built only if a real migration need exists and must preserve that distinction.

The historical Page schema permits arbitrary frontmatter values at the type boundary. JSON would silently coerce or
drop some JavaScript values, so export recursively validates page/revision/asset metadata and stops with a clear
error for non-finite numbers, BigInt, undefined, non-plain objects, or other non-JSON-safe values rather than falsely
claiming a lossless archive. Asset binaries are encoded sequentially, and the UI reports selected binary bytes before
base64/JSON overhead so unusually large exports are visible before the user starts the download.

### Explicit erasure

The same page can delete the entire retired `ley-notes` IndexedDB only after the user types
`ERASE LEGACY DATA`. Browser `deleteDatabase()` may be blocked by another same-origin tab and later complete when
that connection closes, so a blocked request remains visibly pending rather than being reported as cancelled.

Erasure does not claim forensic deletion of browser/profile backups, sync copies, downloaded archives, source
folders, or current Desktop continuity.

### Repository cleanup

The unreferenced Dexie singleton, notebook schema declarations, and compatibility-only Dexie test are removed.
`dexie` is removed from production dependencies. `fake-indexeddb` remains a test-only dependency for deterministic
recovery/erasure fixtures.

## Consequences

- The ordinary public website and Desktop have no IndexedDB notebook dependency or runtime access.
- Historical browser-local data remains recoverable only on the same origin where the browser stored it, which is
  an IndexedDB security property rather than a Ley account/cloud migration mechanism.
- Recovery cannot accidentally upgrade or initialize an empty legacy database.
- Filesystem-cache projections are not promoted into browser-local authority.
- The retired notebook's final live code is a small, explicit migration utility rather than a hidden compatibility
  runtime.

## Non-goals

This ADR does not restore the browser notebook, import notes into agent memory, recreate Markdown ZIP import/export,
sync browser data, inspect other origins/profiles, or infer whether an ambiguous historical active table is newer
than its stashed snapshot.
