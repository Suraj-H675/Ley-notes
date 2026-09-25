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

Until migration work explicitly revalidates a historical decision, treat it as evidence to consult — not
an API or architecture that must be preserved. When a subsystem is migrated or retired, its superseded
documents should be moved under an explicitly historical/archive area rather than kept in the current
navigation path.

Security/privacy invariants that the reset retains (provenance, bounded evidence, project scope,
human-authority protection, egress, erasure, revision awareness) should be re-expressed against the
simplified architecture instead of preserved by copying old implementation shapes wholesale.
