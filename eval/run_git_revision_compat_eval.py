#!/usr/bin/env python3
"""Measure Ley Git revision compatibility across disposable repository shapes.

This lane is deterministic for classification/command-policy checks but records wall-clock timing as
environment-sensitive evidence. A temporary `git` wrapper is placed only in Ley's subprocess PATH so
fixture setup is not counted. The wrapper delegates to the real Git binary and records argv plus the
safety environment Ley supplied.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, Callable

import run_eval as harness


ALLOWED_GIT_SUBCOMMANDS = {"status", "rev-parse", "merge-base"}
DEFAULT_MAX_GIT_COMMANDS = 3
HARD_MAX_GIT_COMMANDS = 8


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run disposable Git revision-compatibility shapes through real Ley session retrieval and "
            "record query-time Git command counts plus end-to-end query wall time."
        )
    )
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument(
        "--max-git-commands",
        type=int,
        default=DEFAULT_MAX_GIT_COMMANDS,
        help=(
            "Per-query Git subprocess bound; defaults to the optimized query-path ceiling of "
            f"{DEFAULT_MAX_GIT_COMMANDS} and may not exceed the diagnostic safety ceiling of "
            f"{HARD_MAX_GIT_COMMANDS}."
        ),
    )
    parser.add_argument("--output", help="Optional JSON report path")
    parser.add_argument(
        "--require-all",
        action="store_true",
        help="Exit non-zero unless every classification and command-policy check passes.",
    )
    return parser.parse_args()


def log_git_command(args: list[str]) -> None:
    log = Path(os.environ["LEY_GIT_PROBE_LOG"])
    with log.open("a", encoding="utf-8") as handle:
        handle.write(
            json.dumps(
                {
                    "argv": args,
                    "optionalLocks": os.environ.get("GIT_OPTIONAL_LOCKS"),
                    "noLazyFetch": os.environ.get("GIT_NO_LAZY_FETCH"),
                    "locale": os.environ.get("LC_ALL"),
                },
                separators=(",", ":"),
            )
            + "\n"
        )
        handle.flush()


def git_wrapper_main(args: list[str]) -> int:
    real_git = os.environ["LEY_GIT_PROBE_REAL_GIT"]
    log_git_command(args)
    os.execv(real_git, [real_git, *args])
    return 127


def git_log_only_main(args: list[str]) -> int:
    log_git_command(args)
    return 0


def write_git_wrapper(directory: Path) -> Path:
    runner = Path(__file__).resolve()
    if os.name == "nt":
        rustc = shutil.which("rustc")
        if not rustc:
            raise RuntimeError(
                "Windows Git command instrumentation requires rustc to build the temporary git.exe shim"
            )
        source = directory / "git_wrapper.rs"
        source.write_text(
            r'''use std::{env, ffi::OsString, process::{exit, Command, Stdio}};

fn main() {
    let python = env::var("LEY_GIT_PROBE_PYTHON").expect("missing probe Python path");
    let runner = env::var("LEY_GIT_PROBE_RUNNER").expect("missing probe runner path");
    let real_git = env::var("LEY_GIT_PROBE_REAL_GIT").expect("missing real Git path");
    let args: Vec<OsString> = env::args_os().skip(1).collect();

    let logged = Command::new(&python)
        .arg(&runner)
        .arg("--git-log-only")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("failed to launch Git probe logger");
    if !logged.success() {
        exit(logged.code().unwrap_or(1));
    }

    // Launch real Git directly from the native shim so Ley's inherited stdout/stderr pipe handles
    // are not routed through Python's Windows exec emulation.
    let status = Command::new(real_git)
        .args(&args)
        .status()
        .expect("failed to launch real Git");
    exit(status.code().unwrap_or(1));
}
''',
            encoding="utf-8",
        )
        wrapper = directory / "git.exe"
        result = subprocess.run(
            [rustc, str(source), "-O", "-o", str(wrapper)],
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "failed to build temporary Windows git.exe instrumentation shim: "
                + (result.stderr.strip() or result.stdout.strip())
            )
    else:
        wrapper = directory / "git"
        wrapper.write_text(
            f"#!{sys.executable}\n"
            "import os, sys\n"
            f"os.execv({sys.executable!r}, [{sys.executable!r}, {str(runner)!r}, '--git-wrapper', *sys.argv[1:]])\n",
            encoding="utf-8",
        )
        wrapper.chmod(0o755)
    return wrapper


def load_git_log(log: Path) -> list[dict[str, Any]]:
    if not log.exists():
        return []
    rows: list[dict[str, Any]] = []
    for line in log.read_text(encoding="utf-8").splitlines():
        payload = json.loads(line)
        if isinstance(payload, dict):
            rows.append(payload)
    return rows


def verify_git_wrapper_passthrough(wrapper: Path, env: dict[str, str], log: Path) -> None:
    result = subprocess.run(
        [str(wrapper), "--version"],
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if result.returncode != 0 or not result.stdout.strip().startswith("git version "):
        detail = result.stderr.strip() or result.stdout.strip() or "no Git stdout received"
        raise RuntimeError(
            "temporary Git instrumentation shim did not preserve real Git stdout: " + detail
        )
    log.unlink(missing_ok=True)


def powershell_output(script: str, env: dict[str, str] | None = None) -> str:
    powershell = shutil.which("powershell.exe") or shutil.which("powershell")
    if not powershell:
        raise RuntimeError("Windows private-root ACL verification requires PowerShell")
    result = subprocess.run(
        [powershell, "-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script],
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError("PowerShell ACL probe failed: " + detail)
    return result.stdout.strip()


def validate_windows_acl_payload(payload: dict[str, Any], current_sid: str) -> None:
    if payload.get("protected") is not True:
        raise RuntimeError("Windows evaluation private root still inherits parent ACL entries")
    rules = payload.get("access")
    if not isinstance(rules, list) or not rules:
        raise RuntimeError("Windows evaluation private root has no explicit access rule")
    full_control = False
    for rule in rules:
        if not isinstance(rule, dict):
            raise RuntimeError("Windows evaluation private root returned a malformed ACL rule")
        if rule.get("sid") != current_sid:
            raise RuntimeError(
                "Windows evaluation private root grants access to an unexpected trustee: "
                + str(rule.get("sid"))
            )
        if rule.get("type") != "Allow" or rule.get("inherited") is not False:
            raise RuntimeError(
                "Windows evaluation private root contains a denied or inherited access rule"
            )
        inheritance = str(rule.get("inheritance", ""))
        if "ContainerInherit" not in inheritance or "ObjectInherit" not in inheritance:
            raise RuntimeError(
                "Windows evaluation private root access rule is not inheritable by files and directories"
            )
        if str(rule.get("propagation", "")) != "None":
            raise RuntimeError(
                "Windows evaluation private root access rule has unexpected propagation flags"
            )
        if "FullControl" in str(rule.get("rights", "")):
            full_control = True
    if not full_control:
        raise RuntimeError(
            "Windows evaluation private root does not grant the current user full control"
        )


def harden_windows_private_tree(paths: list[Path]) -> bool:
    if os.name != "nt":
        return False
    current_sid = powershell_output(
        "[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value"
    )
    if not current_sid.startswith("S-"):
        raise RuntimeError("failed to resolve the current Windows user SID")

    harden_script = r'''
$ErrorActionPreference = 'Stop'
$sid = [System.Security.Principal.SecurityIdentifier]::new($env:LEY_EVAL_ACL_SID)
$acl = [System.Security.AccessControl.DirectorySecurity]::new()
$acl.SetOwner($sid)
$acl.SetAccessRuleProtection($true, $false)
$rule = [System.Security.AccessControl.FileSystemAccessRule]::new(
    $sid,
    [System.Security.AccessControl.FileSystemRights]::FullControl,
    [System.Security.AccessControl.InheritanceFlags]::ContainerInherit -bor [System.Security.AccessControl.InheritanceFlags]::ObjectInherit,
    [System.Security.AccessControl.PropagationFlags]::None,
    [System.Security.AccessControl.AccessControlType]::Allow
)
[void]$acl.AddAccessRule($rule)
Set-Acl -LiteralPath $env:LEY_EVAL_ACL_PATH -AclObject $acl
'''
    inspect_script = r'''
$acl = Get-Acl -LiteralPath $env:LEY_EVAL_ACL_PATH
$rules = @($acl.Access | ForEach-Object {
    $sid = try {
        $_.IdentityReference.Translate([System.Security.Principal.SecurityIdentifier]).Value
    } catch {
        $_.IdentityReference.Value
    }
    [pscustomobject]@{
        sid = $sid
        type = $_.AccessControlType.ToString()
        rights = $_.FileSystemRights.ToString()
        inheritance = $_.InheritanceFlags.ToString()
        propagation = $_.PropagationFlags.ToString()
        inherited = $_.IsInherited
    }
})
[pscustomobject]@{
    protected = $acl.AreAccessRulesProtected
    access = $rules
} | ConvertTo-Json -Depth 4 -Compress
'''
    for path in paths:
        probe_env = os.environ.copy()
        probe_env["LEY_EVAL_ACL_PATH"] = str(path)
        probe_env["LEY_EVAL_ACL_SID"] = current_sid
        powershell_output(harden_script, probe_env)
        raw = powershell_output(inspect_script, probe_env)
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError as error:
            raise RuntimeError(
                f"Windows ACL probe returned invalid JSON for {path}: {raw}"
            ) from error
        if not isinstance(payload, dict):
            raise RuntimeError(f"Windows ACL probe returned malformed data for {path}")
        validate_windows_acl_payload(payload, current_sid)
    return True


def fixture_git(project: Path, args: list[str]) -> str:
    real_git = harness.EVAL_ENV.get("LEY_GIT_PROBE_REAL_GIT")
    if not real_git:
        raise RuntimeError("Git probe fixture environment has no real Git binary")
    env = os.environ.copy()
    for key in list(env):
        if key.startswith("GIT_"):
            env.pop(key, None)
    env.update(
        {
            "HOME": harness.EVAL_ENV["HOME"],
            "USERPROFILE": harness.EVAL_ENV["USERPROFILE"],
            "XDG_CONFIG_HOME": harness.EVAL_ENV["XDG_CONFIG_HOME"],
            "APPDATA": harness.EVAL_ENV["APPDATA"],
            "LOCALAPPDATA": harness.EVAL_ENV["LOCALAPPDATA"],
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_TERMINAL_PROMPT": "0",
            "LC_ALL": "C",
        }
    )
    result = subprocess.run(
        [real_git, "-C", str(project), *args],
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"fixture git {' '.join(args)} failed: {detail}")
    return result.stdout.strip()


def fixture_commit_all(project: Path, message: str) -> str:
    fixture_git(
        project,
        ["add", "-A", "--", ".", ":(exclude).ley", ":(exclude).ley/**"],
    )
    fixture_git(
        project,
        [
            "-c",
            "user.name=Ley Eval",
            "-c",
            "user.email=ley-eval@example.invalid",
            "commit",
            "-m",
            message,
        ],
    )
    return fixture_git(project, ["rev-parse", "HEAD"])


def run_git_version(real_git: str) -> str:
    result = subprocess.run(
        [real_git, "--version"],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"git --version failed: {detail}")
    return result.stdout.strip()


def git_subcommand(argv: list[str]) -> str | None:
    index = 0
    options_with_values = {"-c", "-C", "--git-dir", "--work-tree", "--namespace"}
    option_prefixes = ("--git-dir=", "--work-tree=", "--namespace=", "--exec-path=")
    while index < len(argv):
        value = argv[index]
        if value in options_with_values:
            index += 2
            continue
        if value.startswith(option_prefixes) or value.startswith("-"):
            index += 1
            continue
        return value
    return None


def new_project(case_dir: Path, name: str) -> tuple[Path, Path, str]:
    project = case_dir / "project"
    vault = case_dir / "vault"
    project.mkdir(parents=True)
    vault.mkdir(parents=True)
    fixture_git(project, ["init", "-b", "main"])
    (project / "README.md").write_text(f"# {name}\n", encoding="utf-8")
    base = fixture_commit_all(project, "base")
    harness.init_project(project, name, vault)
    return project, vault, base


def capture_session(project: Path, seed: str) -> str:
    session_id, _ = harness.create_structured_session(
        project,
        seed=seed,
        name=f"Revision probe {seed}",
        goal="Preserve revision applicability",
        summary=f"Captured revision state for {seed}",
        decisions=[
            {
                "title": f"Revision decision {seed}",
                "decision": f"Use revision state {seed}",
            }
        ],
    )
    return session_id


def commit_file(project: Path, relative: str, body: str, message: str) -> str:
    destination = project / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(body, encoding="utf-8")
    return fixture_commit_all(project, message)


def prepare_current(case_dir: Path) -> tuple[Path, str]:
    project, _, _ = new_project(case_dir, "current-lineage")
    return project, capture_session(project, "current-lineage")


def prepare_ancestor(case_dir: Path) -> tuple[Path, str]:
    project, _, _ = new_project(case_dir, "ancestor")
    session = capture_session(project, "ancestor")
    commit_file(project, "next.txt", "descendant\n", "descendant")
    return project, session


def prepare_detached_ancestor(case_dir: Path) -> tuple[Path, str]:
    project, _, _ = new_project(case_dir, "detached-ancestor")
    session = capture_session(project, "detached-ancestor")
    current = commit_file(project, "next.txt", "descendant\n", "descendant")
    fixture_git(project, ["checkout", "--detach", current])
    return project, session


def prepare_linked_worktree_ancestor(case_dir: Path) -> tuple[Path, str]:
    repository = case_dir / "repository"
    project = case_dir / "linked-worktree"
    vault = case_dir / "vault"
    repository.mkdir(parents=True)
    vault.mkdir(parents=True)
    fixture_git(repository, ["init", "-b", "main"])
    (repository / "README.md").write_text(
        "# linked-worktree-ancestor\n", encoding="utf-8"
    )
    fixture_commit_all(repository, "base")
    fixture_git(
        repository,
        ["worktree", "add", "-b", "worktree-probe", str(project), "HEAD"],
    )
    if not (project / ".git").is_file():
        raise RuntimeError("linked-worktree probe did not create Git file indirection")
    harness.init_project(project, "linked-worktree-ancestor", vault)
    session = capture_session(project, "linked-worktree-ancestor")
    commit_file(project, "next.txt", "linked descendant\n", "linked descendant")
    return project, session


def prepare_divergent_common(
    case_dir: Path, name: str
) -> tuple[Path, str, str, str]:
    project, _, base = new_project(case_dir, name)
    fixture_git(project, ["checkout", "-b", "experiment"])
    experiment = commit_file(
        project, "experiment.txt", "experiment\n", "experiment"
    )
    harness.run(["ingest", str(project), "--json"])
    session = capture_session(project, name)
    fixture_git(project, ["checkout", "main"])
    main = commit_file(project, "main.txt", "mainline\n", "mainline")
    return project, session, base, experiment


def prepare_divergent(case_dir: Path) -> tuple[Path, str]:
    project, session, _, _ = prepare_divergent_common(case_dir, "divergent")
    return project, session


def prepare_merged(case_dir: Path) -> tuple[Path, str]:
    project, session, _, _ = prepare_divergent_common(case_dir, "merged")
    fixture_git(
        project,
        [
            "-c",
            "user.name=Ley Eval",
            "-c",
            "user.email=ley-eval@example.invalid",
            "merge",
            "--no-ff",
            "experiment",
            "-m",
            "merge experiment",
        ],
    )
    return project, session


def prepare_shallow_unknown(case_dir: Path) -> tuple[Path, str]:
    project, session, base, _ = prepare_divergent_common(
        case_dir, "shallow-unknown"
    )
    shallow = Path(fixture_git(project, ["rev-parse", "--git-path", "shallow"]))
    if not shallow.is_absolute():
        shallow = project / shallow
    shallow.parent.mkdir(parents=True, exist_ok=True)
    shallow.write_text(base + "\n", encoding="utf-8")
    if fixture_git(project, ["rev-parse", "--is-shallow-repository"]) != "true":
        raise RuntimeError("failed to establish the shallow-repository probe shape")
    return project, session


def prepare_missing_metadata(case_dir: Path) -> tuple[Path, str]:
    project, _, _ = new_project(case_dir, "missing-git-metadata")
    session = capture_session(project, "missing-git-metadata")
    (project / ".git").rename(project / ".git.probe-hidden")
    return project, session


CASE_PREPARERS: list[
    tuple[str, str, bool, Callable[[Path], tuple[Path, str]]]
] = [
    ("current-lineage", "current-lineage", True, prepare_current),
    ("ancestor", "ancestor", True, prepare_ancestor),
    ("detached-ancestor", "ancestor", True, prepare_detached_ancestor),
    ("linked-worktree-ancestor", "ancestor", True, prepare_linked_worktree_ancestor),
    ("divergent", "divergent", True, prepare_divergent),
    ("merged", "merged", True, prepare_merged),
    ("shallow-unknown", "unknown", True, prepare_shallow_unknown),
    ("missing-git-metadata", "unknown", False, prepare_missing_metadata),
]


def query_once(
    *,
    project: Path,
    session_id: str,
    log: Path,
) -> tuple[dict[str, Any], float, list[dict[str, Any]]]:
    log.write_text("", encoding="utf-8")
    started = time.perf_counter()
    payload = harness.mcp_call(
        project,
        "ley_session_get",
        {
            "sessionId": session_id,
            "maxCheckpoints": 5,
            "maxCharacters": 8_000,
        },
    )
    elapsed_ms = (time.perf_counter() - started) * 1000.0
    return payload, elapsed_ms, load_git_log(log)


def actual_compatibility(payload: dict[str, Any]) -> tuple[str, str, bool]:
    checkpoints = [
        item for item in payload.get("checkpoints", []) if isinstance(item, dict)
    ]
    if not checkpoints:
        raise RuntimeError("revision probe returned no checkpoint")
    checkpoint = checkpoints[-1]
    applicability = checkpoint.get("revisionApplicability", {})
    freshness = payload.get("revisionFreshness", {})
    if not isinstance(applicability, dict) or not isinstance(freshness, dict):
        raise RuntimeError("revision probe returned malformed revision metadata")
    return (
        str(applicability.get("compatibility", "")),
        str(freshness.get("captureCompatibility", "")),
        freshness.get("liveGitChecked") is True,
    )


def evaluate_case(
    *,
    base: Path,
    name: str,
    expected: str,
    expected_live_git: bool,
    prepare: Callable[[Path], tuple[Path, str]],
    repetitions: int,
    max_git_commands: int,
    log: Path,
) -> dict[str, Any]:
    case_dir = base / name
    case_dir.mkdir()
    project, session_id = prepare(case_dir)
    samples: list[dict[str, Any]] = []
    for _ in range(repetitions):
        payload, wall_ms, commands = query_once(
            project=project, session_id=session_id, log=log
        )
        checkpoint_compat, capture_compat, live_git_checked = actual_compatibility(
            payload
        )
        kinds = [
            kind
            for row in commands
            for kind in [git_subcommand([str(value) for value in row.get("argv", [])])]
            if kind is not None
        ]
        instrumentation_observed = bool(commands)
        command_policy_ok = (
            instrumentation_observed
            and len(kinds) == len(commands)
            and set(kinds).issubset(ALLOWED_GIT_SUBCOMMANDS)
            and all(row.get("optionalLocks") == "0" for row in commands)
            and all(row.get("noLazyFetch") == "1" for row in commands)
        )
        sample_ok = (
            checkpoint_compat == expected
            and capture_compat == expected
            and live_git_checked is expected_live_git
            and command_policy_ok
            and len(commands) <= max_git_commands
        )
        samples.append(
            {
                "ok": sample_ok,
                "wallMs": round(wall_ms, 3),
                "checkpointCompatibility": checkpoint_compat,
                "captureCompatibility": capture_compat,
                "liveGitChecked": live_git_checked,
                "gitCommandCount": len(commands),
                "gitCommandKinds": kinds,
                "gitCommands": [row.get("argv", []) for row in commands],
                "gitInstrumentationObserved": instrumentation_observed,
                "commandPolicyOk": command_policy_ok,
            }
        )
    return {
        "expectedCompatibility": expected,
        "expectedLiveGitChecked": expected_live_git,
        "ok": all(sample["ok"] for sample in samples),
        "medianWallMs": round(
            statistics.median(sample["wallMs"] for sample in samples), 3
        ),
        "maxGitCommandCount": max(sample["gitCommandCount"] for sample in samples),
        "samples": samples,
    }


def main() -> int:
    args = parse_args()
    if args.repetitions < 1:
        raise RuntimeError("--repetitions must be at least 1")
    if args.max_git_commands < 1:
        raise RuntimeError("--max-git-commands must be at least 1")
    if args.max_git_commands > HARD_MAX_GIT_COMMANDS:
        raise RuntimeError(
            f"--max-git-commands may not exceed the hard safety ceiling of {HARD_MAX_GIT_COMMANDS}"
        )
    real_git = shutil.which("git")
    if not real_git:
        raise RuntimeError("git is required to construct the compatibility matrix")
    original_eval_env = dict(harness.EVAL_ENV)
    try:
        with tempfile.TemporaryDirectory(prefix="ley-git-revision-compat-") as directory:
            base = Path(directory)
            private_root = base / "private-state"
            xdg_config = private_root / "config"
            xdg_cache = private_root / "cache"
            home = base / "home"
            appdata = base / "appdata"
            local_appdata = base / "local-appdata"
            wrapper_dir = base / "git-wrapper"
            wrapper_dir.mkdir()
            private_root.mkdir(mode=0o700)
            xdg_config.mkdir(mode=0o700)
            xdg_cache.mkdir(mode=0o700)
            if os.name != "nt":
                private_root.chmod(0o700)
                xdg_config.chmod(0o700)
                xdg_cache.chmod(0o700)
            windows_private_root_dacl_verified = harden_windows_private_tree(
                [private_root, xdg_config, xdg_cache]
            )
            home.mkdir()
            appdata.mkdir()
            local_appdata.mkdir()
            log = base / "git-commands.jsonl"
            wrapper = write_git_wrapper(wrapper_dir)
            wrapper_path = str(wrapper_dir) + os.pathsep + os.environ.get("PATH", "")
            harness.EVAL_ENV.clear()
            harness.EVAL_ENV.update(
                {
                    "LEY_EVAL_PRIVATE_ROOT": str(private_root),
                    "XDG_CONFIG_HOME": str(xdg_config),
                    "XDG_CACHE_HOME": str(xdg_cache),
                    "HOME": str(home),
                    "USERPROFILE": str(home),
                    "APPDATA": str(appdata),
                    "LOCALAPPDATA": str(local_appdata),
                    "PATH": wrapper_path,
                    "LEY_GIT_PROBE_REAL_GIT": real_git,
                    "LEY_GIT_PROBE_LOG": str(log),
                    "LEY_GIT_PROBE_PYTHON": sys.executable,
                    "LEY_GIT_PROBE_RUNNER": str(Path(__file__).resolve()),
                }
            )
            wrapper_env = os.environ.copy()
            wrapper_env.update(harness.EVAL_ENV)
            verify_git_wrapper_passthrough(wrapper, wrapper_env, log)
            cases: dict[str, Any] = {}
            for name, expected, expected_live_git, prepare in CASE_PREPARERS:
                cases[name] = evaluate_case(
                    base=base,
                    name=name,
                    expected=expected,
                    expected_live_git=expected_live_git,
                    prepare=prepare,
                    repetitions=args.repetitions,
                    max_git_commands=args.max_git_commands,
                    log=log,
                )

            # Git-not-found is a separate degradation shape: keep the captured session but remove Git
            # from Ley's PATH only for the measured query.
            missing_dir = base / "missing-git-binary"
            missing_dir.mkdir()
            project, session_id = prepare_current(missing_dir)
            empty_path = base / "empty-path"
            empty_path.mkdir()
            harness.EVAL_ENV["PATH"] = str(empty_path)
            missing_samples: list[dict[str, Any]] = []
            for _ in range(args.repetitions):
                payload, wall_ms, commands = query_once(
                    project=project, session_id=session_id, log=log
                )
                checkpoint_compat, capture_compat, live_git_checked = actual_compatibility(
                    payload
                )
                ok = (
                    checkpoint_compat == "unknown"
                    and capture_compat == "unknown"
                    and live_git_checked is False
                    and commands == []
                )
                missing_samples.append(
                    {
                        "ok": ok,
                        "wallMs": round(wall_ms, 3),
                        "checkpointCompatibility": checkpoint_compat,
                        "captureCompatibility": capture_compat,
                        "liveGitChecked": live_git_checked,
                        "gitCommandCount": 0,
                        "gitCommandKinds": [],
                        "gitCommands": [],
                        "gitInstrumentationObserved": False,
                        "commandPolicyOk": True,
                    }
                )
            cases["missing-git-binary"] = {
                "expectedCompatibility": "unknown",
                "expectedLiveGitChecked": False,
                "ok": all(sample["ok"] for sample in missing_samples),
                "medianWallMs": round(
                    statistics.median(sample["wallMs"] for sample in missing_samples), 3
                ),
                "maxGitCommandCount": 0,
                "samples": missing_samples,
            }

            report: dict[str, Any] = {
                "schemaVersion": 1,
                "platform": {
                    "system": platform.system(),
                    "release": platform.release(),
                    "machine": platform.machine(),
                    "python": platform.python_version(),
                    "git": run_git_version(real_git),
                },
                "temporaryStateOnly": True,
                "allowedGitSubcommands": sorted(ALLOWED_GIT_SUBCOMMANDS),
                "maxGitCommandsPerQuery": args.max_git_commands,
                "windowsPrivateRootDaclVerified": (
                    windows_private_root_dacl_verified if os.name == "nt" else None
                ),
                "repetitions": args.repetitions,
                "cases": cases,
                "allPassed": all(case["ok"] for case in cases.values()),
            }
    finally:
        harness.EVAL_ENV.clear()
        harness.EVAL_ENV.update(original_eval_env)

    output = json.dumps(report, indent=2, sort_keys=True)
    print(output)
    if args.output:
        Path(args.output).write_text(output + "\n", encoding="utf-8")
    if args.require_all and not report["allPassed"]:
        return 1
    return 0


if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "--git-wrapper":
        raise SystemExit(git_wrapper_main(sys.argv[2:]))
    if len(sys.argv) >= 2 and sys.argv[1] == "--git-log-only":
        raise SystemExit(git_log_only_main(sys.argv[2:]))
    raise SystemExit(main())
