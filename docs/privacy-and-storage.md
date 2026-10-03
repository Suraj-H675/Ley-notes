# Local storage and data boundaries

Ley is local-first. The public website serves product information and static assets only; it has no project-memory backend and no browser edition of the application.

## Product surfaces

| Surface | Local authority | Network boundary |
| --- | --- | --- |
| Ley Desktop | User-selected local projects plus Ley's owner-private application state | Context leaves only through configured integrations/explicit network features |
| Public website | No project or knowledge data in the normal website runtime | Ordinary static website delivery only |
| Legacy browser recovery page | Explicit same-origin access to the retired `ley-notes` IndexedDB only after user action | No upload; local inspect/export/erase only |

The former browser workspace/PWA, browser-folder mode, and browser-local IndexedDB vault are retired. ADR 0097
closes the last browser-data compatibility gap: the normal website never opens that database, while a separate
`/legacy-recovery.html` migration page can explicitly inspect, export, or erase same-origin historical browser-local
data without uploading it. The dead Dexie runtime/schema island is no longer needed to preserve those bytes.

## Current and target storage

During migration, two storage generations coexist:

### Current implementation being migrated

- a small repository-local `.ley/` project identity/capture configuration;
- owner-private OS configuration registries for current authority plus retained pre-cutover project bindings;
- filesystem Agent Memory data only as an existing legacy migration source; current projects do not create new
  bound vaults;
- historical browser IndexedDB data may still exist in old browser profiles until the user exports/erases it through
  the migration-only recovery page; it is no longer opened by the product runtime.

These stores remain supported only as long as required to preserve existing data and benchmark/migrate the current engine.

### Target implementation

Machine-managed continuity metadata moves toward owner-private SQLite, partitioned by project. Large immutable evidence may use content-addressed private files. Rebuildable lexical/vector indexes remain disposable.

The repo-local `.ley/` directory stays small and portable. It must not contain conversations, generated memory, credentials, embeddings, machine-specific private paths, or raw transcripts.

Portable continuity export/import is separate from browser-notebook recovery. Current continuity uses its validated
SQLite/evidence bundle; the retired browser notebook exports a lossless archival JSON only and is never imported as
agent memory by default.

## Agent/model egress

Ley does not independently upload project files, sessions, queries, or indexes.

When a configured cloud coding agent receives Ley context, that selected context becomes part of the request handled by that agent provider. Automatic lifecycle/task briefing, when enabled, must therefore be visible to the user and governed by the same egress boundary as explicit retrieval. Product copy must not imply that cloud context is sent only after a per-turn manual retrieval if automatic injection is configured.

The reset keeps these principles:

- egress is explicit/configured rather than ambient;
- project scope is fixed and inspectable;
- broad historical memory cannot bypass a more restrictive known source policy;
- agent-facing tools cannot silently grant themselves new human authority;
- bounded outputs expose provenance but not unnecessary absolute machine paths.

Current project-level agent-context egress is user-inspectable in Desktop **Capture & privacy** and through
`ley egress`. `agent-ok`, `local-model-only`, `confirm-per-use`, and `never-send` remain the enforced vocabulary.
Desktop uses the same mixed-version transition authority as the CLI and rejects stale project-identity or policy
writes before mutation.
`confirm-per-use` remains intentionally fail-closed until a trustworthy confirmation decision can be scoped at the
actual retrieval boundary; Desktop does not simulate that with an unrelated settings dialog.

The existing detailed egress registries/Context Mount/Scope/Policy Bundle implementation is migration-era architecture. Its safety lessons survive; its exact object hierarchy does not automatically survive.

## Evidence capture

Historical agent memory is untrusted evidence, not executable instruction. Capture must remain bounded and redact recognizable credentials before persistence.

The focused product should prefer compact structured continuity records and exact evidence references over whole-project duplication. Git/live project tools already provide current source. Retain immutable source snippets/blobs only where historical evidence cannot be reconstructed safely or the user explicitly chooses that retention.

Image/multimodal evidence and the current Full Evidence mode remain optional migration-era capabilities and must re-earn first-class product complexity through realistic evaluation.

## Optional semantic retrieval

The bundled local Model2Vec experiment is deferred from the focused product by ADR 0094. Canonical native Search and transition cross-kind ranking now use the deterministic lexical baseline and do not inspect the optional model cache. A pre-cutover legacy artifact fallback may still exercise retained semantic compatibility internals. The Desktop and CLI no longer download or install the model.

Retained core semantic/index code is compatibility/research machinery rather than a current product feature. Existing cache files are derived public-model data and are left untouched. Model-assisted canonical retrieval should return only after a native-state retrieval/task ablation shows material value over the lexical baseline under the same trust, revision, egress, and budget constraints.

## External network connectors

The current public-GitHub connector uses an explicit fixed-origin, bounded fetch path and stores no authentication token. It is safe as implemented within its stated boundary, but provider-specific connectors are no longer assumed to be core product functionality because coding hosts already provide strong GitHub integrations.

If the connector is retired, existing stored connector evidence must be migrated/exported or explicitly erased; it must not be silently orphaned.

## Erasure and migration

Logical deletion must remove Ley-controlled continuity state for the selected project without deleting the user's source repository. It is not a forensic wipe of backups, snapshots, SSD remnants, or external model/provider copies.

The SQLite migration must prove:

- import from the existing JSON/filesystem stores without losing retained user data;
- transactional project/session deletion;
- crash/concurrent-writer recovery;
- portable export/import;
- cleanup of unreferenced content-addressed evidence;
- no resurrection of deleted data through stale derived indexes.

## Retired notebook filesystem surface

The old desktop note/Canvas filesystem engine and watcher are no longer executable product surfaces. Its
capability-rooted no-follow implementation and six-lane hosted confinement evidence from run `36151215222`
remain historical evidence for the retired implementation, not a current Ley security boundary.

User-owned Markdown, Canvas, attachments, and other files in a former Ley vault remain ordinary user data and
are not deleted when Ley continuity is erased or the notebook implementation is removed. The active legacy
migration boundary is narrower: when an older project needs its moved Agent Memory vault, Ley accepts the
selected directory only if existing captured memory validates for that exact project before any ingest or
binding change.

## Website boundary

The normal website does not need project storage, service workers, PWA installability, directory handles, local
agent processes, or IndexedDB knowledge state. Keeping that runtime intentionally boring from a data-authority
perspective is a security and maintenance advantage, not a missing feature. The sole exception is the separately
built `/legacy-recovery.html` migration utility from ADR 0097: it performs no database access until the user chooses
Inspect, can access only same-origin browser state, and never participates in ordinary website or Desktop product
behavior.
