# ADR 0066: Deterministic host tool evidence

- Status: Accepted
- Date: 2026-09-20
- Extends: ADR 0029, ADR 0057, ADR 0065

## Context

Ley's Reliable Memory Compiler can now recover several structured memory shapes from bounded
post-checkpoint prompt/response evidence without silently promoting them to trusted truth. That closes
many omission and atomicity gaps, but one important Tier-1 evidence class from `LEY.md` is still
missing: safely observed host tool outcome metadata.

Today the automatic lifecycle path records only `UserPromptSubmit` and final assistant `Stop` text.
Explicit checkpoints can contain Commands and Verification, but those fields are authored by the
agent. A recovery candidate based only on assistant prose therefore cannot prove that a command was
actually invoked, that a tool returned, or that a verification command succeeded.

Both supported hosts now expose structured tool lifecycle hooks. The first safe slice should use that
stable boundary rather than parse host transcripts or infer execution from model text.

The new evidence must not accidentally broaden the already-released candidate-bound recovery
contracts. Schema-v3/v8/v9/v10/v11/v12/v13 recovery binds to `tev_` prompt/response evidence and
requires complete accounting of that recovery window. Making newly captured tool observations
mandatory `tev_` evidence would either make those writers unable to close existing windows or force a
new Command/Verification writer before its execution semantics are designed.

## Decision

Add first-class immutable **host tool observations** as a separate Tier-1 evidence stream.

The first slice is intentionally narrow:

- supported tool: shell/Bash only;
- Codex source: `PostToolUse` with `tool_name == "Bash"`;
- Claude Code sources: `PostToolUse` and `PostToolUseFailure` with `tool_name == "Bash"`;
- no transcript parsing;
- no automatic Command or Verification checkpoint write;
- no automatic/model-generated candidate inside Ley;
- no inference that a returned tool result means a test passed or a command succeeded.

An observation records only mechanically known host facts:

```text
host
opaque turn reference when available
opaque tool-call reference
tool name
observation kind: returned | explicit-failure
bounded/redacted command text when retention policy permits
bounded/redacted host result/error text when retention policy permits
capture mode and truncation/retention metadata
```

Raw host tool-call IDs are accepted only as transient correlation material and are converted to an
opaque Ley-owned identifier before persistence. They are never returned directly.

`returned` means only that the host emitted its normal post-tool event. In particular, Codex Bash
`PostToolUse` may also represent a non-zero command. Ley does not derive an exit code, verification
status, or successful outcome unless a later contract has authoritative structured evidence for that
claim. Claude Code's separate `PostToolUseFailure` may be retained as `explicit-failure`, which means
the host reported a tool invocation failure; it still does not create a durable Verification record.

## Durable schema and privacy

Tool observations are additive session events using schema version 14 and `session-v14.json`. Older
events/projections remain readable without rewrite.

Structured and Full Evidence capture retain bounded command/result text after the same credential
redaction used for other automatic evidence. Minimal capture appends a metadata-only disclosure
record. Automatic prompt/response and tool bodies share one aggregate retained-evidence byte budget so
tool-heavy sessions cannot silently double the existing automatic-capture resource surface.

The host adapter also applies a hard pre-normalization input bound before copying command/error text
or flattening structured tool responses. Structured flattening has cumulative character, visited-node,
immediate fan-out, and nesting-depth limits; object fan-out is checked before key collection/sorting.
A hostile or unexpectedly huge host payload therefore cannot force unbounded traversal or temporary
allocation before the ordinary retained-text redaction/truncation boundary runs.

The event is append-only, idempotent under a stable hook request identity, and valid only while the
session is active. Reused request identity with changed retained content fails rather than replacing
history.

Tool output remains untrusted host evidence. Stored command/result text grants no filesystem,
network, tool, write, review, trust, or egress authority.

## Memory Compiler boundary

`ley_session_memory_compile` exposes post-checkpoint tool observations in a **separate supporting tool
evidence collection**. They do not join the existing candidate-bound `evidence` collection, do not
change `totalUnconsolidatedEvidence`, and do not change existing prompt/response recovery-state or
coverage semantics in this slice.

The pack explicitly reports that these tool observations are supporting provenance only and are not
eligible `recordId` anchors for the current candidate-bound writers. This keeps the old fingerprint,
coverage, and replay contracts immutable while giving a recovering agent mechanically stronger
context than assistant prose alone.

After a checkpoint closes the current recovery window, the same immutable tool observations remain
available through explicit bounded session-history inspection. Existing prompt/response `turns` and
their counters retain their current meaning; tool observations are returned through a separate
collection and separate counters.

## Host integration boundary

The packaged Codex and Claude Code hooks opt into only the supported Bash post-tool events. The core
adapter still validates the fixed Ley project/session, active-session state, egress boundary, host
identifier, tool name, payload bounds, and capture policy before appending anything.

Unsupported tools and unsupported hook events are no-ops rather than generic JSON retention. This
prevents the first slice from becoming an ambient raw tool log.

## Consequences

- Ley gains deterministic evidence that a supported shell tool interaction actually occurred.
- Future Command/Verification recovery can be designed against observed execution provenance rather
  than assistant claims.
- Existing candidate-bound recovery contracts and fingerprints remain unchanged.
- Recovery packs become more useful after crashes without claiming tool-result semantic correctness.
- Tool-heavy sessions add bounded storage and hook traffic; strict retention and shared capacity are
  therefore part of the feature, not deferred hardening.

## Deliberately deferred

- candidate-bound Command recovery;
- candidate-bound Verification recovery;
- automatic conversion from Bash output/exit state into verification success or failure;
- general retention of arbitrary non-shell tool request/response bodies;
- attaching observed tool evidence directly to existing v3/v8-v13 candidate fingerprints;
- automatic/model-generated Memory Compiler candidates inside Ley;
- background/scheduled consolidation; and
- automatic authority/trust promotion.

## Verification

The slice must prove:

- Codex Bash `PostToolUse` and Claude Code Bash `PostToolUse` / `PostToolUseFailure` append bounded
  idempotent observations to the current active Ley session;
- unsupported tools/events remain no-ops;
- raw host tool-call IDs are never persisted or returned;
- Structured/Full command and result bodies are credential-redacted and bounded;
- oversized, over-wide, or over-deep raw command/error/structured-response payloads fail closed before unbounded normalization;
- Minimal capture retains metadata only;
- prompt/response and tool automatic evidence share the aggregate session capacity and disclose
  capacity omission instead of exceeding it;
- tool observations survive replay and schema-v1..v13 history remains readable;
- explicit session history returns tool observations separately without changing existing `turns`
  semantics;
- Memory Compiler exposes only post-checkpoint tool observations as supporting provenance while old
  candidate-bound evidence counts/state/fingerprints remain unchanged;
- Codex normal `PostToolUse` is never labeled verification success merely because it returned;
- Claude explicit tool failure remains an observation, not a durable Verification record;
- host packages contain the exact new hook registrations; and
- a real host/eval path proves redaction, no raw tool-call-ID leakage, no authority promotion, and zero
  privacy leakage.
