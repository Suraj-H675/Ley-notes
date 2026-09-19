# ADR 0053: Explicit reusable team and organization Policy Bundles

Status: Accepted

## Context

ADR 0052 introduced explicit reusable team/organization Knowledge Scopes for lower-precedence read-only project reference context. That solves repeated cross-project evidence reuse, but it deliberately does **not** make retrieved shared project text into policy.

Ley still needs a bounded way for a user to reuse already-approved human directives across several projects without copying them into every active project and without creating ambient organization-wide authority.

The reusable policy form must preserve Ley's existing constitutional boundaries:

- human intent must remain distinct from historical/project evidence;
- authority must never increase because project text was retrieved, summarized, or transformed;
- active-project intent must be able to override broader reusable policy;
- cross-project scope must be explicit rather than ambient;
- storage permission must remain separate from model-egress permission;
- detaching reusable policy must not erase provenance needed to prevent derivative laundering;
- bundled policy must never grant tool, filesystem, network, review, write, or egress permission.

## Decision

Ley adds an owner-private reusable **Policy Bundle** authority that composes:

1. one existing immutable `team` or `organization` Knowledge Scope; and
2. one or more exact already-approved Specifications belonging to source projects inside that scope.

A Policy Bundle is not a search result and is not inferred from scope contents. It exists only after explicit local creation.

The durable definition contains:

- a stable `pbd_` bundle ID;
- parent Knowledge Scope ID, kind, and name;
- a human-readable bundle name;
- 1–16 immutable sources;
- for each source: stable source project ID, stable Specification ID, and exact approved SHA-256 content hash. The current approved project-relative path remains owned by Specification authority and is resolved when Ley validates/reads the source rather than duplicated into this registry;
- creation time;
- explicit per-active-project attachments;
- bounded historical bundle/source-project/source-Specification ancestry after detach.

The owner-private registry is `policy-bundles-v1.json`, schema version 1. It stores no policy body and no project/vault path.

The implementation is deliberately bounded:

- at most 32 retained Policy Bundles;
- at most 16 Specification sources per bundle;
- at most 8 attached bundles per active project;
- at most 256 retained historical bundle-source ancestry records per active project.

Creating the same scope/name/exact-source-revision set is idempotent. Changing source membership or approving a different Specification revision creates a different immutable bundle rather than silently mutating an existing definition.

## Scope and activation

Policy Bundle authority composes Knowledge Scope authority rather than bypassing it.

- Every bundle source project must belong to the parent Knowledge Scope.
- The parent Knowledge Scope must already be explicitly attached to the active project before the bundle can be attached.
- Scope attachment alone does **not** activate Policy Bundle authority.
- Bundle attachment is a separate explicit local action.
- A bundle may not source the active project itself.

The local user-facing surface is:

- `ley policy-bundle create SCOPE_ID NAME --source SOURCE_PROJECT SPECIFICATION_ID...`;
- `ley policy-bundle list`;
- `ley policy-bundle attach BUNDLE_ID [ACTIVE_PROJECT]`;
- `ley policy-bundle attached [ACTIVE_PROJECT]`;
- `ley policy-bundle status [ACTIVE_PROJECT]`;
- `ley policy-bundle detach BUNDLE_ID [ACTIVE_PROJECT]`.

MCP and host lifecycle adapters expose no Policy Bundle authority mutation route.

## Exact Specification revision authority

Each bundle source is tied to the exact revision that the user already approved through Ley's Specification authority.

At bundle creation Ley resolves the source project, verifies the Specification is approved for that source project, and records the approved path/hash identity.

At compile time the source is usable only when the current Specification authority still resolves to that exact path/hash revision. Missing, changed, revoked, moved, identity-changed, or unavailable sources do not silently substitute newer text.

This preserves the distinction between:

- **approval of a source Specification revision**; and
- **authorization to reuse that approved revision through a Policy Bundle**.

Policy Bundle creation is not a second Specification-approval path.

## Context Compiler precedence

Policy Bundle Specifications are human intent, but they are intentionally lower precedence than current active-project Specifications.

The relevant human-intent order is:

1. current approved active-project Specifications;
2. explicitly attached Policy Bundle Specifications;
3. historical project memory and lower-authority evidence.

The compiler discloses:

- `policyBundlePrecedence: active-project-specification-over-policy-bundle`;
- `policyBundles`;
- `policyBundlePolicies`;
- `policyBundleExclusions`;
- `policyBundleCoverage`.

Returned bundled policies carry:

- `authority: human-intent`;
- `sourceBoundary: user-approved-policy-bundle-specification`;
- stable bundle ID;
- parent scope ID;
- source project ID/name;
- Specification ID/path/hash;
- exact-match/relevance and token accounting.

If an active-project Specification directly contradicts a bundled policy, the active-project Specification wins. The bundled policy is excluded with `contradicts-active-specification` plus stable conflicting Specification IDs rather than silently disappearing or overriding local intent.

Bundled policy also participates in the existing human-intent conflict suppression for historical memory: contradictory historical guidance is excluded as historical evidence, while direct captured evidence may still show that the implementation currently differs from intent.

## Egress and pre-read filtering

Policy Bundle authority does not bypass model-sharing restrictions.

For every bundle source, Ley checks **both**:

1. the source project's project-level agent-egress policy; and
2. the exact source Specification's agent-egress policy.

Those checks occur before the policy Markdown is opened, task-scored, conflict-scored, or admitted.

A blocked source contributes no policy text. Agent-facing diagnostics expose bounded stable-ID/policy metadata only.

The compiler distinguishes bundle-source blocks through typed exclusions and `egressCoverage.blockedPolicyBundleSources`.

An explicit `local` target may use a `local-model-only` source. `confirm-per-use` remains fail-closed until Ley has a trustworthy local confirmation flow.

## Detach ancestry and non-laundering

Detaching a Policy Bundle removes current bundle authority, but Ley intentionally retains bounded stable source ancestry for that active project.

The retained relation contains bundle/source-project/source-Specification identities, not policy text or machine paths. It exists because historical session/decision/problem/learning records may have been influenced while the bundle was attached.

When a retained bundle source project or source Specification is disallowed for the configured target, Ley conservatively applies the existing derivative rule where independence is unproven:

- task compilation withholds unproven session/decision/problem/learning derivatives;
- `egressCoverage.historicalMemoryWithheld` becomes true;
- `withheldDerivedResults` and `blockedPolicyBundleSources` disclose the ceiling;
- broad historical MCP readers fail closed;
- lifecycle `SessionStart` withholds historical startup context.

An explicit local target can still use history when the retained restriction allows it.

Detach therefore cannot be used as an egress-laundering bypass.

## Persistence and safety

`policy-bundles-v1.json` uses Ley's owner-private locked atomic authority-registry pattern.

Validation fails closed for:

- unsupported/corrupt schema;
- non-private/symlink/non-regular registry state;
- invalid bundle/scope/project/Specification IDs;
- duplicate bundle sources;
- invalid content hashes;
- source projects outside the parent scope;
- active-project self-source attachment;
- missing parent-scope attachment;
- configured source/bundle/attachment/history bounds.

Source project/vault paths continue to live only in Ley's private Project Catalog and Binding Registry. Policy Markdown remains ordinary user-owned vault content under Specification authority.

## Context Pack Inspector

Context Pack Inspector schema version 3 adds Policy Bundle attribution without becoming another policy-content store.

The Inspector exposes:

- `policyBundlePrecedence`;
- bundle/scope/source/spec/hash metadata;
- Policy Bundle exclusions;
- Policy Bundle coverage;
- Policy Bundle token contribution.

It deliberately omits admitted Policy Bundle source bodies, just as it omits admitted active Specification and project-memory bodies.

An expected context-pack ID mismatch still means the older pack was not reconstructed exactly.

## Agent and host contract

MCP and packaged Codex/Claude integrations may consume allowed Policy Bundle context from `ley_compile_context` and diagnostic attribution from `ley_context_pack_inspect`.

They must:

- preserve active-project-Specification-over-bundle precedence;
- treat bundled policy as human intent, not execution permission;
- respect source-project/source-Specification egress exclusions;
- not reconstruct withheld historical derivatives;
- not create, list, attach, detach, or otherwise mutate Policy Bundle authority through the agent workflow.

Bundle lifecycle remains an explicit local-user CLI workflow.

## Deliberately deferred

This slice does not add:

- a dedicated desktop Policy Bundle manager;
- automatic policy discovery from project text;
- automatic bundle creation from Knowledge Scope contents;
- remote accounts, synchronization, organization membership, roles, or access-control servers;
- shared writable sessions/learnings;
- policy-driven tool/filesystem/network/write/review permission;
- automatic conflict winner selection beyond active-project Specification precedence;
- cross-device collaborative policy editing.

Those require separate product/security/evaluation decisions.

## Evaluation

P2 coverage includes the deterministic `team-organization-policy-bundle` scenario through the real CLI and stdio MCP surfaces.

Passing requires:

- no bundle policy before explicit bundle attachment;
- scope attachment alone not activating bundled policy;
- immutable/idempotent bundle creation over exact approved Specification revisions;
- unrelated project/policy isolation;
- active-project Specification precedence over conflicting bundled policy;
- path/body-safe Context Pack Inspector schema-v3 attribution;
- source-Specification `local-model-only` enforcement before policy file read;
- explicit local-target access;
- detach removing current Policy Bundle context;
- retained source ancestry withholding unproven historical derivatives and broad historical readers for cloud;
- zero privacy-canary violations.

Focused core, CLI, MCP, host, Inspector, locking, corruption, permission, bound, and egress regressions independently exercise the same boundary.
