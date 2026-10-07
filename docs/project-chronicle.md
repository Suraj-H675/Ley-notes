# Project Chronicle and Codex capture

M3 adds Ley's first canonical **Project Chronicle** path for Project Brains. It records supported observable Codex
activity into the Project Brain's owner-private SQLite state. It is not a transcript mirror and it does not infer
hidden reasoning, completion, causality, or semantic project knowledge.

## Authorization boundary

Repository attachment/import, Codex installation, capture, and model sharing are separate permissions.

For an existing Project Brain, open **Project settings → Codex activity capture**. Ley lists only that Brain's
currently authorized working copies. Enabling capture opens a native confirmation showing the exact Project, working
copy, locator, host, and retention mode. The hook/CLI/MCP surfaces have no capture-grant operation.

A retained grant is bound to the Project, working-copy locator and directory identity. Revocation or a changed
retention decision fences in-flight writers through the Project generation. Moving/replacing the working copy makes
an old grant ineffective until it is explicitly reviewed again. Connecting Codex by itself does not enable capture.

## Canonical records

A `Session` identifies one observed Codex host session. Each retained `Episode` has an explicit per-session sequence,
producer/origin, evidence basis, optional host turn/tool reference, time/provenance, and explicit gaps. Independent
sessions have independent sequence spaces; Ley does not manufacture a global causal order between parallel agents.

The current Codex adapter supports:

- `SessionStart` → session-start observation;
- `UserPromptSubmit` → bounded visible user prompt;
- Bash `PostToolUse` → bounded command/result observation;
- `Stop` → bounded visible assistant response when present;
- `SessionEnd` → session end.

The bundled hook path never reads `transcript_path`. Unsupported/missing observations are represented as gaps rather
than invented history. A Bash `PostToolUse` result is an observed returned result, not proof that the command, test,
or requested task succeeded.

Historical structured continuity events are normalized as compatibility Chronicle metadata without strengthening
their claims. Checkpoint/structured submissions remain **reported**, and the migration records that complete
observable chronology is unavailable. New canonical Codex hook episodes are **observed** only because Ley received
that supported hook payload; observation still does not prove semantic truth.

## Retention and bounds

- **Minimal** records supported boundaries/metadata/gaps without message, command, or result bodies.
- **Structured** retains bounded, pattern-redacted visible bodies.
- **Full evidence** currently has the same supported Codex hook coverage and bounded/redacted body behavior; it does
  not unlock transcript scraping or extra tools.
- Individual prompt/response/tool bodies use the existing bounded redaction limits.
- A canonical Codex session also has aggregate retained-body and episode-count ceilings. Once a body ceiling is
  reached, later supported metadata may continue with explicit omission gaps; once the episode ceiling is reached,
  later hook events are not retained.

Exact retries use stable host identities where Codex provides them and replay the existing Episode rather than
creating duplicates. Reuse of the same stable identity with different observable content fails closed.

## Deletion and inspection

Session erasure removes the Session's retained events and Chronicle metadata through existing foreign-key/dependency
erasure and leaves a scrubbed session tombstone that cannot be resurrected by a later hook. Whole-Brain erasure also
removes Chronicle state. Capture revocation stops future capture but does not erase already retained history.

Read-only inspection is available through `ley brain sessions`, `ley brain history`, and `ley brain capture-state`.
These are transition/debug surfaces for the M3 engine, not the final M4 `$ley` retrieval experience.

## Transition boundary

M3 makes Codex the first real Project Brain host adapter. The older focused-continuity Codex/Claude paths remain
compatibility behavior for projects that are not routed through a Project Brain capture grant. Claude Project Brain
Chronicle capture is deliberately deferred to the cross-agent milestone. M3 does not implement `$ley` orientation,
derived Entities/Assertions, sources-first construction, or the redesigned Desktop information architecture.
