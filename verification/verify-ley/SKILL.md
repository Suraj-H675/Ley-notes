---
name: verify-ley
description: Verify Ley Desktop and its local CLI continuity engine in the Ley-notes repository. Use after changes to capture, sessions, search, brief/evidence, or privacy, or when a runtime proof is needed.
---

# Verify Ley

Use only in `/home/suraj/projects/Ley-notes`. The installed Codex link points at this repository because this checkout's `.agents` and `.codex` directories are read-only. The tracked skill and helper live in `verification/verify-ley/`.

Read [the feature index](features/README.md), then the affected feature files. They describe the current transition runtime, not the future Project Brain navigation in `LEY.md`. Ley Desktop is the primary app. The CLI and native MCP are local engine/integration entry points; the public website is marketing only.

## Launch

For the Linux CLI, from the repo root:

```bash
proof_dir="$PWD/.cache/verification/ley-$(date -u +%Y%m%dT%H%M%S)-$$"
verification/verify-ley/scripts/control-ley launch "$proof_dir"
```

Requires Python 3, Git, pinned Rust from `rust-toolchain.toml`, and cached Cargo dependencies. The helper runs `cargo build --locked --offline -p ley-cli` and copies the resulting binary into a unique `/tmp/ley-verify-*` directory. It records the binary hash, revision, dirty worktree, and every command's output. Readiness means initialization, doctor, and native-continuity ingestion all passed. CLI commands are noninteractive JSON invocations with captured pipes; no server, PTY prompt, port, auth, or daemon is involved.

The helper sets fresh `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, and `XDG_DATA_HOME` for every invocation, clears `LEY_EVAL_PRIVATE_ROOT`, and disables global/system Git config. It uses a disposable project, never this source checkout as the capture target. Concurrent runs need distinct evidence directories. Non-Linux hosts require the existing `eval-private-root` feature/fixtures, since XDG isolation does not establish private storage isolation there.

For native Desktop UI proof, use a separate disposable project and owner-private config directory. First run the native doctor below. On Linux, check `ss -ltnp 'sport = :1420'` and the process tree before launch; an occupied port or an existing user Ley instance is a blocked native check. Do not drive that instance. With native prerequisites available:

```bash
native_scratch="$(mktemp -d /tmp/ley-native-verify-XXXXXXXX)"
mkdir -m 700 "$native_scratch/config" "$native_scratch/cache" "$native_scratch/data" "$native_scratch/project"
printf '# Native verification\nverificationbeacon continuity evidence\n' > "$native_scratch/project/README.md"
env -u LEY_EVAL_PRIVATE_ROOT XDG_CONFIG_HOME="$native_scratch/config" XDG_CACHE_HOME="$native_scratch/cache" XDG_DATA_HOME="$native_scratch/data" npm run desktop
```

Save the scratch path and execution session ID in the native evidence directory before driving. The documented command prepares the helper and starts Vite on port 1420 plus the native app. Identify the native PID/window from that process tree. Readiness requires a rendered Ley window with the Projects hub, not merely a Vite log. Port 1420 and generated Desktop resources are shared; run only one native verification instance at a time.

Use native accessibility automation for the actual Tauri webview. Opening `dev:desktop-ui` in a normal browser cannot prove native filesystem or IPC behavior. If native startup or accessibility fails, record the exact failure and mark UI paths unverified. Keep CLI proof separate.

## Doctor

```bash
verification/verify-ley/scripts/control-ley doctor "$proof_dir"
```

This reads run ownership, checks the copied binary's SHA-256, and runs `ley doctor PROJECT --json`. Require the expected root and project name. It checks identity/capture configuration; it does not by itself prove retrieval or database health. A new failed doctor attempt cleans up scratch and retains its diagnostic record.

For Desktop, first run `cua-driver doctor` and `cua-driver status`. Require reachable accessibility and display services. Discover the live tools with `cua-driver list-tools` and `cua-driver describe get_window_state`, `click`, and `type_text`. Create a unique task session with `start_session`; discover and select only the PID/window belonging to this launch. A fresh accessibility tree must identify Ley and the expected project before each sequence. Never install or upgrade a driver as part of verification.

## Drive

```bash
verification/verify-ley/scripts/control-ley drive "$proof_dir"
```

This executes real `ley preview`, `session start/checkpoint/show`, `search`, and `egress project/list` commands on the fixture. The exact commands, stdout, stderr, and exit codes appear in the evidence JSON files. The feature map lists each assertion and the Desktop entry points that require separate proof.

For Desktop, snapshot the run-owned window, select the stable accessible names in the feature file, act once, then snapshot and verify the postcondition. Use fresh element tokens from the snapshot rather than coordinates or tab order. Cua arguments vary by installed version; use `describe` before supplying PID, window, session, token, or value arguments. Save the exact action request and resulting accessibility tree/screenshot into the evidence directory. An absent or inaccessible control is a blocked path, not permission to invoke internal React setters or Tauri commands instead.

## Evidence

Evidence stays in `$proof_dir`, outside disposable scratch. `run.json` identifies the checkout, binary hash, scope, passed CLI features, and cleanup state. Each command JSON records its arguments, exit code, stdout, and stderr. `files.json` records fixture/private-state file hashes. `session-show.json` and `egress-list.json` prove persisted mutations through independent public reads; source hashes prove the fixture README remained unchanged.

Prove the real user action and resulting state together. For native UI proof, capture the action, accessibility snapshot, and exact-window screenshot before and after, plus a public readback of the relevant persistence. Read historical citations through the evidence entry point. Match the chosen project and egress target. External systems may be mocked only at an existing production boundary; local CLI operations here use no mocks.

The helper uses real writes in disposable state, not a dry-run. No host integration is installed and no provider is contacted by these commands. Cargo uses offline mode. For future dry-run/test-mode recipes, observe actual file, network, and Git-ref effects before claiming they skip them. Report feature IDs and entry points independently; passing the CLI does not verify Desktop, MCP, coding-host hooks, packaged installers, or the website.

## Cleanup

```bash
verification/verify-ley/scripts/control-ley cleanup "$proof_dir"
test -s "$proof_dir/run.json"
test -s "$proof_dir/session-show.json"
```

The helper validates its scratch ownership marker and removes only that scratch directory, including copied binary, fixture project, and private app state. It never removes evidence. `all`, failed launch, and failed drive clean up automatically. After cleanup, a new run needs a new evidence directory. Cleanup can be repeated.

For Desktop, end the Cua task session, stop the exact execution session/process tree started for the run, and verify port 1420 has been released. Remove only the run-owned scratch directory after copying all proof out. Never kill by process name, stop a shared Cua daemon, or erase real project memory. Run cleanup after failed native attempts too.

## Helpers

For Project Brain orientation, follow [the dedicated feature](features/project-orientation.md).
Its offline driver checks the current CLI and MCP in disposable private state. Its native driver retains
real earlier/fresh Codex receipts and records nested command failures at the native host boundary.
Report these gates separately; offline checks cannot establish native fresh-session acceptance.

`scripts/control-ley` is executable and accepts `launch`, `doctor`, `drive`, `cleanup`, and `all`, followed by the evidence directory. To execute the full CLI recipe with automatic cleanup:

```bash
verification/verify-ley/scripts/control-ley all "$proof_dir"
```

The helper uses the checkout's default Cargo target path, `target/debug/ley`. Builds with a custom `CARGO_TARGET_DIR` are unsupported by this recipe. Run `/maintain-verification-skill` when updating this map for changed behavior.
