# Agent memory threat model

Status: project initialization, capture preview, private vault binding, deterministic artifact ingestion, cited project-graph projection, fixed-project MCP retrieval, opt-in structured session capture, stable-field Codex/Claude/Gemini lifecycle adapters, and evidence-backed learning review.

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
| Parser/model executes repository content | Local Tree-sitter parsing only; no compiler, package manager, build script, language server, or model is invoked during graph projection | Malicious grammar-input corpus, parser dependency review, and resource benchmarks |
| Syntactic name promoted into a false fact | Calls and references target unresolved name nodes; no heuristic cross-file resolution is emitted as deterministic | Resolution evaluation corpus before adding inferred edges |
| Git command expands capture scope | Porcelain-v2 tracked status only; untracked files, optional locks, fsmonitor, and untracked cache disabled; retain changes only for already-approved artifact paths; 8 MiB output cap | Adversarial Git config/worktree/submodule fixtures on every supported OS |
| Graph corruption or dangling citations | Strict schema/size/identity/endpoint/citation checks; artifact snapshot written before graph; immutable graph snapshot verified before reads | Process-kill matrix across artifact/graph current-pointer updates |
| Historical graph paired with wrong evidence | Bounded history entries pin one graph snapshot to its artifact snapshot; both immutable files, project identity, cross-references, and hashes are verified under a shared lock before use | Corrupt-index, concurrent-ingestion, and retention-deletion fault matrix |
| Graph inspector becomes arbitrary file reader | Source reads require an exact citation already attached to the selected graph node or edge, verify the matching artifact/hash, and return only a bounded redacted immutable excerpt; Minimal has no source fallback | Forged, widened, cross-node, cross-snapshot, and encoded-path citation corpus |
| Graph leaks source or machine paths | Project-relative citations only; external name nodes contain syntax-level names; no project/vault path persisted | Schema inspection and sensitive-path fixtures |
| Deleted evidence becomes unverifiable | Immutable snapshot manifests and content blobs remain after current-source deletion | Reviewed retention/deletion policy and storage-budget UI |
| Cross-project memory access | One project/binding is fixed at MCP process start; tools have no project/vault selector; resource URI includes only the stable project ID | Actual-host multi-project adversarial tests |
| Prompt injection in stored material | Typed results mark content `untrusted-project-evidence`, repeat a non-instruction warning, and never promote retrieved text to server instructions | Injection corpus across every host adapter |
| Stale source presented as current | Every result identifies immutable artifact/graph snapshots and capture time and says `liveSourceChecked: false` | Source-change UX, re-ingestion hooks, and stale-result evals |
| Oversized MCP context | Queries, result counts, token estimates, snippets, evidence ranges, session checkpoints/text, graph depth, and visited nodes are capped; every serialized tool result has a 256 KB hard limit | Large-graph and long-session latency corpus |
| MCP argument path traversal | Evidence reads accept only an exact current-manifest artifact path and use no-follow capability reads | Encoded/platform-specific traversal corpus |
| MCP output leaks machine paths | Results contain project-relative citations and stable IDs; tool errors are sanitized; project/vault paths never enter schemas | Cross-platform output snapshot scanning |
| Duplicate, reordered, or stale session writes | Deterministic event IDs and request fingerprints; exact retries replay; changed reuse fails; project lock; contiguous sequence validation; desktop renames match the inspected event count under the lock | Multi-process hook retry, concurrent rename, and process-kill matrix |
| Unstable transcript coupling | First-party adapters ignore `transcript_path`; automatic fallback uses only documented stable final-response fields; no transcript parser ships | Host-version fixtures and vault scans for transcript paths/content |
| Hook payload selects another project | CLI command path/default working directory is authoritative; payload `cwd` and transcript paths never select project or vault | Cross-project forged-payload exercise |
| Global hook mutates unrelated repositories | Missing identity, binding, or available vault returns host-valid `{}` without initialization, scanning, binding, ingestion, or session creation | Real non-Ley-directory hook invocation |
| Partial session projection | Immutable event-per-file source; atomic JSON and Markdown projections; reads verify and replay events | Fault injection before and during every durable write |
| Concurrent session data loss | Exclusive advisory lock covers validation, append, replay, and projection replacement | Cross-process stress tests on supported filesystems |
| Session secret capture | Bounded fields and collections; local credential redaction before persistence; per-event redaction metadata; no automatic raw transcript | Host-specific secret corpus and false-negative measurement |
| Session path or symlink escape | Stable ID-only store paths; no-follow capability directories and files; touched paths must exactly match the current artifact manifest | Encoded path and concurrent replacement corpus |
| Uncited session artifact claims | Touched paths become current-snapshot citations with post-redaction content hash and line range | Source-change and deleted-artifact scenarios |
| Session history corruption | Strict schemas, IDs, request fingerprints, timestamps, record IDs, limits, and event ordering are verified on every read | Property tests and malformed-event fuzzing |
| Stored session prompt injection | Session packs use an `untrusted-agent-memory` boundary and repeat a non-instruction warning; MCP never promotes memory into server instructions | Injection corpus across goals, decisions, problems, final responses, and handoffs |
| Memory poisoning or stale user approval | Every proposal cites an existing session record and starts review-required; actor/provenance must agree; only user confirmation grants trust; desktop corrections preserve complete evidence and reset trust; correction/review actions match the inspected event count under the writer lock | Provenance, concurrent correction/review rejection, correction, corroboration, supersession, forged-authority corpus |
| Unreviewed memory enters human notes | Promotion is visible only for a fully rendered current trusted lesson; it copies the reviewed guidance plus bounded identifiers rather than evidence notes, records trust/version provenance in YAML, refuses unrelated title collisions, and deduplicates by learning ID | Promotion visibility, deterministic Markdown draft, collision, rename, and replay tests |
| Stale trusted lessons | Artifact citations pin post-redaction hashes; changed/deleted sources return trusted lessons to the review inbox | Modify, rename, and delete cited artifacts across ingestion |
| Learning history loss or forgery | Immutable deterministic events, request fingerprints, contiguous replay, atomic projections, private file modes, symlink rejection, and a cross-process writer lock | Retry conflicts, corruption, interrupted projection, and concurrent-proposal tests |
| Ambient MCP write authority | Session write routes are absent by default and require `--allow-session-writes` at process startup; stored content cannot enable them; writes are append-only and idempotent | Actual-host permission UX and adversarial injection tests |
| Agent-controlled session naming | Rename is absent from MCP, requires an explicit local user action and reason, preserves original history and stable IDs, and redacts/bounds both fields | Adversarial stored-name and stale-inspector corpus |
| Agent self-approval | `--allow-learning-proposals` adds only one agent-authored, review-required route; MCP has no confirm, correct, reject, supersede, delete, or promote route; default retrieval requires trusted, artifact-current memory | Inspector tool-surface diff and proposal/retrieval lifecycle |
| Poisoned or cross-project startup context | Resume selection admits only verified/trusted/current lessons, keeps a fixed project binding, labels all text untrusted, and reports no live-source check | Two-project multi-session scenario with adversarial proposal, correction, confirmation, and source drift |
| Cross-project search leaks or conflates memory | Search considers only the bounded explicit observation catalog; every root must match its stable ID and resolve its own binding; unavailable scopes are skipped; results retain project identity and global/per-source bounds; MCP remains fixed-project | Multi-project marker isolation, missing-root degradation, copied-identity rejection, bounded-query/result tests |
| Stale, concurrent, or coerced capture-mode escalation | A private project-specific cross-process lock serializes updates; project identity and the caller's observed current mode are revalidated under the lock; Full Evidence needs a separate explicit consent flag; other policy fields are preserved; the `.ley` directory is revalidated before atomic replacement | Concurrent-writer serialization, stale expected-mode rejection, missing-consent rejection, custom-scope preservation, real Structured→Minimal retention round trip |
| Local MCP compromise | Local stdio only; exact host launch command; fixed project scope; no Roots, sampling, network listener, deletion tools, subscriptions, or ambient project discovery | Host-adapter sandbox profiles |

## Explicit non-claims

- Local storage is not encryption at rest; device/vault encryption is a separate choice.
- When a user intentionally supplies Ley context to a cloud agent, that context is visible to that provider.
- Memory and citations can reduce unsupported guessing; no system can guarantee that an LLM never hallucinates.
