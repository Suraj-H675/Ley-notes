# Project Brain M0 repository reconciliation — 2026-10-06

## Status

**M0 product/architecture reconciliation.**

This document records what the current Ley repository actually contains at the Project Brain reset, what
deserves to survive, what must change, and what remains transition-only. It is deliberately not an
implementation plan for every later milestone.

The current product contract is [`../../LEY.md`](../../LEY.md). When this document and historical ADRs or
older research disagree about what Ley should become, the current product contract wins. Current runtime
truth still comes from the repository and reproducible behavior, not from this document.

## Why M0 exists

Ley reached a strong but narrower continuity product before the Project Brain reset. That work includes
substantial persistence, evidence, capture, agent-integration, privacy, packaging, and evaluation machinery.
The reset does not justify discarding those foundations, but it also does not justify carrying their product
ontology forward by inertia.

M0 therefore answers two different questions:

1. **What should Ley become?** — the Project Brain contract.
2. **What does the repository contain today, and which parts still earn a place?** — this reconciliation.

## Evidence inspected

The M0 audit used current repository evidence at branch point `ea1620cbc6fdf7526ff98db88d481fa3478e37c4`,
including:

- root `LEY.md` as it existed before this reset;
- `README.md`, `docs/README.md`, `docs/architecture.md`, `docs/privacy-and-storage.md`, and
  `docs/runtime-verification.md`;
- current `docs/agent-memory/*` operational references;
- historical ADR/research/archive material where needed to understand why compatibility state exists;
- `crates/ley-core` project identity, continuity store, ingestion, sessions, learning, revision,
  approved-source, catalog, retrieval, and host-adapter implementation;
- `crates/ley-cli` and `crates/ley-mcp` public/transition surfaces;
- Tauri/Desktop host integration and runtime code;
- React Desktop project/Agent Memory surfaces;
- current Codex and Claude integration packages and hooks;
- current local Codex and Claude CLI capability surfaces;
- existing eval and release/packaging structure.

Three independent GPT-6 Luna xhigh read-only audits covered persistence/core, current docs, and agent/Desktop
surfaces. After the parent audit produced a tentative contract, a GPT-6.1 Sol high review challenged that
contract. Its useful corrections are incorporated below, notably around reviewed-claim durability,
SourceVersion semantics, repository markers as non-authority, export completeness, and deletion/resurrection.
Worker conclusions were verified against repository evidence rather than accepted automatically.

Current agent-host capability assumptions were also rechecked rather than inherited from old docs. The installed
Codex CLI at M0 exposes non-interactive execution, read-only sandboxing, ephemeral sessions, explicit working
directory, structured output schema, and user-config suppression; the installed Claude Code CLI exposes print/
non-interactive execution, restricted/safe customization modes, tool restrictions, structured schema output, and
no-session-persistence. Current OpenAI [Skills documentation](https://developers.openai.com/plugins/concepts/skills)
continues to define Skills as the workflow layer around MCP-controlled data/actions, and the
[skill-building guide](https://developers.openai.com/plugins/build/skills) documents direct Codex skill invocation
(for example `$skill-creator`). Those checks support the skill-first user workflow and restricted optional-analysis
direction; they do not make a prompt/skill the authorization boundary.

## Current repository reality

### Product shape

Before this reset, current docs described Ley as a local continuity/context layer for coding agents. The
canonical model-facing surface was intentionally small: task Brief, bounded Search, exact Evidence, and an
optional structured Checkpoint write. Desktop was a focused “Agent Memory” control center.

That product is implemented enough that its documentation cannot simply be reworded as if the Project Brain
already ships. During the remake, docs must distinguish **target contract** from **current implementation**.

### Project identity

Current project initialization is filesystem-root based. `.ley/project.json` stores a stable generated
`prj_...` identity, while `.ley/capture.json` and `.leyignore` hold small portable project-owned capture
configuration.

Git is optional; the actual architectural limitation is that a project currently requires an existing
directory/root and repo-local identity marker. A source-only Brain with no repository/folder does not yet fit
that model.

### Canonical local persistence

Current canonical machine-managed continuity is an owner-private SQLite database, schema v11 at the reset.
Important durable structures include:

- `projects`;
- append-only/versioned `events`;
- `event_links`;
- local `project_observations`;
- approved-source authority and retained immutable approved-source blobs;
- artifact snapshots/files/current-artifact state;
- explicit migration/authority-cutover state.

Large retained artifact evidence may live in a private per-project content-addressed store. The implementation
already uses transactions, foreign keys, WAL, full synchronous writes, request idempotency, sequence checks,
private permissions, locking, and bounded payloads in important paths.

### Sessions and learned knowledge

Current Sessions are richer than simple summaries. Structured records can preserve goals, decisions, attempts,
problems/resolutions, verification, unresolved work, visible prompt/response evidence, tool observations, and
artifact citations. Current Learnings provide review/correction/supersession/freshness/provenance behavior.

However, these types encode the focused-continuity product ontology. They are **not** automatically the final
Project Chronicle or general Assertion model.

### Captured project artifacts

Native artifact snapshots already provide part of the future SourceVersion foundation: capture identity,
project-relative paths, hashes, optional retained bytes, capture time, and bounded Git state. Current garbage
collection intentionally keeps current/cited snapshots rather than promising an archive of every observed
version.

That retention rule must change for Project Brain Sources explicitly chosen for durable retention. An uncited
imported specification is still canonical project input and cannot disappear merely because no current event
cites it.

### Evidence and applicability

The repo has strong exact snapshot/path/hash citations and a bounded evidence reader. Git revision relation is
recomputed and can remain unknown rather than inventing certainty. Learning/source freshness and human review
also preserve useful distinctions.

These mechanisms are foundations. Their existing labels/ontology are not automatically the final knowledge
model.

### Capture/privacy

Current ingestion already contains valuable safety work:

- project-scoped capture preview;
- `.gitignore`/`.leyignore` handling;
- bounded file/total sizes;
- default secret/binary/build exclusions;
- no-follow/capability-oriented filesystem handling;
- credential-pattern redaction;
- Minimal / Structured / Full Evidence retention modes;
- project egress policy;
- project/session erasure paths.

Those boundaries should be adapted, not casually rebuilt.

### Codex and Claude integration

The repository already has project-scoped Codex and Claude packages, lifecycle hooks, app-owned helper
packaging, explicit Desktop Connect/Disconnect flows, and a useful distinction between:

- host executable detected;
- Ley configuration installed;
- host restart/review still needed;
- matching Ley activity actually observed.

Current hooks retain supported bounded observations, not complete host behavior or hidden reasoning. That
honesty remains a requirement.

### Desktop

Current Desktop has a Projects hub and many Agent Memory sections, but its IA is substantially denser than the
approved Project Brain experience. It should not constrain the new sparse Home / Projects / Settings and
Overview / Knowledge / History / Review design.

### Legacy/compatibility breadth

`ley-core` still contains modules and error/API surface for historical concepts including legacy bindings,
Bootstrap Specifications/References, Context Mounts, Knowledge Scopes, Policy Bundles, external connectors,
graph history, historical-host import, consolidation/recovery machinery, legacy Session/Learning stores, and
transition registries.

Some creation/growth paths are already retired while read/remove/migrate/privacy-ancestry paths remain. Their
existence is not proof they belong in the future architecture, but deleting them without closing their persisted
state/privacy obligations could corrupt or resurrect user data.

## Keep / adapt / replace / remove / defer

### KEEP — foundations that already earn their place

These concepts should survive unless later implementation evidence exposes a concrete defect:

| Foundation | Why it survives |
| --- | --- |
| Stable opaque `prj_...` IDs | Useful project identity primitive independent of display name. |
| Owner-private SQLite authority | Appropriate local-first durable state boundary. |
| Transactional/idempotent event writes | Required for trustworthy interrupted/replayed capture. |
| Project-qualified foreign keys/links | Strong cross-project isolation foundation. |
| Explicit event/evidence identity | Supports inspectable provenance and safe retries. |
| Bounded/redacted/no-follow ingestion techniques | Directly relevant to deterministic local import. |
| Exact retained evidence citations | Required for “why does Ley believe this?” inspection. |
| Explicit omission/truncation disclosure | Prevents false claims of complete capture. |
| Revision relation with `unknown` | Useful applicability evidence without pretending Git proves semantics. |
| Human review concurrency/stale guards | Prevent stale UI/actions from silently overwriting newer state. |
| Project/session erasure dependency planning | Strong basis for Project Brain deletion semantics. |
| Egress policy and source ancestry lessons | Required when derivatives or agent analysis may leave the machine. |
| Host config ownership checks | Prevent Ley from overwriting foreign Codex/Claude configuration. |
| Detected/configured/reviewed/observed host-state separation | Avoids fake integration-health claims. |
| Local stdio agent boundary | Fits local-first integration and explicit project binding. |
| Existing packaging/signing/update work | Product distribution concern independent of the memory ontology. |
| Focused continuity eval scenarios | Failed-attempt, stale-memory, evidence, crash, and isolation tests remain valuable. |

“Keep” means preserve the invariant/capability. It does not mean freeze every current API/type name.

### ADAPT — good work with the wrong current product shape

| Current area | Adaptation required |
| --- | --- |
| `.ley/project.json` | Becomes repository/working-copy association to a logical private Brain, not the Brain itself. |
| Project Catalog / path observation | Track authorized locators/working copies without making one path authoritative identity. |
| Artifact snapshots | Generalize toward Source + SourceVersion + capture occurrence; preserve transformation/retention truth. |
| Approved Sources | Reuse exact-version human authority lessons for source adoption; do not keep the old ontology by default. |
| Session events | Preserve as canonical historical evidence/migration input; evolve Chronicle around supported observations/gaps. |
| Learning review | Reuse correction/supersession/concurrency ideas; split evidence, adoption, producer, and applicability semantics. |
| Event envelope / event links | Keep versioning/provenance, extend only for real Chronicle/human-decision needs. |
| Brief/Search/Evidence/Checkpoint | Preserve bounded/evidence/write invariants while allowing the user-facing agent vocabulary to change. |
| Host hooks | Expand supported observable Chronicle capture deliberately; keep adapter/version gaps explicit. |
| Desktop project onboarding | Rebuild around Brain creation/import and optional agent connection rather than folder-first Agent Memory. |
| Current capture modes | Re-evaluate names/defaults against SourceVersion retention; keep explicit sensitivity/consent boundaries. |
| Portable continuity | Expand backup contract to all retained canonical Brain state, not cited-only evidence. |
| Current retrieval/evals | Use as baseline evidence; typed relationships/semantics must re-earn complexity. |

### REPLACE — assumptions/ontologies that cannot define the Project Brain

| Existing assumption | Replacement |
| --- | --- |
| Filesystem root = Project | Logical Project/Brain identity + explicit repository/source attachments. |
| Structured checkpoint is the main historical substrate | Supported observable Chronicle + exact source/session evidence. |
| `Learning` as the general project knowledge object | Assertion/relationship model with separate evidence/adoption/applicability. |
| `trusted` as shorthand for usable truth | Exact human action + evidence basis + adoption + applicability scope/assessment. |
| Current code-oriented graph as project knowledge graph | Typed evidence-backed Assertion relationships; visual Map is only a projection. |
| Current dense Agent Memory IA | Sparse Project Brain IA. |
| Current folder-only first-run model | Brain can be source-only; repository attachment is optional/later. |
| “Only cited artifacts belong in portable backup” | All intentionally retained canonical Brain state belongs in backup. |

### REMOVE — but only after concrete obligations are closed

The following are not part of the new product model and should trend toward deletion rather than continued
feature growth:

- legacy vault-binding product behavior;
- Context Mount as a product concept;
- Knowledge Scope as a product concept;
- Policy Bundle as a product concept;
- retired external-connector provider lifecycle;
- old source-code graph history as canonical knowledge;
- shape-specific recovery writer families;
- duplicate/legacy model-facing search/evidence/resume wrappers;
- legacy Session/Learning JSON authority after migration cutover is truly complete;
- compatibility registries that no longer protect persisted state, erasure, or restrictive egress ancestry;
- obsolete Desktop sections/components once their useful capability has moved into the new four project areas.

M0 does **not** delete these blindly. Each removal needs an obligation closure check first.

### DEFER — explicitly later milestones

- final new database schema implementation (M1);
- deterministic Project Brain import implementation (M2);
- expanded Project Chronicle capture (M3);
- final `$ley` bootstrap/orientation flow (M4);
- sources-first implementation workflow (M5);
- derived Assertion/entity engine (M6);
- final retrieval/embedding decision (M7);
- completed Claude cross-agent continuity (M8);
- Desktop-driven agent analysis (M9);
- sparse Desktop rebuild (M10–M11);
- Knowledge Map (M12);
- general URL/external-source connectors unless a later milestone earns them.

## Locked domain boundaries for M1

### Project / Brain

Stable private logical identity. It may exist without a repository or live filesystem root.

### Repository / working-copy attachment

The initial Project Brain supports **zero or one logical code-repository attachment**. That repository may have
multiple explicitly authorized working-copy/worktree locators, each with independent path/revision observation and
authorization state. Multi-repository Projects are deferred rather than speculatively modeled in M1.

A repo-local Brain ID helps association but grants no private-state, capture, or egress permission. M1 must
distinguish Project identity, repository attachment identity, and working-copy locator identity.

### Source

Stable project-owned identity for an evolving input. Different Sources remain distinct even when their current
bytes match.

### SourceVersion

Immutable retained representation of one Source. It records content identity and enough transformation/retention
metadata to say whether Ley retained original bytes, redacted text, extracted text, or another representation.

### Capture/import occurrence

Canonical observation of when/how a SourceVersion entered the Brain, including relevant actor/session/scope and
omission/exclusion metadata.

### Session

One identifiable coding-agent work session with project/host/provider correlation and explicit ordering/gaps.

### Episode

One supported observable occurrence. Semantic groupings inferred across observations are derived, not promoted
to canonical history by naming them “episodes.”

### Evidence reference

Exact reference to retained SourceVersion or Session/Episode evidence. Integrity != semantic truth.

### Assertion

A claim or relationship. Unreviewed machine Assertions may be rebuilt. A human action against an Assertion must
freeze the exact reviewed version and its dependencies so rebuild cannot mutate what was accepted/rejected.

An `actor: user` value on an agent-writable request is not sufficient evidence of human authority. Human-review
and permission actions must enter through a human-control boundary unavailable as ordinary agent write authority,
and the durable event must preserve that origin.

### AnalysisJob

Operational record for optional model analysis. It is not itself project truth. It must be revocable/cancellable,
scope-bound, and unable to commit candidate interpretation as human authority.

## Knowledge-state semantics

The original “three dimensions” remain, but applicability is clarified rather than represented by one overloaded
enum.

1. **Evidence basis:** observed / reported / inferred (with producer identity separate).
2. **Adoption:** proposed / accepted / rejected / superseded.
3. **Applicability:**
   - scope: project/source/version/revision/branch/worktree/etc.;
   - assessment for a context/time: current / stale / contradicted / unknown.

This avoids nonsensical choices such as forcing “branch-specific” to compete with “stale.”

## Agent contract locked at capability level

The new user mental model is the Ley skill (`$ley` in Codex; native equivalent in other hosts), but M0 does not
freeze final MCP tool names.

Regardless of naming, the engine must enforce:

- explicit Project/Session binding;
- bounded task-conditioned orientation/query;
- exact evidence validation;
- idempotent/concurrency-safe writes;
- observation vs reported/inferred claim distinctions;
- no self-granted human authority;
- capture and egress permission boundaries;
- cross-project isolation.

The skill may teach the workflow. It may not be the security model.

For an unregistered repository, invoking Ley previews scope and asks before creating/attaching a Brain. For an
existing Brain, invoking Ley with no other request means bounded orientation. Natural-language questions are the
primary query experience.

## Capture contract

Ley promises a faithful record of **supported observations**, not a complete record of everything an agent did.

Every adapter must preserve, where available:

- host/session identity;
- occurrence and recording order/time where the distinction matters;
- stable retry/delivery identity;
- supported visible prompts/responses/tool activity;
- explicit omissions/truncation/unsupported events;
- relevant repository/revision context without pretending revision proves semantics.

Hidden reasoning is never claimed or reconstructed as observed evidence.

## Source-first and human-authority contract

A Brain can be created from files/folders/pasted text before code exists. Imported material is reference evidence
until the user explicitly adopts a requirement/constraint/other authoritative project intent.

Adoption freezes the exact reviewed target. Later source changes may make applicability stale/unknown, but they do
not rewrite the historical adoption action.

## Optional AI analysis contract

Optional Desktop analysis is a future egress-producing action, not an extension of deterministic local import.

It must have:

- explicit agent/provider destination;
- explicit eligible retained source/version scope;
- quota/billing disclosure;
- invocation-scoped authorization initially;
- invalidation when source/policy/project identity changes;
- restricted/read-only host execution where supported;
- bounded output/retries/cancellation;
- structured candidate output;
- citation identity validation;
- no automatic promotion to accepted truth.

The fact that Ley launches a local Codex/Claude executable does not imply the model execution is local.

## Deletion and lifecycle contract

M1 must distinguish:

- detach working copy;
- remove Source from active project use;
- erase retained source history;
- erase Session;
- clear rebuildable projection;
- erase Brain.

Erasure has two additional invariants:

1. dependent Ley-controlled derivatives/caches/reviewed knowledge cannot retain restricted erased evidence in
   violation of the deletion contract;
2. stale/in-flight hook, import, or analysis results cannot recreate state after erasure completion.

A generation/tombstone/equivalent durable mechanism is acceptable. The exact schema is M1 work.

Filesystem and SQLite cleanup are not assumed to be one atomic transaction. Interrupted erasure needs a
recoverable/retryable state rather than a false all-or-nothing claim.

## Export/import contract

The future complete Brain export includes all intentionally retained canonical state, including retained Sources
and SourceVersions that no Assertion currently cites. Rebuildable projections may be omitted.

Import must preserve evidence/provenance and human decisions but does not automatically import machine-local paths,
host trust, capture permission, model-sharing permission, or secrets as active authority.

## Compatibility obligation ledger

The following categories are known to require explicit closure rather than aesthetic deletion:

| Compatibility category | Why it cannot be blindly deleted | Removal condition |
| --- | --- | --- |
| Pre-cutover vault/binding state | May contain historical user memory/evidence and erasure targets. | Native migration/erasure tested for every supported persisted format; no live creation path. |
| Legacy Session/Learning JSON stores | Historical events may not yet be cut over everywhere. | Supported formats migrate/replay/erase losslessly into the new canonical model; stale stores cannot resurrect state. |
| Approved-source legacy issues/authority | Protects exact human-approved source versions and stale-source behavior. | Equivalent Source/SourceVersion/adoption semantics migrated with no authority widening. |
| Context Mount / Knowledge Scope / Policy Bundle ancestry | Retained restrictions can still constrain egress/privacy. | Derivative provenance migrated or erased such that removing ancestry cannot broaden egress. |
| Bootstrap Specification/Reference state | May be user-approved context for uninitialized workspaces. | Source-first Brain flow has an explicit migration/detach path and no state is silently adopted. |
| Historical connector snapshots/overrides | Local retained source/privacy state may still exist. | Inspect/remove/migrate path closes all supported records and restrictive ancestry. |
| Legacy graph/artifact snapshot metadata | Old citations/replay may still reference it. | Exact cited evidence remains resolvable or is deliberately migrated/erased before graph metadata removal. |
| Browser legacy recovery page | Historical IndexedDB may exist outside native store. | Separate explicit recovery promise is intentionally ended only with a user-visible migration policy. |

This ledger is a starting set, not permission to keep compatibility forever. Later cleanup should attach concrete
schema/version evidence and tests to each surviving obligation.

## M0 scenarios that the contract now resolves

These scenarios are the persistence-design checks M1 must be able to satisfy without changing the product model:

### Source-only Brain, repository later

Create Brain `P`. Import specifications. Later attach repository `R`. `P` keeps the same Project ID; the repository
becomes an attachment and may gain a small association marker.

### Reviewed candidate survives rebuild

Agent proposes candidate Assertion `A1`. User accepts exact `A1`. Derived indexes/Assertions are deleted and rebuilt.
Ley still knows exactly what the user accepted and can link any newly derived equivalent/different claim to that
historical action rather than rewriting it.

### Uncited retained source survives backup

User imports `requirements.md`, chooses to retain it, but no knowledge Assertion cites it yet. A complete Brain
export/import preserves that retained SourceVersion and provenance.

### Copied repo marker grants nothing

Repository copy contains `.ley/project.json` for Project `P`. Opening the copy does not automatically reveal `P`'s
private Brain, enable capture, or permit provider sharing. Ley detects an association conflict/unknown copy and
requires explicit resolution.

### Concurrent erasure vs analysis/write

User erases evidence/Brain while a stale hook or optional analysis is running. Completion of that stale operation
cannot recreate the erased state; it must fail/reject against the newer lifecycle generation/equivalent guard.

### Legacy migration preserves weakness

Old checkpoint says an agent claimed a test passed. Migration records that historical structured claim/evidence; it
does not invent an observed successful test. Old “trusted” learning state is preserved historically rather than
silently converted to a new human acceptance action.

Likewise, a learning or requirement that was historically tied to one source version, session, revision, branch, or
other narrow context cannot become a project-wide Assertion just because the new schema can represent one. Migration
must preserve the weakest supportable claim content, evidence basis, adoption, and applicability scope/assessment.
When the mapping is uncertain, keep the legacy record as historical evidence or an unadopted narrow/unknown-scope
candidate rather than broadening it.

## Documentation cleanup decisions

M0 deliberately performs **contract cleanup, not broad implementation deletion**.

- Root `LEY.md` becomes a concise **tracked** current Project Brain contract rather than the former local-only,
  multi-thousand-line running execution diary. The old local diary is not promoted into tracked documentation;
  this reconciliation preserves its material product/migration conclusions.
- `README.md`, `docs/README.md`, `docs/architecture.md`, and `docs/privacy-and-storage.md` are reconciled so the
  target direction is explicit while current shipped behavior remains truthfully labeled.
- Existing ADRs stay historical evidence. “Accepted” in an ADR means accepted for that historical decision, not
  permanent product authority.
- `docs/agent-memory/*` remains current/transition implementation guidance until the corresponding runtime is
  replaced; it is not the target Desktop IA or final domain model.
- Production code is not mass-deleted during M0. Removal follows the compatibility obligation closure above.

### Claims intentionally retained during the transition

Not every visible string is rewritten to the future product before the implementation exists. That would make
documentation/marketing outrun reality. M0 intentionally leaves these current-runtime claims in place until their
own milestones change the behavior:

- `docs/agent-memory/*` operational instructions for the focused-continuity runtime;
- current MCP tool names and schemas in code/tests/evals;
- current Codex/Claude package display text and hook behavior;
- current Desktop route/component names and onboarding behavior;
- public website copy that describes behavior the released/current app actually implements.

The current contract and README clearly label those as transition behavior. When the corresponding milestone
changes runtime behavior, its stale tests/docs/copy must change in the same coherent batch rather than being kept for
compatibility aesthetics.

The target UI itself is locked only at product-contract level in M0: optional multi-agent connection during
first-run, sparse Home/Projects/Settings, project Overview/Knowledge/History/Review, editable human-owned project
description, and closable/pinnable/resizable Project Contents with Sources/Repository. No React/Tauri implementation
is performed in M0; current components therefore remain runtime evidence rather than architecture.

## M0 conclusion

The repository contains a trustworthy continuity kernel worth preserving, but its old product ontology no longer
defines Ley. M1 can now design persistence around a stable logical Project, explicit Source/SourceVersion/history
semantics, durable reviewed human actions, faithful observable Chronicle records, and enforceable privacy/erasure
boundaries without treating the existing folder-first Agent Memory model as sacred.

There is no remaining **product-model** ambiguity that requires choosing the M1 table layout speculatively. M1 still
has real engineering choices—schema normalization, representation of authorized working-copy identities,
generation/tombstone mechanism, SourceVersion storage layout, migration staging—but those choices are now
constrained by explicit product behavior rather than hidden assumptions.
