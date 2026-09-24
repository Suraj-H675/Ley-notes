# Ley open validation register

Status date: 2026-09-24

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
| Which Git lineage/worktree relations can be determined cheaply and portably enough for query-time revision compatibility? | **Partial** | Ley currently uses bounded local Git evidence to classify `current-lineage`, `ancestor`, `merged`, `divergent`, or `unknown`. The divergent-branch journey performs a real branch + `--no-ff` merge and proves applicability changes without rewriting historical memory; candidate-cap regressions protect requested-scope admission. Git failures fail soft to `unknown`, and live file contents are not read. | The relation set is evidence-backed, but “cheaply and portably enough” still needs broader cross-platform/performance sampling, especially shallow/unusual repositories and non-Linux environments. |
| When should Topic Dossiers be consolidated/rebuilt, and how can their maintenance cost be bounded? | **Evidence-backed direction** | The current P1 answer avoids hidden maintenance entirely: Topic Dossiers are on-demand, non-authoritative derived projections with bounded source-search/coverage disclosure, deterministic rebuild behavior, privacy gates, and erasure regression coverage. ADR 0039 explicitly rejects a persisted dossier cache for this evaluated slice. | A future persisted/background dossier would require a separate experiment on invalidation frequency, storage/latency cost, erasure dependencies, and whether caching materially improves outcomes. Current evidence supports rebuild-on-demand as the baseline. |
| Which project-graph relationships provide measurable downstream value beyond the current deterministic set? | **Partial** | Current evaluated additions are deliberately narrow: captured relative JS/TS imports, source-bearing re-exports, literal dynamic imports, Python relative imports, exact captured-path resolution, one-hop dependent lookup, and transitive ripple retrieval. The graph scenarios require downstream evidence contracts rather than graph-shape novelty alone. | Additional relations should be added only through the same extraction + downstream comparison discipline. The repo does not establish that broader call/type/runtime/dependency inference would improve agent outcomes enough to justify ambiguity or cost. |
| Which bounded runtime evidence types provide the most debugging value without turning Ley into a log store? | **Partial** | Ley now has evidence-backed slices for structured Verification artifact links, supported Bash tool observations, exact Procedure-application outcomes, context-utility bindings/outcomes, and observed-command recovery. These remain bounded, typed, provenance-linked, and explicitly do not convert ordinary tool returns into success authority. | There is no comparative downstream study ranking which evidence types most improve real debugging. Additional runtime/log capture should not be added until a blinded task benchmark demonstrates incremental value over the existing bounded set. |
| How should a context pack expose inclusion/exclusion reasons without consuming excessive agent context itself? | **Evidence-backed direction** | Current design keeps the task pack compact while exposing detailed reasoning through the on-demand Context Pack Inspector. Inspector v4 reproduces exact pack identity, included-record metadata, exclusions, conflicts, retrieval/revision coverage, budget composition, and follow-ups without copying included bodies. Deterministic scenarios distinguish bounded search loss, stale-memory exclusion, and historical conflict diagnostics. | The exact rendering can still be tuned, but current evidence supports the architectural split: compact task-facing diagnostics plus deeper on-demand inspection rather than embedding a full explanation manifest into every prompt. |
| How should deferred Memory Compiler consolidation be scheduled so it improves capture without creating noisy micro-memories or surprising background work? | **Partial** | The current P2 step establishes an evaluated no-surprise baseline: `ley_consolidation_inbox` is an on-demand, body-free, non-persistent review surface over meaningful terminal-session evidence and can feed the separately authorized review-required learning proposal flow. | Background scheduling, batching thresholds, deduplication, idle/resource policy, notification/review UX, and benefit vs noise remain intentionally unproven. A scheduling experiment must compare any candidate background policy against this on-demand/manual baseline before a daemon/job is added. |
| Which transition verifier can detect omission/corruption/hallucination reliably enough without requiring an expensive judge on every update? | **Partial** | Ley has increasingly strict deterministic verifiers for unresolved/minimal/typed/rich/composite recovery and observed commands: exact evidence IDs, event-count/fingerprint binding, complete-window accounting, stale/deferred/metadata-only rejection, idempotent retry, and writer-lock revalidation. These strongly detect structural omission/corruption and stale writes without a judge. | Deterministic structural verification does not prove semantic faithfulness or detect every hallucinated interpretation. A model/judge should be added only if a reproducible benchmark shows semantic errors that deterministic evidence binding cannot catch at acceptable false-positive/cost rates. |
| How should parallel-agent conflicts be summarized before project-level consolidation without privileging the last writer? | **Evidence-backed direction** | `parallel-agent-session-separation` preserves independent agent sessions/Decisions, compiles the pre-review state as `conflicting-state`, withholds both Decisions, and Inspector attribution names both stable Decision IDs without retrieval-truncation ambiguity. Reconciliation creates a separately reviewed project-level learning citing both exact checkpoints while leaving both histories unchanged. | Future summarization UI/text can improve presentation, but current evidence supports the core rule: preserve both histories/conflict first; only explicit reviewed synthesis becomes reusable current knowledge. |
| What derivation-dependency representation is sufficient for full-pipeline erasure without overcomplicating every record? | **Partial** | Ley already proves important dependency paths: reviewed session erasure cascades through cited/supersession-dependent learnings; whole-project Agent Memory erasure removes the private namespace while preserving user-owned Markdown/Canvas/project files; rebuildable dossiers/state/Inspector/graph projections do not require durable dependency rows; deletion-fidelity scenarios require zero residue. | There is no single universal derivation DAG covering every future persisted derivative. Any new persistent derivative/background cache must still declare invalidation/erasure dependencies explicitly. More complexity is justified only when a non-rebuildable derivative actually needs it. |
| When is explicit user-wide reusable knowledge worth adding beyond mounted notes/specs, and how do we avoid hidden profile inference? | **Partial** | Ley has taken an explicit-scope route instead of inferred profiling: Context Mounts, Bootstrap References, reusable team/organization Knowledge Scopes, and Policy Bundles are all opt-in authority with provenance/egress/non-laundering evaluation. No ambient whole-vault/user-profile inference is introduced. | A generic user-wide knowledge layer is still unproven. It should require a concrete cross-project task benchmark showing material value beyond explicit scopes/mounts, plus UX/privacy evidence that users can inspect and revoke exactly what became reusable. |
| Which multimodal evidence types justify first-class support and which should remain ordinary user attachments? | **Partial** | The first evaluated slice supports bounded original PNG/JPEG/WebP evidence under explicit Full Evidence capture. Citations preserve media type and immutable snapshot/hash; readers return original historical bytes; no OCR/vision description or live-source claim is invented. P2 and focused core/MCP/desktop tests cover this boundary. | Audio, video, PDF/document rendering, OCR, generated descriptions, embeddings, and other modalities have no evidence-backed first-class case yet. Each should remain an ordinary attachment until a task benchmark demonstrates value that cannot be obtained from existing portable files/citations. |
| When should Ley adopt MCP `2026-07-28` or later, given actual Codex/Claude host support? | **Open / external-validation-dependent** | As of 2026-09-24, the MCP specification's current final version is `2026-07-28`. Installed Codex `0.156.1` embeds `CODEX_MCP_PROTOCOL_VERSION` with an explicit `2026-07-28` expectation. Installed Claude Code `2.1.217` embeds MCP SDK default `2025-11-25` and older supported protocol dates, with no `2026-07-28` support string. Ley currently pins `ProtocolVersion::V_2025_11_25`. These are useful dated compatibility clues, not proof that the exact host binaries and Ley server successfully negotiate or operate together at either version. | Keep Ley's current pin unchanged until direct no-model host↔Ley handshake/operation probes establish the real compatibility intersection. Re-run those probes after relevant host/SDK updates, then run the packaged integration acceptance suite before changing the server protocol pin. Do not infer readiness from embedded version strings or the specification version alone. |
| Which compiler features still help as frontier models improve, and which should be deleted because the model/harness no longer needs them? | **Open / external-validation-dependent** | The deterministic harness proves safety/selection invariants and the opt-in `eval/run_agent_task_eval.py` provides blinded baseline-vs-Ley downstream tasks with context-utility recording. `--validate` proves fixture isolation/retrieval without model spend. | The repo does not currently contain repeated frontier-model comparison results sufficient to remove or retain features based on model capability trends. This needs periodic fixed-fixture model runs across baseline vs Ley (and ideally ablations) with version/cost/latency recorded. Safety/authority features should not be removed merely because one model succeeds without them. |

## Questions whose current wording is now partly stale

Some §36 questions remain useful, but the repository has already selected and evaluated a first
direction. Future experiments should compare against that baseline rather than restarting from zero:

- **Git lineage/worktree relations:** the current five-state bounded relation set is implemented and
  evaluated; the remaining question has narrowed to portability/cost at additional repository shapes
  and platforms.
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

## Highest-value next experiments

### 1. Frontier-agent baseline vs Ley vs targeted compiler ablations

Use `eval/run_agent_task_eval.py` with fixed blinded fixtures and a pinned agent/model version. The
current runner directly supports only baseline-vs-Ley. A third targeted ablation arm therefore requires
either a small explicit runner extension or separate orchestration that preserves identical task/source
conditions. Compare at least:

1. baseline with no Ley context;
2. normal Ley context;
3. where the harness has been extended/orchestrated for it, one targeted ablation (for example
   conflict/premise suppression removed, or a simpler retrieval-only pack).

Record task success, allowed-file violations, context pack ID/size, latency, model/version, runner
configuration, and Ley's existing context-utility outcome. This directly informs §36 questions 1, 2,
9, and 18. It is opt-in because it may consume external model quota/cost and cannot be deterministic CI.

### 2. Context Mount + egress comprehension study

Give users several small project/reference diagrams and ask them to predict what a cloud vs local agent
can read before/after mount, scope attachment, egress restriction, and detach. Compare prediction to the
actual Inspector/compiler result. Measure correctness and identify wording/control states that cause
wrong mental models. This informs §36 questions 4 and 5 without weakening the current authority model.

### 3. Blinded transition-verifier challenge set

Build a fixture set that deliberately includes:

- omitted evidence;
- duplicated evidence;
- stale event counts / fingerprints;
- corrupted or mismatched evidence IDs;
- structurally complete but semantically unsupported candidate claims.

Score deterministic verifier detection separately from expert-rated semantic faithfulness and false
positives. This directly tests whether Ley's current evidence/fingerprint verifiers already cover the
high-value failure modes and identifies the narrow residual where an expensive model/judge might add
value. Do not introduce a judge until the benchmark demonstrates a semantic gap that deterministic
verification cannot catch reliably.

The MCP host-parity question has useful dated evidence below but is **not closed**. Embedded version
strings do not prove negotiation or successful Ley operation. A direct no-model host↔Ley handshake /
operation probe remains required before changing the protocol pin, and should be repeated after relevant
host/SDK updates.

## Dated MCP compatibility evidence — 2026-09-24

- The [official MCP specification](https://modelcontextprotocol.io/specification/2026-07-28) lists
  `2026-07-28` as the current final protocol version.
- [OpenAI's current Codex MCP documentation](https://developers.openai.com/codex/mcp) confirms local
  Codex clients support stdio and Streamable
  HTTP MCP servers, OAuth, and server instructions, but does not itself publish a protocol-version
  guarantee. The installed Codex `0.156.1` binary contains `CODEX_MCP_PROTOCOL_VERSION`, an explicit
  `expected 2026-07-28` stdio-server error string, and the `MCP-Protocol-Version: 2026-07-28` header.
- [Anthropic's Claude Code MCP documentation](https://docs.anthropic.com/en/docs/claude-code/mcp)
  confirms local/remote MCP support but likewise does not
  publish a protocol-version guarantee. The installed Claude Code `2.1.217` binary embeds MCP SDK
  default `2025-11-25` plus supported older dates (`2024-11-05`, `2025-03-26`, `2025-06-18`) and does
  not embed `2026-07-28`.
- `crates/ley-mcp/src/lib.rs` pins all Ley MCP server modes to
  `ProtocolVersion::V_2025_11_25`. This is the current Ley choice, not yet a proven negotiated shared
  denominator for the exact installed Codex and Claude Code binaries.

The binary inspection is local runtime evidence for these exact installed versions, not a handshake
result and not a promise about future Codex/Claude releases. Re-check through a real no-model
initialize/tool-list operation after host upgrades and before any protocol-pin change.

## Maintenance rule

Update this register when an experiment or shipped evaluated slice materially changes the evidence for
one of the §36 questions. Do not upgrade a status merely because code was added. Preserve the remaining
uncertainty and point to the scenario/ADR/runtime evidence that justifies any stronger classification.
