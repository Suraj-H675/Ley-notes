# Ley open validation register

Status date: 2026-09-25

This document tracks the empirical questions in `LEY.md` §36 without turning implementation choices
into stronger claims than the evidence supports. `LEY.md` remains the canonical product/architecture
source of truth; this register is a current evidence map that may change as experiments improve.

The status labels are intentionally conservative:

- **Evidence-backed direction** — current implementation plus deterministic/runtime evaluation is
  strong enough to guide the present design, while future evidence may still refine it.
- **Partial** — Ley has meaningful implementation/evaluation evidence, but an important comparative,
  usability, portability, or semantic question remains open.
- **Open / external-validation-dependent** — current deterministic repository evidence cannot honestly
  answer the question yet; the missing evidence may be UX, operational, host-version, or model-dependent.

Implementation existence is not treated as empirical proof by itself. The strongest current anchors are
the deterministic corpus in `eval/run_eval.py`, its P0/P1/P2 capability matrices, focused native/core
regressions, recorded runtime verification, and the separate opt-in model-dependent evaluation lanes.

## Current status of the §36 questions

| §36 question | Status | Current evidence-backed direction | What remains to validate |
| --- | --- | --- | --- |
| How much of the Context Compiler should be deterministic planning vs local/model-assisted query planning? | **Partial** | The shipped compiler keeps authority, admission, conflict, freshness, revision, egress, budgeting, and pack assembly deterministic. Optional local semantic retrieval may improve candidate ranking without changing authority. `retrieval-fallback-budget-ladder` proves lexical fallback and budget behavior; `eval/run_semantic_eval.py` is the separate verified-model semantic lane. | No reproducible experiment currently compares deterministic task/query planning against a model-assisted planner under the same authority/budget contract. Any such experiment must measure downstream value, latency/cost, and prompt-injection/failure behavior rather than assuming a planner helps. |
| What admission policy gives the best relevance/safety tradeoff without becoming an opaque second classifier stack? | **Partial** | Current admission is explicit and inspectable: trust, authority, revision applicability, durable conflict, human-intent conflict, egress, and bounded retrieval signals remain separate. P0 adversarial/downstream/privacy/regression coverage plus Context Pack Inspector attribution exercise those rules. | The repo does not yet contain a controlled policy-ablation study comparing alternative admission policies on real downstream agent success. The opt-in real-agent runner can support that experiment, but its fixtures currently validate the harness rather than establish one globally optimal policy. |
| What is the minimum useful Specification metadata needed beyond ordinary Markdown? | **Partial** | Current direction is deliberately small: portable Markdown/YAML plus stable Specification identity, exact approved revision/hash, and read-only Acceptance-criteria / Verification-method projections. `specification-authority-context` and bootstrap Specification scenarios prove exact revision binding, authority precedence, and non-interpretation of criterion/method status. | “Minimum” has not been established by an ablation/usability study. It is still unknown whether any current metadata can be removed without harming reliable authority/revision handling, or whether another small portable field materially improves authoring/reuse. |
| What Context Mount UX is understandable enough that users can predict exactly what an agent can access? | **Partial** | The authority semantics are strongly evaluated: explicit mount identity, source-project egress, lower authority, no ambient project enumeration, poisoning resistance, detach/non-laundering, and Inspector attribution. | There is no user-comprehension study proving people can predict the access boundary from the current controls/wording. This needs a task-based UX test (for example: predict which of several project facts an agent can read before and after mount/detach), not another retrieval unit test. |
| What egress-policy vocabulary is simple enough for ordinary users while remaining enforceable through derivatives? | **Partial** | The current four-state vocabulary (`agent-ok`, `confirm-per-use`, `local-model-only`, `never-send`) is enforced through project and narrower authority scopes, including retained derivative ancestry/non-laundering. P0/P2 egress scenarios prove fail-closed enforcement. | Simplicity/comprehension is unvalidated, and `confirm-per-use` intentionally remains fail-closed until a real local confirmation flow exists. A UX study should test whether users can correctly predict cloud/local behavior and derivative blocking from the labels. |
| Which Git lineage/worktree relations can be determined cheaply and portably enough for query-time revision compatibility? | **Evidence-backed direction** | Ley currently uses bounded local Git evidence to classify `current-lineage`, `ancestor`, `merged`, `divergent`, or `unknown`. The divergent-branch journey proves applicability changes without rewriting historical memory. `eval/run_git_revision_compat_eval.py` now exercises nine disposable shapes through real `ley_session_get`: current-lineage, ancestor, detached-head ancestor, linked-worktree ancestor, divergent, merged, shallow→unknown, missing Git metadata→unknown, and missing Git binary→unknown. The matrix first exposed duplicated MCP-startup/revision work (maximum observed 4 same-head / 7 ancestry-class Git subprocesses per measured query); an evidence-backed optimization removed startup live-Git freshness and seeded the resolver with its already-computed captured relation. On 2026-09-25 the optimized matrix passed on six GitHub-hosted native combinations: Ubuntu 24.04 x64/ARM64, macOS 15 Intel/Apple Silicon, and Windows x64/ARM64, using Git 2.55.x / Python 3.14.7. All six preserved identical classifications, metadata-only commands (`status`, `rev-parse`, `merge-base`), optional locks/lazy fetch disabled, and the same maxima: 2 subprocesses for same-head, 3 for ancestry/divergent/merged/shallow—including the real linked-worktree ancestor shape—1 for missing metadata, and 0 when Git is unavailable. The Windows lanes additionally drove fixes for evaluator-private state isolation, native Git-shim stdout passthrough, CRLF-safe shallow detection, malformed shallow-output fail-closed behavior, and verified evaluation-private-root DACL hardening before final green run `36120102189`. | Current evidence is strong enough for the present bounded five-state relation model across the supported hosted OS/architecture matrix, including a real linked worktree with `.git` file indirection. Continue to treat wall time and hosted image/toolchain versions as environment-sensitive rather than universal thresholds, and rerun the matrix after material Git/runner/process-launch changes or when adding broader repository/worktree shapes. The evaluator's verified Windows DACL is not a claim that ordinary production Windows config directories have an equivalent Ley-enforced ACL policy. |
| When should Topic Dossiers be consolidated/rebuilt, and how can their maintenance cost be bounded? | **Evidence-backed direction** | The current P1 answer avoids hidden maintenance entirely: Topic Dossiers are on-demand, non-authoritative derived projections with bounded source-search/coverage disclosure, deterministic rebuild behavior, privacy gates, and erasure regression coverage. ADR 0039 explicitly rejects a persisted dossier cache for this evaluated slice. | A future persisted/background dossier would require a separate experiment on invalidation frequency, storage/latency cost, erasure dependencies, and whether caching materially improves outcomes. Current evidence supports rebuild-on-demand as the baseline. |
| Which project-graph relationships provide measurable downstream value beyond the current deterministic set? | **Partial** | Current evaluated additions are deliberately narrow: captured relative JS/TS imports, source-bearing re-exports, literal dynamic imports, Python relative imports, exact captured-path resolution, one-hop dependent lookup, and transitive ripple retrieval. The graph scenarios require downstream evidence contracts rather than graph-shape novelty alone. | Additional relations should be added only through the same extraction + downstream comparison discipline. The repo does not establish that broader call/type/runtime/dependency inference would improve agent outcomes enough to justify ambiguity or cost. |
| Which bounded runtime evidence types provide the most debugging value without turning Ley into a log store? | **Partial** | Ley now has evidence-backed slices for structured Verification artifact links, supported Bash tool observations, exact Procedure-application outcomes, context-utility bindings/outcomes, and observed-command recovery. These remain bounded, typed, provenance-linked, and explicitly do not convert ordinary tool returns into success authority. | There is no comparative downstream study ranking which evidence types most improve real debugging. Additional runtime/log capture should not be added until a blinded task benchmark demonstrates incremental value over the existing bounded set. |
| How should a context pack expose inclusion/exclusion reasons without consuming excessive agent context itself? | **Evidence-backed direction** | Current design keeps the task pack compact while exposing detailed reasoning through the on-demand Context Pack Inspector. Inspector v4 reproduces exact pack identity, included-record metadata, exclusions, conflicts, retrieval/revision coverage, budget composition, and follow-ups without copying included bodies. Deterministic scenarios distinguish bounded search loss, stale-memory exclusion, and historical conflict diagnostics. | The exact rendering can still be tuned, but current evidence supports the architectural split: compact task-facing diagnostics plus deeper on-demand inspection rather than embedding a full explanation manifest into every prompt. |
| How should deferred Memory Compiler consolidation be scheduled so it improves capture without creating noisy micro-memories or surprising background work? | **Partial** | The current P2 step establishes an evaluated no-surprise baseline: `ley_consolidation_inbox` is an on-demand, body-free, non-persistent review surface over meaningful terminal-session evidence and can feed the separately authorized review-required learning proposal flow. | Background scheduling, batching thresholds, deduplication, idle/resource policy, notification/review UX, and benefit vs noise remain intentionally unproven. A scheduling experiment must compare any candidate background policy against this on-demand/manual baseline before a daemon/job is added. |
| Which transition verifier can detect omission/corruption/hallucination reliably enough without requiring an expensive judge on every update? | **Partial** | Ley has strict deterministic verifiers for unresolved/minimal/typed/rich/composite recovery and observed commands. The new `transition-verifier-challenge-matrix` confirms through the real MCP path that omitted evidence, duplicated references, out-of-window IDs, and stale event counts are rejected, while complete evidence accounting remains reproducible. It also deliberately shows the residual boundary: a semantically opposite claim over the same complete retained evidence still reaches `review-required`, with `semanticFaithfulnessProven: false`, exactly like the aligned claim. | The benchmark now proves where deterministic verification stops. The next question is whether a semantic reviewer/judge can reliably distinguish the controlled contradicted claim from the aligned claim at acceptable false-positive, latency, privacy, and cost levels. Do not add a judge merely to repeat structural checks the deterministic verifier already catches. |
| How should parallel-agent conflicts be summarized before project-level consolidation without privileging the last writer? | **Evidence-backed direction** | `parallel-agent-session-separation` preserves independent agent sessions/Decisions, compiles the pre-review state as `conflicting-state`, withholds both Decisions, and Inspector attribution names both stable Decision IDs without retrieval-truncation ambiguity. Reconciliation creates a separately reviewed project-level learning citing both exact checkpoints while leaving both histories unchanged. | Future summarization UI/text can improve presentation, but current evidence supports the core rule: preserve both histories/conflict first; only explicit reviewed synthesis becomes reusable current knowledge. |
| What derivation-dependency representation is sufficient for full-pipeline erasure without overcomplicating every record? | **Partial** | Ley already proves important dependency paths: reviewed session erasure cascades through cited/supersession-dependent learnings; whole-project Agent Memory erasure removes the private namespace while preserving user-owned Markdown/Canvas/project files; rebuildable dossiers/state/Inspector/graph projections do not require durable dependency rows; deletion-fidelity scenarios require zero residue. | There is no single universal derivation DAG covering every future persisted derivative. Any new persistent derivative/background cache must still declare invalidation/erasure dependencies explicitly. More complexity is justified only when a non-rebuildable derivative actually needs it. |
| When is explicit user-wide reusable knowledge worth adding beyond mounted notes/specs, and how do we avoid hidden profile inference? | **Partial** | Ley has taken an explicit-scope route instead of inferred profiling: Context Mounts, Bootstrap References, reusable team/organization Knowledge Scopes, and Policy Bundles are all opt-in authority with provenance/egress/non-laundering evaluation. No ambient whole-vault/user-profile inference is introduced. | A generic user-wide knowledge layer is still unproven. It should require a concrete cross-project task benchmark showing material value beyond explicit scopes/mounts, plus UX/privacy evidence that users can inspect and revoke exactly what became reusable. |
| Which multimodal evidence types justify first-class support and which should remain ordinary user attachments? | **Partial** | The first evaluated slice supports bounded original PNG/JPEG/WebP evidence under explicit Full Evidence capture. Citations preserve media type and immutable snapshot/hash; readers return original historical bytes; no OCR/vision description or live-source claim is invented. P2 and focused core/MCP/desktop tests cover this boundary. | Audio, video, PDF/document rendering, OCR, generated descriptions, embeddings, and other modalities have no evidence-backed first-class case yet. Each should remain an ordinary attachment until a task benchmark demonstrates value that cannot be obtained from existing portable files/citations. |
| When should Ley adopt MCP `2026-07-28` or later, given actual Codex/Claude host support? | **Evidence-backed direction** | As of 2026-09-24, direct no-model probes against the installed hosts succeed with Ley's current server: Codex `0.156.1` actually sends MCP `2025-06-18`, Ley negotiates/responds `2025-06-18`, then Codex inventories 32 tools plus the project resource; Claude Code `2.1.217` sends `2025-11-25`, Ley responds `2025-11-25`, and Claude reports the server connected after `tools/list`. Ley currently pins `ProtocolVersion::V_2025_11_25`, and the server successfully negotiates the older Codex version rather than requiring every client to send the same version. | Keep the current server pin while these supported hosts interoperate. Re-run `eval/run_mcp_host_compat_eval.py --require-all` after relevant Codex/Claude/SDK upgrades and before any protocol-pin change. Move to `2026-07-28` only after the actual supported host paths negotiate/operate successfully there and the packaged integration acceptance gates remain green; do not upgrade merely because the specification is newer or a binary embeds a newer version string. |
| Which compiler features still help as frontier models improve, and which should be deleted because the model/harness no longer needs them? | **Open / external-validation-dependent** | The deterministic harness proves safety/selection invariants and the opt-in `eval/run_agent_task_eval.py` now supports four matched downstream conditions: no-history baseline, human `HANDOFF.md`, fixture-derived minimal brief, and current/full Ley. The six checked-in tasks include two exact-prior contracts plus changed-requirement/stale-memory, known-failed-attempt, interrupted multi-file resume, and divergent-branch stale-memory suppression. The divergent fixture self-verifies Ley classifies its captured prior checkpoint as `divergent`, gives the same stale decision to the two simpler historical baselines, and requires current/full Ley to withhold all listed stale-history markers. Script-oracle fixtures prove initial failure/reference-solution success before model spend. Selected multi-task suites rotate starting arms per task/repetition and report overall, per-task, and per-family outcomes plus explicit Ley regression lists; full-corpus model spend requires `--all-tasks`. | The repo still lacks repeated pinned frontier-model results across the expanded corpus, and crash-recovery, verified-vs-claimed, cross-project isolation, and downstream post-merge branch task families are not yet represented in the real-agent corpus. Safety/authority features should not be removed merely because one model succeeds without them. |

## Questions whose current wording is now partly stale

Some §36 questions remain useful, but the repository has already selected and evaluated a first
direction. Future experiments should compare against that baseline rather than restarting from zero:

- **Git lineage/worktree relations:** the current five-state bounded relation set is implemented and
  evaluated across native x64/ARM64 Linux, macOS, and Windows hosted paths; the remaining question has
  narrowed to additional repository/worktree shapes, future Git/runner versions, and
  environment-sensitive cost.
- **Context-pack explanations:** the on-demand Context Pack Inspector is now the evidence-backed baseline
  for rich reasoning without bloating every task pack.
- **Parallel-agent conflicts:** the current baseline preserves both conflicting histories and requires
  explicit reviewed synthesis instead of last-writer wins.
- **Topic Dossier maintenance:** the current safe baseline is on-demand/rebuildable; the open question is
  whether any persisted/background form produces enough value to justify its lifecycle cost.
- **Multimodal evidence:** original bounded still-image evidence is now proven; the open question has
  narrowed to additional modalities and derived interpretation.
- **Reusable cross-project knowledge:** Knowledge Scopes and Policy Bundles now provide an explicit,
  evaluated baseline beyond one-off mounts/specifications; the still-open question is whether a truly
  user-global knowledge layer adds enough value to justify another authority surface.

This does **not** require changing `LEY.md`: §36 is intentionally a durable research agenda. The register
simply records how far current evidence has moved each question.

- Routine CI run `36164225888` passed on pinned Node 24.21.0 / Rust 1.98.1 after the stale consolidation-inbox schema assertion was corrected: frontend install/typecheck/lint/tests/website+desktop builds/production audit all passed, and the Ubuntu Rust job passed format, full workspace check, and full workspace tests.

## Highest-value next experiments

### 1. Frontier-agent complexity ladder and targeted compiler ablations

Use `eval/run_agent_task_eval.py` with fixed blinded fixtures and a pinned agent/model version. Compare
the built-in four-arm ladder under identical task/oracle conditions:

1. baseline with no historical context;
2. concise human `HANDOFF.md`;
3. fixture-derived minimal continuity brief (benchmark baseline, not redesigned Ley);
4. current/full Ley compiled context.

Only after the ladder has repeated model evidence should a targeted internal compiler ablation be added
(for example conflict/premise suppression removed or a simpler retrieval-only pack). Keep any such
ablation separate from the fixture-derived minimal baseline so product implementation and benchmark
control are not conflated.

Record task success, allowed-file violations, context pack ID/size, latency, model/version, runner
configuration, and Ley's existing context-utility outcome. This directly informs §36 questions 1, 2,
9, and 18. It is opt-in because it may consume external model quota/cost and cannot be deterministic CI.

### 2. Context Mount + egress comprehension study

Give users several small project/reference diagrams and ask them to predict what a cloud vs local agent
can read before/after mount, scope attachment, egress restriction, and detach. Compare prediction to the
actual Inspector/compiler result. Measure correctness and identify wording/control states that cause
wrong mental models. This informs §36 questions 4 and 5 without weakening the current authority model.

### 3. Transition-verifier semantic faithfulness challenge

The deterministic transition-verifier challenge matrix now proves structural completeness while also
showing its deliberate semantic boundary: aligned and semantically opposite claims over the same
complete retained evidence both reach `review-required` with `semanticFaithfulnessProven: false`.
The next experiment should compare one narrowly scoped semantic reviewer/judge against that controlled
pair and additional paraphrase/negation variants. Measure false positives/negatives, abstention,
latency, privacy/egress requirements, and cost. Do not add a model judge merely to repeat deterministic
omission/ID/window checks that are already enforced locally.

This experiment is intentionally opt-in/model-dependent. The default product remains deterministic and
fail-closed until repeated evidence shows that a semantic reviewer adds enough value to justify its
privacy, latency, and operational cost.

## Dated Git revision portability evidence — 2026-09-25

- GitHub Actions run `36114468607` passed the original full eight-shape matrix on `ubuntu-24.04`,
  `macos-15-intel`, and `windows-2025`; follow-up run `36115330719` passed the expanded nine-shape
  matrix after adding a real linked-worktree ancestor case with `.git` file indirection. Final run
  `36120102189` passed that nine-shape matrix on all six native x64/ARM64 Linux/macOS/Windows lanes.
- Ubuntu: Linux x86_64, Git 2.55.0, Python 3.14.7, Rust/Cargo 1.98.1.
- macOS: Darwin x86_64 (`macos-15-intel`), Git 2.55.0, Python 3.14.7, Rust/Cargo 1.98.0.
- Windows: Windows 2025 Server AMD64, Git 2.55.0.windows.5, Python 3.14.7, Rust/Cargo 1.98.1.
- ARM64 coverage: Ubuntu 24.04 (`aarch64-unknown-linux-gnu`), macOS 15
  (`aarch64-apple-darwin`), and Windows 11 (`aarch64-pc-windows-msvc`) all passed natively.
- Every lane preserved the optimized per-query maxima: 2 Git subprocesses for same-head, 3 for
  ancestry/divergent/merged/shallow, 1 for missing Git metadata, and 0 for missing Git binary.
- Both Windows lanes independently reported `windowsPrivateRootDaclVerified: true`; the evaluator
  constructs a protected current-user-only inheritable DACL and re-reads the persisted rules before
  starting Ley. This verifies the evaluation-only private root, not ordinary production config ACLs.
- Follow-up run `36121570078` passed an eight-process private-registry contention test on all six lanes.
  Its Linux/macOS x64+ARM64 lanes also ran the ordinary production CLI with umask `0000`; each created
  `app.leynotes.desktop` as `0700` and `bindings-v1.json`, `bindings-v1.lock`, `projects-v1.json`, and
  `projects-v1.lock` as `0600`. This closes fresh POSIX mode-bit creation evidence on hosted macOS but
  does not claim broader macOS ACL semantics or permission repair for pre-existing directories.
- Follow-up run `36151215222` passed the native desktop-vault confinement gate on all six native lanes.
  Linux/macOS x64+ARM64 passed no-follow attacks covering a symlinked parent directory, a symlinked
  final Markdown target, and reserved `attachments`/`canvases`/`.trash` directories. Windows x64+ARM64
  passed real directory-junction attacks created with `mklink /J` for both a parent escape and those
  reserved directories. The targeted read/write/rename/trash paths and recursive vault scans therefore
  have hosted evidence that they do not follow these tested link/reparse escape shapes outside the
  selected vault. This remains evidence for the fixed hosted filesystems/runner families rather than a
  proof covering every possible filesystem or reparse tag.
- Pinned-toolchain follow-up runs exposed a narrower desktop portability boundary: the project catalog
  passes on hosted Linux/macOS/Windows x64+ARM64, while Notify 8.2.0 native watcher delivery repeatedly
  produced no callback on GitHub-hosted macOS 15 Intel or Apple Silicon within an 8-second bounded
  readiness probe. Linux/Windows hosted lanes and local Linux observed the event. The workflow therefore
  keeps project-catalog portability blocking on all six lanes but scopes raw watcher-delivery blocking to
  Linux/Windows. This is not evidence that real-machine macOS watcher delivery is broken or proven; it is
  an explicit hosted-runner evidence gap. Do not erase that distinction by adding arbitrary sleeps or a
  production polling backend solely for CI.
- Scoped portability/security run `36168018632` then passed all six hosted jobs under that explicit
  evidence boundary: macOS Intel/Apple Silicon skipped only raw watcher callback delivery while still
  passing vault confinement, project-catalog portability, cross-process registry contention, production
  private-config permissions, and revision compatibility; Linux/Windows x64+ARM64 also passed the watcher
  gate. Routine CI run `36168018551` independently passed frontend plus the full pinned Rust workspace.
- The divergent-branch real-agent fixture subsequently found a compiler admission gap that narrower
  component tests had missed: divergent Session/Problem historical memory could still reach active task
  context even though divergent Decision/Revision/Learning candidates were withheld. Ley now treats all
  five branch-bound historical semantic kinds (`Session`, `Revision`, `Decision`, `Problem`, `Learning`)
  as ineligible while revision applicability is `divergent`; direct Artifact/Symbol/Dependency evidence
  keeps its separate authority path. The fixture self-verifies true two-sided Git divergence, requires all
  substantive stale branch fields to stay out of the Ley pack, and still validates a normal current-lineage
  seeded-secret fixture against the same rebuilt CLI.
- The runner uses only `status`, `rev-parse`, and `merge-base`, disables optional locks/lazy fetch,
  verifies that the temporary Git shim preserves real Git stdout before measuring cases, and requires
  all normal cases to observe the instrumentation shim.
- This is runtime evidence for those fixed hosted runner families/toolchains, not a universal timing or
  all-Git-version guarantee. Re-run after material runner/Git/process-launch changes.

## Dated MCP compatibility evidence — 2026-09-24

- The [official MCP specification](https://modelcontextprotocol.io/specification/2026-07-28) lists
  `2026-07-28` as the current final protocol version.
- [OpenAI's current Codex MCP documentation](https://developers.openai.com/codex/mcp) confirms local
  Codex clients support stdio and Streamable HTTP MCP servers, OAuth, and server instructions, but does
  not itself publish a protocol-version guarantee. The direct no-model `codex app-server`
  `mcpServerStatus/list` probe with Codex `0.156.1` is stronger evidence: its real MCP client sends
  `initialize.protocolVersion = 2025-06-18`; Ley responds `2025-06-18`; Codex then completes
  `tools/list`, `resources/list`, and `resources/templates/list` and inventories 32 Ley tools plus one
  project resource.
- [Anthropic's Claude Code MCP documentation](https://docs.anthropic.com/en/docs/claude-code/mcp)
  confirms local/remote MCP support but likewise does not publish a protocol-version guarantee. The
  direct no-model `claude mcp get` health check with Claude Code `2.1.217` sends
  `initialize.protocolVersion = 2025-11-25`; Ley responds `2025-11-25`; Claude completes `tools/list`
  and reports the disposable Ley server as connected with 32 tools.
- `crates/ley-mcp/src/lib.rs` pins all Ley MCP server modes to
  `ProtocolVersion::V_2025_11_25`. The direct probes show the current server interoperates with both
  installed hosts: exact `2025-11-25` negotiation for Claude Code and negotiated `2025-06-18` for
  Codex.
- `eval/run_mcp_host_compat_eval.py --require-all` reproduces both probes with a disposable Ley
  project and isolated temporary Codex/Claude configuration, records only the negotiated/inventory
  evidence, invokes no model turn, and removes its temporary state on exit.

This is host-version-sensitive runtime evidence for the exact installed versions, not a promise about
future Codex/Claude releases. Re-run the no-model evaluator after host upgrades and before any
protocol-pin change.

## Maintenance rule

Update this register when an experiment or shipped evaluated slice materially changes the evidence for
one of the §36 questions. Do not upgrade a status merely because code was added. Preserve the remaining
uncertainty and point to the scenario/ADR/runtime evidence that justifies any stronger classification.
