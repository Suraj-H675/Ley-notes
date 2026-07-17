# Agent memory threat model

Status: project initialization, capture preview, private vault binding, and deterministic artifact-ingestion boundary; expand before AST projections and MCP writes.

## Assets

- User notes, attachments, canvases, and agent memories in the selected vault
- Project source, documentation, local Git state, and session evidence
- Capture consent and ignore rules
- Memory trust, provenance, corrections, and project isolation

## Trust boundaries

1. User-selected filesystem vault
2. Explicitly initialized project root
3. Private OS-local project-to-vault binding registry
4. Local CLI/desktop/MCP process
5. Agent host and its lifecycle adapter
6. Cloud model provider when the user intentionally retrieves context

Repository content, transcripts, tool output, generated summaries, MCP arguments, and imported memory are untrusted data. They are not agent policy or executable instructions.

## Initial threats and required controls

| Threat | Initial control | Required future proof |
| --- | --- | --- |
| Scanning outside the project | Only project-relative approved roots; reject root, prefix, and parent components; refuse symlinked metadata/approved roots; never follow content symlinks; ingest through a sandboxed directory capability with final symlink following disabled | Cross-platform adversarial path fixtures and continuous dependency review |
| Metadata memory exhaustion | Regular metadata files are limited to 1 MiB before reading | Fuzz malformed and boundary-sized metadata |
| Partial/corrupt initialization | Stage the complete `.ley/` directory and atomically rename | Crash/fault-injection test |
| Capture escalation on repeat init | Existing identity and policy are read without rewriting | Explicit, reviewed policy-update command |
| Raw transcript capture without consent | Only Full Evidence may enable it | UI/CLI confirmation and adapter enforcement |
| Secret collection | Credential-container ignore defaults; pre-persistence redaction for key blocks, credential assignments/URLs, and known token prefixes; hash redacted rather than matched text | Provider-pattern corpus, contextual entropy detector, false-positive/negative evaluation, output rescanning |
| Legitimate code hidden by broad ignores | No wildcard `secret`/`credential` terms | Git-compatible matcher conformance fixtures |
| Machine-dependent capture | Parent/global Git excludes and generic `.ignore` files are disabled; output paths are sorted | Cross-platform golden preview fixtures |
| Project identity collision | Random UUID-backed `prj_` identifier | Collision test and duplicate-binding diagnosis |
| Repository leaks a vault location | `.ley/` contains no vault path; binding registry is outside the repository and contains no project root/name | Packaging and repository-content regression tests |
| Binding registry disclosure | Minimal project-ID-to-vault-path data; owner-only Unix creation permissions; OS per-user config directory | Windows ACL and macOS protection verification |
| Binding corruption or concurrent lost update | Strict schema/size validation; regular non-symlink files; advisory cross-process lock; atomic replacement | Crash/fault injection and multi-process contention tests |
| Stale vault location | Canonical path at bind time; missing target fails with an explicit rebind requirement | Removable-volume and permission-change tests |
| Ambient or guessed vault selection | Explicit bind/rebind; temporary override is validated and never persisted | Desktop consent UI and host-adapter end-to-end tests |
| Recursive self-ingestion | Refuse a vault equal to or nested below the project | Overlap fixtures on case-insensitive filesystems |
| Artifact-store path replacement | Capability-scoped no-follow directories, private permissions, immutable collision checks, atomic manifest replacement | Cross-platform concurrent attacker and removable-volume tests |
| Partial or duplicate ingestion | Per-project cross-process lock; content-addressed blobs; deterministic snapshot IDs; identical ingestion is a no-op | Process-kill fault injection at every durable write |
| Deleted evidence becomes unverifiable | Immutable snapshot manifests and content blobs remain after current-source deletion | Reviewed retention/deletion policy and storage-budget UI |
| Cross-project memory access | Project ID required at every future storage/query boundary | Multi-project adversarial tests |
| Prompt injection in stored material | Stored text remains untrusted evidence | Typed context envelope and agent-facing injection warnings |
| Memory poisoning | No inference becomes trusted automatically | Provenance, review, correction, corroboration, supersession |
| Local MCP compromise | No MCP server in this phase | Exact-command consent, stdio default, scoped tools, no ambient home access |

## Explicit non-claims

- Local storage is not encryption at rest; device/vault encryption is a separate choice.
- When a user intentionally supplies Ley context to a cloud agent, that context is visible to that provider.
- Memory and citations can reduce unsupported guessing; no system can guarantee that an LLM never hallucinates.
