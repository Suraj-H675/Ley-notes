# ADR 0002: Deterministic capture preview

- Status: Accepted
- Date: 2026-07-16

## Context

Agent memory must not begin by silently reading a project. A user or agent needs an exact, inspectable file manifest before Ley performs content ingestion. The preview must be reproducible across machines and must not follow repository-controlled paths outside the initialized project.

## Decision

Add `ley preview [path] [--json]`. It reads the initialized project policy and returns lexically sorted, project-relative paths and byte sizes. Preview reads filesystem metadata, not file contents.

The traversal:

- scans only validated `approvedRoots`;
- rejects symlinked approved-root components and never follows content symlinks;
- includes hidden project material such as `.github/` unless an explicit rule excludes it;
- applies project `.gitignore` files only when `respectGitignore` is true;
- disables parent, global, `.git/info/exclude`, and generic `.ignore` sources so two machines produce the same project manifest;
- applies `.ley/.leyignore` as an additional root-anchored exclusion layer that cannot re-include a Git-ignored path;
- includes regular files only, deduplicates overlapping approved roots, and normalizes output separators to `/`;
- reports symlinks, per-file limit exclusions, and deterministic total-limit exclusions separately.

Invalid ignore syntax, traversal errors, non-UTF-8 paths, and unsafe layouts fail closed. File-type/content classification is intentionally deferred: previewing a regular file does not yet claim that a later Structured capture will treat text and binary data identically.

## Evidence

- The Rust `ignore` walker documents Git-compatible matching, custom filtering, disabled-by-default symlink following, and deterministic path sorting: <https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html>.
- Rust `symlink_metadata` inspects a link itself instead of following its target: <https://doc.rust-lang.org/std/fs/fn.symlink_metadata.html>.
- Git defines hierarchical ignore files and last-match behavior: <https://git-scm.com/docs/gitignore>.
- The checked-in Graphify reference documents multiple regressions caused by ignore-layer replacement, parent leakage, and out-of-root symlinks. Ley adopts the defensive lessons but validates its own behavior independently.

## Consequences

- A preview is safe to call from a future read-only MCP tool because it does not ingest or persist project content.
- Machine-global ignore preferences cannot silently change agent memory.
- The total-byte cap currently selects eligible files in lexical order. A future semantic ingestion planner may prioritize within that cap, but must expose and version that policy rather than silently changing preview meaning.
