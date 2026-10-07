# Session continuity

Ley retains a named session and checkpoint so a later user or agent can inspect what happened.

## Sub-features

- `session-start` records a name and goal.
- `session-checkpoint` retains a summary and touched-file evidence.
- `session-read` retrieves the persisted checkpoint independently.

## How to get to it (user POV)

- Coding-agent lifecycle capture or CLI session commands creates sessions.
- Desktop project `Recall`, then `Sessions`, then the named session card opens history.
- CLI `ley session start PROJECT --name "Verification session" --goal "Preserve verificationbeacon evidence" --json` and `ley session show SESSION PROJECT --json`.

## Driving it with control-ley and native accessibility

Preconditions: captured fixture and successful doctor.

- Run `drive`. `session-start.json` returns the session ID, also retained in `run.json`.
- The helper runs `ley session checkpoint SESSION PROJECT --summary "Verified verificationbeacon capture" --touched README.md --json`. Read `checkpoint.json` and `session-show.json`; the first checkpoint summary must match exactly. This independent process read proves retained state.
- For Desktop, seed a session in that native run's private state using these public CLI commands with the same isolated XDG config. Choose `Recall`, `Sessions`, and `Verification session`. Capture the card-opening action, name, summary, and revision/evidence details. A CLI-seeded session proves viewing, not host-hook capture.

## Gotchas

- Session IDs come from actual command output; never hardcode them.
- Historical session text is retained context, not live source truth.
- This initial recipe does not verify host-hook capture, rename concurrency, erasure, or finish behavior.
