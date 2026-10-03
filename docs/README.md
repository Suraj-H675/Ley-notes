# Ley documentation status

Ley completed a first-principles product/architecture reset on 2026-09-25.

Current direction:

- local, trustworthy continuity/context for coding agents;
- focused native control center rather than a general note application;
- small goal-oriented agent API;
- transactional machine-managed state;
- realistic downstream evaluation before advanced features earn their way back.

Read first:

1. `../LEY.md` — concise current execution direction (local planning file).
2. `research/first-principles-audit-2026-09-25.md` — detailed audit, evidence, decisions, and migration plan.

## Historical documents

The existing ADRs, research notes, runtime reports, and feature documents record real implementation
history and useful experiments. They are **not automatically current product requirements** after the
2026-09-25 reset.

Documents that are clearly superseded but still useful as evidence live under `archive/` rather than in
the current documentation path. The archive contains the pre-reset product-surface/acceptance work plus
the earlier Markdown/second-brain research that informed the legacy product. The former notebook/PWA
runtime-verification diary is archived there as `runtime-verification-notebook-legacy.md`; the top-level
`runtime-verification.md` now describes only the focused continuity product. Those documents explain
historical decisions but no longer describe Ley's intended product surface.

`adr/` remains historical implementation rationale during the migration because many entries document
durable data/security semantics that must be understood before legacy state can be imported or retired.
They should not be treated as a requirement to preserve the implementation shape that produced them.

Until migration work explicitly revalidates a historical decision, treat it as evidence to consult — not
an API or architecture that must be preserved. When a subsystem is migrated or retired, its superseded
documents should be moved under an explicitly historical/archive area rather than kept in the current
navigation path.

Security/privacy invariants that the reset retains (provenance, bounded evidence, project scope,
human-authority protection, egress, erasure, revision awareness) should be re-expressed against the
simplified architecture instead of preserved by copying old implementation shapes wholesale.
