# Ley documentation status

Ley began a **Project Brain** product/architecture reset on 2026-10-06. The previous focused-continuity
product remains important implementation and migration evidence, but it is no longer the current product
contract.

Current direction:

- one local-first Project Brain per software project;
- projects may begin from sources/specifications before code exists;
- durable supported project history stays distinct from rebuildable interpretation;
- evidence, human adoption, and current applicability stay inspectable and separate;
- coding agents retrieve bounded relevant context rather than receiving history dumps;
- Codex, Claude, and future supported agents contribute to the same Brain under explicit project/privacy boundaries;
- the eventual Desktop is sparse: Home / Projects / Settings, then Overview / Knowledge / History / Review;
- optional model analysis is never required for deterministic local import/core use.

Read first:

1. `../LEY.md` — current Project Brain product/architecture contract.
2. `research/project-brain-m0-reconciliation-2026-10-06.md` — current-repo audit, keep/adapt/replace/remove/defer map,
   migration obligations, and the locked M1 input model.
3. `architecture.md` — current implementation foundation and transition boundary.
4. `privacy-and-storage.md` — privacy/storage requirements and current transition behavior.

Current/transition operational references:

- `agent-memory/host-integrations.md` — normal Desktop-driven Codex/Claude setup and the host trust boundaries;
- `native-release.md` — native installers, signing/notarization, updater keys, checksums, and release procedure.

The `agent-memory/` guides describe the **currently implemented continuity runtime** while the Project Brain
remake is in progress. They are useful for operating and migrating today's code; their names, IA, and data
ontology are not automatically the final Project Brain contract.

## Historical documents

The existing ADRs, research notes, runtime reports, and feature documents record real implementation
history and useful experiments. They are **not automatically current product requirements** after either
the 2026-09-25 reset or the 2026-10-06 Project Brain reset.

Documents that are clearly superseded but still useful as evidence live under `archive/` rather than in
the current documentation path where that history was tracked documentation. This includes pre-reset
product-surface/acceptance work, earlier Markdown/second-brain research, and the old notebook/PWA
runtime-verification diary. The former local-only `LEY.md` execution diary has been replaced by the concise
tracked Project Brain contract; the M0 reconciliation preserves the material migration/product conclusions.
Historical documents explain decisions but do not describe Ley's intended product surface.

`adr/` remains historical implementation rationale while finite compatibility state is being retired because many
entries document durable data/security semantics that must be understood before old local state can be imported,
cleaned up, or safely removed. An ADR whose historical status says `Accepted` was accepted for that decision at
that time; it is not permanent authority over the current Project Brain contract. ADRs should not be treated as a
requirement to preserve the implementation shape that produced them.

Unless current product/runtime evidence explicitly revalidates a historical decision, treat it as evidence to
consult — not an API or architecture that must be preserved. When a subsystem is migrated or retired, its superseded
documents should be moved under an explicitly historical/archive area rather than kept in the current
navigation path.

Security/privacy invariants that survive (provenance, bounded evidence, project scope, human-authority
protection, egress, erasure, revision awareness, interruption/idempotency) should be re-expressed against
the Project Brain architecture instead of preserved by copying old implementation shapes wholesale.
