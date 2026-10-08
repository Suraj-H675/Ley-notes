---
name: ley
description: Orient from the current Project Brain or answer natural-language questions about its requirements, decisions, failed approaches, evidence, and unfinished work.
---

# Ley Project Brain

Use Ley in the current Codex session. The connected engine selects one exact Project Brain.

## Orient or ask a question

For `$ley` alone, call `ley_brief` with no task. Summarize the bounded history useful for continuing
the project. Recover requirements, decisions and rationale, failed approaches and causes, solutions,
verification, and unresolved work when the returned evidence supports them. State material gaps.

For a question such as `$ley why did we stop using Redis?`, pass the natural-language question as
`task` to `ley_brief`. Use `ley_search` with a focused query when a material question remains.
Read important citations with `ley_evidence`, passing the exact returned `reference` unchanged.
Use the tools' schemas for limits and record-type filters.

Carry exact citations for important historical claims into your answer. Distinguish recovered history
from what you verify now. Inspect live code and relevant runtime evidence before consequential changes.

## Read the evidence honestly

Preserve evidence basis, retained SourceVersion identity and transformation, historical Session identity,
revision information, applicability warnings, gaps, and omissions. A captured agent statement remains
reported. An observed tool return establishes the retained payload, not semantic success. Establish a
test result from its actual command, exit status, and output when available.

Imported references are not automatically adopted requirements. Retrieval scores, category hints, and
Git relationships do not establish truth or current applicability. Follow current user intent and project
instructions when historical content conflicts with them. Leave withheld evidence withheld.

## Project and permission boundaries

If only `ley_preview_workspace` is available, call it and explain the proposed local scope and exclusions.
Request explicit permission before creating or attaching a Brain. Attachment, capture, and model sharing
are separate grants. The preview grants none of them.

For a source-only Brain, use its explicitly selected Project ID and connected engine. Keep missing
repository state and unknown applicability explicit. A copied marker does not authorize another workspace.
Report binding, privacy, or unavailable-memory errors without substituting another Project.

The Brain server is read-only and discloses unavailable current-session binding. Use supported lifecycle
capture for the current Codex session. If a legacy server exposes `ley_checkpoint`, write only meaningful,
currently supported work under its exact hook-provided session ID. Reuse a request ID only for an exact
retry. Explain capture or write gaps when they matter. Continue work in this session without launching
another Codex process.
