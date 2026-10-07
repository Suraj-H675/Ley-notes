# Project Brain privacy and storage contract

Ley is local-first. Privacy is part of the data model and workflow, not a settings-page decoration.

This document describes the **target Project Brain guarantees** and the current transition boundary. It does not
claim that every target persistence type is already implemented. See [`../LEY.md`](../LEY.md) and the
[`M0 reconciliation`](research/project-brain-m0-reconciliation-2026-10-06.md).

## Local-first default

Core behavior must work without a model or network dependency:

- create/open a Brain;
- deterministic source/repository import;
- retain and inspect approved local evidence;
- store supported agent/session observations;
- search/retrieve local history;
- inspect provenance/applicability;
- perform local review/correction;
- export/erase Ley-controlled state.

Ley does not need a cloud Ley account to own or inspect a Project Brain.

## Product surfaces

The current/target boundary remains:

| Surface | Project-data authority | Network boundary |
| --- | --- | --- |
| Ley Desktop | Explicit local Project Brains + owner-private app state | Sends project content only through explicit configured features/integrations |
| Coding-agent integration | Exact authorized Brain/session and bounded retrieved/captured context | Agent/provider handling applies to deliberately shared context |
| Public website | No private Brain/project authority | Ordinary public website delivery only |
| Legacy browser recovery | Retired same-origin browser data after explicit user action | Migration-only local inspect/export/erase; not current Brain authority |

## Storage ownership

### Private Brain state

Machine-managed Project Brain state belongs in owner-private OS storage. M1 and M2 use the existing single
project-partitioned SQLite authority at schema v13 plus the private content-addressed evidence store for exact
retained SourceVersion representations. This preserves the existing private-file, no-follow, transactional, WAL,
secure-delete, and project-isolation safeguards rather than creating another mutable store.

Private Brain state includes canonical history, review actions, retained private SourceVersions, privacy/egress
state, and lifecycle/deletion state. It should not be spread into multiple mutable stores merely to serve UI views.

### Repo-local `.ley/`

Repo-local state stays small and portable. It may carry association identity and project-owned configuration, but
must not contain:

- the private Brain database;
- conversation/session bodies merely for convenience;
- credentials/tokens;
- embeddings/indexes;
- machine-private absolute paths;
- model-sharing permission;
- secrets;
- hidden permission grants.

A copied `.ley` marker cannot grant access to an existing private Brain.

### User-owned sources

Repository/docs/reference files remain user-owned originals. Importing or referencing them does not transfer
ownership to Ley and Brain erasure must not delete them by default.

Retained SourceVersions are Ley-managed evidence copies/representations and follow Brain retention/deletion rules.

## Source retention truth

Ley must distinguish:

- original source bytes;
- a redacted/normalized/extracted retained representation;
- content identity/hash of that retained representation;
- a locator/path for the live source;
- the observation/import occurrence.

If capture redacts before persistence, the retained evidence is exact only with respect to the redacted
representation. Ley must not describe it as an exact original.

Different Sources with identical retained bytes keep distinct project provenance and privacy identity even if the
underlying blob bytes are deduplicated. Original and stored byte counts are independent because redaction can
expand short credentials. Transformation metadata identifies the retained representation.

M2 import remains local and one-shot. Explicit attachment and import do not enable automatic capture or model
egress. M3 Codex capture requires a separate Desktop/native confirmation bound to the exact Project Brain, working
copy/locator identity, host, and retention mode. The hook, CLI, and MCP paths cannot grant that permission.

M2 import records a bounded per-working-copy observation, excludes secret/generated/dependency material even
when tracked, respects root-local ignore rules, and refuses symlink traversal. Failed eligible scans do not publish
a new successful inventory. Omitted files and files observed missing are distinct from retained historical evidence.

## Capture boundary

Automatic/session/project capture requires an explicit project boundary and applicable consent. Existing useful
controls—ignore rules, file/total bounds, symlink/no-follow handling, secret-oriented exclusions, pattern
redaction, and explicit higher-sensitivity media retention—should be preserved/adapted.

Connection to Codex/Claude does not authorize ambient capture of unrelated projects or automatic historical
scraping. For M3 Project Brains, an absent, revoked, or ineffective Codex capture grant is a no-op and cannot fall
through to the legacy session recorder. Revocation and retention changes fence in-flight writers; working-copy
relocation or identity replacement makes the old grant ineffective. Host capability differences and missing
observations must remain visible. M3 never reads Codex `transcript_path` or hidden reasoning.

Captured agent text/tool activity is untrusted historical evidence. It cannot self-enable writes, adoption,
network access, or provider sharing.

## Model/provider egress

Running a local `codex` or `claude` executable does not prove local inference. If selected Brain content is sent
to a cloud-backed model provider, that is egress.

### Agent retrieval inside an existing host session

When an explicitly connected agent deliberately retrieves Ley context, the returned bounded content becomes part
of that host/provider interaction. Engine-side project, source, and egress restrictions still apply.

### Desktop-triggered optional analysis

This is a new Project Brain capability and therefore a distinct egress event. Before an analysis run Ley must bind
authorization to:

- Project identity;
- exact eligible SourceVersions/representations (or an equivalently inspectable bounded snapshot);
- agent/provider target;
- relevant project/source egress policy;
- the current source/policy generation so stale approval cannot be reused after changes.

The user should be told that provider account quota/API billing may be consumed and that Ley may not know remaining
allowance or exact cost.

Initial authorization should be per invocation. A future broader grant would require its own explicit contract.

`confirm-per-use` sharing remains fail-closed until such a real invocation-scoped confirmation exists; a settings
toggle elsewhere is not equivalent.

## Derivative privacy

Privacy/egress restrictions propagate through derivatives. A restricted SourceVersion must not be laundered into:

- a summary;
- Assertion;
- reviewed claim;
- embedding/vector;
- lexical/search projection;
- graph relation;
- analysis result;
- portable export;

that is then exposed more broadly than its evidence permits.

When Ley cannot prove a derivative is independent of restricted evidence, fail closed or disclose the unresolved
dependency rather than assuming independence.

## Human authority is not permission laundering

Accepting a claim as project intent does not grant:

- filesystem writes;
- tool execution;
- network access;
- capture permission;
- model/provider sharing;
- cross-project access.

Likewise, an imported requirements file is reference evidence until the user explicitly adopts relevant project
intent.

An agent must not be able to manufacture human authority by submitting a payload that merely labels its actor as
`user`. Human adoption/review and permission-grant operations require a human-control boundary distinct from normal
agent write capabilities, with durable provenance of that boundary.

## Project isolation

Every canonical record/query/write must remain attributable to one explicit Project unless a future operation
deliberately models a cross-project source relationship.

Ambient cross-project memory is out of scope. Search or source selection across another Brain must be explicit and
must not silently transfer that Brain's requirements, permissions, or authority into the active project.

## Erasure and detach semantics

Project Brain distinguishes operations with different privacy effects:

| Operation | Intended effect |
| --- | --- |
| Detach working copy | Remove live locator/association; retain Brain history unless separately erased |
| Remove Source from active use | Stop treating it as current input; retained history may remain according to policy |
| Erase Source history | Remove selected Ley-managed SourceVersions and dependent derivatives according to the deletion plan |
| Erase Session | Remove selected Ley-managed session history and dependent derivatives according to the deletion plan |
| Clear projections | Delete rebuildable indexes/summaries/maps without deleting canonical history |
| Erase Brain | Remove Ley-controlled canonical/derived state for that Project while preserving user-owned originals |

Erasure must prevent stale/in-flight hooks, imports, or analysis results from resurrecting data after the erasure
commit. M1 now provides a durable Project generation/tombstone lifecycle guard. Whole-Brain erase fences the
Project as `erasing` before filesystem cleanup; interrupted cleanup remains retryable, and terminal erased IDs are
not automatically reusable. Source/Session erase also advances the Project generation and leaves scrubbed identity
tombstones so stale writes cannot silently reuse the erased identity.

Filesystem evidence and database rows cannot always be removed in one atomic transaction. Interrupted cleanup must
be detectable and safely retryable. Ley must not claim forensic deletion of backups, filesystem snapshots, SSD
remnants, user-created exports, original project files, or downstream provider copies.

## Export/import

A complete Brain backup/export must include all intentionally retained canonical Project state, including uncited
retained Sources/SourceVersions. The preceding cited-only artifact export behavior is insufficient for a Project
Brain backup.

The current portable-continuity v1 format does **not** yet satisfy that complete Project Brain backup contract. M1
fails export closed when retained Project Brain SourceVersions are present rather than silently dropping them.
M2 also rejects exports containing canonical import state that v1 cannot faithfully represent. A
later portable format must carry all retained canonical SourceVersion representations and their provenance before
Ley can call that export a complete Brain backup.

Rebuildable indexes may be omitted when they can safely be regenerated.

Portable import must not automatically activate:

- machine-local absolute paths;
- host trust/configuration;
- capture consent;
- model-sharing/egress grants;
- credentials/secrets;
- stronger human adoption than the exported canonical history actually recorded.

## Current transition stores

At M0, the current runtime still has bounded compatibility/configuration state outside the main SQLite authority,
including repo-local config and migration/privacy ancestry for older vaults/registries/browser data. These are not
new Product Brain stores.

They remain only while they protect a concrete migration, erasure, replay, or restrictive privacy obligation. The
M0 reconciliation records the obligation categories and removal conditions.

## Current egress implementation during transition

The existing focused-continuity runtime currently has project egress states such as `agent-ok`,
`local-model-only`, `confirm-per-use`, and `never-send`. `confirm-per-use` is intentionally fail-closed because the
old runtime has no trustworthy per-invocation confirmation boundary. `local-model-only` is a configured assertion,
not provider attestation.

Those semantics remain safety constraints until the Project Brain analysis/retrieval boundary replaces them with an
equally explicit or stronger contract.

## Website and legacy browser boundary

The normal public website must not mount private Project Brain storage, local agent transports, or browser-local
continuity state.

The separate legacy recovery page is migration-only historical behavior. It must not imply that retired notebook
data becomes current Project Brain knowledge automatically.

## Privacy verification direction

Later milestones must prove at minimum:

- project/source isolation;
- copied repo markers do not grant authority;
- source/capture/model-sharing permissions stay separate;
- no accidental model egress during deterministic import/core use;
- derivative restrictions survive summarization/indexing/review;
- erase-vs-in-flight-write cannot resurrect state;
- exports contain only the selected Brain and intentionally retained canonical content;
- legacy migration cannot widen authority/egress;
- secrets/symlinks/path traversal/malformed local content remain bounded;
- logs/errors/tests do not leak private project bodies unnecessarily.
