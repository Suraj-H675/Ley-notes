# ADR 0038: Enforce agent-context egress separately from local storage

Status: accepted

## Context

Ley can retain project evidence, user-approved Specifications, structured sessions, learnings, and explicitly mounted reference projects locally. Local retention is not permission to hand that content to a connected model provider. Before this slice, mounts required explicit agent-context enablement and the product disclosed the provider boundary, but Ley did not have one enforceable per-source policy vocabulary that could stop otherwise retrievable content from entering an agent response.

The policy cannot live in repository text or remembered content because that would let untrusted data grant itself sharing authority. It must also be checked before derivative compilation: filtering sensitive text only after retrieval would still let that text influence ranking, conflicts, or which safer content is selected.

## Decision

Ley stores agent-egress authority in an owner-private OS-local `agent-egress-v1.json` registry. The registry contains stable project, Specification, and Context Mount identities plus policy labels only; it stores no source text, project path, or vault path. Writes are locked and atomic. MCP exposes no policy-mutation tool; the first local user-controlled surface is `ley egress`.

The first enforceable vocabulary is:

- `agent-ok` — may be supplied to the configured agent target under the normal retrieval/admission rules;
- `confirm-per-use` — denied for every agent target in this slice because Ley does not yet have a trustworthy local per-use confirmation flow;
- `local-model-only` — allowed only when the MCP process was explicitly launched with `--egress-target local`;
- `never-send` — denied for every agent target.

An unconfigured scope remains `agent-ok` for compatibility with existing installations. That compatibility default is not evidence that every future sensitive knowledge source should default to `agent-ok`.

`ley mcp` defaults to `--egress-target cloud`. `--egress-target local` is an explicit integration/user assertion made at process startup; retrieved text cannot change it and no MCP tool argument can widen it. This label is **not** runtime attestation that the host/model is technically local. Users and host packages must use the local target only for a deliberately approved local-model/runtime configuration.

Project policy is a ceiling over the whole fixed MCP scope. A blocked project cannot return tools/resources content or perform agent-originated session/learning writes for that target. The running server rechecks project egress for every agent-facing operation. The egress-registry lock is held across the protected operation so a local policy change linearizes before or after an in-flight response rather than racing against a stale snapshot.

Lifecycle hooks are an agent-egress surface too. `ley hook --host ...` therefore defaults to the `cloud` target and accepts the same explicit `--egress-target local` assertion. Project-level denial returns a host-valid no-op before host-session creation/capture. If only fine-grained historical egress is blocked, SessionStart may create/continue the stable Ley session identity but returns a content-free egress-withheld notice instead of reading/injecting resume sessions, learnings, or recovery state. Prompt/response capture remains a local storage operation when the project target itself is allowed.

Specification overrides are checked before Ley opens the approved Markdown source, checks its revision, performs task scoring, or lets it participate in conflict/admission logic. A blocked Specification can therefore produce only a bounded stable-ID policy diagnostic; its path/text cannot leak through either `ley_project_specifications` or `ley_compile_context`.

Context Mount overrides are checked before source-project memory is searched. The mounted source project's project-level policy is also a ceiling on that mount. A blocked mount therefore cannot contribute direct text or indirectly steer the compiled pack. Mount/write authority remains unchanged.

The Context Mount registry advances to schema v3 and retains a bounded history of stable mount-ID → source-project-ID pairs that were explicitly agent-enabled. Unmount removes current reference access but does not erase that provenance relation. Existing schema-v2 agent-enabled mounts are copied into mount history before the next registry mutation; legacy v1 mounts remain agent-disabled until explicitly re-added. This allows both later mount-specific restrictions and later source-project restrictions to continue constraining active-project historical derivatives even after the current mount has been removed, without retaining source text or machine paths.

The Context Compiler returns bounded `egressTarget`, `egressCoverage`, and `egressExclusions` metadata for source-level omissions. Egress diagnostics consume the same strict context budget. Project-wide denial returns no compiled pack. If any finer-grained source is blocked for the target, Ley does not pretend that generic historical session/decision/problem/learning records are independent of it: those candidates are withheld, `historicalMemoryWithheld` becomes true, and `withheldDerivedResults` reports the bounded omission count. Direct captured revision/artifact/symbol/dependency evidence remains governed by the active-project policy. Broad MCP historical readers fail closed under the same uncertainty instead of becoming a bypass path.

Restrictive Specification/mount overrides remain authoritative by stable ID even if their currently approved/mounted scope later disappears. That prevents revoke/unmount from becoming a laundering bypass. The local user may still deliberately change or clear a retained override through `ley egress`; resetting it to `agent-ok` explicitly removes that fine-grained ceiling. These rules enforce inheritance for the authority scopes Ley can currently identify before model egress: the active project, approved Specifications, mounted projects, retained fine-grained policy IDs, and stable mount/source ancestry retained from previously agent-enabled mounts. They deliberately use conservative withholding when causal independence is unknown; they do not claim generic content-fingerprint DLP or precise per-origin attribution for every historical record. Origin-preserving learning lineage remains prerequisite metadata for a future less-conservative per-record policy join.

## Consequences

- Local storage no longer implies model-sharing permission.
- `local-model-only`, `confirm-per-use`, and `never-send` cannot leak through direct Specification retrieval or a derivative Context Compiler pack to a disallowed cloud target.
- `confirm-per-use` is deliberately less convenient but honest until a real user confirmation UX and scoped confirmation token exist.
- Existing host packages remain cloud-targeted by default and therefore cannot receive `local-model-only` content accidentally.
- Policy mutation remains local/user-controlled and cannot be enabled by stored instructions.
- A restrictive project policy also blocks agent-originated writes for that process; local human/desktop operations remain separate from agent egress.
- Restrictive fine-grained sources can reduce historical-memory utility because Ley prefers withholding to laundering when causal independence is unproven.
- Future work includes desktop policy UI, genuine per-use confirmation, stronger assurance for approved local runtimes, additional note/folder/artifact source classes, and precise per-origin derivative aggregation that can safely recover utility.
