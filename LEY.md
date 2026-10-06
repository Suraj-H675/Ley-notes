# Ley — Project Brain product contract

Status: **current direction**
Effective: **2026-10-06**

This file defines the product and architectural contract Ley is now working toward. It supersedes the
2026-09-25 focused-continuity product shape. The implementation has not yet completed this migration, so
this document intentionally distinguishes **target contract** from **current implementation** where that
matters.

Existing code, tests, schemas, ADRs, research, compatibility machinery, and earlier decisions remain
evidence. They are not authority merely because they exist. Preserve proven work where it serves this
contract; replace or remove it where it does not. Historical material must not be deleted blindly when it
still protects real persisted data, privacy, migration, or erasure obligations.

The dated M0 repository reconciliation is:

`docs/research/project-brain-m0-reconciliation-2026-10-06.md`

The immediately preceding focused-continuity product decision remains documented by the 2026-09-25 audit,
historical ADRs/research, current transition implementation, and the M0 reconciliation. The former local
multi-thousand-line execution diary is not part of the current contract.

## 1. Product

Ley is a **local-first Project Brain for software development**.

Each software project gets one logical Brain that can preserve the project history a future developer or
coding agent actually needs:

- what the project is and what it is trying to accomplish;
- requirements, constraints, references, and other project sources;
- coding-agent sessions and supported observable activity;
- decisions and rationale;
- problems, attempts, failures, root causes, and solutions;
- verification and evidence;
- unresolved work and open questions;
- concepts, components, and relationships;
- relevant repository/revision history;
- corrections, supersession, and human adoption/rejection;
- whether historical knowledge still applies now.

The defining user outcome is not “Ley stored a lot of data.” It is:

> A fresh coding-agent session can recover the right project context, with evidence and uncertainty, and
> continue the work without repeating avoidable historical mistakes.

Ley is not a generic PKM, note-taking application, transcript warehouse, IDE, terminal, graph-demo product,
cloud-required memory service, or ambient cross-project surveillance system.

## 2. Core principle: store broadly, retrieve selectively

Ley may retain substantially more meaningful project history than it supplies to any one agent turn.

The conceptual flow is:

```text
consented project sources + observable work
                    ↓
             Project Chronicle
                    ↓
      derived knowledge + relationships
                    ↓
       bounded task-conditioned retrieval
                    ↓
              Codex / Claude / ...
```

“Remember the project” never means “dump the entire project history into the model.”

Retrieval must stay bounded, task-relevant, project-scoped, provenance-bearing, applicability-aware, and
honest about omissions or uncertainty.

## 3. One logical Brain, independent of a repository path

A **Project** is the logical Brain. Its stable private identity is not a filesystem path, Git remote, folder
name, branch, worktree, or clone.

A Project may exist with:

- no repository yet;
- imported files/folders/text only;
- one attached repository/workspace;
- additional explicitly attached sources.

Repository attachments are associations with the Brain, not the Brain itself.

### 3.1 Repository association

When a repository/workspace is attached, Ley may place a small repo-local marker such as `.ley/project.json`
to carry the Brain ID and portable project-owned configuration. That marker is an **association hint**, not a
permission token and not private Brain storage.

Copying, cloning, or moving a repository must not silently grant access to existing private history, capture
permission, or model-sharing permission. Matching names, paths, remotes, or copied `.ley` IDs must never
silently merge Brains.

For the initial Project Brain product, a Project has **zero or one logical code-repository attachment**. That
repository may have multiple explicitly authorized working-copy/worktree locators on one machine. Each locator
keeps its own path/revision observation and authorization state. A future multi-repository Project is a separate
product decision; M1 must not add speculative multi-repo semantics.

Ley must explicitly resolve conflicting simultaneous working copies/worktrees. A copied marker, a newly discovered
clone, or a second path claiming the same Brain is not automatically authorized merely because the ID matches.

### 3.2 Paths are observations

Machine-local paths are locators/observations. They may change. They do not become portable project authority
or leak into agent/provider output without a justified contract.

## 4. Project sources and versions

Ley distinguishes four concepts that must not be collapsed:

### Source

A stable project-owned identity for an evolving input, for example:

- a repository file;
- an imported specification file;
- an imported folder;
- pasted reference text;
- a future explicitly supported external reference.

A Source can have zero or more local/external locators. Locator identity is not Source identity.

### SourceVersion

An immutable retained representation of one Source at a point in time. It records enough provenance to know
what representation Ley actually retained, including content identity, representation/transformation details,
capture/import time, and revision information where applicable.

“Exact retained evidence” means exact bytes/text of the **retained representation**. If Ley redacts,
normalizes, extracts, or transforms content before retention, it must not call that representation the exact
original source.

### Capture/import occurrence

The observation that a SourceVersion entered or was observed by Ley at a particular time and scope. Reimporting
identical retained bytes may deduplicate storage while still preserving a meaningful new observation/provenance
event. Two distinct Sources with identical bytes remain distinct Sources and retain separate privacy/provenance.

### Evidence reference

An exact reference to retained evidence: Project, SourceVersion (or supported Session observation), and the
relevant selector/range/record identity. Evidence integrity proves what was retained; it does not by itself prove
that an interpretation logically follows from that evidence.

Imported reference material is **not automatically a project requirement**. Human adoption is a separate action.

## 5. Project Chronicle and sessions

The **Project Chronicle** is Ley’s durable record of supported observable project history.

It is not a claim that Ley observed everything.

### Session

A Session is one identifiable coding-agent work session associated with a Project and host/provider identity.
It retains ordering/provenance sufficient to distinguish sessions, hosts, revisions, and supported observations.

### Episode

An Episode is a canonical **observable occurrence** when Ley has a real event/boundary to record: for example a
session lifecycle event, visible user prompt, visible agent response, tool invocation/result, capture/import,
verification observation, or explicit human action.

An inferred semantic grouping such as “the authentication debugging episode” is derived knowledge, not a
canonical Episode merely because a model grouped events that way.

### 5.1 Completeness and ordering

Ley must expose gaps when host capabilities, retention settings, limits, interruption, or unsupported tool types
prevent complete observation. It must not claim hidden chain-of-thought or provider-internal reasoning.

Observing an agent say “tests passed” establishes that the statement was reported. It does not establish that the
test run was observed successfully. Likewise, a tool event must not be silently upgraded into semantic success.

Retries/duplicate delivery must be idempotent where the host exposes stable identity. Per-session ordering should
be explicit. Parallel sessions must not be converted into a fictitious causal sequence just by timestamp sorting.

## 6. Canonical history versus rebuildable interpretation

Ley must preserve what actually happened separately from how the current version of Ley interprets it.

### Canonical project history

Canonical state includes, as applicable:

- Project identity and explicit attachment/authorization actions;
- Sources and retained SourceVersions;
- capture/import occurrences and omission disclosures;
- Sessions and supported observable Episodes;
- exact evidence references/dependencies;
- explicit human actions such as adoption, rejection, correction, supersession, merge/split decisions, and
  privacy/egress grants or revocations;
- **the exact reviewed claim/version targeted by a human action**;
- deletion/erasure state needed to prevent stale writers from resurrecting removed data.

The last two points are important. Machine-generated interpretations can be rebuildable, but once a human accepts,
rejects, corrects, or supersedes a specific candidate, Ley must retain exactly what that person acted on. A future
extractor must not silently replace the reviewed content with a newly generated “equivalent” claim.

### Derived/rebuildable state

Derived state may include:

- Entities;
- unreviewed candidate Assertions;
- inferred relationships;
- summaries;
- retrieval indexes and rankings;
- embeddings, if they later earn their cost;
- graph layouts/communities;
- inferred semantic groupings.

Derived state must be rebuildable without erasing canonical history or human decisions.

## 7. Knowledge and truth model

An **Assertion** is a claim or relationship about the project. Examples include:

```text
AuthService depends_on TokenStore
Decision 28 supersedes Decision 12
Approach X failed_because race condition
Solution Y resolved Problem Z
Verification V supports Assertion A
```

Each important piece of knowledge must keep separate answers to different questions.

### 7.1 Evidence basis

Examples:

- directly observed;
- reported;
- inferred.

Producer identity is separate from evidence basis. “Claude reported X” and “Ley inferred X from files” are not
the same provenance.

### 7.2 Adoption

Examples:

- proposed;
- accepted;
- rejected;
- superseded.

Accepted means an authorized human adopted the exact reviewed claim for a stated project purpose. Acceptance does
not prove factual truth and does not grant filesystem/tool/network/model permission.

### 7.3 Applicability

Applicability has two parts rather than one overloaded status:

- **scope** — project-wide, source/version-bound, revision/branch/worktree-bound, or another explicit scope;
- **assessment** — current, stale, contradicted, or unknown for the evaluated context/time.

Git ancestry is evidence about revision relationship. It is not an oracle of semantic applicability. A
project-wide human requirement does not become false merely because the code branch diverged.

Supersession remains an explicit relationship, not only a status label.

## 8. Human authority and review

Ley must not allow an agent, imported file, model output, or historical memory to self-grant human authority.

Human-authority actions include, where the product exposes them:

- adopting a reference as a project requirement;
- accepting/rejecting/correcting/superseding candidate knowledge;
- connecting or attaching a project/repository/source where permission is required;
- enabling automatic capture;
- allowing model/provider sharing;
- erasing retained state.

The durable record must identify the exact target the user acted on and enough context to reject stale UI/actions.
An `actor = user` field supplied by an agent-writable API is not proof of human authority. Human adoption,
rejection, correction, and permission grants must originate from a human-control surface/protocol that the
connected agent cannot invoke as ordinary model authority, with that origin recorded durably enough to audit.

Creation/attachment permission, local capture permission, and model-sharing permission are distinct grants even if
one screen presents them together. Revoking one must not be interpreted as revoking or granting another.

## 9. Import and analysis

Project import is deliberately two-phase.

### Phase A — deterministic local import

No model is required. Ley itself establishes the Brain and captures the approved local evidence boundary: project
identity, selected Sources, versions/hashes, relevant repository/Git state, manifests/docs/source inventory,
ignore/exclusion behavior, and existing Ley history where applicable.

The Project Brain must already be useful if:

- the machine is offline;
- Codex/Claude is unavailable;
- the user is rate-limited;
- optional analysis fails.

### Phase B — optional agent analysis

Desktop may later invoke a configured coding agent to analyze an explicitly selected retained source scope.

Before the invocation, the user must see enough information to understand:

- which agent/provider target is being used;
- which retained representations are eligible to leave the local boundary;
- that the agent/provider may consume account quota or incur billing;
- that Ley may not know remaining allowance or exact cost;
- that analysis is optional.

Analysis authorization is invocation-scoped unless a future explicit product contract creates a broader scope.
Changed source versions, project identity, or sharing policy must invalidate a stale pending authorization.

The execution should be restricted/read-only and bounded where the host supports it. Unrelated plugins/hooks/MCP
or customizations should not silently expand the analysis boundary.

Model output is **candidate knowledge**. Citation validation establishes that the referenced retained evidence
exists and matches its identity; it does not prove semantic entailment or truth.

When Ley is invoked from inside an already-running coding-agent session, it should use that session rather than
recursively launching another instance of the same agent.

## 10. Agent experience and enforceable engine contract

The user-facing workflow should be centered on the native Ley skill/invocation style of the host.

For Codex, the intended mental model is `$ley`. Claude should use the cleanest equivalent supported by its native
skill/plugin conventions rather than fake syntax parity.

### New/unregistered repository

Invoking Ley should explain that the current workspace is not associated with a Brain, preview the intended local
scope/exclusions, and request the required permission before creating/attaching anything.

It must not implicitly initialize arbitrary workspaces.

### Existing Brain

Invoking Ley with no further instruction means:

> Orient yourself using the relevant, current, evidence-backed context from this Project Brain.

Natural-language questions such as “why did we stop using Redis?” are the primary deeper query interface.

The skill is a workflow layer, not a security boundary. Regardless of future MCP/tool names, the local engine must
continue enforcing:

- exact authorized Project/Session binding;
- bounded retrieval;
- citation/evidence validation;
- idempotent/concurrency-safe writes where relevant;
- observation versus agent-claim distinctions;
- human-authority boundaries;
- privacy/egress policy;
- project isolation.

Source-only Brains must be addressable without inventing a repository path.

## 11. Automatic capture

After explicit project/host capture permission, supported lifecycle integrations should capture future observable
work automatically so users do not need to invoke Ley manually every session.

Automatic capture must remain:

- project-scoped;
- adapter/version aware;
- bounded by the active retention policy;
- explicit about unsupported/missing observations;
- resilient to retries and interruption;
- incapable of manufacturing authority from captured text.

Existing historical host data is not silently scraped merely because an agent is connected. Historical import, if
supported, remains deliberate and provenance-preserving.

## 12. Retrieval and context compilation

Start from the simplest retrieval stack that proves useful:

```text
exact/project/type filters
+ lexical FTS/BM25
+ typed relationship traversal where earned
+ time/source/revision applicability
+ authority/conflict/egress handling
+ bounded task-conditioned assembly
```

Embeddings or model-assisted retrieval may be added only if controlled downstream evaluation shows material value
over this baseline under the same evidence, privacy, applicability, latency/resource, and token-budget constraints.

Embeddings, scores, recency, and graph proximity may locate evidence. They never become evidence of truth.

## 13. Product experience

The target Desktop information architecture is deliberately sparse.

### First run

```text
Welcome
  → Connect coding agents (optional)
  → Import/create first Project Brain or Skip
  → Home
```

Do not force one “main agent.” Codex, Claude, and future supported agents can all contribute to the same Brain.
The connection step may detect supported installed hosts and offer them together. Connecting a host means Ley knows
how to integrate with that host for projects the user explicitly connects; it does **not** silently authorize every
project, historical transcript import, automatic capture, or model egress.

### Global navigation

```text
Home
Projects
Settings
```

Home answers **“Where do I want to continue?”** It is not an analytics dashboard.

Keep Home intentionally sparse: global project search, recent Brains, an obvious Import/Create action, and at most
small genuinely actionable review state. Do not add entity counts, capture charts, memory-health scores, graph
statistics, or card grids merely because Ley has the data.

`Projects` answers **“Which Project Brains do I have?”** It is a searchable catalog with Import/Create and only
small useful metadata such as recent activity or connected-agent state when that materially helps selection.

### Inside a project

```text
Overview
Knowledge
History
Review
```

An optional/resizable/pinnable **Project Contents** rail exposes:

```text
Sources | Repository
```

Repository/source content may be inspected and opened externally. Ley does not initially become an arbitrary
repository editor or IDE.

The project header uses a prominent project title plus a plain-English description of what the project actually
is. That description is editable. If analysis proposes the initial description, it remains a suggestion until the
user accepts/edits it; later extraction must not silently rewrite human-owned project metadata.

Project Contents may be closed, temporarily opened, pinned, and resized, with that local presentation preference
remembered. `Sources` exposes retained/imported project inputs and their versions; `Repository` exposes the live
working-copy tree/status as a locator onto current project state. Historical retained evidence must remain visually
distinguishable from a current live file.

### Overview

Current evidence-backed context, open work, recent decisions, recent activity, and project search.

### Knowledge

List-first project knowledge such as requirements, decisions, problems/solutions, constraints, concepts,
components, and learnings. Important claims expose their evidence and connections.

### History

Session history grouped/filterable by agent (for example Codex / Claude / Other), with supported observable
conversation/activity and extracted knowledge. “History” must not imply Ley captured unsupported or hidden data.

### Review

Only material candidate/conflicting/stale knowledge requiring human judgment. Review is not an infinite approval
queue for every machine-generated fact.

### Knowledge Map

Knowledge may later offer `List | Map` and focused “Show connections” views. The Map visualizes typed,
evidence-backed relationships that have already earned utility; it is not a giant homepage graph.

## 14. Privacy, locality, and egress

Core Brain creation, deterministic import, storage, inspection, and retrieval must work locally without mandatory
cloud/model access.

Project/private data stays local by default.

Any operation that deliberately sends retained project content to a cloud-backed coding agent is an egress event,
even if Ley invokes a local CLI executable. The UI must not equate “local executable” with “local inference.”

Privacy restrictions propagate into derivatives. A summary, Assertion, embedding, index, or reviewed claim must not
launder content that its evidence/source is not allowed to expose.

`confirm-per-use` style sharing must fail closed until Ley has a real invocation-scoped confirmation boundary.

When a user is already inside Codex/Claude and invokes Ley, avoid redundant generic quota warnings, but do not skip
project/capture/sharing boundaries that have not already been granted.

## 15. Deletion, detach, and resurrection prevention

These operations are different and must not be conflated:

- detach a repository/working-copy locator;
- remove a Source from current project use;
- erase retained Source/SourceVersion history;
- erase a Session;
- clear rebuildable projections/indexes;
- erase the whole Brain.

Erasure must account for Ley-controlled derivatives, indexes/caches, reviewed knowledge dependencies, retained
evidence, and pending analysis results. Human acceptance does not exempt derived knowledge from the privacy/deletion
dependency of its evidence.

In-flight or stale hooks/imports/analysis jobs must not resurrect state after a completed erasure. M1 must choose a
simple durable generation/tombstone/equivalent mechanism sufficient for this invariant rather than relying on
timing.

Filesystem evidence and database state cannot always be erased atomically together. Recovery/retry semantics must
be explicit. Ley does not claim forensic deletion of backups, filesystem snapshots, SSD remnants, user-owned source
files, exports outside Ley control, or downstream provider copies.

## 16. Export/import and portability

A complete Project Brain export must preserve the canonical project state the user chose Ley to retain, including
uncited retained Sources/SourceVersions. “Only evidence already cited by an event” is no longer a sufficient backup
contract for the Project Brain.

Rebuildable projections may be omitted when they can be recreated safely.

Portable import must not silently broaden project permissions, egress rights, local path authority, or human
adoption. Machine-local locators and secrets remain machine-local unless a future explicit format says otherwise.

## 17. Migration and compatibility

Current Ley already contains valuable transactional persistence, capture, evidence, revision, host-integration,
privacy, erasure, packaging, and evaluation work. Reuse those foundations where they satisfy this contract.

Current Session/Learning/checkpoint structures, approved-source machinery, Brief/Search/Evidence/Checkpoint names,
legacy vaults/registries, Bootstrap/Context Mount/Knowledge Scope/Policy Bundle/connector ancestry, and the existing
Desktop IA are **transition evidence**, not automatically the new product ontology.

Compatibility code survives only while it has a concrete obligation. Every retained compatibility path should be
traceable to:

1. a supported historical format/version or persisted state;
2. the migration/cleanup/privacy behavior it protects;
3. the target destination/behavior;
4. the verification required before removal;
5. a real removal condition.

Old records must migrate without inventing stronger observations or authority. In particular:

- old checkpoints remain historical structured submissions, not complete chronology;
- old `trusted` learning state does not mechanically become new human `accepted` state;
- old path/project markers do not become capture or egress permission;
- legacy privacy ancestry must not be discarded before derivatives are proven safe or erased.

Migration also must not silently **broaden or strengthen the meaning of a historical claim**. It may not turn a
source-, session-, revision-, branch-, or other narrow-scope record into a project-wide Assertion merely because the
new schema has a more convenient shape. Claim content, evidence basis, producer/origin, adoption, and applicability
scope/assessment must remain no stronger or broader than the legacy evidence establishes. If an exact mapping is
uncertain, preserve the record as historical evidence or an unadopted/narrow-or-unknown-scope candidate rather than
promoting it.

## 18. Explicitly retired assumptions

The Project Brain reset retires these assumptions from the previous focused product:

- a filesystem project root defines the Project itself;
- structured checkpoints are the complete historical substrate;
- one `trusted` state can stand in for adoption, factual truth, and freshness/applicability;
- uncited retained project sources are inherently disposable;
- `ley_brief`, `ley_search`, `ley_evidence`, and `ley_checkpoint` are immutable product vocabulary;
- coding-agent continuity alone is the complete product boundary;
- new Project Brain import should be built on legacy vault/Bootstrap/Context-Mount/Scope/Policy machinery.

Retiring an assumption does not authorize destructive deletion of existing persisted data.

## 19. Non-goals

Unless new evidence changes the product boundary, Ley is not building:

- a generic notes/PKM/Obsidian replacement;
- arbitrary repository code editing, language tooling, or IDE behavior;
- a terminal;
- whole-computer or cross-project ambient surveillance;
- a cloud-required Ley account/service for core behavior;
- hidden autonomous coding started without consent;
- a graph-database dependency merely to support a visual Map;
- a giant knowledge graph as Home;
- mandatory model analysis just to register/import a project;
- authority based on “the AI said so.”

## 20. M1 input contract

M1 may begin only from these locked outcomes:

1. a source-only Project Brain can exist before any repository;
2. attaching a repository later does not change the Project identity;
3. Source identity, SourceVersion, capture occurrence, and evidence reference are distinct;
4. canonical Chronicle records supported observations and visible gaps, not fictional completeness;
5. generated interpretation is rebuildable, while exact human-reviewed targets/actions are durable canonical state;
6. evidence basis, adoption, and applicability remain separate; applicability scope and assessment are not
   collapsed;
7. repo markers/paths are associations, never permission tokens;
8. project attachment, capture, and model-sharing grants are distinct;
9. local deterministic import works without model access;
10. optional analysis is explicit, bounded, provider/egress-aware, and yields candidate knowledge only;
11. all retained canonical Brain state—not only cited evidence—belongs in the eventual portable backup contract;
12. erasure must prevent stale/in-flight writers from resurrecting erased state;
13. migration cannot strengthen/broaden claim meaning, scope, evidence basis, adoption, observations, approvals,
    applicability, or egress rights;
14. agent workflow can change while the engine continues enforcing project, evidence, authority, privacy,
    concurrency, and boundedness invariants.

M1 should implement the smallest trustworthy persistence/identity foundation satisfying those constraints. It
should not prematurely implement the final UI, knowledge graph visualization, optional AI analysis, or every later
milestone.

## 21. Definition of the Project Brain remake

The remake is successful when a fresh agent can return to a cold project and recover the requirement that triggered
the work, relevant decisions and rationale, rejected/failed approaches and why they failed, the eventual solution,
verification, relevant sources/files, unresolved work, current applicability, and exact evidence—and then continue
without repeating the known failure.

Separately, a Project Brain created from specifications/references alone must be able to guide a later empty/new
code project using the **accepted** project knowledge rather than treating every imported source as authoritative.

Everything else exists to make those workflows trustworthy, local-first, inspectable, and pleasant to use.
