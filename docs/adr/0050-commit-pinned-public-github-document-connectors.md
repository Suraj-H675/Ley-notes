# ADR 0050: Commit-pinned public GitHub document connectors

Status: Accepted

## Context

ADR 0049 established the first external connector authority/storage/egress boundary using public GitHub issues and pull requests. P2 also calls for external docs connectors, but accepting arbitrary documentation URLs or mutable branch/tag URLs would widen the network and freshness model substantially. Generic web URLs create SSRF/redirect/content-type risks, while a branch-based repository document can change after Ley records its authority and make provenance ambiguous.

## Decision

Ley extends the existing public GitHub connector with one immutable documentation form:

`https://github.com/OWNER/REPO/blob/FULL_40_HEX_COMMIT/PATH`

The document must satisfy all of the following:

- the host is exactly `https://github.com`;
- the revision is a full 40-hex commit SHA, canonicalized to lowercase;
- the path is 1–512 safe relative ASCII characters split into non-empty segments;
- `.` / `..`, percent-encoded names, query strings, fragments, and unsafe control/path characters are rejected;
- the path ends in `.md`, `.mdx`, `.txt`, `.rst`, or `.adoc`;
- branch names, tags, abbreviated SHAs, arbitrary GitHub pages, and non-GitHub documentation sites are not accepted.

The existing deterministic connector identity remains project ID + canonical source URL, so document remove/re-add behavior inherits the same retained egress semantics as issue/PR connectors. Existing v1 issue/PR registry JSON remains readable: `number` becomes an optional source field, while optional `revision` and `path` fields are absent for legacy issue/PR entries.

## Network boundary

The network adapter reparses the canonical source before every refresh and derives exactly:

`https://raw.githubusercontent.com/OWNER/REPO/FULL_COMMIT/PATH`

It does not accept caller-supplied fetch URLs. Redirects remain disabled, authentication is unsupported, and the response remains hard-bounded to 1 MiB before parsing. Document responses must be valid UTF-8. The core snapshot layer then applies its existing secret redaction/control-character checks and bounded text storage; documents larger than the connector snapshot body budget fail closed rather than being silently truncated.

## Snapshot semantics

A document snapshot stores:

- the exact canonical source with commit SHA and path;
- `title` equal to the authorized path;
- the bounded/redacted UTF-8 body;
- no issue/PR state, author, labels, source-update timestamp, or merged flag.

The immutable snapshot ID remains content-addressed over normalized redacted state. The source commit is already immutable provenance, so Ley does not fabricate a provider “updated at” timestamp for documents. Stored reads remain `untrusted-external-reference` with `liveSourceChecked: false`: reading a captured document does not prove that the repository's current branch still points to that commit or that any live file matches it.

## Agent and authority boundary

No new MCP authority is introduced. `ley_external_connectors_list` and `ley_external_connector_get` continue to list/read allowed local snapshots only. Connector creation, provider refresh, removal, and egress mutation remain explicit local CLI operations. Document content is untrusted evidence and grants no project policy, instruction, filesystem, network, review, write, or egress permission.

The same connector-specific egress policy and conservative historical non-laundering ceiling apply. A blocked document connector hides its URL/body from that agent target and can withhold historical derivatives whose independence is unproven; direct captured active-project evidence remains separately scoped.

## Rejected alternatives

### Branch/tag document URLs

Rejected for this slice because the same authority URL can resolve to different bytes later. Ley would need a provider-resolution manifest and explicit freshness semantics before such mutable references can be represented honestly.

### Arbitrary documentation URLs

Rejected because they require a general network security policy covering DNS/private-address resolution, redirects, content types, robots/rate limits, authentication, site-specific extraction, and provider-specific retention/freshness behavior.

### Binary/PDF documentation

Deferred to the separate multimodal/evidence roadmap. This slice is bounded UTF-8 text only and does not add OCR, PDF parsing, images, or attachment storage.

## Evaluation

P2 coverage now requires both an issue/PR authority scenario and a commit-pinned document authority scenario through the real CLI/MCP surfaces. The deterministic document scenario proves stable identity, target-specific egress, non-laundering, direct-evidence preservation, and remove/re-add restriction retention without relying on public internet availability.

Landing verification separately performs a real fixed-origin refresh of this repository's `README.md` at an already-pushed immutable commit and requires a typed `document` source, content-addressed snapshot, no issue-only fields, `untrusted-external-reference`, and `liveSourceChecked: false`.
