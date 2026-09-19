# ADR 0056: Review first capture before desktop initialization

Status: Accepted

## Context

Ley's first-time desktop project flow previously combined three actions behind one button:

1. create repository-local `.ley` metadata;
2. bind the project to the open filesystem vault;
3. ingest the first redacted Agent Memory snapshot.

The user could inspect the detailed capture boundary only after that first capture already existed. That contradicted Ley's higher-level setup contract: an uninitialized project should show what would be captured, what policy excludes, and where durable memory will live **before** the user approves initialization.

The existing deterministic capture preview could not be reused directly because it required an already initialized `.ley/capture.json` and `.ley/.leyignore`.

## Decision

Ley adds a read-only initial-capture preview for an uninitialized project directory.

`preview_initial_capture(PROJECT, Structured)` evaluates the same default Structured capture policy and the same default `.leyignore` rules that `initialize_project` would create, but it does not create `.ley`, a vault binding, an Agent Memory store, or any other persistent state.

The desktop's existing `inspect_agent_project` onboarding response now includes a bounded summary of that preview when the selected folder is uninitialized. The UI shows:

- Structured capture as the proposed mode;
- approved roots and file/total byte limits;
- eligible file/byte totals;
- a bounded sample of eligible paths;
- bounded counts for oversized, total-limit, and symlink skips;
- the open vault name where durable Agent Memory will live;
- the default ignore categories and `.gitignore` behavior;
- an explicit statement that inspection has not initialized or written Agent Memory.

The final action is labelled as an approval rather than a passive setup step.

## Approval binding

Showing a preview is insufficient if the project can change before capture and Ley silently ingests a broader set of files.

Capture previews therefore expose a deterministic `planFingerprint` over:

- the capture-policy/ignore fingerprint;
- capture mode;
- eligible project-relative paths and observed sizes;
- included-byte total;
- oversized/total-limit skips;
- skipped symlink paths.

The reusable plan fingerprint intentionally excludes machine-specific absolute project/vault paths and generation time.

The desktop derives a separate ephemeral `approvalFingerprint` from the canonical selected project root plus that exact `planFingerprint`. This keeps the reusable plan identity path-free while preventing an approval from one same-shaped folder being replayed for another. Desktop initialization must submit the exact approval fingerprint the user reviewed. The native command re-previews the still-uninitialized directory and recomputes the project-bound approval before creating metadata. A mismatch fails closed with no initialization or private-memory write.

The first ingestion is also given the approved plan fingerprint. It recomputes the capture plan before opening the private artifact store and fails with `CapturePreviewChanged` if the plan no longer matches. The resulting preview list is then the fixed candidate set for that ingestion, while the existing scoped-read checks still reject files that change while they are being read.

If initialization created `.ley` but the later ingestion check detects drift, the project can remain initialized but unbound. That state is not an approval bypass: desktop inspection must show a fresh current preview and `connect_agent_project` requires its project-bound approval fingerprint before first binding/capture. The unbound path performs the same plan recheck before opening the private artifact store.

This creates two race barriers:

1. project changed between UI review and initialization → reject before `.ley` is created;
2. project changed after initialization began but before ingestion fixed its candidate set → reject before private artifact memory is opened.

The user must review the refreshed boundary before trying again.

## Privacy and authority

The initial preview is metadata-oriented boundary inspection, not source ingestion. It does not retain source text, build a graph, create a session, contact a model/provider, change capture mode, or authorize agent egress.

Default secret-oriented path exclusions and post-capture secret redaction remain defense-in-depth rather than a claim that every sensitive file/value can be detected automatically.

The preview does not grant authority to any captured text. All later evidence/trust/egress rules remain unchanged.

## Deliberately deferred

This slice does not add:

- arbitrary pre-initialization editing of capture roots/limits;
- Full Evidence as a first-run default;
- file-content secret scanning during preview;
- automatic project initialization from an agent or host hook;
- ambient project discovery;
- a browser-folder Agent Memory setup path.

Those changes would require separate UX and trust decisions.

## Verification

Passing requires:

- initial preview creates no `.ley` metadata;
- the initial preview matches the first initialized preview when the project is unchanged;
- changing the candidate set changes `planFingerprint`;
- identical plans under different canonical project roots produce different desktop approval fingerprints;
- stale expected plans fail before the private Agent Memory store is created;
- an initialized-but-unbound retry requires a fresh reviewed approval before first binding/capture;
- desktop inspection exposes the proposed boundary for an uninitialized project;
- the desktop approval action submits the exact project-bound reviewed fingerprint;
- Rust workspace checks, TypeScript checks, and desktop onboarding tests remain green.
