#!/usr/bin/env python3
"""Run the real Ley CLI/MCP evaluation fixtures as fail-fast assertions.

Every scenario must execute its declared setup and checks. A missing tool
response, skipped event kind, invalid fixture identifier, or unmet expectation
is a failed scenario and makes this command exit non-zero.
"""

import argparse
import hashlib
import json
import os
import select
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
FIXTURES = Path(__file__).parent / "fixtures" / "scenarios.jsonl"
K = 5
WRITE_FLAGS = ("--allow-session-writes", "--allow-learning-proposals")
METRIC_NAMES = (
    "recall@k",
    "precision",
    "untrusted_boundary",
    "cross_project_clean",
    "stale_learning",
    "capture_recovery",
    "memory_recovery",
    "memory_transition",
    "memory_binding",
    "origin_lineage",
    "idempotency",
    "token_budget",
    "secret_exclusion",
    "specification_admission",
    "mounted_reference",
    "premise_adjudication",
    "revision_adjudication",
    "egress_policy",
    "selective_abstention",
    "parallel_session_separation",
    "deletion_fidelity",
    "forgetting_residue_rate",
    "inactive_workspace_clean",
    "host_portability",
    "downstream_task_contract",
    "budget_baseline_advantage",
    "privacy_violation_rate",
    "topic_dossier",
    "current_project_state",
    "context_pack_inspector",
)

P0_CAPABILITY_COVERAGE = {
    "context-compiler": {
        "adversarial": ("no-useful-memory-honesty", "selective_abstention", "truthy"),
        "downstream": (
            "budgeted-compiler-vs-recent-resume",
            "downstream_task_contract",
            "truthy",
        ),
        "privacy": (
            "budgeted-compiler-vs-recent-resume",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": ("strict-token-budgets-500", "token_budget", "truthy"),
    },
    "memory-compiler": {
        "adversarial": (
            "crash-before-session-end-resume",
            "memory_transition",
            "truthy",
        ),
        "downstream": (
            "crash-before-session-end-resume",
            "memory_recovery",
            "truthy",
        ),
        "privacy": (
            "crash-before-session-end-resume",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "crash-before-session-end-resume",
            "memory_binding",
            "truthy",
        ),
    },
    "specifications": {
        "adversarial": (
            "specification-authority-context",
            "specification_admission",
            "truthy",
        ),
        "downstream": (
            "specification-authority-context",
            "specification_admission",
            "truthy",
        ),
        "privacy": (
            "specification-agent-egress-canary",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "specification-authority-context",
            "specification_admission",
            "truthy",
        ),
    },
    "context-mounts": {
        "adversarial": (
            "explicit-project-context-mount",
            "mounted_reference",
            "truthy",
        ),
        "downstream": (
            "explicit-project-context-mount",
            "mounted_reference",
            "truthy",
        ),
        "privacy": (
            "explicit-project-context-mount",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "explicit-project-context-mount",
            "mounted_reference",
            "truthy",
        ),
    },
    "origin-lineage": {
        "adversarial": (
            "crash-before-session-end-resume",
            "origin_lineage",
            "truthy",
        ),
        "downstream": (
            "crash-before-session-end-resume",
            "origin_lineage",
            "truthy",
        ),
        "privacy": (
            "session-erasure-derived-residue",
            "forgetting_residue_rate",
            "zero",
        ),
        "regression": (
            "crash-before-session-end-resume",
            "origin_lineage",
            "truthy",
        ),
    },
    "premise-adjudication": {
        "adversarial": (
            "explicit-learning-supersession-premise",
            "premise_adjudication",
            "truthy",
        ),
        "downstream": (
            "explicit-learning-supersession-premise",
            "premise_adjudication",
            "truthy",
        ),
        "privacy": (
            "explicit-learning-supersession-premise",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "explicit-learning-supersession-premise",
            "premise_adjudication",
            "truthy",
        ),
    },
    "revision-awareness": {
        "adversarial": (
            "divergent-branch-state-adjudication",
            "revision_adjudication",
            "truthy",
        ),
        "downstream": (
            "divergent-branch-state-adjudication",
            "revision_adjudication",
            "truthy",
        ),
        "privacy": (
            "divergent-branch-state-adjudication",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "divergent-branch-state-adjudication",
            "revision_adjudication",
            "truthy",
        ),
    },
    "egress-policy": {
        "adversarial": (
            "specification-agent-egress-canary",
            "egress_policy",
            "truthy",
        ),
        "downstream": (
            "specification-agent-egress-canary",
            "egress_policy",
            "truthy",
        ),
        "privacy": (
            "specification-agent-egress-canary",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "specification-agent-egress-canary",
            "egress_policy",
            "truthy",
        ),
    },
}

P1_CAPABILITY_COVERAGE = {
    "topic-dossiers": {
        "adversarial": (
            "session-erasure-derived-residue",
            "forgetting_residue_rate",
            "zero",
        ),
        "downstream": ("topic-dossier-authentication", "topic_dossier", "truthy"),
        "privacy": (
            "topic-dossier-authentication",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": ("topic-dossier-authentication", "topic_dossier", "truthy"),
    },
    "current-project-state": {
        "adversarial": (
            "session-erasure-derived-residue",
            "forgetting_residue_rate",
            "zero",
        ),
        "downstream": (
            "current-project-state-storage",
            "current_project_state",
            "truthy",
        ),
        "privacy": (
            "current-project-state-storage",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "current-project-state-storage",
            "current_project_state",
            "truthy",
        ),
    },
    "context-pack-inspector": {
        "adversarial": (
            "context-pack-inspector-attribution",
            "context_pack_inspector",
            "truthy",
        ),
        "downstream": (
            "context-pack-inspector-attribution",
            "context_pack_inspector",
            "truthy",
        ),
        "privacy": (
            "context-pack-inspector-attribution",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "context-pack-inspector-attribution",
            "context_pack_inspector",
            "truthy",
        ),
    },
}


def find_ley() -> str:
    configured = os.environ.get("LEY_BIN")
    candidates = [
        Path(configured) if configured else None,
        REPO_ROOT / "target" / "debug" / "ley",
        REPO_ROOT / "target" / "release" / "ley",
        Path(shutil.which("ley") or ""),
        Path.home() / ".local" / "bin" / "ley",
    ]
    for candidate in candidates:
        if candidate and candidate.is_file() and os.access(candidate, os.X_OK):
            return str(candidate)
    raise RuntimeError("ley binary not found; build it or set LEY_BIN")


LEY = find_ley()
EVAL_ENV: dict[str, str] = {}


def request_id(seed: str) -> str:
    return "req_" + hashlib.sha256(seed.encode("utf-8")).hexdigest()[:32]


def run(args: list[str], cwd: Path | None = None, stdin: str | None = None) -> str:
    env = os.environ.copy()
    env.update(EVAL_ENV)
    result = subprocess.run(
        [LEY, *args],
        capture_output=True,
        text=True,
        cwd=cwd,
        input=stdin,
        env=env,
    )
    if result.returncode != 0:
        command = "ley " + " ".join(args)
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"{command} failed: {detail}")
    return result.stdout


def git_run(project: Path, args: list[str]) -> str:
    env = os.environ.copy()
    env["LC_ALL"] = "C"
    result = subprocess.run(
        ["git", "-C", str(project), *args],
        capture_output=True,
        text=True,
        env=env,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"git {' '.join(args)} failed: {detail}")
    return result.stdout.strip()


def git_commit_all(project: Path, message: str) -> str:
    # Ley's repository-local identity/policy is lifecycle metadata, not fixture source. Keep it
    # out of synthetic Git history so branch switches cannot make an initialized Ley project
    # disappear merely because one test branch happened to capture `.ley`.
    git_run(project, ["add", "-A", "--", ".", ":(exclude).ley", ":(exclude).ley/**"])
    git_run(
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
    return git_run(project, ["rev-parse", "HEAD"])


def cli_json(args: list[str]) -> object:
    output = run(args)
    try:
        return json.loads(output)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"expected JSON from ley {' '.join(args)}: {output!r}") from error


def write_project_files(project: Path, files: dict[str, str]) -> None:
    for relative, content in files.items():
        path = Path(relative)
        if path.is_absolute() or ".." in path.parts:
            raise RuntimeError(f"fixture contains unsafe project path: {relative}")
        destination = project / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(content, encoding="utf-8")


def init_project(project: Path, name: str, vault: Path) -> None:
    run(["init", str(project), "--name", name, "--json"])
    run(["bind", str(project), "--vault", str(vault), "--json"])
    run(["ingest", str(project), "--json"])


def install_specification_approvals(
    project: Path, vault: Path, specifications: list[dict[str, object]]
) -> None:
    if not specifications:
        return
    diagnostic = cli_json(["doctor", str(project), "--json"])
    if not isinstance(diagnostic, dict):
        raise RuntimeError("doctor returned no project diagnostic for specification fixture")
    identity = diagnostic.get("identity")
    if not isinstance(identity, dict) or not isinstance(identity.get("projectId"), str):
        raise RuntimeError("doctor returned no projectId for specification fixture")
    project_id = str(identity["projectId"])
    registry_path = (
        Path(EVAL_ENV["XDG_CONFIG_HOME"])
        / "app.leynotes.desktop"
        / "specifications-v1.json"
    )
    registry_path.parent.mkdir(parents=True, exist_ok=True)
    if registry_path.exists():
        document = json.loads(registry_path.read_text(encoding="utf-8"))
    else:
        document = {"schemaVersion": 1, "approvals": {}}
    approvals = document.setdefault("approvals", {}).setdefault(project_id, {})
    for index, definition in enumerate(specifications):
        relative_path = str(definition["path"])
        source = str(definition["source"])
        specification_id = str(
            definition.get(
                "specification_id",
                "spec_"
                + hashlib.sha256(
                    f"{project_id}:{relative_path}:{index}".encode("utf-8")
                ).hexdigest()[:32],
            )
        )
        destination = vault / relative_path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(source, encoding="utf-8")
        digest = "sha256:" + hashlib.sha256(source.encode("utf-8")).hexdigest()
        approvals[specification_id] = {
            "relativePath": relative_path,
            "contentHash": digest,
            "approvedAtUnixMs": int(time.time() * 1000) + index,
        }
        definition["resolved_specification_id"] = specification_id
    registry_path.write_text(
        json.dumps(document, sort_keys=True, separators=(",", ":")), encoding="utf-8"
    )
    registry_path.chmod(0o600)


def mcp_call(
    project: Path,
    name: str,
    arguments: dict[str, object],
    flags: tuple[str, ...] = (),
) -> dict[str, object]:
    """Drive one real tools/call through a fresh stdout-clean stdio server."""
    proc = subprocess.Popen(
        [LEY, "mcp", str(project), *flags],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env={**os.environ, **EVAL_ENV},
    )

    responses: list[dict[str, object]] = []

    def send(request: dict[str, object]) -> None:
        if proc.stdin is None:
            raise RuntimeError("MCP stdin was not available")
        proc.stdin.write(json.dumps(request) + "\n")
        proc.stdin.flush()

    def read_until(expected_id: int) -> dict[str, object]:
        deadline = time.monotonic() + 30
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError(
                    f"MCP call {name} timed out waiting for id {expected_id}"
                )
            if proc.stdout is None:
                raise RuntimeError("MCP stdout was not available")
            ready, _, _ = select.select([proc.stdout], [], [], remaining)
            if not ready:
                raise RuntimeError(
                    f"MCP call {name} timed out waiting for id {expected_id}"
                )
            line = proc.stdout.readline()
            if not line:
                raise RuntimeError(
                    f"MCP call {name} returned no result before stdout closed: {responses!r}"
                )
            if not line.strip():
                continue
            try:
                value = json.loads(line)
            except json.JSONDecodeError as error:
                raise RuntimeError(f"MCP wrote non-JSON stdout: {line!r}") from error
            if not isinstance(value, dict):
                raise RuntimeError(f"MCP wrote a non-object JSON message: {value!r}")
            responses.append(value)
            if value.get("id") == expected_id:
                return value

    try:
        send(
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "ley-eval", "version": "1.0"},
                },
            }
        )
        initialize_response = read_until(1)
        if "error" in initialize_response:
            raise RuntimeError(f"MCP initialize returned an error: {initialize_response}")
        send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        send(
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": name, "arguments": arguments},
            }
        )
        response = read_until(2)
        if proc.stdin is not None:
            proc.stdin.close()
        proc.wait(timeout=30)
        if proc.stdout is not None:
            for line in proc.stdout:
                if not line.strip():
                    continue
                try:
                    value = json.loads(line)
                except json.JSONDecodeError as error:
                    raise RuntimeError(f"MCP wrote non-JSON stdout: {line!r}") from error
                if not isinstance(value, dict):
                    raise RuntimeError(f"MCP wrote a non-object JSON message: {value!r}")
                responses.append(value)
        error_output = proc.stderr.read() if proc.stderr is not None else ""
    except (subprocess.TimeoutExpired, BrokenPipeError) as error:
        proc.kill()
        proc.wait()
        raise RuntimeError(f"MCP call {name} did not complete cleanly") from error
    except Exception:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        raise
    if proc.returncode != 0:
        raise RuntimeError(f"MCP call {name} failed: {error_output.strip()}")

    if "result" not in response:
        raise RuntimeError(f"MCP call {name} returned no result: {response!r}")
    result = response["result"]
    if not isinstance(result, dict):
        raise RuntimeError(f"MCP call {name} returned a non-object result")
    content = result.get("content")
    if not isinstance(content, list) or not content:
        raise RuntimeError(f"MCP call {name} returned no content: {result!r}")
    text = content[0].get("text") if isinstance(content[0], dict) else None
    if not isinstance(text, str):
        raise RuntimeError(f"MCP call {name} returned non-text content")
    if result.get("isError"):
        raise RuntimeError(f"MCP call {name} returned an error: {text}")
    try:
        payload = json.loads(text)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"MCP call {name} returned invalid JSON text: {text!r}") from error
    if not isinstance(payload, dict):
        raise RuntimeError(f"MCP call {name} returned a non-object payload")
    return payload


def mcp_tools_list(project: Path, flags: tuple[str, ...] = ()) -> list[dict[str, object]]:
    """Initialize a real stdio MCP server and return its advertised tools."""
    proc = subprocess.Popen(
        [LEY, "mcp", str(project), *flags],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env={**os.environ, **EVAL_ENV},
    )

    def send(request: dict[str, object]) -> None:
        if proc.stdin is None:
            raise RuntimeError("MCP stdin was not available")
        proc.stdin.write(json.dumps(request) + "\n")
        proc.stdin.flush()

    def read_until(expected_id: int) -> dict[str, object]:
        deadline = time.monotonic() + 30
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError(f"MCP tools/list timed out waiting for id {expected_id}")
            if proc.stdout is None:
                raise RuntimeError("MCP stdout was not available")
            ready, _, _ = select.select([proc.stdout], [], [], remaining)
            if not ready:
                raise RuntimeError(f"MCP tools/list timed out waiting for id {expected_id}")
            line = proc.stdout.readline()
            if not line:
                raise RuntimeError("MCP tools/list returned no response before stdout closed")
            if not line.strip():
                continue
            value = json.loads(line)
            if isinstance(value, dict) and value.get("id") == expected_id:
                return value

    try:
        send(
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "ley-eval", "version": "1.0"},
                },
            }
        )
        initialized = read_until(1)
        if "error" in initialized:
            raise RuntimeError(f"MCP initialize returned an error: {initialized}")
        send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        send({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
        response = read_until(2)
        if proc.stdin is not None:
            proc.stdin.close()
        proc.wait(timeout=30)
        stderr = proc.stderr.read() if proc.stderr is not None else ""
    except Exception:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        raise
    if proc.returncode != 0:
        raise RuntimeError(f"MCP tools/list failed: {stderr.strip()}")
    result = response.get("result")
    if not isinstance(result, dict):
        raise RuntimeError("MCP tools/list returned no result object")
    tools = result.get("tools")
    if not isinstance(tools, list):
        raise RuntimeError("MCP tools/list returned no tools array")
    return [tool for tool in tools if isinstance(tool, dict)]


def hook_call(
    project: Path,
    host: str,
    payload: dict[str, str],
    flags: tuple[str, ...] = (),
) -> dict[str, object]:
    output = run(["hook", str(project), "--host", host, *flags], stdin=json.dumps(payload))
    try:
        value = json.loads(output)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"host hook returned invalid JSON: {output!r}") from error
    if not isinstance(value, dict):
        raise RuntimeError("host hook returned a non-object payload")
    return value


def ensure_learning_citations(project: Path, events: list[dict[str, object]]) -> None:
    """Give cited learning fixtures a real initial artifact that can be renamed."""
    for event in events:
        if event.get("type") != "learning":
            continue
        cited = event.get("cited_artifact")
        if isinstance(cited, str) and not (project / cited).exists():
            write_project_files(project, {cited: "def old_name(): pass\n"})


def checkpoint_from_events(events: list[dict[str, object]], artifact_paths: list[str]) -> dict[str, object]:
    summaries: list[str] = []
    decisions: list[dict[str, object]] = []
    problems: list[dict[str, object]] = []
    tasks: list[dict[str, object]] = []
    verification: list[dict[str, object]] = []
    unresolved: list[str] = []
    current_problem: dict[str, object] | None = None
    for event in events:
        kind = event.get("type")
        if kind == "checkpoint":
            summary = str(event.get("summary", "")).strip()
            if summary:
                summaries.append(summary)
        elif kind == "decision":
            title = str(event.get("title", "Decision"))
            decision = str(event.get("decision", ""))
            status = str(event.get("status", "")).strip()
            if status:
                title = f"{title} [{status}]"
                summaries.append(f"Decision state: {title}")
            decisions.append({"title": title, "decision": decision})
        elif kind == "problem":
            current_problem = {
                "title": str(event.get("title", "Problem")),
                "symptom": str(event.get("symptom", "")),
                "attempts": [],
            }
            problems.append(current_problem)
        elif kind == "attempt":
            if current_problem is None:
                current_problem = {"title": "Unattached attempt", "symptom": "", "attempts": []}
                problems.append(current_problem)
            outcome = str(event.get("outcome", "unknown"))
            outcome = {"success": "helped", "failed": "no-effect"}.get(outcome, outcome)
            current_problem["attempts"].append(
                {
                    "action": str(event.get("action", "")),
                    "outcome": outcome,
                    "evidence": str(event.get("evidence", "")),
                }
            )
        elif kind == "resolution":
            if current_problem is None:
                current_problem = {"title": "Resolved problem", "symptom": "", "attempts": []}
                problems.append(current_problem)
            current_problem["resolution"] = {
                "rootCause": str(event.get("root_cause", "Captured from the structured resolution.")),
                "change": str(event.get("solution", event.get("change", ""))),
                "verification": str(event.get("verification", "")),
            }
        elif kind == "learning":
            summaries.append(
                f"Learning proposal: {event.get('title', 'Untitled')} — {event.get('guidance', '')}"
            )
        elif kind == "task":
            tasks.append(
                {
                    "title": str(event.get("title", "Task")),
                    "status": str(event.get("status", "pending")),
                    "details": str(event.get("details", "")),
                }
            )
        elif kind == "verification":
            item: dict[str, object] = {
                "kind": str(event.get("verification_kind", event.get("kind_name", "test"))),
                "status": str(event.get("status", "unknown")),
                "summary": str(event.get("summary", "")),
            }
            if event.get("command") is not None:
                item["command"] = str(event.get("command"))
            verification.append(item)
        elif kind == "unresolved":
            unresolved.append(str(event.get("text", "")))

    checkpoint: dict[str, object] = {
        "summary": ("; ".join(summaries) or "Captured structured project progress.")[:16000],
        "decisions": decisions,
        "problems": problems,
        "tasks": tasks,
        "verification": verification,
        "unresolved": unresolved,
    }
    if artifact_paths:
        checkpoint["touchedArtifacts"] = artifact_paths
    return checkpoint


def capture_events(
    scenario: dict[str, object], project: Path
) -> tuple[str | None, list[dict[str, object]], list[str]]:
    events = scenario.get("session_events", [])
    if not isinstance(events, list) or not events:
        return None, [], []

    # A crash fixture deliberately exercises the host adapter path: the host
    # starts a session and submits a prompt, then never sends Stop.
    if any(event.get("type") == "prompt" for event in events if isinstance(event, dict)):
        external_id = "ley-eval-crash-thread"
        hook_call(project, "codex", {"hook_event_name": "SessionStart", "session_id": external_id})
        prompt_event = next(event for event in events if event.get("type") == "prompt")
        hook_call(
            project,
            "codex",
            {
                "hook_event_name": "UserPromptSubmit",
                "session_id": external_id,
                "turn_id": "ley-eval-crash-turn",
                "prompt": str(prompt_event.get("text", "")),
            },
        )
        sessions = cli_json(["session", "list", str(project), "--json"])
        if not isinstance(sessions, list) or not sessions:
            raise RuntimeError("crash fixture created no host session")
        session_id = str(sessions[0]["sessionId"])
        shown = cli_json(["session", "show", session_id, str(project), "--json"])
        return session_id, [], [json.dumps(sessions[0]), json.dumps(shown)]

    start_receipt = mcp_call(
        project,
        "ley_session_start",
        {
            "requestId": request_id(f"{scenario['id']}:start"),
            "name": str(scenario["goal"])[:128],
            "goal": str(scenario["goal"]),
            "host": "codex",
        },
        WRITE_FLAGS,
    )
    session_id = str(start_receipt["sessionId"])
    receipts: list[dict[str, object]] = []
    artifact_paths = [
        str(event["cited_artifact"])
        for event in events
        if isinstance(event, dict) and isinstance(event.get("cited_artifact"), str)
    ]
    checkpoint_events = [event for event in events if event.get("type") == "checkpoint"]
    structured_events = [
        event
        for event in events
        if event.get("type")
        in {
            "decision",
            "problem",
            "attempt",
            "resolution",
            "learning",
            "task",
            "verification",
            "unresolved",
        }
    ]
    for index, event in enumerate(checkpoint_events):
        rid = str(event.get("request_id") or request_id(f"{scenario['id']}:checkpoint:{index}"))
        args: dict[str, object] = {
            "sessionId": session_id,
            "requestId": rid,
            "summary": str(event.get("summary", "Captured checkpoint")),
        }
        if artifact_paths:
            args["touchedArtifacts"] = artifact_paths
        receipts.append(mcp_call(project, "ley_session_checkpoint", args, WRITE_FLAGS))
    if structured_events:
        args = checkpoint_from_events(structured_events, artifact_paths)
        args.update({"sessionId": session_id, "requestId": request_id(f"{scenario['id']}:structured")})
        receipts.append(mcp_call(project, "ley_session_checkpoint", args, WRITE_FLAGS))
    elif not checkpoint_events:
        args = checkpoint_from_events(events, artifact_paths)
        args.update({"sessionId": session_id, "requestId": request_id(f"{scenario['id']}:fallback")})
        receipts.append(mcp_call(project, "ley_session_checkpoint", args, WRITE_FLAGS))

    evidence_record_id = session_id
    if receipts:
        shown = cli_json(["session", "show", session_id, str(project), "--json"])
        checkpoints = shown.get("checkpoints", []) if isinstance(shown, dict) else []
        if checkpoints:
            evidence_record_id = str(checkpoints[-1]["checkpointId"])
    for index, event in enumerate(events):
        if event.get("type") != "learning":
            continue
        receipts.append(
            mcp_call(
                project,
                "ley_learning_propose",
                {
                    "requestId": request_id(f"{scenario['id']}:learning:{index}"),
                    "kind": str(event.get("kind", "fact")),
                    "title": str(event.get("title", "Untitled learning")),
                    "guidance": str(event.get("guidance", "")),
                    "confidencePercent": int(event.get("confidence_percent", 50)),
                    "provenance": "agent-authored",
                    "evidence": [{"sessionId": session_id, "recordId": evidence_record_id, "note": "Evaluation evidence"}],
                },
                WRITE_FLAGS,
            )
        )
    return session_id, receipts, []


def search_payloads(project: Path, query: str) -> list[dict[str, object]]:
    return [
        mcp_call(project, "ley_search_memory", {"query": query, "maxResults": K, "maxTokens": 500}),
        mcp_call(project, "ley_search_context", {"query": query, "maxResults": K, "maxTokens": 500}),
        mcp_call(project, "ley_search_activity", {"query": query, "maxResults": K}),
    ]


def serialized(values: list[object]) -> str:
    return json.dumps(values, sort_keys=True)


def context_contract_text(payload: dict[str, object]) -> str:
    """Serialize only context bodies that could support a downstream task.

    The task/query itself is deliberately excluded so a fixture cannot satisfy
    its own required marker merely by asking for that marker.
    """
    return json.dumps(
        {
            "specifications": payload.get("specifications", []),
            "items": payload.get("items", []),
            "mountedReferences": payload.get("mountedReferences", []),
        },
        sort_keys=True,
    )


def task_contract_success(
    payload: dict[str, object],
    required: list[str],
    forbidden: list[str],
) -> bool:
    text = context_contract_text(payload).lower()
    return all(marker.lower() in text for marker in required) and all(
        marker.lower() not in text for marker in forbidden
    )


def privacy_violation_rate(canaries: list[str], outputs: list[object]) -> float:
    if not canaries:
        return 0.0
    text = serialized(outputs).lower()
    exposed = sum(canary.lower() in text for canary in canaries)
    return exposed / len(canaries)


def create_structured_session(
    project: Path,
    *,
    seed: str,
    name: str,
    goal: str,
    summary: str,
    decisions: list[dict[str, str]] | None = None,
    host: str = "codex",
) -> tuple[str, dict[str, object]]:
    started = mcp_call(
        project,
        "ley_session_start",
        {
            "requestId": request_id(f"{seed}:start"),
            "name": name,
            "goal": goal,
            "host": host,
        },
        WRITE_FLAGS,
    )
    session_id = str(started["sessionId"])
    checkpoint: dict[str, object] = {
        "sessionId": session_id,
        "requestId": request_id(f"{seed}:checkpoint"),
        "summary": summary,
    }
    if decisions:
        checkpoint["decisions"] = decisions
    receipt = mcp_call(
        project,
        "ley_session_checkpoint",
        checkpoint,
        WRITE_FLAGS,
    )
    return session_id, receipt


def check_citations(payloads: list[dict[str, object]], expected: list[str]) -> float | None:
    if not expected:
        return None
    text = serialized(payloads).lower()
    return sum(item.lower() in text for item in expected) / len(expected)


def check_untrusted(payload: dict[str, object]) -> bool:
    boundary = str(payload.get("sourceBoundary", ""))
    warning = str(payload.get("instructionWarning", ""))
    trusted_flags = [
        item.get("trustedForReuse")
        for item in payload.get("results", [])
        if isinstance(item, dict)
    ]
    return (
        "untrusted" in boundary
        and bool(warning)
        and all(flag is False for flag in trusted_flags)
        and not bool(payload.get("executed"))
        and not bool(payload.get("execution"))
    )


def evaluate_scenario(scenario: dict[str, object], base_dir: Path) -> dict[str, object]:
    failures: list[str] = []
    scores: dict[str, object] = {name: None for name in METRIC_NAMES}

    project = base_dir / "project"
    vault = base_dir / "vault"
    project.mkdir(parents=True)
    vault.mkdir(parents=True)
    files = scenario.get("project_files", {})
    if isinstance(files, dict):
        write_project_files(project, files)
    revision_flow = scenario.get("git_revision_flow")
    if isinstance(revision_flow, dict):
        git_run(project, ["init", "-b", "main"])
        git_commit_all(project, "base")
    large = scenario.get("large_project")
    if isinstance(large, dict):
        count = int(large.get("num_files", 0))
        lines = int(large.get("avg_file_lines", 0))
        for index in range(count):
            body = ["def main(): pass\n" if index == 0 else f"def worker_{index}(): pass\n"]
            body.extend(f"# generated evidence line {line}\n" for line in range(max(0, lines - 1)))
            write_project_files(project, {f"src/module_{index:03d}.py": "".join(body)})
    events = scenario.get("session_events", [])
    if isinstance(events, list):
        ensure_learning_citations(project, [event for event in events if isinstance(event, dict)])

    init_project(project, str(scenario["goal"]), vault)
    specification_definitions = [
        item
        for item in scenario.get("specifications", [])
        if isinstance(item, dict)
    ]
    install_specification_approvals(project, vault, specification_definitions)
    if isinstance(revision_flow, dict):
        branch = str(revision_flow.get("branch", "experiment"))
        git_run(project, ["checkout", "-b", branch])
        experiment_files = revision_flow.get("experiment_files", {})
        if isinstance(experiment_files, dict):
            write_project_files(project, experiment_files)
        git_commit_all(project, "experiment")
        run(["ingest", str(project), "--json"])
    session_id, receipts, host_context = capture_events(scenario, project)
    evidence_text: list[object] = list(host_context)
    if session_id:
        evidence_text.append(cli_json(["session", "show", session_id, str(project), "--json"]))
        evidence_text.append(
            mcp_call(
                project,
                "ley_session_turns_get",
                {"sessionId": session_id, "maxResults": 20, "maxCharacters": 8000},
            )
        )

    premise_expectation = scenario.get("expected_premise_adjudication")
    if isinstance(premise_expectation, dict):
        learning_receipts = [
            receipt
            for receipt in receipts
            if isinstance(receipt, dict) and isinstance(receipt.get("learningId"), str)
        ]
        obsolete_index = int(premise_expectation.get("obsolete_learning_index", 0))
        replacement_index = int(premise_expectation.get("replacement_learning_index", 1))
        if max(obsolete_index, replacement_index) >= len(learning_receipts):
            raise RuntimeError("premise fixture did not create the expected learning proposals")
        obsolete_id = str(learning_receipts[obsolete_index]["learningId"])
        replacement_id = str(learning_receipts[replacement_index]["learningId"])
        cli_json(
            [
                "learning",
                "review",
                replacement_id,
                str(project),
                "--actor",
                "user",
                "--action",
                "confirm",
                "--note",
                "Reviewed current replacement for premise evaluation.",
                "--request-id",
                request_id(f"{scenario['id']}:premise:confirm"),
                "--json",
            ]
        )
        cli_json(
            [
                "learning",
                "review",
                obsolete_id,
                str(project),
                "--actor",
                "user",
                "--action",
                "supersede",
                "--replacement",
                replacement_id,
                "--note",
                "Reviewed replacement supersedes the obsolete state.",
                "--request-id",
                request_id(f"{scenario['id']}:premise:supersede"),
                "--json",
            ]
        )
        query = str(premise_expectation.get("query", ""))
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
        )
        adjudication = compiled.get("premiseAdjudication", {})
        warnings = (
            adjudication.get("warnings", []) if isinstance(adjudication, dict) else []
        )
        expected_state = str(premise_expectation.get("state", "obsolete-assumption"))
        expected_warning = str(
            premise_expectation.get("warning_kind", "superseded-learning")
        )
        warning_ok = any(
            isinstance(warning, dict)
            and warning.get("kind") == expected_warning
            and obsolete_id in warning.get("learningIds", [])
            and warning.get("replacementLearningId") == replacement_id
            for warning in warnings
        )
        follow_up_ok = any(
            isinstance(item, dict)
            and item.get("kind") == "learning"
            and item.get("id") == replacement_id
            for item in compiled.get("followUps", [])
        )
        replacement_admitted = any(
            isinstance(item, dict)
            and item.get("learningId") == replacement_id
            and item.get("trustSignal") == "trusted-current"
            for item in compiled.get("items", [])
        )
        obsolete_withheld = not any(
            isinstance(item, dict) and item.get("learningId") == obsolete_id
            for item in compiled.get("items", [])
        )
        premise_ok = (
            isinstance(adjudication, dict)
            and adjudication.get("state") == expected_state
            and warning_ok
            and follow_up_ok
            and replacement_admitted
            and obsolete_withheld
            and compiled.get("liveSourceChecked") is False
            and int(compiled.get("estimatedTokens", 0)) <= int(compiled.get("maxTokens", 0))
        )
        scores["premise_adjudication"] = premise_ok
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)], [compiled]
        )
        evidence_text.append(compiled)
        if not premise_ok:
            failures.append(
                "premise adjudication did not resist an explicitly superseded task assumption"
            )

    revision_expectation = scenario.get("expected_revision_adjudication")
    if isinstance(revision_expectation, dict):
        if not isinstance(revision_flow, dict):
            raise RuntimeError("revision expectation requires git_revision_flow")
        branch = str(revision_flow.get("branch", "experiment"))
        git_run(project, ["checkout", "main"])
        main_files = revision_flow.get("main_files", {})
        if isinstance(main_files, dict):
            write_project_files(project, main_files)
        git_commit_all(project, "mainline")
        query = str(revision_expectation.get("query", ""))
        divergent = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
        )
        divergent_adjudication = divergent.get("premiseAdjudication", {})
        divergent_warning = (
            any(
                isinstance(item, dict) and item.get("kind") == "divergent-revision"
                for item in divergent_adjudication.get("warnings", [])
            )
            if isinstance(divergent_adjudication, dict)
            else False
        )
        divergent_exclusion = any(
            isinstance(item, dict) and item.get("reason") == "divergent-revision"
            for item in divergent.get("exclusions", [])
        )
        divergent_decision_withheld = not any(
            isinstance(item, dict)
            and item.get("kind") == "decision"
            and query.lower() in json.dumps(item).lower()
            for item in divergent.get("items", [])
        )
        divergent_gap = any(
            isinstance(item, dict) and item.get("kind") == "revision-drift"
            for item in divergent.get("gaps", [])
        )
        divergent_ok = (
            divergent.get("revisionFreshness", {}).get("liveGitChecked") is True
            and divergent.get("revisionFreshness", {}).get("captureCompatibility")
            == "divergent"
            and isinstance(divergent_adjudication, dict)
            and divergent_adjudication.get("state") == "uncertain-state"
            and divergent_warning
            and divergent_exclusion
            and divergent_decision_withheld
            and divergent_gap
            and divergent.get("liveSourceChecked") is False
        )

        git_run(
            project,
            [
                "-c",
                "user.name=Ley Eval",
                "-c",
                "user.email=ley-eval@example.invalid",
                "merge",
                "--no-ff",
                branch,
                "-m",
                "merge experiment",
            ],
        )
        merged = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
        )
        merged_decision = next(
            (
                item
                for item in merged.get("items", [])
                if isinstance(item, dict)
                and item.get("kind") == "decision"
                and query.lower() in json.dumps(item).lower()
            ),
            None,
        )
        merged_ok = (
            merged.get("revisionFreshness", {}).get("captureCompatibility") == "merged"
            and isinstance(merged_decision, dict)
            and merged_decision.get("revisionApplicability", {}).get("compatibility")
            == "merged"
            and not any(
                isinstance(item, dict) and item.get("reason") == "divergent-revision"
                for item in merged.get("exclusions", [])
            )
            and merged.get("liveSourceChecked") is False
        )
        revision_ok = divergent_ok and merged_ok
        scores["revision_adjudication"] = revision_ok
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)], [divergent, merged]
        )
        evidence_text.extend([divergent, merged])
        if not revision_ok:
            failures.append(
                "revision adjudication did not withhold divergent state and re-admit proven merged history"
            )

    specification_expectation = scenario.get("expected_specification_compiler")
    if isinstance(specification_expectation, dict):
        query = str(specification_expectation.get("query", ""))
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
        )
        relevant_index = int(specification_expectation.get("relevant_index", 0))
        unrelated_index = int(specification_expectation.get("unrelated_index", 1))
        relevant_id = str(specification_definitions[relevant_index]["resolved_specification_id"])
        unrelated_id = str(specification_definitions[unrelated_index]["resolved_specification_id"])
        admitted_ids = {
            str(item.get("specificationId"))
            for item in compiled.get("specifications", [])
            if isinstance(item, dict)
        }
        specification_exclusions = [
            item
            for item in compiled.get("specificationExclusions", [])
            if isinstance(item, dict)
        ]
        memory_exclusions = [
            item for item in compiled.get("exclusions", []) if isinstance(item, dict)
        ]
        direct_marker = str(specification_expectation.get("direct_evidence_marker", ""))
        direct_evidence_preserved = any(
            isinstance(item, dict)
            and item.get("kind") == "artifact"
            and direct_marker.lower() in json.dumps(item).lower()
            for item in compiled.get("items", [])
        )
        historical_withheld = any(
            item.get("reason") == "contradicts-human-intent"
            and relevant_id in item.get("specificationIds", [])
            for item in memory_exclusions
        )
        unrelated_omitted = any(
            item.get("specificationId") == unrelated_id
            and item.get("reason") == "low-relevance"
            for item in specification_exclusions
        )
        specification_ok = (
            relevant_id in admitted_ids
            and unrelated_id not in admitted_ids
            and unrelated_omitted
            and historical_withheld
            and direct_evidence_preserved
            and compiled.get("authorityPrecedence") == "human-intent-over-historical-memory"
            and compiled.get("sourceBoundary") == "mixed-authority-context"
        )
        scores["specification_admission"] = specification_ok
        evidence_text.append(compiled)
        if not specification_ok:
            failures.append(
                "task-conditioned Specification admission did not preserve authority/budget/conflict semantics"
            )

    egress_expectation = scenario.get("expected_egress_policy")
    if isinstance(egress_expectation, dict):
        specification_index = int(egress_expectation.get("specification_index", 0))
        if specification_index >= len(specification_definitions):
            raise RuntimeError("egress fixture did not define the expected Specification")
        specification_id = specification_definitions[specification_index].get(
            "resolved_specification_id"
        )
        if not isinstance(specification_id, str):
            raise RuntimeError("egress fixture Specification has no stable ID")
        marker = str(egress_expectation.get("marker", ""))
        derived_marker = str(egress_expectation.get("derived_marker", ""))
        query = str(egress_expectation.get("query", ""))
        if not marker or not derived_marker or not query:
            raise RuntimeError("egress fixture requires marker, derived_marker, and query")

        run(
            [
                "egress",
                "specification",
                specification_id,
                "local-model-only",
                str(project),
                "--json",
            ]
        )
        cloud_direct = mcp_call(
            project,
            "ley_project_specifications",
            {"maxResults": 8, "maxCharacters": 16_000},
        )
        cloud_compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
        )
        cloud_hook = hook_call(
            project,
            "codex",
            {"hook_event_name": "SessionStart", "session_id": "egress-cloud-startup"},
        )
        cloud_blocked = (
            marker not in json.dumps(cloud_direct, sort_keys=True)
            and marker not in json.dumps(cloud_compiled, sort_keys=True)
            and derived_marker not in json.dumps(cloud_compiled, sort_keys=True)
            and derived_marker not in json.dumps(cloud_hook, sort_keys=True)
            and "withheld" in json.dumps(cloud_hook, sort_keys=True).lower()
            and cloud_direct.get("egressTarget") == "cloud"
            and isinstance(cloud_direct.get("egressCoverage"), dict)
            and cloud_direct["egressCoverage"].get("blockedSpecifications") == 1
            and cloud_compiled.get("egressTarget") == "cloud"
            and isinstance(cloud_compiled.get("egressCoverage"), dict)
            and cloud_compiled["egressCoverage"].get("blockedSpecifications") == 1
            and cloud_compiled["egressCoverage"].get("historicalMemoryWithheld") is True
            and int(cloud_compiled["egressCoverage"].get("withheldDerivedResults", 0)) >= 1
        )

        local_flags = ("--egress-target", "local")
        local_direct = mcp_call(
            project,
            "ley_project_specifications",
            {"maxResults": 8, "maxCharacters": 16_000},
            flags=local_flags,
        )
        local_compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
            flags=local_flags,
        )
        local_hook = hook_call(
            project,
            "codex",
            {"hook_event_name": "SessionStart", "session_id": "egress-local-startup"},
            flags=local_flags,
        )
        local_allowed = (
            marker in json.dumps(local_direct, sort_keys=True)
            and marker in json.dumps(local_compiled, sort_keys=True)
            and derived_marker in json.dumps(local_compiled, sort_keys=True)
            and derived_marker in json.dumps(local_hook, sort_keys=True)
            and local_direct.get("egressTarget") == "local"
            and local_compiled.get("egressTarget") == "local"
        )

        run(
            [
                "egress",
                "specification",
                specification_id,
                "confirm-per-use",
                str(project),
                "--json",
            ]
        )
        confirm_direct = mcp_call(
            project,
            "ley_project_specifications",
            {"maxResults": 8, "maxCharacters": 16_000},
            flags=local_flags,
        )
        confirm_compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
            flags=local_flags,
        )
        confirm_hook = hook_call(
            project,
            "codex",
            {"hook_event_name": "SessionStart", "session_id": "egress-confirm-startup"},
            flags=local_flags,
        )
        confirm_blocked = marker not in json.dumps(
            confirm_direct, sort_keys=True
        ) and marker not in json.dumps(confirm_compiled, sort_keys=True) and derived_marker not in json.dumps(
            confirm_hook, sort_keys=True
        )

        run(
            [
                "egress",
                "specification",
                specification_id,
                "never-send",
                str(project),
                "--json",
            ]
        )
        never_direct = mcp_call(
            project,
            "ley_project_specifications",
            {"maxResults": 8, "maxCharacters": 16_000},
            flags=local_flags,
        )
        never_compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
            flags=local_flags,
        )
        never_hook = hook_call(
            project,
            "codex",
            {"hook_event_name": "SessionStart", "session_id": "egress-never-startup"},
            flags=local_flags,
        )
        never_blocked = marker not in json.dumps(
            never_direct, sort_keys=True
        ) and marker not in json.dumps(never_compiled, sort_keys=True) and derived_marker not in json.dumps(
            never_hook, sort_keys=True
        )

        disallowed_probe_groups = [
            [cloud_direct, cloud_compiled, cloud_hook],
            [confirm_direct, confirm_compiled, confirm_hook],
            [never_direct, never_compiled, never_hook],
        ]
        policy_violations = sum(
            marker in serialized(group) or derived_marker in serialized(group)
            for group in disallowed_probe_groups
        )
        scores["privacy_violation_rate"] = policy_violations / len(
            disallowed_probe_groups
        )
        egress_ok = cloud_blocked and local_allowed and confirm_blocked and never_blocked
        scores["egress_policy"] = egress_ok
        evidence_text.extend(
            [
                cloud_direct,
                cloud_compiled,
                cloud_hook,
                local_direct,
                local_compiled,
                local_hook,
                confirm_direct,
                confirm_compiled,
                confirm_hook,
                never_direct,
                never_compiled,
                never_hook,
            ]
        )
        if not egress_ok:
            failures.append(
                "agent egress policy leaked or incorrectly blocked Specification context"
            )

    mounted_definitions = [
        item for item in scenario.get("mounted_projects", []) if isinstance(item, dict)
    ]
    mounted_expectation = scenario.get("expected_mounted_reference_compiler")
    if isinstance(mounted_expectation, dict):
        mounted_projects: list[tuple[Path, Path]] = []
        for index, definition in enumerate(mounted_definitions):
            mounted_project = base_dir / f"mounted-project-{index}"
            mounted_vault = base_dir / f"mounted-vault-{index}"
            mounted_project.mkdir()
            mounted_vault.mkdir()
            write_project_files(mounted_project, definition.get("files", {}))
            init_project(mounted_project, str(definition["name"]), mounted_vault)
            mounted_projects.append((mounted_project, mounted_vault))

        query = str(mounted_expectation.get("query", ""))
        before = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 2_000},
        )
        mount_index = int(mounted_expectation.get("mount_index", 0))
        unmounted_index = int(mounted_expectation.get("unmounted_index", 1))
        mounted_project, _ = mounted_projects[mount_index]
        mount_receipt = cli_json(
            ["mount", "add", str(mounted_project), str(project), "--json"]
        )
        if not isinstance(mount_receipt, dict) or not isinstance(
            mount_receipt.get("mount"), dict
        ):
            raise RuntimeError("mount add returned no mount receipt")
        mount = mount_receipt["mount"]
        mount_id = str(mount.get("mountId", ""))
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 2_000},
        )
        references = [
            item for item in compiled.get("mountedReferences", []) if isinstance(item, dict)
        ]
        scopes = [
            item for item in compiled.get("mountedReferenceScopes", []) if isinstance(item, dict)
        ]
        mounted_marker = str(mounted_expectation.get("mounted_marker", ""))
        unmounted_marker = str(mounted_expectation.get("unmounted_marker", ""))
        serialized_compiled = json.dumps(compiled, sort_keys=True)
        mounted_visible = any(
            item.get("mountId") == mount_id
            and item.get("authority") == "mounted-reference"
            and item.get("sourceBoundary") == "untrusted-mounted-project-memory"
            and mounted_marker.lower() in json.dumps(item).lower()
            for item in references
        )
        scope_visible = any(
            item.get("mountId") == mount_id and item.get("state") == "ready"
            for item in scopes
        )
        no_paths = all(
            str(path) not in serialized_compiled
            for pair in mounted_projects
            for path in pair
        ) and str(project) not in serialized_compiled and str(vault) not in serialized_compiled
        unrelated_hidden = not unmounted_marker or unmounted_marker.lower() not in serialized_compiled.lower()
        before_clean = not before.get("mountedReferenceScopes") and not before.get("mountedReferences")
        run(["mount", "remove", mount_id, str(project), "--json"])
        after = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 2_000},
        )
        after_clean = not after.get("mountedReferenceScopes") and not after.get("mountedReferences")
        mounted_ok = (
            before_clean
            and mount.get("agentContextEnabled") is True
            and mounted_visible
            and scope_visible
            and unrelated_hidden
            and no_paths
            and compiled.get("referencePrecedence") == "active-project-over-mounted-reference"
            and int(compiled.get("estimatedTokens", 0)) <= int(compiled.get("maxTokens", 0))
            and after_clean
        )
        scores["mounted_reference"] = mounted_ok
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)]
            + [str(path) for pair in mounted_projects for path in pair],
            [compiled],
        )
        evidence_text.extend([before, compiled, after])
        if not mounted_ok:
            failures.append(
                "explicit Context Mount did not preserve authorization/isolation/budget/unmount semantics"
            )

    dossier_expectation = scenario.get("expected_topic_dossier")
    if isinstance(dossier_expectation, dict):
        topic = str(dossier_expectation.get("topic", ""))
        max_tokens = int(dossier_expectation.get("max_tokens", 2_000))
        arguments = {
            "topic": topic,
            "maxResults": int(dossier_expectation.get("max_results", 12)),
            "maxTokens": max_tokens,
            "maxSupportingSessions": int(
                dossier_expectation.get("max_supporting_sessions", 6)
            ),
        }
        dossier = mcp_call(project, "ley_topic_dossier", arguments)
        rebuilt = mcp_call(project, "ley_topic_dossier", arguments)
        dossier_text = json.dumps(dossier, sort_keys=True)
        evidence_markers = [
            str(value) for value in dossier_expectation.get("evidence_markers", [])
        ]
        open_markers = [
            str(value) for value in dossier_expectation.get("open_markers", [])
        ]
        verification_marker = str(
            dossier_expectation.get("verification_marker", "")
        )
        artifact_path = str(dossier_expectation.get("artifact_path", ""))
        dossier_ok = (
            dossier.get("schemaVersion") == 1
            and dossier.get("persisted") is False
            and dossier.get("projection") == "on-demand-rebuildable-topic-dossier"
            and str(dossier.get("sourceFingerprint", "")).startswith("sha256:")
            and dossier.get("sourceFingerprint") == rebuilt.get("sourceFingerprint")
            and dossier.get("liveSourceChecked") is False
            and int(dossier.get("estimatedTokens", 0)) <= max_tokens
            and all(marker in dossier_text for marker in evidence_markers)
            and all(
                marker in json.dumps(dossier.get("openItems", []), sort_keys=True)
                for marker in open_markers
            )
            and (
                not verification_marker
                or verification_marker
                in json.dumps(dossier.get("recentVerification", []), sort_keys=True)
            )
            and (
                not artifact_path
                or any(
                    isinstance(item, dict)
                    and item.get("artifactPath") == artifact_path
                    for item in dossier.get("importantArtifacts", [])
                )
            )
            and bool(dossier.get("supportingSessions"))
            and str(project) not in dossier_text
            and str(vault) not in dossier_text
        )
        scores["topic_dossier"] = dossier_ok
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)], [dossier]
        )
        evidence_text.extend([dossier, rebuilt])
        if not dossier_ok:
            failures.append(
                "topic dossier did not preserve deterministic source binding, evidence structure, privacy, or budget"
            )

    current_state_expectation = scenario.get("expected_current_project_state")
    if isinstance(current_state_expectation, dict):
        arguments = {
            "maxSessions": int(current_state_expectation.get("max_sessions", 5)),
            "maxKnowledge": int(current_state_expectation.get("max_knowledge", 12)),
            "maxCharacters": int(current_state_expectation.get("max_characters", 16_000)),
        }
        state = mcp_call(project, "ley_project_state", arguments)
        rebuilt = mcp_call(project, "ley_project_state", arguments)
        state_text = json.dumps(state, sort_keys=True)
        working_marker = str(current_state_expectation.get("working_marker", ""))
        open_markers = [
            str(value) for value in current_state_expectation.get("open_markers", [])
        ]
        decision_marker = str(current_state_expectation.get("decision_marker", ""))
        verification_marker = str(
            current_state_expectation.get("verification_marker", "")
        )
        decision_rows = state.get("recentDecisions", [])
        matching_decision = next(
            (
                item
                for item in decision_rows
                if isinstance(item, dict)
                and (not decision_marker or decision_marker in json.dumps(item, sort_keys=True))
            ),
            None,
        )
        state_ok = (
            state.get("schemaVersion") == 1
            and state.get("persisted") is False
            and state.get("projection") == "on-demand-current-project-state"
            and str(state.get("stateFingerprint", "")).startswith("sha256:")
            and state.get("stateFingerprint") == rebuilt.get("stateFingerprint")
            and state.get("liveSourceChecked") is False
            and bool(state.get("workingSessions"))
            and (not working_marker or working_marker in state_text)
            and all(
                marker in json.dumps(state.get("openWork", []), sort_keys=True)
                for marker in open_markers
            )
            and matching_decision is not None
            and matching_decision.get("authority") == "historical-project-memory"
            and matching_decision.get("currentStateProven") is False
            and (
                not verification_marker
                or verification_marker
                in json.dumps(state.get("recentVerification", []), sort_keys=True)
            )
            and str(project) not in state_text
            and str(vault) not in state_text
        )
        scores["current_project_state"] = state_ok
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)], [state]
        )
        evidence_text.extend([state, rebuilt])
        if not state_ok:
            failures.append(
                "Current Project State did not preserve working-state boundaries, historical decision semantics, privacy, or source binding"
            )

    inspector_expectation = scenario.get("expected_context_pack_inspector")
    if isinstance(inspector_expectation, dict):
        task = str(inspector_expectation.get("task", ""))
        max_results = int(inspector_expectation.get("max_results", 8))
        max_tokens = int(inspector_expectation.get("max_tokens", 1_500))
        hidden_marker = str(inspector_expectation.get("hidden_marker", ""))
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {
                "task": task,
                "maxResults": max_results,
                "maxTokens": max_tokens,
            },
        )
        pack_id = str(compiled.get("contextPackId", ""))
        inspection = mcp_call(
            project,
            "ley_context_pack_inspect",
            {
                "task": task,
                "maxResults": max_results,
                "maxTokens": max_tokens,
                "expectedContextPackId": pack_id,
            },
        )
        mismatch = mcp_call(
            project,
            "ley_context_pack_inspect",
            {
                "task": task,
                "maxResults": max_results,
                "maxTokens": max_tokens,
                "expectedContextPackId": "cpk_" + ("0" * 64),
            },
        )
        compiled_text = json.dumps(compiled, sort_keys=True)
        inspection_text = json.dumps(inspection, sort_keys=True)
        inspector_ok = (
            pack_id.startswith("cpk_")
            and len(pack_id) == 68
            and int(compiled.get("createdAtUnixMs", 0)) > 0
            and compiled.get("liveSourceChecked") is False
            and inspection.get("schemaVersion") == 1
            and inspection.get("persisted") is False
            and inspection.get("inspectionBasis")
            == "current-recompiled-context-pack-manifest"
            and inspection.get("contextPackId") == pack_id
            and inspection.get("matchesExpectedContextPack") is True
            and not inspection.get("mismatchWarning")
            and bool(inspection.get("includedRecords"))
            and inspection.get("budget", {}).get("maxTokens") == max_tokens
            and inspection.get("budget", {}).get("estimatedTokens")
            == compiled.get("estimatedTokens")
            and inspection.get("liveSourceChecked") is False
            and (
                not hidden_marker
                or (
                    hidden_marker in compiled_text
                    and hidden_marker not in inspection_text
                )
            )
            and mismatch.get("matchesExpectedContextPack") is False
            and "does not match" in str(mismatch.get("mismatchWarning", ""))
            and str(project) not in inspection_text
            and str(vault) not in inspection_text
        )
        scores["context_pack_inspector"] = inspector_ok
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)], [inspection, mismatch]
        )
        evidence_text.extend([compiled, inspection, mismatch])
        if not inspector_ok:
            failures.append(
                "Context Pack Inspector did not preserve pack identity, attribution, body omission, mismatch honesty, or privacy"
            )

    abstention_expectation = scenario.get("expected_selective_abstention")
    if isinstance(abstention_expectation, dict):
        query = str(abstention_expectation.get("query", ""))
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {
                "task": query,
                "maxResults": int(abstention_expectation.get("max_results", 8)),
                "maxTokens": int(abstention_expectation.get("max_tokens", 1_500)),
            },
        )
        expected_state = str(
            abstention_expectation.get("evidence_state", "no-useful-evidence")
        )
        abstained = (
            compiled.get("evidenceState") == expected_state
            and not compiled.get("items")
            and not compiled.get("specifications")
            and not compiled.get("mountedReferences")
            and compiled.get("liveSourceChecked") is False
            and int(compiled.get("estimatedTokens", 0))
            <= int(compiled.get("maxTokens", 0))
        )
        scores["selective_abstention"] = abstained
        evidence_text.append(compiled)
        if not abstained:
            failures.append(
                "Context Compiler padded a no-useful-memory task instead of abstaining"
            )

    parallel_expectation = scenario.get("expected_parallel_session_separation")
    if isinstance(parallel_expectation, dict):
        title = str(parallel_expectation.get("title", "Parallel architecture decision"))
        marker_a = str(parallel_expectation.get("marker_a", "parallel_agent_a_marker"))
        marker_b = str(parallel_expectation.get("marker_b", "parallel_agent_b_marker"))
        session_a, _ = create_structured_session(
            project,
            seed=f"{scenario['id']}:parallel:a",
            name="Parallel agent A",
            goal="Keep agent A work isolated",
            summary=f"Agent A recorded {marker_a}.",
            decisions=[
                {
                    "title": title,
                    "decision": f"Agent A chose the A path: {marker_a}.",
                    "rationale": "Independent workstream A.",
                }
            ],
            host="codex",
        )
        session_b, _ = create_structured_session(
            project,
            seed=f"{scenario['id']}:parallel:b",
            name="Parallel agent B",
            goal="Keep agent B work isolated",
            summary=f"Agent B recorded {marker_b}.",
            decisions=[
                {
                    "title": title,
                    "decision": f"Agent B chose the B path: {marker_b}.",
                    "rationale": "Independent workstream B.",
                }
            ],
            host="claude-code",
        )
        context_a = mcp_call(
            project,
            "ley_session_get",
            {"sessionId": session_a, "maxCheckpoints": 5, "maxCharacters": 8_000},
        )
        context_b = mcp_call(
            project,
            "ley_session_get",
            {"sessionId": session_b, "maxCheckpoints": 5, "maxCharacters": 8_000},
        )
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": title, "maxResults": 8, "maxTokens": 1_500},
        )
        text_a = json.dumps(context_a, sort_keys=True)
        text_b = json.dumps(context_b, sort_keys=True)
        adjudication = compiled.get("premiseAdjudication", {})
        separated = (
            session_a != session_b
            and marker_a in text_a
            and marker_b not in text_a
            and marker_b in text_b
            and marker_a not in text_b
            and isinstance(adjudication, dict)
            and adjudication.get("state") == "conflicting-state"
            and not any(
                isinstance(item, dict)
                and item.get("kind") == "decision"
                and (marker_a in json.dumps(item) or marker_b in json.dumps(item))
                for item in compiled.get("items", [])
            )
        )
        scores["parallel_session_separation"] = separated
        evidence_text.extend([context_a, context_b, compiled])
        if not separated:
            failures.append(
                "parallel sessions were merged or conflicting decisions were promoted as current"
            )

    deletion_expectation = scenario.get("expected_deletion_fidelity")
    if isinstance(deletion_expectation, dict):
        marker = str(
            deletion_expectation.get("marker", "deleted_private_memory_canary")
        )
        session_name = "Deletion fidelity session"
        erased_session, _ = create_structured_session(
            project,
            seed=f"{scenario['id']}:erase",
            name=session_name,
            goal=f"Private deletion test {marker}",
            summary=f"Private memory scheduled for deletion: {marker}.",
            decisions=[
                {
                    "title": "Private deletion decision",
                    "decision": f"Sensitive session-only decision {marker}.",
                    "rationale": "Deletion fidelity fixture.",
                }
            ],
        )
        shown = cli_json(["session", "show", erased_session, str(project), "--json"])
        checkpoints = shown.get("checkpoints", []) if isinstance(shown, dict) else []
        if not checkpoints:
            raise RuntimeError("deletion fixture created no checkpoint")
        checkpoint_id = str(checkpoints[-1]["checkpointId"])
        proposed = mcp_call(
            project,
            "ley_learning_propose",
            {
                "requestId": request_id(f"{scenario['id']}:erase:learning"),
                "kind": "fact",
                "title": "Private deletion learning",
                "guidance": f"Dependent learning carrying {marker}.",
                "confidencePercent": 50,
                "provenance": "agent-authored",
                "evidence": [
                    {
                        "sessionId": erased_session,
                        "recordId": checkpoint_id,
                        "note": "Deletion fidelity evidence.",
                    }
                ],
            },
            WRITE_FLAGS,
        )
        learning_id = str(proposed["learningId"])
        event_count = int(shown["eventCount"])
        erased = cli_json(
            [
                "session",
                "erase",
                erased_session,
                str(project),
                "--confirm-name",
                session_name,
                "--expected-events",
                str(event_count),
                "--json",
            ]
        )
        probes: list[object] = [
            mcp_call(
                project,
                "ley_search_memory",
                {"query": "private deletion memory", "maxResults": 20, "maxTokens": 1_000},
            ),
            mcp_call(
                project,
                "ley_search_context",
                {"query": "private deletion memory", "maxResults": 20, "maxTokens": 1_000},
            ),
            mcp_call(
                project,
                "ley_search_activity",
                {"query": "private deletion memory", "maxResults": 20},
            ),
            mcp_call(
                project,
                "ley_project_resume",
                {"maxSessions": 10, "maxLearnings": 20, "maxCharacters": 8_000},
            ),
            mcp_call(
                project,
                "ley_learnings_list",
                {"scope": "all", "maxResults": 50},
            ),
            mcp_call(
                project,
                "ley_topic_dossier",
                {
                    "topic": "private deletion memory",
                    "maxResults": 12,
                    "maxTokens": 2_000,
                    "maxSupportingSessions": 6,
                },
            ),
            mcp_call(
                project,
                "ley_project_state",
                {
                    "maxSessions": 10,
                    "maxKnowledge": 20,
                    "maxCharacters": 8_000,
                },
            ),
            cli_json(["session", "list", str(project), "--json"]),
        ]
        vault_text = "\n".join(
            path.read_text(errors="ignore")
            for path in vault.rglob("*")
            if path.is_file()
        )
        residue_surfaces = [
            marker in json.dumps(probe, sort_keys=True) for probe in probes
        ] + [marker in vault_text]
        residue_rate = sum(residue_surfaces) / len(residue_surfaces)
        erased_learning_ids = (
            erased.get("erasedLearningIds", []) if isinstance(erased, dict) else []
        )
        erased_ids = {
            str(item.get("sessionId"))
            for item in probes[-1]
            if isinstance(item, dict) and item.get("sessionId")
        } if isinstance(probes[-1], list) else set()
        deletion_ok = (
            residue_rate == 0.0
            and learning_id in erased_learning_ids
            and erased_session not in erased_ids
        )
        scores["deletion_fidelity"] = deletion_ok
        scores["forgetting_residue_rate"] = residue_rate
        scores["privacy_violation_rate"] = residue_rate
        evidence_text.extend(probes)
        if not deletion_ok:
            failures.append(
                f"session erasure left Ley-managed residue; residue rate={residue_rate:.3f}"
            )

    inactive_expectation = scenario.get("expected_inactive_workspace")
    if isinstance(inactive_expectation, dict):
        inactive = base_dir / "inactive-workspace"
        inactive.mkdir()
        config_root = Path(EVAL_ENV["XDG_CONFIG_HOME"])
        before_config = {
            str(path.relative_to(config_root))
            for path in config_root.rglob("*")
            if path.is_file()
        } if config_root.exists() else set()
        hook = hook_call(
            inactive,
            "codex",
            {"hook_event_name": "SessionStart", "session_id": "inactive-workspace-thread"},
        )
        tools = mcp_tools_list(inactive)
        after_config = {
            str(path.relative_to(config_root))
            for path in config_root.rglob("*")
            if path.is_file()
        } if config_root.exists() else set()
        inactive_clean = (
            hook == {}
            and tools == []
            and not (inactive / ".ley").exists()
            and list(inactive.iterdir()) == []
            and before_config == after_config
        )
        scores["inactive_workspace_clean"] = inactive_clean
        if not inactive_clean:
            failures.append(
                "globally installed Ley mutated or exposed capabilities in an inactive workspace"
            )

    portability_expectation = scenario.get("expected_host_portability")
    if isinstance(portability_expectation, dict):
        marker = str(
            portability_expectation.get("marker", "portable_host_memory_marker")
        )
        create_structured_session(
            project,
            seed=f"{scenario['id']}:portable",
            name="Portable prior work",
            goal="Preserve durable knowledge across hosts",
            summary=f"Durable cross-host handoff: {marker}.",
            decisions=[],
            host="codex",
        )
        codex = hook_call(
            project,
            "codex",
            {"hook_event_name": "SessionStart", "session_id": "portable-codex-thread"},
        )
        claude = hook_call(
            project,
            "claude",
            {"hook_event_name": "SessionStart", "session_id": "portable-claude-thread"},
        )
        codex_text = json.dumps(codex, sort_keys=True)
        claude_text = json.dumps(claude, sort_keys=True)
        portable = marker in codex_text and marker in claude_text
        scores["host_portability"] = portable
        evidence_text.extend([codex, claude])
        if not portable:
            failures.append(
                "durable Ley context was not usable from both Codex and Claude lifecycle hosts"
            )

    baseline_expectation = scenario.get("expected_budget_baseline")
    if isinstance(baseline_expectation, dict):
        required = [str(value) for value in baseline_expectation.get("required", [])]
        forbidden = [str(value) for value in baseline_expectation.get("forbidden", [])]
        query = str(baseline_expectation.get("query", ""))
        max_tokens = int(baseline_expectation.get("max_tokens", 500))
        resume_characters = int(
            baseline_expectation.get("resume_max_characters", max_tokens * 4)
        )
        create_structured_session(
            project,
            seed=f"{scenario['id']}:relevant",
            name="Older relevant database work",
            goal="Choose the durable database migration strategy",
            summary=(
                "Database migration strategy selected SQLite WAL mode; "
                + " ".join(required)
            ),
            decisions=[
                {
                    "title": "Database migration strategy",
                    "decision": "Use SQLite in WAL mode for durable local migration. "
                    + " ".join(required),
                    "rationale": "Relevant prior experience for this exact task.",
                }
            ],
        )
        distractors = [
            "UI spacing cleanup",
            "Icon export cleanup",
            "Landing page copy",
            "Theme preference cleanup",
        ]
        for index, distractor in enumerate(distractors):
            time.sleep(0.003)
            create_structured_session(
                project,
                seed=f"{scenario['id']}:distractor:{index}",
                name=distractor,
                goal=distractor,
                summary=f"{distractor}: baseline_distractor_{index}.",
                decisions=[],
            )
        compiler = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": max_tokens},
        )
        resume = mcp_call(
            project,
            "ley_project_resume",
            {
                "maxSessions": 3,
                "maxLearnings": 1,
                "maxCharacters": resume_characters,
            },
        )
        compiler_success = (
            task_contract_success(compiler, required, forbidden)
            and int(compiler.get("estimatedTokens", 0)) <= max_tokens
        )
        resume_text = json.dumps(
            {
                "sessions": resume.get("sessions", []),
                "learnings": resume.get("learnings", []),
            },
            sort_keys=True,
        ).lower()
        baseline_success = all(marker.lower() in resume_text for marker in required) and all(
            marker.lower() not in resume_text for marker in forbidden
        )
        scores["downstream_task_contract"] = compiler_success
        scores["budget_baseline_advantage"] = compiler_success and not baseline_success
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)], [compiler, resume]
        )
        evidence_text.extend([compiler, resume])
        if not compiler_success:
            failures.append(
                "500-token Context Compiler failed the deterministic downstream evidence contract"
            )
        if baseline_success:
            failures.append(
                "bounded recent-resume baseline unexpectedly satisfied the older task-specific evidence contract"
            )

    if scenario.get("expected_event_count") is not None:
        if not session_id:
            failures.append("idempotency fixture created no session")
        else:
            shown = cli_json(["session", "show", session_id, str(project), "--json"])
            checkpoint_count = shown.get("checkpointCount") if isinstance(shown, dict) else None
            scores["idempotency"] = checkpoint_count == int(scenario["expected_event_count"])
            if not scores["idempotency"]:
                failures.append(f"expected {scenario['expected_event_count']} checkpoint(s), got {checkpoint_count}")
            if len(receipts) >= 2 and not receipts[-1].get("replayed"):
                failures.append("exact retry did not report replayed=true")

    projects = scenario.get("projects", [])
    if isinstance(projects, list) and len(projects) >= 2:
        project_dirs: list[Path] = []
        for definition in projects:
            name = str(definition["name"])
            project_dir = base_dir / ("project-" + name.lower().replace(" ", "-"))
            vault_dir = base_dir / ("vault-" + name.lower().replace(" ", "-"))
            project_dir.mkdir()
            vault_dir.mkdir()
            write_project_files(project_dir, definition.get("files", {}))
            init_project(project_dir, name, vault_dir)
            project_dirs.append(project_dir)
        query = str(scenario.get("query_from_alpha", ["cross project"])[0])
        payload = mcp_call(project_dirs[0], "ley_search_memory", {"query": query, "maxResults": K, "maxTokens": 500})
        other = projects[1]
        markers = [str(other["name"])] + [str(path) for path in other.get("files", {})] + [str(value) for value in other.get("files", {}).values()]
        returned = serialized([payload]).lower()
        exposed = sum(marker.lower() in returned for marker in markers)
        leak = exposed > 0
        scores["privacy_violation_rate"] = exposed / len(markers) if markers else 0.0
        scores["cross_project_clean"] = not leak
        if leak:
            failures.append("cross-project search returned another project's marker")

    if scenario.get("source_changed"):
        for relative in scenario.get("deleted_artifacts", []):
            target = project / str(relative)
            if target.exists():
                target.unlink()
        run(["ingest", str(project), "--json"])

    all_payloads: list[dict[str, object]] = []
    for query_def in scenario.get("queries", []):
        query = query_def if isinstance(query_def, str) else str(query_def.get("query", ""))
        all_payloads.extend(search_payloads(project, query))
    evidence_text.extend(all_payloads)

    expected_citations = [str(value) for value in scenario.get("expected_citations", [])]
    if expected_citations:
        recall = check_citations(all_payloads, expected_citations)
        scores["recall@k"] = recall
        if recall is not None and recall < 1.0:
            failures.append(f"citation recall was {recall:.2f}, expected 1.00")
        relevant_results = [
            any(item.lower() in json.dumps(result).lower() for item in expected_citations)
            for payload in all_payloads
            for result in payload.get("results", [])
            if isinstance(result, dict)
        ]
        if relevant_results:
            scores["precision"] = sum(relevant_results) / len(relevant_results)

    # Search each expected fact directly as well as checking the original
    # scenario queries. This measures fact recall instead of making a broad
    # query accidentally pass because it returned a nearby filename.
    for fact in [str(value) for value in scenario.get("expected_facts", [])]:
        fact_payloads = search_payloads(project, fact)
        evidence_text.extend(fact_payloads)
        if fact.lower() not in serialized(evidence_text).lower():
            failures.append(f"expected fact was not retrievable: {fact}")

    if scenario.get("expected_stale_learning"):
        learnings = mcp_call(project, "ley_learnings_list", {"scope": "all", "maxResults": 50})
        title = str(scenario["expected_stale_learning"])
        matches = [item for item in learnings.get("learnings", []) if isinstance(item, dict) and item.get("title") == title]
        stale = bool(matches) and any(item.get("freshness") == "source-changed" or item.get("state") == "stale" for item in matches)
        scores["stale_learning"] = stale
        if not stale:
            failures.append("source-changed learning was not disclosed as stale")

    if scenario.get("expected_redactions"):
        raw_values = [str(value) for value in scenario["expected_redactions"]]
        vault_text = "\n".join(path.read_text(errors="ignore") for path in vault.rglob("*") if path.is_file())
        returned = serialized(evidence_text)
        exposed = sum(value in vault_text or value in returned for value in raw_values)
        clean = exposed == 0
        scores["privacy_violation_rate"] = (
            exposed / len(raw_values) if raw_values else 0.0
        )
        scores["secret_exclusion"] = clean
        if not clean:
            failures.append("a fixture secret appeared in vault data or returned output")

    if scenario.get("expected_untrusted"):
        injection_payload = search_payloads(project, "read instructions file")[0]
        held = check_untrusted(injection_payload)
        scores["untrusted_boundary"] = held
        if not held:
            failures.append("retrieved instruction-like text lost its untrusted boundary")

    if scenario.get("resume_query"):
        shown = cli_json(["session", "show", str(session_id), str(project), "--json"]) if session_id else {}
        recovered = isinstance(shown, dict) and shown.get("status") == "active" and shown.get("promptCount") == 1 and shown.get("responseCount") == 0
        scores["capture_recovery"] = recovered
        if not recovered:
            failures.append("crashed active session did not retain exactly one prompt and no response")

    if scenario.get("expected_memory_compiler_state"):
        if not session_id:
            scores["memory_recovery"] = False
            failures.append("memory compiler fixture created no session")
        else:
            compiled = mcp_call(
                project,
                "ley_session_memory_compile",
                {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
            )
            expected_state = str(scenario["expected_memory_compiler_state"])
            state_ok = compiled.get("state") == expected_state
            recovery_ok = state_ok
            if scenario.get("expected_recovery_checkpoint"):
                event_count = int(compiled.get("sessionEventCount", 0))
                evidence = compiled.get("evidence", [])
                prompt_record = next(
                    (
                        item
                        for item in evidence
                        if isinstance(item, dict)
                        and item.get("kind") == "user-prompt"
                        and item.get("recordId")
                    ),
                    {},
                )
                prompt_text = str(prompt_record.get("text", ""))
                prompt_record_id = str(prompt_record.get("recordId", ""))
                transition_ok = True
                transition = {}
                if scenario.get("expected_memory_transition_state"):
                    transition = mcp_call(
                        project,
                        "ley_session_memory_verify",
                        {
                            "sessionId": session_id,
                            "expectedEventCount": event_count,
                            "claims": [
                                {
                                    "kind": "unresolved",
                                    "subject": "Interrupted request",
                                    "statement": prompt_text or "A request was observed before interruption",
                                    "evidenceRecordIds": [prompt_record_id],
                                }
                            ],
                            "deferredEvidenceRecordIds": [],
                        },
                    )
                    expected_transition = str(scenario["expected_memory_transition_state"])
                    transition_ok = (
                        transition.get("state") == expected_transition
                        and transition.get("actualEventCount") == event_count
                        and transition.get("semanticFaithfulnessProven") is False
                        and transition.get("liveSourceChecked") is False
                        and "untrusted" in str(transition.get("sourceBoundary", ""))
                        and str(transition.get("candidateFingerprint", "")).startswith("sha256:")
                        and bool(transition.get("coverage", {}).get("coverageComplete"))
                    )
                    scores["memory_transition"] = transition_ok
                    if not transition_ok:
                        failures.append(
                            f"memory transition failed: expected {expected_transition}, got {transition.get('state')}"
                        )
                recovery_request_id = request_id(f"{scenario['id']}:memory-recovery")
                commit_args = {
                    "sessionId": session_id,
                    "requestId": recovery_request_id,
                    "expectedEventCount": event_count,
                    "candidateFingerprint": transition.get("candidateFingerprint", ""),
                    "subject": "Interrupted request",
                    "statement": prompt_text or "A request was observed before interruption",
                    "evidenceRecordIds": [prompt_record_id],
                }
                receipt = mcp_call(
                    project,
                    "ley_session_memory_commit_unresolved",
                    commit_args,
                    WRITE_FLAGS,
                )
                retry = mcp_call(
                    project,
                    "ley_session_memory_commit_unresolved",
                    commit_args,
                    WRITE_FLAGS,
                )
                binding_ok = (
                    transition_ok
                    and str(transition.get("candidateFingerprint", "")).startswith("sha256:")
                    and receipt.get("eventCount") == event_count + 1
                    and receipt.get("replayed") is False
                    and retry.get("eventCount") == event_count + 1
                    and retry.get("replayed") is True
                )
                scores["memory_binding"] = binding_ok
                if not binding_ok:
                    failures.append("bound recovery commit did not preserve verifier binding/idempotency")
                after = mcp_call(
                    project,
                    "ley_session_memory_compile",
                    {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
                )
                lineage_ok = True
                if scenario.get("expected_origin_lineage"):
                    shown_after = cli_json(["session", "show", str(session_id), str(project), "--json"])
                    checkpoints = shown_after.get("checkpoints", []) if isinstance(shown_after, dict) else []
                    checkpoint_id = str(checkpoints[-1].get("checkpointId", "")) if checkpoints else ""
                    proposed_learning = mcp_call(
                        project,
                        "ley_learning_propose",
                        {
                            "requestId": request_id(f"{scenario['id']}:origin-lineage"),
                            "kind": "fact",
                            "title": "Interrupted request remained unresolved",
                            "guidance": prompt_text or "A request was observed before interruption",
                            "confidencePercent": 50,
                            "provenance": "inferred",
                            "evidence": [
                                {
                                    "sessionId": session_id,
                                    "recordId": checkpoint_id,
                                    "note": "Derived only from the bound recovery checkpoint.",
                                }
                            ],
                        },
                        WRITE_FLAGS,
                    )
                    learning_id = str(proposed_learning.get("learningId", ""))
                    learning_context = mcp_call(
                        project,
                        "ley_learning_get",
                        {"learningId": learning_id, "maxCharacters": 4_000},
                    )
                    lineage = learning_context.get("originLineage", {})
                    sources = lineage.get("sources", []) if isinstance(lineage, dict) else []
                    lineage_ok = (
                        bool(checkpoint_id)
                        and lineage.get("mechanicallyResolved") is True
                        and lineage.get("causalCompletenessProven") is False
                        and lineage.get("automaticAuthorityCeiling") == "review-required"
                        and any(
                            isinstance(source, dict)
                            and source.get("kind") == "session-record"
                            and source.get("recordId") == checkpoint_id
                            for source in sources
                        )
                        and any(
                            isinstance(source, dict)
                            and source.get("kind") == "recovery-candidate"
                            and source.get("candidateFingerprint") == transition.get("candidateFingerprint")
                            for source in sources
                        )
                        and any(
                            isinstance(source, dict)
                            and source.get("kind") == "turn-evidence"
                            and source.get("recordId") == prompt_record_id
                            for source in sources
                        )
                    )
                    scores["origin_lineage"] = lineage_ok
                    if not lineage_ok:
                        failures.append("derived learning did not preserve the bound recovery origin chain")
                recovery_ok = (
                    state_ok
                    and transition_ok
                    and binding_ok
                    and lineage_ok
                    and after.get("state") == "no-unconsolidated-evidence"
                    and after.get("totalUnconsolidatedEvidence") == 0
                )
            scores["memory_recovery"] = recovery_ok
            scores["privacy_violation_rate"] = privacy_violation_rate(
                [str(project), str(vault)], [compiled]
            )
            if not recovery_ok:
                failures.append(
                    f"memory recovery failed: expected {expected_state}, got {compiled.get('state')}"
                )

    if scenario.get("expected_max_tokens") is not None:
        budget_payload = all_payloads[0] if all_payloads else {}
        max_tokens = budget_payload.get("maxTokens")
        coverage = budget_payload.get("coverage", {})
        disclosed = bool(budget_payload.get("truncated")) or any(
            isinstance(item, dict) and item.get("truncated") for item in budget_payload.get("results", [])
        ) or any(isinstance(coverage, dict) and coverage.get(key, 0) for key in ("omittedResults", "omittedCandidates", "truncatedResultContent"))
        budget_ok = max_tokens == int(scenario["expected_max_tokens"]) and int(budget_payload.get("estimatedTokens", 0)) <= int(max_tokens) and disclosed
        scores["token_budget"] = budget_ok
        if not budget_ok:
            failures.append(f"token budget was not disclosed/enforced: maxTokens={max_tokens}")

    return {
        "id": str(scenario["id"]),
        "category": str(scenario.get("category", "")),
        **scores,
        "passed": not failures,
        "failures": failures,
    }


def metric_requirement_passes(
    result: dict[str, object],
    metric: str,
    expectation: str,
) -> bool:
    value = result.get(metric)
    if expectation == "truthy":
        return value is True
    if expectation == "zero":
        return value is not None and float(value) == 0.0
    raise RuntimeError(f"unsupported capability coverage expectation: {expectation}")


def validate_coverage_config(
    label: str,
    coverage: dict[str, dict[str, tuple[str, str, str]]],
    scenarios: list[dict[str, object]],
) -> None:
    scenario_ids = {str(scenario["id"]) for scenario in scenarios}
    required_dimensions = {"adversarial", "downstream", "privacy", "regression"}
    for capability, dimensions in coverage.items():
        if set(dimensions) != required_dimensions:
            raise RuntimeError(
                f"{label} capability {capability} must define exactly {sorted(required_dimensions)}"
            )
        for dimension, (scenario_id, metric, expectation) in dimensions.items():
            if scenario_id not in scenario_ids:
                raise RuntimeError(
                    f"{label} coverage {capability}/{dimension} references unknown scenario {scenario_id}"
                )
            if metric not in METRIC_NAMES:
                raise RuntimeError(
                    f"{label} coverage {capability}/{dimension} references unknown metric {metric}"
                )
            if expectation not in {"truthy", "zero"}:
                raise RuntimeError(
                    f"{label} coverage {capability}/{dimension} has unsupported expectation {expectation}"
                )


def parse_eval_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run Ley's deterministic end-to-end evaluation corpus."
    )
    parser.add_argument(
        "--scenario",
        action="append",
        default=[],
        metavar="ID",
        help="Run only this scenario ID. Repeat to select multiple scenarios.",
    )
    parser.add_argument(
        "--category",
        action="append",
        default=[],
        metavar="NAME",
        help="Run only scenarios in this category. Repeat to select multiple categories.",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="List scenario IDs/categories without running them.",
    )
    parser.add_argument(
        "--p0-coverage",
        action="store_true",
        help="Run only P0 coverage-matrix representative scenarios and enforce the matrix.",
    )
    parser.add_argument(
        "--p1-coverage",
        action="store_true",
        help="Run only implemented P1 coverage-matrix representative scenarios and enforce the matrix.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    arguments = parse_eval_arguments(argv)
    all_scenarios = [
        json.loads(line)
        for line in FIXTURES.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    validate_coverage_config("P0", P0_CAPABILITY_COVERAGE, all_scenarios)
    validate_coverage_config("P1", P1_CAPABILITY_COVERAGE, all_scenarios)
    if arguments.list:
        for scenario in all_scenarios:
            print(f"{scenario['id']}\t{scenario.get('category', '')}")
        return 0

    requested_ids = set(arguments.scenario)
    requested_categories = set(arguments.category)
    if arguments.p0_coverage and arguments.p1_coverage:
        raise SystemExit("--p0-coverage and --p1-coverage are mutually exclusive")
    if (arguments.p0_coverage or arguments.p1_coverage) and (
        requested_ids or requested_categories
    ):
        raise SystemExit(
            "coverage modes cannot be combined with --scenario or --category"
        )
    if arguments.p0_coverage:
        coverage_ids = {
            scenario_id
            for dimensions in P0_CAPABILITY_COVERAGE.values()
            for scenario_id, _, _ in dimensions.values()
        }
        scenarios = [
            scenario
            for scenario in all_scenarios
            if str(scenario["id"]) in coverage_ids
        ]
    elif arguments.p1_coverage:
        coverage_ids = {
            scenario_id
            for dimensions in P1_CAPABILITY_COVERAGE.values()
            for scenario_id, _, _ in dimensions.values()
        }
        scenarios = [
            scenario
            for scenario in all_scenarios
            if str(scenario["id"]) in coverage_ids
        ]
    elif requested_ids or requested_categories:
        scenarios = [
            scenario
            for scenario in all_scenarios
            if str(scenario["id"]) in requested_ids
            or str(scenario.get("category", "")) in requested_categories
        ]
        if not scenarios:
            available = ", ".join(str(item["id"]) for item in all_scenarios)
            raise SystemExit(
                "no eval scenarios matched the requested filters; available IDs: "
                + available
            )
    else:
        scenarios = all_scenarios
    full_corpus = len(scenarios) == len(all_scenarios) and {
        str(item["id"]) for item in scenarios
    } == {str(item["id"]) for item in all_scenarios}
    enforce_p0_coverage = full_corpus or arguments.p0_coverage
    enforce_p1_coverage = full_corpus or arguments.p1_coverage
    print(f"Running {len(scenarios)} eval scenarios with {LEY}...\n", flush=True)
    results: list[dict[str, object]] = []
    with tempfile.TemporaryDirectory(prefix="ley-eval-") as temporary:
        EVAL_ENV["XDG_CONFIG_HOME"] = str(Path(temporary) / "config")
        for index, scenario in enumerate(scenarios, start=1):
            try:
                result = evaluate_scenario(scenario, Path(temporary) / str(scenario["id"]))
            except Exception as error:  # one broken scenario must not hide the rest
                result = {
                    "id": str(scenario["id"]),
                    "category": str(scenario.get("category", "")),
                    "passed": False,
                    "failures": [f"unhandled scenario error: {error}"],
                }
            results.append(result)
            print(
                f"[{index}/{len(scenarios)}] {scenario['id']}: {'PASS' if result.get('passed') else 'FAIL'}",
                flush=True,
            )
            for metric in METRIC_NAMES:
                if result.get(metric) is not None:
                    print(f"  {metric}: {result[metric]}", flush=True)
            for failure in result.get("failures", []):
                print(f"  ERROR: {failure}", flush=True)

    passed = sum(bool(result.get("passed")) for result in results)
    print(f"\n=== Aggregate ({len(results)} scenarios) ===", flush=True)
    print(f"Scenarios passed: {passed}/{len(results)}", flush=True)
    rate_metrics = {
        "recall@k": f"Mean recall@{K}",
        "precision": "Mean precision",
        "privacy_violation_rate": "Mean privacy violation rate",
        "forgetting_residue_rate": "Mean forgetting residue rate",
    }
    for metric, label in rate_metrics.items():
        values = [
            float(result[metric])
            for result in results
            if result.get(metric) is not None
        ]
        if not values:
            continue
        print(f"{label}: {sum(values) / len(values):.3f}", flush=True)
        if metric in {"privacy_violation_rate", "forgetting_residue_rate"}:
            print(f"Max {metric}: {max(values):.3f}", flush=True)

    for metric in METRIC_NAMES:
        if metric in rate_metrics:
            continue
        values = [
            bool(result[metric])
            for result in results
            if result.get(metric) is not None
        ]
        if values:
            print(
                f"{metric}: {sum(values)}/{len(values)} passed",
                flush=True,
            )

    coverage_failures: list[str] = []
    if enforce_p0_coverage:
        result_by_id = {
            str(result.get("id")): result
            for result in results
            if isinstance(result.get("id"), str)
        }
        print("", flush=True)
        print("=== P0 capability metric coverage ===", flush=True)
        for capability, dimensions in P0_CAPABILITY_COVERAGE.items():
            dimension_results: list[str] = []
            for dimension, (scenario_id, metric, expectation) in dimensions.items():
                result = result_by_id.get(scenario_id, {})
                ok = metric_requirement_passes(result, metric, expectation)
                dimension_results.append(f"{dimension}={'PASS' if ok else 'FAIL'}")
                if not ok:
                    coverage_failures.append(
                        f"{capability}/{dimension} requires {scenario_id}:{metric}={expectation}"
                    )
            print(f"{capability}: " + ", ".join(dimension_results), flush=True)
        for failure in coverage_failures:
            print(f"  COVERAGE ERROR: {failure}", flush=True)
    else:
        print(
            "P0 capability coverage matrix: skipped for focused subset run.",
            flush=True,
        )

    if enforce_p1_coverage:
        result_by_id = {
            str(result.get("id")): result
            for result in results
            if isinstance(result.get("id"), str)
        }
        print("", flush=True)
        print("=== P1 capability metric coverage ===", flush=True)
        for capability, dimensions in P1_CAPABILITY_COVERAGE.items():
            dimension_results: list[str] = []
            for dimension, (scenario_id, metric, expectation) in dimensions.items():
                result = result_by_id.get(scenario_id, {})
                ok = metric_requirement_passes(result, metric, expectation)
                dimension_results.append(f"{dimension}={'PASS' if ok else 'FAIL'}")
                if not ok:
                    coverage_failures.append(
                        f"{capability}/{dimension} requires {scenario_id}:{metric}={expectation}"
                    )
            print(f"{capability}: " + ", ".join(dimension_results), flush=True)
        for failure in coverage_failures:
            if any(
                failure.startswith(f"{capability}/")
                for capability in P1_CAPABILITY_COVERAGE
            ):
                print(f"  COVERAGE ERROR: {failure}", flush=True)
    else:
        print(
            "P1 capability coverage matrix: skipped for focused subset run.",
            flush=True,
        )

    return 0 if passed == len(results) and not coverage_failures else 1


if __name__ == "__main__":
    sys.exit(main())
