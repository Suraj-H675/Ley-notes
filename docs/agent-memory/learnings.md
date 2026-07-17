# Review project learnings

Ley turns evidence from structured sessions into project-level lessons without treating an agent’s statement as fact. A lesson can describe a procedure, constraint, pitfall, convention, or fact. Every proposal cites an existing session record and starts in the review inbox.

## Propose a cited lesson

Use the session and record IDs shown by `ley session show --json`. Actor and provenance are required so a script cannot silently impersonate a user:

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

Valid actor/provenance pairs are `user` with `user-authored`, or `agent` with `agent-authored`/`inferred`. One proposal can cite up to twenty distinct session records. It cannot cite arbitrary files or invent a record ID.

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

Use `--json` with propose, correct, review, list, or show for an automation-safe response. Supply `--request-id req_<32 lowercase hex characters>` when a caller needs retry-safe delivery; reusing the same ID with changed content fails.

## Freshness

Artifact-backed evidence pins the approved ingestion snapshot. After a cited file changes or disappears and the project is ingested again, the lesson becomes `source-changed`. A previously trusted lesson then returns to `learning list --review`; it is not silently rewritten or declared false.

Evidence records without touched artifacts are `uncited`. They still preserve the session record that motivated the lesson, but Ley makes no current-source claim.

## On-disk contract

Immutable events live at:

```text
<vault>/.ley/agent-memory/projects/<project-id>/learnings/events/
```

`learnings-v1.json` and `review.md` are derived views. Do not edit them as the source of truth. Ley can rebuild them from the events. The event engine bounds text and collections, redacts recognized credentials, rejects malformed or symlinked entries, verifies contiguous history, and serializes concurrent writers.

Ley stores this data locally and sends nothing by itself. A cloud agent receives a lesson only when an integration intentionally retrieves it.
