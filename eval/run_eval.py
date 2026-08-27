#!/usr/bin/env python3
"""Run the real Ley CLI/MCP evaluation fixtures as fail-fast assertions.

Every scenario must execute its declared setup and checks. A missing tool
response, skipped event kind, invalid fixture identifier, or unmet expectation
is a failed scenario and makes this command exit non-zero.
"""

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


def hook_call(project: Path, host: str, payload: dict[str, str]) -> dict[str, object]:
    output = run(["hook", str(project), "--host", host], stdin=json.dumps(payload))
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

    checkpoint: dict[str, object] = {
        "summary": ("; ".join(summaries) or "Captured structured project progress.")[:16000],
        "decisions": decisions,
        "problems": problems,
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
        if event.get("type") in {"decision", "problem", "attempt", "resolution", "learning"}
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
    scores: dict[str, object] = {
        "recall@k": None,
        "precision": None,
        "untrusted_boundary": None,
        "cross_project_clean": None,
        "stale_learning": None,
        "capture_recovery": None,
        "idempotency": None,
        "token_budget": None,
        "secret_exclusion": None,
    }

    project = base_dir / "project"
    vault = base_dir / "vault"
    project.mkdir(parents=True)
    vault.mkdir(parents=True)
    files = scenario.get("project_files", {})
    if isinstance(files, dict):
        write_project_files(project, files)
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
        leak = any(marker.lower() in serialized([payload]).lower() for marker in markers)
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
        clean = all(value not in vault_text and value not in serialized(evidence_text) for value in raw_values)
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

    return {**scores, "passed": not failures, "failures": failures}


def main() -> int:
    scenarios = [json.loads(line) for line in FIXTURES.read_text(encoding="utf-8").splitlines() if line.strip()]
    print(f"Running {len(scenarios)} eval scenarios with {LEY}...\n", flush=True)
    results: list[dict[str, object]] = []
    with tempfile.TemporaryDirectory(prefix="ley-eval-") as temporary:
        EVAL_ENV["XDG_CONFIG_HOME"] = str(Path(temporary) / "config")
        for index, scenario in enumerate(scenarios, start=1):
            try:
                result = evaluate_scenario(scenario, Path(temporary) / str(scenario["id"]))
            except Exception as error:  # one broken scenario must not hide the rest
                result = {"passed": False, "failures": [f"unhandled scenario error: {error}"]}
            results.append(result)
            print(
                f"[{index}/{len(scenarios)}] {scenario['id']}: {'PASS' if result.get('passed') else 'FAIL'}",
                flush=True,
            )
            for metric in ("recall@k", "precision", "untrusted_boundary", "cross_project_clean", "stale_learning", "capture_recovery", "idempotency", "token_budget", "secret_exclusion"):
                if result.get(metric) is not None:
                    print(f"  {metric}: {result[metric]}", flush=True)
            for failure in result.get("failures", []):
                print(f"  ERROR: {failure}", flush=True)

    passed = sum(bool(result.get("passed")) for result in results)
    print(f"\n=== Aggregate ({len(results)} scenarios) ===", flush=True)
    print(f"Scenarios passed: {passed}/{len(results)}", flush=True)
    recall = [float(result["recall@k"]) for result in results if result.get("recall@k") is not None]
    precision = [float(result["precision"]) for result in results if result.get("precision") is not None]
    if recall:
        print(f"Mean recall@{K}: {sum(recall) / len(recall):.3f}", flush=True)
    if precision:
        print(f"Mean precision: {sum(precision) / len(precision):.3f}", flush=True)
    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
