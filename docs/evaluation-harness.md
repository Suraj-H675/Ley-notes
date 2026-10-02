# Ley end-to-end evaluation harness

Empirical status for the broader research questions in `LEY.md` §36 is tracked separately in
[`docs/research/open-validation-register.md`](research/open-validation-register.md). The register treats
this deterministic corpus as evidence, not as automatic proof that comparative UX/model questions are
closed.

Ley's executable acceptance corpus lives in `eval/fixtures/scenarios.jsonl` and is driven by
`eval/run_eval.py`. The harness exercises the real `ley` CLI, stdio MCP server, and lifecycle-hook
surfaces against isolated temporary projects/vaults. It is intentionally deterministic: unit tests
prove local invariants, while these scenarios prove multi-surface product behavior.

Host-version-sensitive MCP negotiation is validated separately by
`eval/run_mcp_host_compat_eval.py`. That lane creates a disposable Ley project and isolated temporary
Codex/Claude configuration, then uses the installed hosts' real no-model MCP status/health-check paths
to capture `initialize` negotiation and successful inventory. It does not start an LLM/model turn and
is not a deterministic CI contract because installed host versions are external moving dependencies.
Use `--require-all` when both supported host CLIs are expected to be installed.

Git revision portability/cost is measured separately by `eval/run_git_revision_compat_eval.py`. That
runner creates disposable repository shapes, puts a temporary logging `git` shim only in Ley's child
process `PATH`, and records the exact metadata commands plus end-to-end query wall time used by real
session retrieval. Its
classification and command-policy checks are deterministic; wall-clock timing is environment-sensitive
evidence. The shim is an executable Python launcher on Unix-like hosts and a temporary native
`git.exe` launcher built with `rustc` on Windows, so bare `Command::new("git")` can be instrumented
without relying on shell-command aliasing. The shared MCP evaluation transport uses thread-backed
subprocess-pipe reads rather than Unix-only `select()` pipe readiness, so the matrix does not depend on
that platform-specific I/O behavior. The full matrix passed on GitHub-hosted Linux, macOS Intel, and
Windows 2025 runners on 2026-09-25; the dated evidence and exact toolchains are recorded in the
open-validation register. Use `--require-all` for the full
correctness/command-bound gate. Normal matrix cases must also observe at least one command through the
temporary Git instrumentation shim, so a bypassed/broken logger cannot satisfy the upper-bound check
with a vacuous zero-command result. The default command bound is three subprocesses per measured query,
matching the optimized ancestry/divergence path; the higher hard ceiling exists only for explicit
diagnostic overrides.

The repository includes a manual-only GitHub Actions workflow,
`.github/workflows/portability-security.yml`, for portability and security evidence. It runs the same matrix
on six fixed standard runner labels: Ubuntu 24.04 x64/ARM64, macOS 15 Intel/Apple Silicon, and Windows
2025 x64 / Windows 11 ARM64. This keeps architecture coverage explicit without turning the experiment
into an every-push CI requirement. The workflow records the actual runner OS/architecture/image,
Git/Python/Rust/Cargo toolchain, Rust host target, and PowerShell version on Windows before running the
matrix. That workflow deliberately builds the CLI
with the non-default `eval-private-root` feature; ordinary Ley builds do not accept evaluation path
redirection.

The same manual workflow also carries focused native/private-state gates. First, the migration boundary proves
that an empty/unrelated directory cannot become a new legacy binding or explicit override, while an existing/moved
legacy vault must already validate as captured memory for the exact project before reconnect can mutate bindings.
Artifact cutover fences/imports that historical snapshot and captures current source only into native continuity.
Project-memory erasure proves that independent
user-owned Markdown/Canvas copies remain untouched. Second, `binding_process_contention` launches independent OS processes against
one binding/project-catalog pair and requires every concurrent mutation to survive. Third, Linux/macOS lanes build the ordinary
CLI before the eval-feature rebuild and run `eval/run_private_config_permissions_eval.py` with umask
`0000`; the fresh OS-native application directory must still be `0700`, current binding/continuity files
and migration lock must be `0600`, and a native-born project must not recreate the retired
`projects-v1.json` catalog. Run `36121570078` passed the earlier contention and
production-permission gates on all applicable x64 and ARM64 lanes, including both macOS architectures.
Follow-up run `36151215222` remains historical evidence for the now-retired native notebook filesystem
engine; its confinement and watcher tests are no longer part of the current matrix because that engine is
no longer registered or shipped. Project-catalog behavior remains a six-lane
Linux/macOS/Windows x64+ARM64 portability gate.

Each run also owns private temporary `XDG_CONFIG_HOME` **and** `XDG_CACHE_HOME` roots. This prevents a
developer's real Ley configuration or locally installed semantic model from silently changing which
retrieval system an acceptance scenario exercises. Deterministic scenarios therefore start with no
semantic model unless a future fixture explicitly stages one inside that run's private cache.

The Git revision portability runner additionally creates one temporary private-state root with
pre-created `config/` and `cache/` children and supplies it as `LEY_EVAL_PRIVATE_ROOT`. Feature-enabled
Ley children derive both authority registries and semantic cache from that root on every operating
system, while the runner points its XDG variables at the same children for its own direct fixture
inspection. Invalid roots fail closed and never fall back to the developer/runner profile. Unix eval
roots are mode-checked as owner-only. Windows eval roots reject reparse-point redirects and live under
the runner's per-user temporary directory. Before Ley starts, the Windows runner constructs a fresh
protected `DirectorySecurity` DACL for the root and both children, grants inheritable full control only
to the current user SID, applies it with `Set-Acl`, then independently inspects each resulting DACL with
`Get-Acl`. Unexpected trustees, deny/inherited rules, missing full control, or failed ACL inspection
abort the evaluation rather than falling back to the user profile. Core does not treat this evaluator
hardening as a general production ACL override.

## What the harness measures

The corpus contains both write-time and read/use-time checks. Current metric families include:

- retrieval recall/precision and strict token-budget enforcement;
- selective abstention when no useful memory exists;
- read-only crash/interruption recovery, session/learning mutation idempotency, and origin-lineage preservation;
- meaningful-boundary local consolidation review, direct turn-evidence lineage, and terminal-session non-mutation;
- human-intent Specification admission using exact approved Markdown, premise resistance, and revision applicability;
- parallel-session separation and cross-host durability;
- secret/cross-project/model-egress privacy violation rate;
- deletion fidelity and forgetting-residue rate across Ley-managed raw/derived retrieval surfaces;
- harmless behavior in uninitialized workspaces;
- a deterministic downstream task-evidence contract and a bounded recent-resume baseline comparison;
- explicit missing-semantic-model lexical fallback plus sparse-repository quality across strict
  500/1,500/3,000/8,000-token budgets;
- immediate and delayed repository-memory poisoning resistance without hiding captured evidence;
- cross-surface stale-write rejection plus native desktop stale learning-review protection;

`privacy_violation_rate` and `forgetting_residue_rate` are lower-is-better metrics. A zero result means
the fixture's canaries were not extractable from the probed Ley-managed surfaces; it is not a claim
that arbitrary undiscovered channels or user-owned external copies do not exist.

## Fixed-project cross-project isolation

`two-projects-cross-isolation` proves the LEY.md isolation journey across more than captured source.
The evaluator initializes independent Alpha and Beta projects/vaults, then gives Beta three distinct
private canaries: captured source content, a completed structured session/Decision, and an explicitly
user-reviewed trusted learning. None is mounted into Alpha.

Alpha is probed through canonical `ley_search` / `ley_brief` plus explicit local session/learning
inspection. All three foreign canaries must remain absent, and the task-context check serializes only
returned support rather than allowing the query text to satisfy the assertion. The scenario therefore
covers source/session/learning isolation without requiring the retired broad MCP catalog. It does not
prohibit separately retained compatibility authority whose isolation/cleanup boundaries are tested
independently.

## Whole-project erasure and human portability

Whole-project Agent Memory erasure is intentionally **desktop/user authority**, not an MCP or automatic
host capability, so its executable acceptance proof lives in the native Tauri test surface rather than
`run_eval.py`. `desktop_project_erasure_preserves_user_owned_markdown_canvas_and_binding` drives the
same registry-resolved helper used by the `erase_agent_project_memory` Tauri command against a temporary
initialized project and bound vault.

The fixture creates real captured artifacts/graph state, a structured session, and an evidence-backed
learning containing a private-memory canary. It also creates independent user-owned Markdown and JSON
Canvas copies plus ordinary project source. Erasure must return the project to `needs-capture`, remove
the complete per-project Agent Memory namespace so ordinary memory inspection becomes unavailable, and
preserve the project source, repository-local `.ley/project.json`, private vault binding, Markdown note,
and Canvas bytes exactly. The user-owned note/Canvas copies intentionally remain even when they contain
content also present in erased Agent Memory; the UI now states this explicitly so logical Ley-memory
deletion is not misrepresented as deletion of independent human-owned copies. Core tests separately
cover recapture, lifecycle-lock waiting, and symlink/no-follow defense. This native acceptance path is
the whole-project counterpart to the Python session-erasure residue scenario.

## Parallel-agent separation and reviewed reconciliation

`parallel-agent-session-separation` proves both halves of the parallel-agent acceptance journey.
Codex and Claude Code first create independent structured sessions with contradictory Decisions under
the same topic. Each checkpoint cites the same unchanged captured README only as a freshness anchor;
session inspection must show its own canary and never the other agent's canary. Before reconciliation,
the Context Compiler must report `conflicting-state` and withhold both historical Decisions from
task-supporting context rather than choosing a winner by host or recency.

That pre-review conflicting pack is also one of the deterministic explainable-failure diagnostic
representatives. The canonical Brief itself must preserve the conflicting premise state and withhold
both stable Decision IDs as `conflicting-memory` exclusions without retrieval-truncation ambiguity. This
proves an explicit historical-memory conflict boundary; it does not claim that conflict was the causal
reason for an arbitrary downstream model failure.

The evaluator then proposes one new project-level learning through the local CLI that cites the two exact
session/checkpoint pairs, explicitly reviews it as the local user, and re-reads the current surfaces. The learning must be
verified/trusted/current, corroborated by both sessions, retain a review-required automatic authority
ceiling plus `causalCompletenessProven: false`, appear in canonical Search, and enter
compiled task context as `trusted-current`. Both original session checkpoint/Decision projections must
remain byte-for-byte equivalent to their pre-review JSON projection, and the compiler must continue to
withhold both Decision IDs as `conflicting-memory` while preserving the historical conflict disclosure.
Review therefore adds a trusted current synthesis without rewriting, merging, or falsely claiming to
supersede either agent's episodic history.

Current Specification representatives use the exact approved Markdown revision as the human-intent source.
Acceptance-criteria and Verification-method headings remain ordinary source prose; the retired `acr_`/`vmd_`
derived product objects are not release claims. The egress canary places private markers inside requirement/
method prose so a blocked Specification must withhold the whole source. Historical Verification remains a
separate evidence channel and never automatically proves requirement satisfaction, method execution, semantic
coverage, current implementation, or live-source truth.

## Downstream task contract and baseline

The deterministic downstream contract is intentionally separate from a feature's own success flag.
It serializes only task-supporting context bodies and excludes the task/query text itself, so a fixture
cannot satisfy its own required marker merely by asking for that marker. A contract passes only when
the assembled context contains all declared required evidence and excludes every declared distractor.

The budgeted-quality scenario compares a 500-token Context Compiler result with two deterministic
baselines. The first is the bounded recent-resume projection; the second enumerates **all retained
sessions** through `ley_sessions_list` and reads every exact Session Context through `ley_session_get`.
The compiler passes only when its returned context bodies satisfy the downstream contract and remain
inside the requested budget. The recent-resume arm still uses a character budget chosen as an
approximate four-characters-per-token equivalent. The all-history arm must be complete/non-truncated,
retain the same required evidence, expose the deliberately irrelevant distractor markers that raw
history carries, and serialize to a larger approximate four-characters-per-token text footprint than
the compiler's reported estimate. `budget_full_history_efficiency` therefore measures deterministic
evidence selection/size efficiency, not model reasoning quality or an exact tokenizer-equivalent cost.

The P0 matrix also uses this same independent contract where a feature has a distinct downstream-use
surface:

- Context Compiler;
- Specifications;
- origin-preserving derivation lineage;
- premise/state adjudication;
- revision awareness; and
- agent-context egress policy.

Their downstream matrix cell must reference `downstream_task_contract`; the evaluator rejects a P0
configuration that aliases those cells back to the capability's adversarial/regression metric.
Representative contracts prove, for example, that a current replacement learning reaches task
context while its superseded guidance does not; divergent branch state is withheld before merge and becomes
usable only after Git proves it merged; relevant exact approved Specification source is present while an
unrelated Specification is absent; and egress-gated private context is usable for the allowed local target
while absent from blocked targets. Origin lineage uses a deliberately different form of the same contract:
after explicit user review, local learning inspection preserves the exact mechanically resolved origins,
canonical Search reports the trusted-current learning, and Brief admits it without auto-injecting uncited or
untrusted history.

The same rule is applied selectively to P1 surfaces that actually produce reusable task-facing
knowledge rather than merely diagnostics. The current P1 downstream cell is Bootstrap Specifications.
Its contract proves independently usable output rather than reusing the feature's overall pass bit:
approved intent must remain exact while prompt/live-target canaries stay withheld.

P2 applies the same independent downstream rule only to **explicit historical host import**, because that
capability itself feeds progressively readable/discoverable historical context. Its contract requires the explicitly selected host turns to remain
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
queries canonical `ley_search` and `ley_brief` at 500, 1,500, 3,000, and 8,000 tokens and
requires:

- the sparse required marker at every budget;
- `estimatedTokens <= maxTokens` at every layer/budget;
- low-level and compiler retrieval metadata to report `lexical` for overall, bounded-rerank, and
  artifact-context modes;
- no model-cache fallback reason and no compiler `semantic-fallback` gap, because the canonical
  lexical baseline intentionally does not attempt the deferred model;
- no private cache/project/vault path leakage;
- non-decreasing bounded search-result counts as budget grows; and
- strictly more retained results at 8,000 tokens than at 500.

The deterministic harness deliberately does **not** download Ley's pinned semantic model. ADR 0094 now makes
that lexical behavior the canonical native Search baseline rather than an environment-dependent fallback:
bounded cross-kind transition/native ranking does not inspect the optional model cache, and native artifact
continuity remains lexical-only. A pre-cutover transition read may still use the retained legacy artifact
semantic path as compatibility. The former opt-in `run_semantic_eval.py` lane depended on the legacy-vault
artifact semantic-index path and was retired with ADR 0093 instead of being relabeled as evidence for canonical
Search. Core semantic/index tests remain compatibility/research evidence. Any future model-assisted canonical
retrieval needs a new native-state downstream ablation against this baseline before it becomes a release lane.

## Learning mutation idempotency

`duplicate-learning-mutation-idempotency` complements the older duplicate session-event fixture.
The scenario creates one real retained checkpoint, proposes a cited learning with a caller-stable
request ID, retries the exact proposal, then deliberately reuses that request ID with changed learning
content. It requires the exact retry to return the same event/learning identity with
`replayed: true`, and the conflicting retry to be rejected rather than appended or conflated.

The same contract is then exercised through explicit user review: one confirm review is recorded,
an exact retry replays the same review event, and a same-request/different-note retry is rejected. The
final learning must remain exactly two durable events (proposal + review), with verified/trusted state.
This is idempotency evidence only. Historical Context Utility / Procedure-application experiments are no longer active deterministic release scenarios after the canonical MCP contraction; their durable event semantics remain covered by focused core/session compatibility tests without re-entering the release matrix.

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

## Known-failure reuse

`bug-diagnosis-failed-attempts` now exercises the complete deterministic progressive-disclosure path
for a previously solved failure rather than passing on nearby keywords alone. The fixture records one
structured watcher-startup Problem with an exact dead-end restart attempt, a successful event-loop
initialization attempt, explicit root cause/change/verification fields, and a snapshot-bound citation to
`src/watcher.py`. An exact-title canonical Search establishes the durable Problem/session handles. The actual
reuse contract then uses a separate incident description that is not the stored Problem title. That
paraphrased Search must recover the same durable Problem/session IDs; following the returned `sessionId` through
`ley_session_get` must expose the same Problem ID, both ordered attempts and typed outcomes, the verified
Resolution, and the immutable captured-source citation.

The evaluator then proposes a Procedure from that exact recovered Problem record, confirms it through
the explicit local user-review route, and requires both the Procedure-specific query and the same
paraphrased incident description to return the learning as verified/trusted/current reusable guidance
with its exact reviewed text while the incident query still returns the original Problem. This proves the
known-failure journey as evidence -> structured debugging episode -> reviewed reusable procedure; it
does not claim that semantic similarity alone proves a new failure has the same cause.

## Stale-learning use-time recovery

`renamed-code-invalidates-learning` now proves the stale-learning journey at use time rather than only
checking a freshness label. Before changing source, the evaluator explicitly user-confirms the
artifact-backed `old_name` learning and requires verified/trusted/current state. It then deletes the
cited source, re-ingests the project, and keeps the newer `renamed_fn` source independently
retrievable.

After that source drift, the same stable learning must remain inspectable only as historical/review
state: local `learning show/list` discloses `source-changed`/stale freshness, canonical `ley_search`
labels it `trustSignal: stale` with `trustedForReuse: false`, and `ley_brief` must not copy the obsolete
guidance into task context even when the task is phrased to make that old entry-point claim relevant.
This proves review recovery without silently deleting the historical claim or treating a source rename
as automatic semantic correction. The retired Health/derived-state products and compatibility-only
Inspector are not required to establish this safety property.

## Ten-session changing-requirement continuity

`ten-session-changing-requirements-handoff` exercises the long-horizon coding-continuity case from
LEY.md without introducing a newest-session-wins rule. The fixture records ten distinct completed
implementation sessions, alternating supported hosts, and gives every session one structured checkpoint
plus an explicit finish/handoff. Requirements evolve from remote/Redis-dependent startup toward the
final offline-local behavior.

The evaluator requires all ten sessions to remain independently addressable and completed with exactly
one checkpoint plus one finish event. A bounded local `ley resume` request for three sessions must
report `totalSessions: 10`, `omittedSessions: 7`, and return the three latest completed handoffs in
order, including the final continuation marker. The tenth checkpoint/finish also carries one explicit
unresolved live-source follow-up; that unresolved marker must survive into resume rather than being
silently treated as completed work. Separate project-activity search must still recover all
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

The same fixture now composes that long-horizon Resume with the live-source boundary in one fresh-host
continuation. After the ten historical sessions are complete, the evaluator mutates `docs/runtime.md`,
starts a new Codex lifecycle session, and requires schema-7 SessionStart to expose only stable session/retrieval
guidance: neither the final historical handoff nor the explicit unresolved marker may be auto-injected. Their
durability is already proven through the explicit bounded Resume read earlier in the scenario. The following
`UserPromptSubmit` must remain capture-only and direct the agent toward `ley_brief`; the explicit Brief for the
same task must recover the approved current Specification, remain `liveSourceChecked: false`, and expose the
`live-source-unchecked` instruction without leaking the new live marker.

Finally, the evaluator executes a real project-relative `cat` of the mutated file and forwards that
exact command/result through Codex `PostToolUse`. The fresh continuation must retain exactly one
untruncated `observationKind: returned` row with the normalized live result, while creating no checkpoint
or Verification authority and continuing to report `liveSourceChecked: false`. The
`weeks_later_continuation` therefore proves the deterministic composition of durable explicit Resume handoff,
guidance-only host startup, current-authority selection through deliberate Brief, and explicit live workspace
observation. It remains a deterministic proxy: it does not simulate elapsed wall-clock weeks or prove that a
model independently chooses the correct edit after reading the live file.

## Opt-in real-agent downstream evaluation

`eval/run_agent_task_eval.py` is the separate model-dependent downstream runner. It is deliberately
**not** part of `run_eval.py`, the P0/P1/P2 coverage matrices, or deterministic CI acceptance. It may
invoke a paid/networked external coding agent, so one green run is an observation about that exact
runner/task invocation rather than a reproducible product claim.

The runner uses only synthetic task fixtures from `eval/fixtures/agent_tasks.jsonl`. Each fixture
contains a tiny disposable repository, one prior Ley session/Decision that represents information a
returning teammate could know, a task-family label, explicit required/forbidden context markers, allowed
changed files, a visible-test command, and a hidden executable oracle. Fixtures can also declare a
runtime-secret contract. For those fixtures the
consequential answer does **not** exist in the checked-in fixture or oracle source: a fresh 32-byte
master seed is generated in the evaluator process, the hidden contract is derived from it, and only a
SHA-256 commitment is written to the normal report.

Fixture validation rejects unsafe or malformed schema fields/paths, invalid change allowlists, tasks
outside the Context Compiler query bound, and historical markers already visible in the task/live
project files. `--validate` initializes the synthetic Ley project, seeds the materialized prior memory,
compiles the real 500-token context pack by default, and requires the minimum task-relevant historical
markers to be genuinely admitted while any explicitly forbidden stale markers remain absent before any
model spend. Required and forbidden markers must both originate in the prior-memory fixture and must not
already be exposed through the task/live project surface. Script-oracle fixtures additionally carry an
evaluator-only reference solution: validation proves the buggy initial repository **fails** the hidden
oracle and the reference solution **passes** it. This catches vacuous or broken hidden tests before an
external model is invoked. The runtime-secret retry fixture still uses a seeded legacy probe, so its
consequential answer cannot be recovered from checked-in benchmark source.

For a real comparison, the runner creates fresh temporary workspaces and executes the same external
agent command in four arms by default:

- **baseline** — the synthetic code repository only, with no historical handoff/context supplied;
- **handoff** — the same repository plus a concise evaluator-created `HANDOFF.md` containing the prior
  goal/state/decision/rationale. It is ordinary repository context, not prompt-injected Ley state;
- **minimal** — the code repository plus a deliberately tiny fixture-derived continuity brief injected
  into the prompt. This is a **benchmark baseline**, not an implementation or claim that redesigned Ley
  already exists;
- **ley** — the external agent receives the same task repository plus the bounded current/full Ley context
  projection in its prompt. The prior Ley history is built in a separate private synthetic project. A
  neutral measurement session is started so instrumentation does not outrank historical task evidence;
  the exact context pack is compiled and bound; then the entire Ley project/vault/config tree is
  snapshotted into parent-process memory and removed from the filesystem before the external agent
  starts. It is restored only after the agent/oracle finish so the typed utility outcome can be
  recorded against the original binding.

The four conditions form a complexity ladder rather than a single no-memory control. Current/full Ley
must be compared not only with no history, but also with the much cheaper human handoff and minimal-brief
baselines. The comparison remains a whole-product observation; it is not a claim that only one internal
retrieval component changed. The legacy `--variant both` option remains available for exact
baseline-vs-Ley compatibility, while `--variant all` is the default.

The external runner contract is intentionally small: `--runner-command` is parsed without a shell,
receives the complete task/context prompt on stdin, and runs inside a Linux `bwrap` namespace with the
disposable project as `/workspace`. Each arm receives its own randomly named temporary workspace, and
that workspace is deleted before the next arm begins. `/usr` is read-only, the project alone is
writable, `/tmp` is private, the process runs in its own PID namespace, and the host home/repository are
not mounted. Network is intentionally retained because remote model APIs need it. The sandbox mounts
only common **public CA trust material** needed for TLS (`/etc/ssl/certs`, public CA-bundle targets, and
common extracted/cert directories when present); it does not mount broad `/etc/ssl` or `/etc/pki` trees,
which may contain host private-key material on some Linux distributions.

That network allowance is an important benchmark limitation. The outer Bubblewrap boundary proves that
the runner cannot read the host checkout/fixture/oracle files from the local filesystem, but it cannot
both allow an arbitrary remote-model API and cryptographically prove that an arbitrary runner never uses
internet tools to look up a public benchmark repository. Prefer runner configurations whose **tool**
network is disabled even when model transport is allowed, retain seeded/novel fixtures, and treat public
fixed fixtures as increasingly contamination-prone over time. For the strongest blinded studies, use a
local/offline runner or a transport setup that can allowlist only the model endpoint. Do not describe the
current generic runner as network-isolated.

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
is executed. Python `__pycache__` trees are excluded from this semantic project snapshot, and the external
runner receives `PYTHONDONTWRITEBYTECODE=1`, because interpreter bytecode generated by ordinary test runs
is reproducible runtime noise rather than a source/config mutation. Other generated files remain visible
to the snapshot unless a fixture explicitly allows them.

The handoff arm has one additional benchmark-only exception: changes to evaluator-injected `HANDOFF.md`
are ignored by changed-file constraints because that file exists solely to deliver the comparison
condition and is not part of the synthetic user repository. This exemption applies only to the handoff
arm and only to that exact path; source, tests, config, symlinks, and every other path remain enforced.

Visible tests and hidden oracles run inside a separate Linux `bwrap` namespace with the project mounted
read-only, `/usr` mounted read-only, a private `/tmp`, no inherited evaluator environment, and network
access unshared/disabled. Legacy probe oracles receive only the probe module/function/inputs; expected
outputs remain in the evaluator parent. New script oracles receive bounded Python source directly from
the evaluator process through `python -c`; that source is never materialized into the agent workspace and
can verify multi-file behavior with ordinary assertions. The evaluator snapshots the project again after
visible tests/oracle execution and requires the tree to be byte-for-byte stable.

A task passes only when all of these are true:

- the runner completed successfully;
- only fixture-approved files changed, required target files still exist, and every non-target initial
  file is byte-for-byte unchanged;
- the post-run tree contains no symlinks or unsupported special filesystem nodes;
- the fixture's visible test command passes;
- the post-agent hidden oracle passes; and
- the project tree remains unchanged while evaluator-controlled tests/oracle execute.

The current corpus contains ten fixtures: two exact-prior-contract tasks plus changed-requirement-vs-
stale-memory, avoid-known-failed-attempt, interrupted-implementation-resume, divergent-branch stale-
memory suppression, post-merge branch applicability, crash/missing-final-checkpoint recovery,
recorded Verification vs narrative claim, and explicit second-project reference without ambient
leakage. The divergent fixture uses one narrow typed setup rather than arbitrary fixture
scripting: Ley is ingested on an experimental empty commit, prior structured memory is captured there,
the repository returns to `main` and receives a distinct empty commit, and pre-model validation requires
the prior checkpoint to read back as `revisionApplicability=divergent`. The underlying task files and
typed branch topology are common across arms; only the designated context-delivery surface differs.
The handoff arm intentionally adds one `HANDOFF.md` context file, the minimal arm injects a prompt brief,
and the Ley arm keeps its structured historical state private from the external workspace.
A first end-to-end validation of this fixture exposed a compiler admission bug: divergent Decision,
Revision, and Learning candidates were already withheld, but divergent Session/Problem historical memory
could still enter active task context. The compiler now applies the same fail-closed divergent-revision
rule to every branch-bound historical semantic kind (`Session`, `Revision`, `Decision`, `Problem`,
`Verification`, and `Learning`) while leaving direct Artifact/Symbol/Dependency evidence under its
separate authority model.
Focused core coverage preserves the existing post-merge behavior: once Git proves the branch landed, the
previously divergent decision becomes eligible historical context again. The crash fixture keeps the
ordinary handoff identical in the simpler historical arms while full Ley additionally receives bounded
unconsolidated evidence from the active post-checkpoint session. The Verification fixture distinguishes a
stale narrative claim from a structured checkpoint Verification record without treating `status=passed`
as trusted/current proof. The explicit-reference fixture creates two real registered Ley projects,
selects exactly one, requires the selected marker downstream, and treats either unrelated-project canary
as a privacy failure. Historical Phase-0 used a retained Context Mount control for this task and required a
1,000-token minimum context budget where the normal compact request was 500; that remains historical
evidence for the capability, not the current implementation. The current harness instead initializes the
reference projects through native continuity, passes the selected project's exact `projectId` to one bounded
canonical `ley_search`, verifies returned citation `projectId` provenance, and renders that response as a
separate selected-source recall section. The active brief's `contextPackId` does not claim to identify this
additional response; reports record the selected-search project ID, digest, character/token estimate, result
count, and separate token budget. A pass remains evidence for that fixture's task contract only.

The corrected Phase-0 pinned-model study is recorded in
[`research/phase0-frontier-agent-benchmark-2026-09-26.md`](research/phase0-frontier-agent-benchmark-2026-09-26.md).
Its accepted sample contains 48 valid `gpt-6-luna` / `xhigh` attempts (three observations for every cell
across four representative tasks and four arms) with zero runner failures. Fifteen additional attempts
were discarded only after path hashes proved one of the two evaluator artifacts above and were replaced
under the corrected harness. The study is evidence for the capabilities exercised by those fixtures, not
a general claim that the current full Ley architecture should survive the reset.

The historical Phase-0 report remains the reproducibility record for the older full-Ley implementation; the
current `ley` arm no longer recreates retired Mount contribution. Current executable Ley arms exercise shipped
canonical surfaces, and cross-project fixtures add the same explicit selected-source `ley_search` described
above. `ley-brief` starts a real host session and supplies the full canonical active-project `ley_brief` result.

The former `ley-auto` arm is now **historical only**. ADR 0086 retired initialized-project automatic task-
history injection after the 2026-10-01 B1 study. The report schema still keeps `ley-auto` summary fields so
older B1 JSON remains readable, but current `--all` schedules exclude it and explicit `ley-auto` / `briefing`
runs fail rather than silently relabel capture-only hooks as automatic briefing. When a fixture declares an
explicit selected source, current Ley arms still receive the separate project-qualified `ley_search` and keep
Ley/private state outside the downstream runner. Forbidden-marker leakage remains a hard evaluator error for
normal Ley arms.

The opt-in `compiler-ablation` mode is different by design. It compares `ley-brief` with an experimental
benchmark-only `ley-search` arm that supplies the same host startup/session state and the same explicit
selected-source Search, but replaces active-project Brief with canonical active-project `ley_search`. The Search
arm preserves Search's own trust, revision-applicability, conflict, freshness, provenance, instruction, and
privacy metadata; it adds no compiler premise adjudication, admission exclusions, or follow-ups. In this arm,
forbidden benchmark markers are **counted instead of rejected before the model runs**, because stale/conflicting
history exposure is one of the outcomes the ablation is intended to measure. `ley-search` is not a shipped host
workflow, is excluded from normal `--all` schedules, and must not be cited as a product surface.

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

Historical pre-R3 frontier-agent runs also recorded task outcomes through the experimental Context
Utility protocol. That instrumentation never proved context use/causation and never changed trust or
ranking. It is no longer part of the canonical product contract or deterministic release matrix. A
future rerun against the minimal surface should keep benchmark outcome accounting in the benchmark
report unless a calibrated product need for utility feedback is independently re-established.

Repeated all-arm runs rotate the five current executable arms across **task × repetition** so a particular
task family is not systematically coupled to the same first arm. The former dedicated `briefing` mode is
retired with `ley-auto`; use the recorded 2026-10-01 B1 result when discussing that historical comparison.
The separate `compiler-ablation` mode alternates only `ley-brief` and the benchmark-only `ley-search` arm.
The separate `interruption-recovery` mode alternates only `ley` and experimental `ley-no-recovery`, and accepts
only `crashed-active` fixtures. Both arms seed the same structured prior session and compile the same Ley context;
the control withholds only the bounded post-checkpoint crash evidence and asserts that crash-only markers do not
leak into the supplied context. This mode is a study harness, not part of the normal five-arm comparison.
A suite report aggregates the current attempts overall,
per-task, and per-family; it also records any task IDs/families where Ley's pass rate is below at least
one simpler arm. Aggregate improvement must therefore never be used to hide stale-memory harm in a
specific family. A result is not a product claim merely because `leyTaskAdvantageObserved` is true.
Before using an external-agent result to justify additional product complexity, reproduce it across
multiple runs and preferably multiple task fixtures/runners, report the exact non-secret runner label
and task IDs, retain negative/no-advantage results, and distinguish “Ley context was present before a
passing outcome” from “Ley caused the outcome.”

Validate all real-agent fixtures without invoking a model:

```text
python eval/run_agent_task_eval.py --validate
```

Run one opt-in comparison:

```text
python eval/run_agent_task_eval.py \
  --task changed-display-name-requirement \
  --runner-label pinned-runner-model \
  --runner-ro-bind "$HOME/.codex/auth.json=/home/runner/.codex/auth.json" \
  --runner-command 'codex exec --ignore-user-config --ignore-rules --ephemeral -s workspace-write -m <pinned-model> -c model_reasoning_effort="<pinned-effort>" -'
```

Run a selected suite by repeating `--task`. Task order is preserved, duplicate selectors are rejected,
and the runner prints the total planned external-agent attempt count before the first model call:

```text
python eval/run_agent_task_eval.py \
  --task changed-display-name-requirement \
  --task avoid-naive-csv-split \
  --task resume-cache-key-migration \
  --runner-label pinned-runner-model \
  --runner-command '<runner command>'
```

Historical B1 command (no longer executable on schema-6 current builds):

```text
python eval/run_agent_task_eval.py \
  --task divergent-feature-flag-contract \
  --task verified-timeout-contract \
  --task crash-slug-normalization-contract \
  --task explicit-reference-cache-contract \
  --variant briefing \
  --first-variant ley-brief \
  --repetitions 3 \
  --runner-label pinned-runner-model \
  --runner-command '<runner command>'
```

Run the opt-in compiler-vs-retrieval ablation on a fixed task set:

```text
python eval/run_agent_task_eval.py \
  --task divergent-feature-flag-contract \
  --task verified-timeout-contract \
  --task explicit-reference-cache-contract \
  --variant compiler-ablation \
  --first-variant ley-brief \
  --repetitions 2 \
  --runner-label pinned-runner-model \
  --runner-command '<runner command>'
```

Use fixtures where Brief and Search can be compared under the same durable-memory inputs. A crash fixture may
also be included deliberately, but canonical Brief and Search both omit unconsolidated recovery bodies unless a
separate recovery surface is supplied, so interpret that family as a shared-capability limitation rather than a
clean compiler ablation. Reports expose Brief/Search task and hidden-oracle rates, mean supplied context size,
Search required-marker coverage, and Search forbidden-marker leakage. The mode is model-dependent evidence and
is never a deterministic CI gate.

Validate the matched read-only interruption-recovery study without invoking a model:

```text
python eval/run_agent_task_eval.py \
  --validate \
  --task crash-slug-normalization-contract \
  --task crash-retry-window-contract \
  --task crash-cache-namespace-contract
```

Run the opt-in recovery-vs-no-recovery pair only when external-agent quota/cost is intended:

```text
python eval/run_agent_task_eval.py \
  --task crash-slug-normalization-contract \
  --task crash-retry-window-contract \
  --task crash-cache-namespace-contract \
  --variant interruption-recovery \
  --first-variant ley \
  --repetitions 2 \
  --runner-label pinned-runner-model \
  --runner-command '<runner command>'
```

The report includes recovery/no-recovery task and hidden-oracle rates, mean runner time, mean supplied context
characters, their deltas, and `completedProcessWithFailedOracleCount`. That last metric means only that the runner
process exited normally while the hidden oracle failed; it is not evidence that the model explicitly claimed success.
The reproducibility protocol and current no-result status are recorded in
[`research/interruption-recovery-study-protocol-2026-10-02.md`](research/interruption-recovery-study-protocol-2026-10-02.md).

The first completed compiler-ablation study ran on 2026-10-01 with Codex `0.159.3`, `gpt-6-luna`, `xhigh`,
two repetitions, and three task families: divergent revision, verified-vs-claimed, and explicit selected-source
context (12 attempts total). `ley-brief` passed 6/6 tasks and 6/6 hidden oracles. The benchmark-only Search arm
passed 4/6 tasks; all four hidden oracles it reached passed, while both explicit selected-source Search runs
failed an earlier file-change constraint and skipped the oracle. Search kept full required-marker coverage but
exposed eight forbidden divergent-history markers across the two divergent runs; Brief exposed none. Mean
supplied context was 2,257.7 characters for Brief versus 3,418.0 for Search (~51.4% more for Search). This earns
the compiler/admission layer over raw retrieval on this slice, not every individual heuristic. The detailed
reproducibility note is
[`research/compiler-vs-search-ablation-2026-10-01.md`](research/compiler-vs-search-ablation-2026-10-01.md).

The first repaired post-R3 briefing study ran on 2026-10-01 with Codex `0.159.3`,
`gpt-6-luna`, `xhigh`, three representative task families (`verified-vs-claimed`,
`crash-missing-checkpoint`, and `explicit-cross-project-reference`), and two repetitions per arm
(12 model attempts total). Both `ley-brief` and `ley-auto` passed 5/6 tasks (83.3%). `ley-auto`
attempted all six hidden oracles and passed 5/6, with one genuine crash-recovery oracle failure.
`ley-brief` passed all five hidden oracles it attempted; its remaining run was rejected before the
oracle because the model modified disallowed `test_adapter.py`. Mean supplied context was 2,217.7
characters for explicit Brief and 3,607.7 for automatic context (~63% more). Selected-source Search
was separate and identical between arms, reached full required-marker coverage in both repetitions,
and leaked zero forbidden markers. This is a small single-runner repeated observation, not causal proof
or a general model-quality claim. It establishes no measured task-pass advantage for automatic
injection under this slice and should be interpreted together with context size and failure mode, not
the aggregate pass-rate tie alone.

The full checked-in corpus is intentionally **not** the implicit real-agent default. Use
`--all-tasks` explicitly when you actually intend to pay for every selected arm/repetition. By contrast,
`--validate` with no task selector still validates the full corpus because it makes no model calls.

The Codex example exposes only `auth.json` read-only inside the isolated runner home. For standalone
Codex distributions outside `/usr`, the harness also detects a sibling `codex-code-mode-host` binary and
mounts that executable read-only at `/runner/codex-code-mode-host`; it does not expose the surrounding
installation directory. Do not mount the whole real home or Codex install tree merely for convenience.
Other runners should use the same minimum-exposure pattern for authentication and required companion
executables.

`--require-ley-advantage` is a local experiment assertion on the **overall task pass rate**, not the
hidden-oracle pass rate. In an all-arm run the current Ley arm must be strictly above every simpler included
arm **and**
must not regress on any reported task or task family; in legacy `both` mode it compares baseline and Ley
under the same no-regression rule. Normal schema-v3 reports expose overall/per-task/per-family variant
summaries, hidden-oracle attempted/passed/failed/skipped counts, mean runner time, mean supplied context
characters, canonical explicit/automatic briefing pass rates and context-size summaries, regression
lists, selected task IDs/families, planned attempt count, and opt-in Brief/Search compiler-ablation summaries
while retaining the older baseline/Ley
summary fields and single-task `taskId`/`taskFamily` fields for compatibility. A hidden
oracle that was not run because an earlier gate failed is recorded as `skipped`, never as `failed`.
Do not add this assertion to deterministic CI or reinterpret one failed simpler arm / passed Ley arm as
causal proof.

## SessionStart content vs guidance-only study

The C4 startup study is **complete** and the one-off evaluator has been retired from the current tree after the
product decision. Its exact runnable harness/fixture state remains reproducible at commit
`43bcdcfe47c5f2ebd749875c54e5f99d9d1a6cbd`; the durable study record is
[`research/session-start-content-vs-guidance-2026-10-01.md`](research/session-start-content-vs-guidance-2026-10-01.md)
and the adopted product contract is ADR 0087.

The controlled study compared shipped revision-safe contentful startup with a benchmark-only guidance-only arm
while keeping the HostHook session-ID algorithm, current prompt capture, repository snapshot, runner/sandbox,
hidden oracle, and live canonical four-tool Ley MCP constant. The guidance-only arm was not forced to retrieve;
whether it called Brief/Search/Evidence was part of the measured behavior.

Across six continuity-heavy risk classes × two arms × two repetitions (24 isolated Codex `0.159.3`,
`gpt-6-luna`, `xhigh` attempts), both arms passed 12/12 tasks and 12/12 hidden oracles with zero MCP server
failures. Guidance-only startup averaged 1,188.7 context characters versus 1,983.8 for contentful startup
(~40.1% less), mean runner time was effectively tied (85.70s vs 86.13s), and mean retrieval calls were 0.83 vs
0.75. Guidance-only exposed zero forbidden stale markers; contentful startup exposed four stale same-lineage
markers across the two stale-history attempts.

That evidence earned guidance-only initialized `SessionStart` in adapter schema 7. Re-run the historical C4
harness only from its pinned source commit when investigating the study itself; do not recreate a permanent
benchmark-only contentful mode on the current product surface.

## P0 capability coverage

A full-corpus run validates the current P0 matrix for:

- Context Compiler;
- native crash/interruption recovery;
- user-authored Specifications;
- origin-preserving derivation lineage;
- premise/state adjudication;
- revision/branch-aware retrieval; and
- agent-context egress policy.

Every capability keeps adversarial, downstream, privacy, and regression evidence. Task-facing capabilities and origin-lineage use the independent `downstream_task_contract`; interruption recovery instead uses the real `crash-before-session-end-resume` outcome because its job is to preserve bounded interrupted-session evidence rather than select normal task context.

The crash representative creates an active native session with one retained prompt and no completed response, then
reads that session/turn evidence through the current local CLI and checks the canonical native MCP tool inventory.
The retained prompt must remain bounded `tev_` evidence with an untrusted source boundary and
`liveSourceChecked: false`; an `--allow-session-writes` server may expose `ley_checkpoint`, but must not regain
granular `ley_session_start` or `ley_session_memory_compile`. The interrupted request is never claimed to have
succeeded. The retained read-only Memory Compiler remains separately covered by core/MCP compatibility tests for
older/degraded continuity modes; it is not part of the fresh canonical native release surface.

Origin-lineage coverage now comes from `parallel-agent-session-separation`: two independent checkpoints remain unchanged, an explicitly reviewed project-level learning cites both exact records, its durable origin lineage keeps the automatic authority ceiling at `review-required` with causal completeness unproven, canonical Search/Brief admit the reviewed synthesis, and conflicting raw Decisions remain historical/withheld. This tests lineage through a current workflow rather than recovery-generated synthetic checkpoints.

Context Mounts, Knowledge Scopes, Policy Bundles, external connector agent reads, Context Utility, and shape-specific recovery APIs are not release-matrix capabilities after the R3 contraction. Their retained privacy/migration/cleanup obligations are covered by focused Rust/CLI regressions instead of downstream product claims.

Focused subset runs validate matrix schema but intentionally skip full result-value coverage when not every representative scenario was executed.

## P2 capability coverage

The current P2 matrix covers **multimodal Agent Memory evidence**, **explicit historical-host import**, and **local consolidation review**. `python eval/run_eval.py --p2-coverage` runs deterministic scenarios through the real CLI/MCP surfaces.

The multimodal scenario captures a real PNG under Full Evidence, mutates the live file afterward, and requires canonical `ley_search` plus citation-bound `ley_evidence` to preserve the exact historical bytes, media type, immutable snapshot/hash provenance, non-text `0/0` routing semantics, zero generated-description/live-source claims, and zero local-path/live-mutation leakage. Historical-host import requires explicit source/session selection, bounded/redacted imported user history, original timestamps, source-file independence, Resume exclusion, idempotency, and no fabricated assistant/tool/model evidence. Consolidation review keeps active/imported sessions excluded, exposes only body-free stable evidence handles from terminal native sessions, remains non-persistent/non-autonomous, and preserves separate review-required learning authority.

Retired external connector, Knowledge Scope, and Policy Bundle release scenarios are no longer P2 capability claims. Existing connector snapshots and retained scope/bundle ancestry are compatibility/privacy cleanup state; focused core/CLI tests prove no agent content contribution, retained egress ceilings, and safe local removal/detach behavior.

## P1 capability coverage

P1 has a separate matrix rather than weakening the completed P0 contract. The current entries are
**Bootstrap Specifications**, **Verification Evidence Links**, and **Branch / Worktree Controls**. Their
matrices require:

- Bootstrap Specification authority to stay explicit, read-only, source/egress bounded, and absent from
  normal initialized-project authority. Retained Bootstrap Reference grants are cleanup-only
  compatibility state and are not an agent-context capability.
- Verification Evidence to preserve immutable captured citations while canonical `ley_evidence` reads
  the exact snapshot/path/hash content; deliberate live-file drift must not rewrite historical evidence,
  and host-side current-file inspection remains a separate non-authoritative observation.
- Branch/Worktree Controls to recompute retained checkpoint/search applicability across divergent and
  merged Git states without re-ingestion, while keeping `liveSourceChecked: false`.

Context Pack Inspector is now deleted from the runtime/API after the canonical MCP contraction. Context Utility
Feedback's durable historical events remain compatibility provenance, while ADR 0090 deletes its already-disabled
model-facing MCP wrappers. Their historical scenarios/ADRs remain evidence for compatibility cleanup, not current
agent-surface claims. Retired Topic Dossier, Current Project State, Memory Health, Agent
Legibility, and dedicated graph query products likewise remain historical evidence only.

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

Run the complete retained corpus only when investigating historical/compatibility fixtures:

```text
python eval/run_eval.py
```

The retained corpus includes scenarios for product/API shapes that have since been retired, so the no-flag
command is **not** the focused product release gate and is not expected to remain all-green after deliberate
surface contraction. The current deterministic release gates are `--p0-coverage`, `--p1-coverage`, and
`--p2-coverage`, plus any focused regression scenario changed by the current work. The harness is sequential
rather than concurrent, but individual scenarios can still compile/start real Ley processes.

## Interpretation rules

- Captured snapshots are not live source; tests must not convert `liveSourceChecked: false` into a freshness claim.
- Similarity/retrieval success is not authority. Scenarios separately assert Specification, trust, premise, revision, and egress gates.
- Privacy canaries should be unique and must not appear in the query/task itself when the assertion inspects returned context.
- Erasure checks probe both public retrieval surfaces and Ley-managed vault files, while honestly excluding independent user-owned copies.
- A green scenario is evidence for the exact fixture contract, not proof of arbitrary real-world correctness.
- General benchmarks are comparative signals only; do not claim benchmark superiority without reproducible runs.
