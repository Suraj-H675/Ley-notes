# ADR 0049: Explicit public GitHub reference connectors

Status: Accepted

## Context

P2 begins expanding Ley beyond repository-local and mounted-project evidence. External issue/PR/docs connectors are useful only if they preserve the same scope, provenance, privacy, and egress boundaries as the local Agent Memory system. A generic URL fetcher would create a much larger trust surface immediately: SSRF/private-network access, redirect handling, authentication secrets, arbitrary response formats, unclear deletion ownership, and prompt-injection persistence.

The first evaluated slice therefore needs to prove the connector authority/storage/egress model before generalizing provider or resource coverage.

## Decision

The first external connector supports **public GitHub issues and pull requests only**.

### Explicit local authority

- `ley connector add https://github.com/OWNER/REPO/issues/N ...` and `/pull/N` are the only accepted source forms.
- Add is local authority creation only and performs **no network request**.
- URLs must use exact HTTPS `github.com`, have no query/fragment, and match a strict owner/repository/positive-number grammar.
- Ley canonicalizes owner/repository case and derives a deterministic `ext_` connector ID from project identity plus canonical source. Remove/re-add of the same project/source therefore returns the same identity.
- The owner-private `external-connectors-v1.json` registry stores connector authority only. It contains stable project/source/connector metadata, not fetched bodies, project paths, vault paths, or credentials. It is locked, private, atomic, and symlink-safe.
- MCP has no connector add/refresh/remove authority.

### Explicit fixed-origin refresh

- `ley connector refresh CONNECTOR_ID ...` is the only first-slice network action.
- Network code lives in the separate `ley-github-connector` crate; `ley-core` remains network-free.
- The fetcher accepts only a core-validated structured source, reparses its canonical URL, and builds the request target itself as `https://api.github.com/repos/...`.
- GitHub redirects are disabled; no caller-controlled host/path, authentication token, cookie, arbitrary header, or private-repository credential is accepted.
- Responses are hard-bounded to 1 MiB before JSON parsing. Only title/body/state/author login/labels/upstream update time and PR merged state are projected into core input.
- Issue endpoint responses that are actually pull requests are rejected; callers must authorize the canonical `/pull/N` source instead.

### Snapshot and trust semantics

- Refresh requires existing captured project Agent Memory; it cannot create a project memory namespace independently.
- Fetched title/body/labels pass through Ley's existing secret redaction and strict character/count bounds before persistence.
- Snapshots live under the existing per-project Agent Memory namespace and are therefore covered by whole-project Agent Memory erasure.
- Snapshot IDs are content-addressed over normalized redacted source state. Rechecking unchanged provider content updates only the current refresh pointer, not immutable evidence identity.
- Returned content is labeled `untrusted-external-reference`, carries an instruction warning, and reports `liveSourceChecked: false` on stored reads. A later MCP/local read is not a provider refresh.
- Connector authority and snapshot access serialize through the connector-registry lock and project-memory lifecycle lock so removal cannot race a stale in-flight snapshot read/write.

### Agent egress

- `AgentEgressScopeKind` adds `external-connector`; the same `agent-ok`, `local-model-only`, `confirm-per-use`, and `never-send` semantics apply.
- `ley_external_connectors_list` exposes only connector metadata allowed for the configured target. A blocked connector contributes only its opaque stable ID plus policy/block reason; its URL/body is withheld.
- `ley_external_connector_get` reads only an already-captured local snapshot after project + exact connector policy checks. It performs no network request.
- There is no MCP network-refresh or connector-mutation route.
- Connector restrictions join the existing conservative non-laundering ceiling: if a connector is blocked, broad historical session/decision/problem/learning derivatives are withheld when Ley cannot prove causal independence. Direct captured active-project evidence remains governed by the active project's policy.
- Restrictive connector egress overrides remain keyed by deterministic connector ID after connector removal. Remove/re-add cannot clear the restriction implicitly; only an explicit local `ley egress connector ... agent-ok` change can do that.

## Consequences

This slice proves a narrow provider boundary without turning Ley into a web crawler or background sync service. It supports useful issue/PR references while preserving explicit authority, local snapshot provenance, project erasure, exact egress control, and network-free MCP operation.

The following remain deliberately deferred:

- arbitrary URLs or generic HTTP connectors;
- private GitHub repositories or authentication tokens;
- comments, reviews, timelines, attachments, or webhook/background refresh;
- GitHub repository-document/docs connectors;
- other issue trackers/document providers;
- connector text directly entering the Context Compiler as a new first-class retrieval channel.

Those expansions require their own evaluated scope, retention, authentication, redaction, freshness, and egress semantics rather than inheriting authority from this first slice.
