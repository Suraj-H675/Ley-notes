# Native Project Brain orientation

M4 verification uses a disposable Brain and two genuinely separate Codex processes.
The earlier process receives a durable FIFO webhook requirement, investigates SQLite versus Redis,
observes a failed-send test, fixes acknowledgement order, verifies the fix, and leaves retry scheduling open.
The fresh process receives `$ley` alone through the installed native Ley skill. It must recover exact
retained evidence and continue the unresolved work without repeating delete-before-send.

## Runtime preflight

Before private-home preparation, the driver checks Codex's fixed shared socket directory,
`/tmp/codex-daemon-UID`. Codex resolves this directory independently of `CODEX_HOME` and `TMPDIR`.
If the directory exists, `shared-runtime-preflight.json` records its owner, mode, and symlink state.
The driver never changes this shared directory. Its state describes the outer process boundary and does
not gate native app-server startup. When it can verify an owned mode-`0000` directory at an exact read-only
mountpoint, it requests Codex's `externalSandbox` turn policy with restricted networking. This avoids a
second bubblewrap filesystem sandbox while retaining the verified outer sandbox. Direct host launches,
or ambiguous mount evidence, use ordinary `workspaceWrite` instead; do not treat an unverified caller as
externally sandboxed. The nested path requires managed policy to allow `external-sandbox`; if it does not,
report the policy rejection as blocked native execution rather than a model or provider failure.

`scripts/verify-project-orientation-native` establishes the private Codex runtime before every app-server launch.
It validates current-user ownership and mode `0700` for `CODEX_HOME`, `tmp`, `ipc`, `app-server-control`,
`app-server-daemon`, their existing subdirectories, and the dedicated XDG runtime directory.
It rejects symlink directories, routes `TMPDIR`, `TMP`, and `TEMP` into the private runtime,
and starts the child with umask `0077` so newly created directories remain private.
`runtime-before-launch.json` records the checked paths, owners, modes, and child umask.

Run the focused regression from the repository root:

```sh
python3 -m unittest discover -s verification/verify-ley/tests -p test_native_runtime.py -v
```

The regression covers existing `0755` private-home directories, rejection of symlinks without changing their targets,
private directories created by a real child process even when the shared directory is masked,
the `externalSandbox` turn policy, and accurate reporting of the shared socket directory.
It does not prove nested Codex command execution.

## Real native sessions

The setup directory contains `setup.json` with the disposable project, isolated Codex home,
Codex binary, installed local plugin, and existing account auth origin. Auth contents are not recorded.
The isolated home includes only the shipped Ley plugin and its audited lifecycle hooks.
Capture permission in the disposable Brain is a separate fixture setup action, not a native UI acceptance claim.

Run an earlier session into a unique attempt directory:

```sh
verification/verify-ley/scripts/verify-project-orientation-native SETUP_DIR --mode earlier --attempt EARLIER_ATTEMPT
```

After the M4 binary and installed native skill are ready, run a fresh process:

```sh
verification/verify-ley/scripts/verify-project-orientation-native SETUP_DIR --mode fresh --attempt FRESH_ATTEMPT
```

The fresh request contains only `$ley` and the native skill input. Its same-thread followup asks why Redis
was rejected, then requests continuation of retry scheduling. No earlier conversation or seeded answer is supplied.
Each launch preserves protocol messages, stderr, exact thread and turn IDs, hook discovery, and cleanup state.
Existing attempt directories are never overwritten. Current routing defaults to native Luna at `xhigh`.
A failed turn returns a nonzero driver exit code. `modelProcessStarted` distinguishes app-server launch
from a process that failed before initialization.

## Offline engine and transport proof

Build the current CLI with locked, cached dependencies, then exercise actual CLI commands and stdio MCP
in disposable private state:

```sh
cargo build --locked --offline -p ley-cli
verification/verify-ley/scripts/verify-project-orientation-offline NEW_PROOF_DIR --binary target/debug/ley
```

The driver creates its proof directory with mode `0700` and uses mode `0600` for new files. It records
the binary hash, Git revision and dirty state, command outputs and exit codes, and complete MCP requests
and responses. It verifies unregistered preview without creating a Brain database
or marker, exact source-only Project selection, read-only tool inventory even with legacy write flags,
natural-language lexical retrieval through both `ley_brief(task=...)` and `ley_search`, exact citation readback, rejected cross-project/hash/range forgery,
and retained evidence after a live-source change. Scratch is removed and receipts remain in the proof directory.

The fixture sources are deterministic test inputs. This driver does not create historical agent activity,
launch Codex, or prove that a fresh agent recovered or continued anything. Core behavior/security tests
cover the remaining Chronicle, lifecycle, revocation, erasure, and concurrency boundaries.

## Acceptance evidence

A completed model turn is not proof that a shell command ran or that the task succeeded.
Check actual command completion, exit codes, and output, then independently inspect public Brain session/history
readbacks and the queue behavior oracle. Read important returned citations through the native evidence tool.
Require exact project isolation, evidence basis, revision and applicability warnings, retained source versions,
the failed approach and cause, observed verification, and unresolved work.

Preserve blocked command diagnostics and mark affected gates unverified. A sandboxed shell smoke test proves
only command execution. It does not prove orientation, native retrieval, continuation, or Desktop UI behavior.
