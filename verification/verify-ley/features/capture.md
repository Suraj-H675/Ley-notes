# Project onboarding and capture

A user deliberately enables Ley on a chosen coding project and captures bounded project evidence into private local continuity.

## Sub-features

- `capture-enable` creates project identity after an explicit choice.
- `capture-preview` lists the allowed file boundary.
- `capture-refresh` writes a current private snapshot without a legacy vault.

## How to get to it (user POV)

- Desktop Projects hub, `Add project`, native folder picker, `Enable Ley for this project`.
- Existing project, `Capture project` when required, or `Refresh snapshot`.
- CLI `ley init PROJECT --name "Ley verification" --json`, `ley preview PROJECT --json`, `ley ingest PROJECT --json`.

## Driving it with control-ley and native accessibility

Preconditions: a disposable project with the seeded README and isolated private state.

- Run helper `launch`, then `drive`. Inspect `init.json`, `doctor.json`, `preview.json`, and `ingest.json`. Preview must include `README.md`; ingestion must report `storage: native-continuity` with a null binding. `files.json` must include private config persistence and the unchanged fixture source hash.
- For Desktop, capture the fresh Projects hub, choose `Add project`, select only the disposable folder, and capture the bounded preview before enabling. Verify the project name and file boundary. Enable, wait for the project workspace, then choose `Refresh snapshot` and verify captured revision/time through Evidence. Capture both actions and their resulting state.
- Inspect fixture contents and private state after capture. Project identity/config writes are expected; README content must remain unchanged. No legacy vault should appear.

## Gotchas

- CLI `preview` requires initialized identity; Desktop has an onboarding preview before enabling. The helper's post-init preview does not prove the pre-enable UI boundary.
- `doctor` checks configuration, not full snapshot integrity.
- Folder selection and capture must target disposable state. Existing projects or neighboring folders invalidate isolation.
