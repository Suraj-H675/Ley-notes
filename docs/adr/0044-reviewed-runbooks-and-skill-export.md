# ADR 0044: Reviewed runbooks and explicit host Skill export

Status: Accepted

## Context

Ley already stores reusable operational knowledge as typed `procedure`, `pitfall`, and `convention` learnings. Those learnings can be proposed by an agent, but automatic derivation remains review-required until a user explicitly confirms the learning. Artifact citations can also later make a previously trusted learning stale or source-changed.

The P1 roadmap calls for a reviewed procedure/runbook workflow and optional host Skill export. A runbook should make a deliberately selected group of stable operational learnings portable and readable without inventing a second authority layer. Exporting that material into an Agent Skill is more sensitive: host Skill text can influence future agent behavior, so Ley must not silently turn remembered text into executable scripts or privileged policy.

## Decision

Ley provides an on-demand, non-persistent reviewed runbook projection over an explicit list of learning IDs.

- `ley runbook compile` accepts a title and one or more explicit learning IDs.
- Every selected learning must belong to the current project and be `verified`, `trusted`, and `current`.
- Only `procedure`, `pitfall`, and `convention` learnings are eligible. Facts and constraints are not silently promoted into operating instructions.
- Selection is explicit rather than similarity- or recency-driven.
- The projection preserves each learning's provenance, confidence, freshness, event count, and a deterministic source hash.
- `authorityIncreasedThroughProjection` is always false. The runbook is reviewed project knowledge, not a new authority source.
- `liveSourceChecked` remains false. Current trusted citations are stronger than uncited or stale memory, but compiling a runbook does not inspect live files.
- Runbook Markdown contains only the selected reviewed learning title/guidance plus Ley source-binding metadata. Evidence notes, prompt/response bodies, absolute project/vault paths, and unrelated session state are not copied.
- Runbooks are bounded to 20 source learnings and 32,000 rendered characters in this slice.
- `runbookId` and `sourceFingerprint` are deterministic over logical source state and exclude generation time. A change to a selected learning or its source state changes the current runbook identity.

Optional host Skill export is a separate explicit local action:

- `ley runbook export-skill` requires the same title and learning IDs, an exact previously reviewed `--expected-runbook` ID, an explicit `--host`, and an explicit `--egress-target`.
- Ley recompiles the runbook at export time and fails if the current ID no longer matches the reviewed ID.
- Historical-memory egress policy is checked before Skill content is returned. Project or finer-grained restrictions cannot be bypassed through export.
- The result is content only: `persisted: false`, `installed: false`, and `explicitUserActionRequired: true`.
- Ley does not write `SKILL.md` into Codex, Claude Code, the project, or the vault in this slice.
- There is deliberately no MCP tool for runbook-to-Skill export. An agent cannot acquire a self-service route that promotes remembered text into its own host instructions.
- Exported Skill text repeats that it grants no tool, filesystem, network, review, or egress permission and does not replace the current user request, repository policy, or live-source inspection.

## Consequences

The first slice is intentionally narrow. Users choose the exact operational learnings rather than asking Ley to infer a runbook cluster, and host installation remains outside Ley. This adds review ceremony, but it preserves the authority boundary and makes stale-source or changed-learning races fail closed through the exact runbook ID.

Future UI may make selection/review/export easier, and future formats may support other portable instruction artifacts. Any future install action must remain separately explicit, reviewable, host-scoped, egress-aware, and unable to grant itself permissions through stored text.
