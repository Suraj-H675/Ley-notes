# Local storage and data boundaries

Ley is local-first. The public website serves product information and static assets only; it has no project-memory backend and no browser edition of the application.

## Product surfaces

| Surface | Local authority | Network boundary |
| --- | --- | --- |
| Ley Desktop | User-selected local projects plus Ley's owner-private application state | Context leaves only through configured integrations/explicit network features |
| Public website | No project or knowledge data | Ordinary static website delivery only |

The former browser workspace/PWA, browser-folder mode, and browser-local IndexedDB vault are retired. Legacy IndexedDB stores remain inert in the migration tree only so old local data is not destructively dropped before the SQLite migration defines explicit handling.

## Current and target storage

During migration, two storage generations coexist:

### Current implementation being migrated

- a small repository-local `.ley/` project identity/capture configuration;
- owner-private OS configuration registries for project bindings/authority;
- filesystem Agent Memory data associated with the current binding;
- Dexie projection/cache data used by the legacy desktop note workspace.

These stores remain supported only as long as required to preserve existing data and benchmark/migrate the current engine.

### Target implementation

Machine-managed continuity metadata moves toward owner-private SQLite, partitioned by project. Large immutable evidence may use content-addressed private files. Rebuildable lexical/vector indexes remain disposable.

The repo-local `.ley/` directory stays small and portable. It must not contain conversations, generated memory, credentials, embeddings, machine-specific private paths, or raw transcripts.

Portable export/import is a requirement of the migration, but it should be designed for the continuity store rather than inherited from the retired browser notebook ZIP workflow.

## Agent/model egress

Ley does not independently upload project files, sessions, queries, or indexes.

When a configured cloud coding agent receives Ley context, that selected context becomes part of the request handled by that agent provider. Automatic lifecycle/task briefing, when enabled, must therefore be visible to the user and governed by the same egress boundary as explicit retrieval. Product copy must not imply that cloud context is sent only after a per-turn manual retrieval if automatic injection is configured.

The reset keeps these principles:

- egress is explicit/configured rather than ambient;
- project scope is fixed and inspectable;
- broad historical memory cannot bypass a more restrictive known source policy;
- agent-facing tools cannot silently grant themselves new human authority;
- bounded outputs expose provenance but not unnecessary absolute machine paths.

The existing detailed egress registries/Context Mount/Scope/Policy Bundle implementation is migration-era architecture. Its safety lessons survive; its exact object hierarchy does not automatically survive.

## Evidence capture

Historical agent memory is untrusted evidence, not executable instruction. Capture must remain bounded and redact recognizable credentials before persistence.

The focused product should prefer compact structured continuity records and exact evidence references over whole-project duplication. Git/live project tools already provide current source. Retain immutable source snippets/blobs only where historical evidence cannot be reconstructed safely or the user explicitly chooses that retention.

Image/multimodal evidence and the current Full Evidence mode remain optional migration-era capabilities and must re-earn first-class product complexity through realistic evaluation.

## Optional semantic retrieval

The current implementation can explicitly install a pinned local Model2Vec model from Hugging Face. Model files are public, checksum-verified, and inference stays local; project text and queries are not included in the download request.

The reset treats this bundled model as optional/deferred. The storage/network boundary remains valid while the feature exists, but the model stays in the focused product only if realistic retrieval/task ablation shows material value over the lexical baseline.

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

## Native filesystem safety

While the legacy note workspace remains runnable, native relative-path operations stay inside the selected vault through capability-rooted no-follow traversal rather than textual `..` rejection alone. Targeted reads/writes/renames/trash operations and recursive scans refuse or skip symlink/reparse redirection, including the reserved `attachments`, `canvases`, and `.trash` roots. Hosted run `36151215222` passed these confinement attacks on Linux/macOS/Windows x64+ARM64, including real Windows directory junctions. That is evidence for the tested hosted filesystems/reparse shapes, not a universal guarantee for every filesystem implementation.

## Website boundary

The website does not need project storage, service workers, PWA installability, directory handles, local agent processes, or IndexedDB knowledge state. Keeping the website intentionally boring from a data-authority perspective is a security and maintenance advantage, not a missing feature.
