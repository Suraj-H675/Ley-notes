# Review project learnings

Ley turns retained session evidence into project-level lessons without treating an agent’s statement as fact. A lesson can describe a procedure, constraint, pitfall, convention, or fact. Every proposal cites existing Ley session evidence and starts in the review inbox.

## Propose a cited lesson

Use a session ID plus an eligible structured record ID from `ley session show --json`, or an exact captured `tev_` user-prompt/assistant-response record surfaced by an explicit evidence workflow such as the local Consolidation Inbox. Body-free turn observations are not valid learning evidence. Schema-v14 `toe_` host-tool observations remain supporting provenance and are not accepted as direct learning evidence. ADR 0085 adds one indirect path: when an isolated `toe_` has first been reviewed and committed through the dedicated schema-v16 observed-Command recovery route, the resulting durable Command is an ordinary structured session record that may later support a Learning proposal. Actor and provenance are required so a script cannot silently impersonate a user:

```bash
ley learning propose /path/to/project \
  --actor agent \
  --provenance inferred \
  --kind procedure \
  --title "Check the complete workspace" \
  --guidance "Run cargo check --workspace before delivery." \
  --confidence 85 \
  --evidence ses_01234567890123456789012345678901:ckp_01234567890123456789012345678901
```

Valid actor/provenance pairs are `user` with `user-authored`, or `agent` with `agent-authored`/`inferred`. One proposal can cite up to twenty distinct `(sessionId, recordId)` evidence references. It cannot cite arbitrary files or invent a record ID.

For every new proposal, Ley preserves a bounded origin lineage in the immutable learning event. For structured records, the mechanically known origins include the cited session record and any captured artifact snapshots attached to that record. A direct captured turn citation records `turn-evidence` lineage to that exact session and `tev_` record. If the cited structured record is candidate-bound recovery, lineage also reaches the exact recovery candidate fingerprint and the record-specific persisted evidence binding that produced it. For an atomic schema-v11 recovery checkpoint, citing the checkpoint itself resolves the complete batch evidence union, while citing one recovered Decision/Problem/Task/Plan child or one read-projected `unr_...` unresolved child resolves only that child's persisted evidence binding. For schema-v12 rich-Problem recovery, citing the checkpoint resolves the complete debugging-episode evidence union, while citing the recovered Problem, one Attempt, or its Resolution resolves only that durable child's component-specific evidence subset. Schema-v13 composite recovery combines those rules without widening any child: the checkpoint receives the complete composite evidence union, while the rich Problem parent, each Attempt/Resolution, and every minimal sibling resolve only their own persisted evidence binding and retain the shared composite candidate fingerprint. Schema-v16 isolated observed-Command recovery follows the same origin-preservation rule across a different evidence namespace: citing the recovered Command retains the recovery candidate fingerprint plus the exact source `toe_` as `tool-evidence`; Ley must not relabel that source as `turn-evidence`. These recorded origins do not prove that Ley knows every causal influence on an agent-authored claim or that a retained source is semantically sufficient merely because it was mechanically preserved.

## Inspect and review

```bash
ley learning list /path/to/project --review
ley learning show lrn_01234567890123456789012345678901 /path/to/project
ley learning review lrn_01234567890123456789012345678901 \
  /path/to/project \
  --actor user \
  --action confirm \
  --note "Verified against the release workflow."
```

Only a user action can `confirm`, `reject`, or `supersede`. An agent may `contest` or `mark-stale` so it can surface a concern without granting or permanently removing trust. Supersession also requires `--replacement <learning-id>`.

Confirmation changes a proposal to `verified` and `trusted`. Rejected and superseded lessons leave the actionable inbox but remain in immutable history. A correction returns the lesson to review:

```bash
ley learning correct lrn_01234567890123456789012345678901 \
  /path/to/project \
  --actor agent \
  --title "Check and test the complete workspace" \
  --guidance "Run cargo check --workspace and cargo test --workspace." \
  --confidence 92 \
  --evidence ses_01234567890123456789012345678901:ver_01234567890123456789012345678901 \
  --note "A later verified run expanded the procedure."
```

Ley Desktop exposes the same behavior under **Agent Memory → Lessons → Provenance inspector**. The
inspector shows bounded origin lineage separately from the broad provenance label: whether the retained
lineage was mechanically resolved, whether causal completeness is proven, the automatic authority
ceiling, and stable source handles for retained session records, turns, tool evidence, captured artifacts, or recovery
candidates. Those handles contain no source bodies and do not increase authority. **Correct** edits the
current title, guidance, and confidence, requires a reason, preserves the complete cited evidence set,
and appends a new immutable version. It does not rewrite the old claim. The corrected version returns
to review and must be confirmed separately. **Supersede** is also available to the user when another
non-terminal learning is present in the bounded project learning list. It requires an explicit replacement and reason,
passes the same visible-event-count stale guard as other reviews, and also submits the replacement learning's observed
event count. Under the locked learning mutation, core rejects a replacement that changed meanwhile, became terminal,
does not exist, points to itself, or would violate supersession consistency. The old learning remains immutable
terminal history with its stable `supersededBy` link; the
replacement does not become trusted merely because another learning points to it.
The Desktop picker uses the existing bounded learning list rather than issuing another search: each option shows
title, state, trust, freshness, and a stable short ID, and the UI discloses when additional learnings were omitted so
the CLI remains the fallback for a replacement outside that bounded list.

Corrections also preserve origin history: newly resolved origins are unioned with the prior lineage rather than replacing it. `automaticAuthorityCeiling: review-required` means the derivation/proposal path cannot self-promote its output. Explicit user confirmation can establish trusted learning state, but it does not rewrite the origin chain or turn `causalCompletenessProven: false` into a stronger claim.

Every desktop correction and review decision is tied to the ledger event count visible when the inspector opened. If another agent or window changes the learning first, Ley refuses the stale action and asks the user to reload rather than applying a decision to unseen text. If the bounded inspector had to truncate the claim, review controls remain unavailable until the complete projection is inspected through the CLI. Rejected and superseded learnings remain inspectable terminal history without non-working action buttons; superseded history opens the replacement by title/state when it is present in the bounded list and otherwise shows the stable replacement learning ID.

## Historical procedure/application instrumentation

Older Ley session schemas may contain Context Utility or Procedure-application observation events created by earlier explicitly instrumented workflows. They remain historical provenance for compatibility/migration, not a current agent capability. The model-facing `ley_context_utility_bind` / `ley_context_utility_observe` routes are retired, and current learning trust/ranking must not change because historical utility/application rows exist.

New workflows should record concrete observed outcomes through ordinary checkpoints/Verification evidence and keep causal claims separate. Reintroduce utility/application instrumentation only if a future controlled downstream study shows material value beyond the smaller current continuity surface.

## Freshness

Artifact-backed evidence pins the approved ingestion snapshot. After a cited file changes or disappears and the project is ingested again, the lesson becomes `source-changed`. A previously trusted lesson then returns to `learning list --review`; it is not silently rewritten or declared false.

Evidence records without touched artifacts are `uncited`. They still preserve the session record that motivated the lesson, but Ley makes no current-source claim.

## Durable storage contract

For projects that have crossed learning-authority cutover, authoritative learning events live in Ley's OS-private continuity SQLite database. Each continuity row embeds the validated schema-v3 `LearningEvent`; learning request identity remains inside that event, and provenance/deletion relationships are stored as explicit `depends-on-session` and `supersedes` links. Native learning mutations are serialized transactionally and do not create or update legacy learning projection files.

Older projects may still have the compatibility ledger at:

```text
<vault>/.ley/agent-memory/projects/<project-id>/learnings/events/
```

Before the first native learning write, transition reads can reconcile that current legacy ledger into continuity. The first transition write requires native session authority, fences the legacy learning writer, imports one final learning-only snapshot, records the `learning-authority-cutover` marker, and then appends only native continuity events. After that marker, the legacy learning tree is not re-synchronized as authority. `learnings-v1.json` and `review.md` are legacy derived views, not authoritative state.

Current learning events still use schema v3 and persist bounded `originLineage`, including distinct `tool-evidence` origins for schema-v16 recovered Commands. Previous schema-v2 lineage-bearing events remain readable through migration, and legacy v1 events remain readable with reconstructed lineage explicitly marked as not mechanically complete. Lineage is included in the immutable request fingerprint, so changing it without the matching event fingerprint fails validation. The event engine bounds text and collections, redacts recognized credentials, validates contiguous per-learning history, and serializes concurrent native writers.

The private captured-memory vault remains relevant for cited artifact snapshots and freshness checks. That dependency does not make the legacy learning event files authoritative again.

The durable ledger retains at most 256 origin identities per learning derivation chain. Explicit learning inspection returns at most 32 of those sources and discloses any additional omissions by marking the returned lineage unresolved. Broad lists/search/context carry only compact origin counts/flags. Full source text is not copied into lineage merely because the source was cited.

Ley stores this data locally and sends nothing by itself. A cloud agent receives a lesson only when an integration intentionally retrieves it.
