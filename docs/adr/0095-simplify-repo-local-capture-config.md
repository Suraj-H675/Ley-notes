# ADR 0095: Simplify repo-local capture config to mode only

**Status:** Accepted — 2026-10-02

## Context

The reset keeps a tiny portable `.ley/` marker but explicitly calls for simplifying its policy/config fields.
The original schema-1 `.ley/capture.json` stored seven fields:

- capture mode;
- approved roots;
- Git-ignore behavior;
- per-file and total byte ceilings; and
- a `storeRawTranscripts` boolean.

Only capture mode is a current product mutation. New projects always use the project root (`.`), respect
`.gitignore`, and use the same bounded byte ceilings. The old raw-transcript bit is a dormant grant for a future
adapter; current lifecycle adapters never read raw transcript paths. The Desktop displays the effective bounds but
does not edit them. `.leyignore` is the current project-owned exclusion surface.

Keeping the unused knobs writable in every new repository makes the trust boundary look more configurable than
the product actually supports. It also leaves a dormant future-adapter permission beside a mode whose current
implemented purpose is different.

Existing projects are different: historical schema-1 files were hand-editable, and some may contain narrower
roots or tighter byte limits. Silently replacing such a policy with the new defaults could broaden capture and
would violate Ley's privacy boundary.

## Decision

1. New `.ley/capture.json` files use schema version 2 and contain only:

   ```json
   {
     "schemaVersion": 2,
     "mode": "structured"
   }
   ```

2. Runtime capture still uses the existing full `CapturePolicy` value. For schema-2 config Ley derives it from
   fixed reviewed defaults: project root `.` only, Git ignore enabled, 1 MiB per file, 512 MiB total, and
   `storeRawTranscripts = false` for every mode. Full Evidence remains the explicit higher-sensitivity boundary for
   the retention behavior Ley actually implements today (for example supported original images); it is not a
   standing grant to a hypothetical future transcript adapter.
3. `.ley/.leyignore` remains the editable repository-local exclusion layer. The deterministic preview continues
   to show the effective root, byte ceilings, Git-ignore behavior, skips, and candidate counts before capture.
4. Schema-1 full capture policies remain readable and validated for migration compatibility.
5. Reading an old project never rewrites its repository. On an explicit capture-mode change:
   - a schema-1 policy that exactly matches the old defaults is rewritten as schema 2 with the requested mode;
   - a genuinely customized schema-1 policy keeps its roots/Git-ignore/byte-limit settings and remains schema 1,
     with only mode and its historical raw-transcript bit changed under the old compatibility semantics.
6. `project.json` remains the stable portable project-identity file. Capture mode is not merged into identity;
   that would couple privacy-setting migration to the durable project-ID/name/creation-time contract for no
   material product benefit.
7. Historical artifact manifests and native artifact snapshot metadata continue to serialize the complete
   normalized `CapturePolicy`. This preserves exact capture provenance and compatibility even though the current
   repo-local config is smaller.

The capture fingerprint remains a hash of the normalized full effective policy plus `.leyignore`. Minimal and
Structured schema-1 defaults therefore retain the same effective policy/fingerprint when represented by schema 2.
Legacy Full Evidence included the dormant raw-transcript grant, while schema-2 Full Evidence deliberately does
not; collapsing into schema 2 is therefore an explicit policy/fingerprint change rather than a transparent format
rewrite.

## Why not migrate every old file on read?

Diagnosis, preview, search, and other reads should not create unexplained repository changes. More importantly,
custom schema-1 policies cannot always be collapsed safely: replacing a narrow `approvedRoots` value or tighter
limit with current defaults could retain more data. Compatibility therefore stays explicit and conservative.

## Consequences

- Fresh repositories have one identity file, one two-field capture setting, and `.leyignore` rather than a
  pseudo-advanced policy surface.
- Current configuration cannot disable Git-ignore protection or raise capture ceilings by adding undocumented
  fields; schema 2 rejects unknown fields.
- Old custom capture boundaries remain honored until Ley gains a separately reviewed explicit reset/migration
  action, if one is ever needed.
- Full Evidence remains an explicit consent boundary for its current higher-sensitivity retention effects, but the
  repo no longer stores a dormant transcript grant. A future transcript-capable adapter must define and obtain its
  own explicit consent rather than infer authorization from capture mode.

## Non-goals

This ADR does not change capture-mode semantics, erase historical evidence, remove `.leyignore`, rewrite artifact
manifest schemas, or automatically normalize custom legacy policies. It also does not decide whether Full Evidence
itself remains a long-term focused-product feature.
