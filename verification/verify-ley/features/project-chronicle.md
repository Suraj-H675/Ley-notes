# Project Chronicle and Codex capture

M3 records supported observable Codex activity into canonical Project Brain Sessions/Episodes. This is not the M4
`$ley` retrieval flow and it is not the older focused-continuity session recorder.

## Entry points

- Desktop selected project → **Project settings → Codex activity capture** reads only that Brain's authorized working
  copies.
- **Review and enable** opens a native confirmation for the exact Project, working copy, Codex host, and retention
  mode. Connection/import are not capture permission.
- Installed Codex lifecycle hooks invoke `ley hook --host codex` from the working directory.
- `ley brain sessions --project PROJECT` and `ley brain history --project PROJECT --session SESSION` are read-only M3
  inspection surfaces. `ley brain capture-state` inspects one working-copy grant without granting it.

## Automated proof

From the repository root run:

```sh
verification/verify-ley/scripts/verify-project-chronicle
```

The script runs the focused canonical-core tests, the CLI integration that spawns the real `ley hook --host codex`
process, the native Desktop capture-control unit test, the React consent-control test, and validates the bundled
Codex hook manifest. It proves:

- attachment alone does not grant capture and the Project Brain route cannot fall through to legacy capture;
- canonical session/episode persistence, per-session ordering, observed-vs-reported semantics, replay identity,
  redaction, bounds, and explicit gaps;
- independent sequence spaces for parallel sessions;
- returned Bash output is not promoted to verification/success;
- session erase and grant revoke prevent resurrection;
- working-copy movement invalidates an old permission;
- the real CLI hook process persists Chronicle state and ignores `transcript_path`;
- the Desktop control is scoped to the selected Project and requires an explicit enable action.

## Native user/host proof

Use a disposable Project Brain and isolated Ley private state. Do not grant capture in the user's real project merely
for verification.

1. Attach/import the disposable working copy, connect Codex, and verify **Codex activity capture** still says disabled.
2. Choose **Review and enable**. Capture the native confirmation and require the exact Project name/ID, working copy,
   locator, host, retention mode, and exclusions. Cancel once and prove no grant appeared; then confirm deliberately.
3. Start a fresh Codex session in that exact working copy. Exercise a visible prompt and a harmless Bash command,
   allow the session to end, then inspect `ley brain sessions/history` using the same isolated private state.
4. Require ordered Episodes for the supported hooks, redacted/bounded bodies for Structured/Full Evidence, explicit
   gap text, no transcript-path retention, and no claim that returned tool output verified success.
5. Stop capture in Desktop, start another fresh Codex session, and require no new canonical Chronicle session.
6. Erase a disposable Chronicle session and require its history to be unavailable and a later hook with the same
   external session identity to remain non-resurrecting.

If the native accessibility/host prerequisite is unavailable, report this UI/host proof as **blocked**. Do not
substitute the automated test's direct core authorization for proof that a human clicked the native confirmation.

## Transition boundary

Claude Project Brain Chronicle capture is not an M3 claim; it remains for the cross-agent milestone. Projects that
are not routed through an M3 Project Brain Codex capture boundary may still use the older focused-continuity adapter.
That compatibility behavior does not redefine canonical Chronicle semantics.
