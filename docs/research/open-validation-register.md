# Ley open validation register

Status date: 2026-10-02

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
| How much of the Context Compiler should be deterministic planning vs local/model-assisted query planning? | **Partial** | The shipped compiler keeps authority, admission, conflict, freshness, revision, egress, budgeting, and pack assembly deterministic. Canonical native Search is currently lexical-only; `retrieval-fallback-budget-ladder` proves its bounded fallback/budget behavior. The old verified-model semantic runner depended on the legacy-vault search path and was retired during ADR 0093 rather than being presented as evidence for the canonical product. | No reproducible experiment currently compares the canonical lexical baseline against vector/model-assisted retrieval or query planning under the same native authority/budget contract. Any such experiment must measure downstream value, latency/cost, and prompt-injection/failure behavior rather than assuming a planner helps. |
| What admission policy gives the best relevance/safety tradeoff without becoming an opaque second classifier stack? | **Partial** | Current admission remains explicit in canonical Brief/Search outputs: trust, authority, revision applicability, durable conflict, human-intent conflict, egress, and bounded retrieval signals stay separate. Historical Context Pack Inspector experiments provided deeper attribution, but that separate product/API is no longer part of the canonical release surface. | The repo does not yet contain a controlled policy-ablation study comparing alternative admission policies on real downstream agent success. Any future “Why” surface should be earned by that study and fit inside Evidence/Brief rather than recreate a standalone Inspector by default. |
| What is the minimum useful Specification metadata needed beyond ordinary Markdown? | **Partial** | Current direction is deliberately small: portable Markdown plus stable Specification identity and exact approved revision/hash. Acceptance-criteria / Verification-method headings remain ordinary source text; the separate derived product objects were retired. `specification-authority-context` and bootstrap Specification scenarios prove exact revision binding, authority precedence, and non-interpretation of requirement/method prose. | “Minimum” has not been established by an ablation/usability study. It is still unknown whether any current metadata can be removed without harming reliable authority/revision handling, or whether another small portable field materially improves authoring/reuse. |
| Should persistent Context Mount UX return? | **Closed for current product** | Persistent Mount content contribution was retired during R3. Retained mount/source IDs exist only as cleanup/privacy ancestry and do not resolve/search cross-project content. The current direction for any future cross-project context is explicit per-task/session source selection with visible provenance rather than a standing mount graph. | Reintroduce a persistent mount only if a controlled user/task study proves it is materially clearer or more effective than explicit source selection and justifies the extra revocation, privacy, and lifecycle surface. |
| What egress-policy vocabulary is simple enough for ordinary users while remaining enforceable through derivatives? | **Partial** | The current four-state vocabulary (`agent-ok`, `confirm-per-use`, `local-model-only`, `never-send`) is enforced through project and narrower authority scopes, including retained derivative ancestry/non-laundering. P0/P2 egress scenarios prove fail-closed enforcement. | Simplicity/comprehension is unvalidated, and `confirm-per-use` intentionally remains fail-closed until a real local confirmation flow exists. A UX study should test whether users can correctly predict cloud/local behavior and derivative blocking from the labels. |
| Which Git lineage/worktree relations can be determined cheaply and portably enough for query-time revision compatibility? | **Evidence-backed direction** | Ley currently uses bounded local Git evidence to classify `current-lineage`, `ancestor`, `merged`, `divergent`, or `unknown`. The divergent-branch journey proves applicability changes without rewriting historical memory. `eval/run_git_revision_compat_eval.py` now exercises nine disposable shapes through real `ley_session_get`: current-lineage, ancestor, detached-head ancestor, linked-worktree ancestor, divergent, merged, shallow→unknown, missing Git metadata→unknown, and missing Git binary→unknown. The matrix first exposed duplicated MCP-startup/revision work (maximum observed 4 same-head / 7 ancestry-class Git subprocesses per measured query); an evidence-backed optimization removed startup live-Git freshness and seeded the resolver with its already-computed captured relation. On 2026-09-25 the optimized matrix passed on six GitHub-hosted native combinations: Ubuntu 24.04 x64/ARM64, macOS 15 Intel/Apple Silicon, and Windows x64/ARM64, using Git 2.55.x / Python 3.14.7. All six preserved identical classifications, metadata-only commands (`status`, `rev-parse`, `merge-base`), optional locks/lazy fetch disabled, and the same maxima: 2 subprocesses for same-head, 3 for ancestry/divergent/merged/shallow—including the real linked-worktree ancestor shape—1 for missing metadata, and 0 when Git is unavailable. The intermediate C1 startup correction reused the overview resolver to filter positively divergent recent sessions without duplicating the 3-command ancestry classification; schema 7 subsequently removed historical SessionStart bodies entirely. The same revision-aware filter remains on deliberate ProjectResumePack inspection, while host startup no longer invokes it. The Windows lanes additionally drove fixes for evaluator-private state isolation, native Git-shim stdout passthrough, CRLF-safe shallow detection, malformed shallow-output fail-closed behavior, and verified evaluation-private-root DACL hardening before final green run `36120102189`. | Current evidence is strong enough for the present bounded five-state relation model across the supported hosted OS/architecture matrix, including a real linked worktree with `.git` file indirection. Continue to treat wall time and hosted image/toolchain versions as environment-sensitive rather than universal thresholds, and rerun the matrix after material Git/runner/process-launch changes or when adding broader repository/worktree shapes. The evaluator's verified Windows DACL is not a claim that ordinary production Windows config directories have an equivalent Ley-enforced ACL policy. |
| Should a named Topic Dossier product return at all? | **Closed for current product** | The named Topic Dossier surface was retired in the 2026-09-30 focused-product reset. Current topic-oriented continuity is served by bounded `ley_brief` / `ley_search` plus exact cited evidence, avoiding a second derived authority/maintenance layer. Historical ADR 0039/eval results remain evidence for the retired experiment, not a current release claim. | Reintroduce a named dossier only if a controlled downstream benchmark shows material value beyond Brief/Search and justifies its extra lifecycle, privacy, erasure, and maintenance surface. |
| Should dedicated project-graph query APIs return? | **Closed for current product** | Dedicated graph query tools and their live eval matrix were retired. Structural impact questions now use bounded Ley continuity plus the coding host's live repository tools; retained graph/snapshot state is compatibility/migration evidence, not a separate agent API. | Reintroduce graph APIs only if a task benchmark shows measurable incremental value over live workspace inspection and bounded continuity, with explicit ambiguity/cost/privacy bounds. |
| Which bounded runtime evidence types provide the most debugging value without turning Ley into a log store? | **Partial** | Current product evidence is strongest for structured Verification citations and bounded Bash supporting observations. Procedure-application and Context Utility histories remain historical/research evidence in the session schema but are no longer canonical agent APIs after R3. | There is no comparative downstream study ranking which evidence types most improve real debugging. Additional runtime/log capture or utility instrumentation should not become product surface until a blinded task benchmark demonstrates incremental value over the smaller current set. |
| How should a context pack expose inclusion/exclusion reasons without consuming excessive agent context itself? | **Open after simplification** | Historical Context Pack Inspector experiments proved that detailed attribution can be computed without copying full bodies. ADR 0089 now removes the leftover legacy Inspector runtime/API entirely; canonical Brief/Search retain compact authority/coverage signals. | Reintroduce deeper “Why” only if users/agents materially need it, preferably as bounded Evidence/Brief drill-down rather than a separate product object. Measure context cost and downstream debugging value first. |
| How should deferred Memory Compiler consolidation be scheduled so it improves capture without creating noisy micro-memories or surprising background work? | **Partial** | The current P2 step establishes an evaluated no-surprise baseline: `ley_consolidation_inbox` is an on-demand, body-free, non-persistent review surface over meaningful terminal-session evidence and can feed the separately authorized review-required learning proposal flow. | Background scheduling, batching thresholds, deduplication, idle/resource policy, notification/review UX, and benefit vs noise remain intentionally unproven. A scheduling experiment must compare any candidate background policy against this on-demand/manual baseline before a daemon/job is added. |
| Should shape-specific recovery verifiers return to the model-facing product? | **Closed for current product** | The v1–v16 verifier/commit family was retired from MCP during R3, and ADR 0091 now deletes the unreachable MCP wrapper implementation entirely. Historical deterministic core verifier/writer tests remain compatibility evidence for old recovery events, but current crash recovery is read-only Memory Compiler evidence plus live re-verification and ordinary checkpointing. | Reintroduce a semantic/structural recovery writer only if controlled downstream evidence shows that read-only interruption evidence + normal checkpointing is materially insufficient and the added authority/state-machine complexity is justified. |
| Should canonical Search expose the Problem facts that actually caused a structured debugging episode to match? | **Closed: bounded query-aware Problem excerpts adopted** | Problem ranking already searched symptom/expected state, ordered attempt action/evidence, and resolution root-cause/change/verification, but the returned preview previously collapsed resolved Problems to only the final change. A failing regression demonstrated a Problem could rank on failed-attempt/root-cause markers while exposing neither. ADR 0092 keeps the Search schema unchanged and renders deterministic labeled, per-part bounded, match-centered Problem excerpts; a 500-token regression with oversized fields now exposes both markers and keeps `trustedForReuse: false`. | Keep this focused preview behavior. Revisit only if measured downstream failures show that surrounding session/checkpoint ranking context also needs explicit match attribution or that agents require the whole structured episode in the initial Search response. |
| Should the three legacy MCP search wrappers survive beside canonical Search? | **Closed: retire wrappers** | `ley_search_memory` duplicated canonical Search; `ley_search_context` duplicated captured-evidence recall that Search/Brief plus host live-source tools already cover under the intended trust boundary; and the only concrete blocker for retiring `ley_search_activity` was its richer structured episode view. ADR 0092 plus the migrated known-failure journey now use canonical Search for stable Problem/session discovery and bounded `ley_session_get` only for full attempts/resolution/citations. ADR 0093 removes only the MCP wrappers and keeps shared core retrieval/activity state. | Do not restore parallel search route names without a concrete downstream failure that cannot be served by Search → Evidence/session detail or the host's live workspace tools. Any future richer Search output must re-earn its context/schema cost. |
| How should parallel-agent conflicts be summarized before project-level consolidation without privileging the last writer? | **Evidence-backed direction** | `parallel-agent-session-separation` preserves independent agent sessions/Decisions, compiles the pre-review state as `conflicting-state`, withholds both Decisions, and Inspector attribution names both stable Decision IDs without retrieval-truncation ambiguity. Reconciliation creates a separately reviewed project-level learning citing both exact checkpoints while leaving both histories unchanged. | Future summarization UI/text can improve presentation, but current evidence supports the core rule: preserve both histories/conflict first; only explicit reviewed synthesis becomes reusable current knowledge. |
| What derivation-dependency representation is sufficient for full-pipeline erasure without overcomplicating every record? | **Partial** | Ley proves important durable dependency paths: reviewed session erasure cascades through cited/supersession-dependent learnings; whole-project Agent Memory erasure removes the private namespace while preserving user-owned Markdown/Canvas/project files; canonical Brief/Search/Evidence are rebuilt from current durable state rather than persisted as a second derivative store; deletion-fidelity probes canonical and retained compatibility reads plus raw managed storage for zero residue. | There is no single universal derivation DAG covering every future persisted derivative. Any new persistent derivative/background cache must declare invalidation/erasure dependencies explicitly; add complexity only when a non-rebuildable derivative actually needs it. |
| When is explicit user-wide reusable knowledge worth adding beyond mounted notes/specs, and how do we avoid hidden profile inference? | **Evidence-backed minimal slice; broader composition unearned** | The corrected Phase-0 frontier-agent comparison includes an explicit second-project task: no-history, human-handoff, and minimal-brief arms were all 0/3 while the then-current full Ley/Context-Mount arm was 3/3, with an equally real unselected project absent through leakage canaries. That earned the capability, not the persistent graph. Post-R3 canonical `ley_search` now supports one exact request-scoped already-observed `projectId`; selected reads are native-ID-pinned, egress-revalidated, source-qualified in citations, and `ley_evidence` follows that citation without creating persistent sharing state. The repaired 2026-10-01 pinned-model B1 comparison gave both briefing arms the same separate selected-source Search; both selected-source repetitions had full required-marker coverage and zero forbidden-marker leakage. B2 therefore kept active-project `ley_brief` plus separate explicit selected-source Search rather than adding multi-project Brief composition. | There is no current evidence that `ley_brief(referenceProjectIds=...)`, session-persisted selection, a generic user-wide layer, persistent mount graph, or inferred profile is worth the shared-budget/precedence/revocation complexity. Reopen composition only if a concrete task/UX failure appears with the simpler request-scoped Search baseline. |
| Which multimodal evidence types justify first-class support and which should remain ordinary user attachments? | **Partial** | The first evaluated slice supports bounded original PNG/JPEG/WebP evidence under explicit Full Evidence capture. Citations preserve media type and immutable snapshot/hash; readers return original historical bytes; no OCR/vision description or live-source claim is invented. P2 and focused core/MCP/desktop tests cover this boundary. | Audio, video, PDF/document rendering, OCR, generated descriptions, embeddings, and other modalities have no evidence-backed first-class case yet. Each should remain an ordinary attachment until a task benchmark demonstrates value that cannot be obtained from existing portable files/citations. |
| When should Ley adopt MCP `2026-07-28` or later, given actual Codex/Claude host support? | **Evidence-backed direction** | On 2026-10-01 the no-model host probe used a native-born project and the same `--allow-session-writes` configuration as the packaged integrations. Codex `0.159.2` negotiated MCP `2025-06-18` and Claude Code `2.1.217` negotiated `2025-11-25`; both connected successfully to exactly the canonical four-tool surface, and Codex observed zero resources/templates. No model invocation occurred. | Keep the current server pin while supported hosts interoperate. Re-run `eval/run_mcp_host_compat_eval.py --require-all` after relevant Codex/Claude/SDK upgrades and before any protocol-pin change. |
| Which compiler features still help as frontier models improve, and which should be deleted because the model/harness no longer needs them? | **Evidence-backed direction; retrieval-only ablation completed; derived follow-up layer retired** | The deterministic harness covers ten task families, and the corrected Phase-0 pinned frontier-agent study added 48 valid downstream attempts (3 observations for every cell across four representative tasks × four arms) on Codex `0.157.1`, `gpt-6-luna`, `xhigh`. Historical final results were: no history 6/12, human handoff 4/12, minimal brief 3/12, then-current/full Ley 12/12. The repaired post-R3 briefing study then ran 12 isolated Codex `0.159.3` / `gpt-6-luna` / `xhigh` attempts and found no advantage for initialized automatic injection, so B2 retired it. The 2026-10-01 C3 study then ran 12 isolated attempts across divergent revision, verified-vs-claimed, and explicit selected-source tasks comparing canonical Brief with benchmark-only active-project Search. Brief passed 6/6 tasks and 6/6 hidden oracles; Search passed 4/6 tasks, with 4/4 attempted hidden oracles passing and two selected-source runs rejected before oracle. Search retained 100% required-marker coverage but exposed eight forbidden divergent-history markers and averaged 3,418 context characters versus 2,257.7 for Brief (~51.4% more). A later structural audit then found compiler `followUps` carried no unique information: every value duplicated admitted-item citation/session/learning fields or a premise warning's `replacementLearningId`, while several implied retired/non-canonical inspection actions. ADR 0088 therefore removes that field, its coverage/token accounting, and the retained Inspector copy; Inspector schema advances to 5. | Keep the compiler/admission layer rather than replacing Brief with raw Search. Do not restore the derived follow-up shell absent a concrete canonical action that requires unique data. Future simplification should target a specific remaining cost/rule (for example conflict/premise/admission policy) and preserve revision-aware withholding, structured evidence, egress, provenance, explicit selected-source recall, and honest bounds. Persistent mount/scope/policy graphs and other legacy breadth still must re-earn themselves. |
| Should initialized `SessionStart` keep contentful historical summaries, or become identity/guidance-only? | **Closed: guidance-only adopted** | The C4 study ran 24 isolated Codex `0.159.3` / `gpt-6-luna` / `xhigh` attempts across six continuity-heavy risk classes, two startup arms, and two repetitions while keeping HostHook session identity, current prompt capture, repository state, hidden oracle, sandbox, and live canonical four-tool Ley MCP access matched. Both arms passed 12/12 tasks and 12/12 hidden oracles with zero MCP server failures. Guidance-only averaged 1,188.7 startup characters vs 1,983.8 for contentful (~40.1% less), mean runner time was effectively tied (85.70s vs 86.13s), and mean retrieval calls were 0.83 vs 0.75. Guidance-only exposed zero forbidden stale markers; contentful startup exposed four stale same-lineage markers. | Adapter schema 7 adopts guidance-only initialized SessionStart (ADR 0087): stable session identity, deliberate retrieval/checkpoint guidance, and body-free same-session recovery signaling only. Reopen automatic startup bodies only with new controlled evidence showing a material downstream advantage that justifies unconditional disclosure/steering. |

## Questions whose current wording is now partly stale

Some §36 questions remain useful, but the repository has already selected and evaluated a first
direction. Future experiments should compare against that baseline rather than restarting from zero:

- **Git lineage/worktree relations:** the current five-state bounded relation set is implemented and
  evaluated across native x64/ARM64 Linux, macOS, and Windows hosted paths; the remaining question has
  narrowed to additional repository/worktree shapes, future Git/runner versions, and
  environment-sensitive cost.
- **Context-pack explanations:** the standalone Inspector experiment is historical; the current baseline is
  compact Brief/Search diagnostics, with any future deeper “Why” expected to live inside Evidence/Brief.
- **Parallel-agent conflicts:** the current baseline preserves both conflicting histories and requires
  explicit reviewed synthesis instead of last-writer wins.
- **Topic-oriented continuity:** the named Topic Dossier product is retired; Brief/Search/Evidence is the current baseline. Any future dossier/cache must re-earn itself through downstream value and lifecycle-cost evidence.
- **Multimodal evidence:** original bounded still-image evidence is now proven; the open question has
  narrowed to additional modalities and derived interpretation.
- **Reusable cross-project knowledge:** persistent Context Mount/Knowledge Scope/Policy Bundle content
  surfaces are retired. The current direction is explicit per-task/session source selection with inspectable
  provenance and revocation; any persistent sharing or user-global layer must re-earn its authority/lifecycle
  cost against that simpler baseline.

This does **not** require changing `LEY.md`: §36 is intentionally a durable research agenda. The register
simply records how far current evidence has moved each question.

- Routine CI run `36164225888` passed on pinned Node 24.21.0 / Rust 1.98.1 after the stale consolidation-inbox schema assertion was corrected: frontend install/typecheck/lint/tests/website+desktop builds/production audit all passed, and the Ubuntu Rust job passed format, full workspace check, and full workspace tests.

## Highest-value next experiments

### 1. Frontier-agent complexity ladder and narrower compiler ablations

Use `eval/run_agent_task_eval.py` with fixed blinded fixtures and a pinned agent/model version. The normal
current comparison uses five executable arms under identical task/oracle conditions:

1. baseline with no historical context;
2. concise human `HANDOFF.md`;
3. fixture-derived minimal continuity brief (benchmark baseline, not redesigned Ley);
4. current/full Ley compiled context;
5. canonical explicit `ley_brief`.

The first targeted internal compiler ablation is complete: canonical `ley_brief` beat benchmark-only
active-project `ley_search` 6/6 vs 4/6 tasks while using less context and withholding divergent-history markers.
The later derived `followUps` layer was removed without a model study because a structural audit proved it added
no unique information or canonical action surface. Do not rerun either result merely to accumulate samples. Open
another compiler experiment only for a specific remaining removable rule or cost target (for example a
conflict/premise/admission sublayer), and keep any such
ablation separate from the fixture-derived minimal baseline so product implementation and benchmark control are
not conflated.

Record task success, allowed-file violations, context pack ID/size, latency, model/version, and runner
configuration. Context Utility mutation is retired and should not be reintroduced merely to instrument this
benchmark. This directly informs §36 questions 1, 2,
9, and 18. It is opt-in because it may consume external model quota/cost and cannot be deterministic CI.

### 2. Explicit source-selection + egress comprehension study

If cross-project context is reintroduced, start with explicit per-task/session source selection rather than
standing Mount/Scope authority. Give users several small project/reference diagrams and ask them to predict
what a cloud vs local agent can read before/after selecting a source, applying project egress, and revoking
that selection. Measure correctness and identify wording/control states that cause wrong mental models. A
persistent mount/scope graph should be tested only as a competing arm after this simpler baseline exists.

### 3. Read-only interruption-recovery sufficiency study

Use interrupted coding tasks to compare the current baseline—bounded read-only Memory Compiler evidence,
live repository/runtime re-verification, then an ordinary checkpoint for newly supportable state—against
no retained interruption evidence. Measure task completion, false completion claims, recovery time, privacy,
and context cost. Only if this simpler path is materially insufficient should a semantic/structured recovery
writer become a competing arm. Do not reintroduce the v1–v16 verifier/commit state machine merely because its
historical deterministic tests still exist.

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
  proof covering every possible filesystem or reparse tag. The notebook filesystem engine covered by this
  run has since been retired; this is historical evidence, not a current product gate.
- Pinned-toolchain follow-up runs exposed a narrower desktop portability boundary: the project catalog
  passes on hosted Linux/macOS/Windows x64+ARM64, while Notify 8.2.0 native watcher delivery repeatedly
  produced no callback on GitHub-hosted macOS 15 Intel or Apple Silicon within an 8-second bounded
  readiness probe. Linux/Windows hosted lanes and local Linux observed the event. That was the explicit
  evidence boundary while the notebook watcher existed. The watcher has since been retired with the notebook
  filesystem surface, so the hosted-macOS callback gap is historical rather than an open focused-product
  requirement.
- Scoped portability/security run `36168018632` then passed all six hosted jobs under that explicit
  evidence boundary: macOS Intel/Apple Silicon skipped only raw watcher callback delivery while still
  passing vault confinement, project-catalog portability, cross-process registry contention, production
  private-config permissions, and revision compatibility; Linux/Windows x64+ARM64 also passed the watcher
  gate. Routine CI run `36168018551` independently passed frontend plus the full pinned Rust workspace. The
  current manual matrix replaces the retired notebook gates with moved-legacy-vault reconnect safety and
  user-owned-file preservation during Agent Memory erasure.
- The divergent-branch real-agent fixture subsequently found a compiler admission gap that narrower
  component tests had missed: divergent Session/Problem historical memory could still reach active task
  context even though divergent Decision/Revision/Learning candidates were withheld. Ley now treats all
  six branch-bound historical semantic kinds (`Session`, `Revision`, `Decision`, `Problem`,
  `Verification`, `Learning`)
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
