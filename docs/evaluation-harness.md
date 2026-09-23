# Ley end-to-end evaluation harness

Ley's executable acceptance corpus lives in `eval/fixtures/scenarios.jsonl` and is driven by
`eval/run_eval.py`. The harness exercises the real `ley` CLI, stdio MCP server, and lifecycle-hook
surfaces against isolated temporary projects/vaults. It is intentionally deterministic: unit tests
prove local invariants, while these scenarios prove multi-surface product behavior.

Each run also owns private temporary `XDG_CONFIG_HOME` **and** `XDG_CACHE_HOME` roots. This prevents a
developer's real Ley configuration or locally installed semantic model from silently changing which
retrieval system an acceptance scenario exercises. Deterministic scenarios therefore start with no
semantic model unless a future fixture explicitly stages one inside that run's private cache.

## What the harness measures

The corpus contains both write-time and read/use-time checks. Current metric families include:

- retrieval recall/precision and strict token-budget enforcement;
- selective abstention when no useful memory exists;
- crash recovery, transition binding, session/learning mutation idempotency, and origin-lineage preservation;
- meaningful-boundary local consolidation review, direct turn-evidence lineage, and terminal-session non-mutation;
- human-intent Specification admission plus revision-bound structured Acceptance criteria and Verification methods, Context Mount isolation, premise resistance, and revision applicability;
- explicit external GitHub connector scope/egress, read-only MCP exposure, stable remove/re-add identity, and non-laundering;
- reusable team/organization Knowledge Scope authority, bounded multi-source retrieval, source-egress inheritance, detach ancestry, and non-laundering;
- parallel-session separation and cross-host durability;
- secret/cross-project/model-egress privacy violation rate;
- deletion fidelity and forgetting-residue rate across Ley-managed raw/derived retrieval surfaces;
- harmless behavior in uninitialized workspaces;
- a deterministic downstream task-evidence contract and a bounded recent-resume baseline comparison;
- explicit missing-semantic-model lexical fallback plus sparse-repository quality across strict
  500/1,500/3,000/8,000-token budgets;
- deterministic code→test, trace→code, and transitive ripple-effect graph/progressive-disclosure retrieval;
- immediate and delayed repository-memory poisoning resistance without hiding captured evidence;
- cross-surface stale-write rejection plus native desktop stale learning-review protection;
- Topic Dossier source binding, bounded topic-state coverage, privacy, and post-erasure rebuild behavior.

`privacy_violation_rate` and `forgetting_residue_rate` are lower-is-better metrics. A zero result means
the fixture's canaries were not extractable from the probed Ley-managed surfaces; it is not a claim
that arbitrary undiscovered channels or user-owned external copies do not exist.

ADR 0068 extends the existing Specification representatives rather than adding a weaker standalone score. `specification-authority-context` requires one exact revision-bound `acr_` row with raw criterion text, source line range, `statusInterpreted: false`, and `persisted: false`; the empty-workspace bootstrap representative requires the same shared projection before target initialization. ADR 0071 extends those same representatives with exact revision-bound `vmd_` Verification-method rows and requires `criterionBindingProven: false`, `observedResultBindingProven: false`, `statusInterpreted: false`, and `persisted: false`. Acceptance criteria retain budget priority over methods. The egress canary places separate private markers inside both an Acceptance criterion and Verification method so a blocked Specification must withhold the whole source and both derived fields. Core regressions prove parser/container false-positive resistance, atomic limits, compiler/Policy-Bundle parity, and that methods use only budget left after existing context plus Acceptance criteria. MCP regressions prove the 256 KiB transport guard drops methods before criteria and preserves the parent before falling back to the existing oversized-result error. ADR 0070 adds the exact criterion-to-historical-Verification review; ADR 0072 extends that same review in-place with optional exact `verificationMethodId`. The real `specification-authority-context` scenario now supplies the projected `acr_`, projected `vmd_`, and real session `ver_` together, requires the method-aware link fingerprint/boundary and exact handles, and still requires method execution/outcome, criterion satisfaction, semantic coverage, current implementation, persistence, automatic-write, and live-source proof to remain false.

## Downstream task contract and baseline

The deterministic downstream contract is intentionally separate from a feature's own success flag.
It serializes only task-supporting context bodies and excludes the task/query text itself, so a fixture
cannot satisfy its own required marker merely by asking for that marker. A contract passes only when
the assembled context contains all declared required evidence and excludes every declared distractor.

The budgeted-quality scenario compares a 500-token Context Compiler result with a bounded recent-resume
baseline. The compiler passes only when its returned context bodies satisfy that downstream contract
and remain inside the requested budget. The resume baseline uses a character budget chosen as an
approximate four-characters-per-token equivalent.

The P0 matrix also uses this same independent contract where a feature has a distinct downstream-use
surface:

- Context Compiler;
- Specifications;
- Context Mounts;
- origin-preserving derivation lineage;
- premise/state adjudication;
- revision awareness; and
- agent-context egress policy.

Their downstream matrix cell must reference `downstream_task_contract`; the evaluator rejects a P0
configuration that aliases those cells back to the capability's adversarial/regression metric.
Representative contracts prove, for example, that a current replacement learning reaches task
context while its superseded guidance does not; an explicitly mounted reference contributes the
requested marker while an unmounted project does not; divergent branch state is withheld before merge
and becomes usable only after Git proves it merged; relevant Specification criteria/method content is
present while an unrelated Specification is absent; and egress-gated private context is usable for the
allowed local target while absent from blocked targets. Origin lineage uses a deliberately different
form of the same contract: after explicit user review, the exact learning guidance and mechanically
resolved lineage must remain available through `ley_learning_get`, while an `uncited` learning keeps
`trustedForReuse: false` and must not be auto-injected by the Context Compiler merely because the
review action marked it trusted.

The same rule is applied selectively to P1 surfaces that actually produce reusable task-facing
knowledge rather than merely diagnostics. The following P1 downstream cells must also use
`downstream_task_contract`:

- Bootstrap Specifications;
- Bootstrap Reference projects;
- Topic Dossiers;
- Current Project State;
- reviewed Runbook/Skill export; and
- richer graph relations.

Their contracts prove the independently usable projection rather than reusing the feature's overall
pass bit. For example, a Bootstrap Specification must expose the exact requirement/criteria/method
while withholding prompt/live-target canaries; a Bootstrap Reference must expose only the attached
reference marker; a Topic Dossier must contain the bounded decision/open-work/verification/artifact
briefing; Current Project State must expose working/open state without changing its historical-authority
semantics; reviewed Skill content must contain only reviewed reusable guidance; and graph traversal
must surface the relevant test that direct context search intentionally misses while excluding the
unrelated test.

P2 applies the same rule only where the expansion itself feeds reusable agent context:

- external reference connectors;
- team/organization Knowledge Scopes;
- team/organization Policy Bundles; and
- explicit historical host import.

External-reference contracts require the relevant historical context to be usable for the allowed
local target while absent from a blocked cloud target. Knowledge Scope contracts require all explicitly
attached shared markers while excluding unrelated scope material. Policy Bundle contracts require the
allowed bundled policy plus active-project human intent while excluding unrelated/conflicting private
policy text. Historical-import contracts require the explicitly selected host turns to remain
progressively readable/discoverable while unrelated-session and secret markers stay absent.

Multimodal evidence and local consolidation remain on their native provenance/review metrics: their
downstream value is preserving original evidence or proposing reviewable consolidation, not ordinary
read-time task-context selection.

This is a deterministic evidence-sufficiency proxy, **not** a score for model reasoning and not a claim
that Ley has beaten an external agent benchmark. Model-dependent downstream benchmarks can be layered
on later, but they must remain reproducible and separately reported.

## Retrieval fallback and budget ladder

`retrieval-fallback-budget-ladder` is the P0 Context Compiler regression representative for retrieval
robustness. It materializes an 80-file, roughly 120-lines-per-file project where many files contain
lower-signal migration terms and one sparse file contains the stronger task evidence. The scenario
queries both `ley_search_memory` and `ley_compile_context` at 500, 1,500, 3,000, and 8,000 tokens and
requires:

- the sparse required marker at every budget;
- `estimatedTokens <= maxTokens` at every layer/budget;
- low-level and compiler retrieval metadata to report `lexical` for overall, bounded-rerank, and
  artifact-context modes;
- non-empty “model not installed” fallback reasons without leaking the private cache/project/vault
  paths;
- an explicit compiler `semantic-fallback` gap;
- non-decreasing bounded search-result counts as budget grows; and
- strictly more retained results at 8,000 tokens than at 500.

The deterministic harness deliberately does **not** download Ley's pinned semantic model and therefore
does not call this a lexical-vs-hybrid benchmark. True hybrid comparison requires an explicitly staged
verified model and belongs in a separately reproducible model-enabled run. Core semantic-retrieval
tests cover corrupt model/index validation; the runtime index path refuses invalid cached indexes and
rebuilds them only when a valid local model is available.

### Opt-in verified-model semantic evaluation

`python3 eval/run_semantic_eval.py` is the separate model-enabled lane for the two §28.3 cases that
cannot be honestly exercised by the cache-isolated deterministic corpus. It **never downloads a
model**: startup requires Ley's exact pinned Model2Vec artifact to already be installed and
checksum-verified in the caller's normal local cache, otherwise the run exits before creating a
project.

The fixture uses one captured project/snapshot and the same paraphrased storage-durability query for
both modes. The runner first points only that search process at an empty temporary cache, forcing
explicit lexical-only fallback under a three-result budget. The gold storage artifact must be absent.
It then restores the caller's verified cache and repeats through real `ley_search_memory`; hybrid mode
must recover the gold artifact inside the same top-three budget with `semanticRank: 1` and a minimum
semantic-similarity floor. This is a measured retrieval addition, not a claim that hybrid ranking always
beats lexical ranking.

The same run then exercises the derived-index failure lifecycle against the real private project memory
namespace. After hybrid search creates the snapshot-bound index, the fixture corrupts only that derived
JSON file and temporarily removes write permission from its already-private directory. The next real
MCP search must remain available but report `artifactContextMode: lexical` with the explicit
`snapshot-bound semantic index could not be built` fallback reason. Restoring write permission and
searching again must repair the index and return to hybrid mode. Finally, the fixture changes and
re-ingests the gold source: the new snapshot must use a new semantic-index binding/file, the gold
citation's artifact snapshot ID must change, and the old derived index must not be reused as current.
All returned agent/search outputs are checked for project/vault/index-path leakage.

## Retrieval relation and progressive-disclosure scenarios

The deterministic graph/retrieval corpus now includes eight distinct relation/query shapes rather than
counting one direct import edge as evidence for every retrieval task:

- `graph-relative-import-test-impact` is the one-hop code→test baseline. Direct context search for an
  implementation-only marker must miss the importing test; one incoming deterministic `imports` edge
  and the exact reverse path must recover it while excluding an unrelated test.
- `graph-dynamic-import-test-impact` applies the same one-hop contract to a quoted literal
  JavaScript/TypeScript dynamic `import('../src/renderer')`. Core regressions require computed and
  template-string imports to produce no module relation and a literal package target to remain an
  external module.
- `graph-dynamic-import-options-impact` verifies that the standard optional second dynamic-import
  options/attributes argument does not hide the exact quoted first-argument module relation. The
  imported captured JSON file must recover its dependent test while an unrelated test stays absent.
- `graph-python-relative-import-test-impact` applies the same one-hop contract to an explicit Python
  relative import (`from ..renderer import render_frame`). Absolute Python imports are not promoted by
  this slice; the captured-path resolver must recover only the exact relative target.
- `graph-exact-path-disambiguation` gives two captured files the shared suffix `src/renderer.ts` and
  queries the intended file by its exact project-relative path. Exact-path resolution must select the
  captured `File` node before broader suffix matching, recover its importing test, and exclude the
  legacy same-suffix file/unrelated test. The unit boundary separately verifies that symbols carrying
  the same artifact path do not make an exact file-path query ambiguous.
- `trace-to-code-progressive-disclosure` keeps a runtime trace as ordinary captured evidence rather
  than inventing trace graph nodes. Direct search returns only the trace artifact, bounded
  `ley_read_evidence` exposes the stable `parseSession` symbol cue, and incoming deterministic
  `defines` traversal must resolve that symbol to `src/session.ts` while excluding an unrelated
  source file. The trace itself deliberately contains no source path, so the graph step supplies the
  defining-file discovery rather than merely echoing the trace.
- `graph-ripple-transitive-impact` materializes a four-hop repository shape: changed core module,
  importing service, importing API, and importing API test. Direct search and depth-1 graph traversal
  must miss the transitive test, while depth-3 incoming traversal and the exact outgoing path recover
  the full test→API→service→core chain and still exclude an unrelated test.
- `graph-barrel-reexport-ripple` keeps the same transitive contract but makes the middle relation a
  source-bearing TypeScript barrel re-export. The graph must recover the exact
  test→consumer→barrel→implementation chain; an implementation-only direct-search baseline must miss
  the test. Core regressions separately prove that exported string values do not become dependency
  edges, package re-exports stay external, and ambiguous local re-exports are not guessed.

All eight use captured snapshot relations only, preserve `liveSourceChecked: false`, require
deterministic provenance, and reject local-path leakage. The P1 Richer Graph Relations downstream cell
continues to use the independent one-hop task contract, while its regression cell now uses the deeper
ripple-effect representative.

## Learning mutation idempotency

`duplicate-learning-mutation-idempotency` complements the older duplicate session-event fixture.
The scenario creates one real retained checkpoint, proposes a cited learning with a caller-stable
request ID, retries the exact proposal, then deliberately reuses that request ID with changed learning
content. It requires the exact retry to return the same event/learning identity with
`replayed: true`, and the conflicting retry to be rejected rather than appended or conflated.

The same contract is then exercised through explicit user review: one confirm review is recorded,
an exact retry replays the same review event, and a same-request/different-note retry is rejected. The
final learning must remain exactly two durable events (proposal + review), with verified/trusted state.
This is idempotency evidence only. Version-bound Procedure application/outcome history is exercised
separately by `procedure-application-outcome-history` and does not change the replay contract here. That
same pass → fail → pass journey also probes Memory Health schema v5: only the failed exact-current
Procedure application may produce `procedure-application-outcome-attention`, and the signal must retain
the same non-causality/authority boundaries as the underlying observation. The report additionally
must disclose three inspected Procedure application claims and three exact-current claims.

`context-utility-unobserved-binding-health` covers the complementary measurement-gap path. It binds a
real non-empty context pack, finishes the session without observing it, then requires body-free
unobserved-binding coverage from `ley_session_get`, exact terminal finish-event identity on both the
finish and unobserved-binding row, and exactly one Memory Health v5
`unobserved-context-utility-binding` review signal. The report must also count one inspected binding,
zero uniquely observed bindings, one unobserved binding, and zero Procedure application claims.
Context-body and absolute-path canaries must remain absent. Core regressions additionally prove that active sessions are not signaled, a later
valid observation removes the signal, more than five recent unobserved binding rows are bounded with
omission/truncation disclosure, and two observations of one binding count as one observed binding.
The targeted desktop Session Inspector regression separately renders one unobserved binding from this
typed projection and requires visible bound/observed/unobserved counts, the body-free binding handle,
and the explicit terminal-anchor explanation without adding a mutation control or quality score.

## Immediate and delayed memory-poisoning resistance

`malicious-text-as-instruction` remains the immediate retrieval boundary: instruction-like repository
text is retrievable as captured evidence, but retains an untrusted source boundary, never reports
execution, and does not become an instruction merely because its wording looks imperative.

`delayed-learning-poisoning-resistance` extends that into a write→retrieve→later-agent sequence. A
captured `INSTRUCTIONS.md` contains an explicit attempt to persist itself as trusted policy and
exfiltrate project data. The evaluator deliberately creates a later agent-authored Procedure learning
derived from a checkpoint that touched that file, giving the proposed learning a unique marker that
does **not** occur in the original repository text. Ley must preserve the proposal and its provenance
for inspection while refusing to launder its authority:

- proposal state remains `tentative`, trust remains `review-required`, and
  `requiresUserReview: true`;
- explicit learning inspection preserves the exact checkpoint plus captured-artifact origin lineage,
  with `automaticAuthorityCeiling: review-required`, `causalCompletenessProven: false`, and
  `trustedForReuse: false`;
- provenance readers keep durable ledger schema identity separate from read-projection identity:
  ordinary utility Session Context reports ledger schema v5 plus projection v1, Procedure-claim
  Session Context reports ledger schema v15 plus projection v1, and Learning Context reports durable
  learning schema v3 plus projection v1;
- low-level Memory Search may return the proposed learning, but labels it
  `trustSignal: unverified` / `trustedForReuse: false`;
- the Context Compiler must emit an admission-stage `unverified-learning` exclusion for that exact
  learning ID, and the poisoned learning marker must not appear in compiled task-supporting items;
- the default current-trusted learning list remains empty while explicit `scope: all` inspection can
  still see the review-required proposal; and
- a later real Codex `SessionStart` plus `UserPromptSubmit` lifecycle must not inject the poisoned
  learning marker into automatic startup or task context.

The malicious repository body itself is still allowed to appear in later task context as
`authority=direct-evidence trusted=false`; the rendered host context must continue to state that such
memory/reference text is evidence rather than host policy or permission. This scenario therefore tests
delayed **authority laundering**, not censorship of captured adversarial evidence. Project/vault paths
remain absent from all probed outputs.

## Cross-surface concurrency and stale user review

`host-local-stale-session-write` exercises one real lifecycle-host/local-writer race. A Codex
`SessionStart` creates the durable Ley session and a local reader records its current event count.
A subsequent Codex `UserPromptSubmit` appends one retained host observation to that exact session.
A local version-guarded `session rename --expected-events <old-count>` must then fail with
`reload before saving`, leave both the name and event count unchanged, and succeed only after the
caller reloads the new count. The scenario also reads the retained session turns to prove the host
marker caused the intervening event and requires zero project/vault path leakage.

Learning review has a stronger user-authority boundary: MCP/agents may propose review-required
learnings but do not receive confirm/correct/reject/supersede authority. The desktop bridge therefore
has a focused native regression,
`desktop_learning_review_rejects_stale_visible_event_count`, rather than pretending an MCP action is
a desktop review. The Tauri `review_agent_learning` command delegates through the same helper under
test and forwards the event count visible when the inspector opened. The regression creates a
review-required learning at event 1, applies a concurrent correction to event 2, and requires the stale
event-1 desktop confirmation to fail without appending or trusting unseen text. After reloading event 2,
the same desktop review path may confirm it, producing exactly event 3 and trusted/verified state.

Together these checks cover concurrent host/local durable writes and stale user review while preserving
the intentional authority split: lifecycle/MCP surfaces can advance session evidence or propose
tentative memory, whereas high-consequence learning trust remains a version-guarded local user action.

## Ten-session changing-requirement continuity

`ten-session-changing-requirements-handoff` exercises the long-horizon coding-continuity case from
LEY.md without introducing a newest-session-wins rule. The fixture records ten distinct completed
implementation sessions, alternating supported hosts, and gives every session one structured checkpoint
plus an explicit finish/handoff. Requirements evolve from remote/Redis-dependent startup toward the
final offline-local behavior.

The evaluator requires all ten sessions to remain independently addressable and completed with exactly
one checkpoint plus one finish event. A bounded `ley_project_resume` request for three sessions must
report `totalSessions: 10`, `omittedSessions: 7`, and return the three latest completed handoffs in
order, including the final continuation marker. Separate project-activity search must still recover all
ten requirement decisions, including markers from early Redis/cloud/remote phases. This proves bounded
resume is a progressive-disclosure handoff rather than destructive compaction of older experience.

Current intent is deliberately modeled separately from that history. The fixture installs one exact,
user-approved Offline Startup Specification with the current requirement. An early session contains the
explicit opposite-polarity clause `Require Redis network bootstrap for startup`, while the approved
Specification states `Do not require Redis network bootstrap for startup`. That old Decision must be
withheld from task-supporting context as `contradicts-human-intent`; the current Specification marker
must be present and the compiler must continue to disclose
`authorityPrecedence: human-intent-over-historical-memory`.

Other non-contradictory historical sessions/decisions are not censored merely because they are old. If
retrieved, they must remain `authority: historical-project-memory` and
`trustedForReuse: false`. This is intentional: the scenario tests durable handoffs plus authority
discipline, not automatic semantic rewriting of ten sessions into one supposedly canonical narrative.
The downstream contract therefore requires the approved current requirement and final bounded handoff,
forbids only the explicitly contradictory Redis marker from task-supporting context, and still requires
zero project/vault path leakage.

## Opt-in real-agent downstream evaluation

`eval/run_agent_task_eval.py` is the separate model-dependent downstream runner. It is deliberately
**not** part of `run_eval.py`, the P0/P1/P2 coverage matrices, or deterministic CI acceptance. It may
invoke a paid/networked external coding agent, so one green run is an observation about that exact
runner/task invocation rather than a reproducible product claim.

The runner uses only synthetic task fixtures from `eval/fixtures/agent_tasks.jsonl`. Each fixture
contains a tiny disposable repository, one prior Ley session/Decision that represents information a
returning teammate could know, explicit context markers, allowed changed files, a visible-test command,
and an external oracle. Fixtures can also declare a runtime-secret contract. For those fixtures the
consequential answer does **not** exist in the checked-in fixture or oracle source: a fresh 32-byte
master seed is generated in the evaluator process, the hidden contract is derived from it, and only a
SHA-256 commitment is written to the normal report.

Fixture validation rejects unsafe or malformed schema fields/paths, invalid change allowlists, tasks
outside the Context Compiler query bound, and historical markers already visible in the task/live
project files. `--validate` initializes the synthetic Ley project, seeds the materialized prior memory,
compiles the real 500-token context pack by default, and requires the expected historical markers to be
genuinely admitted before any model spend. The runtime-secret retry fixture therefore tests retrieval
of an answer that a baseline cannot recover by reading the checked-in benchmark source.

For a real comparison, the runner creates fresh temporary workspaces and executes the same external
agent command in two arms:

- **baseline** — the synthetic code repository only, with no Ley project/history/context supplied;
- **ley** — the external agent receives the same code-only repository plus the bounded Ley context
  projection in its prompt. The prior Ley history is built in a separate private synthetic project. A
  neutral measurement session is started so instrumentation does not outrank historical task evidence;
  the exact context pack is compiled and bound; then the entire Ley project/vault/config tree is
  snapshotted into parent-process memory and removed from the filesystem before the external agent
  starts. It is restored only after the agent/oracle finish so the typed utility outcome can be
  recorded against the original binding.

The comparison is therefore a whole-product “without Ley vs with Ley context” ablation, not a claim
that only one internal retrieval component changed.

The external runner contract is intentionally small: `--runner-command` is parsed without a shell,
receives the complete task/context prompt on stdin, and runs inside a Linux `bwrap` namespace with the
disposable project as `/workspace`. Each arm receives its own randomly named temporary workspace, and
that workspace is deleted before the next arm begins. `/usr` is read-only, the project alone is
writable, `/tmp` is private, the process runs in its own PID namespace, and the host home/repository are
not mounted. Network is intentionally retained because remote model APIs need it.

The runner receives a fixed isolated `HOME=/home/runner` and a small environment allowlist. Additional
environment variables must be explicitly named with `--runner-env NAME`; `HOME`, `PATH`, `PWD`, `USER`,
and related identity variables cannot be inherited. A runner that needs authentication or other host
state must expose the minimum required path explicitly with `--runner-ro-bind SOURCE=DEST`; destinations
are restricted beneath `/home/runner/` and are mounted read-only. Normal reports record only mount
counts/destinations, never host source paths.

The prompt additionally forbids direct Ley invocation, external memory, hidden evaluation files, and
agent-side network services unrelated to the task. The filesystem sandbox enforces the important host
boundary even if prompt instructions are ignored. For blinded runtime-secret fixtures, the hidden
contract exists only in parent-process memory / the Ley prompt during the model turn: the private Ley
filesystem has already been removed and the exact expected oracle outputs are never written into the
agent workspace.

External runner timeouts terminate the Bubblewrap namespace, including descendants that fork, double-
fork, or create a new session. A timeout, output-limit breach, or non-zero runner exit is a **failed task
attempt**, not an evaluator exception that disappears from the denominator. Runner and evaluator-
controlled stdout/stderr are bounded to prevent output-based memory/disk exhaustion. Infrastructure
errors in the evaluator itself still abort the run.

After the external runner exits, the evaluator stops trusting Git state entirely. It takes a direct
no-follow filesystem snapshot of the synthetic project and compares it with the initial snapshot, so a
runner cannot hide unauthorized changes by committing them, changing the index, or editing Git exclude
metadata. Symlinks and other special filesystem entries are rejected before evaluator-controlled code
is executed.

Visible tests and the hidden oracle probe run inside a separate Linux `bwrap` namespace with the
project mounted read-only, `/usr` mounted read-only, a private `/tmp`, no inherited evaluator
environment, and network access unshared/disabled. The oracle child receives only the public probe
module/function/inputs; the expected outputs remain in the evaluator parent and are compared only after
the sandboxed process returns. The evaluator snapshots the project again after visible tests/oracle and
requires the tree to be byte-for-byte stable.

A task passes only when all of these are true:

- the runner completed successfully;
- only fixture-approved files changed, required target files still exist, and every non-target initial
  file is byte-for-byte unchanged;
- the post-run tree contains no symlinks or unsupported special filesystem nodes;
- the fixture's visible test command passes;
- the post-agent hidden oracle passes; and
- the project tree remains unchanged while evaluator-controlled tests/oracle execute.

The current oracles are intentionally narrow and synthetic; a pass is evidence for that fixture's task
contract only.

Runner stdout/stderr are captured through anonymous temporary file descriptors and discarded after
their byte counts/hashes are computed. Normal reports retain no raw model output, no full context body,
no raw runner command, and no runtime-secret value. They record the executable name, optional
non-secret `--runner-label`, explicitly inherited environment-variable **names** (never values),
argument count, command hash, prompt/context hashes, changed-file counts plus path-set hashes, a diff
hash/byte count derived from direct filesystem before/after state and therefore also covering
committed/ignored/new file contents, visible-test/hidden-oracle results, runtime, the fixture-secret
commitment, and the Ley utility-binding metadata. Do not put API keys or other credentials in
`--runner-label`.

For independent review, `--audit-dir PATH` is an explicit post-run mode. The path must not already
exist. Nothing is written there until **all** model arms have finished. The resulting mode-0700 bundle
contains the exact materialized fixture (including the runtime-secret answer), runner command, public
report, per-arm prompt/context, raw runner/oracle streams, diff material, and a complete post-agent
project snapshot. This intentionally trades privacy for auditability; never point it at a location you
are unwilling to persist sensitive benchmark content. `--write-fixture-seed PATH` can additionally
export the run seed after the experiment using mode 0600, allowing an exact runtime-secret fixture to
be reproduced later with `--fixture-seed-file PATH`.

The Ley arm records the final task result through the existing context-utility protocol:
compile → bind → external work → visible tests/hidden oracle → typed checkpoint/session finish →
observe. The evaluator verifies the bind receipt, exact context-pack/binding identity, downstream event
IDs, revalidation state, and utility projection. That observation must continue to report:

- `contextUsageProven: false`;
- `causalUtilityProven: false`;
- `trustChangesApplied: false`; and
- `rankingChangesApplied: false`.

The evaluator never turns a model-dependent success into memory authority or ranking changes.

Repeated two-arm runs alternate which arm executes first to reduce simple order effects. A result is
not a product claim merely because `leyTaskAdvantageObserved` is true. Before using an external-agent result
to justify additional product complexity, reproduce it across multiple runs and preferably multiple
task fixtures/runners, report the exact non-secret runner label and task IDs, retain negative/no-
advantage results, and distinguish “Ley context was present before a passing outcome” from “Ley caused
the outcome.”

Validate all real-agent fixtures without invoking a model:

```text
python eval/run_agent_task_eval.py --validate
```

Run one opt-in comparison:

```text
python eval/run_agent_task_eval.py \
  --task prior-retry-delay-contract \
  --runner-label codex-luna-xhigh \
  --runner-ro-bind "$HOME/.codex/auth.json=/home/runner/.codex/auth.json" \
  --runner-command 'codex exec --ignore-user-config --ignore-rules --ephemeral -s workspace-write -m gpt-5.6-luna -c model_reasoning_effort="xhigh" -'
```

The Codex example exposes only `auth.json` read-only inside the isolated runner home. Do not mount the
whole real home directory merely for convenience. Other runners should use the same minimum-exposure
pattern for their authentication material.

`--require-ley-advantage` is a local experiment assertion on the **overall task pass rate**, not the
hidden-oracle pass rate. Normal reports expose separate baseline/Ley overall-task rates and separate
hidden-oracle attempted/passed/failed/skipped counts plus pass rate among attempted oracles. A hidden
oracle that was not run because an earlier gate failed is recorded as `skipped`, never as `failed`.
Do not add this assertion to deterministic CI or reinterpret one failed baseline / passed Ley arm as
causal proof.

## P0 capability coverage

A full-corpus run validates a matrix for each P0 capability:

- Context Compiler
- Reliable Memory Compiler
- user-authored Specifications
- Context Mounts
- origin-preserving derivation lineage
- premise/state adjudication
- revision/branch-aware retrieval
- agent-context egress policy

Every capability must retain measured adversarial, downstream, privacy, and regression evidence. The
matrix references concrete scenario/metric pairs. Missing scenarios, misspelled metrics, unsupported
expectations, or failing metric values make a full-corpus run fail. For task-facing P0 context
capabilities and origin-lineage progressive disclosure, downstream evidence must use the independent
`downstream_task_contract` described above. Memory Compiler keeps its separate crash-recovery outcome
signal because its downstream contract is successful recovery/closure of interrupted evidence rather
than read-time context selection. P1 capabilities listed in the downstream-contract section above are
likewise configuration-enforced; diagnostic/inspection surfaces such as Memory Health or Context Pack
Inspector keep their native metrics rather than being forced into an artificial task-content benchmark.
The listed P2 reusable-context capabilities are configuration-enforced in the same way.
The Context Compiler regression cell is additionally bound to the retrieval fallback/budget-ladder
scenario rather than the older single-500-token truncation check; that older fixture remains in the
full corpus as a narrower regression.

The crash-recovery representative exercises the supported candidate-bound recovery writers through the
real MCP server. It first verifies and idempotently commits one unresolved claim through the legacy
schema-v3 route, then records new bounded host evidence and commits a Decision through schema v8, a
typed Task through schema v9, and a typed Plan through schema v10. It then creates one new interrupted
evidence window supporting several facts at once and uses `ley_session_memory_verify_batch` plus
`ley_session_memory_commit_batch` to preserve a Decision, Task, Plan, and unresolved item in one
schema-v11 checkpoint. The representative requires exact replay, closed recovery-window state,
mechanically preserved recovery-candidate/turn-evidence lineage, exact durable Task/Plan state,
read-projected `unr_...` unresolved identity, and record-specific schema-v11 child lineage for both a
structured child and the unresolved child so unrelated evidence from the same atomic checkpoint is not
attributed to every child. It then records one more interrupted debugging window and uses
`ley_session_memory_verify_problem` plus `ley_session_memory_commit_problem` to preserve a rich
schema-v12 Problem episode containing expected behavior, ordered Attempts/outcomes/evidence, and an
optional Resolution. The representative requires exact retry, closed recovery-window state, durable
`session-v12.json`, and component-specific origin lineage proving an Attempt/Resolution learning does
not inherit unrelated turns from the same debugging episode. It then records a fourth independent
recovery window in which one rich Problem, failed Attempt, Resolution, Decision, and completed Task all
need to survive together. `ley_session_memory_verify_composite` plus
`ley_session_memory_commit_composite` must preserve the whole set in one schema-v13 checkpoint rather
than allowing either the rich episode or siblings to strand the other side of the window. The eval
requires `session-v13.json`, exact retry, closed-window state, full-union checkpoint lineage, and
component-specific Attempt/Decision lineage.
After all v3-v13 recovery windows are closed, the same scenario sends one real Codex Bash
`PostToolUse` payload through `ley hook`. The resulting schema-v14 observation must appear separately
as supporting Memory Compiler provenance and bounded explicit session history, keep
`totalUnconsolidatedEvidence == 0`, remain ineligible for candidate binding, preserve the existing
checkpoint count, and label the normal post-tool event only as `returned` even when the synthetic
response carries non-zero-looking metadata. The same compiler call must derive exactly one read-only
automatic Command candidate from the complete retained command, reference the exact `toe_` source row,
serialize `exitCode: null`, and carry a deterministic candidate fingerprint. The scenario then calls
`ley_session_memory_verify_observed_command` with that exact `toe_` row and event count and requires a
`review-required` result with the same fingerprint. Automatic write/Verification/outcome/semantic
proof remain false, while `candidateBindingAllowed` becomes true only because this representative remains active, has
zero current `tev_` turns, and has no sibling current tool observations. The scenario then uses the
separate write-gated `ley_session_memory_commit_observed_command` route, requires one schema-v16
Command with `exitCode: null` and exact `toe_`/source-event/observation-kind provenance, retries the
same request idempotently, and confirms that the closed window no longer returns a current automatic
Command candidate. The read-only derived candidate itself is never persisted as a separate object.
Task-, Plan-, batch-, rich-Problem-, and composite-specific secret canaries are injected into captured
host prompts and must be redacted from recovery packs and absent from durable `session-v9.json`,
`session-v10.json`, `session-v11.json`, `session-v12.json`, and `session-v13.json`. A separate Bash
secret canary and raw host tool-call ID are injected into the schema-v14 phase; both must be absent
from compiler/history output, `session-v14.json`, and the later `session-v16.json` projection. The representative therefore requires zero
privacy leakage and keeps rich/composite/tool-evidence behavior inside the existing Reliable
Memory Compiler, memory-binding, and origin-lineage gates rather than introducing weaker standalone
metrics.

The Context Mount representative also exercises reference precedence, current-state conflict
adjudication, and the complete public fail-closed availability lifecycle. Its active project carries a
user-reviewed current Constraint that says not to use Redis for startup state, while the explicitly
mounted source retains both raw captured Redis source evidence and a historical Decision to use Redis.
The raw mounted artifact remains visible as lower-authority evidence, but the historical mounted
Decision must be withheld as `conflicting-memory`, cite the exact active learning ID through
`conflictingActiveProjectEntityIds`, and increment
`mountedReferenceCoverage.activeProjectConflicts`. Active-project reviewed knowledge stays in the
ordinary active context, mounted direct evidence stays `authority: mounted-reference` /
`sourceBoundary: untrusted-mounted-project-memory`, and the compiler still discloses
`referencePrecedence: active-project-over-mounted-reference`. The conflict rule uses deterministic
opposite-polarity/high-term-overlap clauses only; it does not treat semantic similarity as truth or hide
contradictory direct source evidence.

The same mounted source is then moved and legitimately reobserved, after which a different initialized
Ley project is placed at the observed location. Compilation must report
`source-identity-changed`, keep the authorized mount diagnostically visible, search zero mounted
sources, return no mounted body, and preserve active-project context. After restoring the original
source identity, the scenario temporarily removes the observed source path and requires
`source-project-unavailable` with the same fail-closed behavior. It then restores the project,
removes only the source vault, and requires `source-vault-unavailable`. In all three degraded states
`authorizedMounts == 1`, `readyMounts == 0`, `unavailableMounts == 1`, and
`searchedMounts == 0`; active context survives, mounted content is withheld, and original/moved/
parked project or vault paths remain private. Explicit unmount finally removes the current authority.
This covers the changed-identity/unavailable-mounted-scope adversarial cases without editing Ley's
private registries by hand.

Focused subset runs validate the matrix schema but intentionally skip full result-value coverage because
not every representative scenario was executed.

## P2 capability coverage

The implemented P2 matrix covers public GitHub issue/PR connectors, commit-pinned text-document connectors, the first bounded multimodal Agent Memory evidence slice, reusable team/organization Knowledge Scopes, reusable team/organization Policy Bundles, explicit historical-host import, and the first on-demand local consolidation-review slice. `python eval/run_eval.py --p2-coverage` runs deterministic, network-free scenarios through the real CLI/MCP surfaces. Connector scenarios require authority creation without an implicit provider request, target-specific egress/non-laundering behavior, immutable document provenance, and retained restriction across remove/re-add. The multimodal scenario uses a real binary PNG under Full Evidence, writes a verification citation, mutates the live file afterward, and calls the real `ley_read_media_evidence` MCP route; it must return the exact old captured bytes through a native image block, preserve schema-v6 `mediaType` + non-text `0/0` citation semantics, report no generated description/live-source check, and leak no local path or live mutation canary. The Knowledge Scope scenario creates three real initialized/bound projects, explicitly attaches a two-source team scope, proves unrelated-project isolation and path-free Inspector attribution, and now also creates one user-reviewed current active Constraint plus one contradictory historical Decision in a shared source. The shared historical Decision must be withheld as `conflicting-memory` with the exact active learning ID while the source project's contradictory raw artifact remains available as lower-authority evidence. The same journey then applies a source-project `local-model-only` ceiling before cloud retrieval, verifies local-target access, detaches the scope, and requires retained ancestry to withhold derived historical memory and broad historical reads for cloud. The Policy Bundle scenario composes a two-source organization scope with exact approved source-Specification revisions, proves scope attachment alone does not activate policy, verifies an active-project Specification overrides conflicting bundled policy, proves a source-Specification egress block is applied before the policy file is opened, checks path/body-safe Inspector v4 attribution, and then verifies detach plus retained bundle ancestry still withholds unproven historical derivatives for cloud while an explicit local target remains allowed. The historical-host-import scenario writes a synthetic Codex global message-history JSONL containing selected, unrelated, and secret canaries; imports one explicit session UUID through the real CLI; requires schema-v7 imported turns with original timestamps and `untrusted-imported-host-history`; proves Memory Compiler read-only inspection, automatic Resume exclusion, historical rather than import-time search ranking, exact retry idempotency, changed-snapshot immutability, source-file deletion independence, zero fabricated assistant history, and zero path/raw-host-ID/unrelated/secret leakage. The local-consolidation scenario creates active and completed native sessions, requires the active session to stay excluded, retrieves only body-free stable evidence handles from the completed session through the real MCP tool, rebuilds the same deterministic inbox fingerprint, and now requires schema-v2 inspected-subset coverage (`allEligibleSessionsInspected` plus `inspectedSessionsWithUnconsolidatedEvidence`) so bounded inspection cannot masquerade as a project-wide count. It then proposes a review-required learning from the exact retained turn IDs through the independently authorized proposal path, verifies direct `turn-evidence` lineage/authority ceiling, proves the terminal session event count is unchanged, and requires zero body/path/privacy-canary leakage. Every P2 capability defines adversarial, downstream, privacy, and regression dimensions just like P0/P1.

The provider network adapter is tested separately so the normal evaluation corpus does not depend on public internet availability. Rust tests verify canonical-target revalidation, issue-vs-PR parsing, pinned-document UTF-8 mapping, response bounds, v1 registry compatibility, and structured-source tamper rejection. Multimodal core/MCP/desktop tests separately cover Full-Evidence-only retention, signature validation, exact snapshot/hash reads, original-vs-derived labeling, serialized output bounds, and historical UI inspection. Knowledge Scope core/CLI tests separately cover owner-private persistence, symlink/corruption failure, bounded membership/attachments/history, immutable/idempotent definitions, unavailable-source states, lock serialization, compiler precedence, host startup withholding, runbook export withholding, and broad MCP historical gating. Policy Bundle core/CLI/MCP/host tests separately cover immutable exact-revision definitions, parent-scope activation, active-Specification precedence, source-project/source-Specification egress-before-read, Inspector body/path privacy, detach ancestry, and broad historical non-laundering. Historical-import core/CLI tests separately cover source-session isolation, secret redaction, Minimal body omission, no-follow source handling, malformed/missing selection rejection, opaque provenance, schema-v7 projection, original timestamps, Resume/Memory-Health temporal behavior, explicit Memory Compiler boundaries, idempotent retry, and changed immutable snapshots. Consolidation core/CLI/MCP tests separately cover terminal-native selection, active/import exclusion, body/path privacy, deterministic fingerprints, direct captured-turn learning evidence, body-free rejection, direct turn-origin lineage, independent proposal consent, and terminal-session non-mutation. Real disposable CLI workflows are also used during landing verification to exercise an issue refresh plus this repository's `README.md` at an already-pushed immutable commit, without touching user project/config state. This separation keeps the deterministic corpus reproducible while still testing actual fixed-origin network paths and local binary-evidence behavior before release.

## P1 capability coverage

Implemented P1 capabilities get a separate matrix rather than weakening the completed P0 contract. The
current entries are **Topic Dossiers**, **Current Project State**, **Context Pack Inspector**, **Memory Health / Hygiene**, **Agent Legibility Map**, **Reviewed Runbooks / Skill Export**, **Verification Evidence Links**, **Branch / Worktree Controls**, **Richer Graph Relations**, and **Context / Memory Utility Feedback**. Their matrices require:

- adversarial deletion/forgetting evidence from the session-erasure scenario;
- downstream/regression success from the real authentication dossier journey, including Topic Dossier schema-v2 source-search candidate-limit/collected/omitted/source-truncation disclosure distinct from post-ranking omitted results;
- zero privacy-canary leakage from the returned dossier.
- Current Project State adversarial deletion/forgetting coverage;
- a real active-session project-state journey proving working/open-state, historical-decision, verification, exact current Specification authority handles without Specification-body copying, current→changed Specification attention/fingerprint invalidation, and privacy semantics;
- zero privacy-canary leakage from the returned project-state projection.
- Context Pack Inspector schema-v4 exact-ID reproduction, hidden-body omission, forged-ID mismatch honesty, quantitative active-project source-search loss attribution, and zero path leakage through the real MCP path.
- Memory Health schema-v5 advisory/non-destructive semantics, review/uncited/duplicate/session/unresolved/recovery signals, exact-current Procedure application outcome attention for the failed member of the real pass → fail → pass history, pass-only/version-mismatch suppression, bounded total/exact-current Procedure-application claim coverage, terminal unobserved context-utility binding attention plus bounded total/observed/unobserved binding coverage, explicit unsupported reverification/helpfulness disclosure, private recovery/task/Procedure/context-body omission, deletion fidelity, and zero path leakage through the real MCP path.
- Agent Legibility schema-v2 coverage for captured architecture, important directory, declared command, observed command, policy, schema/migration, API candidate, observability reference, current plan, and approved Specification metadata; explicit inspected-session observed-command/current-plan candidate counts plus `allSessionsInspected`; deterministic source binding; explicit no-score semantics; private Specification-body omission; deletion fidelity; and zero local-path leakage through the real MCP path.
- Reviewed Runbook coverage for explicit user-confirmed current procedure/pitfall/convention selection, deterministic runbook/source identity including cross-category CLI-order canonicalization, no authority increase, unrelated-session omission, exact-ID stale-export rejection, explicit host/egress selection, cloud blocking under local-only policy, non-installing Skill output, and zero local-path leakage through the real CLI path.
- Verification Evidence coverage for a real structured test outcome with an immutable captured-artifact citation; propagation through `ley_session_get` and Current Project State; deliberate live-file drift after the checkpoint without hash drift in Ley; explicit `liveSourceChecked: false`; and zero live-canary or local-path leakage. Unknown/uncaptured evidence paths are rejected by focused core coverage.
- Branch / Worktree Controls reuse the real divergent-branch journey. Before merge, exact `divergent` Memory Search must return only divergent-applicable history, `current-lineage` must exclude the experimental evidence, and `ley_session_get` must show the checkpoint and capture freshness as divergent. After a real `--no-ff` merge, the same retained search/session evidence must recompute to `merged` without re-ingestion. A focused candidate-cap regression additionally requires requested-revision applicability to receive admission priority before the bounded candidate cap, so stronger incompatible history cannot make an exact scope falsely empty. Every surface keeps `liveSourceChecked: false`, and local-path privacy remains zero.
- Richer Graph Relations use both the real captured one-hop implementation/importing-test fixture and
  the transitive ripple fixture described above. Direct context search remains the simpler baseline;
  the one-hop representative proves exact incoming-import discovery and the test→implementation path,
  while the regression representative proves depth-3 expansion through service/API layers to a
  transitive impacted test that depth-1/direct retrieval miss. Focused core coverage additionally
  proves ambiguous dual file matches, package imports, and project-escape paths are not promoted into
  local deterministic file relations. Graph results keep `liveSourceChecked: false` and leak no
  machine paths.
- Context / Memory Utility Feedback keeps the original real write-enabled MCP journey as its downstream baseline: start a session, compile a task pack, bind the exact logical pack before work, replay that bind idempotently, record typed checkpoint and terminal outcomes, reject a pre-binding session event as downstream evidence, observe the valid outcome pair, replay the observation idempotently, and inspect the joined bounded session projection. It requires typed completed/resolved/helped/passed outcome counts, `contextUsageProven: false`, `causalUtilityProven: false`, no trust/ranking mutation, omission of a canary present in the compiled context body, `liveSourceChecked: false`, and zero absolute-path/privacy-canary leakage. The additional `procedure-application-outcome-history` adversarial/regression representative creates one explicitly user-confirmed Procedure and three separate bound work sessions under different task/condition excerpts with verification outcomes pass → fail → pass. Every observation must claim the exact bound Procedure/version through schema v15, a fake/unbound learning claim must fail without consuming the optimistic event count, and `ley_learning_get` must retain all three exact-version histories with `procedureFollowedProven: false`, `conditionApplicabilityProven: false`, usage/causation false, no trust/ranking mutation, unchanged reviewed learning event count, and zero path leakage. Memory Health then rebuilds over those same ledgers and must expose exactly one review-only outcome-attention signal for the failed run without upgrading the observation into Procedure failure/reverification or copying the task excerpt.

As later P1 capabilities land, each should add its own adversarial, downstream, privacy, and regression
representatives before the slice is considered complete.

## Running safely

List available scenarios without executing them:

```text
python eval/run_eval.py --list
```

Run one or more exact scenarios:

```text
python eval/run_eval.py --scenario no-useful-memory-honesty
python eval/run_eval.py --scenario crash-before-session-end-resume --scenario session-erasure-derived-residue
```

Run a category:

```text
python eval/run_eval.py --category egress-policy
```

Run only the representative scenarios required by the P0 capability matrix, while still enforcing
every adversarial/downstream/privacy/regression cell:

```text
python eval/run_eval.py --p0-coverage
```

Run only the representative scenarios for currently implemented P1 capabilities while enforcing their
coverage matrix:

```text
python eval/run_eval.py --p1-coverage
```

When Ley's exact pinned local semantic model is already installed and checksum-verified, run the
separate model-enabled retrieval/index lane:

```text
python eval/run_semantic_eval.py
```

That command never installs/downloads a model and is intentionally outside the deterministic P0/P1/P2
coverage matrices.

Run the complete corpus only when the machine can tolerate it:

```text
python eval/run_eval.py
```

The harness is sequential rather than concurrent, but individual scenarios can still compile/start real
Ley processes. For constrained development machines, prefer focused runs while iterating and reserve the
full corpus for a final gate.

## Interpretation rules

- Captured snapshots are not live source; tests must not convert `liveSourceChecked: false` into a freshness claim.
- Similarity/retrieval success is not authority. Scenarios separately assert Specification, trust, premise, revision, and egress gates.
- Privacy canaries should be unique and must not appear in the query/task itself when the assertion inspects returned context.
- Erasure checks probe both public retrieval surfaces and Ley-managed vault files, while honestly excluding independent user-owned copies.
- A green scenario is evidence for the exact fixture contract, not proof of arbitrary real-world correctness.
- General benchmarks are comparative signals only; do not claim benchmark superiority without reproducible runs.
