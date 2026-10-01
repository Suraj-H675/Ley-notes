#!/usr/bin/env python3
"""Compare revision-safe contentful SessionStart with guidance-only startup.

This is an opt-in, model-dependent C4 evaluator. It reuses the checked-in agent-task
oracle/sandbox machinery but differs from the static agent-context evaluator in one
important way: the external model receives live access to Ley's canonical four MCP
tools while Ley's private continuity state remains outside the writable workspace.

The two arms are:

* contentful: the shipped revision-safe SessionStart context plus normal prompt guidance;
* guidance-only: session identity/retrieval/checkpoint guidance, with no prior-session or
  learning bodies automatically injected. An interrupted-current-session fixture keeps
  the same body-free recovery notice in this arm.

The guidance-only arm is benchmark-only. This file does not add a product mode or change
the shipped host adapter.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
from collections import Counter
from pathlib import Path
from typing import Any

import run_agent_task_eval as agent_eval
import run_eval as harness


REPO_ROOT = Path(__file__).resolve().parents[1]
FIXTURES = Path(__file__).parent / "fixtures" / "startup_context_tasks.jsonl"
CANONICAL_TOOLS = {"ley_brief", "ley_search", "ley_evidence", "ley_checkpoint"}
VARIANTS = ("contentful", "guidance-only")
MAX_REPETITIONS = 6


def load_fixtures() -> list[dict[str, object]]:
    base_tasks = {
        str(item["id"]): item
        for item in agent_eval.load_fixtures()
    }
    fixtures: list[dict[str, object]] = []
    seen: set[str] = set()
    for line in FIXTURES.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        if not isinstance(row, dict):
            raise RuntimeError("startup-context fixture row must be an object")
        fixture_id = str(row.get("id", ""))
        if not fixture_id or fixture_id in seen:
            raise RuntimeError("startup-context fixture IDs must be non-empty and unique")
        seen.add(fixture_id)
        source_task_id = row.get("sourceTaskId")
        if source_task_id is not None:
            source = base_tasks.get(str(source_task_id))
            if source is None:
                raise RuntimeError(
                    f"startup-context fixture {fixture_id} references unknown agent task {source_task_id}"
                )
            resolved = json.loads(json.dumps(source))
            resolved["id"] = fixture_id
            resolved["sourceTaskId"] = str(source_task_id)
            for key, value in row.items():
                if key not in {"sourceTaskId"}:
                    resolved[key] = value
        else:
            resolved = row
        if resolved.get("startupCase") not in {
            "ordinary",
            "trusted-learning",
            "interrupted-current-session",
        }:
            raise RuntimeError(
                f"startup-context fixture {fixture_id} has unsupported startupCase"
            )
        expectations = resolved.get("startupExpectations")
        if not isinstance(expectations, dict) or set(expectations) != set(VARIANTS):
            raise RuntimeError(
                f"startup-context fixture {fixture_id} requires startupExpectations for both variants"
            )
        for variant, expectation in expectations.items():
            if variant not in VARIANTS or not isinstance(expectation, dict):
                raise RuntimeError(
                    f"startup-context fixture {fixture_id} has invalid expectation for {variant}"
                )
            for field in ("requiredMarkerCoverage", "forbiddenMarkerLeakCount"):
                value = expectation.get(field)
                if not isinstance(value, (int, float)):
                    raise RuntimeError(
                        f"startup-context fixture {fixture_id} expectation {variant}.{field} must be numeric"
                    )
        agent_eval.validate_fixture_does_not_leak_context(resolved)
        fixtures.append(resolved)
    return fixtures


def stable_host_request_id(external_session_id: str) -> str:
    digest = hashlib.sha256()
    for part in ("start", "codex", external_session_id):
        digest.update(part.encode("utf-8"))
        digest.update(b"\0")
    return "req_" + digest.hexdigest()[:32]


def cli_session_id(payload: dict[str, object]) -> str:
    direct = payload.get("sessionId")
    if isinstance(direct, str) and direct.startswith("ses_"):
        return direct
    session = payload.get("session")
    if isinstance(session, dict):
        value = session.get("sessionId")
        if isinstance(value, str) and value.startswith("ses_"):
            return value
    raise RuntimeError("Ley CLI session mutation returned no stable session ID")


def cli_learning_id(payload: dict[str, object]) -> str:
    direct = payload.get("learningId")
    if isinstance(direct, str) and direct.startswith("lrn_"):
        return direct
    learning = payload.get("learning")
    if isinstance(learning, dict):
        value = learning.get("learningId")
        if isinstance(value, str) and value.startswith("lrn_"):
            return value
    raise RuntimeError("Ley CLI learning mutation returned no stable learning ID")


def init_native_project(project: Path, name: str) -> None:
    harness.run(
        [
            "init",
            str(project),
            "--name",
            name,
            "--capture",
            "structured",
            "--json",
        ]
    )
    harness.run(["ingest", str(project), "--json"])


def write_checkpoint_data(
    destination: Path,
    *,
    request_id: str,
    summary: str,
    decisions: list[dict[str, object]] | None = None,
    touched_artifacts: list[str] | None = None,
    verification: list[dict[str, object]] | None = None,
    unresolved: list[str] | None = None,
) -> None:
    payload = {
        "requestId": request_id,
        "summary": summary,
        "plan": [],
        "decisions": decisions or [],
        "tasks": [],
        "problems": [],
        "touchedArtifacts": touched_artifacts or [],
        "commands": [],
        "verification": verification or [],
        "unresolved": unresolved or [],
    }
    destination.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def start_cli_session(
    project: Path,
    *,
    request_id: str,
    name: str,
    goal: str,
    host: str = "codex",
) -> str:
    return cli_session_id(
        harness.cli_json(
            [
                "session",
                "start",
                str(project),
                "--name",
                name,
                "--goal",
                goal,
                "--host",
                host,
                "--request-id",
                request_id,
                "--json",
            ]
        )
    )


def checkpoint_cli_session(
    project: Path,
    session_id: str,
    checkpoint_path: Path,
) -> None:
    harness.cli_json(
        [
            "session",
            "checkpoint",
            session_id,
            str(project),
            "--data",
            str(checkpoint_path),
            "--json",
        ]
    )


def finish_cli_session(
    project: Path,
    session_id: str,
    *,
    request_id: str,
    summary: str,
    handoff: str,
) -> None:
    harness.cli_json(
        [
            "session",
            "finish",
            session_id,
            str(project),
            "--request-id",
            request_id,
            "--status",
            "completed",
            "--summary",
            summary,
            "--final-response",
            "Prior work was recorded for later continuity.",
            "--handoff",
            handoff,
            "--json",
        ]
    )


def seed_prior_session_native(
    project: Path,
    fixture: dict[str, object],
    private_root: Path,
) -> str | None:
    prior = fixture.get("prior_memory")
    if not isinstance(prior, dict):
        return None
    revision_state = str(fixture.get("prior_revision_state", "current"))
    original_branch = ""
    if revision_state in {"divergent", "merged"}:
        original_branch = agent_eval.git_run(project, ["branch", "--show-current"])
        if not original_branch:
            raise RuntimeError("revision-aware startup fixture requires an attached Git branch")
        agent_eval.git_run(project, ["checkout", f"ley-eval-prior-{revision_state}"])
        harness.run(["ingest", str(project), "--json"])
    try:
        seed = f"{fixture['id']}:prior"
        session_id = start_cli_session(
            project,
            request_id=harness.request_id(f"{seed}:start"),
            name=str(prior["name"]),
            goal=str(prior["goal"]),
        )
        checkpoint = private_root / "prior-checkpoint.json"
        write_checkpoint_data(
            checkpoint,
            request_id=harness.request_id(f"{seed}:checkpoint"),
            summary=str(prior["summary"]),
            decisions=[
                {
                    "title": str(prior["decision_title"]),
                    "decision": str(prior["decision"]),
                    "rationale": str(prior.get("rationale", "")),
                    "alternatives": [],
                }
            ],
            verification=[
                {
                    "kind": str(item["kind"]),
                    "status": str(item["status"]),
                    "summary": str(item["summary"]),
                    **(
                        {"command": str(item["command"])}
                        if isinstance(item.get("command"), str)
                        else {}
                    ),
                    "evidenceArtifactPaths": [],
                }
                for item in fixture.get("prior_verification", [])
                if isinstance(item, dict)
            ],
        )
        checkpoint_cli_session(project, session_id, checkpoint)
        finish_cli_session(
            project,
            session_id,
            request_id=harness.request_id(f"{seed}:finish"),
            summary=str(prior["summary"]),
            handoff="Use the recorded prior contract when this topic is revisited.",
        )
        return session_id
    finally:
        if revision_state in {"divergent", "merged"} and original_branch:
            agent_eval.git_run(project, ["checkout", original_branch])


def seed_trusted_learning_native(
    project: Path,
    fixture: dict[str, object],
    private_root: Path,
) -> str:
    specification = fixture.get("trusted_learning")
    if not isinstance(specification, dict):
        raise RuntimeError("trusted-learning startup fixture requires trusted_learning")
    evidence_path = str(specification["evidence_path"])
    evidence_file = project / evidence_path
    evidence_file.parent.mkdir(parents=True, exist_ok=True)
    evidence_file.write_text(str(specification["evidence_body"]), encoding="utf-8")
    harness.run(["ingest", str(project), "--json"])

    seed = f"{fixture['id']}:trusted-learning"
    session_id = start_cli_session(
        project,
        request_id=harness.request_id(f"{seed}:start"),
        name="Reviewed project convention evidence",
        goal="Preserve one reviewed project convention with explicit evidence.",
    )
    checkpoint_path = private_root / "learning-checkpoint.json"
    write_checkpoint_data(
        checkpoint_path,
        request_id=harness.request_id(f"{seed}:checkpoint"),
        summary="Reviewed evidence was captured for a project convention.",
        touched_artifacts=[evidence_path],
    )
    checkpoint_cli_session(project, session_id, checkpoint_path)
    shown = harness.cli_json(["session", "show", session_id, str(project), "--json"])
    checkpoints = shown.get("checkpoints", []) if isinstance(shown, dict) else []
    checkpoint_id = next(
        (
            str(item.get("checkpointId"))
            for item in reversed(checkpoints)
            if isinstance(item, dict) and isinstance(item.get("checkpointId"), str)
        ),
        "",
    )
    if not checkpoint_id:
        raise RuntimeError("trusted-learning fixture produced no checkpoint ID")
    proposed = harness.cli_json(
        [
            "learning",
            "propose",
            str(project),
            "--request-id",
            harness.request_id(f"{seed}:propose"),
            "--actor",
            "agent",
            "--provenance",
            "inferred",
            "--kind",
            str(specification["kind"]),
            "--title",
            str(specification["title"]),
            "--guidance",
            str(specification["guidance"]),
            "--confidence",
            str(int(specification["confidence_percent"])),
            "--evidence",
            f"{session_id}:{checkpoint_id}",
            "--json",
        ]
    )
    learning_id = cli_learning_id(proposed)
    harness.cli_json(
        [
            "learning",
            "review",
            learning_id,
            str(project),
            "--actor",
            "user",
            "--action",
            "confirm",
            "--note",
            "Confirmed for the startup-context evaluation.",
            "--request-id",
            harness.request_id(f"{seed}:review"),
            "--json",
        ]
    )
    finish_cli_session(
        project,
        session_id,
        request_id=harness.request_id(f"{seed}:finish"),
        summary="Reviewed project convention evidence recorded.",
        handoff="No additional handoff; rely on the reviewed learning when relevant.",
    )
    return learning_id


def precreate_host_session(project: Path, external_session_id: str) -> str:
    request_id = stable_host_request_id(external_session_id)
    short_id = request_id[len("req_") : len("req_") + 8]
    return start_cli_session(
        project,
        request_id=request_id,
        name=f"Codex session {short_id}",
        goal="Preserve durable, local continuity for this Codex project session.",
    )


def guidance_only_startup_context(
    session_id: str,
    *,
    recovery_records: int = 0,
) -> str:
    lines = [
        "# Ley project memory",
        "",
        f"Current Ley session: {session_id}.",
        "Historical Ley project memory was not auto-injected at startup. Call ley_brief when prior project continuity would materially help the current task; use ley_search and ley_evidence for deeper cited recall.",
    ]
    if recovery_records > 0:
        lines.extend(
            [
                "",
                (
                    "Recovery signal: this same Ley session has "
                    f"{recovery_records} prompt/response record(s) after its latest structured checkpoint "
                    "(reviewable-evidence). Their bodies were not injected here. Treat the interrupted window "
                    "as incomplete historical evidence: verify relevant repository/runtime state with normal "
                    "host tools before claiming an outcome, and use ley_checkpoint only for current state you "
                    "can now support. Keep anything uncertain or unfinished explicit."
                ),
            ]
        )
    return "\n".join(lines).rstrip() + "\n"


def combine_context(*parts: str) -> str:
    return "\n\n".join(part.strip() for part in parts if part.strip()).rstrip() + "\n"


def build_live_agent_prompt(task: str, context: str) -> str:
    return (
        "You are running one isolated coding-task evaluation.\n"
        "Work only inside the current repository. Do not read parent directories or hidden evaluation files.\n"
        "Ley MCP tools are available. Decide yourself whether prior project continuity materially helps this task; do not call Ley reflexively and do not assume startup history is authoritative.\n"
        "Use Ley only through the provided MCP tools. Inspect live repository files before editing; current live source and the user task outrank historical memory.\n"
        "Do not ask clarifying questions; make the best evidence-grounded change you can. You may run local commands/tests.\n\n"
        "Task:\n"
        f"{task.strip()}\n\n"
        f"{context.strip()}\n\n"
        "Finish by leaving the repository in the state you believe satisfies the task.\n"
    )


BRIDGE_SOURCE = r'''import socket, sys, threading
s = socket.create_connection((sys.argv[1], int(sys.argv[2])))
s.sendall((sys.argv[3] + "\n").encode("utf-8"))
def upload():
    for line in sys.stdin.buffer:
        s.sendall(line)
    try:
        s.shutdown(socket.SHUT_WR)
    except OSError:
        pass
threading.Thread(target=upload, daemon=True).start()
for line in s.makefile("rb"):
    sys.stdout.buffer.write(line)
    sys.stdout.buffer.flush()
'''


class McpRelay:
    """Forward one or more sandbox MCP connections to host-side Ley stdio servers."""

    def __init__(self, project: Path, env: dict[str, str]) -> None:
        self.project = project
        self.env = env
        self.token = hashlib.sha256(os.urandom(32)).hexdigest()
        self.listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen(4)
        self.listener.settimeout(0.2)
        self.port = int(self.listener.getsockname()[1])
        self.stop_event = threading.Event()
        self.thread: threading.Thread | None = None
        self.lock = threading.Lock()
        self.methods: list[str] = []
        self.tool_calls: list[str] = []
        self.server_failures = 0

    def start(self) -> None:
        self.thread = threading.Thread(target=self._serve, daemon=True)
        self.thread.start()

    def _record(self, line: str) -> None:
        try:
            payload = json.loads(line)
        except json.JSONDecodeError:
            return
        if not isinstance(payload, dict):
            return
        method = payload.get("method")
        if not isinstance(method, str):
            return
        with self.lock:
            self.methods.append(method)
            if method == "tools/call":
                params = payload.get("params")
                if isinstance(params, dict) and isinstance(params.get("name"), str):
                    self.tool_calls.append(str(params["name"]))

    def _serve(self) -> None:
        while not self.stop_event.is_set():
            try:
                connection, _ = self.listener.accept()
            except socket.timeout:
                continue
            except OSError:
                if self.stop_event.is_set():
                    return
                raise
            threading.Thread(
                target=self._handle_connection,
                args=(connection,),
                daemon=True,
            ).start()

    def _handle_connection(self, connection: socket.socket) -> None:
        reader = connection.makefile("r", encoding="utf-8", newline="\n")
        writer = connection.makefile("w", encoding="utf-8", newline="\n")
        if reader.readline().strip() != self.token:
            connection.close()
            return
        process = subprocess.Popen(
            [
                str(REPO_ROOT / "target" / "debug" / "ley"),
                "mcp",
                str(self.project),
                "--allow-session-writes",
            ],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=self.env,
        )
        assert process.stdin is not None
        assert process.stdout is not None
        assert process.stderr is not None

        def client_to_server() -> None:
            try:
                for line in reader:
                    self._record(line)
                    process.stdin.write(line)
                    process.stdin.flush()
            except (BrokenPipeError, OSError):
                pass
            finally:
                try:
                    process.stdin.close()
                except (BrokenPipeError, OSError):
                    pass

        def drain_stderr() -> None:
            for _line in process.stderr:
                pass

        threading.Thread(target=client_to_server, daemon=True).start()
        threading.Thread(target=drain_stderr, daemon=True).start()
        try:
            for line in process.stdout:
                writer.write(line)
                writer.flush()
        except (BrokenPipeError, OSError):
            pass
        finally:
            try:
                connection.close()
            except OSError:
                pass
            try:
                code = process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                process.kill()
                code = process.wait()
            if code != 0:
                with self.lock:
                    self.server_failures += 1

    def close(self) -> None:
        self.stop_event.set()
        try:
            self.listener.close()
        except OSError:
            pass
        if self.thread is not None:
            self.thread.join(timeout=1)

    def summary(self) -> dict[str, object]:
        with self.lock:
            counts = Counter(self.tool_calls)
            return {
                "toolCallCount": len(self.tool_calls),
                "toolCallCounts": dict(sorted(counts.items())),
                "retrievalCallCount": sum(
                    counts.get(name, 0)
                    for name in ("ley_brief", "ley_search", "ley_evidence")
                ),
                "checkpointCallCount": counts.get("ley_checkpoint", 0),
                "briefCalled": counts.get("ley_brief", 0) > 0,
                "searchCalled": counts.get("ley_search", 0) > 0,
                "evidenceCalled": counts.get("ley_evidence", 0) > 0,
                "serverFailureCount": self.server_failures,
            }


def assert_canonical_tool_inventory(project: Path) -> list[str]:
    tools = harness.mcp_tools_list(project, ("--allow-session-writes",))
    names = sorted(
        str(item["name"])
        for item in tools
        if isinstance(item, dict) and isinstance(item.get("name"), str)
    )
    if set(names) != CANONICAL_TOOLS or len(names) != len(CANONICAL_TOOLS):
        raise RuntimeError(
            "C4 live-MCP project is not canonical-only; advertised tools: "
            + ", ".join(names)
        )
    return names


def make_codex_mounts(
    root: Path,
    relay: McpRelay,
    operator_mounts: list[tuple[Path, str]],
) -> list[tuple[Path, str]]:
    connector = root / "ley-mcp-bridge.py"
    connector.write_text(BRIDGE_SOURCE, encoding="utf-8")
    config = root / "codex-config.toml"
    config.write_text(
        "[mcp_servers.ley]\n"
        'command = "python3"\n'
        "args = ["
        '"/home/runner/.ley-c4/bridge.py", '
        '"127.0.0.1", '
        f'"{relay.port}", '
        f'"{relay.token}"'
        "]\n",
        encoding="utf-8",
    )
    required = [
        (config.resolve(), "/home/runner/.codex/config.toml"),
        (connector.resolve(), "/home/runner/.ley-c4/bridge.py"),
    ]
    destinations = {destination for _, destination in operator_mounts}
    for source, destination in required:
        if destination in destinations:
            raise RuntimeError(f"operator mount collides with C4 mount {destination}")
        operator_mounts.append((source, destination))
        destinations.add(destination)
    return operator_mounts


def context_marker_metrics(
    fixture: dict[str, object],
    context: str,
) -> dict[str, object]:
    required = [
        str(value)
        for value in [
            *fixture.get("context_markers", []),
            *fixture.get("ley_context_markers", []),
        ]
    ]
    forbidden = [
        str(value)
        for value in [
            *fixture.get("forbidden_context_markers", []),
            *fixture.get("ley_forbidden_context_markers", []),
        ]
    ]
    lower = context.lower()
    present = sum(marker.lower() in lower for marker in required)
    leaks = sum(marker.lower() in lower for marker in forbidden)
    return {
        "requiredMarkerCount": len(required),
        "presentRequiredMarkerCount": present,
        "requiredMarkerCoverage": present / len(required) if required else 1.0,
        "forbiddenMarkerCount": len(forbidden),
        "forbiddenMarkerLeakCount": leaks,
    }


def prepare_startup_context(
    memory_project: Path,
    fixture: dict[str, object],
    *,
    variant: str,
    external_session_id: str,
    current_turn_id: str,
) -> tuple[str, str]:
    session_id = precreate_host_session(memory_project, external_session_id)
    recovery_records = 0
    if fixture.get("startupCase") == "interrupted-current-session":
        prior_prompt = str(fixture["interrupted_prompt"])
        prior = harness.hook_call(
            memory_project,
            "codex",
            {
                "hook_event_name": "UserPromptSubmit",
                "session_id": external_session_id,
                "turn_id": current_turn_id + "-prior",
                "prompt": prior_prompt,
            },
        )
        if session_id not in harness.hook_additional_context(prior):
            raise RuntimeError("interrupted C4 prompt resolved a different Ley session")
        recovery_records = 1

    if variant == "contentful":
        startup = harness.hook_call(
            memory_project,
            "codex",
            {
                "hook_event_name": "SessionStart",
                "session_id": external_session_id,
            },
        )
        if harness.hook_ley_session_id(startup) != session_id:
            raise RuntimeError("C4 SessionStart resolved a different Ley session")
        startup_context = harness.hook_additional_context(startup).strip()
    else:
        startup_context = guidance_only_startup_context(
            session_id,
            recovery_records=recovery_records,
        ).strip()

    current = harness.hook_call(
        memory_project,
        "codex",
        {
            "hook_event_name": "UserPromptSubmit",
            "session_id": external_session_id,
            "turn_id": current_turn_id,
            "prompt": str(fixture["task"]),
        },
    )
    if session_id not in harness.hook_additional_context(current):
        raise RuntimeError("C4 current prompt resolved a different Ley session")
    guidance = harness.hook_additional_context(current).strip()
    combined = combine_context(startup_context, guidance)
    if str(memory_project) in combined:
        raise RuntimeError("C4 startup context leaked the private Ley project path")
    return combined, session_id


def seed_fixture_memory(
    memory_project: Path,
    fixture: dict[str, object],
    private_root: Path,
) -> None:
    startup_case = str(fixture["startupCase"])
    if startup_case == "trusted-learning":
        seed_trusted_learning_native(memory_project, fixture, private_root)
    elif startup_case != "interrupted-current-session":
        seed_prior_session_native(memory_project, fixture, private_root)


def execute_variant(
    fixture: dict[str, object],
    root: Path,
    command: list[str],
    operator_mounts: list[tuple[Path, str]],
    variant: str,
    repetition: int,
    timeout_seconds: int,
    capture_audit: bool,
) -> dict[str, object]:
    if variant not in VARIANTS:
        raise RuntimeError(f"unsupported C4 variant {variant!r}")
    visible = root / "project"
    memory_root = root / "ley-private"
    memory_project = memory_root / "project"
    visible.mkdir(parents=True)
    memory_root.mkdir(mode=0o700)
    files = fixture.get("project_files")
    if not isinstance(files, dict):
        raise RuntimeError("C4 fixture requires project_files")
    agent_eval.write_project_files(
        visible,
        {str(path): str(body) for path, body in files.items()},
    )
    agent_eval.git_run(visible, ["init", "-b", "main"])
    agent_eval.git_commit_all(visible, "startup-context fixture")
    agent_eval.prepare_fixture_git_state(visible, fixture)
    shutil.copytree(visible, memory_project)
    initial_snapshot = agent_eval.snapshot_project_tree(visible)

    previous_config = harness.EVAL_ENV.get("XDG_CONFIG_HOME")
    harness.EVAL_ENV["XDG_CONFIG_HOME"] = str(memory_root / "config")
    relay: McpRelay | None = None
    try:
        init_native_project(memory_project, "Startup context evaluation")
        seed_fixture_memory(memory_project, fixture, memory_root)
        canonical_tools = assert_canonical_tool_inventory(memory_project)

        external_session_id = (
            "ley-c4-"
            + hashlib.sha256(
                f"{fixture['id']}:{variant}:{repetition}".encode("utf-8")
            ).hexdigest()[:20]
        )
        current_turn_id = "turn-" + hashlib.sha256(
            f"{fixture['id']}:{variant}:{repetition}:current".encode("utf-8")
        ).hexdigest()[:20]
        context, session_id = prepare_startup_context(
            memory_project,
            fixture,
            variant=variant,
            external_session_id=external_session_id,
            current_turn_id=current_turn_id,
        )
        marker_metrics = context_marker_metrics(fixture, context)
        if fixture.get("startupCase") == "interrupted-current-session":
            leaked = [
                str(marker)
                for marker in fixture.get("forbidden_context_markers", [])
                if str(marker).lower() in context.lower()
            ]
            if leaked:
                raise RuntimeError("C4 startup leaked interrupted prompt body markers")

        relay_env = os.environ.copy()
        relay_env.update(harness.EVAL_ENV)
        relay = McpRelay(memory_project, relay_env)
        relay.start()
        mounts = make_codex_mounts(root, relay, list(operator_mounts))
        prompt = build_live_agent_prompt(str(fixture["task"]), context)
        runner = agent_eval.run_external_agent(
            command,
            visible,
            prompt,
            timeout_seconds,
            f"startup-{variant}",
            [],
            mounts,
            capture_audit,
        )
        runner_stdout = runner.pop("_stdout", b"")
        runner_stderr = runner.pop("_stderr", b"")
        time.sleep(0.15)
        relay.close()
        relay_summary = relay.summary()
        after_snapshot = agent_eval.snapshot_project_tree(visible)
        changed_files, diff_material = agent_eval.compare_project_snapshots(
            initial_snapshot,
            after_snapshot,
        )
        constraints = agent_eval.evaluate_task_constraints(
            fixture,
            visible,
            initial_snapshot,
            after_snapshot,
            changed_files,
            timeout_seconds,
        )
        oracle = (
            agent_eval.run_hidden_oracle(
                fixture,
                visible,
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
                "stdoutSha256": agent_eval.sha256_bytes(b""),
                "stderrSha256": agent_eval.sha256_bytes(b""),
            }
        )
        oracle_stdout = oracle.pop("_stdout", "")
        oracle_stderr = oracle.pop("_stderr", "")
        final_snapshot = agent_eval.snapshot_project_tree(visible)
        post_check_tree_stable = final_snapshot == after_snapshot
        if not post_check_tree_stable:
            changed_files, diff_material = agent_eval.compare_project_snapshots(
                initial_snapshot,
                final_snapshot,
            )
        task_passed = agent_eval.task_attempt_passed(
            runner,
            constraints,
            oracle,
            post_check_tree_stable,
        )
        result: dict[str, object] = {
            "variant": variant,
            "taskPassed": task_passed,
            "hiddenOracleAttempted": oracle["attempted"],
            "hiddenOracleStatus": oracle["status"],
            "hiddenOraclePassed": oracle["passed"],
            "postCheckTreeStable": post_check_tree_stable,
            "runner": runner,
            "constraints": agent_eval.public_constraint_summary(constraints),
            "changedFileCount": len(changed_files),
            "changedPathsSha256": agent_eval.path_set_sha256(changed_files),
            "diffBytes": len(diff_material),
            "diffSha256": agent_eval.sha256_bytes(diff_material),
            "promptSha256": agent_eval.sha256_text(prompt),
            "promptCharacters": len(prompt),
            "startup": {
                "sessionId": session_id,
                "canonicalTools": canonical_tools,
                "contextCharacters": len(context),
                "estimatedTokens": agent_eval.approximate_text_tokens(context),
                "contextSha256": agent_eval.sha256_text(context),
                **marker_metrics,
            },
            "mcp": relay_summary,
        }
        if capture_audit:
            result["_audit"] = {
                "prompt": prompt,
                "context": context,
                "runnerStdout": runner_stdout,
                "runnerStderr": runner_stderr,
                "oracleStdout": oracle_stdout,
                "oracleStderr": oracle_stderr,
                "diffMaterial": diff_material,
                "changedFiles": changed_files,
            }
        return result
    finally:
        if relay is not None:
            relay.close()
        if previous_config is None:
            harness.EVAL_ENV.pop("XDG_CONFIG_HOME", None)
        else:
            harness.EVAL_ENV["XDG_CONFIG_HOME"] = previous_config


def variants_for_repetition(first_variant: str, repetition: int) -> tuple[str, str]:
    if first_variant not in VARIANTS:
        raise RuntimeError("invalid C4 first variant")
    if repetition % 2 == 1:
        first = first_variant
    else:
        first = VARIANTS[1] if first_variant == VARIANTS[0] else VARIANTS[0]
    second = VARIANTS[1] if first == VARIANTS[0] else VARIANTS[0]
    return first, second


def summarize(results: list[dict[str, object]], variant: str) -> dict[str, object]:
    rows = [item for item in results if item.get("variant") == variant]
    task_passes = sum(bool(item.get("taskPassed")) for item in rows)
    oracle_attempted = [item for item in rows if item.get("hiddenOracleAttempted") is True]
    oracle_passes = sum(item.get("hiddenOraclePassed") is True for item in oracle_attempted)
    contexts = [
        int(item["startup"]["contextCharacters"])
        for item in rows
        if isinstance(item.get("startup"), dict)
    ]
    runtimes = [
        float(item["runner"]["seconds"])
        for item in rows
        if isinstance(item.get("runner"), dict)
        and isinstance(item["runner"].get("seconds"), (int, float))
    ]
    retrieval_calls = [
        int(item["mcp"]["retrievalCallCount"])
        for item in rows
        if isinstance(item.get("mcp"), dict)
    ]
    forbidden_leaks = sum(
        int(item["startup"].get("forbiddenMarkerLeakCount", 0))
        for item in rows
        if isinstance(item.get("startup"), dict)
    )
    marker_coverages = [
        float(item["startup"]["requiredMarkerCoverage"])
        for item in rows
        if isinstance(item.get("startup"), dict)
        and isinstance(
            item["startup"].get("requiredMarkerCoverage"),
            (int, float),
        )
    ]
    checkpoint_calls = [
        int(item["mcp"]["checkpointCallCount"])
        for item in rows
        if isinstance(item.get("mcp"), dict)
    ]
    brief_calls = [
        bool(item["mcp"]["briefCalled"])
        for item in rows
        if isinstance(item.get("mcp"), dict)
    ]
    search_calls = [
        bool(item["mcp"]["searchCalled"])
        for item in rows
        if isinstance(item.get("mcp"), dict)
    ]
    evidence_calls = [
        bool(item["mcp"]["evidenceCalled"])
        for item in rows
        if isinstance(item.get("mcp"), dict)
    ]
    server_failures = sum(
        int(item["mcp"].get("serverFailureCount", 0))
        for item in rows
        if isinstance(item.get("mcp"), dict)
    )
    return {
        "attempts": len(rows),
        "taskPassRate": task_passes / len(rows) if rows else None,
        "hiddenOracleAttempted": len(oracle_attempted),
        "hiddenOraclePassRate": (
            oracle_passes / len(oracle_attempted) if oracle_attempted else None
        ),
        "meanContextCharacters": sum(contexts) / len(contexts) if contexts else 0,
        "meanRunnerSeconds": sum(runtimes) / len(runtimes) if runtimes else None,
        "meanRetrievalCalls": (
            sum(retrieval_calls) / len(retrieval_calls) if retrieval_calls else 0
        ),
        "retrievalUsedRate": (
            sum(value > 0 for value in retrieval_calls) / len(retrieval_calls)
            if retrieval_calls
            else None
        ),
        "briefCalledRate": (
            sum(brief_calls) / len(brief_calls) if brief_calls else None
        ),
        "searchCalledRate": (
            sum(search_calls) / len(search_calls) if search_calls else None
        ),
        "evidenceCalledRate": (
            sum(evidence_calls) / len(evidence_calls) if evidence_calls else None
        ),
        "meanCheckpointCalls": (
            sum(checkpoint_calls) / len(checkpoint_calls) if checkpoint_calls else 0
        ),
        "meanRequiredMarkerCoverage": (
            sum(marker_coverages) / len(marker_coverages) if marker_coverages else None
        ),
        "forbiddenMarkerLeakCount": forbidden_leaks,
        "mcpServerFailureCount": server_failures,
    }


def grouped_summaries(
    results: list[dict[str, object]],
    key: str,
) -> dict[str, dict[str, dict[str, object]]]:
    groups: dict[str, list[dict[str, object]]] = {}
    for item in results:
        value = item.get(key)
        if not isinstance(value, str) or not value:
            raise RuntimeError(f"C4 result is missing grouping key {key!r}")
        groups.setdefault(value, []).append(item)
    return {
        group: {
            variant: summarize(group_results, variant)
            for variant in VARIANTS
        }
        for group, group_results in sorted(groups.items())
    }


def study_source_metadata(*, require_clean: bool) -> dict[str, object]:
    head = agent_eval.git_run(REPO_ROOT, ["rev-parse", "HEAD"])
    status = agent_eval.git_run(REPO_ROOT, ["status", "--porcelain"])
    if require_clean and status:
        raise RuntimeError(
            "C4 model runs require a clean committed repository so the report can be reproduced from its HEAD"
        )
    return {
        "headSha": head,
        "worktreeClean": not bool(status),
        "harnessSha256": agent_eval.sha256_bytes(Path(__file__).read_bytes()),
        "fixtureManifestSha256": agent_eval.sha256_bytes(FIXTURES.read_bytes()),
    }


def compact_console_report(
    report: dict[str, object],
    *,
    output: Path | None,
    audit_dir: Path | None,
) -> dict[str, object]:
    comparison = report["comparison"]
    assert isinstance(comparison, dict)
    return {
        "study": report["study"],
        "source": report["source"],
        "selectedTaskCount": report["selectedTaskCount"],
        "repetitions": report["repetitions"],
        "plannedAgentAttempts": report["plannedAgentAttempts"],
        "output": str(output) if output else None,
        "auditDir": str(audit_dir) if audit_dir else None,
        "taskPassRateDeltaContentfulMinusGuidance": comparison[
            "taskPassRateDeltaContentfulMinusGuidance"
        ],
        "variantSummaries": comparison["variantSummaries"],
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the opt-in C4 contentful-vs-guidance-only SessionStart study."
    )
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--validate", action="store_true")
    parser.add_argument("--task", action="append", default=[])
    parser.add_argument("--all-tasks", action="store_true")
    parser.add_argument("--repetitions", type=int, default=1)
    parser.add_argument("--first-variant", choices=VARIANTS, default="contentful")
    parser.add_argument("--runner-command")
    parser.add_argument("--runner-label", default="")
    parser.add_argument(
        "--runner-ro-bind",
        action="append",
        default=[],
        metavar="SOURCE=DEST",
    )
    parser.add_argument("--timeout-seconds", type=int, default=600)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--audit-dir", type=Path)
    return parser.parse_args(argv)


def select_fixtures(
    fixtures: list[dict[str, object]],
    requested: list[str],
    all_tasks: bool,
    *,
    default_all: bool,
) -> list[dict[str, object]]:
    by_id = {str(item["id"]): item for item in fixtures}
    if len(requested) != len(set(requested)):
        raise RuntimeError("duplicate --task selector")
    if requested and all_tasks:
        raise RuntimeError("--task and --all-tasks cannot be combined")
    if requested:
        missing = [item for item in requested if item not in by_id]
        if missing:
            raise RuntimeError("unknown C4 task: " + ", ".join(missing))
        return [by_id[item] for item in requested]
    if all_tasks or default_all:
        return fixtures
    return []


def validate_fixture(fixture: dict[str, object], root: Path) -> dict[str, object]:
    oracle = agent_eval.validate_script_oracle_reference(
        fixture,
        root,
        agent_eval.ORACLE_REFERENCE_TIMEOUT_SECONDS,
    )
    startup_context_validation = validate_startup_contexts(fixture, root / "startup-context")
    return {
        "taskId": fixture["id"],
        "riskClass": fixture["riskClass"],
        "startupCase": fixture["startupCase"],
        "oracleReferenceValidation": oracle,
        "startupContextValidation": startup_context_validation,
    }


def validate_startup_contexts(
    fixture: dict[str, object],
    root: Path,
) -> dict[str, object]:
    rows: dict[str, object] = {}
    for variant in VARIANTS:
        attempt_root = root / variant
        project = attempt_root / "project"
        private_root = attempt_root / "ley-private"
        project.mkdir(parents=True)
        private_root.mkdir(mode=0o700)
        files = fixture.get("project_files")
        if not isinstance(files, dict):
            raise RuntimeError("C4 fixture requires project_files")
        agent_eval.write_project_files(
            project,
            {str(path): str(body) for path, body in files.items()},
        )
        agent_eval.git_run(project, ["init", "-b", "main"])
        agent_eval.git_commit_all(project, "startup-context validation fixture")
        agent_eval.prepare_fixture_git_state(project, fixture)

        previous_config = harness.EVAL_ENV.get("XDG_CONFIG_HOME")
        harness.EVAL_ENV["XDG_CONFIG_HOME"] = str(private_root / "config")
        try:
            init_native_project(project, "Startup context validation")
            seed_fixture_memory(project, fixture, private_root)
            external_session_id = (
                "ley-c4-validation-"
                + hashlib.sha256(
                    f"{fixture['id']}:{variant}".encode("utf-8")
                ).hexdigest()[:20]
            )
            current_turn_id = "turn-validation-" + hashlib.sha256(
                f"{fixture['id']}:{variant}:current".encode("utf-8")
            ).hexdigest()[:20]
            context, _session_id = prepare_startup_context(
                project,
                fixture,
                variant=variant,
                external_session_id=external_session_id,
                current_turn_id=current_turn_id,
            )
            metrics = context_marker_metrics(fixture, context)
            expectations = fixture["startupExpectations"]
            assert isinstance(expectations, dict)
            expected = expectations[variant]
            assert isinstance(expected, dict)
            for field in ("requiredMarkerCoverage", "forbiddenMarkerLeakCount"):
                if metrics[field] != expected[field]:
                    raise RuntimeError(
                        f"C4 {fixture['id']} {variant} startup {field}={metrics[field]!r}; expected {expected[field]!r}"
                    )
            if variant == "guidance-only":
                for forbidden_section in ("## Recent work", "## Reviewed project learnings"):
                    if forbidden_section in context:
                        raise RuntimeError(
                            f"C4 guidance-only startup unexpectedly contains {forbidden_section}"
                        )
            rows[variant] = {
                "contextCharacters": len(context),
                **metrics,
            }
        finally:
            if previous_config is None:
                harness.EVAL_ENV.pop("XDG_CONFIG_HOME", None)
            else:
                harness.EVAL_ENV["XDG_CONFIG_HOME"] = previous_config
    return rows


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    fixtures = load_fixtures()
    if args.list:
        for fixture in fixtures:
            print(f"{fixture['id']}\t{fixture['riskClass']}\t{fixture['task']}")
        return 0
    try:
        selected = select_fixtures(
            fixtures,
            list(args.task),
            args.all_tasks,
            default_all=args.validate,
        )
    except RuntimeError as error:
        raise SystemExit(str(error)) from error
    if args.validate:
        rows: list[dict[str, object]] = []
        validation_seed = bytes.fromhex("c4" * 32)
        with tempfile.TemporaryDirectory(prefix="ley-c4-validate-") as temporary:
            root = Path(temporary)
            for index, raw_fixture in enumerate(selected):
                fixture = agent_eval.materialize_fixture(raw_fixture, validation_seed)
                rows.append(validate_fixture(fixture, root / f"task-{index:03d}"))
        print(json.dumps({"validated": rows}, indent=2, sort_keys=True))
        return 0
    if not selected:
        raise SystemExit("C4 model runs require --task or --all-tasks")
    if not args.runner_command:
        raise SystemExit("--runner-command is required")
    if args.repetitions < 1 or args.repetitions > MAX_REPETITIONS:
        raise SystemExit(f"--repetitions must be between 1 and {MAX_REPETITIONS}")
    if args.timeout_seconds < 1:
        raise SystemExit("--timeout-seconds must be positive")
    if args.audit_dir and args.audit_dir.exists():
        raise SystemExit(f"audit directory already exists: {args.audit_dir}")
    command = agent_eval.shlex.split(args.runner_command)
    if not command:
        raise SystemExit("--runner-command parsed to an empty command")
    if "--ignore-user-config" in command:
        raise SystemExit(
            "C4 runner must not use --ignore-user-config because the isolated Codex config carries the Ley MCP definition"
        )
    try:
        operator_mounts = [
            agent_eval.parse_runner_read_only_mount(spec)
            for spec in args.runner_ro_bind
        ]
    except RuntimeError as error:
        raise SystemExit(str(error)) from error
    try:
        source_metadata = study_source_metadata(require_clean=True)
    except RuntimeError as error:
        raise SystemExit(str(error)) from error

    planned = len(selected) * args.repetitions * 2
    print(
        f"Selected {len(selected)} C4 task(s); planned external-agent attempts: {planned}",
        flush=True,
    )
    results: list[dict[str, object]] = []
    audit_rows: list[dict[str, object]] = []
    attempt = 0
    master_seed = os.urandom(32)
    with tempfile.TemporaryDirectory(prefix="ley-c4-run-") as temporary:
        base = Path(temporary)
        for repetition in range(1, args.repetitions + 1):
            for task_index, raw_fixture in enumerate(selected, start=1):
                fixture = agent_eval.materialize_fixture(raw_fixture, master_seed)
                order_index = task_index + repetition - 1
                variants = variants_for_repetition(args.first_variant, order_index)
                for variant in variants:
                    attempt += 1
                    print(
                        f"[{attempt}/{planned}] rep={repetition}/{args.repetitions} {fixture['id']} {variant}: running",
                        flush=True,
                    )
                    result = execute_variant(
                        fixture,
                        base / f"task-{task_index:03d}-rep-{repetition:02d}-{variant}",
                        command,
                        operator_mounts,
                        variant,
                        repetition,
                        args.timeout_seconds,
                        args.audit_dir is not None,
                    )
                    audit = result.pop("_audit", None)
                    result["taskId"] = fixture["id"]
                    result["taskFamily"] = fixture.get("task_family", fixture["riskClass"])
                    result["riskClass"] = fixture["riskClass"]
                    result["repetition"] = repetition
                    results.append(result)
                    if isinstance(audit, dict):
                        audit_rows.append(
                            {
                                "taskId": fixture["id"],
                                "variant": variant,
                                "repetition": repetition,
                                "payload": audit,
                            }
                        )
                    print(
                        "  task="
                        + ("PASS" if result["taskPassed"] else "FAIL")
                        + f" retrieval_calls={result['mcp']['retrievalCallCount']} runner_seconds={result['runner']['seconds']}",
                        flush=True,
                    )

    summaries = {variant: summarize(results, variant) for variant in VARIANTS}
    per_task = grouped_summaries(results, "taskId")
    per_risk_class = grouped_summaries(results, "riskClass")
    report = {
        "schemaVersion": 1,
        "study": "session-start-content-vs-guidance-only",
        "source": source_metadata,
        "runner": {
            "executable": Path(command[0]).name,
            "label": args.runner_label,
            "commandSha256": agent_eval.sha256_text("\0".join(command)),
            "operatorReadOnlyMountDestinations": sorted(
                destination for _, destination in operator_mounts
            ),
        },
        "repetitions": args.repetitions,
        "firstVariant": args.first_variant,
        "selectedTaskCount": len(selected),
        "plannedAgentAttempts": planned,
        "taskIds": [str(item["id"]) for item in selected],
        "results": results,
        "comparison": {
            "variantSummaries": summaries,
            "perTask": per_task,
            "perRiskClass": per_risk_class,
            "taskPassRateDeltaContentfulMinusGuidance": (
                float(summaries["contentful"]["taskPassRate"])
                - float(summaries["guidance-only"]["taskPassRate"])
                if summaries["contentful"]["taskPassRate"] is not None
                and summaries["guidance-only"]["taskPassRate"] is not None
                else None
            ),
            "interpretation": (
                "This is an opt-in downstream observation, not deterministic CI. Both arms have the same live canonical four-tool Ley MCP access and prompt capture. The guidance-only arm receives no automatic prior-session/learning bodies and is not forced to retrieve; whether it calls Ley is part of the measured product behavior."
            ),
        },
    }
    encoded = json.dumps(report, indent=2, sort_keys=True)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded + "\n", encoding="utf-8")
    if args.audit_dir:
        args.audit_dir.mkdir(parents=True, mode=0o700)
        (args.audit_dir / "report.json").write_text(encoded + "\n", encoding="utf-8")
        for index, row in enumerate(audit_rows, start=1):
            run_dir = args.audit_dir / f"attempt-{index:03d}-{row['variant']}"
            run_dir.mkdir(mode=0o700)
            payload = row["payload"]
            assert isinstance(payload, dict)
            (run_dir / "prompt.txt").write_text(str(payload["prompt"]), encoding="utf-8")
            (run_dir / "context.txt").write_text(str(payload["context"]), encoding="utf-8")
            (run_dir / "runner.stdout").write_bytes(bytes(payload["runnerStdout"]))
            (run_dir / "runner.stderr").write_bytes(bytes(payload["runnerStderr"]))
            (run_dir / "oracle.stdout").write_text(str(payload["oracleStdout"]), encoding="utf-8")
            (run_dir / "oracle.stderr").write_text(str(payload["oracleStderr"]), encoding="utf-8")
            (run_dir / "diff.bin").write_bytes(bytes(payload["diffMaterial"]))
            (run_dir / "changed-files.json").write_text(
                json.dumps(payload["changedFiles"], indent=2) + "\n",
                encoding="utf-8",
            )
    if args.output or args.audit_dir:
        print(
            "\n"
            + json.dumps(
                compact_console_report(
                    report,
                    output=args.output,
                    audit_dir=args.audit_dir,
                ),
                indent=2,
                sort_keys=True,
            ),
            flush=True,
        )
    else:
        print("\n" + encoded, flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        raise SystemExit(2) from error
