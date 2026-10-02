#!/usr/bin/env python3
"""Run the real Ley CLI/MCP evaluation fixtures as fail-fast assertions.

Every scenario must execute its declared setup and checks. A missing tool
response, skipped event kind, invalid fixture identifier, or unmet expectation
is a failed scenario and makes this command exit non-zero.
"""

import argparse
import base64
import hashlib
import json
import os
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
from typing import TextIO


REPO_ROOT = Path(__file__).resolve().parents[1]
FIXTURES = Path(__file__).parent / "fixtures" / "scenarios.jsonl"
K = 5
WRITE_FLAGS = ("--allow-session-writes", "--allow-learning-proposals")
METRIC_NAMES = (
    "recall@k",
    "precision",
    "known_failure_reuse",
    "untrusted_boundary",
    "cross_project_clean",
    "stale_learning",
    "stale_learning_recovery",
    "capture_recovery",
    "interruption_recovery",
    "origin_lineage",
    "idempotency",
    "learning_idempotency",
    "delayed_poisoning_resistance",
    "token_budget",
    "secret_exclusion",
    "specification_admission",
    "premise_adjudication",
    "revision_adjudication",
    "egress_policy",
    "selective_abstention",
    "parallel_session_separation",
    "parallel_session_reconciliation",
    "cross_surface_staleness",
    "long_horizon_continuity",
    "weeks_later_continuation",
    "deletion_fidelity",
    "forgetting_residue_rate",
    "inactive_workspace_clean",
    "host_portability",
    "downstream_task_contract",
    "budget_baseline_advantage",
    "budget_full_history_efficiency",
    "retrieval_robustness",
    "privacy_violation_rate",
    "verification_evidence_links",
    "live_source_honesty",
    "branch_worktree_controls",
    "multimodal_evidence",
    "historical_host_import",
    "consolidation_inbox",
    "bootstrap_specification",
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
        "regression": (
            "retrieval-fallback-budget-ladder",
            "retrieval_robustness",
            "truthy",
        ),
    },
    "interruption-recovery": {
        "adversarial": (
            "crash-before-session-end-resume",
            "interruption_recovery",
            "truthy",
        ),
        "downstream": (
            "crash-before-session-end-resume",
            "interruption_recovery",
            "truthy",
        ),
        "privacy": (
            "crash-before-session-end-resume",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "crash-before-session-end-resume",
            "interruption_recovery",
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
            "downstream_task_contract",
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
    "origin-lineage": {
        "adversarial": (
            "parallel-agent-session-separation",
            "origin_lineage",
            "truthy",
        ),
        "downstream": (
            "parallel-agent-session-separation",
            "downstream_task_contract",
            "truthy",
        ),
        "privacy": (
            "session-erasure-derived-residue",
            "forgetting_residue_rate",
            "zero",
        ),
        "regression": (
            "parallel-agent-session-separation",
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
            "downstream_task_contract",
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
            "downstream_task_contract",
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
            "downstream_task_contract",
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

P0_INDEPENDENT_DOWNSTREAM_CAPABILITIES = frozenset(
    {
        "context-compiler",
        "specifications",
        "origin-lineage",
        "premise-adjudication",
        "revision-awareness",
        "egress-policy",
    }
)

P1_INDEPENDENT_DOWNSTREAM_CAPABILITIES = frozenset(
    {
        "bootstrap-specifications",
    }
)

P2_INDEPENDENT_DOWNSTREAM_CAPABILITIES = frozenset(
    {
        "explicit-historical-host-import",
    }
)

P1_CAPABILITY_COVERAGE = {
    "bootstrap-specifications": {
        "adversarial": (
            "empty-workspace-bootstrap-specification",
            "bootstrap_specification",
            "truthy",
        ),
        "downstream": (
            "empty-workspace-bootstrap-specification",
            "downstream_task_contract",
            "truthy",
        ),
        "privacy": (
            "empty-workspace-bootstrap-specification",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "empty-workspace-bootstrap-specification",
            "bootstrap_specification",
            "truthy",
        ),
    },
    "verification-evidence-links": {
        "adversarial": (
            "verification-evidence-links",
            "verification_evidence_links",
            "truthy",
        ),
        "downstream": (
            "verification-evidence-links",
            "verification_evidence_links",
            "truthy",
        ),
        "privacy": (
            "verification-evidence-links",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "verification-evidence-links",
            "verification_evidence_links",
            "truthy",
        ),
    },
    "branch-worktree-controls": {
        "adversarial": (
            "divergent-branch-state-adjudication",
            "branch_worktree_controls",
            "truthy",
        ),
        "downstream": (
            "divergent-branch-state-adjudication",
            "branch_worktree_controls",
            "truthy",
        ),
        "privacy": (
            "divergent-branch-state-adjudication",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "divergent-branch-state-adjudication",
            "branch_worktree_controls",
            "truthy",
        ),
    },
}

P2_CAPABILITY_COVERAGE = {
    "multimodal-agent-memory-evidence": {
        "adversarial": (
            "multimodal-original-image-evidence",
            "multimodal_evidence",
            "truthy",
        ),
        "downstream": (
            "multimodal-original-image-evidence",
            "multimodal_evidence",
            "truthy",
        ),
        "privacy": (
            "multimodal-original-image-evidence",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "multimodal-original-image-evidence",
            "multimodal_evidence",
            "truthy",
        ),
    },
    "explicit-historical-host-import": {
        "adversarial": (
            "explicit-codex-message-history-import",
            "historical_host_import",
            "truthy",
        ),
        "downstream": (
            "explicit-codex-message-history-import",
            "downstream_task_contract",
            "truthy",
        ),
        "privacy": (
            "explicit-codex-message-history-import",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "explicit-codex-message-history-import",
            "historical_host_import",
            "truthy",
        ),
    },
    "local-consolidation-review": {
        "adversarial": (
            "local-consolidation-inbox",
            "consolidation_inbox",
            "truthy",
        ),
        "downstream": (
            "local-consolidation-inbox",
            "consolidation_inbox",
            "truthy",
        ),
        "privacy": (
            "local-consolidation-inbox",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "local-consolidation-inbox",
            "consolidation_inbox",
            "truthy",
        ),
    },
}


def find_ley() -> str:
    configured = os.environ.get("LEY_BIN")
    binary_name = "ley.exe" if os.name == "nt" else "ley"
    candidates = [
        Path(configured) if configured else None,
        REPO_ROOT / "target" / "debug" / binary_name,
        REPO_ROOT / "target" / "release" / binary_name,
        Path(shutil.which("ley") or ""),
        Path.home() / ".local" / "bin" / binary_name,
    ]
    for candidate in candidates:
        if candidate and candidate.is_file() and os.access(candidate, os.X_OK):
            return str(candidate)
    raise RuntimeError("ley binary not found; build it or set LEY_BIN")


LEY = find_ley()
EVAL_ENV: dict[str, str] = {}
BOOTSTRAP_UNSUPPORTED_MARKER = (
    "cannot establish the directory generation required for Ley bootstrap Specification authority"
)


def read_process_line_with_timeout(
    stream: TextIO,
    timeout_seconds: float,
    *,
    timeout_message: str,
) -> str:
    """Read one text line from a subprocess pipe with a cross-platform timeout."""
    result: list[str] = []
    errors: list[Exception] = []

    def read_line() -> None:
        try:
            result.append(stream.readline())
        except Exception as error:  # propagate pipe/decoder failures to the caller
            errors.append(error)

    reader = threading.Thread(target=read_line, daemon=True)
    reader.start()
    reader.join(timeout_seconds)
    if reader.is_alive():
        raise RuntimeError(timeout_message)
    if errors:
        raise RuntimeError("failed reading subprocess output") from errors[0]
    return result[0] if result else ""


class BootstrapScenarioUnsupported(RuntimeError):
    pass


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


def session_mutation_payload(raw: object) -> dict[str, object]:
    if not isinstance(raw, dict) or not isinstance(raw.get("session"), dict):
        raise RuntimeError("session mutation returned no session payload")
    session = dict(raw["session"])
    session["eventId"] = raw.get("eventId")
    session["replayed"] = raw.get("replayed")
    session["storage"] = raw.get("storage")
    return session


def cli_session_start(
    project: Path,
    *,
    seed: str,
    name: str,
    goal: str,
    host: str | None = None,
) -> dict[str, object]:
    args = [
        "session",
        "start",
        str(project),
        "--request-id",
        request_id(f"{seed}:start"),
        "--name",
        name,
        "--goal",
        goal,
    ]
    if host:
        args.extend(["--host", host])
    args.append("--json")
    return session_mutation_payload(cli_json(args))


def cli_session_checkpoint(
    project: Path,
    session_id: str,
    payload: dict[str, object],
) -> dict[str, object]:
    checkpoint = dict(payload)
    checkpoint.pop("sessionId", None)
    if not isinstance(checkpoint.get("requestId"), str):
        raise RuntimeError("checkpoint fixture requires requestId")
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        suffix=".json",
        delete=False,
    ) as handle:
        json.dump(checkpoint, handle, separators=(",", ":"))
        data_path = Path(handle.name)
    try:
        raw = cli_json(
            [
                "session",
                "checkpoint",
                session_id,
                str(project),
                "--data",
                str(data_path),
                "--json",
            ]
        )
    finally:
        data_path.unlink(missing_ok=True)
    return session_mutation_payload(raw)


def cli_session_show(project: Path, session_id: str) -> dict[str, object]:
    value = cli_json(["session", "show", session_id, str(project), "--json"])
    if not isinstance(value, dict):
        raise RuntimeError("session show returned no object")
    return value


def cli_session_turns(
    project: Path,
    session_id: str,
    *,
    max_results: int,
    max_characters: int,
) -> dict[str, object]:
    value = cli_json(
        [
            "session",
            "turns",
            session_id,
            str(project),
            "--max-results",
            str(max_results),
            "--max-characters",
            str(max_characters),
            "--json",
        ]
    )
    if not isinstance(value, dict):
        raise RuntimeError("session turns returned no object")
    return value


def cli_session_finish(
    project: Path,
    session_id: str,
    *,
    request_id_value: str,
    status: str,
    summary: str,
    final_response: str = "",
    handoff: str = "",
    unresolved: list[str] | None = None,
) -> dict[str, object]:
    args = [
        "session",
        "finish",
        session_id,
        str(project),
        "--request-id",
        request_id_value,
        "--status",
        status,
        "--summary",
        summary,
    ]
    if final_response:
        args.extend(["--final-response", final_response])
    if handoff:
        args.extend(["--handoff", handoff])
    for item in unresolved or []:
        args.extend(["--unresolved", item])
    args.append("--json")
    return session_mutation_payload(cli_json(args))


def cli_session_list_payload(project: Path) -> dict[str, object]:
    value = cli_json(["session", "list", str(project), "--json"])
    if not isinstance(value, list):
        raise RuntimeError("session list returned no array")
    sessions = [item for item in value if isinstance(item, dict)]
    return {
        "sessions": sessions,
        "totalSessions": len(sessions),
        "omittedSessions": 0,
    }


def cli_resume_payload(
    project: Path,
    *,
    max_sessions: int,
    max_learnings: int,
    max_characters: int,
) -> dict[str, object]:
    value = cli_json(
        [
            "resume",
            str(project),
            "--max-sessions",
            str(max_sessions),
            "--max-learnings",
            str(max_learnings),
            "--max-characters",
            str(max_characters),
            "--json",
        ]
    )
    if not isinstance(value, dict):
        raise RuntimeError("resume returned no object")
    return value


def write_project_files(project: Path, files: dict[str, str]) -> None:
    for relative, content in files.items():
        path = Path(relative)
        if path.is_absolute() or ".." in path.parts:
            raise RuntimeError(f"fixture contains unsafe project path: {relative}")
        destination = project / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(content, encoding="utf-8")


def write_project_binary_files(project: Path, files: dict[str, str]) -> None:
    for relative, encoded in files.items():
        path = Path(relative)
        if path.is_absolute() or ".." in path.parts:
            raise RuntimeError(f"fixture contains unsafe binary project path: {relative}")
        destination = project / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        try:
            body = base64.b64decode(encoded, validate=True)
        except ValueError as error:
            raise RuntimeError(
                f"fixture contains invalid base64 binary project file: {relative}"
            ) from error
        destination.write_bytes(body)


def init_project(project: Path, name: str, capture_mode: str = "structured") -> None:
    run(
        [
            "init",
            str(project),
            "--name",
            name,
            "--capture",
            capture_mode,
            "--json",
        ]
    )
    run(["ingest", str(project), "--json"])


def install_specification_approvals(
    project: Path, specifications: list[dict[str, object]]
) -> None:
    """Seed explicit human approval for deterministic native-continuity fixtures.

    Production approval is intentionally a local Desktop action. The eval harness has no GUI, so fixture setup
    writes the same project-file authority rows directly into the already-initialized private continuity database,
    before any MCP/hook process starts. Runtime behavior remains exercised through the real CLI/MCP surfaces.
    """
    if not specifications:
        return
    diagnostic = cli_json(["doctor", str(project), "--json"])
    if not isinstance(diagnostic, dict):
        raise RuntimeError("doctor returned no project diagnostic for specification fixture")
    identity = diagnostic.get("identity")
    if not isinstance(identity, dict) or not isinstance(identity.get("projectId"), str):
        raise RuntimeError("doctor returned no projectId for specification fixture")
    project_id = str(identity["projectId"])
    continuity_path = (
        Path(EVAL_ENV["XDG_CONFIG_HOME"])
        / "app.leynotes.desktop"
        / "continuity.sqlite3"
    )
    if not continuity_path.is_file():
        raise RuntimeError("native continuity database missing before Specification fixture setup")
    with sqlite3.connect(continuity_path) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
        authority = connection.execute(
            "SELECT approved_source_authority_migrated FROM projects WHERE project_id = ?",
            (project_id,),
        ).fetchone()
        if authority != (1,):
            raise RuntimeError(
                "native approved-source authority is not ready before Specification fixture setup"
            )
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
            destination = project / relative_path
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(source, encoding="utf-8")
            digest = "sha256:" + hashlib.sha256(source.encode("utf-8")).hexdigest()
            connection.execute(
                """
                INSERT INTO approved_sources(
                    project_id, source_id, source_kind, display_name,
                    project_relative_path, content_hash, snapshot_blob_hash,
                    approved_at_unix_ms
                ) VALUES (?, ?, 'project-file', ?, ?, ?, NULL, ?)
                """,
                (
                    project_id,
                    specification_id,
                    relative_path,
                    relative_path,
                    digest,
                    int(time.time() * 1000) + index,
                ),
            )
            definition["resolved_specification_id"] = specification_id


def mcp_call_result(
    project: Path,
    name: str,
    arguments: dict[str, object],
    flags: tuple[str, ...] = (),
) -> tuple[dict[str, object], dict[str, object]]:
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
            line = read_process_line_with_timeout(
                proc.stdout,
                remaining,
                timeout_message=f"MCP call {name} timed out waiting for id {expected_id}",
            )
            if not line:
                exit_code = proc.poll()
                stderr = ""
                if exit_code is not None and proc.stderr is not None:
                    stderr = proc.stderr.read().strip()
                detail = (
                    f"; exit={exit_code}; stderr={stderr!r}"
                    if exit_code is not None or stderr
                    else ""
                )
                raise RuntimeError(
                    f"MCP call {name} returned no result before stdout closed: {responses!r}{detail}"
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
    finally:
        for stream in (proc.stdin, proc.stdout, proc.stderr):
            if stream is not None and not stream.closed:
                stream.close()
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
    return payload, result


def mcp_call(
    project: Path,
    name: str,
    arguments: dict[str, object],
    flags: tuple[str, ...] = (),
) -> dict[str, object]:
    payload, _ = mcp_call_result(project, name, arguments, flags)
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
            line = read_process_line_with_timeout(
                proc.stdout,
                remaining,
                timeout_message=f"MCP tools/list timed out waiting for id {expected_id}",
            )
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


def hook_additional_context(payload: dict[str, object]) -> str:
    hook_output = payload.get("hookSpecificOutput", {})
    if not isinstance(hook_output, dict):
        return ""
    value = hook_output.get("additionalContext", "")
    return value if isinstance(value, str) else ""


def automatic_hook_context(payload: dict[str, object]) -> str:
    context = hook_additional_context(payload)
    marker = "# Ley task context (automatic)"
    start = context.find(marker)
    return "" if start < 0 else context[start:]


def hook_ley_session_id(payload: dict[str, object]) -> str:
    for line in hook_additional_context(payload).splitlines():
        if line.startswith("Current Ley session: ") and line.endswith("."):
            return line.removeprefix("Current Ley session: ").removesuffix(".")
    return ""


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
    plan: list[dict[str, object]] = []
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
        elif kind == "plan":
            plan.append(
                {
                    "text": str(event.get("text", "")),
                    "status": str(event.get("status", "pending")),
                }
            )
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
            evidence_paths = event.get("evidence_artifact_paths")
            if isinstance(evidence_paths, list):
                item["evidenceArtifactPaths"] = [str(path) for path in evidence_paths]
            verification.append(item)
        elif kind == "unresolved":
            unresolved.append(str(event.get("text", "")))

    checkpoint: dict[str, object] = {
        "summary": ("; ".join(summaries) or "Captured structured project progress.")[:16000],
        "plan": plan,
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

    start_receipt = cli_session_start(
        project,
        seed=str(scenario["id"]),
        name=str(scenario["goal"])[:128],
        goal=str(scenario["goal"]),
        host="codex",
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
            "plan",
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
        receipts.append(cli_session_checkpoint(project, session_id, args))
    if structured_events:
        args = checkpoint_from_events(structured_events, artifact_paths)
        args.update({"sessionId": session_id, "requestId": request_id(f"{scenario['id']}:structured")})
        receipts.append(cli_session_checkpoint(project, session_id, args))
    elif not checkpoint_events:
        args = checkpoint_from_events(events, artifact_paths)
        args.update({"sessionId": session_id, "requestId": request_id(f"{scenario['id']}:fallback")})
        receipts.append(cli_session_checkpoint(project, session_id, args))

    evidence_record_id = session_id
    if receipts:
        shown = cli_json(["session", "show", session_id, str(project), "--json"])
        checkpoints = shown.get("checkpoints", []) if isinstance(shown, dict) else []
        if checkpoints:
            evidence_record_id = str(checkpoints[-1]["checkpointId"])
    for index, event in enumerate(events):
        if event.get("type") != "learning":
            continue
        proposed = cli_json(
            [
                "learning",
                "propose",
                str(project),
                "--request-id",
                request_id(f"{scenario['id']}:learning:{index}"),
                "--actor",
                "agent",
                "--provenance",
                "agent-authored",
                "--kind",
                str(event.get("kind", "fact")),
                "--title",
                str(event.get("title", "Untitled learning")),
                "--guidance",
                str(event.get("guidance", "")),
                "--confidence",
                str(int(event.get("confidence_percent", 50))),
                "--evidence",
                f"{session_id}:{evidence_record_id}",
                "--json",
            ]
        )
        learning = proposed.get("learning", {}) if isinstance(proposed, dict) else {}
        receipts.append(
            {
                "learningId": learning.get("learningId"),
                "replayed": proposed.get("replayed", False)
                if isinstance(proposed, dict)
                else False,
            }
        )
    return session_id, receipts, []


def search_payloads(project: Path, query: str) -> list[dict[str, object]]:
    return [
        mcp_call(project, "ley_search", {"query": query, "maxResults": K, "maxTokens": 500}),
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
            "policyBundlePolicies": payload.get("policyBundlePolicies", []),
            "items": payload.get("items", []),
            "mountedReferences": payload.get("mountedReferences", []),
            "sharedKnowledgeReferences": payload.get("sharedKnowledgeReferences", []),
            "references": payload.get("references", []),
        },
        sort_keys=True,
    )


def task_contract_success(
    payload: dict[str, object],
    required: list[str],
    forbidden: list[str],
) -> bool:
    return text_contract_success(
        context_contract_text(payload),
        required,
        forbidden,
    )


def text_contract_success(
    text: str,
    required: list[str],
    forbidden: list[str],
) -> bool:
    text = text.lower()
    return all(marker.lower() in text for marker in required) and all(
        marker.lower() not in text for marker in forbidden
    )


def record_downstream_task_contract(
    scores: dict[str, object],
    failures: list[str],
    passed: bool,
    failure: str,
) -> None:
    """Accumulate independent task-facing contracts without allowing overwrite."""
    previous = scores.get("downstream_task_contract")
    scores["downstream_task_contract"] = (
        bool(passed) if previous is None else bool(previous) and bool(passed)
    )
    if not passed:
        failures.append(failure)


def privacy_violation_rate(canaries: list[str], outputs: list[object]) -> float:
    if not canaries:
        return 0.0
    text = serialized(outputs).lower()
    exposed = sum(canary.lower() in text for canary in canaries)
    return exposed / len(canaries)


def evaluate_bootstrap_specification_scenario(
    scenario: dict[str, object],
    base_dir: Path,
    target: Path,
    expectation: dict[str, object],
) -> tuple[bool, bool, float, list[str]]:
    failures: list[str] = []
    source = base_dir / "bootstrap-source"
    source_vault = base_dir / "bootstrap-source-vault"
    source.mkdir(parents=True)
    source_vault.mkdir(parents=True)
    write_project_files(source, {"README.md": "Bootstrap source project.\n"})
    init_project(source, "Bootstrap Specification source")

    relative_path = str(expectation.get("path", "Specs/Product.md"))
    source_text = str(expectation.get("source", ""))
    marker = str(expectation.get("marker", ""))
    task = str(expectation.get("task", marker))
    prompt_marker = str(expectation.get("prompt_marker", ""))
    target_private_marker = str(expectation.get("target_private_marker", ""))
    if not source_text or not marker or not task or not prompt_marker or not target_private_marker:
        raise RuntimeError("bootstrap Specification eval fixture is incomplete")

    definitions: list[dict[str, object]] = [
        {"path": relative_path, "source": source_text}
    ]
    install_specification_approvals(source, definitions)
    specification_id = str(definitions[0].get("resolved_specification_id", ""))
    if not specification_id.startswith("spec_"):
        raise RuntimeError("bootstrap Specification fixture did not resolve a specification ID")

    try:
        attached = cli_json(
            [
                "bootstrap-spec",
                "attach",
                str(source),
                specification_id,
                str(target),
                "--json",
            ]
        )
    except RuntimeError as error:
        if BOOTSTRAP_UNSUPPORTED_MARKER in str(error):
            raise BootstrapScenarioUnsupported(
                "bootstrap Specification authority is unsupported on this platform/filesystem"
            ) from error
        raise
    listed = cli_json(["bootstrap-spec", "list", str(target), "--json"])
    if (target / ".ley").exists():
        failures.append("bootstrap attachment initialized or mutated target .ley metadata")

    tools = mcp_tools_list(target)
    tool_names = [str(item.get("name", "")) for item in tools]
    compiled = mcp_call(
        target,
        "ley_compile_context",
        {"task": task, "maxResults": 8, "maxTokens": 1500},
    )

    codex_start = hook_call(
        target,
        "codex",
        {"hook_event_name": "SessionStart", "session_id": "bootstrap-eval-codex"},
    )
    claude_start = hook_call(
        target,
        "claude",
        {"hook_event_name": "SessionStart", "session_id": "bootstrap-eval-claude"},
    )
    codex_prompt = hook_call(
        target,
        "codex",
        {
            "hook_event_name": "UserPromptSubmit",
            "session_id": "bootstrap-eval-codex",
            "turn_id": "bootstrap-eval-turn-1",
            "prompt": f"{task} {prompt_marker}",
        },
    )
    claude_prompt = hook_call(
        target,
        "claude",
        {
            "hook_event_name": "UserPromptSubmit",
            "session_id": "bootstrap-eval-claude",
            "prompt": f"{task} {prompt_marker}",
        },
    )
    codex_context = hook_additional_context(codex_prompt)
    claude_context = hook_additional_context(claude_prompt)

    initial_specifications = compiled.get("specifications", [])
    initial_spec = (
        initial_specifications[0]
        if isinstance(initial_specifications, list) and initial_specifications
        and isinstance(initial_specifications[0], dict)
        else {}
    )
    initial_ok = (
        isinstance(attached, dict)
        and attached.get("created") is True
        and isinstance(listed, dict)
        and listed.get("targetInitialized") is False
        and listed.get("totalGrants") == 1
        and tool_names == ["ley_compile_context"]
        and compiled.get("projectMemoryAvailable") is False
        and compiled.get("automaticWriteAllowed") is False
        and compiled.get("targetInitialized") is False
        and initial_spec.get("source") == source_text
        and initial_spec.get("specificationId") == specification_id
        and codex_start == {}
        and claude_start == {}
        and codex_context.startswith("# Ley bootstrap task context (automatic)")
        and claude_context.startswith("# Ley bootstrap task context (automatic)")
        and marker in codex_context
        and marker in claude_context
        and prompt_marker not in codex_context
        and prompt_marker not in claude_context
        and len(codex_context.encode("utf-8")) <= 3500
        and len(claude_context.encode("utf-8")) <= 3500
        and not (target / ".ley").exists()
    )
    downstream_required = [
        str(value)
        for value in expectation.get(
            "downstream_required",
            [marker],
        )
    ]
    downstream_forbidden = [
        str(value)
        for value in expectation.get(
            "downstream_forbidden",
            [prompt_marker, target_private_marker],
        )
    ]
    downstream_ok = task_contract_success(
        compiled,
        downstream_required,
        downstream_forbidden,
    )
    if not initial_ok:
        failures.append(
            "bootstrap Specification was not delivered through the single-tool MCP and both prompt hooks without initializing the target"
        )
    if not downstream_ok:
        failures.append(
            "bootstrap Specification context did not satisfy the independent downstream human-intent contract"
        )

    cli_json(
        [
            "egress",
            "project",
            "never-send",
            str(source),
            "--json",
        ]
    )
    blocked = mcp_call(
        target,
        "ley_compile_context",
        {"task": task, "maxResults": 8, "maxTokens": 1500},
    )
    blocked_specs = blocked.get("specifications", [])
    blocked_coverage = blocked.get("coverage", {})
    blocked_ok = (
        isinstance(blocked_specs, list)
        and len(blocked_specs) == 0
        and isinstance(blocked_coverage, dict)
        and blocked_coverage.get("egressBlocked") == 1
        and marker not in serialized(blocked)
    )
    if not blocked_ok:
        failures.append("source-project never-send policy did not fail closed in bootstrap MCP")

    cli_json(
        [
            "egress",
            "project",
            "agent-ok",
            str(source),
            "--json",
        ]
    )
    cli_json(
        [
            "init",
            str(target),
            "--name",
            "Bootstrap eval target",
            "--json",
        ]
    )
    after_init = cli_json(["bootstrap-spec", "list", str(target), "--json"])
    post_init_hook = hook_call(
        target,
        "codex",
        {
            "hook_event_name": "UserPromptSubmit",
            "session_id": "bootstrap-eval-codex",
            "turn_id": "bootstrap-eval-turn-2",
            "prompt": task,
        },
    )
    transition_ok = (
        isinstance(after_init, dict)
        and after_init.get("targetInitialized") is True
        and after_init.get("totalGrants") == 0
        and post_init_hook == {}
        and (target / ".ley").is_dir()
    )
    if not transition_ok:
        failures.append("normal initialization did not retire bootstrap authority cleanly")

    agent_outputs: list[object] = [
        tools,
        compiled,
        codex_start,
        claude_start,
        codex_prompt,
        claude_prompt,
        blocked,
        post_init_hook,
    ]
    privacy = privacy_violation_rate(
        [
            str(target),
            str(source),
            str(source_vault),
            target_private_marker,
            prompt_marker,
        ],
        agent_outputs,
    )
    if privacy != 0.0:
        failures.append("bootstrap Specification agent output leaked a private path or prompt/live-target canary")

    return not failures, downstream_ok, privacy, failures


def create_structured_session(
    project: Path,
    *,
    seed: str,
    name: str,
    goal: str,
    summary: str,
    decisions: list[dict[str, str]] | None = None,
    verification: list[dict[str, object]] | None = None,
    touched_artifacts: list[str] | None = None,
    unresolved: list[str] | None = None,
    host: str = "codex",
) -> tuple[str, dict[str, object]]:
    started = cli_session_start(
        project,
        seed=seed,
        name=name,
        goal=goal,
        host=host,
    )
    session_id = str(started["sessionId"])
    checkpoint: dict[str, object] = {
        "sessionId": session_id,
        "requestId": request_id(f"{seed}:checkpoint"),
        "summary": summary,
    }
    if decisions:
        checkpoint["decisions"] = decisions
    if verification:
        checkpoint["verification"] = verification
    if touched_artifacts:
        checkpoint["touchedArtifacts"] = touched_artifacts
    if unresolved:
        checkpoint["unresolved"] = unresolved
    receipt = cli_session_checkpoint(project, session_id, checkpoint)
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
    binary_files = scenario.get("project_binary_files", {})
    if isinstance(binary_files, dict):
        write_project_binary_files(project, binary_files)
    revision_flow = scenario.get("git_revision_flow")
    if isinstance(revision_flow, dict):
        git_run(project, ["init", "-b", "main"])
        git_commit_all(project, "base")
    large = scenario.get("large_project")
    if isinstance(large, dict):
        count = int(large.get("num_files", 0))
        lines = int(large.get("avg_file_lines", 0))
        distractor_text = str(large.get("distractor_text", "")).strip()
        relevant_index = int(large.get("relevant_index", -1))
        relevant_text = str(large.get("relevant_text", "")).strip()
        for index in range(count):
            body = ["def main(): pass\n" if index == 0 else f"def worker_{index}(): pass\n"]
            if distractor_text:
                body.append(f"# {distractor_text}\n")
            if index == relevant_index and relevant_text:
                body.append(f"# {relevant_text}\n")
            body.extend(f"# generated evidence line {line}\n" for line in range(max(0, lines - 1)))
            write_project_files(project, {f"src/module_{index:03d}.py": "".join(body)})
    events = scenario.get("session_events", [])
    if isinstance(events, list):
        ensure_learning_citations(project, [event for event in events if isinstance(event, dict)])

    bootstrap_expectation = scenario.get("expected_bootstrap_specification")
    if isinstance(bootstrap_expectation, dict):
        passed, downstream, privacy, bootstrap_failures = evaluate_bootstrap_specification_scenario(
            scenario,
            base_dir,
            project,
            bootstrap_expectation,
        )
        scores["bootstrap_specification"] = passed
        scores["downstream_task_contract"] = downstream
        scores["privacy_violation_rate"] = privacy
        failures.extend(bootstrap_failures)
        return {
            "id": str(scenario["id"]),
            "category": str(scenario.get("category", "")),
            **scores,
            "passed": not failures,
            "failures": failures,
        }

    init_project(
        project,
        str(scenario["goal"]),
        str(scenario.get("capture_mode", "structured")),
    )
    specification_definitions = [
        item
        for item in scenario.get("specifications", [])
        if isinstance(item, dict)
    ]
    install_specification_approvals(project, specification_definitions)
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
            cli_json(
                [
                    "session",
                    "turns",
                    session_id,
                    str(project),
                    "--max-results",
                    "20",
                    "--max-characters",
                    "8000",
                    "--json",
                ]
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
            "ley_brief",
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
            and replacement_admitted
            and obsolete_withheld
            and compiled.get("liveSourceChecked") is False
            and int(compiled.get("estimatedTokens", 0)) <= int(compiled.get("maxTokens", 0))
        )
        scores["premise_adjudication"] = premise_ok
        downstream_required = [
            str(value)
            for value in premise_expectation.get("downstream_required", [])
        ]
        downstream_forbidden = [
            str(value)
            for value in premise_expectation.get("downstream_forbidden", [])
        ]
        if downstream_required or downstream_forbidden:
            record_downstream_task_contract(
                scores,
                failures,
                task_contract_success(
                    compiled,
                    downstream_required,
                    downstream_forbidden,
                ),
                "premise adjudication context did not satisfy the independent downstream replacement-state contract",
            )
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
            "ley_brief",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
        )
        divergent_host_start = hook_call(
            project,
            "codex",
            {
                "hook_event_name": "SessionStart",
                "session_id": f"{scenario['id']}-divergent-host",
            },
        )
        divergent_host_context = hook_additional_context(divergent_host_start)
        divergent_host_revision_safe = (
            query.lower() not in divergent_host_context.lower()
            and "historical ley project memory was not auto-injected" in divergent_host_context.lower()
            and "call ley_brief" in divergent_host_context.lower()
            and "## recent work" not in divergent_host_context.lower()
            and "## reviewed project learnings" not in divergent_host_context.lower()
            and str(project) not in divergent_host_context
            and str(vault) not in divergent_host_context
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
            and divergent_host_revision_safe
            and divergent.get("liveSourceChecked") is False
        )

        branch_controls_expectation = scenario.get("expected_branch_worktree_controls")
        divergent_search: dict[str, object] | None = None
        current_lineage_search: dict[str, object] | None = None
        divergent_session: dict[str, object] | None = None
        branch_controls_pre_merge_ok = True
        if isinstance(branch_controls_expectation, dict):
            if not session_id:
                raise RuntimeError("branch/worktree controls fixture created no session")
            controls_query = str(branch_controls_expectation.get("query", query))
            divergent_search = mcp_call(
                project,
                "ley_search",
                {
                    "query": controls_query,
                    "revisionCompatibility": "divergent",
                    "maxResults": 12,
                    "maxTokens": 4_000,
                },
            )
            current_lineage_search = mcp_call(
                project,
                "ley_search",
                {
                    "query": controls_query,
                    "revisionCompatibility": "current-lineage",
                    "maxResults": 12,
                    "maxTokens": 4_000,
                },
            )
            divergent_session = cli_session_show(project, session_id)
            divergent_results = [
                item
                for item in divergent_search.get("results", [])
                if isinstance(item, dict)
            ]
            divergent_checkpoints = [
                item
                for item in divergent_session.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            branch_controls_pre_merge_ok = (
                divergent_search.get("revisionFilter") == "divergent"
                and bool(divergent_results)
                and all(
                    item.get("revisionApplicability", {}).get("compatibility")
                    == "divergent"
                    for item in divergent_results
                )
                and not current_lineage_search.get("results")
                and current_lineage_search.get("revisionFilter") == "current-lineage"
                and bool(divergent_checkpoints)
                and divergent_checkpoints[-1]
                .get("revisionApplicability", {})
                .get("compatibility")
                == "divergent"
                and divergent_session.get("revisionFreshness", {}).get(
                    "captureCompatibility"
                )
                == "divergent"
                and divergent_session.get("revisionFreshness", {}).get("liveGitChecked")
                is True
                and divergent_session.get("liveSourceChecked") is False
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
            "ley_brief",
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
        downstream_required_after_merge = [
            str(value)
            for value in revision_expectation.get(
                "downstream_required_after_merge", []
            )
        ]
        downstream_forbidden_while_divergent = [
            str(value)
            for value in revision_expectation.get(
                "downstream_forbidden_while_divergent", []
            )
        ]
        if downstream_required_after_merge or downstream_forbidden_while_divergent:
            downstream_revision_ok = task_contract_success(
                divergent,
                [],
                downstream_forbidden_while_divergent,
            ) and task_contract_success(
                merged,
                downstream_required_after_merge,
                [],
            )
            record_downstream_task_contract(
                scores,
                failures,
                downstream_revision_ok,
                "revision-aware context did not withhold divergent task state and re-admit it only after merge",
            )
        branch_controls_post_merge_ok = True
        branch_controls_evidence: list[object] = []
        if isinstance(branch_controls_expectation, dict):
            controls_query = str(branch_controls_expectation.get("query", query))
            merged_search = mcp_call(
                project,
                "ley_search",
                {
                    "query": controls_query,
                    "revisionCompatibility": "merged",
                    "maxResults": 12,
                    "maxTokens": 4_000,
                },
            )
            merged_session = cli_session_show(project, session_id)
            merged_results = [
                item
                for item in merged_search.get("results", [])
                if isinstance(item, dict)
            ]
            merged_checkpoints = [
                item
                for item in merged_session.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            branch_controls_post_merge_ok = (
                merged_search.get("revisionFilter") == "merged"
                and bool(merged_results)
                and all(
                    item.get("revisionApplicability", {}).get("compatibility") == "merged"
                    for item in merged_results
                )
                and bool(merged_checkpoints)
                and merged_checkpoints[-1]
                .get("revisionApplicability", {})
                .get("compatibility")
                == "merged"
                and merged_session.get("revisionFreshness", {}).get("captureCompatibility")
                == "merged"
                and merged_session.get("revisionFreshness", {}).get("liveGitChecked") is True
                and merged_session.get("liveSourceChecked") is False
            )
            scores["branch_worktree_controls"] = (
                branch_controls_pre_merge_ok and branch_controls_post_merge_ok
            )
            branch_controls_evidence = [
                divergent_search,
                current_lineage_search,
                divergent_session,
                merged_search,
                merged_session,
            ]
            if not scores["branch_worktree_controls"]:
                failures.append(
                    "branch/worktree controls did not filter exact revision applicability or refresh session compatibility across merge"
                )
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)],
            [divergent, divergent_host_start, merged, *branch_controls_evidence],
        )
        evidence_text.extend(
            [divergent, divergent_host_start, merged, *branch_controls_evidence]
        )
        if not revision_ok:
            failures.append(
                "revision adjudication did not withhold divergent state and re-admit proven merged history"
            )

    specification_expectation = scenario.get("expected_specification_compiler")
    if isinstance(specification_expectation, dict):
        query = str(specification_expectation.get("query", ""))
        compiled = mcp_call(
            project,
            "ley_brief",
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
        relevant_specification = next(
            (
                item
                for item in compiled.get("specifications", [])
                if isinstance(item, dict)
                and str(item.get("specificationId")) == relevant_id
            ),
            {},
        )
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
        downstream_required = [
            str(value)
            for value in specification_expectation.get("downstream_required", [])
        ]
        downstream_forbidden = [
            str(value)
            for value in specification_expectation.get("downstream_forbidden", [])
        ]
        if downstream_required or downstream_forbidden:
            record_downstream_task_contract(
                scores,
                failures,
                task_contract_success(
                    compiled,
                    downstream_required,
                    downstream_forbidden,
                ),
                "Specification context did not satisfy the independent downstream human-intent contract",
            )
        evidence_text.append(compiled)
        if not specification_ok:
            failures.append(
                "task-conditioned Specification admission did not preserve authority/budget/conflict semantics"
            )

    egress_expectation = scenario.get("expected_egress_policy")
    if isinstance(egress_expectation, dict):
        marker = str(egress_expectation.get("marker", ""))
        derived_marker = str(egress_expectation.get("derived_marker", ""))
        verification_method_marker = str(
            egress_expectation.get("verification_method_marker", "")
        )
        query = str(egress_expectation.get("query", ""))
        if not marker or not derived_marker or not verification_method_marker or not query:
            raise RuntimeError(
                "egress fixture requires marker, derived_marker, verification_method_marker, and query"
            )

        def advertised_tools(*, flags: tuple[str, ...] = ()) -> list[str]:
            return [
                str(tool.get("name", ""))
                for tool in mcp_tools_list(project, flags=flags)
                if isinstance(tool, dict)
            ]

        run(["egress", "project", "local-model-only", str(project), "--json"])
        cloud_tools = advertised_tools()
        cloud_blocked = (
            "ley_brief" not in cloud_tools
            and marker not in serialized(cloud_tools)
            and derived_marker not in serialized(cloud_tools)
            and verification_method_marker not in serialized(cloud_tools)
        )

        local_flags = ("--egress-target", "local")
        local_compiled = mcp_call(
            project,
            "ley_brief",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
            flags=local_flags,
        )
        local_text = json.dumps(local_compiled, sort_keys=True)
        local_allowed = (
            marker in local_text
            and verification_method_marker in local_text
            and derived_marker in local_text
            and local_compiled.get("egressTarget") == "local"
        )

        run(["egress", "project", "confirm-per-use", str(project), "--json"])
        confirm_tools = advertised_tools(flags=local_flags)
        confirm_blocked = (
            "ley_brief" not in confirm_tools
            and marker not in serialized(confirm_tools)
            and derived_marker not in serialized(confirm_tools)
            and verification_method_marker not in serialized(confirm_tools)
        )

        run(["egress", "project", "never-send", str(project), "--json"])
        never_tools = advertised_tools(flags=local_flags)
        never_blocked = (
            "ley_brief" not in never_tools
            and marker not in serialized(never_tools)
            and derived_marker not in serialized(never_tools)
            and verification_method_marker not in serialized(never_tools)
        )

        run(["egress", "project", "agent-ok", str(project), "--json"])
        restored_tools = advertised_tools()
        restored = mcp_call(
            project,
            "ley_brief",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
        )
        restored_text = json.dumps(restored, sort_keys=True)
        restored_ok = (
            "ley_brief" in restored_tools
            and marker in restored_text
            and derived_marker in restored_text
        )

        blocked_outputs = [cloud_tools, confirm_tools, never_tools]
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [marker, derived_marker, verification_method_marker, str(project), str(vault)],
            blocked_outputs,
        )
        egress_ok = cloud_blocked and local_allowed and confirm_blocked and never_blocked and restored_ok
        scores["egress_policy"] = egress_ok
        downstream_required_local = [
            str(value)
            for value in egress_expectation.get("downstream_required_local", [])
        ]
        downstream_forbidden_blocked = [
            str(value)
            for value in egress_expectation.get("downstream_forbidden_blocked", [])
        ]
        record_downstream_task_contract(
            scores,
            failures,
            task_contract_success(local_compiled, downstream_required_local, [])
            and all(
                all(marker_value not in serialized(payload) for marker_value in downstream_forbidden_blocked)
                for payload in blocked_outputs
            ),
            "project egress did not satisfy the independent local-allowed/blocked-target task contract",
        )
        evidence_text.extend([local_compiled, restored])
        if not egress_ok:
            failures.append(
                "project-level agent egress did not enforce local-model-only/confirm-per-use/never-send boundaries: "
                f"cloudBlocked={cloud_blocked}, localAllowed={local_allowed}, "
                f"confirmBlocked={confirm_blocked}, neverBlocked={never_blocked}, restored={restored_ok}; "
                f"cloudTools={cloud_tools!r}; confirmTools={confirm_tools!r}; neverTools={never_tools!r}"
            )

    historical_import_expectation = scenario.get("expected_historical_host_import")
    if isinstance(historical_import_expectation, dict):
        selected_session_id = str(
            historical_import_expectation.get("selected_session_id", "")
        )
        unrelated_session_id = str(
            historical_import_expectation.get("unrelated_session_id", "")
        )
        selected_markers = [
            str(value)
            for value in historical_import_expectation.get("selected_markers", [])
        ]
        unrelated_marker = str(
            historical_import_expectation.get("unrelated_marker", "")
        )
        secret_marker = str(
            historical_import_expectation.get("secret_marker", "")
        )
        changed_marker = str(
            historical_import_expectation.get("changed_marker", "")
        )
        source_timestamps = [
            int(value)
            for value in historical_import_expectation.get(
                "source_timestamps", []
            )
        ]
        if (
            not selected_session_id
            or not unrelated_session_id
            or len(selected_markers) != 2
            or len(source_timestamps) != 2
            or not unrelated_marker
            or not secret_marker
            or not changed_marker
        ):
            raise RuntimeError("historical host import fixture is incomplete")

        history_path = base_dir / "codex-history.jsonl"
        history_rows = [
            {
                "session_id": unrelated_session_id,
                "ts": source_timestamps[0] - 1,
                "text": unrelated_marker,
            },
            {
                "session_id": selected_session_id,
                "ts": source_timestamps[0],
                "text": f"{selected_markers[0]} api_key={secret_marker}",
            },
            {
                "session_id": selected_session_id,
                "ts": source_timestamps[1],
                "text": selected_markers[1],
            },
        ]
        history_path.write_text(
            "".join(
                json.dumps(row, separators=(",", ":")) + "\n"
                for row in history_rows
            ),
            encoding="utf-8",
        )

        before_resume = cli_json(["resume", str(project), "--json"])
        imported = cli_json(
            [
                "session",
                "import",
                "codex-history",
                str(project),
                "--source",
                str(history_path),
                "--host-session",
                selected_session_id,
                "--json",
            ]
        )
        if not isinstance(imported, dict):
            raise RuntimeError("historical host import returned no receipt")
        imported_session_id = str(imported.get("sessionId", ""))
        source_reference = str(imported.get("sourceReference", ""))
        session_context = cli_json(
            [
                "session",
                "show",
                imported_session_id,
                str(project),
                "--json",
            ]
        )
        turns = cli_json(
            [
                "session",
                "turns",
                imported_session_id,
                str(project),
                "--max-results",
                "20",
                "--max-characters",
                "12000",
                "--json",
            ]
        )
        import_tools = mcp_tools_list(project)
        resume = cli_json(["resume", str(project), "--json"])
        search = cli_json(
            [
                "search",
                "Imported Codex history",
                str(project),
                "--max-results",
                "8",
                "--max-tokens",
                "1500",
                "--json",
            ]
        )
        retry = cli_json(
            [
                "session",
                "import",
                "codex-history",
                str(project),
                "--source",
                str(history_path),
                "--host-session",
                selected_session_id,
                "--json",
            ]
        )

        history_rows.append(
            {
                "session_id": selected_session_id,
                "ts": source_timestamps[1] + 1,
                "text": changed_marker,
            }
        )
        history_path.write_text(
            "".join(
                json.dumps(row, separators=(",", ":")) + "\n"
                for row in history_rows
            ),
            encoding="utf-8",
        )
        changed_import = cli_json(
            [
                "session",
                "import",
                "codex-history",
                str(project),
                "--source",
                str(history_path),
                "--host-session",
                selected_session_id,
                "--json",
            ]
        )
        changed_session_id = (
            str(changed_import.get("sessionId", ""))
            if isinstance(changed_import, dict)
            else ""
        )
        history_path.unlink()
        changed_turns_after_delete = cli_json(
            [
                "session",
                "turns",
                changed_session_id,
                str(project),
                "--max-results",
                "20",
                "--max-characters",
                "12000",
                "--json",
            ]
        )

        import_shape_ok = (
            imported.get("schemaVersion") == 1
            and imported.get("host") == "codex"
            and imported.get("sessionSourceKind") == "import"
            and imported.get("sourceKind") == "codex-message-history"
            and imported.get("matchedPrompts") == 2
            and imported.get("capturedPrompts") == 2
            and imported.get("assistantMessagesImported") == 0
            and imported.get("replayed") is False
            and imported.get("sourcePathRetained") is False
            and imported.get("rawHostSessionIdRetained") is False
            and imported.get("liveSourceChecked") is False
            and imported.get("authority") == "untrusted-historical-evidence"
            and source_reference.startswith("hsi_")
            and imported_session_id.startswith("ses_")
        )
        expected_source_ms = [value * 1000 for value in source_timestamps]
        session_source_ok = (
            isinstance(session_context, dict)
            and session_context.get("schemaVersion") == 7
            and session_context.get("status") == "completed"
            and session_context.get("promptCount") == 2
            and session_context.get("responseCount") == 0
            and isinstance(session_context.get("source"), dict)
            and session_context["source"].get("kind") == "import"
            and session_context["source"].get("host") == "codex"
            and session_context["source"].get("sourceReference")
            == source_reference
            and session_context["source"].get("agent") is None
        )
        returned_turns = (
            turns.get("turns", []) if isinstance(turns, dict) else []
        )
        turns_ok = (
            isinstance(turns, dict)
            and turns.get("schemaVersion") == 7
            and turns.get("promptCount") == 2
            and turns.get("responseCount") == 0
            and turns.get("retainedTurnCount") == 2
            and turns.get("liveSourceChecked") is False
            and len(returned_turns) == 2
            and all(
                isinstance(turn, dict)
                and turn.get("origin") == "import"
                and turn.get("host") == "codex"
                and turn.get("sourceBoundary")
                == "untrusted-imported-host-history"
                for turn in returned_turns
            )
            and [
                int(turn.get("sourceRecordedAtUnixMs", 0))
                for turn in returned_turns
                if isinstance(turn, dict)
            ]
            == expected_source_ms
            and all(
                marker in json.dumps(returned_turns, sort_keys=True)
                for marker in selected_markers
            )
        )
        import_inspection_ok = (
            all(
                isinstance(turn, dict)
                and turn.get("origin") == "import"
                and turn.get("sourceBoundary") == "untrusted-imported-host-history"
                for turn in returned_turns
            )
            and "ley_session_memory_compile"
            not in {
                str(tool.get("name", ""))
                for tool in import_tools
                if isinstance(tool, dict)
            }
        )
        resume_ok = (
            isinstance(before_resume, dict)
            and before_resume.get("totalSessions") == 0
            and isinstance(resume, dict)
            and resume.get("totalSessions") == 1
            and resume.get("excludedImportedSessions") == 1
            and resume.get("sessions") == []
        )
        search_results = (
            search.get("results", []) if isinstance(search, dict) else []
        )
        historical_search_ok = any(
            isinstance(item, dict)
            and item.get("kind") == "session"
            and item.get("entityId") == imported_session_id
            and item.get("updatedAtUnixMs") == expected_source_ms[-1]
            for item in search_results
        )
        replay_ok = (
            isinstance(retry, dict)
            and retry.get("replayed") is True
            and retry.get("sessionId") == imported_session_id
            and retry.get("sourceReference") == source_reference
        )
        changed_snapshot_ok = (
            isinstance(changed_import, dict)
            and changed_import.get("replayed") is False
            and changed_import.get("sessionId") != imported_session_id
            and changed_import.get("sourceReference") == source_reference
            and changed_import.get("matchedPrompts") == 3
            and isinstance(changed_turns_after_delete, dict)
            and changed_turns_after_delete.get("promptCount") == 3
            and changed_turns_after_delete.get("responseCount") == 0
            and changed_turns_after_delete.get("liveSourceChecked") is False
            and changed_marker
            in json.dumps(
                changed_turns_after_delete.get("turns", []), sort_keys=True
            )
        )
        observable_outputs = [
            imported,
            session_context,
            turns,
            import_tools,
            resume,
            search,
            retry,
            changed_import,
            changed_turns_after_delete,
        ]
        privacy_rate = privacy_violation_rate(
            [
                str(history_path),
                selected_session_id,
                unrelated_session_id,
                unrelated_marker,
                secret_marker,
            ],
            observable_outputs,
        )
        history_text = json.dumps(observable_outputs, sort_keys=True)
        privacy_ok = (
            privacy_rate == 0.0
            and unrelated_marker not in history_text
            and secret_marker not in history_text
        )
        historical_host_import_ok = (
            import_shape_ok
            and session_source_ok
            and turns_ok
            and import_inspection_ok
            and resume_ok
            and historical_search_ok
            and replay_ok
            and changed_snapshot_ok
            and privacy_ok
        )
        scores["historical_host_import"] = historical_host_import_ok
        downstream_required = [
            str(value)
            for value in historical_import_expectation.get(
                "downstream_required",
                selected_markers,
            )
        ]
        downstream_forbidden = [
            str(value)
            for value in historical_import_expectation.get(
                "downstream_forbidden",
                [unrelated_marker, secret_marker],
            )
        ]
        record_downstream_task_contract(
            scores,
            failures,
            historical_search_ok
            and text_contract_success(
                json.dumps(returned_turns, sort_keys=True),
                downstream_required,
                downstream_forbidden,
            ),
            "explicit historical host import did not satisfy the independent downstream bounded-evidence contract",
        )
        scores["privacy_violation_rate"] = privacy_rate
        evidence_text.extend(observable_outputs)
        if not historical_host_import_ok:
            failed_checks = [
                name
                for name, passed in [
                    ("import-shape", import_shape_ok),
                    ("session-source", session_source_ok),
                    ("turns", turns_ok),
                    ("native-import-inspection", import_inspection_ok),
                    ("resume-exclusion", resume_ok),
                    ("historical-search-time", historical_search_ok),
                    ("idempotent-replay", replay_ok),
                    ("immutable-changed-snapshot", changed_snapshot_ok),
                    ("privacy", privacy_ok),
                ]
                if not passed
            ]
            failures.append(
                "historical Codex message import failed: "
                + ", ".join(failed_checks)
            )

    consolidation_expectation = scenario.get("expected_consolidation_inbox")
    if isinstance(consolidation_expectation, dict):
        active_marker = str(consolidation_expectation.get("active_marker", ""))
        prompt_marker = str(consolidation_expectation.get("prompt_marker", ""))
        response_marker = str(consolidation_expectation.get("response_marker", ""))
        if not active_marker or not prompt_marker or not response_marker:
            raise RuntimeError("consolidation inbox fixture is incomplete")

        active = cli_json(
            [
                "session",
                "start",
                str(project),
                "--name",
                "Active consolidation control",
                "--goal",
                "Remain outside meaningful-boundary consolidation",
                "--json",
            ]
        )
        if not isinstance(active, dict):
            raise RuntimeError("consolidation fixture returned no active session")
        active_session = active.get("session", {})
        active_session_id = (
            str(active_session.get("sessionId", ""))
            if isinstance(active_session, dict)
            else ""
        )
        json.loads(
            run(
                [
                    "session",
                    "prompt",
                    active_session_id,
                    str(project),
                    "--stdin",
                    "--json",
                ],
                stdin=active_marker,
            )
        )

        terminal = cli_json(
            [
                "session",
                "start",
                str(project),
                "--name",
                "Completed consolidation boundary",
                "--goal",
                "Review retained evidence without rewriting terminal history",
                "--json",
            ]
        )
        if not isinstance(terminal, dict):
            raise RuntimeError("consolidation fixture returned no terminal session")
        terminal_session = terminal.get("session", {})
        terminal_session_id = (
            str(terminal_session.get("sessionId", ""))
            if isinstance(terminal_session, dict)
            else ""
        )
        json.loads(
            run(
                [
                    "session",
                    "prompt",
                    terminal_session_id,
                    str(project),
                    "--stdin",
                    "--json",
                ],
                stdin=prompt_marker,
            )
        )
        json.loads(
            run(
                [
                    "session",
                    "response",
                    terminal_session_id,
                    str(project),
                    "--stdin",
                    "--json",
                ],
                stdin=response_marker,
            )
        )
        turns = cli_json(
            [
                "session",
                "turns",
                terminal_session_id,
                str(project),
                "--json",
            ]
        )
        if not isinstance(turns, dict):
            raise RuntimeError("consolidation fixture returned no terminal turns")
        retained_turn_ids = [
            str(turn.get("recordId", ""))
            for turn in turns.get("turns", [])
            if isinstance(turn, dict)
            and turn.get("kind") in {"user-prompt", "assistant-response"}
            and isinstance(turn.get("recordId"), str)
        ]
        if len(retained_turn_ids) != 2:
            raise RuntimeError("consolidation fixture did not retain exactly two turns")

        finished = cli_json(
            [
                "session",
                "finish",
                terminal_session_id,
                str(project),
                "--summary",
                "Completed without a final structured checkpoint",
                "--status",
                "completed",
                "--json",
            ]
        )
        if not isinstance(finished, dict):
            raise RuntimeError("consolidation fixture returned no finish receipt")
        finished_session = finished.get("session", {})
        terminal_event_count = (
            int(finished_session.get("eventCount", 0))
            if isinstance(finished_session, dict)
            else 0
        )

        inbox = cli_json(
            [
                "consolidation",
                "inbox",
                str(project),
                "--max-items",
                "20",
                "--max-sessions",
                "30",
                "--json",
            ]
        )
        rebuilt = cli_json(
            [
                "consolidation",
                "inbox",
                str(project),
                "--max-items",
                "20",
                "--max-sessions",
                "30",
                "--json",
            ]
        )
        candidates = [
            item
            for item in inbox.get("items", [])
            if isinstance(item, dict)
            and item.get("sessionId") == terminal_session_id
        ]
        candidate = candidates[0] if len(candidates) == 1 else {}
        proposal_ids = (
            [str(value) for value in candidate.get("proposalEvidenceRecordIds", [])]
            if isinstance(candidate, dict)
            else []
        )
        inbox_text = json.dumps([inbox, rebuilt], sort_keys=True)
        inbox_ok = (
            inbox.get("schemaVersion") == 2
            and inbox.get("persisted") is False
            and inbox.get("modelInvoked") is False
            and inbox.get("backgroundWorkStarted") is False
            and inbox.get("destructiveActionsTaken") is False
            and inbox.get("liveSourceChecked") is False
            and inbox.get("inboxFingerprint") == rebuilt.get("inboxFingerprint")
            and int(inbox.get("coverage", {}).get("excludedActiveSessions", 0)) >= 1
            and inbox.get("coverage", {}).get("allEligibleSessionsInspected") is True
            and int(
                inbox.get("coverage", {}).get(
                    "inspectedSessionsWithUnconsolidatedEvidence", 0
                )
            )
            == 1
            and len(candidates) == 1
            and candidate.get("sessionStatus") == "completed"
            and candidate.get("automaticWriteAllowed") is False
            and candidate.get("semanticFaithfulnessProven") is False
            and set(proposal_ids) == set(retained_turn_ids)
            and active_session_id not in {
                str(item.get("sessionId", ""))
                for item in inbox.get("items", [])
                if isinstance(item, dict)
            }
            and active_marker not in inbox_text
            and prompt_marker not in inbox_text
            and response_marker not in inbox_text
            and str(project) not in inbox_text
            and str(vault) not in inbox_text
        )

        proposal_args = [
            "learning",
            "propose",
            str(project),
            "--request-id",
            request_id(f"{scenario['id']}:consolidation:learning"),
            "--actor",
            "agent",
            "--provenance",
            "inferred",
            "--kind",
            "procedure",
            "--title",
            "Review completed retained evidence",
            "--guidance",
            "Inspect the retained evidence before reusing this completed workflow.",
            "--confidence",
            "60",
        ]
        for record_id in proposal_ids:
            proposal_args.extend(
                ["--evidence", f"{terminal_session_id}:{record_id}"]
            )
        proposal_args.append("--json")
        proposed = cli_json(proposal_args)
        proposed_learning = (
            proposed.get("learning", {}) if isinstance(proposed, dict) else {}
        )
        learning_id = str(proposed_learning.get("learningId", ""))
        learning = cli_json(
            ["learning", "show", learning_id, str(project), "--json"]
        )
        lineage = learning.get("originLineage", {})
        sources = lineage.get("sources", []) if isinstance(lineage, dict) else []
        after = cli_json(
            ["session", "show", terminal_session_id, str(project), "--json"]
        )
        after_event_count = (
            int(after.get("eventCount", 0)) if isinstance(after, dict) else 0
        )
        proposal_ok = (
            proposed_learning.get("state") == "tentative"
            and proposed_learning.get("trustState") == "review-required"
            and learning.get("state") == "tentative"
            and learning.get("trustState") == "review-required"
            and lineage.get("automaticAuthorityCeiling") == "review-required"
            and lineage.get("causalCompletenessProven") is False
            and len(sources) == 2
            and all(
                isinstance(source, dict)
                and source.get("kind") == "turn-evidence"
                and source.get("sessionId") == terminal_session_id
                and source.get("recordId") in retained_turn_ids
                for source in sources
            )
            and terminal_event_count > 0
            and after_event_count == terminal_event_count
        )
        observable_outputs = [inbox, rebuilt, proposed, learning, after]
        privacy_rate = privacy_violation_rate(
            [
                str(project),
                str(vault),
                active_marker,
                prompt_marker,
                response_marker,
            ],
            observable_outputs,
        )
        consolidation_ok = inbox_ok and proposal_ok and privacy_rate == 0.0
        scores["consolidation_inbox"] = consolidation_ok
        scores["privacy_violation_rate"] = privacy_rate
        evidence_text.extend(observable_outputs)
        if not consolidation_ok:
            failed_checks = [
                name
                for name, passed in [
                    ("read-only-inbox", inbox_ok),
                    ("review-required-proposal", proposal_ok),
                    ("privacy", privacy_rate == 0.0),
                ]
                if not passed
            ]
            failures.append(
                "local consolidation inbox failed: " + ", ".join(failed_checks)
            )

    verification_evidence_expectation = scenario.get("expected_verification_evidence")
    if isinstance(verification_evidence_expectation, dict):
        if not session_id:
            raise RuntimeError("verification evidence fixture created no structured session")
        evidence_path = str(verification_evidence_expectation.get("evidence_path", ""))
        verification_marker = str(
            verification_evidence_expectation.get("verification_marker", "")
        )
        live_mutation_marker = str(
            verification_evidence_expectation.get("live_mutation_marker", "")
        )
        evidence_file = project / evidence_path
        captured_bytes = evidence_file.read_bytes()
        captured_text = captured_bytes.decode("utf-8").strip()
        captured_hash = "sha256:" + hashlib.sha256(captured_bytes).hexdigest()
        if live_mutation_marker:
            evidence_file.write_text(
                f"{live_mutation_marker}\nThis live file changed after the Ley capture.\n",
                encoding="utf-8",
            )
        live_hash = "sha256:" + hashlib.sha256(evidence_file.read_bytes()).hexdigest()
        session_context = cli_session_show(project, session_id)
        checkpoint_rows = session_context.get("checkpoints", [])
        verification_rows = [
            item
            for checkpoint in checkpoint_rows
            if isinstance(checkpoint, dict)
            for item in checkpoint.get("verification", [])
            if isinstance(item, dict)
        ]
        matching = next(
            (
                item
                for item in verification_rows
                if not verification_marker
                or verification_marker in json.dumps(item, sort_keys=True)
            ),
            None,
        )
        evidence_rows = (
            matching.get("evidenceArtifacts", []) if isinstance(matching, dict) else []
        )
        citation = next(
            (
                item
                for item in evidence_rows
                if isinstance(item, dict) and item.get("artifactPath") == evidence_path
            ),
            None,
        )
        exact_evidence = (
            mcp_call(
                project,
                "ley_evidence",
                {
                    "reference": {
                        "artifactPath": citation.get("artifactPath"),
                        "startLine": int(citation.get("startLine", 0) or 0),
                        "startColumn": int(citation.get("startColumn", 0) or 0),
                        "endLine": int(citation.get("endLine", 0) or 0),
                        "endColumn": int(citation.get("endColumn", 0) or 0),
                        "artifactSnapshotId": citation.get("artifactSnapshotId"),
                        "contentHash": citation.get("contentHash"),
                        "mediaType": citation.get("mediaType"),
                    },
                    "contextLines": 0,
                    "maxCharacters": 8_000,
                },
            )
            if isinstance(citation, dict)
            else {}
        )
        host_external_session_id = f"{scenario['id']}-live-source-host"
        host_startup = hook_call(
            project,
            "codex",
            {
                "hook_event_name": "SessionStart",
                "session_id": host_external_session_id,
            },
        )
        host_ley_session_id = hook_ley_session_id(host_startup)
        if not host_ley_session_id:
            raise RuntimeError(
                "verification evidence live-source fixture did not resolve the host Ley session"
            )
        host_prompt = hook_call(
            project,
            "codex",
            {
                "hook_event_name": "UserPromptSubmit",
                "session_id": host_external_session_id,
                "turn_id": "verification-live-source-turn",
                "prompt": (
                    f"Inspect the current {evidence_path} before any consequential edit."
                ),
            },
        )
        host_prompt_context = hook_additional_context(host_prompt)
        host_brief = mcp_call(
            project,
            "ley_brief",
            {
                "task": f"Inspect the current {evidence_path} before any consequential edit.",
                "maxResults": 8,
                "maxTokens": 1_500,
            },
        )
        live_read_command = f"cat -- {evidence_path}"
        live_read = subprocess.run(
            ["cat", "--", evidence_path],
            cwd=project,
            capture_output=True,
            text=True,
            check=False,
        )
        if live_read.returncode != 0:
            raise RuntimeError(
                "verification evidence live-source fixture could not read the current workspace file: "
                + live_read.stderr.strip()
            )
        live_read_output = live_read.stdout
        hook_call(
            project,
            "codex",
            {
                "hook_event_name": "PostToolUse",
                "session_id": host_external_session_id,
                "turn_id": "verification-live-source-turn",
                "tool_name": "Bash",
                "tool_use_id": "verification-live-source-read",
                "tool_input": {"command": live_read_command},
                "tool_response": {
                    "output": live_read_output,
                    "metadata": {"exit_code": live_read.returncode},
                },
            },
        )
        host_turns = cli_session_turns(
            project,
            host_ley_session_id,
            max_results=20,
            max_characters=16_000,
        )
        host_session = cli_session_show(project, host_ley_session_id)
        matching_host_tool_rows = [
            item
            for item in host_turns.get("toolObservations", [])
            if isinstance(item, dict)
            and item.get("toolName") == "Bash"
            and item.get("command") == live_read_command
        ]
        host_tool_row = (
            matching_host_tool_rows[0]
            if len(matching_host_tool_rows) == 1
            else None
        )
        expected_tool_result = (
            f"metadata.exit_code: {live_read.returncode}\noutput: {live_read_output}"
        ).strip()
        session_text = json.dumps(session_context, sort_keys=True)
        evidence_result_text = json.dumps(exact_evidence, sort_keys=True)
        host_context_lower = host_prompt_context.lower()
        host_brief_text = context_contract_text(host_brief)
        host_brief_gaps = json.dumps(host_brief.get("gaps", []), sort_keys=True).lower()
        live_host_honesty_ok = (
            host_ley_session_id.startswith("ses_")
            and "does not inject task-specific project history" in host_context_lower
            and "call ley_brief" in host_context_lower
            and "# ley task context (automatic)" not in host_context_lower
            and host_brief.get("liveSourceChecked") is False
            and "live-source-unchecked" in host_brief_gaps
            and "inspect live source before consequential" in host_brief_gaps
            and (not live_mutation_marker or live_mutation_marker not in host_prompt_context)
            and (not live_mutation_marker or live_mutation_marker not in host_brief_text)
            and live_read.returncode == 0
            and (not live_mutation_marker or live_mutation_marker in live_read_output)
            and hashlib.sha256(live_read_output.encode("utf-8")).hexdigest()
            == live_hash.removeprefix("sha256:")
            and len(matching_host_tool_rows) == 1
            and isinstance(host_tool_row, dict)
            and host_tool_row.get("observationKind") == "returned"
            and host_tool_row.get("command") == live_read_command
            and host_tool_row.get("result") == expected_tool_result
            and host_tool_row.get("commandTruncatedAtCapture") is False
            and host_tool_row.get("resultTruncatedAtCapture") is False
            and host_tool_row.get("commandTruncatedForContext") is False
            and host_tool_row.get("resultTruncatedForContext") is False
            and host_turns.get("liveSourceChecked") is False
            and host_session.get("checkpointCount") == 0
            and not host_session.get("checkpoints")
            and host_session.get("liveSourceChecked") is False
            and str(project) not in host_prompt_context
            and str(vault) not in host_prompt_context
            and str(project) not in json.dumps(host_brief, sort_keys=True)
            and str(vault) not in json.dumps(host_brief, sort_keys=True)
            and str(project) not in json.dumps(host_turns, sort_keys=True)
            and str(vault) not in json.dumps(host_turns, sort_keys=True)
        )
        verification_evidence_ok = (
            matching is not None
            and citation is not None
            and citation.get("contentHash") == captured_hash
            and str(citation.get("artifactSnapshotId", "")).startswith("snp_")
            and int(citation.get("startLine", 0)) >= 1
            and int(citation.get("endLine", 0)) >= int(citation.get("startLine", 0))
            and exact_evidence.get("artifactSnapshotId") == citation.get("artifactSnapshotId")
            and isinstance(exact_evidence.get("citation"), dict)
            and exact_evidence["citation"].get("contentHash") == captured_hash
            and captured_text in str(exact_evidence.get("text", ""))
            and captured_hash != live_hash
            and session_context.get("liveSourceChecked") is False
            and (not live_mutation_marker or live_mutation_marker not in session_text)
            and (not live_mutation_marker or live_mutation_marker not in evidence_result_text)
            and str(project) not in session_text
            and str(vault) not in session_text
            and str(project) not in evidence_result_text
            and str(vault) not in evidence_result_text
            and live_host_honesty_ok
        )
        scores["verification_evidence_links"] = verification_evidence_ok
        scores["live_source_honesty"] = live_host_honesty_ok
        privacy_canaries = [str(project), str(vault)]
        if live_mutation_marker:
            privacy_canaries.append(live_mutation_marker)
        scores["privacy_violation_rate"] = privacy_violation_rate(
            privacy_canaries, [session_context, exact_evidence, host_prompt_context, host_brief]
        )
        evidence_text.extend(
            [session_context, exact_evidence, host_startup, host_prompt, host_brief, host_turns, host_session]
        )
        if not verification_evidence_ok:
            failures.append(
                "verification evidence links/live-source host handoff did not preserve immutable historical provenance plus non-authoritative current workspace observation; "
                f"liveHost={live_host_honesty_ok}, "
                f"matchingRows={len(matching_host_tool_rows)}, "
                f"hostToolRow={json.dumps(host_tool_row, sort_keys=True) if isinstance(host_tool_row, dict) else host_tool_row}, "
                f"expectedToolResult={expected_tool_result!r}"
            )

    multimodal_expectation = scenario.get("expected_multimodal_evidence")
    if isinstance(multimodal_expectation, dict):
        if not session_id:
            raise RuntimeError("multimodal evidence fixture created no structured session")
        evidence_path = str(multimodal_expectation.get("evidence_path", ""))
        expected_media_type = str(multimodal_expectation.get("media_type", ""))
        expected_mime_type = str(multimodal_expectation.get("mime_type", ""))
        search_marker = str(multimodal_expectation.get("search_marker", "")).strip()
        binary_definition = scenario.get("project_binary_files", {})
        if (
            not isinstance(binary_definition, dict)
            or not isinstance(binary_definition.get(evidence_path), str)
        ):
            raise RuntimeError(
                "multimodal evidence fixture requires a matching base64 project binary file"
            )
        expected_bytes = base64.b64decode(
            str(binary_definition[evidence_path]), validate=True
        )
        expected_hash = "sha256:" + hashlib.sha256(expected_bytes).hexdigest()

        session_context = cli_session_show(project, session_id)
        citation = next(
            (
                evidence
                for checkpoint in session_context.get("checkpoints", [])
                if isinstance(checkpoint, dict)
                for verification in checkpoint.get("verification", [])
                if isinstance(verification, dict)
                for evidence in verification.get("evidenceArtifacts", [])
                if isinstance(evidence, dict)
                and evidence.get("artifactPath") == evidence_path
            ),
            None,
        )
        if not isinstance(citation, dict):
            raise RuntimeError("multimodal evidence fixture produced no media citation")

        live_marker = b"live-source-drift-after-media-capture"
        (project / evidence_path).write_bytes(live_marker)

        memory_search: dict[str, object] = {}
        memory_citation: dict[str, object] | None = None
        if search_marker:
            memory_search = mcp_call(
                project,
                "ley_search",
                {"query": search_marker, "maxResults": 8, "maxTokens": 2000},
            )
            memory_citation = next(
                (
                    result.get("citation")
                    for result in memory_search.get("results", [])
                    if isinstance(result, dict)
                    and result.get("kind") == "decision"
                    and isinstance(result.get("citation"), dict)
                ),
                None,
            )

        media_payload, media_result = mcp_call_result(
            project,
            "ley_evidence",
            {
                "reference": {
                    "artifactPath": evidence_path,
                    "startLine": int(citation.get("startLine", 0) or 0),
                    "startColumn": int(citation.get("startColumn", 0) or 0),
                    "endLine": int(citation.get("endLine", 0) or 0),
                    "endColumn": int(citation.get("endColumn", 0) or 0),
                    "artifactSnapshotId": citation.get("artifactSnapshotId"),
                    "contentHash": citation.get("contentHash"),
                    "mediaType": citation.get("mediaType"),
                },
                "maxBytes": len(expected_bytes),
            },
        )
        image_block = next(
            (
                item
                for item in media_result.get("content", [])
                if isinstance(item, dict) and item.get("type") == "image"
            ),
            None,
        )
        returned_bytes = b""
        if isinstance(image_block, dict) and isinstance(image_block.get("data"), str):
            returned_bytes = base64.b64decode(str(image_block["data"]), validate=True)
        tool_names = {
            str(tool.get("name", ""))
            for tool in mcp_tools_list(project)
            if isinstance(tool, dict)
        }
        serialized_media = json.dumps(media_payload, sort_keys=True)
        search_routes_ok = (
            not search_marker
            or (
                isinstance(memory_citation, dict)
                and memory_citation.get("contentHash") == expected_hash
                and memory_citation.get("mediaType") == expected_media_type
                and memory_citation.get("startLine") == 0
                and memory_citation.get("endLine") == 0
            )
        )
        multimodal_ok = (
            "ley_evidence" in tool_names
            and session_context.get("schemaVersion") == 6
            and citation.get("contentHash") == expected_hash
            and citation.get("mediaType") == expected_media_type
            and citation.get("startLine") == 0
            and citation.get("endLine") == 0
            and media_payload.get("artifactPath") == evidence_path
            and media_payload.get("artifactSnapshotId") == citation.get("artifactSnapshotId")
            and media_payload.get("contentHash") == expected_hash
            and media_payload.get("mediaType") == expected_media_type
            and media_payload.get("mimeType") == expected_mime_type
            and media_payload.get("sourceBytes") == len(expected_bytes)
            and media_payload.get("deliveredBytes") == len(expected_bytes)
            and media_payload.get("evidenceRole") == "original-media"
            and media_payload.get("sourceBoundary") == "untrusted-project-evidence"
            and media_payload.get("liveSourceChecked") is False
            and media_payload.get("derivedDescriptionIncluded") is False
            and isinstance(image_block, dict)
            and image_block.get("mimeType") == expected_mime_type
            and returned_bytes == expected_bytes
            and returned_bytes != live_marker
            and search_routes_ok
            and str(project) not in serialized_media
            and str(vault) not in serialized_media
        )
        scores["multimodal_evidence"] = multimodal_ok
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault), live_marker.decode("ascii")],
            [session_context, memory_search, media_payload],
        )
        evidence_text.extend([session_context, memory_search, media_payload])
        if not multimodal_ok:
            failures.append(
                "multimodal evidence did not preserve exact original media, immutable citation provenance, canonical search/evidence routing, non-text semantics, or bounded MCP image delivery"
            )

    abstention_expectation = scenario.get("expected_selective_abstention")
    if isinstance(abstention_expectation, dict):
        query = str(abstention_expectation.get("query", ""))
        compiled = mcp_call(
            project,
            "ley_brief",
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
            touched_artifacts=["README.md"],
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
            touched_artifacts=["README.md"],
            host="claude-code",
        )
        context_a = cli_session_show(project, session_a)
        context_b = cli_session_show(project, session_b)
        compiled = mcp_call(
            project,
            "ley_brief",
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

        reconciliation = parallel_expectation.get("reconciliation")
        if isinstance(reconciliation, dict):
            title_reconciled = str(reconciliation.get("title", ""))
            guidance_reconciled = str(reconciliation.get("guidance", ""))
            marker_reconciled = str(reconciliation.get("marker", ""))
            query_reconciled = str(reconciliation.get("query", title_reconciled))
            checkpoints_a = [
                item
                for item in context_a.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            checkpoints_b = [
                item
                for item in context_b.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            if len(checkpoints_a) != 1 or len(checkpoints_b) != 1:
                raise RuntimeError(
                    "parallel reconciliation fixture expected exactly one checkpoint per session"
                )
            checkpoint_a = checkpoints_a[0]
            checkpoint_b = checkpoints_b[0]
            checkpoint_id_a = str(checkpoint_a.get("checkpointId", ""))
            checkpoint_id_b = str(checkpoint_b.get("checkpointId", ""))
            decisions_a = [
                item
                for item in checkpoint_a.get("decisions", [])
                if isinstance(item, dict)
            ]
            decisions_b = [
                item
                for item in checkpoint_b.get("decisions", [])
                if isinstance(item, dict)
            ]
            if len(decisions_a) != 1 or len(decisions_b) != 1:
                raise RuntimeError(
                    "parallel reconciliation fixture expected one decision per session"
                )
            decision_id_a = str(decisions_a[0].get("id", ""))
            decision_id_b = str(decisions_b[0].get("id", ""))
            before_checkpoint_a = json.dumps(checkpoint_a, sort_keys=True)
            before_checkpoint_b = json.dumps(checkpoint_b, sort_keys=True)

            proposed = cli_json(
                [
                    "learning",
                    "propose",
                    str(project),
                    "--request-id",
                    request_id(f"{scenario['id']}:parallel:reconcile:proposal"),
                    "--actor",
                    "agent",
                    "--provenance",
                    "inferred",
                    "--kind",
                    "fact",
                    "--title",
                    title_reconciled,
                    "--guidance",
                    guidance_reconciled,
                    "--confidence",
                    "95",
                    "--evidence",
                    f"{session_a}:{checkpoint_id_a}",
                    "--evidence",
                    f"{session_b}:{checkpoint_id_b}",
                    "--json",
                ]
            )
            proposed_learning = (
                proposed.get("learning", {}) if isinstance(proposed, dict) else {}
            )
            learning_id = str(proposed_learning.get("learningId", ""))
            reviewed = cli_json(
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
                    "Reviewed both parallel workstreams and accepted one project-level synthesis.",
                    "--request-id",
                    request_id(f"{scenario['id']}:parallel:reconcile:review"),
                    "--json",
                ]
            )
            learning = cli_json(
                ["learning", "show", learning_id, str(project), "--json"]
            )
            trusted_list = cli_json(["learning", "list", str(project), "--json"])
            search_after = mcp_call(
                project,
                "ley_search",
                {"query": query_reconciled, "maxResults": 10, "maxTokens": 2_000},
            )
            compiled_after = mcp_call(
                project,
                "ley_brief",
                {"task": query_reconciled, "maxResults": 10, "maxTokens": 2_000},
            )
            context_a_after = cli_json(
                ["session", "show", session_a, str(project), "--json"]
            )
            context_b_after = cli_json(
                ["session", "show", session_b, str(project), "--json"]
            )
            after_checkpoints_a = [
                item
                for item in context_a_after.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            after_checkpoints_b = [
                item
                for item in context_b_after.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            sessions_unchanged = (
                len(after_checkpoints_a) == 1
                and len(after_checkpoints_b) == 1
                and json.dumps(after_checkpoints_a[0], sort_keys=True) == before_checkpoint_a
                and json.dumps(after_checkpoints_b[0], sort_keys=True) == before_checkpoint_b
                and marker_a in json.dumps(context_a_after, sort_keys=True)
                and marker_b not in json.dumps(context_a_after, sort_keys=True)
                and marker_b in json.dumps(context_b_after, sort_keys=True)
                and marker_a not in json.dumps(context_b_after, sort_keys=True)
            )

            evidence_rows = [
                item for item in learning.get("evidence", []) if isinstance(item, dict)
            ]
            evidence_pairs = {
                (str(item.get("sessionId", "")), str(item.get("recordId", "")))
                for item in evidence_rows
            }
            origin_lineage = learning.get("originLineage", {})
            reviewed_learning = (
                reviewed.get("learning", {}) if isinstance(reviewed, dict) else {}
            )
            origin_lineage_ok = (
                evidence_pairs
                == {(session_a, checkpoint_id_a), (session_b, checkpoint_id_b)}
                and isinstance(origin_lineage, dict)
                and origin_lineage.get("automaticAuthorityCeiling") == "review-required"
                and origin_lineage.get("causalCompletenessProven") is False
            )
            trusted_learning_ok = (
                reviewed_learning.get("state") == "verified"
                and reviewed_learning.get("trustState") == "trusted"
                and learning.get("state") == "verified"
                and learning.get("trustState") == "trusted"
                and learning.get("freshness") == "current"
                and learning.get("corroboratingSessions") == 2
                and origin_lineage_ok
                and any(
                    isinstance(item, dict)
                    and item.get("learningId") == learning_id
                    and item.get("state") == "verified"
                    and item.get("trustState") == "trusted"
                    and item.get("freshness") == "current"
                    for item in trusted_list
                )
                and any(
                    isinstance(item, dict)
                    and item.get("kind") == "learning"
                    and item.get("learningId") == learning_id
                    and item.get("trustSignal") == "trusted-current"
                    and item.get("trustedForReuse") is True
                    for item in search_after.get("results", [])
                )
            )

            compiled_learning_ok = any(
                isinstance(item, dict)
                and item.get("learningId") == learning_id
                and item.get("trustSignal") == "trusted-current"
                and item.get("trustedForReuse") is True
                and marker_reconciled in json.dumps(item)
                for item in compiled_after.get("items", [])
            )
            decisions_withheld = all(
                not any(
                    isinstance(item, dict)
                    and item.get("entityId") == decision_id
                    for item in compiled_after.get("items", [])
                )
                for decision_id in (decision_id_a, decision_id_b)
            )
            exclusions = [
                item
                for item in compiled_after.get("exclusions", [])
                if isinstance(item, dict)
            ]
            decision_exclusions_ok = all(
                any(
                    item.get("entityId") == decision_id
                    and item.get("reason") == "conflicting-memory"
                    for item in exclusions
                )
                for decision_id in (decision_id_a, decision_id_b)
            )
            conflicts = [
                item
                for item in compiled_after.get("conflicts", [])
                if isinstance(item, dict)
            ]
            conflict_preserved = any(
                decision_id_a in item.get("entityIds", [])
                and decision_id_b in item.get("entityIds", [])
                for item in conflicts
            )
            adjudication_after = compiled_after.get("premiseAdjudication", {})
            compiler_reconciliation_ok = (
                compiled_learning_ok
                and decisions_withheld
                and decision_exclusions_ok
                and conflict_preserved
                and isinstance(adjudication_after, dict)
                and adjudication_after.get("state") == "conflicting-state"
                and compiled_after.get("evidenceState") == "conflicting-evidence"
                and int(compiled_after.get("estimatedTokens", 0))
                <= int(compiled_after.get("maxTokens", 0))
            )
            reconciled = sessions_unchanged and trusted_learning_ok and compiler_reconciliation_ok
            scores["parallel_session_reconciliation"] = reconciled
            scores["origin_lineage"] = origin_lineage_ok
            record_downstream_task_contract(
                scores,
                failures,
                trusted_learning_ok and compiler_reconciliation_ok,
                "reviewed multi-session lineage did not produce trusted bounded downstream context",
            )
            evidence_text.extend(
                [
                    proposed,
                    reviewed,
                    learning,
                    trusted_list,
                    search_after,
                    compiled_after,
                    context_a_after,
                    context_b_after,
                ]
            )
            if not reconciled:
                failures.append(
                    "parallel-agent reconciliation incomplete: "
                    f"sessionsUnchanged={sessions_unchanged}, "
                    f"trustedLearning={trusted_learning_ok}, "
                    f"compilerReconciliation={compiler_reconciliation_ok}"
                )

    cross_surface_expectation = scenario.get("expected_cross_surface_staleness")
    if isinstance(cross_surface_expectation, dict):
        host_session_id = str(
            cross_surface_expectation.get(
                "host_session_id",
                "cross-surface-staleness-host",
            )
        )
        prompt_marker = str(
            cross_surface_expectation.get(
                "prompt_marker",
                "cross_surface_host_write_marker",
            )
        )
        final_name = str(
            cross_surface_expectation.get(
                "final_name",
                "Reloaded local session name",
            )
        )
        if not host_session_id or not prompt_marker or not final_name:
            raise RuntimeError("cross-surface staleness fixture is incomplete")

        startup = hook_call(
            project,
            "codex",
            {
                "hook_event_name": "SessionStart",
                "session_id": host_session_id,
            },
        )
        host_ley_session_id = hook_ley_session_id(startup)
        if not host_ley_session_id:
            raise RuntimeError(
                "cross-surface staleness fixture did not resolve the host Ley session"
            )
        observed = cli_json(
            [
                "session",
                "show",
                host_ley_session_id,
                str(project),
                "--json",
            ]
        )
        observed_count = int(observed.get("eventCount", 0))
        observed_name = str(observed.get("name", ""))

        host_prompt = hook_call(
            project,
            "codex",
            {
                "hook_event_name": "UserPromptSubmit",
                "session_id": host_session_id,
                "turn_id": "cross-surface-staleness-turn",
                "prompt": (
                    f"Record this concurrent host observation: {prompt_marker}"
                ),
            },
        )
        after_host = cli_json(
            [
                "session",
                "show",
                host_ley_session_id,
                str(project),
                "--json",
            ]
        )
        after_host_count = int(after_host.get("eventCount", 0))

        stale_error = ""
        try:
            run(
                [
                    "session",
                    "rename",
                    host_ley_session_id,
                    str(project),
                    "--name",
                    "Stale local rename must fail",
                    "--note",
                    "This local view predates a host memory write.",
                    "--expected-events",
                    str(observed_count),
                    "--json",
                ]
            )
        except RuntimeError as error:
            stale_error = str(error)

        after_stale = cli_json(
            [
                "session",
                "show",
                host_ley_session_id,
                str(project),
                "--json",
            ]
        )
        reloaded_count = int(after_stale.get("eventCount", 0))
        cli_json(
            [
                "session",
                "rename",
                host_ley_session_id,
                str(project),
                "--name",
                final_name,
                "--note",
                "Reloaded after the concurrent host write.",
                "--expected-events",
                str(reloaded_count),
                "--json",
            ]
        )
        final_session = cli_json(
            [
                "session",
                "show",
                host_ley_session_id,
                str(project),
                "--json",
            ]
        )
        turns = mcp_call(
            project,
            "ley_session_turns_get",
            {
                "sessionId": host_ley_session_id,
                "maxResults": 20,
                "maxCharacters": 8_000,
            },
        )
        turns_text = json.dumps(turns, sort_keys=True)
        cross_surface_checks = {
            "startup-session": host_ley_session_id.startswith("ses_"),
            "initial-event-count": observed_count >= 1,
            "host-write-advanced":
                after_host_count == observed_count + 1
                and prompt_marker in turns_text,
            "stale-local-write-rejected":
                (
                    f"session changed from {observed_count} events to "
                    f"{after_host_count}; reload before saving"
                )
                in stale_error,
            "stale-write-nonmutating":
                int(after_stale.get("eventCount", 0)) == after_host_count
                and after_stale.get("name") == observed_name,
            "reloaded-write-succeeds":
                final_session.get("name") == final_name
                and int(final_session.get("eventCount", 0))
                == after_host_count + 1,
            "snapshot-only":
                turns.get("liveSourceChecked") is False,
        }
        cross_surface_ok = all(cross_surface_checks.values())
        scores["cross_surface_staleness"] = cross_surface_ok
        cross_surface_outputs = [
            startup,
            host_prompt,
            observed,
            after_host,
            after_stale,
            final_session,
            turns,
        ]
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)],
            cross_surface_outputs,
        )
        evidence_text.extend(cross_surface_outputs)
        if not cross_surface_ok:
            failed_cross_surface_checks = [
                label
                for label, passed in cross_surface_checks.items()
                if not passed
            ]
            failures.append(
                "concurrent host/local session mutation did not preserve stale-write protection: "
                + ", ".join(failed_cross_surface_checks)
            )

    long_horizon_expectation = scenario.get(
        "expected_long_horizon_continuity"
    )
    if isinstance(long_horizon_expectation, dict):
        iterations = [
            item
            for item in long_horizon_expectation.get("iterations", [])
            if isinstance(item, dict)
        ]
        query = str(long_horizon_expectation.get("query", ""))
        activity_query = str(
            long_horizon_expectation.get(
                "activity_query",
                "Startup requirement",
            )
        )
        current_requirement_marker = str(
            long_horizon_expectation.get(
                "current_requirement_marker",
                "",
            )
        )
        legacy_markers = [
            str(value)
            for value in long_horizon_expectation.get(
                "legacy_markers",
                [],
            )
        ]
        final_handoff_marker = str(
            long_horizon_expectation.get(
                "final_handoff_marker",
                "",
            )
        )
        final_unresolved_marker = str(
            long_horizon_expectation.get(
                "final_unresolved_marker",
                "",
            )
        )
        live_path = str(long_horizon_expectation.get("live_path", ""))
        live_mutation_marker = str(
            long_horizon_expectation.get("live_mutation_marker", "")
        )
        if (
            len(iterations) != 10
            or not query
            or not activity_query
            or not current_requirement_marker
            or not legacy_markers
            or not final_handoff_marker
            or not final_unresolved_marker
            or not live_path
            or not live_mutation_marker
        ):
            raise RuntimeError(
                "long-horizon continuity fixture requires exactly ten iterations, current/legacy markers, query, final handoff/unresolved markers, and live-source path/marker"
            )

        session_ids: list[str] = []
        decision_record_ids_by_marker: dict[str, str] = {}
        session_contexts: list[dict[str, object]] = []
        for index, iteration in enumerate(iterations, start=1):
            name = str(iteration.get("name", f"Iteration {index:02d}"))
            goal = str(
                iteration.get(
                    "goal",
                    "Continue the changing startup requirement implementation.",
                )
            )
            summary = str(
                iteration.get(
                    "summary",
                    f"Iteration {index:02d} checkpoint.",
                )
            )
            decision_title = str(
                iteration.get(
                    "decision_title",
                    f"Startup requirement iteration {index:02d}",
                )
            )
            decision = str(iteration.get("decision", ""))
            handoff = str(iteration.get("handoff", ""))
            unresolved = [
                str(value)
                for value in iteration.get("unresolved", [])
            ]
            result_summary = str(
                iteration.get(
                    "result_summary",
                    f"Iteration {index:02d} completed.",
                )
            )
            if not decision or not handoff:
                raise RuntimeError(
                    f"long-horizon iteration {index} requires decision and handoff"
                )

            long_session_id, _ = create_structured_session(
                project,
                seed=f"{scenario['id']}:long-horizon:{index}",
                name=name,
                goal=goal,
                summary=summary,
                decisions=[
                    {
                        "title": decision_title,
                        "decision": decision,
                        "rationale": (
                            "Historical requirement state for this exact iteration; "
                            "the current approved Specification remains authoritative."
                        ),
                    }
                ],
                unresolved=unresolved,
                host="codex" if index % 2 else "claude-code",
            )
            cli_session_finish(
                project,
                long_session_id,
                request_id_value=request_id(
                    f"{scenario['id']}:long-horizon:{index}:finish"
                ),
                status="completed",
                summary=result_summary,
                handoff=handoff,
                unresolved=unresolved,
            )
            context = cli_session_show(project, long_session_id)
            session_ids.append(long_session_id)
            session_contexts.append(context)
            for checkpoint in context.get("checkpoints", []):
                if not isinstance(checkpoint, dict):
                    continue
                for row in checkpoint.get("decisions", []):
                    if not isinstance(row, dict):
                        continue
                    decision_text = str(row.get("decision", ""))
                    for marker in legacy_markers:
                        if marker in decision_text:
                            decision_record_ids_by_marker[marker] = str(
                                row.get("id", "")
                            )
            time.sleep(0.003)

        session_list = cli_session_list_payload(project)
        resume = cli_resume_payload(
            project,
            max_sessions=3,
            max_learnings=1,
            max_characters=12_000,
        )
        activity_search = mcp_call(
            project,
            "ley_search",
            {
                "query": activity_query,
                "maxResults": 20,
                "maxTokens": 8_000,
            },
        )
        compiled = mcp_call(
            project,
            "ley_brief",
            {
                "task": query,
                "maxResults": 12,
                "maxTokens": 3_000,
            },
        )

        listed_sessions = [
            item
            for item in session_list.get("sessions", [])
            if isinstance(item, dict)
        ]
        resumed_sessions = [
            item
            for item in resume.get("sessions", [])
            if isinstance(item, dict)
        ]
        decisions = [
            item
            for item in activity_search.get("results", [])
            if isinstance(item, dict)
            and item.get("kind") == "decision"
        ]
        activity_text = json.dumps(activity_search, sort_keys=True)
        compiled_context_text = context_contract_text(compiled)
        compiled_exclusions = [
            item
            for item in compiled.get("exclusions", [])
            if isinstance(item, dict)
        ]
        legacy_record_ids = {
            record_id
            for record_id in decision_record_ids_by_marker.values()
            if record_id
        }
        contradicted_legacy_ids = {
            str(item.get("entityId", ""))
            for item in compiled_exclusions
            if item.get("reason") == "contradicts-human-intent"
        }
        contradictory_legacy_marker = str(
            long_horizon_expectation.get(
                "contradictory_legacy_marker",
                legacy_markers[0],
            )
        )
        contradictory_legacy_id = decision_record_ids_by_marker.get(
            contradictory_legacy_marker,
            "",
        )
        historical_items = [
            item
            for item in compiled.get("items", [])
            if isinstance(item, dict)
            and item.get("kind") in {"session", "decision"}
        ]
        expected_latest_names = [
            str(iterations[index].get("name", ""))
            for index in (9, 8, 7)
        ]
        actual_latest_names = [
            str(item.get("name", ""))
            for item in resumed_sessions
        ]
        resumed_results = [
            item.get("result", {})
            for item in resumed_sessions
            if isinstance(item.get("result"), dict)
        ]
        all_handoffs = json.dumps(resumed_results, sort_keys=True)

        long_horizon_checks = {
            "ten-distinct-sessions":
                len(session_ids) == 10
                and len(set(session_ids)) == 10,
            "session-list-complete":
                session_list.get("totalSessions") == 10
                and session_list.get("omittedSessions") == 0
                and len(listed_sessions) == 10
                and all(
                    item.get("status") == "completed"
                    and item.get("checkpoints") == 1
                    and int(item.get("eventCount", 0)) == 3
                    for item in listed_sessions
                ),
            "resume-bounded":
                resume.get("totalSessions") == 10
                and resume.get("omittedSessions") == 7
                and len(resumed_sessions) == 3
                and actual_latest_names == expected_latest_names
                and all(
                    item.get("status") == "completed"
                    and isinstance(item.get("result"), dict)
                    and item["result"].get("status") == "completed"
                    for item in resumed_sessions
                ),
            "latest-handoff-visible":
                final_handoff_marker in all_handoffs,
            "latest-unresolved-visible":
                final_unresolved_marker in json.dumps(resume, sort_keys=True),
            "history-inspectable":
                len(decisions) == 10
                and all(marker in activity_text for marker in legacy_markers),
            "legacy-records-resolved":
                len(decision_record_ids_by_marker) == len(legacy_markers)
                and len(legacy_record_ids) == len(legacy_markers),
            "current-specification-present":
                current_requirement_marker.lower()
                in compiled_context_text.lower()
                and compiled.get("authorityPrecedence")
                == "human-intent-over-historical-memory",
            "contradictory-history-withheld":
                bool(contradictory_legacy_id)
                and contradictory_legacy_marker.lower()
                not in compiled_context_text.lower(),
            "historical-conflict-disclosed":
                bool(contradictory_legacy_id)
                and contradictory_legacy_id in contradicted_legacy_ids,
            "remaining-history-lower-authority":
                all(
                    item.get("authority") == "historical-project-memory"
                    and item.get("trustedForReuse") is False
                    for item in historical_items
                ),
            "snapshot-only":
                session_list.get("liveSourceChecked", False) is False
                and resume.get("liveSourceChecked") is False
                and activity_search.get("liveSourceChecked") is False
                and compiled.get("liveSourceChecked") is False,
        }
        base_long_horizon_ok = all(long_horizon_checks.values())

        live_relative = Path(live_path)
        if live_relative.is_absolute() or ".." in live_relative.parts:
            raise RuntimeError("long-horizon live_path must be a safe project-relative path")
        live_target = project / live_relative
        if not live_target.is_file():
            raise RuntimeError("long-horizon live_path does not exist in the project")
        live_target.write_text(
            f"{live_mutation_marker}\nCurrent runtime source changed after the captured ten-session history.\n",
            encoding="utf-8",
        )
        live_hash = "sha256:" + hashlib.sha256(live_target.read_bytes()).hexdigest()

        continuation_external_id = f"{scenario['id']}-fresh-continuation-host"
        continuation_startup = hook_call(
            project,
            "codex",
            {
                "hook_event_name": "SessionStart",
                "session_id": continuation_external_id,
            },
        )
        continuation_session_id = hook_ley_session_id(continuation_startup)
        if not continuation_session_id:
            raise RuntimeError(
                "long-horizon continuation fixture did not resolve the fresh host Ley session"
            )
        continuation_startup_context = hook_additional_context(continuation_startup)
        continuation_prompt = hook_call(
            project,
            "codex",
            {
                "hook_event_name": "UserPromptSubmit",
                "session_id": continuation_external_id,
                "turn_id": "long-horizon-live-continuation-turn",
                "prompt": (
                    "Continue the approved offline startup requirement and inspect the current "
                    f"{live_path} before any consequential edit."
                ),
            },
        )
        continuation_task_context = hook_additional_context(continuation_prompt)
        continuation_brief = mcp_call(
            project,
            "ley_brief",
            {
                "task": (
                    "Continue the approved offline startup requirement and inspect the current "
                    f"{live_path} before any consequential edit."
                ),
                "maxResults": 12,
                "maxTokens": 3_000,
            },
        )

        live_read_command = f"cat -- {live_path}"
        live_read = subprocess.run(
            ["cat", "--", live_path],
            cwd=project,
            capture_output=True,
            text=True,
            check=False,
        )
        if live_read.returncode != 0:
            raise RuntimeError(
                "long-horizon continuation could not read the current workspace file: "
                + live_read.stderr.strip()
            )
        live_read_output = live_read.stdout
        hook_call(
            project,
            "codex",
            {
                "hook_event_name": "PostToolUse",
                "session_id": continuation_external_id,
                "turn_id": "long-horizon-live-continuation-turn",
                "tool_name": "Bash",
                "tool_use_id": "long-horizon-live-source-read",
                "tool_input": {"command": live_read_command},
                "tool_response": {
                    "output": live_read_output,
                    "metadata": {"exit_code": live_read.returncode},
                },
            },
        )
        continuation_turns = cli_session_turns(
            project,
            continuation_session_id,
            max_results=20,
            max_characters=16_000,
        )
        continuation_session = cli_session_show(project, continuation_session_id)
        matching_live_rows = [
            item
            for item in continuation_turns.get("toolObservations", [])
            if isinstance(item, dict)
            and item.get("toolName") == "Bash"
            and item.get("command") == live_read_command
        ]
        live_tool_row = matching_live_rows[0] if len(matching_live_rows) == 1 else None
        expected_live_result = (
            f"metadata.exit_code: {live_read.returncode}\noutput: {live_read_output}"
        ).strip()
        startup_lower = continuation_startup_context.lower()
        task_lower = continuation_task_context.lower()
        brief_text = context_contract_text(continuation_brief)
        brief_gaps_text = json.dumps(
            continuation_brief.get("gaps", []),
            sort_keys=True,
        ).lower()
        continuation_turns_text = json.dumps(continuation_turns, sort_keys=True)
        continuation_live_checks = {
            "host-session": continuation_session_id.startswith("ses_"),
            "startup-guidance-only":
                "historical ley project memory was not auto-injected" in startup_lower
                and "call ley_brief" in startup_lower
                and "## recent work" not in startup_lower
                and "## reviewed project learnings" not in startup_lower,
            "startup-no-handoff": final_handoff_marker not in continuation_startup_context,
            "startup-no-unresolved": final_unresolved_marker not in continuation_startup_context,
            "startup-no-live-marker": live_mutation_marker not in continuation_startup_context,
            "capture-only-guidance":
                "does not inject task-specific project history" in task_lower
                and "call ley_brief" in task_lower
                and "# ley task context (automatic)" not in task_lower,
            "capture-no-live-marker":
                live_mutation_marker not in continuation_task_context,
            "brief-current-spec":
                current_requirement_marker.lower() in brief_text.lower(),
            "brief-non-live": continuation_brief.get("liveSourceChecked") is False,
            "brief-live-gap": "live-source-unchecked" in brief_gaps_text,
            "brief-live-instruction":
                "inspect live source before consequential" in brief_gaps_text,
            "brief-no-live-marker":
                live_mutation_marker not in json.dumps(continuation_brief, sort_keys=True),
            "live-read-success": live_read.returncode == 0,
            "live-read-current-marker": live_mutation_marker in live_read_output,
            "live-read-current-hash":
                "sha256:"
                + hashlib.sha256(live_read_output.encode("utf-8")).hexdigest()
                == live_hash,
            "one-live-tool-row": len(matching_live_rows) == 1,
            "tool-row-returned":
                isinstance(live_tool_row, dict)
                and live_tool_row.get("observationKind") == "returned",
            "tool-row-exact-result":
                isinstance(live_tool_row, dict)
                and live_tool_row.get("result") == expected_live_result,
            "tool-row-untruncated":
                isinstance(live_tool_row, dict)
                and live_tool_row.get("commandTruncatedAtCapture") is False
                and live_tool_row.get("resultTruncatedAtCapture") is False
                and live_tool_row.get("commandTruncatedForContext") is False
                and live_tool_row.get("resultTruncatedForContext") is False,
            "tool-history-non-live": continuation_turns.get("liveSourceChecked") is False,
            "no-checkpoint-authority":
                continuation_session.get("checkpointCount") == 0
                and not continuation_session.get("checkpoints"),
            "session-non-live": continuation_session.get("liveSourceChecked") is False,
            "startup-path-private":
                str(project) not in continuation_startup_context
                and str(vault) not in continuation_startup_context,
            "task-path-private":
                str(project) not in continuation_task_context
                and str(vault) not in continuation_task_context,
            "tool-history-path-private":
                str(project) not in continuation_turns_text
                and str(vault) not in continuation_turns_text,
        }
        continuation_live_ok = all(continuation_live_checks.values())
        long_horizon_ok = base_long_horizon_ok and continuation_live_ok
        scores["long_horizon_continuity"] = long_horizon_ok
        scores["weeks_later_continuation"] = long_horizon_ok
        record_downstream_task_contract(
            scores,
            failures,
            task_contract_success(
                compiled,
                [current_requirement_marker],
                [contradictory_legacy_marker],
            )
            and final_handoff_marker in all_handoffs,
            "long-horizon continuity did not preserve current approved intent plus the bounded latest handoff without admitting obsolete requirement markers",
        )
        long_horizon_outputs = [
            session_list,
            resume,
            activity_search,
            compiled,
            *session_contexts,
            continuation_startup,
            continuation_prompt,
            continuation_brief,
            continuation_turns,
            continuation_session,
        ]
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)],
            long_horizon_outputs,
        )
        evidence_text.extend(long_horizon_outputs)
        if not long_horizon_ok:
            failed_long_horizon_checks = [
                label
                for label, passed in long_horizon_checks.items()
                if not passed
            ]
            if not continuation_live_ok:
                failed_long_horizon_checks.extend(
                    "fresh-host-" + label
                    for label, passed in continuation_live_checks.items()
                    if not passed
                )
            failures.append(
                "ten-session changing-requirement continuity failed: "
                + ", ".join(failed_long_horizon_checks)
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
        proposed = cli_json(
            [
                "learning",
                "propose",
                str(project),
                "--request-id",
                request_id(f"{scenario['id']}:erase:learning"),
                "--actor",
                "agent",
                "--provenance",
                "agent-authored",
                "--kind",
                "fact",
                "--title",
                "Private deletion learning",
                "--guidance",
                f"Dependent learning carrying {marker}.",
                "--confidence",
                "50",
                "--evidence",
                f"{erased_session}:{checkpoint_id}",
                "--json",
            ]
        )
        proposed_learning = proposed.get("learning", {}) if isinstance(proposed, dict) else {}
        learning_id = str(proposed_learning.get("learningId", ""))
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
                "ley_search",
                {"query": "private deletion memory", "maxResults": 20, "maxTokens": 1_000},
            ),
            mcp_call(
                project,
                "ley_brief",
                {"task": "private deletion memory", "maxResults": 20, "maxTokens": 1_000},
            ),
            cli_json(["learning", "list", str(project), "--json"]),
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
        portable_session_id, _ = create_structured_session(
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
        codex_prompt_marker = "codex_prompt_only_private_marker_7f31"
        claude_prompt_marker = "claude_prompt_only_private_marker_4aa2"
        codex_prompt = {
            "hook_event_name": "UserPromptSubmit",
            "session_id": "portable-codex-thread",
            "turn_id": "portable-turn-1",
            "prompt": "Preserve durable knowledge across hosts " + codex_prompt_marker,
        }
        claude_prompt = {
            "hook_event_name": "UserPromptSubmit",
            "session_id": "portable-claude-thread",
            "prompt": "Preserve durable knowledge across hosts " + claude_prompt_marker,
        }
        codex_session_id = hook_ley_session_id(codex)
        claude_session_id = hook_ley_session_id(claude)
        codex_task = hook_call(project, "codex", codex_prompt)
        codex_after_first = mcp_call(
            project,
            "ley_session_get",
            {"sessionId": codex_session_id, "maxCheckpoints": 5, "maxCharacters": 8_000},
        )
        codex_retry = hook_call(project, "codex", codex_prompt)
        codex_after_retry = mcp_call(
            project,
            "ley_session_get",
            {"sessionId": codex_session_id, "maxCheckpoints": 5, "maxCharacters": 8_000},
        )
        claude_task = hook_call(project, "claude", claude_prompt)
        claude_after_first = mcp_call(
            project,
            "ley_session_get",
            {"sessionId": claude_session_id, "maxCheckpoints": 5, "maxCharacters": 8_000},
        )
        claude_retry = hook_call(project, "claude", claude_prompt)
        claude_after_retry = mcp_call(
            project,
            "ley_session_get",
            {"sessionId": claude_session_id, "maxCheckpoints": 5, "maxCharacters": 8_000},
        )
        codex_text = json.dumps(codex, sort_keys=True)
        claude_text = json.dumps(claude, sort_keys=True)
        codex_startup_context = hook_additional_context(codex)
        claude_startup_context = hook_additional_context(claude)
        codex_task_context = hook_additional_context(codex_task)
        claude_task_context = hook_additional_context(claude_task)
        codex_retry_context = hook_additional_context(codex_retry)
        claude_retry_context = hook_additional_context(claude_retry)
        codex_brief = mcp_call(
            project,
            "ley_brief",
            {"task": "Preserve durable knowledge across hosts", "maxResults": 8, "maxTokens": 1_500},
        )
        claude_brief = mcp_call(
            project,
            "ley_brief",
            {"task": "Preserve durable knowledge across hosts", "maxResults": 8, "maxTokens": 1_500},
        )
        codex_brief_text = context_contract_text(codex_brief)
        claude_brief_text = context_contract_text(claude_brief)
        portable = (
            codex_session_id.startswith("ses_")
            and claude_session_id.startswith("ses_")
            and marker not in codex_text
            and marker not in claude_text
            and "historical ley project memory was not auto-injected"
            in codex_startup_context.lower()
            and "historical ley project memory was not auto-injected"
            in claude_startup_context.lower()
            and "call ley_brief" in codex_startup_context.lower()
            and "call ley_brief" in claude_startup_context.lower()
            and "## recent work" not in codex_startup_context.lower()
            and "## recent work" not in claude_startup_context.lower()
            and "does not inject task-specific project history" in codex_task_context
            and "does not inject task-specific project history" in claude_task_context
            and "call ley_brief" in codex_task_context.lower()
            and "call ley_brief" in claude_task_context.lower()
            and "# Ley task context (automatic)" not in codex_task_context
            and "# Ley task context (automatic)" not in claude_task_context
            and portable_session_id in codex_brief_text
            and portable_session_id in claude_brief_text
            and "Portable prior work" in codex_brief_text
            and "Portable prior work" in claude_brief_text
            and codex_prompt_marker not in codex_task_context
            and claude_prompt_marker not in claude_task_context
            and codex_prompt_marker not in codex_brief_text
            and claude_prompt_marker not in claude_brief_text
            and codex_retry_context == codex_task_context
            and claude_retry_context == claude_task_context
            and codex_after_first.get("eventCount") == codex_after_retry.get("eventCount")
            and claude_after_first.get("eventCount") == claude_after_retry.get("eventCount")
            and codex_after_retry.get("promptCount") == 1
            and claude_after_retry.get("promptCount") == 1
            and codex_after_retry.get("contextUtilityBindingCount") == 0
            and claude_after_retry.get("contextUtilityBindingCount") == 0
        )
        scores["host_portability"] = portable
        evidence_text.extend([codex, claude, codex_task, claude_task, codex_retry, claude_retry, codex_brief, claude_brief])
        if not portable:
            failures.append(
                "durable Ley context or explicit Brief retrieval was not usable from both Codex and Claude lifecycle hosts"
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
            "ley_brief",
            {"task": query, "maxResults": 8, "maxTokens": max_tokens},
        )
        resume = cli_resume_payload(
            project,
            max_sessions=3,
            max_learnings=1,
            max_characters=resume_characters,
        )
        session_list = cli_session_list_payload(project)
        listed_sessions = [
            item
            for item in session_list.get("sessions", [])
            if isinstance(item, dict) and isinstance(item.get("sessionId"), str)
        ]
        raw_session_contexts = [
            cli_session_show(project, str(item["sessionId"]))
            for item in listed_sessions
        ]
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
        raw_history_text = json.dumps(raw_session_contexts, sort_keys=True).lower()
        compiler_contract_text = context_contract_text(compiler).lower()
        raw_required_count = sum(
            marker.lower() in raw_history_text for marker in required
        )
        compiler_required_count = sum(
            marker.lower() in compiler_contract_text for marker in required
        )
        raw_forbidden_count = sum(
            marker.lower() in raw_history_text for marker in forbidden
        )
        compiler_forbidden_count = sum(
            marker.lower() in compiler_contract_text for marker in forbidden
        )
        raw_history_estimated_tokens = max(
            1,
            (len(raw_history_text) + 3) // 4,
        )
        compiler_estimated_tokens = int(compiler.get("estimatedTokens", 0))
        expected_session_count = 1 + len(distractors)
        full_history_complete = (
            session_list.get("totalSessions") == expected_session_count
            and session_list.get("omittedSessions") == 0
            and len(listed_sessions) == expected_session_count
            and len(raw_session_contexts) == expected_session_count
            and all(
                context.get("truncated") is False
                for context in raw_session_contexts
            )
        )
        full_history_efficiency = (
            full_history_complete
            and raw_required_count == len(required)
            and compiler_required_count >= raw_required_count
            and raw_forbidden_count == len(forbidden)
            and compiler_forbidden_count <= raw_forbidden_count
            and compiler_forbidden_count == 0
            and compiler_estimated_tokens < raw_history_estimated_tokens
        )
        scores["downstream_task_contract"] = compiler_success
        scores["budget_baseline_advantage"] = (
            compiler_success and not baseline_success and full_history_efficiency
        )
        scores["budget_full_history_efficiency"] = full_history_efficiency
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)],
            [compiler, resume, session_list, *raw_session_contexts],
        )
        evidence_text.extend([compiler, resume, session_list, *raw_session_contexts])
        if not compiler_success:
            failures.append(
                "500-token Context Compiler failed the deterministic downstream evidence contract"
            )
        if baseline_success:
            failures.append(
                "bounded recent-resume baseline unexpectedly satisfied the older task-specific evidence contract"
            )
        if not full_history_efficiency:
            failures.append(
                "500-token Context Compiler did not preserve equal-or-better required/distractor evidence selection at a smaller approximate text budget than all retained session history: "
                f"fullHistoryComplete={full_history_complete}, "
                f"required={compiler_required_count}/{raw_required_count}, "
                f"distractors={compiler_forbidden_count}/{raw_forbidden_count}, "
                f"tokens={compiler_estimated_tokens}/{raw_history_estimated_tokens}"
            )

    retrieval_expectation = scenario.get("expected_retrieval_robustness")
    if isinstance(retrieval_expectation, dict):
        query = str(retrieval_expectation.get("query", ""))
        required_marker = str(
            retrieval_expectation.get("required_marker", "")
        )
        budgets = [
            int(value)
            for value in retrieval_expectation.get(
                "budgets",
                [500, 1_500, 3_000, 8_000],
            )
        ]
        max_results = int(retrieval_expectation.get("max_results", 20))
        if (
            not query
            or not required_marker
            or budgets != sorted(set(budgets))
            or not budgets
            or budgets[0] < 500
        ):
            raise RuntimeError("retrieval robustness fixture is incomplete")

        result_counts: list[int] = []
        search_outputs: list[dict[str, object]] = []
        compiler_outputs: list[dict[str, object]] = []
        per_budget_checks: list[bool] = []
        private_cache = str(EVAL_ENV.get("XDG_CACHE_HOME", ""))

        for budget in budgets:
            search_payload = mcp_call(
                project,
                "ley_search",
                {
                    "query": query,
                    "maxResults": max_results,
                    "maxTokens": budget,
                },
            )
            compiler_payload = mcp_call(
                project,
                "ley_brief",
                {
                    "task": query,
                    "maxResults": max_results,
                    "maxTokens": budget,
                },
            )
            search_outputs.append(search_payload)
            compiler_outputs.append(compiler_payload)
            result_counts.append(
                len(
                    [
                        item
                        for item in search_payload.get("results", [])
                        if isinstance(item, dict)
                    ]
                )
            )

            search_retrieval = search_payload.get("retrieval", {})
            compiler_retrieval = compiler_payload.get("retrieval", {})
            compiler_semantic_gap = any(
                isinstance(item, dict)
                and item.get("kind") == "semantic-fallback"
                for item in compiler_payload.get("gaps", [])
            )
            search_text = json.dumps(
                search_payload.get("results", []),
                sort_keys=True,
            )
            serialized_outputs = json.dumps(
                [search_payload, compiler_payload],
                sort_keys=True,
            )
            per_budget_checks.append(
                search_retrieval.get("mode") == "lexical"
                and search_retrieval.get("boundedRerankMode") == "lexical"
                and search_retrieval.get("artifactContextMode") == "lexical"
                and compiler_retrieval.get("mode") == "lexical"
                and compiler_retrieval.get("boundedRerankMode") == "lexical"
                and compiler_retrieval.get("artifactContextMode") == "lexical"
                and "boundedRerankFallbackReason" not in search_retrieval
                and "artifactContextFallbackReason" not in search_retrieval
                and "boundedRerankFallbackReason" not in compiler_retrieval
                and "artifactContextFallbackReason" not in compiler_retrieval
                and not compiler_semantic_gap
                and required_marker in search_text
                and task_contract_success(
                    compiler_payload,
                    [required_marker],
                    [],
                )
                and int(search_payload.get("estimatedTokens", 0)) <= budget
                and int(compiler_payload.get("estimatedTokens", 0)) <= budget
                and (
                    not private_cache
                    or private_cache not in serialized_outputs
                )
                and str(project) not in serialized_outputs
                and str(vault) not in serialized_outputs
            )

        ladder_non_decreasing = all(
            left <= right
            for left, right in zip(result_counts, result_counts[1:])
        )
        ladder_expands = result_counts[-1] > result_counts[0]
        robustness_ok = (
            all(per_budget_checks)
            and ladder_non_decreasing
            and ladder_expands
        )
        scores["retrieval_robustness"] = robustness_ok
        scores["token_budget"] = (
            all(per_budget_checks)
            and ladder_non_decreasing
        )
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)]
            + ([private_cache] if private_cache else []),
            [*search_outputs, *compiler_outputs],
        )
        evidence_text.extend(search_outputs)
        evidence_text.extend(compiler_outputs)
        if not robustness_ok:
            failures.append(
                "retrieval robustness did not preserve lexical fallback, useful sparse evidence, or the multi-budget ladder: "
                f"checks={per_budget_checks}, resultCounts={result_counts}, "
                f"searchRetrieval={[payload.get('retrieval', {}) for payload in search_outputs]}, "
                f"compilerRetrieval={[payload.get('retrieval', {}) for payload in compiler_outputs]}"
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

    learning_idempotency_expectation = scenario.get(
        "expected_learning_idempotency"
    )
    if isinstance(learning_idempotency_expectation, dict):
        if not session_id:
            raise RuntimeError("learning idempotency fixture created no session")
        session_snapshot = cli_json(
            ["session", "show", session_id, str(project), "--json"]
        )
        checkpoint_rows = (
            session_snapshot.get("checkpoints", [])
            if isinstance(session_snapshot, dict)
            else []
        )
        evidence_record_id = next(
            (
                str(checkpoint.get("checkpointId"))
                for checkpoint in reversed(checkpoint_rows)
                if isinstance(checkpoint, dict)
                and isinstance(checkpoint.get("checkpointId"), str)
            ),
            "",
        )
        if not evidence_record_id:
            raise RuntimeError(
                "learning idempotency fixture created no stable checkpoint evidence record"
            )

        title = str(
            learning_idempotency_expectation.get(
                "title",
                "Idempotent reviewed learning",
            )
        )
        guidance = str(
            learning_idempotency_expectation.get(
                "guidance",
                "Reuse exact retries without duplicating durable learning events.",
            )
        )
        proposal_request_id = request_id(
            f"{scenario['id']}:learning-idempotency:proposal"
        )
        proposal_args = {
            "requestId": proposal_request_id,
            "kind": "fact",
            "title": title,
            "guidance": guidance,
            "confidencePercent": 80,
            "provenance": "agent-authored",
            "evidence": [
                {
                    "sessionId": session_id,
                    "recordId": evidence_record_id,
                    "note": "Stable evidence for learning idempotency evaluation.",
                }
            ],
        }
        proposed = mcp_call(
            project,
            "ley_learning_propose",
            proposal_args,
            WRITE_FLAGS,
        )
        proposal_retry = mcp_call(
            project,
            "ley_learning_propose",
            proposal_args,
            WRITE_FLAGS,
        )
        learning_id = str(proposed.get("learningId", ""))

        proposal_conflict_error = ""
        conflicting_proposal_args = dict(proposal_args)
        conflicting_proposal_args["guidance"] = (
            guidance + " conflicting retry payload"
        )
        try:
            mcp_call(
                project,
                "ley_learning_propose",
                conflicting_proposal_args,
                WRITE_FLAGS,
            )
        except RuntimeError as error:
            proposal_conflict_error = str(error)

        review_request_id = request_id(
            f"{scenario['id']}:learning-idempotency:review"
        )
        review_args = [
            "learning",
            "review",
            learning_id,
            str(project),
            "--actor",
            "user",
            "--action",
            "confirm",
            "--note",
            "Exact reviewed learning idempotency evaluation.",
            "--request-id",
            review_request_id,
            "--json",
        ]
        reviewed = cli_json(review_args)
        review_retry = cli_json(review_args)

        review_conflict_error = ""
        conflicting_review_args = list(review_args)
        note_index = conflicting_review_args.index("--note") + 1
        conflicting_review_args[note_index] = (
            "Conflicting retry payload for the same review request."
        )
        try:
            run(conflicting_review_args)
        except RuntimeError as error:
            review_conflict_error = str(error)

        final_learning = mcp_call(
            project,
            "ley_learning_get",
            {
                "learningId": learning_id,
                "maxEvidence": 10,
                "maxHistory": 10,
                "maxArtifactsPerEvidence": 10,
                "maxCharacters": 8_000,
            },
        )
        reviewed_learning = (
            reviewed.get("learning", {})
            if isinstance(reviewed, dict)
            else {}
        )
        retried_learning = (
            review_retry.get("learning", {})
            if isinstance(review_retry, dict)
            else {}
        )
        proposal_conflict_rejected = any(
            marker in proposal_conflict_error
            for marker in (
                "request ID was already used with different learning content",
                "learning request ID was reused with different content",
            )
        )
        review_conflict_rejected = any(
            marker in review_conflict_error
            for marker in (
                "request ID was already used with different learning content",
                "learning request ID was reused with different content",
            )
        )
        learning_idempotency_checks = {
            "learning-id": learning_id.startswith("lrn_"),
            "proposal-recorded":
                proposed.get("replayed") is False
                and proposed.get("eventCount") == 1,
            "proposal-replayed":
                proposal_retry.get("replayed") is True
                and proposal_retry.get("learningId") == learning_id
                and proposal_retry.get("eventId") == proposed.get("eventId")
                and proposal_retry.get("eventCount") == 1,
            "proposal-conflict-rejected": proposal_conflict_rejected,
            "review-recorded": reviewed.get("replayed") is False,
            "review-replayed":
                review_retry.get("replayed") is True
                and review_retry.get("eventId") == reviewed.get("eventId"),
            "review-learning-identity":
                reviewed_learning.get("learningId") == learning_id
                and retried_learning.get("learningId") == learning_id,
            "review-event-count":
                reviewed_learning.get("eventCount") == 2
                and retried_learning.get("eventCount") == 2,
            "review-conflict-rejected": review_conflict_rejected,
            "final-learning-identity":
                final_learning.get("learningId") == learning_id,
            "final-event-count": final_learning.get("eventCount") == 2,
            "final-state": final_learning.get("state") == "verified",
            "final-trust": final_learning.get("trustState") == "trusted",
        }
        learning_idempotency_ok = all(learning_idempotency_checks.values())
        scores["learning_idempotency"] = learning_idempotency_ok
        evidence_text.extend(
            [
                proposed,
                proposal_retry,
                final_learning,
            ]
        )
        if not learning_idempotency_ok:
            failed_learning_idempotency_checks = [
                label
                for label, passed in learning_idempotency_checks.items()
                if not passed
            ]
            proposal_conflict_detail = (
                f"; proposal conflict observed: {proposal_conflict_error}"
                if "proposal-conflict-rejected"
                in failed_learning_idempotency_checks
                else ""
            )
            failures.append(
                "learning proposal/review retries did not preserve exact replay identity, conflicting-request rejection, or two-event durable state: "
                + ", ".join(failed_learning_idempotency_checks)
                + proposal_conflict_detail
            )

    delayed_poisoning_expectation = scenario.get(
        "expected_delayed_poisoning"
    )
    if isinstance(delayed_poisoning_expectation, dict):
        source_path = str(
            delayed_poisoning_expectation.get("source_path", "")
        )
        source_marker = str(
            delayed_poisoning_expectation.get("source_marker", "")
        )
        poison_title = str(
            delayed_poisoning_expectation.get("poison_title", "")
        )
        poison_marker = str(
            delayed_poisoning_expectation.get("poison_marker", "")
        )
        poison_guidance = str(
            delayed_poisoning_expectation.get("poison_guidance", "")
        )
        query = str(delayed_poisoning_expectation.get("query", ""))
        if not all(
            [
                source_path,
                source_marker,
                poison_title,
                poison_marker,
                poison_guidance,
                query,
            ]
        ):
            raise RuntimeError("delayed poisoning fixture is incomplete")

        poisoning_session = cli_session_start(
            project,
            seed=f"{scenario['id']}:poisoning",
            name="Inspect suspicious repository text",
            goal=(
                "Inspect suspicious repository text without treating it "
                "as trusted instructions."
            ),
            host="codex",
        )
        poisoning_session_id = str(poisoning_session.get("sessionId", ""))
        mcp_call(
            project,
            "ley_checkpoint",
            {
                "sessionId": poisoning_session_id,
                "requestId": request_id(
                    f"{scenario['id']}:poisoning:checkpoint"
                ),
                "summary": (
                    "Inspected suspicious repository content as untrusted "
                    "captured evidence."
                ),
                "touchedArtifacts": [source_path],
            },
            WRITE_FLAGS,
        )
        poisoning_snapshot = cli_json(
            [
                "session",
                "show",
                poisoning_session_id,
                str(project),
                "--json",
            ]
        )
        poisoning_checkpoints = (
            poisoning_snapshot.get("checkpoints", [])
            if isinstance(poisoning_snapshot, dict)
            else []
        )
        poisoning_checkpoint_id = next(
            (
                str(item.get("checkpointId"))
                for item in reversed(poisoning_checkpoints)
                if isinstance(item, dict)
                and isinstance(item.get("checkpointId"), str)
            ),
            "",
        )
        if not poisoning_checkpoint_id:
            raise RuntimeError(
                "delayed poisoning fixture created no checkpoint evidence"
            )

        proposal_raw = cli_json(
            [
                "learning",
                "propose",
                str(project),
                "--request-id",
                request_id(f"{scenario['id']}:poisoning:learning"),
                "--actor",
                "agent",
                "--provenance",
                "agent-authored",
                "--kind",
                "procedure",
                "--title",
                poison_title,
                "--guidance",
                poison_guidance,
                "--confidence",
                "99",
                "--evidence",
                f"{poisoning_session_id}:{poisoning_checkpoint_id}",
                "--json",
            ]
        )
        proposal_learning = (
            proposal_raw.get("learning", {})
            if isinstance(proposal_raw, dict)
            else {}
        )
        proposal = dict(proposal_learning) if isinstance(proposal_learning, dict) else {}
        proposal["replayed"] = (
            proposal_raw.get("replayed")
            if isinstance(proposal_raw, dict)
            else None
        )
        proposal["requiresUserReview"] = (
            proposal.get("trustState") == "review-required"
        )
        learning_id = str(proposal.get("learningId", ""))
        learning = cli_json(
            ["learning", "show", learning_id, str(project), "--json"]
        )
        search = mcp_call(
            project,
            "ley_search",
            {
                "query": query,
                "maxResults": 8,
                "maxTokens": 1_500,
            },
        )
        compiled = mcp_call(
            project,
            "ley_brief",
            {
                "task": query,
                "maxResults": 8,
                "maxTokens": 1_500,
            },
        )
        later_host_id = f"{scenario['id']}-later-host"
        startup = hook_call(
            project,
            "codex",
            {
                "hook_event_name": "SessionStart",
                "session_id": later_host_id,
            },
        )
        later_prompt = hook_call(
            project,
            "codex",
            {
                "hook_event_name": "UserPromptSubmit",
                "session_id": later_host_id,
                "turn_id": "delayed-poisoning-later-turn",
                "prompt": query,
            },
        )
        startup_context = hook_additional_context(startup)
        task_context = hook_additional_context(later_prompt)

        lineage = learning.get("originLineage", {})
        lineage_sources = (
            lineage.get("sources", [])
            if isinstance(lineage, dict)
            else []
        )
        search_learning = next(
            (
                item
                for item in search.get("results", [])
                if isinstance(item, dict)
                and item.get("kind") == "learning"
                and item.get("learningId") == learning_id
            ),
            {},
        )
        compiler_exclusion = next(
            (
                item
                for item in compiled.get("exclusions", [])
                if isinstance(item, dict)
                and item.get("entityId") == learning_id
            ),
            {},
        )
        compiler_items_text = json.dumps(
            compiled.get("items", []),
            sort_keys=True,
        )
        source_item = next(
            (
                item
                for item in compiled.get("items", [])
                if isinstance(item, dict)
                and item.get("kind") == "artifact"
                and item.get("citation", {}).get("artifactPath")
                == source_path
            ),
            {},
        )
        delayed_poisoning_checks = {
            "proposal-review-required":
                learning_id.startswith("lrn_")
                and proposal.get("state") == "tentative"
                and proposal.get("trustState") == "review-required"
                and proposal.get("requiresUserReview") is True
                and proposal.get("replayed") is False,
            "learning-not-reusable":
                learning.get("state") == "tentative"
                and learning.get("trustState") == "review-required",
            "lineage-ceiling":
                isinstance(lineage, dict)
                and lineage.get("automaticAuthorityCeiling")
                == "review-required"
                and lineage.get("causalCompletenessProven") is False
                and lineage.get("mechanicallyResolved") is True,
            "lineage-session-record":
                any(
                    isinstance(item, dict)
                    and item.get("kind") == "session-record"
                    and item.get("sessionId") == poisoning_session_id
                    and item.get("recordId") == poisoning_checkpoint_id
                    for item in lineage_sources
                ),
            "lineage-captured-artifact":
                any(
                    isinstance(item, dict)
                    and item.get("kind") == "captured-artifact"
                    and item.get("artifactPath") == source_path
                    for item in lineage_sources
                ),
            "explicit-search-inspectable":
                search_learning.get("learningTrustState")
                == "review-required"
                and search_learning.get("trustSignal") == "unverified"
                and search_learning.get("trustedForReuse") is False
                and poison_marker
                in str(search_learning.get("excerpt", "")),
            "compiler-excludes-learning":
                compiler_exclusion.get("kind") == "learning"
                and compiler_exclusion.get("stage") == "admission"
                and compiler_exclusion.get("reason")
                == "unverified-learning"
                and compiler_exclusion.get("trustSignal")
                == "unverified"
                and poison_marker not in compiler_items_text,
            "source-remains-evidence":
                source_marker in str(source_item.get("excerpt", ""))
                and source_item.get("authority") == "direct-evidence"
                and source_item.get("trustedForReuse") is False,
            "local-review-state-inspectable":
                learning.get("trustState") == "review-required"
                and learning.get("state") == "tentative"
                and poison_marker in str(learning.get("guidance", "")),
            "startup-does-not-inject-poison":
                poison_marker not in startup_context,
            "prompt-hook-is-capture-only":
                bool(task_context)
                and poison_marker not in task_context
                and source_marker not in task_context
                and "does not inject task-specific project history" in task_context
                and "call ley_brief" in task_context.lower()
                and "# Ley task context (automatic)" not in task_context,
        }
        delayed_poisoning_ok = all(
            delayed_poisoning_checks.values()
        )
        scores["delayed_poisoning_resistance"] = (
            delayed_poisoning_ok
        )
        poisoning_outputs = [
            proposal,
            learning,
            search,
            compiled,
            startup,
            later_prompt,
        ]
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)],
            poisoning_outputs,
        )
        evidence_text.extend(poisoning_outputs)
        if not delayed_poisoning_ok:
            failed_delayed_poisoning_checks = [
                label
                for label, passed in delayed_poisoning_checks.items()
                if not passed
            ]
            failures.append(
                "delayed repository-text poisoning crossed a review/trust boundary: "
                + ", ".join(failed_delayed_poisoning_checks)
            )

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
            init_project(project_dir, name)
            project_dirs.append(project_dir)
        query = str(scenario.get("query_from_alpha", ["cross project"])[0])
        payload = mcp_call(
            project_dirs[0],
            "ley_search",
            {"query": query, "maxResults": K, "maxTokens": 500},
        )
        other = projects[1]
        markers = [str(other["name"])] + [str(path) for path in other.get("files", {})] + [str(value) for value in other.get("files", {}).values()]
        returned = serialized([payload]).lower()
        exposed = sum(marker.lower() in returned for marker in markers)
        leak = exposed > 0
        cross_project_clean = not leak

        isolation = scenario.get("expected_cross_project_isolation")
        if isinstance(isolation, dict):
            foreign_index = int(isolation.get("foreign_project_index", 1))
            if foreign_index <= 0 or foreign_index >= len(project_dirs):
                raise RuntimeError("cross-project isolation fixture selected an invalid foreign project")
            foreign_project = project_dirs[foreign_index]
            session_marker = str(isolation.get("session_marker", ""))
            learning_marker = str(isolation.get("learning_marker", ""))
            source_marker = str(isolation.get("source_marker", ""))
            isolation_query = str(isolation.get("query", query))

            foreign_session_id, _ = create_structured_session(
                foreign_project,
                seed=f"{scenario['id']}:foreign-session",
                name=f"Beta private session {session_marker}",
                goal=f"Preserve private Beta continuity {session_marker}",
                summary=f"Private Beta checkpoint {session_marker}",
                decisions=[
                    {
                        "title": "Private Beta billing decision",
                        "decision": session_marker,
                    }
                ],
            )
            foreign_session = mcp_call(
                foreign_project,
                "ley_session_get",
                {
                    "sessionId": foreign_session_id,
                    "maxCheckpoints": 5,
                    "maxCharacters": 8_000,
                },
            )
            foreign_checkpoints = [
                item
                for item in foreign_session.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            if not foreign_checkpoints:
                raise RuntimeError("cross-project isolation fixture created no foreign checkpoint")
            foreign_checkpoint_id = str(foreign_checkpoints[-1].get("checkpointId", ""))
            foreign_source = mcp_call(
                foreign_project,
                "ley_search",
                {"query": "RATE", "maxResults": 20, "maxTokens": 2_000},
            )
            proposed = mcp_call(
                foreign_project,
                "ley_learning_propose",
                {
                    "requestId": request_id(f"{scenario['id']}:foreign-learning:proposal"),
                    "kind": "fact",
                    "title": f"Private Beta billing knowledge {learning_marker}",
                    "guidance": f"Keep {learning_marker} inside Beta Billing only.",
                    "confidencePercent": 95,
                    "provenance": "agent-authored",
                    "evidence": [
                        {
                            "sessionId": foreign_session_id,
                            "recordId": foreign_checkpoint_id,
                            "note": "Cross-project isolation evaluation evidence.",
                        }
                    ],
                },
                WRITE_FLAGS,
            )
            foreign_learning_id = str(proposed.get("learningId", ""))
            reviewed = cli_json(
                [
                    "learning",
                    "review",
                    foreign_learning_id,
                    str(foreign_project),
                    "--actor",
                    "user",
                    "--action",
                    "confirm",
                    "--note",
                    "Explicitly trusted foreign learning for isolation evaluation.",
                    "--request-id",
                    request_id(f"{scenario['id']}:foreign-learning:review"),
                    "--json",
                ]
            )
            foreign_learning = cli_json(
                [
                    "learning",
                    "show",
                    foreign_learning_id,
                    str(foreign_project),
                    "--json",
                ]
            )
            reviewed_learning = (
                reviewed.get("learning", {}) if isinstance(reviewed, dict) else {}
            )
            source_canary_ok = source_marker in serialized([foreign_source])
            session_canary_ok = session_marker in serialized([foreign_session])
            learning_canary_ok = learning_marker in serialized([foreign_learning])
            learning_trusted_ok = (
                reviewed_learning.get("state") == "verified"
                and reviewed_learning.get("trustState") == "trusted"
            )
            foreign_canaries_proven = (
                source_canary_ok
                and session_canary_ok
                and learning_canary_ok
                and learning_trusted_ok
            )
            if not foreign_canaries_proven:
                failures.append(
                    "cross-project isolation fixture did not positively prove all foreign canaries inside Beta: "
                    f"source={source_canary_ok}, "
                    f"session={session_canary_ok}, "
                    f"learning={learning_canary_ok}, "
                    f"trusted={learning_trusted_ok}"
                )
            cli_json(
                [
                    "session",
                    "finish",
                    foreign_session_id,
                    str(foreign_project),
                    "--summary",
                    f"Completed private Beta work {session_marker}",
                    "--json",
                ]
            )

            alpha = project_dirs[0]
            alpha_memory = mcp_call(
                alpha,
                "ley_search",
                {"query": isolation_query, "maxResults": 20, "maxTokens": 2_000},
            )
            alpha_compiled = mcp_call(
                alpha,
                "ley_brief",
                {"task": isolation_query, "maxResults": 20, "maxTokens": 2_000},
            )
            alpha_sessions = mcp_call(
                alpha,
                "ley_sessions_list",
                {"maxResults": 50},
            )
            alpha_learnings = mcp_call(
                alpha,
                "ley_learnings_list",
                {"scope": "all", "maxResults": 50},
            )

            foreign_session_rejected = False
            foreign_session_probe: object = {}
            try:
                foreign_session_probe = mcp_call(
                    alpha,
                    "ley_session_get",
                    {
                        "sessionId": foreign_session_id,
                        "maxCheckpoints": 5,
                        "maxCharacters": 8_000,
                    },
                )
            except RuntimeError as error:
                foreign_session_probe = str(error)
                foreign_session_rejected = (
                    "session not found in this fixed project" in str(error)
                )

            foreign_learning_rejected = False
            foreign_learning_probe: object = {}
            try:
                foreign_learning_probe = mcp_call(
                    alpha,
                    "ley_learning_get",
                    {
                        "learningId": foreign_learning_id,
                        "maxEvidence": 10,
                        "maxHistory": 10,
                        "maxArtifactsPerEvidence": 10,
                        "maxCharacters": 8_000,
                    },
                )
            except RuntimeError as error:
                foreign_learning_probe = str(error)
                foreign_learning_rejected = (
                    "learning not found in this fixed project" in str(error)
                )

            isolated_outputs: list[object] = [
                alpha_memory,
                context_contract_text(alpha_compiled),
                alpha_sessions,
                alpha_learnings,
                foreign_session_probe,
                foreign_learning_probe,
            ]
            private_markers = [source_marker, session_marker, learning_marker]
            private_markers = [marker for marker in private_markers if marker]
            isolated_text = serialized(isolated_outputs)
            private_exposed = sum(marker in isolated_text for marker in private_markers)
            fixed_project_isolation_ok = (
                foreign_canaries_proven
                and private_exposed == 0
                and foreign_session_rejected
                and foreign_learning_rejected
            )
            cross_project_clean = cross_project_clean and fixed_project_isolation_ok
            markers.extend(private_markers)
            exposed += private_exposed
            if not fixed_project_isolation_ok:
                failures.append(
                    "fixed-project isolation leaked foreign source/session/learning memory or accepted a foreign stable ID: "
                    f"markersExposed={private_exposed}, "
                    f"foreignSessionRejected={foreign_session_rejected}, "
                    f"foreignLearningRejected={foreign_learning_rejected}"
                )

        scores["privacy_violation_rate"] = exposed / len(markers) if markers else 0.0
        scores["cross_project_clean"] = cross_project_clean
        if leak:
            failures.append("cross-project search returned another project's marker")

    stale_use_time = scenario.get("expected_stale_use_time")
    stale_use_time_learning_id = ""
    if isinstance(stale_use_time, dict):
        stale_title = str(stale_use_time.get("title", ""))
        pre_change = cli_json(["learning", "list", str(project), "--json"])
        pre_change_learning = next(
            (
                item
                for item in pre_change
                if isinstance(item, dict) and item.get("title") == stale_title
            ),
            None,
        )
        if not isinstance(pre_change_learning, dict):
            raise RuntimeError(
                "stale-learning fixture could not find its pre-change learning"
            )
        stale_use_time_learning_id = str(
            pre_change_learning.get("learningId", "")
        )
        reviewed = cli_json(
            [
                "learning",
                "review",
                stale_use_time_learning_id,
                str(project),
                "--actor",
                "user",
                "--action",
                "confirm",
                "--note",
                "Explicitly trusted before source-change invalidation evaluation.",
                "--request-id",
                request_id(f"{scenario['id']}:stale:pre-change-review"),
                "--json",
            ]
        )
        reviewed_learning = (
            reviewed.get("learning", {}) if isinstance(reviewed, dict) else {}
        )
        if not (
            reviewed_learning.get("state") == "verified"
            and reviewed_learning.get("trustState") == "trusted"
            and reviewed_learning.get("freshness") == "current"
        ):
            raise RuntimeError(
                "stale-learning fixture could not establish trusted current pre-change state"
            )
        evidence_text.extend([pre_change, reviewed])

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

    known_failure = scenario.get("expected_known_failure_reuse")
    if isinstance(known_failure, dict):
        query = str(known_failure.get("query", ""))
        incident_query = str(known_failure.get("incident_query", ""))
        expected_title = str(known_failure.get("problem_title", ""))
        expected_failed = known_failure.get("failed_attempt", {})
        expected_successful = known_failure.get("successful_attempt", {})
        expected_root_cause = str(known_failure.get("root_cause", ""))
        expected_change = str(known_failure.get("change", ""))
        expected_verification = str(known_failure.get("verification", ""))
        expected_citation = str(known_failure.get("citation", ""))
        procedure_spec = known_failure.get("procedure")
        normalized_incident_query = incident_query.strip().lower()
        normalized_problem_title = expected_title.strip().lower()
        normalized_baseline_query = query.strip().lower()
        if (
            not normalized_incident_query
            or (
                normalized_problem_title
                and normalized_problem_title in normalized_incident_query
            )
            or (
                normalized_baseline_query
                and normalized_baseline_query in normalized_incident_query
            )
        ):
            raise RuntimeError(
                "known-failure reuse fixture requires an incident_query that does not embed the stored Problem title/baseline query"
            )

        memory_search = mcp_call(
            project,
            "ley_search",
            {"query": query, "maxResults": K, "maxTokens": 1_500},
        )
        incident_memory_search = mcp_call(
            project,
            "ley_search",
            {"query": incident_query, "maxResults": K, "maxTokens": 1_500},
        )
        evidence_text.extend(
            [
                memory_search,
                incident_memory_search,
            ]
        )

        baseline_memory_problem = next(
            (
                result
                for result in memory_search.get("results", [])
                if isinstance(result, dict)
                and result.get("kind") == "problem"
                and result.get("title") == expected_title
            ),
            None,
        )
        memory_problem = next(
            (
                result
                for result in incident_memory_search.get("results", [])
                if isinstance(result, dict)
                and result.get("kind") == "problem"
                and result.get("title") == expected_title
            ),
            None,
        )
        incident_session = (
            mcp_call(
                project,
                "ley_session_get",
                {
                    "sessionId": str(memory_problem.get("sessionId", "")),
                    "maxCheckpoints": 10,
                    "maxCharacters": 12_000,
                },
            )
            if isinstance(memory_problem, dict)
            else {}
        )
        evidence_text.append(incident_session)
        incident_checkpoint = next(
            (
                checkpoint
                for checkpoint in incident_session.get("checkpoints", [])
                if isinstance(checkpoint, dict)
                and any(
                    isinstance(problem, dict)
                    and problem.get("id") == memory_problem.get("entityId")
                    for problem in checkpoint.get("problems", [])
                )
            ),
            None,
        )
        session_problem = next(
            (
                problem
                for problem in (
                    incident_checkpoint.get("problems", [])
                    if isinstance(incident_checkpoint, dict)
                    else []
                )
                if isinstance(problem, dict)
                and problem.get("id") == memory_problem.get("entityId")
                and problem.get("title") == expected_title
            ),
            None,
        )

        def attempt_matches(attempt: object, expected: object) -> bool:
            return (
                isinstance(attempt, dict)
                and isinstance(expected, dict)
                and attempt.get("action") == expected.get("action")
                and attempt.get("outcome") == expected.get("outcome")
            )

        resolution = (
            session_problem.get("resolutionDetail")
            if isinstance(session_problem, dict)
            else None
        )
        citations = (
            incident_checkpoint.get("touchedArtifacts", [])
            if isinstance(incident_checkpoint, dict)
            else []
        )
        stable_handle_ok = (
            isinstance(baseline_memory_problem, dict)
            and isinstance(memory_problem, dict)
            and isinstance(session_problem, dict)
            and session_problem.get("id") == memory_problem.get("entityId")
            and incident_session.get("sessionId") == memory_problem.get("sessionId")
            and memory_problem.get("entityId")
            == baseline_memory_problem.get("entityId")
            and memory_problem.get("sessionId")
            == baseline_memory_problem.get("sessionId")
        )
        resolution_ok = (
            isinstance(resolution, dict)
            and resolution.get("rootCause") == expected_root_cause
            and resolution.get("change") == expected_change
            and resolution.get("verification") == expected_verification
        )
        citation_ok = any(
            isinstance(citation, dict)
            and citation.get("artifactPath") == expected_citation
            and isinstance(citation.get("artifactSnapshotId"), str)
            and len(citation["artifactSnapshotId"]) == 68
            and citation["artifactSnapshotId"].startswith("snp_")
            and all(
                character in "0123456789abcdef"
                for character in citation["artifactSnapshotId"][4:]
            )
            and isinstance(citation.get("contentHash"), str)
            and len(citation["contentHash"]) == 71
            and citation["contentHash"].startswith("sha256:")
            and all(
                character in "0123456789abcdef"
                for character in citation["contentHash"][7:]
            )
            for citation in citations
        )
        raw_problem_attempts = (
            session_problem.get("attempts", [])
            if isinstance(session_problem, dict)
            else []
        )
        problem_attempts = (
            raw_problem_attempts
            if isinstance(raw_problem_attempts, list)
            and len(raw_problem_attempts) == 2
            and all(isinstance(attempt, dict) for attempt in raw_problem_attempts)
            else []
        )
        failed_attempt_ok = (
            len(problem_attempts) == 2
            and attempt_matches(problem_attempts[0], expected_failed)
        )
        successful_attempt_ok = (
            len(problem_attempts) == 2
            and attempt_matches(problem_attempts[1], expected_successful)
        )
        procedure_ok = procedure_spec is None
        if isinstance(procedure_spec, dict) and isinstance(session_problem, dict):
            procedure_title = str(procedure_spec.get("title", ""))
            procedure_guidance = str(procedure_spec.get("guidance", ""))
            procedure_query = str(procedure_spec.get("query", ""))
            problem_session_id = str(memory_problem.get("sessionId", ""))
            problem_record_id = str(session_problem.get("id", ""))
            proposal = mcp_call(
                project,
                "ley_learning_propose",
                {
                    "requestId": request_id(
                        f"{scenario['id']}:known-failure:procedure:proposal"
                    ),
                    "kind": "procedure",
                    "title": procedure_title,
                    "guidance": procedure_guidance,
                    "confidencePercent": 90,
                    "provenance": "agent-authored",
                    "evidence": [
                        {
                            "sessionId": problem_session_id,
                            "recordId": problem_record_id,
                            "note": "Reviewed known-failure recovery procedure.",
                        }
                    ],
                },
                WRITE_FLAGS,
            )
            procedure_learning_id = str(proposal.get("learningId", ""))
            reviewed = cli_json(
                [
                    "learning",
                    "review",
                    procedure_learning_id,
                    str(project),
                    "--actor",
                    "user",
                    "--action",
                    "confirm",
                    "--note",
                    "Reviewed reusable procedure from the verified watcher recovery.",
                    "--request-id",
                    request_id(f"{scenario['id']}:known-failure:procedure:review"),
                    "--json",
                ]
            )
            procedure_search = mcp_call(
                project,
                "ley_search",
                {
                    "query": procedure_query,
                    "maxResults": K,
                    "maxTokens": 1_500,
                },
            )
            incident_reuse_search = mcp_call(
                project,
                "ley_search",
                {
                    "query": incident_query,
                    "maxResults": K,
                    "maxTokens": 1_500,
                },
            )
            evidence_text.extend(
                [proposal, reviewed, procedure_search, incident_reuse_search]
            )
            reviewed_learning = (
                reviewed.get("learning", {}) if isinstance(reviewed, dict) else {}
            )
            procedure_result = next(
                (
                    result
                    for result in procedure_search.get("results", [])
                    if isinstance(result, dict)
                    and result.get("kind") == "learning"
                    and result.get("learningId") == procedure_learning_id
                    and result.get("title") == procedure_title
                ),
                None,
            )
            incident_procedure_result = next(
                (
                    result
                    for result in incident_reuse_search.get("results", [])
                    if isinstance(result, dict)
                    and result.get("kind") == "learning"
                    and result.get("learningId") == procedure_learning_id
                    and result.get("title") == procedure_title
                ),
                None,
            )
            incident_problem_result = next(
                (
                    result
                    for result in incident_reuse_search.get("results", [])
                    if isinstance(result, dict)
                    and result.get("kind") == "problem"
                    and isinstance(memory_problem, dict)
                    and result.get("entityId") == memory_problem.get("entityId")
                    and result.get("sessionId") == memory_problem.get("sessionId")
                    and result.get("title") == expected_title
                ),
                None,
            )
            procedure_ok = (
                reviewed_learning.get("state") == "verified"
                and reviewed_learning.get("trustState") == "trusted"
                and reviewed_learning.get("freshness") == "current"
                and isinstance(procedure_result, dict)
                and procedure_result.get("trustedForReuse") is True
                and procedure_result.get("trustSignal") == "trusted-current"
                and procedure_result.get("excerpt") == procedure_guidance
                and isinstance(incident_procedure_result, dict)
                and incident_procedure_result.get("trustedForReuse") is True
                and incident_procedure_result.get("trustSignal") == "trusted-current"
                and incident_procedure_result.get("excerpt") == procedure_guidance
                and isinstance(incident_problem_result, dict)
            )
        known_failure_ok = (
            stable_handle_ok
            and failed_attempt_ok
            and successful_attempt_ok
            and resolution_ok
            and citation_ok
            and procedure_ok
        )
        scores["known_failure_reuse"] = known_failure_ok
        if not known_failure_ok:
            incident_memory_summary = [
                {
                    "kind": item.get("kind"),
                    "title": item.get("title"),
                    "entityId": item.get("entityId"),
                }
                for item in incident_memory_search.get("results", [])
                if isinstance(item, dict)
            ]
            incident_session_summary = {
                "sessionId": incident_session.get("sessionId"),
                "problemId": (
                    session_problem.get("id")
                    if isinstance(session_problem, dict)
                    else None
                ),
            }
            failures.append(
                "known-failure reuse incomplete: "
                f"stableHandle={stable_handle_ok}, "
                f"failedAttempt={failed_attempt_ok}, "
                f"successfulAttempt={successful_attempt_ok}, "
                f"resolution={resolution_ok}, "
                f"citation={citation_ok}, "
                f"procedure={procedure_ok}; "
                f"incidentMemory={json.dumps(incident_memory_summary, sort_keys=True)}, "
                f"incidentSession={json.dumps(incident_session_summary, sort_keys=True)}"
            )

    if scenario.get("expected_stale_learning"):
        learnings = cli_json(["learning", "list", str(project), "--json"])
        title = str(scenario["expected_stale_learning"])
        matches = [item for item in learnings if isinstance(item, dict) and item.get("title") == title]
        stale = bool(matches) and any(item.get("freshness") == "source-changed" or item.get("state") == "stale" for item in matches)
        scores["stale_learning"] = stale
        if not stale:
            failures.append("source-changed learning was not disclosed as stale")

    if isinstance(stale_use_time, dict):
        stale_title = str(stale_use_time.get("title", ""))
        stale_guidance = str(stale_use_time.get("guidance", ""))
        stale_query = str(stale_use_time.get("query", ""))
        stale_task = str(stale_use_time.get("task", ""))
        all_learning = cli_json(
            ["learning", "show", stale_use_time_learning_id, str(project), "--json"]
        )
        stale_search = mcp_call(
            project,
            "ley_search",
            {"query": stale_query, "maxResults": K, "maxTokens": 1_500},
        )
        compiled = mcp_call(
            project,
            "ley_brief",
            {"task": stale_task, "maxResults": 8, "maxTokens": 1_500},
        )
        evidence_text.extend([all_learning, stale_search, compiled])
        stale_result = next(
            (
                result
                for result in stale_search.get("results", [])
                if isinstance(result, dict)
                and result.get("kind") == "learning"
                and result.get("learningId") == stale_use_time_learning_id
            ),
            None,
        )
        disclosed_stale_ok = (
            isinstance(all_learning, dict)
            and all_learning.get("title") == stale_title
            and all_learning.get("trustState") == "trusted"
            and (
                all_learning.get("freshness") == "source-changed"
                or all_learning.get("state") == "stale"
            )
        )
        search_marks_stale_ok = (
            isinstance(stale_result, dict)
            and stale_result.get("trustSignal") == "stale"
            and stale_result.get("trustedForReuse") is False
        )
        compiled_suppressed_ok = stale_guidance not in serialized([compiled])
        stale_recovery_ok = (
            disclosed_stale_ok
            and search_marks_stale_ok
            and compiled_suppressed_ok
        )
        scores["stale_learning_recovery"] = stale_recovery_ok
        if not stale_recovery_ok:
            failures.append(
                "stale-learning recovery incomplete: "
                f"disclosed={disclosed_stale_ok}, "
                f"searchStale={search_marks_stale_ok}, "
                f"compiledSuppressed={compiled_suppressed_ok}"
            )

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

    if scenario.get("expected_interruption_recovery"):
        if not session_id:
            scores["interruption_recovery"] = False
            failures.append("interruption-recovery fixture created no session")
        else:
            session = cli_json(
                ["session", "show", session_id, str(project), "--json"]
            )
            turns = cli_json(
                [
                    "session",
                    "turns",
                    session_id,
                    str(project),
                    "--max-results",
                    "20",
                    "--max-characters",
                    "4000",
                    "--json",
                ]
            )
            tools = mcp_tools_list(project, ("--allow-session-writes",))
            tool_names = {
                str(tool.get("name", "")) for tool in tools if isinstance(tool, dict)
            }
            retained_turns = [
                item for item in turns.get("turns", []) if isinstance(item, dict)
            ]
            prompt_evidence = [
                item
                for item in retained_turns
                if item.get("kind") == "user-prompt"
                and str(item.get("recordId", "")).startswith("tev_")
                and "untrusted" in str(item.get("sourceBoundary", ""))
            ]
            recovery_ok = (
                isinstance(session, dict)
                and session.get("status") == "active"
                and session.get("promptCount") == 1
                and session.get("responseCount") == 0
                and isinstance(turns, dict)
                and turns.get("promptCount") == 1
                and turns.get("responseCount") == 0
                and turns.get("retainedTurnCount") == 1
                and len(prompt_evidence) == 1
                and prompt_evidence[0].get("text") == "Fix the login bug"
                and turns.get("liveSourceChecked") is False
                and bool(turns.get("instructionWarning"))
                and int(turns.get("textCharacters", 0)) <= 4_000
                and "ley_checkpoint" in tool_names
                and "ley_session_memory_compile" not in tool_names
                and "ley_session_start" not in tool_names
            )
            scores["interruption_recovery"] = recovery_ok
            scores["privacy_violation_rate"] = privacy_violation_rate(
                [str(project), str(vault)], [session, turns, tools]
            )
            evidence_text.extend([session, turns, tools])
            if not recovery_ok:
                failures.append(
                    "native crash recovery did not preserve bounded untrusted interruption evidence with only canonical checkpoint write authority"
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
) -> bool | None:
    if result.get("skipped"):
        return None
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
        if (
            label == "P0"
            and capability in P0_INDEPENDENT_DOWNSTREAM_CAPABILITIES
            and dimensions["downstream"][1] != "downstream_task_contract"
        ):
            raise RuntimeError(
                f"P0 capability {capability} must use downstream_task_contract for independent downstream evidence"
            )
        if (
            label == "P1"
            and capability in P1_INDEPENDENT_DOWNSTREAM_CAPABILITIES
            and dimensions["downstream"][1] != "downstream_task_contract"
        ):
            raise RuntimeError(
                f"P1 capability {capability} must use downstream_task_contract for independent downstream evidence"
            )
        if (
            label == "P2"
            and capability in P2_INDEPENDENT_DOWNSTREAM_CAPABILITIES
            and dimensions["downstream"][1] != "downstream_task_contract"
        ):
            raise RuntimeError(
                f"P2 capability {capability} must use downstream_task_contract for independent downstream evidence"
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
    parser.add_argument(
        "--p2-coverage",
        action="store_true",
        help="Run only implemented P2 coverage-matrix representative scenarios and enforce the matrix.",
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
    validate_coverage_config("P2", P2_CAPABILITY_COVERAGE, all_scenarios)
    if arguments.list:
        for scenario in all_scenarios:
            print(f"{scenario['id']}\t{scenario.get('category', '')}")
        return 0

    requested_ids = set(arguments.scenario)
    requested_categories = set(arguments.category)
    coverage_modes = sum(
        bool(value)
        for value in (
            arguments.p0_coverage,
            arguments.p1_coverage,
            arguments.p2_coverage,
        )
    )
    if coverage_modes > 1:
        raise SystemExit("--p0-coverage, --p1-coverage, and --p2-coverage are mutually exclusive")
    if coverage_modes and (
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
    elif arguments.p2_coverage:
        coverage_ids = {
            scenario_id
            for dimensions in P2_CAPABILITY_COVERAGE.values()
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
    enforce_p2_coverage = full_corpus or arguments.p2_coverage
    print(f"Running {len(scenarios)} eval scenarios with {LEY}...\n", flush=True)
    results: list[dict[str, object]] = []
    with tempfile.TemporaryDirectory(prefix="ley-eval-") as temporary:
        EVAL_ENV["XDG_CONFIG_HOME"] = str(Path(temporary) / "config")
        EVAL_ENV["XDG_CACHE_HOME"] = str(Path(temporary) / "cache")
        for index, scenario in enumerate(scenarios, start=1):
            try:
                result = evaluate_scenario(scenario, Path(temporary) / str(scenario["id"]))
            except BootstrapScenarioUnsupported as error:
                result = {
                    "id": str(scenario["id"]),
                    "category": str(scenario.get("category", "")),
                    "passed": False,
                    "skipped": True,
                    "skip_reason": str(error),
                    "failures": [],
                }
            except Exception as error:  # one broken scenario must not hide the rest
                result = {
                    "id": str(scenario["id"]),
                    "category": str(scenario.get("category", "")),
                    "passed": False,
                    "failures": [f"unhandled scenario error: {error}"],
                }
            results.append(result)
            status = (
                "SKIP"
                if result.get("skipped")
                else ("PASS" if result.get("passed") else "FAIL")
            )
            print(f"[{index}/{len(scenarios)}] {scenario['id']}: {status}", flush=True)
            if result.get("skipped"):
                print(f"  skip_reason: {result.get('skip_reason', '')}", flush=True)
            for metric in METRIC_NAMES:
                if result.get(metric) is not None:
                    print(f"  {metric}: {result[metric]}", flush=True)
            for failure in result.get("failures", []):
                print(f"  ERROR: {failure}", flush=True)

    skipped = sum(bool(result.get("skipped")) for result in results)
    passed = sum(
        bool(result.get("passed"))
        for result in results
        if not result.get("skipped")
    )
    runnable = len(results) - skipped
    print(f"\n=== Aggregate ({len(results)} scenarios) ===", flush=True)
    print(f"Scenarios passed: {passed}/{runnable}", flush=True)
    if skipped:
        print(f"Scenarios skipped as unsupported: {skipped}", flush=True)
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
                dimension_results.append(
                    f"{dimension}={'SKIP' if ok is None else ('PASS' if ok else 'FAIL')}"
                )
                if ok is False:
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
                dimension_results.append(
                    f"{dimension}={'SKIP' if ok is None else ('PASS' if ok else 'FAIL')}"
                )
                if ok is False:
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

    if enforce_p2_coverage:
        result_by_id = {
            str(result.get("id")): result
            for result in results
            if isinstance(result.get("id"), str)
        }
        print("", flush=True)
        print("=== P2 capability metric coverage ===", flush=True)
        for capability, dimensions in P2_CAPABILITY_COVERAGE.items():
            dimension_results: list[str] = []
            for dimension, (scenario_id, metric, expectation) in dimensions.items():
                result = result_by_id.get(scenario_id, {})
                ok = metric_requirement_passes(result, metric, expectation)
                dimension_results.append(
                    f"{dimension}={'SKIP' if ok is None else ('PASS' if ok else 'FAIL')}"
                )
                if ok is False:
                    coverage_failures.append(
                        f"{capability}/{dimension} requires {scenario_id}:{metric}={expectation}"
                    )
            print(f"{capability}: " + ", ".join(dimension_results), flush=True)
        for failure in coverage_failures:
            if any(
                failure.startswith(f"{capability}/")
                for capability in P2_CAPABILITY_COVERAGE
            ):
                print(f"  COVERAGE ERROR: {failure}", flush=True)
    else:
        print(
            "P2 capability coverage matrix: skipped for focused subset run.",
            flush=True,
        )

    return 0 if passed == runnable and not coverage_failures else 1


if __name__ == "__main__":
    sys.exit(main())
