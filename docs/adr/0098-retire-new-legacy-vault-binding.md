# ADR 0098: Retire new legacy vault binding

**Status:** Accepted — 2026-10-02

## Context

Ley's focused product already creates fresh projects as native-born continuity in owner-private SQLite/CAS state.
The private project-to-vault binding registry survives only because pre-cutover projects may still need an existing
filesystem Agent Memory vault as a migration source.

Two growth paths contradicted that boundary:

- `ley bind PROJECT --vault EMPTY_DIR` could create a new persisted legacy relationship before the first capture; and
- transition ingestion could capture current live source into an unfenced legacy vault immediately before importing
  it into native continuity.

The Desktop had the same first case for an initialized-but-unbound project: selecting an empty directory could turn
it into a new legacy vault. This also made a failed first native capture awkward because Desktop recorded the
native-born origin only after capture succeeded.

Nothing about current Ley requires those writes. Current source can be captured directly into native continuity;
legacy vaults exist to preserve and migrate historical evidence, not to become new storage destinations.

## Decision

### Fresh projects are native before first capture

After an explicitly approved Desktop initialization creates `.ley`, Ley records the project as native-born before
attempting its first artifact capture. If that capture fails or its reviewed plan becomes stale, later inspection
returns **Needs capture** with native storage rather than falling into the legacy-unbound flow. CLI initialization
already followed this native-born rule.

### Unbound is an unknown pre-cutover compatibility state

An initialized project with no persisted binding and no native-born/native read authority is not guessed to be
"fresh." Ley has insufficient evidence to make that claim. The Desktop labels this state as historical reconnect,
and the local user must select an existing legacy vault whose captured memory validates for the exact project ID.

Ley does not infer history from creation time, pathname, or repository age. If no valid legacy vault is available,
the state remains fail-closed; designing an explicit abandon/reset flow is separate work because it may discard
historical continuity.

### `ley bind` is reconnect-only

`ley bind PROJECT --vault VAULT` is retained only as a compatibility reconnect operation:

- an unbound pre-cutover project may bind only to a vault that already validates as captured memory for that exact
  project;
- an empty, corrupt, or different-project vault fails before binding mutation or capture;
- a persisted binding whose old path is unavailable may be rebound to a validated moved copy, including after native
  cutover; and
- a project registered as native-born cannot acquire a new legacy binding.

`LegacyCutover` artifact authority does not by itself block reconnect. That state can legitimately exist if the
historical vault was validated and native artifact cutover committed but the process stopped before the private
binding registry was persisted. A retry may reconnect that same valid historical vault; `NativeBorn` authority still
rejects legacy binding.

An already available persisted binding is not silently replaced with another path.

### Explicit `--vault` overrides cannot manufacture legacy state

Temporary CLI overrides remain useful for bounded compatibility reads and recovery, but every override must already
validate as existing captured memory for the exact project before the requested operation runs. Passing an empty
directory to `ingest`, session/learning commands, Search/Resume, or another override-capable route cannot initialize
legacy memory.

### Transition artifact ingestion is import-only on the legacy side

When a real legacy project first crosses artifact authority into native continuity, Ley now:

1. validates/opens the existing historical artifact store;
2. fences that legacy artifact writer read-only;
3. imports the existing historical snapshot into native continuity; and
4. captures current live project source directly into native continuity.

It does **not** first recapture current source into the legacy vault. A registered native-born project routed through
a transition helper stays native and never touches the supplied legacy path.

The low-level legacy writer remains in `ley-core` for historical format validation/migration fixtures. It is not a
current CLI/Desktop creation path and should not be treated as product setup guidance.

## Consequences

- Normal setup is `init` → reviewed/native `ingest`; users no longer choose a filesystem Agent Memory vault.
- Legacy bindings stop growing through current product flows while moved-vault recovery remains available.
- Historical vault contents are preserved as migration evidence instead of being rewritten from today's repository.
- Native first-capture failure no longer routes users into legacy migration UX.
- A genuinely old unbound project without its historical vault remains unresolved rather than being silently reset.

## Non-goals

This ADR does not delete the binding registry, remove compatibility reads for already-bound projects, erase historical
vaults, create an automatic "start fresh" path for ambiguous old projects, or retire all legacy JSON/session formats.
