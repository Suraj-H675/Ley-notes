#!/usr/bin/env python3
"""Run opt-in black-box agent task evaluations against synthetic Ley fixtures.

This runner is intentionally separate from run_eval.py's deterministic acceptance
corpus. It may invoke a paid/networked external agent command supplied by the
operator, so results are comparative observations rather than CI-stable proof.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import selectors
import secrets
import shutil
import shlex
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
from pathlib import Path

from run_eval import (
    EVAL_ENV,
    WRITE_FLAGS,
    create_structured_session,
    git_commit_all,
    git_run,
    init_project,
    mcp_call,
    request_id,
    write_project_files,
)


REPO_ROOT = Path(__file__).resolve().parents[1]
FIXTURES = Path(__file__).parent / "fixtures" / "agent_tasks.jsonl"
DEFAULT_MAX_RESULTS = 8
DEFAULT_MAX_TOKENS = 500
MAX_REPETITIONS = 10
MAX_SANDBOX_OUTPUT_BYTES = 1_048_576
DEFAULT_RUNNER_ENV = (
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TERM",
    "COLORTERM",
)


def load_fixtures() -> list[dict[str, object]]:
    fixtures: list[dict[str, object]] = []
    seen_ids: set[str] = set()
    for line_number, raw in enumerate(FIXTURES.read_text(encoding="utf-8").splitlines(), start=1):
        if not raw.strip():
            continue
        try:
            value = json.loads(raw)
        except json.JSONDecodeError as error:
            raise RuntimeError(
                f"{FIXTURES.name}:{line_number} contains invalid JSON"
            ) from error
        if not isinstance(value, dict) or not isinstance(value.get("id"), str):
            raise RuntimeError(
                f"{FIXTURES.name}:{line_number} must contain an object with a string id"
            )
        validate_fixture_schema(value, line_number)
        fixture_id = str(value["id"])
        if fixture_id in seen_ids:
            raise RuntimeError(f"{FIXTURES.name}:{line_number} duplicates fixture id {fixture_id!r}")
        seen_ids.add(fixture_id)
        fixtures.append(value)
    if not fixtures:
        raise RuntimeError("agent task fixture corpus is empty")
    return fixtures


def validate_fixture_schema(fixture: dict[str, object], line_number: int) -> None:
    prefix = f"{FIXTURES.name}:{line_number}"
    for field in ("id", "task"):
        value = fixture.get(field)
        if not isinstance(value, str) or not value.strip():
            raise RuntimeError(f"{prefix} requires non-empty string {field}")

    project_files = fixture.get("project_files")
    if not isinstance(project_files, dict) or not project_files:
        raise RuntimeError(f"{prefix} requires non-empty project_files")
    for relative, body in project_files.items():
        if not isinstance(relative, str) or not isinstance(body, str):
            raise RuntimeError(f"{prefix} project_files must map string paths to string bodies")
        path = Path(relative)
        if path.is_absolute() or ".." in path.parts:
            raise RuntimeError(f"{prefix} contains unsafe project path {relative!r}")

    allowed = fixture.get("allowed_changed_files")
    if not isinstance(allowed, list) or not allowed or not all(
        isinstance(value, str) and value for value in allowed
    ):
        raise RuntimeError(f"{prefix} requires non-empty string allowed_changed_files")
    if len(set(allowed)) != len(allowed):
        raise RuntimeError(f"{prefix} allowed_changed_files contains duplicates")
    unknown_allowed = sorted(set(allowed) - set(project_files))
    if unknown_allowed:
        raise RuntimeError(
            f"{prefix} allowed_changed_files are not initial project files: {unknown_allowed}"
        )

    visible_test_command = fixture.get("visible_test_command")
    if not isinstance(visible_test_command, list) or not visible_test_command or not all(
        isinstance(value, str) and value for value in visible_test_command
    ):
        raise RuntimeError(f"{prefix} requires non-empty string visible_test_command")

    prior = fixture.get("prior_memory")
    required_prior = ("name", "goal", "summary", "decision_title", "decision")
    if not isinstance(prior, dict) or any(
        not isinstance(prior.get(field), str) or not str(prior[field]).strip()
        for field in required_prior
    ):
        raise RuntimeError(f"{prefix} requires complete string prior_memory fields")

    markers = fixture.get("context_markers")
    if not isinstance(markers, list) or not markers or not all(
        isinstance(value, str) and value for value in markers
    ):
        raise RuntimeError(f"{prefix} requires non-empty string context_markers")
    prior_text = json.dumps(prior, sort_keys=True).lower()
    missing_prior_markers = [marker for marker in markers if marker.lower() not in prior_text]
    if missing_prior_markers:
        raise RuntimeError(
            f"{prefix} context markers are absent from prior_memory: {missing_prior_markers}"
        )

    oracle_probe = fixture.get("oracle_probe")
    if not isinstance(oracle_probe, dict):
        raise RuntimeError(f"{prefix} requires oracle_probe")
    module = oracle_probe.get("module")
    function = oracle_probe.get("function")
    inputs = oracle_probe.get("inputs")
    if (
        not isinstance(module, str)
        or module not in project_files
        or Path(module).is_absolute()
        or ".." in Path(module).parts
    ):
        raise RuntimeError(f"{prefix} oracle_probe.module must name an initial project file")
    if (
        not isinstance(function, str)
        or not function.isidentifier()
    ):
        raise RuntimeError(f"{prefix} oracle_probe.function must be a Python identifier")
    if not isinstance(inputs, list):
        raise RuntimeError(f"{prefix} oracle_probe.inputs must be a JSON array")
    if not isinstance(fixture.get("secret_contract"), dict):
        expected = fixture.get("oracle_expected")
        if not isinstance(expected, list) or len(expected) != len(inputs):
            raise RuntimeError(
                f"{prefix} non-secret fixtures require oracle_expected matching oracle_probe.inputs"
            )
    secret_contract = fixture.get("secret_contract")
    if secret_contract is not None:
        if (
            not isinstance(secret_contract, dict)
            or secret_contract.get("kind") not in {"retry-schedule-v1"}
        ):
            raise RuntimeError(f"{prefix} contains an unsupported secret_contract")


def load_master_seed(path: Path | None) -> bytes:
    if path is None:
        return secrets.token_bytes(32)
    try:
        text = path.read_text(encoding="utf-8").strip()
        seed = bytes.fromhex(text)
    except (OSError, UnicodeError, ValueError) as error:
        raise RuntimeError(
            f"could not read a hexadecimal fixture seed from {path}"
        ) from error
    if len(seed) != 32:
        raise RuntimeError("fixture seed must decode to exactly 32 bytes")
    return seed


def write_master_seed(path: Path, seed: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        descriptor = os.open(
            path,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL,
            0o600,
        )
    except OSError as error:
        raise RuntimeError(f"could not create fixture seed file {path}") from error
    try:
        os.write(descriptor, seed.hex().encode("ascii") + b"\n")
    finally:
        os.close(descriptor)


def sha256_text(value: str) -> str:
    return "sha256:" + hashlib.sha256(value.encode("utf-8")).hexdigest()


def sha256_bytes(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def parse_runner_read_only_mount(spec: str) -> tuple[Path, str]:
    if "=" not in spec:
        raise RuntimeError(
            "runner read-only mount must use SOURCE=DEST with an absolute DEST"
        )
    source_text, destination = spec.split("=", 1)
    if not source_text or not destination.startswith("/"):
        raise RuntimeError(
            "runner read-only mount must use SOURCE=DEST with an absolute DEST"
        )
    destination_path = Path(destination)
    if ".." in destination_path.parts:
        raise RuntimeError("runner read-only mount destination cannot contain '..'")
    if destination == "/home/runner" or not destination.startswith("/home/runner/"):
        raise RuntimeError(
            "runner read-only mount destinations must live beneath /home/runner/"
        )
    try:
        source = Path(source_text).expanduser().resolve(strict=True)
    except OSError as error:
        raise RuntimeError("runner read-only mount source does not exist") from error
    return source, destination


def runner_sandbox_command(
    command: list[str],
    project: Path,
    inherited_env_names: list[str],
    read_only_mounts: list[tuple[Path, str]],
) -> tuple[list[str], dict[str, str]]:
    bwrap = shutil.which("bwrap")
    if bwrap is None:
        raise RuntimeError("bwrap is required for external-agent evaluation")
    if not command:
        raise RuntimeError("external agent command cannot be empty")

    host_path = os.environ.get("PATH", os.defpath)
    executable = shutil.which(command[0], path=host_path)
    if executable is None:
        raise RuntimeError("external agent executable could not be resolved")
    executable_real = Path(executable).resolve()
    if executable_real.is_relative_to(Path("/usr")):
        sandbox_executable = str(executable_real)
        executable_mount: tuple[Path, str] | None = None
    else:
        sandbox_executable = "/runner/executable"
        executable_mount = (executable_real, sandbox_executable)

    sandbox = [
        bwrap,
        "--die-with-parent",
        "--unshare-all",
        "--share-net",
        "--clearenv",
        "--ro-bind",
        "/usr",
        "/usr",
        "--symlink",
        "usr/bin",
        "/bin",
        "--symlink",
        "usr/lib",
        "/lib",
        "--symlink",
        "usr/lib",
        "/lib64",
        "--ro-bind-try",
        "/etc/ssl",
        "/etc/ssl",
        "--ro-bind-try",
        "/etc/resolv.conf",
        "/etc/resolv.conf",
        "--ro-bind-try",
        "/etc/hosts",
        "/etc/hosts",
        "--ro-bind-try",
        "/etc/nsswitch.conf",
        "/etc/nsswitch.conf",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--tmpfs",
        "/tmp",
        "--dir",
        "/runner",
        "--dir",
        "/home",
        "--dir",
        "/home/runner",
    ]
    if executable_mount is not None:
        sandbox.extend(
            ["--ro-bind", str(executable_mount[0]), executable_mount[1]]
        )

    created_dirs = {"/home", "/home/runner"}
    for source, destination in read_only_mounts:
        parent = Path(destination).parent
        ancestors: list[str] = []
        while str(parent).startswith("/home/runner/") and str(parent) not in created_dirs:
            ancestors.append(str(parent))
            parent = parent.parent
        for directory in reversed(ancestors):
            sandbox.extend(["--dir", directory])
            created_dirs.add(directory)
        sandbox.extend(["--ro-bind", str(source), destination])

    sandbox.extend(
        [
            "--bind",
            str(project),
            "/workspace",
            "--chdir",
            "/workspace",
            "--setenv",
            "PATH",
            "/usr/bin:/bin:/runner",
            "--setenv",
            "HOME",
            "/home/runner",
            "--setenv",
            "PWD",
            "/workspace",
            "--setenv",
            "USER",
            "runner",
            "--setenv",
            "LOGNAME",
            "runner",
        ]
    )
    for name in (*DEFAULT_RUNNER_ENV, *inherited_env_names):
        value = os.environ.get(name)
        if value is not None:
            sandbox.extend(["--setenv", name, value])
    sandbox.extend([sandbox_executable, *command[1:]])
    host_env = {"PATH": host_path}
    return sandbox, host_env


def run_bounded_process(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str],
    stdin_bytes: bytes | None,
    timeout_seconds: int,
    capture_raw: bool,
    max_output_bytes: int = MAX_SANDBOX_OUTPUT_BYTES,
) -> dict[str, object]:
    started = time.monotonic()
    process = subprocess.Popen(
        command,
        cwd=cwd,
        stdin=subprocess.PIPE if stdin_bytes is not None else subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        start_new_session=True,
    )
    if process.stdout is None or process.stderr is None:
        raise RuntimeError("failed to capture subprocess output")
    if stdin_bytes is not None:
        if process.stdin is None:
            raise RuntimeError("failed to open subprocess stdin")
        try:
            process.stdin.write(stdin_bytes)
            process.stdin.flush()
        except BrokenPipeError:
            pass
        finally:
            process.stdin.close()

    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ, "stdout")
    selector.register(process.stderr, selectors.EVENT_READ, "stderr")
    counts = {"stdout": 0, "stderr": 0}
    digests = {
        "stdout": hashlib.sha256(),
        "stderr": hashlib.sha256(),
    }
    raw = {
        "stdout": bytearray(),
        "stderr": bytearray(),
    }
    deadline = started + timeout_seconds
    timed_out = False
    output_limit_exceeded = False

    try:
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0 and process.poll() is None:
                timed_out = True
                break
            events = selector.select(timeout=max(0.0, min(0.1, remaining)))
            if not events and process.poll() is not None:
                events = [
                    (key, selectors.EVENT_READ)
                    for key in list(selector.get_map().values())
                ]
            for key, _ in events:
                try:
                    chunk = os.read(key.fd, 65536)
                except BlockingIOError:
                    continue
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                stream = str(key.data)
                counts[stream] += len(chunk)
                digests[stream].update(chunk)
                if capture_raw and len(raw[stream]) < max_output_bytes:
                    remaining_raw = max_output_bytes - len(raw[stream])
                    raw[stream].extend(chunk[:remaining_raw])
                if counts["stdout"] + counts["stderr"] > max_output_bytes:
                    output_limit_exceeded = True
                    break
            if output_limit_exceeded:
                break
    finally:
        if timed_out or output_limit_exceeded or process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
        for key in list(selector.get_map().values()):
            try:
                selector.unregister(key.fileobj)
            except Exception:
                pass
        selector.close()
        process.stdout.close()
        process.stderr.close()

    elapsed = time.monotonic() - started
    result: dict[str, object] = {
        "completed": (
            not timed_out
            and not output_limit_exceeded
            and process.returncode == 0
        ),
        "timedOut": timed_out,
        "outputLimitExceeded": output_limit_exceeded,
        "exitCode": process.returncode,
        "seconds": round(elapsed, 3),
        "stdoutBytes": counts["stdout"],
        "stderrBytes": counts["stderr"],
        "stdoutSha256": "sha256:" + digests["stdout"].hexdigest(),
        "stderrSha256": "sha256:" + digests["stderr"].hexdigest(),
    }
    if capture_raw:
        result["_stdout"] = bytes(raw["stdout"])
        result["_stderr"] = bytes(raw["stderr"])
    return result


def derive_retry_schedule(seed: bytes) -> list[int]:
    values: list[int] = []
    counter = 0
    while len(values) < 4:
        digest = hashlib.sha256(
            seed + b":retry-schedule-v1:" + counter.to_bytes(4, "big")
        ).digest()
        counter += 1
        for byte in digest:
            candidate = 2 + (byte % 28)
            if candidate not in values:
                values.append(candidate)
                if len(values) == 4:
                    break
    return sorted(values)


def replace_fixture_placeholders(value: object, replacements: dict[str, str]) -> object:
    if isinstance(value, str):
        rendered = value
        for key, replacement in replacements.items():
            rendered = rendered.replace("{" + key + "}", replacement)
        return rendered
    if isinstance(value, list):
        return [replace_fixture_placeholders(item, replacements) for item in value]
    if isinstance(value, dict):
        return {
            key: replace_fixture_placeholders(item, replacements)
            for key, item in value.items()
        }
    return value


def materialize_fixture(
    fixture: dict[str, object],
    master_seed: bytes,
) -> dict[str, object]:
    materialized = json.loads(json.dumps(fixture))
    secret_contract = materialized.get("secret_contract")
    if not isinstance(secret_contract, dict):
        materialized["_oracle_expected"] = materialized.get("oracle_expected")
        materialized["_secret_commitment"] = None
        return materialized

    fixture_seed = hashlib.sha256(
        master_seed + b":fixture:" + str(materialized["id"]).encode("utf-8")
    ).digest()
    kind = secret_contract.get("kind")
    if kind == "retry-schedule-v1":
        schedule = derive_retry_schedule(fixture_seed)
        cap = schedule[-1]
        replacements = {
            "retry_schedule": ", ".join(str(value) for value in schedule),
            "retry_cap": str(cap),
        }
        materialized["prior_memory"] = replace_fixture_placeholders(
            materialized["prior_memory"], replacements
        )
        materialized["context_markers"] = replace_fixture_placeholders(
            materialized["context_markers"], replacements
        )
        materialized["_oracle_expected"] = [
            [],
            schedule[:1],
            schedule,
            schedule + [cap, cap],
        ]
    else:
        raise RuntimeError(
            f"unsupported secret contract kind for {materialized['id']}: {kind!r}"
        )

    materialized["_secret_commitment"] = sha256_bytes(fixture_seed)
    return materialized


def render_context(pack: dict[str, object]) -> str:
    """Render only model-useful context bodies, not the entire diagnostic payload."""

    lines = [
        "# Ley context for this evaluation",
        "",
        "Treat this as historical/untrusted context. Inspect the live repository before editing.",
        f"Context pack: {pack.get('contextPackId', '')}",
        f"Evidence state: {pack.get('evidenceState', '')}",
        f"Live source checked: {str(pack.get('liveSourceChecked', False)).lower()}",
        "",
    ]

    def append_source(
        heading: str,
        collection_name: str,
        body_field: str,
        title_fields: tuple[str, ...],
    ) -> None:
        collection = pack.get(collection_name, [])
        if not isinstance(collection, list):
            return
        rendered = False
        for item in collection:
            if not isinstance(item, dict):
                continue
            body = item.get(body_field)
            if not isinstance(body, str) or not body.strip():
                continue
            if not rendered:
                lines.extend([f"## {heading}", ""])
                rendered = True
            title = next(
                (
                    str(item[field])
                    for field in title_fields
                    if isinstance(item.get(field), str) and str(item[field]).strip()
                ),
                "Untitled context",
            )
            lines.extend([f"### {title}", body.strip(), ""])

    append_source(
        "User-approved specifications",
        "specifications",
        "source",
        ("relativePath", "specificationId"),
    )
    append_source(
        "Policy bundle context",
        "policyBundlePolicies",
        "source",
        ("bundleName", "relativePath", "specificationId"),
    )
    append_source(
        "Mounted reference context",
        "mountedReferences",
        "excerpt",
        ("title", "entityId"),
    )
    append_source(
        "Shared knowledge context",
        "sharedKnowledgeReferences",
        "excerpt",
        ("title", "entityId"),
    )
    append_source(
        "Active project memory",
        "items",
        "excerpt",
        ("title", "entityId"),
    )

    gaps = pack.get("gaps", [])
    if isinstance(gaps, list):
        messages = [
            str(item["message"])
            for item in gaps
            if isinstance(item, dict) and isinstance(item.get("message"), str)
        ]
        if messages:
            lines.extend(["## Ley gaps / cautions", ""])
            lines.extend(f"- {message}" for message in messages)
            lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def build_agent_prompt(task: str, context: str | None) -> str:
    parts = [
        "You are running one isolated coding-task evaluation.",
        "Work only inside the current repository. Do not read parent directories.",
        "Do not invoke Ley, network services, external memory, or hidden evaluation files.",
        "Use only the supplied prompt/context and files inside this repository.",
        "Do not ask clarifying questions; make the best evidence-grounded change you can.",
        "You may inspect files and run local commands/tests.",
        "",
        "Task:",
        task.strip(),
    ]
    if context:
        parts.extend(["", context.strip()])
    parts.extend(
        [
            "",
            "Finish by leaving the repository in the state you believe satisfies the task.",
        ]
    )
    return "\n".join(parts) + "\n"


def run_external_agent(
    command: list[str],
    project: Path,
    prompt: str,
    timeout_seconds: int,
    variant: str,
    inherited_env_names: list[str],
    read_only_mounts: list[tuple[Path, str]],
    capture_raw: bool,
) -> dict[str, object]:
    del variant
    sandbox_command, sandbox_env = runner_sandbox_command(
        command,
        project,
        inherited_env_names,
        read_only_mounts,
    )
    return run_bounded_process(
        sandbox_command,
        cwd=project,
        env=sandbox_env,
        stdin_bytes=prompt.encode("utf-8"),
        timeout_seconds=timeout_seconds,
        capture_raw=capture_raw,
    )


def snapshot_directory(root: Path) -> bytes:
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
        for child in sorted(root.iterdir(), key=lambda path: path.name):
            archive.add(child, arcname=child.name, recursive=True)
    return buffer.getvalue()


def restore_directory(root: Path, archive_bytes: bytes) -> None:
    root.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(archive_bytes), mode="r:gz") as archive:
        for member in archive.getmembers():
            path = Path(member.name)
            if path.is_absolute() or ".." in path.parts:
                raise RuntimeError("internal Ley-state archive contains an unsafe path")
        archive.extractall(root, filter="data")


def snapshot_project_tree(project: Path) -> dict[str, tuple[str, int, bytes]]:
    snapshot: dict[str, tuple[str, int, bytes]] = {}

    def walk(directory_fd: int, prefix: str) -> None:
        try:
            names = sorted(os.listdir(directory_fd))
        except OSError as error:
            raise RuntimeError("project directory changed while being snapshotted") from error
        for name in names:
            if not prefix and name == ".git":
                continue
            relative = f"{prefix}/{name}" if prefix else name
            try:
                entry_stat = os.stat(
                    name,
                    dir_fd=directory_fd,
                    follow_symlinks=False,
                )
            except OSError as error:
                raise RuntimeError("project entry changed while being snapshotted") from error
            mode = stat.S_IMODE(entry_stat.st_mode)
            if stat.S_ISLNK(entry_stat.st_mode):
                try:
                    target_text = os.readlink(name, dir_fd=directory_fd)
                except OSError as error:
                    raise RuntimeError(
                        "project symlink changed while being snapshotted"
                    ) from error
                target = target_text.encode("utf-8", errors="surrogateescape")
                snapshot[relative] = ("symlink", mode, target)
            elif stat.S_ISDIR(entry_stat.st_mode):
                flags = (
                    os.O_RDONLY
                    | os.O_DIRECTORY
                    | getattr(os, "O_CLOEXEC", 0)
                    | getattr(os, "O_NOFOLLOW", 0)
                )
                try:
                    child_fd = os.open(name, flags, dir_fd=directory_fd)
                except OSError as error:
                    raise RuntimeError(
                        "project directory changed while being snapshotted"
                    ) from error
                try:
                    opened_stat = os.fstat(child_fd)
                    if (
                        opened_stat.st_dev != entry_stat.st_dev
                        or opened_stat.st_ino != entry_stat.st_ino
                        or not stat.S_ISDIR(opened_stat.st_mode)
                    ):
                        raise RuntimeError(
                            "project directory changed while being snapshotted"
                        )
                    snapshot[relative] = (
                        "dir",
                        stat.S_IMODE(opened_stat.st_mode),
                        b"",
                    )
                    walk(child_fd, relative)
                finally:
                    os.close(child_fd)
            elif stat.S_ISREG(entry_stat.st_mode):
                flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
                if hasattr(os, "O_NOFOLLOW"):
                    flags |= os.O_NOFOLLOW
                try:
                    descriptor = os.open(name, flags, dir_fd=directory_fd)
                except OSError as error:
                    raise RuntimeError(
                        "project file changed while being snapshotted"
                    ) from error
                try:
                    opened_stat = os.fstat(descriptor)
                    if (
                        opened_stat.st_dev != entry_stat.st_dev
                        or opened_stat.st_ino != entry_stat.st_ino
                        or not stat.S_ISREG(opened_stat.st_mode)
                    ):
                        raise RuntimeError("project file changed while being snapshotted")
                    chunks: list[bytes] = []
                    while True:
                        chunk = os.read(descriptor, 1024 * 1024)
                        if not chunk:
                            break
                        chunks.append(chunk)
                finally:
                    os.close(descriptor)
                snapshot[relative] = ("file", mode, b"".join(chunks))
            else:
                snapshot[relative] = ("other", mode, b"")

    root_flags = (
        os.O_RDONLY
        | os.O_DIRECTORY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    try:
        root_fd = os.open(project, root_flags)
    except OSError as error:
        raise RuntimeError("project root cannot be snapshotted safely") from error
    try:
        walk(root_fd, "")
    finally:
        os.close(root_fd)
    return snapshot


def compare_project_snapshots(
    before: dict[str, tuple[str, int, bytes]],
    after: dict[str, tuple[str, int, bytes]],
) -> tuple[list[str], bytes]:
    changed = sorted(
        path
        for path in set(before) | set(after)
        if before.get(path) != after.get(path)
    )
    material = bytearray()
    for path in changed:
        encoded_path = path.encode("utf-8", errors="surrogateescape")
        material.extend(len(encoded_path).to_bytes(4, "big"))
        material.extend(encoded_path)
        for label, state in ((b"before", before.get(path)), (b"after", after.get(path))):
            material.extend(label)
            if state is None:
                material.extend(b"\0missing\0")
                continue
            kind, mode, payload = state
            material.extend(b"\0")
            material.extend(kind.encode("ascii"))
            material.extend(b"\0")
            material.extend(mode.to_bytes(4, "big"))
            material.extend(len(payload).to_bytes(8, "big"))
            material.extend(payload)
    return changed, bytes(material)


def project_symlink_paths(
    snapshot: dict[str, tuple[str, int, bytes]],
) -> list[str]:
    return sorted(
        path for path, state in snapshot.items() if state[0] == "symlink"
    )


def path_set_sha256(paths: list[str]) -> str:
    return sha256_bytes(
        b"\0".join(
            path.encode("utf-8", errors="surrogateescape")
            for path in sorted(paths)
        )
    )


def public_constraint_summary(
    constraints: dict[str, object],
) -> dict[str, object]:
    path_fields = (
        "unexpectedChangedFiles",
        "unchangedFileMismatches",
        "missingAllowedFiles",
        "symlinkPaths",
        "unsafeSpecialPaths",
    )
    summary: dict[str, object] = {
        key: value
        for key, value in constraints.items()
        if key not in path_fields
    }
    for field in path_fields:
        values = constraints.get(field, [])
        paths = [str(value) for value in values] if isinstance(values, list) else []
        prefix = field[:-1] if field.endswith("s") else field
        summary[prefix + "Count"] = len(paths)
        summary[prefix + "Sha256"] = path_set_sha256(paths)
    return summary


def resolve_sandbox_executable(command: list[str]) -> list[str]:
    if not command:
        raise RuntimeError("sandbox command cannot be empty")
    resolved = shutil.which(command[0], path="/usr/bin:/bin")
    if resolved is None:
        raise RuntimeError("sandbox command executable is unavailable under /usr/bin:/bin")
    real = Path(resolved).resolve()
    try:
        real.relative_to("/usr")
    except ValueError as error:
        raise RuntimeError("sandbox command executable must resolve beneath /usr") from error
    return [resolved, *command[1:]]


def run_sandboxed_project_command(
    command: list[str],
    project: Path,
    timeout_seconds: int,
    capture_raw: bool,
) -> dict[str, object]:
    bwrap = shutil.which("bwrap")
    if bwrap is None:
        raise RuntimeError("bwrap is required for evaluator-controlled project execution")
    sandbox_command = [
        bwrap,
        "--die-with-parent",
        "--unshare-all",
        "--unshare-net",
        "--clearenv",
        "--ro-bind",
        "/usr",
        "/usr",
        "--symlink",
        "usr/bin",
        "/bin",
        "--symlink",
        "usr/lib",
        "/lib",
        "--symlink",
        "usr/lib",
        "/lib64",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--tmpfs",
        "/tmp",
        "--ro-bind",
        str(project),
        "/workspace",
        "--chdir",
        "/workspace",
        "--setenv",
        "PATH",
        "/usr/bin:/bin",
        "--setenv",
        "HOME",
        "/nonexistent",
        "--setenv",
        "PWD",
        "/workspace",
        "--setenv",
        "PYTHONDONTWRITEBYTECODE",
        "1",
        *resolve_sandbox_executable(command),
    ]
    return run_bounded_process(
        sandbox_command,
        cwd=project,
        env={"PATH": os.environ.get("PATH", os.defpath)},
        stdin_bytes=None,
        timeout_seconds=timeout_seconds,
        capture_raw=capture_raw,
    )


def run_oracle_probe(
    fixture: dict[str, object],
    project: Path,
    timeout_seconds: int,
    capture_raw: bool,
) -> dict[str, object]:
    probe = fixture.get("oracle_probe")
    expected = fixture.get("_oracle_expected")
    if not isinstance(probe, dict) or not isinstance(expected, list):
        raise RuntimeError("materialized fixture is missing oracle probe state")
    probe_code = (
        "import contextlib, importlib.util, io, json, pathlib, sys\n"
        "module_path = pathlib.Path('/workspace') / sys.argv[1]\n"
        "spec = importlib.util.spec_from_file_location('ley_eval_target', module_path)\n"
        "module = importlib.util.module_from_spec(spec)\n"
        "assert spec.loader is not None\n"
        "with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):\n"
        "    spec.loader.exec_module(module)\n"
        "    function = getattr(module, sys.argv[2])\n"
        "    values = json.loads(sys.argv[3])\n"
        "    results = [function(value) for value in values]\n"
        "print(json.dumps(results, sort_keys=True, separators=(',', ':')))\n"
    )
    command = [
        "python3",
        "-c",
        probe_code,
        str(probe["module"]),
        str(probe["function"]),
        json.dumps(probe["inputs"], sort_keys=True, separators=(",", ":")),
    ]
    sandbox = run_sandboxed_project_command(
        command,
        project,
        timeout_seconds,
        capture_raw=True,
    )
    stdout_raw = bytes(sandbox.pop("_stdout", b""))
    stderr_raw = bytes(sandbox.pop("_stderr", b""))
    actual: object = None
    parsed = False
    if sandbox["completed"]:
        try:
            actual = json.loads(stdout_raw.decode("utf-8"))
            parsed = True
        except (UnicodeDecodeError, json.JSONDecodeError):
            parsed = False
    passed = bool(sandbox["completed"]) and parsed and actual == expected
    summary: dict[str, object] = {
        "attempted": True,
        "status": "passed" if passed else "failed",
        "passed": passed,
        "exitCode": sandbox["exitCode"],
        "timedOut": sandbox["timedOut"],
        "stdoutSha256": sandbox["stdoutSha256"],
        "stderrSha256": sandbox["stderrSha256"],
    }
    if capture_raw:
        summary["_stdout"] = stdout_raw.decode("utf-8", errors="replace")
        summary["_stderr"] = stderr_raw.decode("utf-8", errors="replace")
    return summary


def evaluate_task_constraints(
    fixture: dict[str, object],
    project: Path,
    before_snapshot: dict[str, tuple[str, int, bytes]],
    after_snapshot: dict[str, tuple[str, int, bytes]],
    changed_files: list[str],
    timeout_seconds: int,
) -> dict[str, object]:
    allowed = {str(value) for value in fixture["allowed_changed_files"]}
    unexpected_changes = sorted(set(changed_files) - allowed)
    symlinks = project_symlink_paths(after_snapshot)
    unsafe_special_paths = sorted(
        path
        for path, state in after_snapshot.items()
        if state[0] not in {"file", "dir", "symlink"}
    )
    unchanged_mismatches = sorted(
        path
        for path in changed_files
        if path in before_snapshot and path not in allowed
    )
    missing_allowed = sorted(
        relative
        for relative in allowed
        if after_snapshot.get(relative, ("missing", 0, b""))[0] != "file"
    )
    structural_passed = (
        not unexpected_changes
        and not unchanged_mismatches
        and not missing_allowed
        and not symlinks
        and not unsafe_special_paths
    )
    if structural_passed:
        visible = run_sandboxed_project_command(
            [str(value) for value in fixture["visible_test_command"]],
            project,
            timeout_seconds,
            capture_raw=False,
        )
        visible_tests_passed = bool(visible["completed"])
        visible_skipped = False
    else:
        visible = {
            "timedOut": False,
            "exitCode": None,
            "stdoutSha256": sha256_bytes(b""),
            "stderrSha256": sha256_bytes(b""),
        }
        visible_tests_passed = False
        visible_skipped = True
    passed = (
        structural_passed
        and visible_tests_passed
    )
    return {
        "passed": passed,
        "unexpectedChangedFiles": unexpected_changes,
        "unchangedFileMismatches": unchanged_mismatches,
        "missingAllowedFiles": missing_allowed,
        "symlinkPaths": symlinks,
        "unsafeSpecialPaths": unsafe_special_paths,
        "visibleTestsPassed": visible_tests_passed,
        "visibleTestsSkipped": visible_skipped,
        "visibleTestsTimedOut": visible["timedOut"],
        "visibleTestsExitCode": visible["exitCode"],
        "visibleTestsStdoutSha256": visible["stdoutSha256"],
        "visibleTestsStderrSha256": visible["stderrSha256"],
    }


def validate_fixture_does_not_leak_context(
    fixture: dict[str, object],
) -> None:
    project_files = fixture.get("project_files", {})
    markers = fixture.get("context_markers", [])
    if not isinstance(project_files, dict) or not isinstance(markers, list):
        raise RuntimeError("agent task fixture has invalid project_files/context_markers")
    task = str(fixture.get("task", ""))
    if not task or len(task) > 256:
        raise RuntimeError("agent task fixture task must contain 1 to 256 characters")
    visible = (
        task
        + "\n"
        + "\n".join(str(path) for path in project_files)
        + "\n"
        + "\n".join(str(body) for body in project_files.values())
        + "\n"
        + "\n".join(str(value) for value in fixture.get("allowed_changed_files", []))
    ).lower()
    leaked = [str(marker) for marker in markers if str(marker).lower() in visible]
    if leaked:
        raise RuntimeError(
            f"agent task fixture exposes {len(leaked)} historical context marker(s) "
            "through the task/live project surface"
        )


def validate_compiled_fixture(
    fixture: dict[str, object],
    root: Path,
    max_results: int,
    max_tokens: int,
) -> dict[str, object]:
    validate_fixture_does_not_leak_context(fixture)
    project = root / "project"
    vault = root / "vault"
    config = root / "config"
    project.mkdir(parents=True)
    vault.mkdir(parents=True)
    previous_config = EVAL_ENV.get("XDG_CONFIG_HOME")
    EVAL_ENV["XDG_CONFIG_HOME"] = str(config)
    files = fixture.get("project_files")
    if not isinstance(files, dict):
        raise RuntimeError("agent task fixture requires project_files")
    write_project_files(project, {str(path): str(body) for path, body in files.items()})
    git_run(project, ["init", "-b", "main"])
    git_commit_all(project, "agent-eval fixture")
    try:
        init_project(project, "Agent downstream eval validation", vault)
        seed_prior_memory(project, fixture)
        rendered, utility = prepare_ley_context(
            project,
            fixture,
            max_results=max_results,
            max_tokens=max_tokens,
        )
        return {
            "taskId": fixture["id"],
            "fixtureSecretCommitment": fixture.get("_secret_commitment"),
            "contextPackId": utility["contextPackId"],
            "contextSha256": utility["contextSha256"],
            "contextCharacters": len(rendered),
            "estimatedTokens": utility["estimatedTokens"],
            "evidenceState": utility["evidenceState"],
        }
    finally:
        if previous_config is None:
            EVAL_ENV.pop("XDG_CONFIG_HOME", None)
        else:
            EVAL_ENV["XDG_CONFIG_HOME"] = previous_config


def seed_prior_memory(
    project: Path,
    fixture: dict[str, object],
) -> None:
    prior = fixture.get("prior_memory")
    if not isinstance(prior, dict):
        raise RuntimeError("agent task fixture requires prior_memory")
    session_id, _ = create_structured_session(
        project,
        seed=f"{fixture['id']}:prior",
        name=str(prior["name"]),
        goal=str(prior["goal"]),
        summary=str(prior["summary"]),
        decisions=[
            {
                "title": str(prior["decision_title"]),
                "decision": str(prior["decision"]),
                "rationale": str(prior.get("rationale", "")),
            }
        ],
    )
    mcp_call(
        project,
        "ley_session_finish",
        {
            "sessionId": session_id,
            "requestId": request_id(f"{fixture['id']}:prior:finish"),
            "status": "completed",
            "summary": str(prior["summary"]),
            "finalResponse": "The prior contract was recorded and verified for handoff.",
            "handoff": "Use the recorded prior contract when this topic is revisited.",
            "unresolved": [],
        },
        WRITE_FLAGS,
    )


def prepare_ley_context(
    project: Path,
    fixture: dict[str, object],
    max_results: int,
    max_tokens: int,
) -> tuple[str, dict[str, object]]:
    task = str(fixture["task"])
    started = mcp_call(
        project,
        "ley_session_start",
        {
            "requestId": request_id(f"{fixture['id']}:eval:start"),
            "name": "Real-agent downstream evaluation",
            "goal": "Record an external fixture outcome against one bound context pack.",
            "host": "codex",
        },
        WRITE_FLAGS,
    )
    session_id = str(started["sessionId"])
    compiled = mcp_call(
        project,
        "ley_compile_context",
        {"task": task, "maxResults": max_results, "maxTokens": max_tokens},
    )
    context_pack_id = str(compiled.get("contextPackId", ""))
    if not context_pack_id.startswith("cpk_"):
        raise RuntimeError("Ley context compilation returned no stable contextPackId")
    rendered = render_context(compiled)
    markers = [str(value) for value in fixture.get("context_markers", [])]
    missing = [marker for marker in markers if marker.lower() not in rendered.lower()]
    if missing:
        raise RuntimeError(
            f"Ley context omitted {len(missing)} required benchmark evidence marker(s)"
        )
    bound = mcp_call(
        project,
        "ley_context_utility_bind",
        {
            "sessionId": session_id,
            "requestId": request_id(f"{fixture['id']}:eval:bind"),
            "expectedEventCount": 1,
            "contextPackId": context_pack_id,
            "task": task,
            "maxResults": max_results,
            "maxTokens": max_tokens,
        },
        WRITE_FLAGS,
    )
    binding_id = str(bound.get("bindingId", ""))
    if (
        not binding_id.startswith("cub_")
        or bound.get("contextPackId") != context_pack_id
        or bound.get("eventCount") != 2
        or bound.get("replayed") is not False
    ):
        raise RuntimeError("Ley context utility bind receipt did not match the compiled pack")
    return rendered, {
        "sessionId": session_id,
        "bindingId": binding_id,
        "contextPackId": context_pack_id,
        "contextSha256": sha256_text(rendered),
        "contextCharacters": len(rendered),
        "estimatedTokens": compiled.get("estimatedTokens"),
        "evidenceState": compiled.get("evidenceState"),
    }


def evaluation_outcome_fields(
    task_passed: bool,
    hidden_oracle_status: str,
) -> dict[str, object]:
    if hidden_oracle_status not in {"passed", "failed", "skipped"}:
        raise RuntimeError("invalid hidden oracle status")
    oracle_summary = {
        "passed": "The hidden oracle passed.",
        "failed": "The hidden oracle failed.",
        "skipped": "The hidden oracle was skipped because an earlier evaluator gate did not pass.",
    }[hidden_oracle_status]
    return {
        "taskStatus": "completed" if task_passed else "blocked",
        "sessionStatus": "completed" if task_passed else "paused",
        "checkpointSummary": "Recorded the external real-agent evaluation outcome.",
        "taskDetails": (
            "Overall outcome is determined by evaluator gates, not model self-report."
        ),
        "verifications": [
            {
                "kind": "agent-task-eval",
                "status": "passed" if task_passed else "failed",
                "summary": (
                    "The external agent fixture passed all evaluator gates."
                    if task_passed
                    else "The external agent fixture did not pass all evaluator gates."
                ),
            },
            {
                "kind": "hidden-oracle",
                "status": hidden_oracle_status,
                "summary": oracle_summary,
            },
        ],
        "sessionSummary": (
            "Real-agent fixture passed all evaluator gates."
            if task_passed
            else "Real-agent fixture did not pass all evaluator gates."
        ),
        "finalResponse": (
            "Evaluation outcome recorded from the complete evaluator gate set."
        ),
        "unresolved": [] if task_passed else ["Downstream fixture remains unsatisfied."],
    }


def record_ley_outcome(
    project: Path,
    fixture: dict[str, object],
    utility: dict[str, object],
    task_passed: bool,
    hidden_oracle_status: str,
) -> dict[str, object]:
    session_id = str(utility["sessionId"])
    fields = evaluation_outcome_fields(task_passed, hidden_oracle_status)
    expected_session_status = str(fields["sessionStatus"])
    checkpoint = mcp_call(
        project,
        "ley_session_checkpoint",
        {
            "sessionId": session_id,
            "requestId": request_id(f"{fixture['id']}:eval:checkpoint"),
            "expectedEventCount": 2,
            "summary": fields["checkpointSummary"],
            "tasks": [
                {
                    "title": "Complete downstream real-agent fixture",
                    "status": fields["taskStatus"],
                    "details": fields["taskDetails"],
                }
            ],
            "verification": fields["verifications"],
        },
        WRITE_FLAGS,
    )
    checkpoint_event_id = str(checkpoint["eventId"])
    finished = mcp_call(
        project,
        "ley_session_finish",
        {
            "sessionId": session_id,
            "requestId": request_id(f"{fixture['id']}:eval:finish"),
            "status": expected_session_status,
            "summary": fields["sessionSummary"],
            "finalResponse": fields["finalResponse"],
            "handoff": "",
            "unresolved": fields["unresolved"],
        },
        WRITE_FLAGS,
    )
    observed = mcp_call(
        project,
        "ley_context_utility_observe",
        {
            "sessionId": session_id,
            "requestId": request_id(f"{fixture['id']}:eval:observe"),
            "expectedEventCount": 4,
            "bindingId": str(utility["bindingId"]),
            "downstreamEventIds": [checkpoint_event_id, str(finished["eventId"])],
        },
        WRITE_FLAGS,
    )
    if (
        observed.get("eventCount") != 5
        or observed.get("replayed") is not False
        or observed.get("status") != expected_session_status
        or not str(observed.get("eventId", "")).startswith("evt_")
    ):
        raise RuntimeError("Ley context utility observation receipt did not match the expected event")
    session = mcp_call(
        project,
        "ley_session_get",
        {"sessionId": session_id, "maxCheckpoints": 3, "maxCharacters": 8_000},
    )
    rows = session.get("contextUtilityObservations", [])
    row = rows[0] if isinstance(rows, list) and len(rows) == 1 and isinstance(rows[0], dict) else {}
    expected_downstream_ids = sorted([checkpoint_event_id, str(finished["eventId"])])
    actual_downstream_ids = sorted(
        str(value) for value in row.get("downstreamEventIds", [])
    ) if isinstance(row.get("downstreamEventIds"), list) else []
    if (
        session.get("contextUtilityBindingCount") != 1
        or session.get("contextUtilityObservationCount") != 1
        or session.get("omittedContextUtilityObservations") != 0
        or row.get("bindingId") != utility["bindingId"]
        or row.get("contextPackId") != utility["contextPackId"]
        or row.get("contextPackRevalidated") is not True
        or actual_downstream_ids != expected_downstream_ids
        or row.get("contextUsageProven") is not False
        or row.get("causalUtilityProven") is not False
        or row.get("trustChangesApplied") is not False
        or row.get("rankingChangesApplied") is not False
    ):
        raise RuntimeError("utility observation violated the non-causal authority boundary")
    return {
        "observationEventId": observed.get("eventId"),
        "contextUsageProven": row["contextUsageProven"],
        "causalUtilityProven": row["causalUtilityProven"],
        "trustChangesApplied": row["trustChangesApplied"],
        "rankingChangesApplied": row["rankingChangesApplied"],
    }


def variants_for_repetition(
    selected_variant: str,
    first_variant: str,
    repetition: int,
) -> tuple[str, ...]:
    if selected_variant != "both":
        return (selected_variant,)
    first = (
        first_variant
        if repetition % 2 == 1
        else ("ley" if first_variant == "baseline" else "baseline")
    )
    return (first, "ley" if first == "baseline" else "baseline")


def summarize_hidden_oracles(
    results: list[dict[str, object]],
) -> dict[str, object]:
    statuses = [str(item.get("hiddenOracleStatus", "skipped")) for item in results]
    passed = sum(status == "passed" for status in statuses)
    failed = sum(status == "failed" for status in statuses)
    skipped = sum(status == "skipped" for status in statuses)
    attempted = passed + failed
    return {
        "attemptedCount": attempted,
        "passedCount": passed,
        "failedCount": failed,
        "skippedCount": skipped,
        "passRateAmongAttempted": (
            passed / attempted if attempted else None
        ),
    }


def task_attempt_passed(
    runner: dict[str, object],
    constraints: dict[str, object],
    oracle: dict[str, object],
    post_check_tree_stable: bool,
) -> bool:
    return (
        bool(runner.get("completed"))
        and bool(constraints.get("passed"))
        and bool(oracle.get("passed"))
        and post_check_tree_stable
    )


def execute_variant(
    fixture: dict[str, object],
    root: Path,
    command: list[str],
    runner_env_names: list[str],
    runner_read_only_mounts: list[tuple[Path, str]],
    variant: str,
    timeout_seconds: int,
    max_results: int,
    max_tokens: int,
    capture_audit: bool,
) -> dict[str, object]:
    project = root / "project"
    project.mkdir(parents=True)
    files = fixture.get("project_files")
    if not isinstance(files, dict):
        raise RuntimeError("agent task fixture requires project_files")
    write_project_files(project, {str(path): str(body) for path, body in files.items()})
    git_run(project, ["init", "-b", "main"])
    git_commit_all(project, "agent-eval fixture")
    initial_snapshot = snapshot_project_tree(project)

    context: str | None = None
    utility: dict[str, object] | None = None
    memory_project: Path | None = None
    memory_root: Path | None = None
    memory_snapshot: bytes | None = None
    previous_config = EVAL_ENV.get("XDG_CONFIG_HOME")
    try:
        if variant == "ley":
            memory_root = Path(tempfile.mkdtemp(prefix="ley-real-agent-memory-"))
            memory_project = memory_root / "project"
            shutil.copytree(project, memory_project)
            vault = memory_root / "vault"
            vault.mkdir()
            EVAL_ENV["XDG_CONFIG_HOME"] = str(memory_root / "config")
            init_project(memory_project, "Agent downstream eval", vault)
            seed_prior_memory(memory_project, fixture)
            context, utility = prepare_ley_context(
                memory_project,
                fixture,
                max_results=max_results,
                max_tokens=max_tokens,
            )
            memory_snapshot = snapshot_directory(memory_root)
            shutil.rmtree(memory_root)

        prompt = build_agent_prompt(str(fixture["task"]), context)
        runner = run_external_agent(
            command,
            project,
            prompt,
            timeout_seconds,
            variant,
            runner_env_names,
            runner_read_only_mounts,
            capture_audit,
        )
        runner_stdout = runner.pop("_stdout", b"")
        runner_stderr = runner.pop("_stderr", b"")
        after_snapshot = snapshot_project_tree(project)
        changed_files, diff_material = compare_project_snapshots(
            initial_snapshot,
            after_snapshot,
        )
        constraints = evaluate_task_constraints(
            fixture,
            project,
            initial_snapshot,
            after_snapshot,
            changed_files,
            timeout_seconds,
        )
        oracle = (
            run_oracle_probe(
                fixture,
                project,
                timeout_seconds,
                capture_audit,
            )
            if constraints["passed"]
            else {
                "attempted": False,
                "status": "skipped",
                "passed": None,
                "exitCode": None,
                "timedOut": False,
                "stdoutSha256": sha256_bytes(b""),
                "stderrSha256": sha256_bytes(b""),
            }
        )
        oracle_stdout = oracle.pop("_stdout", "")
        oracle_stderr = oracle.pop("_stderr", "")
        final_snapshot = snapshot_project_tree(project)
        post_check_tree_stable = final_snapshot == after_snapshot
        if not post_check_tree_stable:
            changed_files, diff_material = compare_project_snapshots(
                initial_snapshot,
                final_snapshot,
            )
        project_snapshot = (
            snapshot_directory(project)
            if capture_audit and not project_symlink_paths(final_snapshot)
            else b""
        )
        task_passed = task_attempt_passed(
            runner,
            constraints,
            oracle,
            post_check_tree_stable,
        )
        utility_result = None
        if (
            memory_project is not None
            and memory_root is not None
            and memory_snapshot is not None
            and utility is not None
        ):
            restore_directory(memory_root, memory_snapshot)
            utility_result = record_ley_outcome(
                memory_project,
                fixture,
                utility,
                task_passed,
                str(oracle["status"]),
            )
        result: dict[str, object] = {
            "variant": variant,
            "taskPassed": task_passed,
            "hiddenOracleAttempted": oracle["attempted"],
            "hiddenOracleStatus": oracle["status"],
            "hiddenOraclePassed": oracle["passed"],
            "hiddenOracleExitCode": oracle["exitCode"],
            "postCheckTreeStable": post_check_tree_stable,
            "runner": runner,
            "constraints": public_constraint_summary(constraints),
            "changedFileCount": len(changed_files),
            "changedPathsSha256": path_set_sha256(changed_files),
            "diffBytes": len(diff_material),
            "diffSha256": sha256_bytes(diff_material),
            "promptSha256": sha256_text(prompt),
            "promptCharacters": len(prompt),
            "context": (
                {
                    key: value
                    for key, value in utility.items()
                    if key not in {"sessionId", "bindingId"}
                }
                if utility is not None
                else None
            ),
            "utilityObservation": utility_result,
        }
        if capture_audit:
            result["_audit"] = {
                "prompt": prompt,
                "context": context or "",
                "runnerStdout": runner_stdout,
                "runnerStderr": runner_stderr,
                "oracleStdout": oracle_stdout,
                "oracleStderr": oracle_stderr,
                "diffMaterial": diff_material,
                "projectSnapshot": project_snapshot,
                "changedFiles": changed_files,
                "constraintDetails": constraints,
            }
        return result
    finally:
        if previous_config is None:
            EVAL_ENV.pop("XDG_CONFIG_HOME", None)
        else:
            EVAL_ENV["XDG_CONFIG_HOME"] = previous_config
        if memory_root is not None:
            shutil.rmtree(memory_root, ignore_errors=True)


def write_audit_bundle(
    destination: Path,
    fixture: dict[str, object],
    command: list[str],
    report: dict[str, object],
    audit_records: list[dict[str, object]],
) -> None:
    try:
        destination.mkdir(parents=True, mode=0o700)
    except FileExistsError as error:
        raise RuntimeError(f"audit directory already exists: {destination}") from error
    try:
        (destination / "runner-command.json").write_text(
            json.dumps(command, indent=2) + "\n",
            encoding="utf-8",
        )
        (destination / "materialized-fixture.json").write_text(
            json.dumps(fixture, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        (destination / "report.json").write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        for record in audit_records:
            repetition = int(record["repetition"])
            variant = str(record["variant"])
            run_dir = destination / f"repetition-{repetition:03d}-{variant}"
            run_dir.mkdir(mode=0o700)
            payload = record["payload"]
            if not isinstance(payload, dict):
                raise RuntimeError("internal audit payload is not an object")
            (run_dir / "prompt.txt").write_text(
                str(payload["prompt"]),
                encoding="utf-8",
            )
            context = str(payload["context"])
            if context:
                (run_dir / "context.txt").write_text(context, encoding="utf-8")
            (run_dir / "runner.stdout").write_bytes(bytes(payload["runnerStdout"]))
            (run_dir / "runner.stderr").write_bytes(bytes(payload["runnerStderr"]))
            (run_dir / "oracle.stdout").write_text(
                str(payload["oracleStdout"]),
                encoding="utf-8",
            )
            (run_dir / "oracle.stderr").write_text(
                str(payload["oracleStderr"]),
                encoding="utf-8",
            )
            (run_dir / "diff-material.bin").write_bytes(bytes(payload["diffMaterial"]))
            (run_dir / "project-after.tar.gz").write_bytes(
                bytes(payload["projectSnapshot"])
            )
            (run_dir / "changed-files.json").write_text(
                json.dumps(payload["changedFiles"], indent=2) + "\n",
                encoding="utf-8",
            )
            (run_dir / "constraint-details.json").write_text(
                json.dumps(payload["constraintDetails"], indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
    except Exception:
        shutil.rmtree(destination, ignore_errors=True)
        raise


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run opt-in real-agent Ley comparisons. The runner command must read the task prompt "
            "from stdin and operate in its current working directory."
        )
    )
    parser.add_argument("--list", action="store_true", help="List available synthetic agent tasks")
    parser.add_argument(
        "--validate",
        action="store_true",
        help="Validate fixture isolation and Ley context retrieval without invoking an external agent",
    )
    parser.add_argument("--task", help="Exact agent task fixture id")
    parser.add_argument(
        "--runner-command",
        help=(
            "External agent command, parsed with shell-like quoting but executed without a shell. "
            "Example: 'codex exec --ignore-user-config --ephemeral -s workspace-write -a never -'"
        ),
    )
    parser.add_argument(
        "--runner-label",
        default="",
        help="Optional non-secret label recorded in the report, e.g. codex-luna-xhigh",
    )
    parser.add_argument(
        "--runner-env",
        action="append",
        default=[],
        metavar="NAME",
        help=(
            "Explicitly inherit one additional environment variable into the external runner. "
            "Repeat as needed; values are never written to the report."
        ),
    )
    parser.add_argument(
        "--runner-ro-bind",
        action="append",
        default=[],
        metavar="SOURCE=DEST",
        help=(
            "Expose one explicit host file/directory read-only inside the runner sandbox. "
            "DEST must live beneath /home/runner/. Repeat as needed."
        ),
    )
    parser.add_argument(
        "--variant",
        choices=("both", "baseline", "ley"),
        default="both",
        help="Run both comparison arms or only one arm",
    )
    parser.add_argument("--repetitions", type=int, default=1)
    parser.add_argument(
        "--first-variant",
        choices=("baseline", "ley"),
        default="baseline",
        help="First arm for repetition 1; later repetitions alternate arm order",
    )
    parser.add_argument("--timeout-seconds", type=int, default=600)
    parser.add_argument("--max-results", type=int, default=DEFAULT_MAX_RESULTS)
    parser.add_argument("--max-tokens", type=int, default=DEFAULT_MAX_TOKENS)
    parser.add_argument(
        "--fixture-seed-file",
        type=Path,
        help=(
            "Optional 32-byte hexadecimal seed file for exact fixture reproduction. "
            "For blinded runs, ensure the external runner cannot read this path."
        ),
    )
    parser.add_argument(
        "--write-fixture-seed",
        type=Path,
        help=(
            "Write the run's 32-byte hexadecimal fixture seed after all agent arms finish. "
            "The destination must not already exist."
        ),
    )
    parser.add_argument(
        "--audit-dir",
        type=Path,
        help=(
            "Optional post-run audit bundle directory. It must not already exist and will contain "
            "raw prompts/context, runner/oracle streams, the exact materialized fixture, runner "
            "command, diff material, and post-agent project snapshots. Do not use this for secrets "
            "you are unwilling to persist."
        ),
    )
    parser.add_argument("--output", type=Path, help="Optional JSON report path")
    parser.add_argument(
        "--require-ley-advantage",
        action="store_true",
        help=(
            "Exit non-zero unless Ley's overall task pass rate is strictly above baseline"
        ),
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    raw_fixtures = load_fixtures()
    if args.list:
        for fixture in raw_fixtures:
            print(f"{fixture['id']}: {fixture['task']}")
        return 0
    master_seed = load_master_seed(args.fixture_seed_file)
    fixtures = [
        materialize_fixture(fixture, master_seed)
        for fixture in raw_fixtures
    ]
    if args.validate:
        selected = (
            fixtures
            if not args.task
            else [item for item in fixtures if item["id"] == args.task]
        )
        if not selected:
            available = ", ".join(str(item["id"]) for item in fixtures)
            raise SystemExit(f"unknown agent task {args.task!r}; available: {available}")
        summaries: list[dict[str, object]] = []
        with tempfile.TemporaryDirectory(prefix="ley-real-agent-validate-") as temporary:
            temp = Path(temporary)
            EVAL_ENV["XDG_CONFIG_HOME"] = str(temp / "config")
            for index, fixture in enumerate(selected):
                summaries.append(
                    validate_compiled_fixture(
                        fixture,
                        temp / f"fixture-{index}",
                        args.max_results,
                        args.max_tokens,
                    )
                )
        print(json.dumps({"validated": summaries}, indent=2, sort_keys=True))
        if args.write_fixture_seed:
            write_master_seed(args.write_fixture_seed, master_seed)
        return 0
    if not args.task:
        raise SystemExit("--task is required unless --list is used")
    fixture = next((item for item in fixtures if item["id"] == args.task), None)
    if fixture is None:
        available = ", ".join(str(item["id"]) for item in fixtures)
        raise SystemExit(f"unknown agent task {args.task!r}; available: {available}")
    if not args.runner_command:
        raise SystemExit("--runner-command is required for a real-agent run")
    if args.repetitions < 1 or args.repetitions > MAX_REPETITIONS:
        raise SystemExit(f"--repetitions must be between 1 and {MAX_REPETITIONS}")
    if args.timeout_seconds < 1:
        raise SystemExit("--timeout-seconds must be positive")
    if args.max_results < 1 or args.max_tokens < 1:
        raise SystemExit("--max-results and --max-tokens must be positive")
    if len(args.runner_label) > 128:
        raise SystemExit("--runner-label must be at most 128 characters")
    if args.audit_dir and args.audit_dir.exists():
        raise SystemExit(f"--audit-dir already exists: {args.audit_dir}")
    for name in args.runner_env:
        if (
            not name
            or not (name[0].isalpha() or name[0] == "_")
            or not all(character.isalnum() or character == "_" for character in name)
            or name in {"HOME", "PATH", "PWD", "OLDPWD", "USER", "LOGNAME"}
        ):
            raise SystemExit(f"invalid --runner-env name: {name!r}")
    if args.require_ley_advantage and args.variant != "both":
        raise SystemExit("--require-ley-advantage requires --variant both")
    command = shlex.split(args.runner_command)
    if not command:
        raise SystemExit("--runner-command parsed to an empty command")
    try:
        runner_read_only_mounts = [
            parse_runner_read_only_mount(spec)
            for spec in args.runner_ro_bind
        ]
    except RuntimeError as error:
        raise SystemExit(str(error)) from error
    validate_fixture_does_not_leak_context(fixture)

    results: list[dict[str, object]] = []
    audit_records: list[dict[str, object]] = []
    with tempfile.TemporaryDirectory(prefix="ley-real-agent-config-") as temporary:
        temp = Path(temporary)
        EVAL_ENV["XDG_CONFIG_HOME"] = str(temp / "config")
        for repetition in range(1, args.repetitions + 1):
            variants = variants_for_repetition(
                args.variant,
                args.first_variant,
                repetition,
            )
            for variant in variants:
                print(
                    f"[{repetition}/{args.repetitions}] {fixture['id']} {variant}: running",
                    flush=True,
                )
                with tempfile.TemporaryDirectory(
                    prefix="ley-real-agent-workspace-"
                ) as run_temporary:
                    result = execute_variant(
                        fixture,
                        Path(run_temporary),
                        command,
                        list(args.runner_env),
                        runner_read_only_mounts,
                        variant,
                        args.timeout_seconds,
                        args.max_results,
                        args.max_tokens,
                        args.audit_dir is not None,
                    )
                audit_payload = result.pop("_audit", None)
                result["repetition"] = repetition
                if audit_payload is not None:
                    audit_records.append(
                        {
                            "repetition": repetition,
                            "variant": variant,
                            "payload": audit_payload,
                        }
                    )
                results.append(result)
                print(
                    f"  task={'PASS' if result['taskPassed'] else 'FAIL'} "
                    f"runner_seconds={result['runner']['seconds']}",
                    flush=True,
                )

    baseline = [item for item in results if item["variant"] == "baseline"]
    ley = [item for item in results if item["variant"] == "ley"]
    baseline_task_rate = (
        sum(bool(item["taskPassed"]) for item in baseline) / len(baseline)
        if baseline
        else None
    )
    ley_task_rate = (
        sum(bool(item["taskPassed"]) for item in ley) / len(ley)
        if ley
        else None
    )
    task_advantage = (
        baseline_task_rate is not None
        and ley_task_rate is not None
        and ley_task_rate > baseline_task_rate
    )
    baseline_oracles = summarize_hidden_oracles(baseline)
    ley_oracles = summarize_hidden_oracles(ley)
    report = {
        "schemaVersion": 1,
        "taskId": fixture["id"],
        "fixtureSecretCommitment": fixture.get("_secret_commitment"),
        "runner": {
            "executable": Path(command[0]).name,
            "label": args.runner_label,
            "argumentCount": max(0, len(command) - 1),
            "commandSha256": sha256_text("\0".join(command)),
            "explicitInheritedEnvironmentNames": sorted(set(args.runner_env)),
            "readOnlyMountCount": len(runner_read_only_mounts),
            "readOnlyMountDestinations": sorted(
                destination for _, destination in runner_read_only_mounts
            ),
        },
        "repetitions": args.repetitions,
        "firstVariant": args.first_variant if args.variant == "both" else None,
        "results": results,
        "comparison": {
            "baselineTaskPassRate": baseline_task_rate,
            "leyTaskPassRate": ley_task_rate,
            "leyTaskAdvantageObserved": task_advantage,
            "baselineHiddenOracle": baseline_oracles,
            "leyHiddenOracle": ley_oracles,
            "contextUsageProven": False,
            "causalUtilityProven": False,
            "interpretation": (
                "This is an opt-in external-agent observation. It is not a deterministic CI gate, "
                "does not prove causation, and must be reproduced before product claims."
            ),
        },
    }
    encoded = json.dumps(report, indent=2, sort_keys=True)
    print("\n" + encoded, flush=True)
    if args.audit_dir:
        write_audit_bundle(
            args.audit_dir,
            fixture,
            command,
            report,
            audit_records,
        )
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded + "\n", encoding="utf-8")
    if args.write_fixture_seed:
        write_master_seed(args.write_fixture_seed, master_seed)
    if args.require_ley_advantage and not task_advantage:
        return 1
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        raise SystemExit(2) from error
