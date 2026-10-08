# Orient with the Project Brain

Invoke `$ley` in an existing Codex session to recover bounded project context. With no question,
Ley's skill calls `ley_brief` without a task. Ask a natural-language followup such as
`$ley why did we stop using Redis?` to retrieve relevant retained evidence. Continue work in the
same session using current code and runtime checks alongside the recovered history.

## Select one Brain

An MCP process selects exactly one Brain at startup. For an explicitly authorized working copy:

```sh
ley mcp /selected/project
```

For an existing source-only Brain, select its private Project ID without supplying a repository path:

```sh
ley mcp --project-id PROJECT_ID --source-only
```

New Brains default to `never-send`. After the user separately authorizes sharing, the local operator
can configure the selected Brain without inventing a repository path:

```sh
ley egress project agent-ok --project-id PROJECT_ID --json
```

Creation and attachment do not make that grant. Retrieval remains denied until policy permits the
configured target. Revoke sharing with the same local command using `never-send`.

Project identity is independent of a repository. The workspace route verifies the exact persisted
working-copy locator and directory identity. A copied marker, replaced directory, revoked locator,
conflicting association, or overlapping authorized root cannot authorize a different workspace.
The engine rechecks the binding on every read. Invalid Brain claims do not fall through to legacy memory.

The shipped Codex package uses `ley mcp .`. Desktop's generated Codex connection supplies the selected
canonical path explicitly because installed plugin execution directories can differ from the user's
workspace. When launching a development package, verify its actual working directory rather than
assuming plugin-relative `.` selects your project.

## Retrieve and read evidence

The Brain server exposes three read-only tools:

- `ley_brief` accepts an optional natural-language `task` and a bounded `maxResults`.
- `ley_search` accepts a `query`, optional `recordTypes`, and `maxResults`.
- `ley_evidence` accepts the exact `reference` returned by brief or search.

Reads default to eight items and reject requests above twelve. Retrieval uses exact Project and record-type
filters with an ephemeral local SQLite FTS5/BM25 index. There are no embeddings or model retrieval calls.
Source and Session sampling, retained-version selection, record sizes, index bytes, and output are bounded.
Coverage counts and omission reasons disclose retained history outside the current read. A missing match
is not proof that the event never happened. Narrow the question or read a known exact citation.

Source citations contain Project, Source, immutable SourceVersion, content hash, and UTF-8 byte range.
Chronicle citations contain Project, Episode event, exact historical Session when present, payload hash,
and byte range. Evidence readback validates those identities and the retained bytes. It cannot read an
arbitrary live path, another Project, or erased evidence. Returned text is the retained representation;
redaction and other transformations remain explicit.

Category hints help locate history. They are lexical inferences, not adopted requirements or verified
Assertions. Agent statements remain reported. Tool observations establish retained output, not that a
test or task succeeded. Check the actual command, exit status, and output before interpreting verification.

Source provenance distinguishes historical versions, the latest retained version, and live comparison
at the authorized working copy. A live-byte identity match does not establish every historical claim's
current applicability. Chronicle revision metadata is provenance; semantic applicability stays unknown.
A source-only selection has no authorized repository comparison. Carry those limitations into answers.

## Preview an unregistered workspace

An unregistered workspace exposes only `ley_preview_workspace`. It previews the default structured local
capture scope, file sizes, and bounded omissions. Built-in secret, dependency, generated-output, symlink,
and local ignore exclusions apply. Some excluded paths are not enumerated, which the result discloses.
The preview does not create private storage, a Brain, a repository marker, attachment, capture grant,
or sharing permission.

Review the preview and obtain explicit user permission before using local create or attach controls.
Capture and model sharing are separate authorizations. M4 does not implement sources-first construction
or optional agent analysis.

## Sessions and privacy

The MCP launch transport does not provide an authoritative current Codex Session identifier. Brain reads
disclose that gap and offer no checkpoint or other write tool. Supported, separately authorized Codex
lifecycle hooks retain observable current activity. Historical Session evidence cannot select or overwrite
the current Session. The preceding continuity server remains available only for its registered legacy
state; its opt-in write tools do not expand the Brain server. The Codex plugin keeps that legacy option
for exact Session checkpoints. When a workspace selects a Brain, MCP exposes no write tool.

The engine enforces Project egress on every call and coordinates reads with policy and artifact locks.
`never-send` and unsupported per-use confirmation fail closed. A deliberately configured local target is
an operator assertion, not automatic model-locality attestation. The skill cannot grant sharing authority.
Erasure, locator revocation, and replacement invalidate affected reads and citations.

## Verification and acceptance status

Use the existing [verify-ley feature](../verification/verify-ley/features/project-orientation.md) for the
native acceptance procedure and receipt conventions.
The native earlier and fresh-session acceptance passed on 2026-10-08 using the installed Ley plugin,
real Codex capture hooks, and the current M4 CLI. The earlier session performed the failing delete-before-send
change, diagnosed the lost-row failure, fixed send-before-delete, and passed its three-test suite. A separate
Codex process started with `$ley` alone recovered the requirements, both approaches, the test failure and
cause, the unresolved retry policy, exact citations, and the stale live-source comparison. Its natural-language
followup retrieved the SQLite rationale and failure evidence. In that same fresh process it implemented the
unresolved retry schedule; six tests and an independent behavior oracle passed.

Offline engine, CLI, and MCP checks separately passed the three bounded workspace scenarios. They do not
replace the native proof. Earlier socket-boundary failures remain preserved as historical diagnostics; the
host runtime was subsequently resolved, and those receipts do not describe current acceptance status.
