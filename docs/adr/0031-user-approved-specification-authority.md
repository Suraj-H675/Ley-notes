# ADR 0031: User-approved Specification authority

- Status: accepted
- Date: 2026-09-16

## Context

Ley needs a first-class source of human intent that is not confused with captured project evidence, historical session memory, or agent-authored learnings. Ordinary Markdown is already the authoritative human knowledge format, but a YAML field written by an agent cannot be allowed to grant itself policy authority. A Specification also needs revision semantics: after the user approves requirements, later edits must not silently inherit the previous approval.

## Decision

A Specification remains an ordinary Markdown note in the bound Ley vault. The desktop user deliberately designates the visible note with portable `ley-type: specification` and stable `ley-spec-id: spec_<uuid>` frontmatter, then approves that exact revision through a local user-controlled action.

Approval authority is stored separately in the private OS-local `specifications-v1.json` registry. Each project-scoped entry contains only the Specification ID, vault-relative Markdown path, SHA-256 of the complete approved source, and approval time. The registry uses the same strict schema/size validation, owner-only Unix permissions, advisory cross-process lock, symlink rejection, and atomic replacement pattern as Ley's binding/catalog registries. Specification text is never copied into the private registry.

Approval reads the bound vault through a no-follow capability path and reads the file twice before hashing so a changing or replaced path fails closed. Visible relative Markdown paths only are accepted. One note cannot simultaneously carry two approved Specification IDs for the same project.

Every read re-hashes the current note. An exact hash match is `current`; changed bytes are `changed`; a missing note is `missing`. Only `current` revisions may be returned to an agent as `authority: human-intent`. Changed or missing revisions contribute diagnostics but no requirement text until the user reviews and approves again.

The desktop verifies that the currently open notes vault canonically matches the selected project's private binding **before** it writes portable Specification metadata. Revocation removes only authority; it does not delete or rewrite the Markdown note. Agent Memory erasure also leaves Specifications and their authority registry untouched because human intent is not episodic agent memory.

The fixed-project MCP server exposes one read-only `ley_project_specifications` tool. It has no project/vault selector and no approve/revoke counterpart. The tool returns only whole current approved notes under caller-reducible result/character limits. If a complete Specification will not fit the requested budget, Ley omits it and reports `character-budget` instead of truncating human requirements mid-document. Changed/missing approvals are also explicit exclusions.

## Consequences

- Markdown/YAML stays readable, editable, portable, searchable, linkable, and user-owned.
- Agent-written YAML alone cannot acquire Specification authority.
- Editing an approved note safely revokes its current applicability without destroying the previous approval record.
- A user may intentionally elevate a note derived from some earlier source, but the elevation is a new explicit human authorization rather than authority inherited through summarization.
- MCP can consume approved intent but cannot create, renew, or revoke that authority.
- Specification content is re-read from the user's vault, so `specificationSourceRevisionChecked: true` refers only to the approved note revision; `projectLiveSourceChecked` remains false.
- This ADR established approval authority only. Later ADRs extend consumption without changing that authority boundary: ADR 0032 integrates exact current approved Specifications into `ley_compile_context`; ADR 0038 adds source-level agent-context egress; ADR 0058 adds explicit Bootstrap Specification authority for uninitialized workspaces; and ADR 0068 adds a read-only acceptance-criteria projection derived from ordinary approved Markdown rather than a proprietary authority store.

## Rejected alternatives

### Trust frontmatter alone

Rejected because an agent or imported note could write the same YAML and silently manufacture human authority.

### Copy approved Specifications into Agent Memory

Rejected because it would conflate user intent with historical memory, complicate erasure semantics, and make private derived storage the authoritative representation.

### Automatically carry approval across edits

Rejected because a small textual edit can materially change a requirement or acceptance criterion. Exact-revision approval is intentionally conservative.

### Truncate large Specifications to fit agent context

Rejected for the initial slice because partial requirements can invert intent. Whole-document omission with explicit diagnostics is safer until task-conditioned Specification retrieval is implemented.
