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
    "learning_idempotency",
    "delayed_poisoning_resistance",
    "token_budget",
    "secret_exclusion",
    "specification_admission",
    "mounted_reference",
    "premise_adjudication",
    "revision_adjudication",
    "egress_policy",
    "selective_abstention",
    "parallel_session_separation",
    "cross_surface_staleness",
    "long_horizon_continuity",
    "deletion_fidelity",
    "forgetting_residue_rate",
    "inactive_workspace_clean",
    "host_portability",
    "downstream_task_contract",
    "budget_baseline_advantage",
    "retrieval_robustness",
    "privacy_violation_rate",
    "topic_dossier",
    "current_project_state",
    "context_pack_inspector",
    "memory_health",
    "agent_legibility",
    "reviewed_runbook",
    "verification_evidence_links",
    "branch_worktree_controls",
    "graph_relation_retrieval",
    "trace_to_code_retrieval",
    "ripple_effect_retrieval",
    "context_memory_utility",
    "procedure_application_history",
    "procedure_outcome_health_attention",
    "external_connector",
    "multimodal_evidence",
    "knowledge_scope",
    "policy_bundle",
    "historical_host_import",
    "consolidation_inbox",
    "bootstrap_specification",
    "bootstrap_reference",
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
    "context-mounts": {
        "adversarial": (
            "explicit-project-context-mount",
            "mounted_reference",
            "truthy",
        ),
        "downstream": (
            "explicit-project-context-mount",
            "downstream_task_contract",
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
            "downstream_task_contract",
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
        "context-mounts",
        "origin-lineage",
        "premise-adjudication",
        "revision-awareness",
        "egress-policy",
    }
)

P1_INDEPENDENT_DOWNSTREAM_CAPABILITIES = frozenset(
    {
        "bootstrap-specifications",
        "bootstrap-reference-projects",
        "topic-dossiers",
        "current-project-state",
        "reviewed-runbook-skill-export",
        "richer-graph-relations",
    }
)

P2_INDEPENDENT_DOWNSTREAM_CAPABILITIES = frozenset(
    {
        "external-reference-connectors",
        "team-organization-knowledge-scopes",
        "team-organization-policy-bundles",
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
    "bootstrap-reference-projects": {
        "adversarial": (
            "empty-workspace-bootstrap-reference",
            "bootstrap_reference",
            "truthy",
        ),
        "downstream": (
            "empty-workspace-bootstrap-reference",
            "downstream_task_contract",
            "truthy",
        ),
        "privacy": (
            "empty-workspace-bootstrap-reference",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "empty-workspace-bootstrap-reference",
            "bootstrap_reference",
            "truthy",
        ),
    },
    "topic-dossiers": {
        "adversarial": (
            "session-erasure-derived-residue",
            "forgetting_residue_rate",
            "zero",
        ),
        "downstream": (
            "topic-dossier-authentication",
            "downstream_task_contract",
            "truthy",
        ),
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
            "downstream_task_contract",
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
    "memory-health": {
        "adversarial": (
            "session-erasure-derived-residue",
            "forgetting_residue_rate",
            "zero",
        ),
        "downstream": (
            "memory-health-hygiene",
            "memory_health",
            "truthy",
        ),
        "privacy": (
            "memory-health-hygiene",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "procedure-application-outcome-history",
            "procedure_outcome_health_attention",
            "truthy",
        ),
    },
    "agent-legibility-map": {
        "adversarial": (
            "session-erasure-derived-residue",
            "forgetting_residue_rate",
            "zero",
        ),
        "downstream": (
            "agent-legibility-project-map",
            "agent_legibility",
            "truthy",
        ),
        "privacy": (
            "agent-legibility-project-map",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "agent-legibility-project-map",
            "agent_legibility",
            "truthy",
        ),
    },
    "reviewed-runbook-skill-export": {
        "adversarial": (
            "reviewed-runbook-skill-export",
            "reviewed_runbook",
            "truthy",
        ),
        "downstream": (
            "reviewed-runbook-skill-export",
            "downstream_task_contract",
            "truthy",
        ),
        "privacy": (
            "reviewed-runbook-skill-export",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "reviewed-runbook-skill-export",
            "reviewed_runbook",
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
    "richer-graph-relations": {
        "adversarial": (
            "graph-relative-import-test-impact",
            "graph_relation_retrieval",
            "truthy",
        ),
        "downstream": (
            "graph-relative-import-test-impact",
            "downstream_task_contract",
            "truthy",
        ),
        "privacy": (
            "graph-relative-import-test-impact",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "graph-ripple-transitive-impact",
            "ripple_effect_retrieval",
            "truthy",
        ),
    },
    "context-memory-utility-feedback": {
        "adversarial": (
            "procedure-application-outcome-history",
            "procedure_application_history",
            "truthy",
        ),
        "downstream": (
            "context-memory-utility-feedback",
            "context_memory_utility",
            "truthy",
        ),
        "privacy": (
            "context-memory-utility-feedback",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "procedure-application-outcome-history",
            "procedure_application_history",
            "truthy",
        ),
    },
}

P2_CAPABILITY_COVERAGE = {
    "external-reference-connectors": {
        "adversarial": (
            "external-github-connector-egress",
            "external_connector",
            "truthy",
        ),
        "downstream": (
            "external-github-document-egress",
            "downstream_task_contract",
            "truthy",
        ),
        "privacy": (
            "external-github-connector-egress",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "external-github-document-egress",
            "external_connector",
            "truthy",
        ),
    },
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
    "team-organization-knowledge-scopes": {
        "adversarial": (
            "team-organization-knowledge-scope",
            "knowledge_scope",
            "truthy",
        ),
        "downstream": (
            "team-organization-knowledge-scope",
            "downstream_task_contract",
            "truthy",
        ),
        "privacy": (
            "team-organization-knowledge-scope",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "team-organization-knowledge-scope",
            "knowledge_scope",
            "truthy",
        ),
    },
    "team-organization-policy-bundles": {
        "adversarial": (
            "team-organization-policy-bundle",
            "policy_bundle",
            "truthy",
        ),
        "downstream": (
            "team-organization-policy-bundle",
            "downstream_task_contract",
            "truthy",
        ),
        "privacy": (
            "team-organization-policy-bundle",
            "privacy_violation_rate",
            "zero",
        ),
        "regression": (
            "team-organization-policy-bundle",
            "policy_bundle",
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
BOOTSTRAP_UNSUPPORTED_MARKER = (
    "cannot establish the directory generation required for Ley bootstrap Specification authority"
)


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


def init_project(
    project: Path, name: str, vault: Path, capture_mode: str = "structured"
) -> None:
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
    init_project(source, "Bootstrap Specification source", source_vault)

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
    install_specification_approvals(source, source_vault, definitions)
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
    expected_acceptance = expectation.get("acceptance_criteria", [])
    acceptance_projection = initial_spec.get("acceptanceCriteria", {})
    acceptance_rows = (
        acceptance_projection.get("criteria", [])
        if isinstance(acceptance_projection, dict)
        else []
    )
    acceptance_ok = True
    if isinstance(expected_acceptance, list) and expected_acceptance:
        acceptance_ok = (
            isinstance(acceptance_projection, dict)
            and acceptance_projection.get("state") == "available"
            and acceptance_projection.get("totalCriteria") == len(expected_acceptance)
            and acceptance_projection.get("returnedCriteria") == len(expected_acceptance)
            and acceptance_projection.get("omittedCriteria") == 0
            and acceptance_projection.get("sourceRevisionBound") is True
            and acceptance_projection.get("statusInterpreted") is False
            and acceptance_projection.get("persisted") is False
            and isinstance(acceptance_rows, list)
            and len(acceptance_rows) == len(expected_acceptance)
            and int(initial_spec.get("acceptanceCriteriaTokens", 0)) > 0
        )
        if acceptance_ok:
            for expected, actual in zip(expected_acceptance, acceptance_rows):
                if not isinstance(expected, dict) or not isinstance(actual, dict):
                    acceptance_ok = False
                    break
                if not (
                    actual.get("text") == expected.get("text")
                    and actual.get("startLine") == expected.get("start_line")
                    and actual.get("endLine") == expected.get("end_line")
                    and str(actual.get("criterionId", "")).startswith("acr_")
                    and all(
                        field not in actual
                        for field in (
                            "checked",
                            "completed",
                            "verified",
                            "satisfied",
                            "remaining",
                            "status",
                        )
                    )
                ):
                    acceptance_ok = False
                    break
    expected_methods = expectation.get("verification_methods", [])
    methods_projection = initial_spec.get("verificationMethods", {})
    method_rows = (
        methods_projection.get("methods", [])
        if isinstance(methods_projection, dict)
        else []
    )
    verification_methods_ok = True
    if isinstance(expected_methods, list) and expected_methods:
        verification_methods_ok = (
            isinstance(methods_projection, dict)
            and methods_projection.get("state") == "available"
            and methods_projection.get("totalMethods") == len(expected_methods)
            and methods_projection.get("returnedMethods") == len(expected_methods)
            and methods_projection.get("omittedMethods") == 0
            and methods_projection.get("sourceRevisionBound") is True
            and methods_projection.get("criterionBindingProven") is False
            and methods_projection.get("observedResultBindingProven") is False
            and methods_projection.get("statusInterpreted") is False
            and methods_projection.get("persisted") is False
            and isinstance(method_rows, list)
            and len(method_rows) == len(expected_methods)
            and int(initial_spec.get("verificationMethodsTokens", 0)) > 0
        )
        if verification_methods_ok:
            for expected, actual in zip(expected_methods, method_rows):
                if not isinstance(expected, dict) or not isinstance(actual, dict):
                    verification_methods_ok = False
                    break
                if not (
                    actual.get("text") == expected.get("text")
                    and actual.get("startLine") == expected.get("start_line")
                    and actual.get("endLine") == expected.get("end_line")
                    and str(actual.get("methodId", "")).startswith("vmd_")
                    and all(
                        field not in actual
                        for field in (
                            "criterionId",
                            "verificationRecordId",
                            "passed",
                            "verified",
                            "satisfied",
                            "status",
                        )
                    )
                ):
                    verification_methods_ok = False
                    break
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
        and acceptance_ok
        and verification_methods_ok
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
            "specification",
            specification_id,
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
        failures.append("source Specification never-send policy did not fail closed in bootstrap MCP")

    cli_json(
        [
            "egress",
            "specification",
            specification_id,
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


def evaluate_bootstrap_reference_scenario(
    scenario: dict[str, object],
    base_dir: Path,
    target: Path,
    expectation: dict[str, object],
) -> tuple[bool, bool, float, list[str]]:
    failures: list[str] = []
    source = base_dir / "bootstrap-reference-source"
    source_vault = base_dir / "bootstrap-reference-vault"
    unrelated = base_dir / "bootstrap-reference-unrelated"
    unrelated_vault = base_dir / "bootstrap-reference-unrelated-vault"
    for path in (source, source_vault, unrelated, unrelated_vault):
        path.mkdir(parents=True)

    marker = str(expectation.get("marker", ""))
    task = str(expectation.get("task", marker))
    unrelated_marker = str(expectation.get("unrelated_marker", ""))
    target_private_marker = str(expectation.get("target_private_marker", ""))
    if not marker or not task or not unrelated_marker or not target_private_marker:
        raise RuntimeError("bootstrap Reference eval fixture is incomplete")

    write_project_files(
        source,
        {
            "REFERENCE.md": (
                "Captured reference implementation evidence.\n"
                f"{marker} is the reusable source-project pattern for this task.\n"
            )
        },
    )
    init_project(source, "Bootstrap Reference source", source_vault)
    write_project_files(
        unrelated,
        {
            "UNRELATED.md": (
                "Unattached project with deliberately similar text.\n"
                f"{marker} {unrelated_marker} must never enter bootstrap context.\n"
            )
        },
    )
    init_project(unrelated, "Unrelated bootstrap reference", unrelated_vault)

    try:
        attached = cli_json(
            [
                "bootstrap-ref",
                "attach",
                str(source),
                str(target),
                "--json",
            ]
        )
    except RuntimeError as error:
        if BOOTSTRAP_UNSUPPORTED_MARKER in str(error):
            raise BootstrapScenarioUnsupported(
                "bootstrap Reference authority is unsupported on this platform/filesystem"
            ) from error
        raise
    listed = cli_json(["bootstrap-ref", "list", str(target), "--json"])
    if (target / ".ley").exists():
        failures.append("bootstrap Reference attachment initialized target .ley metadata")

    tools = mcp_tools_list(target)
    tool_names = [str(item.get("name", "")) for item in tools]
    compiled = mcp_call(
        target,
        "ley_compile_context",
        {"task": task, "maxResults": 8, "maxTokens": 2000},
    )
    references = compiled.get("references", [])
    reference_text = serialized(references)
    hook = hook_call(
        target,
        "codex",
        {
            "hook_event_name": "UserPromptSubmit",
            "session_id": "bootstrap-reference-eval",
            "turn_id": "bootstrap-reference-eval-turn-1",
            "prompt": task,
        },
    )
    initial_ok = (
        isinstance(attached, dict)
        and attached.get("created") is True
        and isinstance(listed, dict)
        and listed.get("targetInitialized") is False
        and listed.get("totalGrants") == 1
        and listed.get("ready") == 1
        and tool_names == ["ley_compile_context"]
        and compiled.get("projectMemoryAvailable") is False
        and compiled.get("referenceMemoryAuthorized") is True
        and compiled.get("automaticWriteAllowed") is False
        and compiled.get("targetInitialized") is False
        and isinstance(references, list)
        and marker in reference_text
        and unrelated_marker not in serialized(compiled)
        and target_private_marker not in serialized(compiled)
        and hook == {}
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
            [unrelated_marker, target_private_marker],
        )
    ]
    downstream_ok = task_contract_success(
        compiled,
        downstream_required,
        downstream_forbidden,
    )
    if not initial_ok:
        failures.append(
            "explicit Bootstrap Reference did not provide isolated captured MCP context while keeping hooks/target inactive"
        )
    if not downstream_ok:
        failures.append(
            "Bootstrap Reference context did not satisfy the independent downstream isolated-reference contract"
        )

    cli_json(["egress", "project", "never-send", str(source), "--json"])
    blocked = mcp_call(
        target,
        "ley_compile_context",
        {"task": task, "maxResults": 8, "maxTokens": 2000},
    )
    blocked_references = blocked.get("references", [])
    blocked_coverage = blocked.get("referenceCoverage", {})
    blocked_ok = (
        isinstance(blocked_references, list)
        and len(blocked_references) == 0
        and isinstance(blocked_coverage, dict)
        and blocked_coverage.get("egressBlocked") == 1
        and blocked_coverage.get("searchedSources") == 0
        and marker not in serialized(blocked_references)
    )
    if not blocked_ok:
        failures.append("source-project never-send policy did not block Bootstrap Reference search")

    cli_json(["egress", "project", "agent-ok", str(source), "--json"])
    cli_json(
        [
            "init",
            str(target),
            "--name",
            "Bootstrap Reference eval target",
            "--json",
        ]
    )
    after_init = cli_json(["bootstrap-ref", "list", str(target), "--json"])
    post_init_tools = mcp_tools_list(target)
    transition_ok = (
        isinstance(after_init, dict)
        and after_init.get("targetInitialized") is True
        and after_init.get("totalGrants") == 0
        and post_init_tools == []
        and (target / ".ley").is_dir()
    )
    if not transition_ok:
        failures.append("normal initialization did not retire Bootstrap Reference authority/tooling")

    agent_outputs: list[object] = [tools, compiled, hook, blocked, post_init_tools]
    privacy = privacy_violation_rate(
        [
            str(target),
            str(source),
            str(source_vault),
            str(unrelated),
            str(unrelated_vault),
            unrelated_marker,
            target_private_marker,
        ],
        agent_outputs,
    )
    if privacy != 0.0:
        failures.append("Bootstrap Reference agent output leaked a private path or unrelated/target canary")

    return not failures, downstream_ok, privacy, failures


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

    bootstrap_reference_expectation = scenario.get("expected_bootstrap_reference")
    if isinstance(bootstrap_reference_expectation, dict):
        passed, downstream, privacy, bootstrap_failures = evaluate_bootstrap_reference_scenario(
            scenario,
            base_dir,
            project,
            bootstrap_reference_expectation,
        )
        scores["bootstrap_reference"] = passed
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
        vault,
        str(scenario.get("capture_mode", "structured")),
    )
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

    runbook_expectation = scenario.get("expected_reviewed_runbook")
    if isinstance(runbook_expectation, dict):
        learning_receipts = [
            receipt
            for receipt in receipts
            if isinstance(receipt, dict) and isinstance(receipt.get("learningId"), str)
        ]
        if not learning_receipts:
            raise RuntimeError("reviewed runbook fixture created no learning proposals")
        learning_ids = [str(receipt["learningId"]) for receipt in learning_receipts]
        for index, learning_id in enumerate(learning_ids):
            cli_json(
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
                    "Reviewed for explicit runbook evaluation.",
                    "--request-id",
                    request_id(f"{scenario['id']}:runbook:confirm:{index}"),
                    "--json",
                ]
            )

        title = str(runbook_expectation.get("title", "Reviewed project runbook"))
        compile_args = ["runbook", "compile", str(project), "--title", title]
        for learning_id in learning_ids:
            compile_args.extend(["--learning", learning_id])
        compile_args.append("--json")
        compiled_runbook = cli_json(compile_args)
        rebuilt_runbook = cli_json(compile_args)
        reordered_compile_args = ["runbook", "compile", str(project), "--title", title]
        for learning_id in reversed(learning_ids):
            reordered_compile_args.extend(["--learning", learning_id])
        reordered_compile_args.append("--json")
        reordered_runbook = cli_json(reordered_compile_args)
        if (
            not isinstance(compiled_runbook, dict)
            or not isinstance(rebuilt_runbook, dict)
            or not isinstance(reordered_runbook, dict)
        ):
            raise RuntimeError("runbook compile returned a non-object payload")
        runbook_id = str(compiled_runbook.get("runbookId", ""))
        markers = [str(value) for value in runbook_expectation.get("markers", [])]
        hidden_marker = str(runbook_expectation.get("hidden_marker", ""))
        serialized_runbook = json.dumps(compiled_runbook, sort_keys=True)
        runbook_ok = (
            compiled_runbook.get("schemaVersion") == 1
            and compiled_runbook.get("projection") == "on-demand-reviewed-runbook"
            and compiled_runbook.get("persisted") is False
            and compiled_runbook.get("authorityIncreasedThroughProjection") is False
            and compiled_runbook.get("requiresExplicitSkillExport") is True
            and compiled_runbook.get("liveSourceChecked") is False
            and runbook_id.startswith("rbk_")
            and compiled_runbook.get("runbookId") == rebuilt_runbook.get("runbookId")
            and compiled_runbook.get("sourceFingerprint")
            == rebuilt_runbook.get("sourceFingerprint")
            and compiled_runbook.get("runbookId") == reordered_runbook.get("runbookId")
            and compiled_runbook.get("sourceFingerprint")
            == reordered_runbook.get("sourceFingerprint")
            and compiled_runbook.get("sourceLearningIds")
            == reordered_runbook.get("sourceLearningIds")
            and compiled_runbook.get("markdown") == reordered_runbook.get("markdown")
            and all(marker in serialized_runbook for marker in markers)
            and (not hidden_marker or hidden_marker not in serialized_runbook)
        )

        export_args = [
            "runbook",
            "export-skill",
            str(project),
            "--title",
            title,
        ]
        for learning_id in learning_ids:
            export_args.extend(["--learning", learning_id])
        export_args.extend(
            [
                "--expected-runbook",
                runbook_id,
                "--host",
                str(runbook_expectation.get("host", "codex")),
                "--egress-target",
                "cloud",
                "--json",
            ]
        )
        exported_skill = cli_json(export_args)
        if not isinstance(exported_skill, dict):
            raise RuntimeError("runbook Skill export returned a non-object payload")
        skill_content = str(exported_skill.get("content", ""))
        export_ok = (
            exported_skill.get("runbookId") == runbook_id
            and exported_skill.get("persisted") is False
            and exported_skill.get("installed") is False
            and exported_skill.get("explicitUserActionRequired") is True
            and exported_skill.get("liveSourceChecked") is False
            and all(marker in skill_content for marker in markers)
            and (not hidden_marker or hidden_marker not in skill_content)
        )

        reordered_export_args = [
            "runbook",
            "export-skill",
            str(project),
            "--title",
            title,
        ]
        for learning_id in reversed(learning_ids):
            reordered_export_args.extend(["--learning", learning_id])
        reordered_export_args.extend(
            [
                "--expected-runbook",
                runbook_id,
                "--host",
                str(runbook_expectation.get("host", "codex")),
                "--egress-target",
                "cloud",
                "--json",
            ]
        )
        reordered_export = cli_json(reordered_export_args)
        export_ok = (
            export_ok
            and isinstance(reordered_export, dict)
            and reordered_export.get("runbookId") == runbook_id
            and reordered_export.get("sourceFingerprint")
            == compiled_runbook.get("sourceFingerprint")
            and reordered_export.get("content") == exported_skill.get("content")
        )

        stale_id = "rbk_" + ("0" * 64)
        stale_blocked = False
        try:
            stale_args = list(export_args)
            stale_args[stale_args.index(runbook_id)] = stale_id
            run(stale_args)
        except RuntimeError as error:
            stale_blocked = "reviewed runbook changed" in str(error)

        run(["egress", "project", "local-model-only", str(project), "--json"])
        cloud_blocked = False
        try:
            run(export_args)
        except RuntimeError as error:
            cloud_blocked = (
                "egress policy 'local-model-only' does not allow target 'cloud'" in str(error)
            )
        finally:
            run(["egress", "project", "agent-ok", str(project), "--json"])

        config_root = Path(EVAL_ENV["XDG_CONFIG_HOME"])
        installed_skill = any(
            path.name == "SKILL.md"
            for root in (project, vault, config_root)
            for path in root.rglob("SKILL.md")
        )
        path_leakage = privacy_violation_rate(
            [str(project), str(vault), str(config_root)],
            [compiled_runbook, reordered_runbook, exported_skill, reordered_export],
        )
        scores["reviewed_runbook"] = (
            runbook_ok and export_ok and stale_blocked and cloud_blocked and not installed_skill
        )
        downstream_required = [
            str(value)
            for value in runbook_expectation.get(
                "downstream_required",
                markers,
            )
        ]
        downstream_forbidden = [
            str(value)
            for value in runbook_expectation.get(
                "downstream_forbidden",
                [hidden_marker] if hidden_marker else [],
            )
        ]
        record_downstream_task_contract(
            scores,
            failures,
            text_contract_success(
                skill_content,
                downstream_required,
                downstream_forbidden,
            ),
            "reviewed Runbook Skill content did not satisfy the independent downstream reusable-procedure contract",
        )
        scores["privacy_violation_rate"] = path_leakage
        evidence_text.extend(
            [compiled_runbook, reordered_runbook, exported_skill, reordered_export]
        )
        if not scores["reviewed_runbook"]:
            failures.append(
                "reviewed runbook/Skill export did not preserve source binding, explicit export, egress, or non-installation semantics"
            )
        if path_leakage != 0.0:
            failures.append("reviewed runbook or Skill export leaked a local path")

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
                "ley_search_memory",
                {
                    "query": controls_query,
                    "revisionCompatibility": "divergent",
                    "maxResults": 12,
                    "maxTokens": 4_000,
                },
            )
            current_lineage_search = mcp_call(
                project,
                "ley_search_memory",
                {
                    "query": controls_query,
                    "revisionCompatibility": "current-lineage",
                    "maxResults": 12,
                    "maxTokens": 4_000,
                },
            )
            divergent_session = mcp_call(
                project,
                "ley_session_get",
                {
                    "sessionId": session_id,
                    "maxCheckpoints": 5,
                    "maxCharacters": 8_000,
                },
            )
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
                "ley_search_memory",
                {
                    "query": controls_query,
                    "revisionCompatibility": "merged",
                    "maxResults": 12,
                    "maxTokens": 4_000,
                },
            )
            merged_session = mcp_call(
                project,
                "ley_session_get",
                {
                    "sessionId": session_id,
                    "maxCheckpoints": 5,
                    "maxCharacters": 8_000,
                },
            )
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
            [str(project), str(vault)], [divergent, merged, *branch_controls_evidence]
        )
        evidence_text.extend([divergent, merged, *branch_controls_evidence])
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
        expected_criterion = str(
            specification_expectation.get("acceptance_criterion", "")
        )
        acceptance_projection = relevant_specification.get("acceptanceCriteria", {})
        acceptance_rows = (
            acceptance_projection.get("criteria", [])
            if isinstance(acceptance_projection, dict)
            else []
        )
        acceptance_row = (
            acceptance_rows[0]
            if isinstance(acceptance_rows, list)
            and acceptance_rows
            and isinstance(acceptance_rows[0], dict)
            else {}
        )
        acceptance_criteria_ok = (
            not expected_criterion
            or (
                isinstance(acceptance_projection, dict)
                and acceptance_projection.get("state") == "available"
                and acceptance_projection.get("totalCriteria") == 1
                and acceptance_projection.get("returnedCriteria") == 1
                and acceptance_projection.get("omittedCriteria") == 0
                and acceptance_projection.get("sourceRevisionBound") is True
                and acceptance_projection.get("statusInterpreted") is False
                and acceptance_projection.get("persisted") is False
                and acceptance_projection.get("authority") == "human-intent"
                and acceptance_projection.get("sourceBoundary")
                == "derived-from-approved-specification"
                and acceptance_row.get("text") == expected_criterion
                and acceptance_row.get("startLine")
                == specification_expectation.get("acceptance_start_line")
                and acceptance_row.get("endLine")
                == specification_expectation.get("acceptance_end_line")
                and str(acceptance_row.get("criterionId", "")).startswith("acr_")
                and all(
                    field not in acceptance_row
                    for field in (
                        "checked",
                        "completed",
                        "verified",
                        "satisfied",
                        "remaining",
                        "status",
                    )
                )
                and int(relevant_specification.get("acceptanceCriteriaTokens", 0)) > 0
            )
        )
        expected_method = str(
            specification_expectation.get("verification_method", "")
        )
        verification_methods_projection = relevant_specification.get(
            "verificationMethods", {}
        )
        verification_method_rows = (
            verification_methods_projection.get("methods", [])
            if isinstance(verification_methods_projection, dict)
            else []
        )
        verification_method_row = (
            verification_method_rows[0]
            if isinstance(verification_method_rows, list)
            and verification_method_rows
            and isinstance(verification_method_rows[0], dict)
            else {}
        )
        verification_methods_ok = (
            not expected_method
            or (
                isinstance(verification_methods_projection, dict)
                and verification_methods_projection.get("state") == "available"
                and verification_methods_projection.get("totalMethods") == 1
                and verification_methods_projection.get("returnedMethods") == 1
                and verification_methods_projection.get("omittedMethods") == 0
                and verification_methods_projection.get("sourceRevisionBound") is True
                and verification_methods_projection.get("criterionBindingProven") is False
                and verification_methods_projection.get("observedResultBindingProven")
                is False
                and verification_methods_projection.get("statusInterpreted") is False
                and verification_methods_projection.get("persisted") is False
                and verification_methods_projection.get("authority") == "human-intent"
                and verification_methods_projection.get("sourceBoundary")
                == "derived-from-approved-specification"
                and verification_method_row.get("text") == expected_method
                and verification_method_row.get("startLine")
                == specification_expectation.get("verification_start_line")
                and verification_method_row.get("endLine")
                == specification_expectation.get("verification_end_line")
                and str(verification_method_row.get("methodId", "")).startswith("vmd_")
                and all(
                    field not in verification_method_row
                    for field in (
                        "criterionId",
                        "verificationRecordId",
                        "passed",
                        "verified",
                        "satisfied",
                        "status",
                    )
                )
                and int(
                    relevant_specification.get("verificationMethodsTokens", 0)
                )
                > 0
            )
        )
        expected_verification_summary = str(
            specification_expectation.get("acceptance_verification_summary", "")
        )
        acceptance_verification_ok = True
        if expected_verification_summary:
            acceptance_verification_ok = False
            criterion_id = str(acceptance_row.get("criterionId", ""))
            verification_method_id = str(
                verification_method_row.get("methodId", "")
            )
            if session_id and criterion_id:
                verification_session = mcp_call(
                    project,
                    "ley_session_get",
                    {
                        "sessionId": session_id,
                        "maxCheckpoints": 5,
                        "maxCharacters": 8_000,
                    },
                )
                verification_row = next(
                    (
                        verification
                        for checkpoint in verification_session.get("checkpoints", [])
                        if isinstance(checkpoint, dict)
                        for verification in checkpoint.get("verification", [])
                        if isinstance(verification, dict)
                        and verification.get("summary") == expected_verification_summary
                    ),
                    {},
                )
                verification_record_id = str(verification_row.get("id", ""))
                if verification_record_id:
                    acceptance_verification_review = mcp_call(
                        project,
                        "ley_acceptance_criterion_verification_review",
                        {
                            "specificationId": relevant_id,
                            "criterionId": criterion_id,
                            **(
                                {"verificationMethodId": verification_method_id}
                                if verification_method_id
                                else {}
                            ),
                            "sessionId": session_id,
                            "verificationRecordId": verification_record_id,
                        },
                    )
                    acceptance_verification_ok = (
                        acceptance_verification_review.get("specificationId")
                        == relevant_id
                        and acceptance_verification_review.get("criterion", {}).get(
                            "criterionId"
                        )
                        == criterion_id
                        and (
                            not verification_method_id
                            or acceptance_verification_review.get(
                                "verificationMethod", {}
                            ).get("methodId")
                            == verification_method_id
                        )
                        and acceptance_verification_review.get("sessionId") == session_id
                        and acceptance_verification_review.get("verification", {}).get(
                            "id"
                        )
                        == verification_record_id
                        and acceptance_verification_review.get("verification", {}).get(
                            "status"
                        )
                        == "passed"
                        and acceptance_verification_review.get("verification", {}).get(
                            "summary"
                        )
                        == expected_verification_summary
                        and str(
                            acceptance_verification_review.get("linkFingerprint", "")
                        ).startswith("sha256:")
                        and acceptance_verification_review.get(
                            "specificationSourceRevisionChecked"
                        )
                        is True
                        and acceptance_verification_review.get(
                            "verificationMethodChecked"
                        )
                        is bool(verification_method_id)
                        and acceptance_verification_review.get(
                            "verificationRecordChecked"
                        )
                        is True
                        and acceptance_verification_review.get(
                            "relationshipSuppliedByCaller"
                        )
                        is True
                        and acceptance_verification_review.get(
                            "verificationMethodRelationshipSuppliedByCaller"
                        )
                        is bool(verification_method_id)
                        and acceptance_verification_review.get(
                            "verificationMethodExecutionProven"
                        )
                        is False
                        and acceptance_verification_review.get(
                            "verificationMethodOutcomeProven"
                        )
                        is False
                        and acceptance_verification_review.get(
                            "verificationStatusInterpretedAsSatisfaction"
                        )
                        is False
                        and acceptance_verification_review.get(
                            "criterionSatisfactionProven"
                        )
                        is False
                        and acceptance_verification_review.get(
                            "semanticCoverageProven"
                        )
                        is False
                        and acceptance_verification_review.get(
                            "currentImplementationProven"
                        )
                        is False
                        and acceptance_verification_review.get("persisted") is False
                        and acceptance_verification_review.get(
                            "automaticWriteAllowed"
                        )
                        is False
                        and acceptance_verification_review.get("liveSourceChecked")
                        is False
                        and acceptance_verification_review.get("criterionAuthority")
                        == "human-intent"
                        and acceptance_verification_review.get(
                            "verificationSourceBoundary"
                        )
                        == "untrusted-historical-verification"
                        and acceptance_verification_review.get("relationshipBoundary")
                        == (
                            "caller-supplied-criterion-method-verification-review-link"
                            if verification_method_id
                            else "caller-supplied-criterion-verification-review-link"
                        )
                    )
                    evidence_text.extend(
                        [verification_session, acceptance_verification_review]
                    )
        specification_ok = (
            relevant_id in admitted_ids
            and unrelated_id not in admitted_ids
            and unrelated_omitted
            and historical_withheld
            and direct_evidence_preserved
            and acceptance_criteria_ok
            and verification_methods_ok
            and acceptance_verification_ok
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
        if not acceptance_verification_ok:
            failures.append(
                "acceptance-criterion Verification review did not preserve exact provenance/non-satisfaction semantics"
            )
        if not verification_methods_ok:
            failures.append(
                "Specification Verification-method projection did not preserve exact human-intent/non-binding semantics"
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
        verification_method_marker = str(
            egress_expectation.get("verification_method_marker", "")
        )
        query = str(egress_expectation.get("query", ""))
        if (
            not marker
            or not derived_marker
            or not verification_method_marker
            or not query
        ):
            raise RuntimeError(
                "egress fixture requires marker, derived_marker, verification_method_marker, and query"
            )

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
            and verification_method_marker
            not in json.dumps(cloud_direct, sort_keys=True)
            and verification_method_marker
            not in json.dumps(cloud_compiled, sort_keys=True)
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
            and verification_method_marker
            in json.dumps(local_direct, sort_keys=True)
            and verification_method_marker
            in json.dumps(local_compiled, sort_keys=True)
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
        ) and marker not in json.dumps(
            confirm_compiled, sort_keys=True
        ) and verification_method_marker not in json.dumps(
            confirm_direct, sort_keys=True
        ) and verification_method_marker not in json.dumps(
            confirm_compiled, sort_keys=True
        ) and derived_marker not in json.dumps(confirm_hook, sort_keys=True)

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
        ) and marker not in json.dumps(
            never_compiled, sort_keys=True
        ) and verification_method_marker not in json.dumps(
            never_direct, sort_keys=True
        ) and verification_method_marker not in json.dumps(
            never_compiled, sort_keys=True
        ) and derived_marker not in json.dumps(never_hook, sort_keys=True)

        disallowed_probe_groups = [
            [cloud_direct, cloud_compiled, cloud_hook],
            [confirm_direct, confirm_compiled, confirm_hook],
            [never_direct, never_compiled, never_hook],
        ]
        policy_violations = sum(
            marker in serialized(group)
            or derived_marker in serialized(group)
            or verification_method_marker in serialized(group)
            for group in disallowed_probe_groups
        )
        scores["privacy_violation_rate"] = policy_violations / len(
            disallowed_probe_groups
        )
        egress_ok = cloud_blocked and local_allowed and confirm_blocked and never_blocked
        scores["egress_policy"] = egress_ok
        downstream_required_local = [
            str(value)
            for value in egress_expectation.get("downstream_required_local", [])
        ]
        downstream_forbidden_blocked = [
            str(value)
            for value in egress_expectation.get(
                "downstream_forbidden_blocked", []
            )
        ]
        if downstream_required_local or downstream_forbidden_blocked:
            downstream_egress_ok = task_contract_success(
                local_compiled,
                downstream_required_local,
                [],
            ) and all(
                task_contract_success(
                    payload,
                    [],
                    downstream_forbidden_blocked,
                )
                for payload in (
                    cloud_compiled,
                    confirm_compiled,
                    never_compiled,
                )
            )
            record_downstream_task_contract(
                scores,
                failures,
                downstream_egress_ok,
                "egress-gated context did not satisfy the independent local-allowed/cloud-withheld task contract",
            )
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

    connector_expectation = scenario.get("expected_external_connector")
    if isinstance(connector_expectation, dict):
        source_url = str(connector_expectation.get("source_url", ""))
        historical_marker = str(connector_expectation.get("historical_marker", ""))
        direct_marker = str(connector_expectation.get("direct_marker", ""))
        query = str(connector_expectation.get("query", ""))
        if not source_url or not historical_marker or not direct_marker or not query:
            raise RuntimeError(
                "external connector fixture requires source_url, historical_marker, direct_marker, and query"
            )

        added = cli_json(["connector", "add", source_url, str(project), "--json"])
        if not isinstance(added, dict) or not isinstance(added.get("connector"), dict):
            raise RuntimeError("connector add returned no connector receipt")
        added_connector = added["connector"]
        connector_id = str(added_connector.get("connectorId", ""))
        source = added_connector.get("source")
        if not connector_id or not isinstance(source, dict):
            raise RuntimeError("connector add returned incomplete connector identity")

        tools = mcp_tools_list(project)
        tool_names = {
            str(tool.get("name", "")) for tool in tools if isinstance(tool, dict)
        }
        read_routes_only = (
            "ley_external_connectors_list" in tool_names
            and "ley_external_connector_get" in tool_names
            and "ley_external_connector_add" not in tool_names
            and "ley_external_connector_refresh" not in tool_names
            and "ley_external_connector_remove" not in tool_names
        )

        allowed_list = mcp_call(project, "ley_external_connectors_list", {})
        allowed_connectors = [
            item
            for item in allowed_list.get("connectors", [])
            if isinstance(item, dict)
        ]
        allowed_discovery = (
            source.get("canonicalUrl") == source_url
            and source.get("resourceKind") in {"issue", "pull-request", "document"}
            and added_connector.get("agentContextEnabled") is True
            and allowed_list.get("networkRequested") is False
            and allowed_list.get("sourceBoundary") == "untrusted-external-reference"
            and any(
                item.get("connectorId") == connector_id
                and isinstance(item.get("source"), dict)
                and item["source"].get("canonicalUrl") == source_url
                for item in allowed_connectors
            )
        )

        cli_json(
            [
                "egress",
                "connector",
                connector_id,
                "local-model-only",
                str(project),
                "--json",
            ]
        )
        blocked_list = mcp_call(project, "ley_external_connectors_list", {})
        blocked_serialized = json.dumps(blocked_list, sort_keys=True)
        blocked_exclusions = [
            item
            for item in blocked_list.get("exclusions", [])
            if isinstance(item, dict)
        ]
        blocked_metadata = (
            blocked_list.get("networkRequested") is False
            and not blocked_list.get("connectors")
            and any(
                item.get("connectorId") == connector_id
                and item.get("policy") == "local-model-only"
                and item.get("blockReason") == "local-model-only"
                for item in blocked_exclusions
            )
            and source_url not in blocked_serialized
        )

        blocked_get_error = ""
        try:
            mcp_call(
                project,
                "ley_external_connector_get",
                {"connectorId": connector_id},
            )
        except RuntimeError as error:
            blocked_get_error = str(error)
        blocked_get = (
            "local-model-only" in blocked_get_error
            and source_url not in blocked_get_error
            and historical_marker not in blocked_get_error
        )

        cloud_compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
        )
        cloud_coverage = cloud_compiled.get("egressCoverage")
        cloud_exclusions = [
            item
            for item in cloud_compiled.get("egressExclusions", [])
            if isinstance(item, dict)
        ]
        cloud_compiled_text = json.dumps(cloud_compiled, sort_keys=True)
        cloud_withheld = (
            isinstance(cloud_coverage, dict)
            and cloud_coverage.get("blockedExternalConnectors") == 1
            and cloud_coverage.get("historicalMemoryWithheld") is True
            and int(cloud_coverage.get("withheldDerivedResults", 0)) >= 1
            and any(
                item.get("scopeKind") == "external-connector"
                and item.get("scopeId") == connector_id
                and item.get("policyOrigin") == "external-connector"
                and item.get("policy") == "local-model-only"
                for item in cloud_exclusions
            )
            and historical_marker not in cloud_compiled_text
        )

        resume_error = ""
        try:
            mcp_call(
                project,
                "ley_project_resume",
                {"maxSessions": 3, "maxLearnings": 3, "maxCharacters": 8_000},
            )
        except RuntimeError as error:
            resume_error = str(error)
        historical_reader_blocked = (
            "historical Ley memory is withheld" in resume_error
            and historical_marker not in resume_error
            and source_url not in resume_error
        )

        direct = mcp_call(
            project,
            "ley_search_context",
            {"query": direct_marker, "maxResults": 4, "maxTokens": 1_000},
        )
        direct_preserved = direct_marker in json.dumps(direct, sort_keys=True)

        local_flags = ("--egress-target", "local")
        local_list = mcp_call(
            project,
            "ley_external_connectors_list",
            {},
            flags=local_flags,
        )
        local_compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 1_500},
            flags=local_flags,
        )
        local_allowed = (
            local_list.get("networkRequested") is False
            and source_url in json.dumps(local_list, sort_keys=True)
            and historical_marker in json.dumps(local_compiled, sort_keys=True)
            and isinstance(local_compiled.get("egressCoverage"), dict)
            and local_compiled["egressCoverage"].get("blockedExternalConnectors") == 0
        )

        removed = cli_json(
            ["connector", "remove", connector_id, str(project), "--json"]
        )
        retained = cli_json(["egress", "list", str(project), "--json"])
        retained_overrides = (
            retained.get("connectorOverrides", []) if isinstance(retained, dict) else []
        )
        retained_restriction = any(
            isinstance(item, dict)
            and item.get("scopeId") == connector_id
            and item.get("policy") == "local-model-only"
            for item in retained_overrides
        )
        readded = cli_json(["connector", "add", source_url, str(project), "--json"])
        readded_connector = readded.get("connector", {}) if isinstance(readded, dict) else {}
        stable_identity = (
            isinstance(removed, dict)
            and removed.get("connectorId") == connector_id
            and isinstance(readded_connector, dict)
            and readded_connector.get("connectorId") == connector_id
        )
        reblocked_list = mcp_call(project, "ley_external_connectors_list", {})
        reblocked_text = json.dumps(reblocked_list, sort_keys=True)
        retained_enforced = (
            retained_restriction
            and not reblocked_list.get("connectors")
            and source_url not in reblocked_text
            and any(
                isinstance(item, dict)
                and item.get("connectorId") == connector_id
                and item.get("policy") == "local-model-only"
                for item in reblocked_list.get("exclusions", [])
            )
        )

        agent_payloads = [
            allowed_list,
            blocked_list,
            cloud_compiled,
            direct,
            local_list,
            local_compiled,
            reblocked_list,
        ]
        local_path_leak = any(
            path in serialized(agent_payloads)
            for path in (str(project), str(vault))
        )
        policy_leak = (
            source_url in blocked_serialized
            or source_url in reblocked_text
            or historical_marker in cloud_compiled_text
            or historical_marker in resume_error
        )
        scores["privacy_violation_rate"] = 1.0 if local_path_leak or policy_leak else 0.0
        connector_ok = (
            read_routes_only
            and allowed_discovery
            and blocked_metadata
            and blocked_get
            and cloud_withheld
            and historical_reader_blocked
            and direct_preserved
            and local_allowed
            and stable_identity
            and retained_enforced
        )
        scores["external_connector"] = connector_ok
        downstream_required_local = [
            str(value)
            for value in connector_expectation.get(
                "downstream_required_local",
                [historical_marker],
            )
        ]
        downstream_forbidden_cloud = [
            str(value)
            for value in connector_expectation.get(
                "downstream_forbidden_cloud",
                [historical_marker],
            )
        ]
        record_downstream_task_contract(
            scores,
            failures,
            task_contract_success(
                local_compiled,
                downstream_required_local,
                [],
            )
            and task_contract_success(
                cloud_compiled,
                [],
                downstream_forbidden_cloud,
            ),
            "external connector did not satisfy the independent downstream local-allowed/cloud-withheld context contract",
        )
        evidence_text.extend(agent_payloads)
        if not connector_ok:
            failures.append(
                "external connector scope/egress contract failed deterministic authority, MCP, non-laundering, or remove/re-add checks"
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
        mount_index = int(mounted_expectation.get("mount_index", 0))
        unmounted_index = int(mounted_expectation.get("unmounted_index", 1))
        active_conflict_learning_id = ""
        active_conflict_setup_ok = True
        active_learning_definition = mounted_expectation.get("active_trusted_learning")
        if isinstance(active_learning_definition, dict):
            source_path = str(active_learning_definition.get("source_path", ""))
            if not source_path:
                raise RuntimeError(
                    "mounted-reference active trusted learning fixture requires source_path"
                )
            started = mcp_call(
                project,
                "ley_session_start",
                {
                    "requestId": request_id(
                        f"{scenario['id']}:mount-conflict:active:start"
                    ),
                    "name": "Active reference-conflict state",
                    "goal": "Preserve reviewed active-project state for mount conflict adjudication.",
                    "host": "codex",
                },
                WRITE_FLAGS,
            )
            active_session_id = str(started.get("sessionId", ""))
            mcp_call(
                project,
                "ley_session_checkpoint",
                {
                    "sessionId": active_session_id,
                    "requestId": request_id(
                        f"{scenario['id']}:mount-conflict:active:checkpoint"
                    ),
                    "expectedEventCount": 1,
                    "summary": "Captured current active-project cache guidance.",
                    "touchedArtifacts": [source_path],
                },
                WRITE_FLAGS,
            )
            active_session = mcp_call(
                project,
                "ley_session_get",
                {
                    "sessionId": active_session_id,
                    "maxCheckpoints": 5,
                    "maxCharacters": 8_000,
                },
            )
            active_checkpoints = [
                item
                for item in active_session.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            if not active_checkpoints:
                raise RuntimeError(
                    "mounted-reference active trusted learning fixture created no checkpoint"
                )
            proposal = mcp_call(
                project,
                "ley_learning_propose",
                {
                    "requestId": request_id(
                        f"{scenario['id']}:mount-conflict:active:learning"
                    ),
                    "kind": str(active_learning_definition.get("kind", "constraint")),
                    "title": str(active_learning_definition.get("title", "")),
                    "guidance": str(active_learning_definition.get("guidance", "")),
                    "confidencePercent": 95,
                    "provenance": "agent-authored",
                    "evidence": [
                        {
                            "sessionId": active_session_id,
                            "recordId": str(
                                active_checkpoints[-1].get("checkpointId", "")
                            ),
                            "note": "Current active-project state for mounted-reference conflict evaluation.",
                        }
                    ],
                },
                WRITE_FLAGS,
            )
            active_conflict_learning_id = str(proposal.get("learningId", ""))
            reviewed = cli_json(
                [
                    "learning",
                    "review",
                    active_conflict_learning_id,
                    str(project),
                    "--actor",
                    "user",
                    "--action",
                    "confirm",
                    "--note",
                    "Explicitly reviewed active-project state for mounted-reference conflict evaluation.",
                    "--request-id",
                    request_id(f"{scenario['id']}:mount-conflict:active:review"),
                    "--json",
                ]
            )
            reviewed_learning = (
                reviewed.get("learning", {}) if isinstance(reviewed, dict) else {}
            )
            active_conflict_setup_ok = (
                active_conflict_learning_id.startswith("lrn_")
                and reviewed_learning.get("eventCount") == 2
                and reviewed_learning.get("state") == "verified"
                and reviewed_learning.get("trustState") == "trusted"
            )

        mounted_conflict_decision_id = ""
        mounted_conflict_setup_ok = True
        mounted_decision_definition = mounted_expectation.get(
            "mounted_historical_decision"
        )
        if isinstance(mounted_decision_definition, dict):
            decision_mount_index = int(
                mounted_decision_definition.get("mount_index", mount_index)
            )
            decision_project, _ = mounted_projects[decision_mount_index]
            decision_session_id, _ = create_structured_session(
                decision_project,
                seed=f"{scenario['id']}:mount-conflict:reference",
                name="Historical mounted cache decision",
                goal="Preserve lower-precedence historical reference guidance.",
                summary="Mounted reference historically chose Redis cache.",
                decisions=[
                    {
                        "title": str(
                            mounted_decision_definition.get(
                                "title", "Redis cache startup state"
                            )
                        ),
                        "decision": str(
                            mounted_decision_definition.get(
                                "decision", "Use Redis cache for startup state."
                            )
                        ),
                        "rationale": "Historical reference-project choice.",
                    }
                ],
            )
            decision_session = mcp_call(
                decision_project,
                "ley_session_get",
                {
                    "sessionId": decision_session_id,
                    "maxCheckpoints": 5,
                    "maxCharacters": 8_000,
                },
            )
            decision_checkpoints = [
                item
                for item in decision_session.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            decision_rows = (
                decision_checkpoints[-1].get("decisions", [])
                if decision_checkpoints
                else []
            )
            if isinstance(decision_rows, list) and decision_rows:
                mounted_conflict_decision_id = str(
                    decision_rows[0].get("id", "")
                    if isinstance(decision_rows[0], dict)
                    else ""
                )
            mounted_conflict_setup_ok = mounted_conflict_decision_id.startswith("dec_")

        before = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 8, "maxTokens": 2_000},
        )
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
        mounted_exclusions = [
            item
            for item in compiled.get("mountedReferenceExclusions", [])
            if isinstance(item, dict)
        ]
        scopes = [
            item for item in compiled.get("mountedReferenceScopes", []) if isinstance(item, dict)
        ]
        active_items = [
            item for item in compiled.get("items", []) if isinstance(item, dict)
        ]
        mounted_marker = str(mounted_expectation.get("mounted_marker", ""))
        unmounted_marker = str(mounted_expectation.get("unmounted_marker", ""))
        active_marker = str(mounted_expectation.get("active_marker", ""))
        serialized_compiled = json.dumps(compiled, sort_keys=True)
        serialized_active_items = json.dumps(active_items, sort_keys=True)
        serialized_references = json.dumps(references, sort_keys=True)
        mounted_visible = any(
            item.get("mountId") == mount_id
            and item.get("authority") == "mounted-reference"
            and item.get("sourceBoundary") == "untrusted-mounted-project-memory"
            and mounted_marker.lower() in json.dumps(item).lower()
            for item in references
        )
        active_visible = (
            not active_marker
            or active_marker.lower() in serialized_active_items.lower()
        )
        active_conflict_visible = (
            not active_conflict_learning_id
            or any(
                item.get("learningId") == active_conflict_learning_id
                and item.get("trustedForReuse") is True
                for item in active_items
            )
        )
        mounted_history_conflict_ok = (
            not mounted_conflict_decision_id
            or (
                not any(
                    item.get("entityId") == mounted_conflict_decision_id
                    for item in references
                )
                and any(
                    item.get("mountId") == mount_id
                    and item.get("entityId") == mounted_conflict_decision_id
                    and item.get("reason") == "conflicting-memory"
                    and item.get("conflictingActiveProjectEntityIds")
                    == [active_conflict_learning_id]
                    for item in mounted_exclusions
                )
                and compiled.get("mountedReferenceCoverage", {}).get(
                    "activeProjectConflicts"
                )
                == 1
            )
        )
        precedence_separation = (
            (not active_marker or active_marker.lower() not in serialized_references.lower())
            and (
                not mounted_marker
                or mounted_marker.lower() not in serialized_active_items.lower()
            )
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

        transient_mount_paths: list[Path] = []
        identity_changed: dict[str, object] | None = None
        identity_changed_ok = True
        if bool(
            mounted_expectation.get(
                "simulate_source_identity_changed",
                False,
            )
        ):
            _, mounted_vault = mounted_projects[mount_index]
            moved_project = base_dir / f"mounted-project-{mount_index}-moved"
            parked_project = base_dir / f"mounted-project-{mount_index}-parked"
            mounted_project.rename(moved_project)
            transient_mount_paths.extend([moved_project, parked_project])
            run(
                [
                    "bind",
                    str(moved_project),
                    "--vault",
                    str(mounted_vault),
                    "--json",
                ]
            )
            moved_project.rename(parked_project)
            moved_project.mkdir()
            run(
                [
                    "init",
                    str(moved_project),
                    "--name",
                    "Replacement mounted source identity",
                    "--capture",
                    "structured",
                    "--json",
                ]
            )
            identity_changed = mcp_call(
                project,
                "ley_compile_context",
                {"task": query, "maxResults": 8, "maxTokens": 2_000},
            )
            identity_scopes = [
                item
                for item in identity_changed.get("mountedReferenceScopes", [])
                if isinstance(item, dict)
            ]
            identity_coverage = identity_changed.get(
                "mountedReferenceCoverage",
                {},
            )
            identity_text = json.dumps(identity_changed, sort_keys=True)
            identity_active_text = json.dumps(
                identity_changed.get("items", []),
                sort_keys=True,
            )
            identity_changed_ok = (
                any(
                    item.get("mountId") == mount_id
                    and item.get("state") == "source-identity-changed"
                    for item in identity_scopes
                )
                and not identity_changed.get("mountedReferences")
                and isinstance(identity_coverage, dict)
                and identity_coverage.get("authorizedMounts") == 1
                and identity_coverage.get("readyMounts") == 0
                and identity_coverage.get("unavailableMounts") == 1
                and identity_coverage.get("searchedMounts") == 0
                and (
                    not active_marker
                    or active_marker.lower() in identity_active_text.lower()
                )
                and (
                    not mounted_marker
                    or mounted_marker.lower() not in identity_text.lower()
                )
                and str(project) not in identity_text
                and str(vault) not in identity_text
                and str(mounted_project) not in identity_text
                and str(mounted_vault) not in identity_text
                and str(moved_project) not in identity_text
                and str(parked_project) not in identity_text
            )
            shutil.rmtree(moved_project)
            parked_project.rename(moved_project)

        project_unavailable: dict[str, object] | None = None
        project_unavailable_ok = True
        if bool(
            mounted_expectation.get(
                "simulate_source_project_unavailable",
                False,
            )
        ):
            unavailable_parked = (
                base_dir / f"mounted-project-{mount_index}-temporarily-unavailable"
            )
            transient_mount_paths.append(unavailable_parked)
            moved_project.rename(unavailable_parked)
            project_unavailable = mcp_call(
                project,
                "ley_compile_context",
                {"task": query, "maxResults": 8, "maxTokens": 2_000},
            )
            project_unavailable_scopes = [
                item
                for item in project_unavailable.get(
                    "mountedReferenceScopes",
                    [],
                )
                if isinstance(item, dict)
            ]
            project_unavailable_coverage = project_unavailable.get(
                "mountedReferenceCoverage",
                {},
            )
            project_unavailable_text = json.dumps(
                project_unavailable,
                sort_keys=True,
            )
            project_unavailable_active_text = json.dumps(
                project_unavailable.get("items", []),
                sort_keys=True,
            )
            project_unavailable_ok = (
                any(
                    item.get("mountId") == mount_id
                    and item.get("state") == "source-project-unavailable"
                    for item in project_unavailable_scopes
                )
                and not project_unavailable.get("mountedReferences")
                and isinstance(project_unavailable_coverage, dict)
                and project_unavailable_coverage.get("authorizedMounts") == 1
                and project_unavailable_coverage.get("readyMounts") == 0
                and project_unavailable_coverage.get("unavailableMounts") == 1
                and project_unavailable_coverage.get("searchedMounts") == 0
                and (
                    not active_marker
                    or active_marker.lower()
                    in project_unavailable_active_text.lower()
                )
                and (
                    not mounted_marker
                    or mounted_marker.lower()
                    not in project_unavailable_text.lower()
                )
                and str(project) not in project_unavailable_text
                and str(vault) not in project_unavailable_text
                and str(mounted_project) not in project_unavailable_text
                and str(mounted_vault) not in project_unavailable_text
                and str(moved_project) not in project_unavailable_text
                and str(unavailable_parked) not in project_unavailable_text
            )
            unavailable_parked.rename(moved_project)

        unavailable: dict[str, object] | None = None
        unavailable_ok = True
        if bool(mounted_expectation.get("simulate_source_vault_unavailable", False)):
            _, mounted_vault = mounted_projects[mount_index]
            shutil.rmtree(mounted_vault)
            unavailable = mcp_call(
                project,
                "ley_compile_context",
                {"task": query, "maxResults": 8, "maxTokens": 2_000},
            )
            unavailable_scopes = [
                item
                for item in unavailable.get("mountedReferenceScopes", [])
                if isinstance(item, dict)
            ]
            unavailable_coverage = unavailable.get("mountedReferenceCoverage", {})
            unavailable_text = json.dumps(unavailable, sort_keys=True)
            unavailable_active_text = json.dumps(
                unavailable.get("items", []), sort_keys=True
            )
            unavailable_ok = (
                any(
                    item.get("mountId") == mount_id
                    and item.get("state") == "source-vault-unavailable"
                    for item in unavailable_scopes
                )
                and not unavailable.get("mountedReferences")
                and isinstance(unavailable_coverage, dict)
                and unavailable_coverage.get("authorizedMounts") == 1
                and unavailable_coverage.get("readyMounts") == 0
                and unavailable_coverage.get("unavailableMounts") == 1
                and unavailable_coverage.get("searchedMounts") == 0
                and (
                    not active_marker
                    or active_marker.lower() in unavailable_active_text.lower()
                )
                and (
                    not mounted_marker
                    or mounted_marker.lower() not in unavailable_text.lower()
                )
                and str(project) not in unavailable_text
                and str(vault) not in unavailable_text
                and str(mounted_project) not in unavailable_text
                and str(mounted_vault) not in unavailable_text
            )

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
            and active_visible
            and active_conflict_setup_ok
            and mounted_conflict_setup_ok
            and active_conflict_visible
            and mounted_history_conflict_ok
            and precedence_separation
            and scope_visible
            and unrelated_hidden
            and no_paths
            and compiled.get("referencePrecedence") == "active-project-over-mounted-reference"
            and int(compiled.get("estimatedTokens", 0)) <= int(compiled.get("maxTokens", 0))
            and identity_changed_ok
            and project_unavailable_ok
            and unavailable_ok
            and after_clean
        )
        scores["mounted_reference"] = mounted_ok
        downstream_required = [
            str(value)
            for value in mounted_expectation.get("downstream_required", [])
        ]
        downstream_forbidden = [
            str(value)
            for value in mounted_expectation.get("downstream_forbidden", [])
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
                "mounted-reference context did not satisfy the independent downstream reference contract",
            )
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)]
            + [str(path) for pair in mounted_projects for path in pair]
            + [str(path) for path in transient_mount_paths],
            [compiled]
            + ([identity_changed] if identity_changed is not None else [])
            + (
                [project_unavailable]
                if project_unavailable is not None
                else []
            )
            + ([unavailable] if unavailable is not None else []),
        )
        evidence_text.extend(
            [before, compiled]
            + ([identity_changed] if identity_changed is not None else [])
            + (
                [project_unavailable]
                if project_unavailable is not None
                else []
            )
            + ([unavailable] if unavailable is not None else [])
            + [after]
        )
        if not mounted_ok:
            failures.append(
                "explicit Context Mount did not preserve authorization/isolation/budget/unmount semantics"
            )

    knowledge_scope_definitions = [
        item
        for item in scenario.get("knowledge_scope_projects", [])
        if isinstance(item, dict)
    ]
    knowledge_scope_expectation = scenario.get("expected_knowledge_scope_compiler")
    if isinstance(knowledge_scope_expectation, dict):
        scope_projects: list[tuple[Path, Path]] = []
        for index, definition in enumerate(knowledge_scope_definitions):
            scope_project = base_dir / f"knowledge-scope-project-{index}"
            scope_vault = base_dir / f"knowledge-scope-vault-{index}"
            scope_project.mkdir()
            scope_vault.mkdir()
            write_project_files(scope_project, definition.get("files", {}))
            init_project(scope_project, str(definition["name"]), scope_vault)
            scope_projects.append((scope_project, scope_vault))

        query = str(knowledge_scope_expectation.get("query", ""))
        historical_query = str(knowledge_scope_expectation.get("historical_query", ""))
        historical_marker = str(knowledge_scope_expectation.get("historical_marker", ""))
        scope_kind = str(knowledge_scope_expectation.get("kind", "team"))
        scope_name = str(knowledge_scope_expectation.get("name", "Shared team"))
        source_indices = [
            int(value) for value in knowledge_scope_expectation.get("source_indices", [])
        ]
        unrelated_index = int(knowledge_scope_expectation.get("unrelated_index", -1))
        restricted_index = int(knowledge_scope_expectation.get("restricted_index", -1))
        markers = [
            str(value) for value in knowledge_scope_expectation.get("markers", [])
        ]
        unrelated_marker = str(
            knowledge_scope_expectation.get("unrelated_marker", "")
        )
        restricted_marker = str(
            knowledge_scope_expectation.get("restricted_marker", "")
        )
        if (
            not query
            or not source_indices
            or any(index < 0 or index >= len(scope_projects) for index in source_indices)
            or unrelated_index < 0
            or unrelated_index >= len(scope_projects)
            or restricted_index < 0
            or restricted_index >= len(scope_projects)
        ):
            raise RuntimeError("knowledge scope fixture indices/query are invalid")

        active_conflict_learning_id = ""
        active_conflict_setup_ok = True
        active_learning_definition = knowledge_scope_expectation.get(
            "active_trusted_learning"
        )
        if isinstance(active_learning_definition, dict):
            source_path = str(active_learning_definition.get("source_path", ""))
            if not source_path:
                raise RuntimeError(
                    "knowledge-scope active trusted learning fixture requires source_path"
                )
            started = mcp_call(
                project,
                "ley_session_start",
                {
                    "requestId": request_id(
                        f"{scenario['id']}:scope-conflict:active:start"
                    ),
                    "name": "Active shared-scope conflict state",
                    "goal": "Preserve reviewed active-project state for shared-scope conflict adjudication.",
                    "host": "codex",
                },
                WRITE_FLAGS,
            )
            active_session_id = str(started.get("sessionId", ""))
            mcp_call(
                project,
                "ley_session_checkpoint",
                {
                    "sessionId": active_session_id,
                    "requestId": request_id(
                        f"{scenario['id']}:scope-conflict:active:checkpoint"
                    ),
                    "expectedEventCount": 1,
                    "summary": "Captured current active-project cache guidance.",
                    "touchedArtifacts": [source_path],
                },
                WRITE_FLAGS,
            )
            active_session = mcp_call(
                project,
                "ley_session_get",
                {
                    "sessionId": active_session_id,
                    "maxCheckpoints": 5,
                    "maxCharacters": 8_000,
                },
            )
            active_checkpoints = [
                item
                for item in active_session.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            if not active_checkpoints:
                raise RuntimeError(
                    "knowledge-scope active trusted learning fixture created no checkpoint"
                )
            proposal = mcp_call(
                project,
                "ley_learning_propose",
                {
                    "requestId": request_id(
                        f"{scenario['id']}:scope-conflict:active:learning"
                    ),
                    "kind": str(active_learning_definition.get("kind", "constraint")),
                    "title": str(active_learning_definition.get("title", "")),
                    "guidance": str(active_learning_definition.get("guidance", "")),
                    "confidencePercent": 95,
                    "provenance": "agent-authored",
                    "evidence": [
                        {
                            "sessionId": active_session_id,
                            "recordId": str(
                                active_checkpoints[-1].get("checkpointId", "")
                            ),
                            "note": "Current active-project state for shared-scope conflict evaluation.",
                        }
                    ],
                },
                WRITE_FLAGS,
            )
            active_conflict_learning_id = str(proposal.get("learningId", ""))
            reviewed = cli_json(
                [
                    "learning",
                    "review",
                    active_conflict_learning_id,
                    str(project),
                    "--actor",
                    "user",
                    "--action",
                    "confirm",
                    "--note",
                    "Explicitly reviewed active-project state for shared-scope conflict evaluation.",
                    "--request-id",
                    request_id(f"{scenario['id']}:scope-conflict:active:review"),
                    "--json",
                ]
            )
            reviewed_learning = (
                reviewed.get("learning", {}) if isinstance(reviewed, dict) else {}
            )
            active_conflict_setup_ok = (
                active_conflict_learning_id.startswith("lrn_")
                and reviewed_learning.get("eventCount") == 2
                and reviewed_learning.get("state") == "verified"
                and reviewed_learning.get("trustState") == "trusted"
            )

        shared_conflict_decision_id = ""
        shared_conflict_setup_ok = True
        shared_decision_definition = knowledge_scope_expectation.get(
            "shared_historical_decision"
        )
        if isinstance(shared_decision_definition, dict):
            decision_source_index = int(
                shared_decision_definition.get("source_index", source_indices[0])
            )
            if (
                decision_source_index < 0
                or decision_source_index >= len(scope_projects)
                or decision_source_index not in source_indices
            ):
                raise RuntimeError(
                    "knowledge-scope historical decision source must be attached to the scope"
                )
            decision_project, _ = scope_projects[decision_source_index]
            decision_session_id, _ = create_structured_session(
                decision_project,
                seed=f"{scenario['id']}:scope-conflict:reference",
                name="Historical shared cache decision",
                goal="Preserve lower-precedence historical shared guidance.",
                summary="Shared source historically chose Redis cache.",
                decisions=[
                    {
                        "title": str(
                            shared_decision_definition.get(
                                "title", "Redis cache startup state"
                            )
                        ),
                        "decision": str(
                            shared_decision_definition.get(
                                "decision", "Use Redis cache for startup state."
                            )
                        ),
                        "rationale": "Historical shared-project choice.",
                    }
                ],
            )
            decision_session = mcp_call(
                decision_project,
                "ley_session_get",
                {
                    "sessionId": decision_session_id,
                    "maxCheckpoints": 5,
                    "maxCharacters": 8_000,
                },
            )
            decision_checkpoints = [
                item
                for item in decision_session.get("checkpoints", [])
                if isinstance(item, dict)
            ]
            decision_rows = (
                decision_checkpoints[-1].get("decisions", [])
                if decision_checkpoints
                else []
            )
            if isinstance(decision_rows, list) and decision_rows:
                shared_conflict_decision_id = str(
                    decision_rows[0].get("id", "")
                    if isinstance(decision_rows[0], dict)
                    else ""
                )
            shared_conflict_setup_ok = shared_conflict_decision_id.startswith("dec_")

        before = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
        )
        create_args = [
            "scope",
            "create",
            scope_kind,
            scope_name,
            *[str(scope_projects[index][0]) for index in source_indices],
            "--json",
        ]
        created = cli_json(create_args)
        if not isinstance(created, dict) or not isinstance(created.get("scope"), dict):
            raise RuntimeError("scope create returned no scope receipt")
        scope = created["scope"]
        scope_id = str(scope.get("scopeId", ""))
        listed = cli_json(["scope", "list", "--json"])
        attached = cli_json(
            ["scope", "attach", scope_id, str(project), "--json"]
        )
        retry_attach = cli_json(
            ["scope", "attach", scope_id, str(project), "--json"]
        )
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
        )
        scope_inspection = mcp_call(
            project,
            "ley_context_pack_inspect",
            {
                "task": query,
                "maxResults": 12,
                "maxTokens": 3_000,
                "expectedContextPackId": str(compiled.get("contextPackId", "")),
            },
        )
        shared_scopes = [
            item
            for item in compiled.get("sharedKnowledgeScopes", [])
            if isinstance(item, dict)
        ]
        shared_references = [
            item
            for item in compiled.get("sharedKnowledgeReferences", [])
            if isinstance(item, dict)
        ]
        shared_exclusions = [
            item
            for item in compiled.get("sharedKnowledgeExclusions", [])
            if isinstance(item, dict)
        ]
        active_items = [
            item for item in compiled.get("items", []) if isinstance(item, dict)
        ]
        compiled_text = json.dumps(compiled, sort_keys=True)
        expected_source_ids = {
            str(item.get("sourceProjectId", ""))
            for item in scope.get("sources", [])
            if isinstance(item, dict)
        }
        listed_text = json.dumps(listed, sort_keys=True)
        scope_inspection_text = json.dumps(scope_inspection, sort_keys=True)
        source_visible = all(
            any(
                reference.get("scopeId") == scope_id
                and reference.get("authority") == "shared-knowledge-reference"
                and reference.get("sourceBoundary")
                == "untrusted-shared-project-memory"
                and marker.lower() in json.dumps(reference).lower()
                for reference in shared_references
            )
            for marker in markers
        )
        scope_visible = any(
            item.get("scopeId") == scope_id
            and item.get("kind") == scope_kind
            and item.get("name") == scope_name
            for item in shared_scopes
        )
        active_conflict_visible = (
            not active_conflict_learning_id
            or any(
                item.get("learningId") == active_conflict_learning_id
                and item.get("trustedForReuse") is True
                for item in active_items
            )
        )
        shared_history_conflict_ok = (
            not shared_conflict_decision_id
            or (
                not any(
                    item.get("entityId") == shared_conflict_decision_id
                    for item in shared_references
                )
                and any(
                    item.get("scopeId") == scope_id
                    and item.get("entityId") == shared_conflict_decision_id
                    and item.get("reason") == "conflicting-memory"
                    and item.get("conflictingActiveProjectEntityIds")
                    == [active_conflict_learning_id]
                    for item in shared_exclusions
                )
                and compiled.get("sharedKnowledgeCoverage", {}).get(
                    "activeProjectConflicts"
                )
                == 1
            )
        )
        unrelated_hidden = (
            not unrelated_marker
            or unrelated_marker.lower() not in compiled_text.lower()
        )
        no_paths = all(
            str(path) not in compiled_text and str(path) not in listed_text
            for pair in scope_projects
            for path in pair
        ) and str(project) not in compiled_text and str(vault) not in compiled_text
        authority_ok = (
            created.get("created") is True
            and scope.get("permission") == "read-only"
            and len(expected_source_ids) == len(source_indices)
            and isinstance(attached, dict)
            and attached.get("created") is True
            and isinstance(retry_attach, dict)
            and retry_attach.get("created") is False
            and not before.get("sharedKnowledgeScopes")
            and not before.get("sharedKnowledgeReferences")
            and scope_visible
            and source_visible
            and active_conflict_setup_ok
            and shared_conflict_setup_ok
            and active_conflict_visible
            and shared_history_conflict_ok
            and unrelated_hidden
            and no_paths
            and compiled.get("sharedKnowledgePrecedence")
            == "explicit-mount-over-shared-knowledge"
            and scope_inspection.get("schemaVersion") == 3
            and scope_inspection.get("matchesExpectedContextPack") is True
            and scope_inspection.get("sharedKnowledgePrecedence")
            == "explicit-mount-over-shared-knowledge"
            and any(
                isinstance(item, dict)
                and item.get("source") == "shared-knowledge-reference"
                and item.get("scopeId") == scope_id
                and item.get("sourceProjectId") in expected_source_ids
                for item in scope_inspection.get("includedRecords", [])
            )
            and all(marker not in scope_inspection_text for marker in markers)
            and int(compiled.get("estimatedTokens", 0))
            <= int(compiled.get("maxTokens", 0))
        )

        restricted_project, _ = scope_projects[restricted_index]
        run(
            [
                "egress",
                "project",
                "local-model-only",
                str(restricted_project),
                "--json",
            ]
        )
        cloud_restricted = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
        )
        cloud_restricted_text = json.dumps(cloud_restricted, sort_keys=True)
        restricted_project_id = ""
        for item in scope.get("sources", []):
            if (
                isinstance(item, dict)
                and item.get("sourceProjectName")
                == knowledge_scope_definitions[restricted_index].get("name")
            ):
                restricted_project_id = str(item.get("sourceProjectId", ""))
                break
        cloud_egress_ok = (
            bool(restricted_project_id)
            and restricted_marker not in cloud_restricted_text
            and any(
                isinstance(item, dict)
                and item.get("scopeKind") == "project"
                and item.get("scopeId") == restricted_project_id
                and item.get("policyOrigin") == "source-project"
                and item.get("policy") == "local-model-only"
                for item in cloud_restricted.get("egressExclusions", [])
            )
        )
        local_flags = ("--egress-target", "local")
        local_restricted = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
            flags=local_flags,
        )
        local_egress_ok = restricted_marker in json.dumps(
            local_restricted, sort_keys=True
        )

        detached = cli_json(
            ["scope", "detach", scope_id, str(project), "--json"]
        )
        after = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
        )
        after_clean = (
            not after.get("sharedKnowledgeScopes")
            and not after.get("sharedKnowledgeReferences")
        )
        cloud_history = mcp_call(
            project,
            "ley_compile_context",
            {"task": historical_query, "maxResults": 8, "maxTokens": 1_500},
        )
        cloud_history_text = json.dumps(cloud_history, sort_keys=True)
        cloud_history_coverage = cloud_history.get("egressCoverage", {})
        historical_withheld = (
            historical_marker not in cloud_history_text
            and isinstance(cloud_history_coverage, dict)
            and cloud_history_coverage.get("historicalMemoryWithheld") is True
            and int(cloud_history_coverage.get("blockedHistoricalSources", 0)) >= 1
            and int(cloud_history_coverage.get("withheldDerivedResults", 0)) >= 1
        )
        historical_reader_error = ""
        try:
            mcp_call(
                project,
                "ley_project_resume",
                {"maxSessions": 3, "maxLearnings": 3, "maxCharacters": 8_000},
            )
        except RuntimeError as error:
            historical_reader_error = str(error)
        historical_reader_blocked = (
            "historical Ley memory is withheld" in historical_reader_error
            and historical_marker not in historical_reader_error
        )
        local_history = mcp_call(
            project,
            "ley_compile_context",
            {"task": historical_query, "maxResults": 8, "maxTokens": 1_500},
            flags=local_flags,
        )
        local_history_ok = historical_marker in json.dumps(
            local_history.get("items", []), sort_keys=True
        )

        knowledge_scope_ok = (
            authority_ok
            and cloud_egress_ok
            and local_egress_ok
            and isinstance(detached, dict)
            and detached.get("scopeId") == scope_id
            and after_clean
            and historical_withheld
            and historical_reader_blocked
            and local_history_ok
        )
        scores["knowledge_scope"] = knowledge_scope_ok
        downstream_required = [
            str(value)
            for value in knowledge_scope_expectation.get(
                "downstream_required",
                markers,
            )
        ]
        downstream_forbidden = [
            str(value)
            for value in knowledge_scope_expectation.get(
                "downstream_forbidden",
                [unrelated_marker] if unrelated_marker else [],
            )
        ]
        record_downstream_task_contract(
            scores,
            failures,
            task_contract_success(
                compiled,
                downstream_required,
                downstream_forbidden,
            ),
            "team/organization Knowledge Scope did not satisfy the independent downstream shared-context contract",
        )
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)]
            + [str(path) for pair in scope_projects for path in pair]
            + ([unrelated_marker] if unrelated_marker else []),
            [before, compiled, cloud_restricted, after, cloud_history],
        )
        evidence_text.extend(
            [
                before,
                created,
                listed,
                attached,
                compiled,
                scope_inspection,
                cloud_restricted,
                local_restricted,
                detached,
                after,
                cloud_history,
                local_history,
            ]
        )
        if not knowledge_scope_ok:
            failures.append(
                "team/organization knowledge scope failed explicit authority, isolation, egress, detach, or historical non-laundering checks"
            )

    policy_bundle_definitions = [
        item
        for item in scenario.get("policy_bundle_projects", [])
        if isinstance(item, dict)
    ]
    policy_bundle_expectation = scenario.get("expected_policy_bundle_compiler")
    if isinstance(policy_bundle_expectation, dict):
        policy_projects: list[tuple[Path, Path]] = []
        for index, definition in enumerate(policy_bundle_definitions):
            source_project = base_dir / f"policy-bundle-project-{index}"
            source_vault = base_dir / f"policy-bundle-vault-{index}"
            source_project.mkdir()
            source_vault.mkdir()
            write_project_files(source_project, definition.get("files", {}))
            init_project(source_project, str(definition["name"]), source_vault)
            source_specifications = [
                item
                for item in definition.get("specifications", [])
                if isinstance(item, dict)
            ]
            install_specification_approvals(
                source_project, source_vault, source_specifications
            )
            policy_projects.append((source_project, source_vault))

        query = str(policy_bundle_expectation.get("query", ""))
        historical_query = str(
            policy_bundle_expectation.get("historical_query", "")
        )
        historical_marker = str(
            policy_bundle_expectation.get("historical_marker", "")
        )
        bundle_kind = str(policy_bundle_expectation.get("kind", "team"))
        bundle_name = str(
            policy_bundle_expectation.get("name", "Shared policy bundle")
        )
        source_indices = [
            int(value)
            for value in policy_bundle_expectation.get("source_indices", [])
        ]
        unrelated_index = int(
            policy_bundle_expectation.get("unrelated_index", -1)
        )
        conflicting_index = int(
            policy_bundle_expectation.get("conflicting_index", -1)
        )
        restricted_index = int(
            policy_bundle_expectation.get("restricted_index", -1)
        )
        active_specification_index = int(
            policy_bundle_expectation.get("active_specification_index", 0)
        )
        allowed_marker = str(
            policy_bundle_expectation.get("allowed_marker", "")
        )
        unrelated_marker = str(
            policy_bundle_expectation.get("unrelated_marker", "")
        )
        conflict_private_marker = str(
            policy_bundle_expectation.get("conflict_private_marker", "")
        )
        if (
            not query
            or not historical_query
            or not historical_marker
            or not allowed_marker
            or not source_indices
            or any(index < 0 or index >= len(policy_projects) for index in source_indices)
            or unrelated_index < 0
            or unrelated_index >= len(policy_projects)
            or conflicting_index not in source_indices
            or restricted_index not in source_indices
            or active_specification_index < 0
            or active_specification_index >= len(specification_definitions)
        ):
            raise RuntimeError("policy bundle fixture indices/query are invalid")

        source_specification_ids: dict[int, str] = {}
        source_specification_paths: dict[int, str] = {}
        source_specification_sources: dict[int, str] = {}
        for index in source_indices:
            definitions = [
                item
                for item in policy_bundle_definitions[index].get("specifications", [])
                if isinstance(item, dict)
            ]
            if len(definitions) != 1:
                raise RuntimeError(
                    "policy bundle fixture requires exactly one Specification per selected source"
                )
            resolved = definitions[0].get("resolved_specification_id")
            if not isinstance(resolved, str):
                raise RuntimeError(
                    "policy bundle source Specification has no stable ID"
                )
            source_specification_ids[index] = resolved
            source_specification_paths[index] = str(definitions[0]["path"])
            source_specification_sources[index] = str(definitions[0]["source"])

        active_specification_id = specification_definitions[
            active_specification_index
        ].get("resolved_specification_id")
        if not isinstance(active_specification_id, str):
            raise RuntimeError("active policy precedence Specification has no stable ID")

        before = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
        )
        create_scope = cli_json(
            [
                "scope",
                "create",
                bundle_kind,
                f"{bundle_name} scope",
                *[str(policy_projects[index][0]) for index in source_indices],
                "--json",
            ]
        )
        if not isinstance(create_scope, dict) or not isinstance(
            create_scope.get("scope"), dict
        ):
            raise RuntimeError("policy bundle parent scope returned no receipt")
        scope_id = str(create_scope["scope"].get("scopeId", ""))
        scope_attached = cli_json(
            ["scope", "attach", scope_id, str(project), "--json"]
        )
        scope_only = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
        )

        create_bundle_args = [
            "policy-bundle",
            "create",
            scope_id,
            bundle_name,
        ]
        for index in source_indices:
            create_bundle_args.extend(
                [
                    "--source",
                    str(policy_projects[index][0]),
                    source_specification_ids[index],
                ]
            )
        create_bundle_args.append("--json")
        created_bundle = cli_json(create_bundle_args)
        if not isinstance(created_bundle, dict) or not isinstance(
            created_bundle.get("bundle"), dict
        ):
            raise RuntimeError("policy bundle create returned no bundle receipt")
        bundle = created_bundle["bundle"]
        bundle_id = str(bundle.get("bundleId", ""))
        listed_bundles = cli_json(["policy-bundle", "list", "--json"])
        attached_bundle = cli_json(
            [
                "policy-bundle",
                "attach",
                bundle_id,
                str(project),
                "--json",
            ]
        )
        retry_bundle_attach = cli_json(
            [
                "policy-bundle",
                "attach",
                bundle_id,
                str(project),
                "--json",
            ]
        )
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
        )
        inspection = mcp_call(
            project,
            "ley_context_pack_inspect",
            {
                "task": query,
                "maxResults": 12,
                "maxTokens": 3_000,
                "expectedContextPackId": str(compiled.get("contextPackId", "")),
            },
        )
        compiled_text = json.dumps(compiled, sort_keys=True)
        inspection_text = json.dumps(inspection, sort_keys=True)
        listed_text = json.dumps(listed_bundles, sort_keys=True)
        policies = [
            item
            for item in compiled.get("policyBundlePolicies", [])
            if isinstance(item, dict)
        ]
        bundle_exclusions = [
            item
            for item in compiled.get("policyBundleExclusions", [])
            if isinstance(item, dict)
        ]
        memory_exclusions = [
            item for item in compiled.get("exclusions", []) if isinstance(item, dict)
        ]
        conflict_specification_id = source_specification_ids[conflicting_index]
        restricted_specification_id = source_specification_ids[restricted_index]
        admitted_active_specifications = {
            str(item.get("specificationId", ""))
            for item in compiled.get("specifications", [])
            if isinstance(item, dict)
        }
        policy_visible = any(
            item.get("bundleId") == bundle_id
            and item.get("specificationId") == restricted_specification_id
            and item.get("authority") == "human-intent"
            and item.get("sourceBoundary")
            == "user-approved-policy-bundle-specification"
            and allowed_marker in json.dumps(item)
            for item in policies
        )
        active_precedence = any(
            item.get("bundleId") == bundle_id
            and item.get("specificationId") == conflict_specification_id
            and item.get("reason") == "contradicts-active-specification"
            and active_specification_id
            in item.get("conflictingSpecificationIds", [])
            for item in bundle_exclusions
        )
        historical_conflict = any(
            item.get("reason") == "contradicts-human-intent"
            and active_specification_id in item.get("specificationIds", [])
            for item in memory_exclusions
        )
        bundle_context_visible = any(
            isinstance(item, dict)
            and item.get("bundleId") == bundle_id
            and item.get("scopeId") == scope_id
            and item.get("name") == bundle_name
            for item in compiled.get("policyBundles", [])
        )
        unrelated_hidden = (
            not unrelated_marker
            or unrelated_marker.lower() not in compiled_text.lower()
        )
        no_paths = (
            all(
                str(path) not in compiled_text
                and str(path) not in inspection_text
                and str(path) not in listed_text
                for pair in policy_projects
                for path in pair
            )
            and str(project) not in compiled_text
            and str(vault) not in compiled_text
        )
        inspector_metadata = any(
            isinstance(item, dict)
            and item.get("source") == "policy-bundle-specification"
            and item.get("bundleId") == bundle_id
            and item.get("scopeId") == scope_id
            and item.get("specificationId") == restricted_specification_id
            for item in inspection.get("includedRecords", [])
        )
        inspector_body_hidden = (
            allowed_marker not in inspection_text
            and (
                not conflict_private_marker
                or conflict_private_marker not in inspection_text
            )
        )
        authority_ok = (
            not before.get("policyBundles")
            and not before.get("policyBundlePolicies")
            and isinstance(scope_attached, dict)
            and scope_attached.get("created") is True
            and not scope_only.get("policyBundles")
            and not scope_only.get("policyBundlePolicies")
            and created_bundle.get("created") is True
            and isinstance(attached_bundle, dict)
            and attached_bundle.get("created") is True
            and isinstance(retry_bundle_attach, dict)
            and retry_bundle_attach.get("created") is False
            and bundle_context_visible
            and policy_visible
            and active_specification_id in admitted_active_specifications
            and active_precedence
            and historical_conflict
            and unrelated_hidden
            and no_paths
            and compiled.get("policyBundlePrecedence")
            == "active-project-specification-over-policy-bundle"
            and compiled.get("authorityPrecedence")
            == "human-intent-over-historical-memory"
            and inspection.get("schemaVersion") == 3
            and inspection.get("matchesExpectedContextPack") is True
            and inspection.get("policyBundlePrecedence")
            == "active-project-specification-over-policy-bundle"
            and inspector_metadata
            and inspector_body_hidden
            and int(compiled.get("estimatedTokens", 0))
            <= int(compiled.get("maxTokens", 0))
        )

        restricted_project, restricted_vault = policy_projects[restricted_index]
        run(
            [
                "egress",
                "specification",
                restricted_specification_id,
                "local-model-only",
                str(restricted_project),
                "--json",
            ]
        )
        restricted_relative_path = source_specification_paths[restricted_index]
        restricted_source = source_specification_sources[restricted_index]
        restricted_path = restricted_vault / restricted_relative_path
        restricted_path.unlink()
        cloud_restricted = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
        )
        cloud_restricted_text = json.dumps(cloud_restricted, sort_keys=True)
        cloud_egress_ok = (
            allowed_marker not in cloud_restricted_text
            and any(
                isinstance(item, dict)
                and item.get("scopeKind") == "specification"
                and item.get("scopeId") == restricted_specification_id
                and item.get("policyOrigin")
                == "policy-bundle-source-specification"
                and item.get("policy") == "local-model-only"
                for item in cloud_restricted.get("egressExclusions", [])
            )
            and any(
                isinstance(item, dict)
                and item.get("bundleId") == bundle_id
                and item.get("specificationId") == restricted_specification_id
                and item.get("reason") == "egress-blocked-specification"
                for item in cloud_restricted.get("policyBundleExclusions", [])
            )
            and int(
                cloud_restricted.get("egressCoverage", {}).get(
                    "blockedPolicyBundleSources", 0
                )
            )
            >= 1
        )
        restricted_path.parent.mkdir(parents=True, exist_ok=True)
        restricted_path.write_text(restricted_source, encoding="utf-8")
        local_flags = ("--egress-target", "local")
        local_restricted = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
            flags=local_flags,
        )
        local_egress_ok = allowed_marker in json.dumps(
            local_restricted.get("policyBundlePolicies", []), sort_keys=True
        )

        detached_bundle = cli_json(
            [
                "policy-bundle",
                "detach",
                bundle_id,
                str(project),
                "--json",
            ]
        )
        after = mcp_call(
            project,
            "ley_compile_context",
            {"task": query, "maxResults": 12, "maxTokens": 3_000},
        )
        after_clean = (
            not after.get("policyBundles")
            and not after.get("policyBundlePolicies")
        )
        cloud_history = mcp_call(
            project,
            "ley_compile_context",
            {"task": historical_query, "maxResults": 8, "maxTokens": 1_500},
        )
        cloud_history_text = json.dumps(cloud_history, sort_keys=True)
        cloud_history_coverage = cloud_history.get("egressCoverage", {})
        historical_withheld = (
            historical_marker not in json.dumps(
                cloud_history.get("items", []), sort_keys=True
            )
            and isinstance(cloud_history_coverage, dict)
            and cloud_history_coverage.get("historicalMemoryWithheld") is True
            and int(cloud_history_coverage.get("blockedPolicyBundleSources", 0)) >= 1
            and int(cloud_history_coverage.get("withheldDerivedResults", 0)) >= 1
            and any(
                isinstance(item, dict)
                and item.get("scopeId") == restricted_specification_id
                and item.get("policyOrigin")
                == "policy-bundle-source-specification"
                for item in cloud_history.get("egressExclusions", [])
            )
        )
        historical_reader_error = ""
        try:
            mcp_call(
                project,
                "ley_project_resume",
                {"maxSessions": 3, "maxLearnings": 3, "maxCharacters": 8_000},
            )
        except RuntimeError as error:
            historical_reader_error = str(error)
        historical_reader_blocked = (
            "historical Ley memory is withheld" in historical_reader_error
            and historical_marker not in historical_reader_error
        )
        local_history = mcp_call(
            project,
            "ley_compile_context",
            {"task": historical_query, "maxResults": 8, "maxTokens": 1_500},
            flags=local_flags,
        )
        local_history_ok = historical_marker in json.dumps(
            local_history.get("items", []), sort_keys=True
        )

        policy_bundle_ok = (
            authority_ok
            and cloud_egress_ok
            and local_egress_ok
            and isinstance(detached_bundle, dict)
            and detached_bundle.get("bundleId") == bundle_id
            and after_clean
            and historical_withheld
            and historical_reader_blocked
            and local_history_ok
        )
        scores["policy_bundle"] = policy_bundle_ok
        downstream_required = [
            str(value)
            for value in policy_bundle_expectation.get(
                "downstream_required",
                [allowed_marker],
            )
        ]
        downstream_forbidden = [
            str(value)
            for value in policy_bundle_expectation.get(
                "downstream_forbidden",
                [
                    value
                    for value in (unrelated_marker, conflict_private_marker)
                    if value
                ],
            )
        ]
        record_downstream_task_contract(
            scores,
            failures,
            task_contract_success(
                compiled,
                downstream_required,
                downstream_forbidden,
            ),
            "team/organization Policy Bundle did not satisfy the independent downstream human-intent contract",
        )
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)]
            + [str(path) for pair in policy_projects for path in pair]
            + ([unrelated_marker] if unrelated_marker else []),
            [
                before,
                scope_only,
                compiled,
                inspection,
                cloud_restricted,
                after,
                cloud_history,
            ],
        )
        evidence_text.extend(
            [
                before,
                create_scope,
                scope_attached,
                scope_only,
                created_bundle,
                listed_bundles,
                attached_bundle,
                compiled,
                inspection,
                cloud_restricted,
                local_restricted,
                detached_bundle,
                after,
                cloud_history,
                local_history,
            ]
        )
        if not policy_bundle_ok:
            failed_checks = [
                name
                for name, passed in [
                    ("authority", authority_ok),
                    ("cloud-egress-before-read", cloud_egress_ok),
                    ("local-egress", local_egress_ok),
                    ("detach", isinstance(detached_bundle, dict)
                     and detached_bundle.get("bundleId") == bundle_id),
                    ("after-clean", after_clean),
                    ("historical-withheld", historical_withheld),
                    ("historical-reader-blocked", historical_reader_blocked),
                    ("local-history", local_history_ok),
                ]
                if not passed
            ]
            failures.append(
                "team/organization policy bundle failed: " + ", ".join(failed_checks)
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
        memory_compilation = mcp_call(
            project,
            "ley_session_memory_compile",
            {
                "sessionId": imported_session_id,
                "maxResults": 20,
                "maxCharacters": 12000,
            },
        )
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
        compilation_evidence = (
            memory_compilation.get("evidence", [])
            if isinstance(memory_compilation, dict)
            else []
        )
        memory_compiler_ok = (
            isinstance(memory_compilation, dict)
            and memory_compilation.get("sessionId") == imported_session_id
            and memory_compilation.get("canCheckpoint") is False
            and memory_compilation.get("returnedEvidence") == 2
            and memory_compilation.get("liveSourceChecked") is False
            and len(compilation_evidence) == 2
            and all(
                isinstance(item, dict)
                and item.get("origin") == "import"
                and item.get("sourceBoundary")
                == "untrusted-imported-host-history"
                for item in compilation_evidence
            )
            and [
                int(item.get("sourceRecordedAtUnixMs", 0))
                for item in compilation_evidence
                if isinstance(item, dict)
            ]
            == expected_source_ms
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
            memory_compilation,
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
            and memory_compiler_ok
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
                    ("memory-compiler", memory_compiler_ok),
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

        inbox = mcp_call(
            project,
            "ley_consolidation_inbox",
            {"maxItems": 20, "maxSessions": 30},
        )
        rebuilt = mcp_call(
            project,
            "ley_consolidation_inbox",
            {"maxItems": 20, "maxSessions": 30},
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
            inbox.get("schemaVersion") == 1
            and inbox.get("persisted") is False
            and inbox.get("modelInvoked") is False
            and inbox.get("backgroundWorkStarted") is False
            and inbox.get("destructiveActionsTaken") is False
            and inbox.get("liveSourceChecked") is False
            and inbox.get("inboxFingerprint") == rebuilt.get("inboxFingerprint")
            and int(inbox.get("coverage", {}).get("excludedActiveSessions", 0)) >= 1
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

        proposed = mcp_call(
            project,
            "ley_learning_propose",
            {
                "requestId": request_id(f"{scenario['id']}:consolidation:learning"),
                "kind": "procedure",
                "title": "Review completed retained evidence",
                "guidance": "Inspect the retained evidence before reusing this completed workflow.",
                "confidencePercent": 60,
                "provenance": "inferred",
                "evidence": [
                    {
                        "sessionId": terminal_session_id,
                        "recordId": record_id,
                        "note": "Selected from the read-only local consolidation inbox.",
                    }
                    for record_id in proposal_ids
                ],
            },
            WRITE_FLAGS,
        )
        learning_id = str(proposed.get("learningId", ""))
        learning = mcp_call(
            project,
            "ley_learning_get",
            {"learningId": learning_id, "maxCharacters": 8_000},
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
            proposed.get("state") == "tentative"
            and proposed.get("trustState") == "review-required"
            and proposed.get("requiresUserReview") is True
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
        downstream_required = [
            str(value)
            for value in dossier_expectation.get("downstream_required", [])
        ]
        downstream_forbidden = [
            str(value)
            for value in dossier_expectation.get("downstream_forbidden", [])
        ]
        if downstream_required or downstream_forbidden:
            record_downstream_task_contract(
                scores,
                failures,
                text_contract_success(
                    dossier_text,
                    downstream_required,
                    downstream_forbidden,
                ),
                "Topic Dossier did not satisfy the independent downstream bounded-briefing contract",
            )
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
        specification_index = int(
            current_state_expectation.get("specification_index", -1)
        )
        specification_definition = (
            specification_definitions[specification_index]
            if 0 <= specification_index < len(specification_definitions)
            else None
        )
        specification_id = (
            str(specification_definition.get("resolved_specification_id", ""))
            if isinstance(specification_definition, dict)
            else ""
        )
        specification_path = str(
            current_state_expectation.get("specification_path", "")
        )
        specification_body_marker = str(
            current_state_expectation.get("specification_body_marker", "")
        )
        authoritative_specifications = [
            item
            for item in state.get("authoritativeSpecifications", [])
            if isinstance(item, dict)
        ]
        matching_specification = next(
            (
                item
                for item in authoritative_specifications
                if item.get("specificationId") == specification_id
            ),
            None,
        )
        specification_authority_ok = (
            not specification_id
            or (
                matching_specification is not None
                and matching_specification.get("relativePath") == specification_path
                and matching_specification.get("state") == "current"
                and matching_specification.get("exactApprovedRevisionAvailable") is True
                and matching_specification.get("sourceIncluded") is False
                and matching_specification.get("authority") == "human-intent"
                and matching_specification.get("sourceBoundary")
                == "user-approved-specification"
                and matching_specification.get("followupTool")
                == "ley_project_specifications"
                and state.get("specificationAuthorityPrecedence")
                == "human-intent-over-historical-memory"
                and (
                    not specification_body_marker
                    or specification_body_marker not in state_text
                )
            )
        )
        changed_specification_ok = True
        changed_state = None
        changed_specification_source = current_state_expectation.get(
            "changed_specification_source"
        )
        changed_specification_body_marker = str(
            current_state_expectation.get("changed_specification_body_marker", "")
        )
        if (
            specification_id
            and specification_path
            and isinstance(changed_specification_source, str)
            and changed_specification_source
        ):
            changed_path = vault / specification_path
            changed_path.write_text(changed_specification_source, encoding="utf-8")
            changed_state = mcp_call(project, "ley_project_state", arguments)
            changed_text = json.dumps(changed_state, sort_keys=True)
            changed_authoritative = [
                item
                for item in changed_state.get("authoritativeSpecifications", [])
                if isinstance(item, dict)
            ]
            changed_attention = [
                item
                for item in changed_state.get("specificationAttention", [])
                if isinstance(item, dict)
            ]
            matching_attention = next(
                (
                    item
                    for item in changed_attention
                    if item.get("specificationId") == specification_id
                ),
                None,
            )
            changed_specification_ok = (
                not any(
                    item.get("specificationId") == specification_id
                    for item in changed_authoritative
                )
                and matching_attention is not None
                and matching_attention.get("relativePath") == specification_path
                and matching_attention.get("state") == "changed"
                and matching_attention.get("exactApprovedRevisionAvailable") is False
                and matching_attention.get("sourceIncluded") is False
                and matching_attention.get("currentContentHash")
                and matching_attention.get("followupTool")
                == "ley_project_specifications"
                and changed_state.get("stateFingerprint")
                != state.get("stateFingerprint")
                and (
                    not specification_body_marker
                    or specification_body_marker not in changed_text
                )
                and (
                    not changed_specification_body_marker
                    or changed_specification_body_marker not in changed_text
                )
            )
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
            state.get("schemaVersion") == 2
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
            and specification_authority_ok
            and changed_specification_ok
            and str(project) not in state_text
            and str(vault) not in state_text
        )
        scores["current_project_state"] = state_ok
        downstream_required = [
            str(value)
            for value in current_state_expectation.get("downstream_required", [])
        ]
        downstream_forbidden = [
            str(value)
            for value in current_state_expectation.get("downstream_forbidden", [])
        ]
        if downstream_required or downstream_forbidden:
            record_downstream_task_contract(
                scores,
                failures,
                text_contract_success(
                    state_text,
                    downstream_required,
                    downstream_forbidden,
                ),
                "Current Project State did not satisfy the independent downstream working-state contract",
            )
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)],
            [state, rebuilt] + ([changed_state] if changed_state is not None else []),
        )
        evidence_text.extend(
            [state, rebuilt] + ([changed_state] if changed_state is not None else [])
        )
        if not state_ok:
            failures.append(
                "Current Project State did not preserve working-state boundaries, Specification authority/revision attention, historical decision semantics, privacy, or source binding"
            )

    graph_relation_expectation = scenario.get("expected_graph_relation_retrieval")
    if isinstance(graph_relation_expectation, dict):
        implementation_path = str(
            graph_relation_expectation.get("implementation_path", "")
        )
        relevant_test_path = str(
            graph_relation_expectation.get("relevant_test_path", "")
        )
        unrelated_test_path = str(
            graph_relation_expectation.get("unrelated_test_path", "")
        )
        baseline_query = str(graph_relation_expectation.get("baseline_query", ""))
        edge_label = str(
            graph_relation_expectation.get("edge_label", "../src/renderer")
        )
        implementation_query = str(
            graph_relation_expectation.get(
                "implementation_query",
                Path(implementation_path).name,
            )
        )
        relevant_test_query = str(
            graph_relation_expectation.get(
                "relevant_test_query",
                Path(relevant_test_path).name,
            )
        )
        if not all(
            [
                implementation_path,
                relevant_test_path,
                unrelated_test_path,
                baseline_query,
                edge_label,
                implementation_query,
                relevant_test_query,
            ]
        ):
            raise RuntimeError(
                "graph relation fixture requires implementation/test paths and baseline query"
            )

        baseline = mcp_call(
            project,
            "ley_search_context",
            {"query": baseline_query, "maxResults": 8, "maxTokens": 1_500},
        )
        neighbors = mcp_call(
            project,
            "ley_graph_neighbors",
            {
                "node": implementation_query,
                "depth": 1,
                "maxNodes": 20,
                "direction": "incoming",
                "edgeKinds": ["imports"],
            },
        )
        path = mcp_call(
            project,
            "ley_graph_path",
            {
                "from": relevant_test_query,
                "to": implementation_query,
                "maxDepth": 1,
                "maxVisitedNodes": 20,
                "direction": "outgoing",
                "edgeKinds": ["imports"],
            },
        )

        neighbor_nodes = [
            item for item in neighbors.get("nodes", []) if isinstance(item, dict)
        ]
        neighbor_edges = [
            item for item in neighbors.get("edges", []) if isinstance(item, dict)
        ]
        neighbor_paths = {
            str(item.get("path"))
            for item in neighbor_nodes
            if isinstance(item.get("path"), str)
        }
        relation_edge = next(
            (
                item
                for item in neighbor_edges
                if item.get("kind") == "imports"
                and item.get("label") == edge_label
            ),
            None,
        )
        path_nodes = [item for item in path.get("nodes", []) if isinstance(item, dict)]
        path_edges = [item for item in path.get("edges", []) if isinstance(item, dict)]
        path_paths = [
            str(item.get("path"))
            for item in path_nodes
            if isinstance(item.get("path"), str)
        ]
        baseline_text = json.dumps(baseline, sort_keys=True)
        graph_relation_ok = (
            baseline.get("liveSourceChecked") is False
            and relevant_test_path not in baseline_text
            and neighbors.get("ambiguous") is False
            and neighbors.get("liveSourceChecked") is False
            and implementation_path in neighbor_paths
            and relevant_test_path in neighbor_paths
            and unrelated_test_path not in neighbor_paths
            and relation_edge is not None
            and relation_edge.get("provenance") == "deterministic"
            and relation_edge.get("confidence") == 1.0
            and relation_edge.get("citation", {}).get("artifactPath")
            == relevant_test_path
            and path.get("found") is True
            and path.get("ambiguous") is False
            and path.get("liveSourceChecked") is False
            and path_paths == [relevant_test_path, implementation_path]
            and len(path_edges) == 1
            and path_edges[0].get("kind") == "imports"
            and path_edges[0].get("provenance") == "deterministic"
        )
        scores["graph_relation_retrieval"] = graph_relation_ok
        downstream_required = [
            str(value)
            for value in graph_relation_expectation.get(
                "downstream_required",
                [relevant_test_path],
            )
        ]
        downstream_forbidden = [
            str(value)
            for value in graph_relation_expectation.get(
                "downstream_forbidden",
                [unrelated_test_path],
            )
        ]
        record_downstream_task_contract(
            scores,
            failures,
            text_contract_success(
                json.dumps({"neighbors": neighbors, "path": path}, sort_keys=True),
                downstream_required,
                downstream_forbidden,
            )
            and not text_contract_success(
                baseline_text,
                downstream_required,
                [],
            ),
            "graph relations did not independently surface the related test beyond the direct-search baseline",
        )
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)], [baseline, neighbors, path]
        )
        evidence_text.extend([baseline, neighbors, path])
        if not graph_relation_ok:
            failures.append(
                "deterministic captured relative-import graph relation did not improve implementation-to-test retrieval over the direct context-search baseline"
                f" (neighbors.ambiguous={neighbors.get('ambiguous')!r}, "
                f"neighborPaths={sorted(neighbor_paths)!r}, relationEdge={relation_edge!r}, "
                f"path.found={path.get('found')!r}, path.ambiguous={path.get('ambiguous')!r}, "
                f"pathPaths={path_paths!r}, pathEdges={path_edges!r})"
            )

    trace_to_code_expectation = scenario.get("expected_trace_to_code_retrieval")
    if isinstance(trace_to_code_expectation, dict):
        trace_query = str(trace_to_code_expectation.get("trace_query", ""))
        trace_path = str(trace_to_code_expectation.get("trace_path", ""))
        symbol = str(trace_to_code_expectation.get("symbol", ""))
        code_path = str(trace_to_code_expectation.get("code_path", ""))
        unrelated_path = str(trace_to_code_expectation.get("unrelated_path", ""))
        if not all([trace_query, trace_path, symbol, code_path, unrelated_path]):
            raise RuntimeError(
                "trace-to-code fixture requires trace query/path, symbol, code path, and unrelated path"
            )

        trace_search = mcp_call(
            project,
            "ley_search_context",
            {
                "query": trace_query,
                "maxResults": 8,
                "maxTokens": 1_500,
            },
        )
        trace_items = [
            item
            for item in trace_search.get("items", [])
            if isinstance(item, dict)
        ]
        trace_item = next(
            (
                item
                for item in trace_items
                if item.get("path") == trace_path
            ),
            {},
        )
        trace_citation = (
            trace_item.get("citation", {})
            if isinstance(trace_item, dict)
            else {}
        )
        trace_evidence = mcp_call(
            project,
            "ley_read_evidence",
            {
                "artifactPath": trace_path,
                "startLine": int(trace_citation.get("startLine", 1)),
                "endLine": int(trace_citation.get("endLine", 20)),
                "maxCharacters": 2_000,
            },
        )
        symbol_neighbors = mcp_call(
            project,
            "ley_graph_neighbors",
            {
                "node": symbol,
                "depth": 1,
                "maxNodes": 20,
                "direction": "incoming",
                "edgeKinds": ["defines"],
            },
        )
        neighbor_nodes = [
            item
            for item in symbol_neighbors.get("nodes", [])
            if isinstance(item, dict)
        ]
        neighbor_edges = [
            item
            for item in symbol_neighbors.get("edges", [])
            if isinstance(item, dict)
        ]
        neighbor_paths = {
            str(item.get("path"))
            for item in neighbor_nodes
            if isinstance(item.get("path"), str)
        }
        defines_edge = next(
            (
                item
                for item in neighbor_edges
                if item.get("kind") == "defines"
                and item.get("citation", {}).get("artifactPath") == code_path
            ),
            None,
        )
        trace_search_text = json.dumps(trace_search, sort_keys=True)
        trace_evidence_text = str(trace_evidence.get("text", ""))
        trace_outputs_text = json.dumps(
            [trace_search, trace_evidence, symbol_neighbors],
            sort_keys=True,
        )
        trace_to_code_checks = {
            "trace-search-snapshot-only":
                trace_search.get("liveSourceChecked") is False,
            "trace-artifact-returned":
                trace_item.get("kind") == "artifact"
                and trace_item.get("path") == trace_path,
            "trace-body-has-symbol": symbol in trace_evidence_text,
            "trace-body-has-marker": trace_query in trace_evidence_text,
            "trace-read-bound":
                trace_evidence.get("artifactPath") == trace_path
                and trace_evidence.get("liveSourceChecked") is False,
            "direct-search-does-not-cheat": code_path not in trace_search_text,
            "symbol-resolution-unambiguous":
                symbol_neighbors.get("ambiguous") is False,
            "graph-snapshot-only":
                symbol_neighbors.get("liveSourceChecked") is False,
            "defining-file-found": code_path in neighbor_paths,
            "unrelated-file-absent": unrelated_path not in neighbor_paths,
            "defines-edge-present": defines_edge is not None,
            "defines-edge-deterministic":
                isinstance(defines_edge, dict)
                and defines_edge.get("provenance") == "deterministic"
                and defines_edge.get("confidence") == 1.0,
            "project-path-private": str(project) not in trace_outputs_text,
            "vault-path-private": str(vault) not in trace_outputs_text,
        }
        trace_to_code_ok = all(trace_to_code_checks.values())
        scores["trace_to_code_retrieval"] = trace_to_code_ok
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)],
            [trace_search, trace_evidence, symbol_neighbors],
        )
        evidence_text.extend(
            [trace_search, trace_evidence, symbol_neighbors]
        )
        if not trace_to_code_ok:
            failed_trace_checks = [
                label
                for label, passed in trace_to_code_checks.items()
                if not passed
            ]
            failures.append(
                "trace-to-code retrieval did not preserve the captured-trace -> symbol -> defining-file progressive-disclosure chain: "
                + ", ".join(failed_trace_checks)
            )

    ripple_expectation = scenario.get("expected_ripple_effect_retrieval")
    if isinstance(ripple_expectation, dict):
        root_path = str(ripple_expectation.get("root_path", ""))
        service_path = str(ripple_expectation.get("service_path", ""))
        api_path = str(ripple_expectation.get("api_path", ""))
        relevant_test_path = str(
            ripple_expectation.get("relevant_test_path", "")
        )
        unrelated_test_path = str(
            ripple_expectation.get("unrelated_test_path", "")
        )
        baseline_query = str(ripple_expectation.get("baseline_query", ""))
        if not all(
            [
                root_path,
                service_path,
                api_path,
                relevant_test_path,
                unrelated_test_path,
                baseline_query,
            ]
        ):
            raise RuntimeError(
                "ripple-effect fixture requires root/service/api/test paths and baseline query"
            )

        baseline = mcp_call(
            project,
            "ley_search_context",
            {
                "query": baseline_query,
                "maxResults": 8,
                "maxTokens": 1_500,
            },
        )
        shallow = mcp_call(
            project,
            "ley_graph_neighbors",
            {
                "node": Path(root_path).name,
                "depth": 1,
                "maxNodes": 30,
                "direction": "incoming",
                "edgeKinds": ["imports"],
            },
        )
        expanded = mcp_call(
            project,
            "ley_graph_neighbors",
            {
                "node": Path(root_path).name,
                "depth": 3,
                "maxNodes": 50,
                "direction": "incoming",
                "edgeKinds": ["imports"],
            },
        )
        path = mcp_call(
            project,
            "ley_graph_path",
            {
                "from": Path(relevant_test_path).name,
                "to": Path(root_path).name,
                "maxDepth": 3,
                "maxVisitedNodes": 100,
                "direction": "outgoing",
                "edgeKinds": ["imports"],
            },
        )
        shallow_paths = {
            str(item.get("path"))
            for item in shallow.get("nodes", [])
            if isinstance(item, dict)
            and isinstance(item.get("path"), str)
        }
        expanded_paths = {
            str(item.get("path"))
            for item in expanded.get("nodes", [])
            if isinstance(item, dict)
            and isinstance(item.get("path"), str)
        }
        path_nodes = [
            item for item in path.get("nodes", []) if isinstance(item, dict)
        ]
        path_edges = [
            item for item in path.get("edges", []) if isinstance(item, dict)
        ]
        path_paths = [
            str(item.get("path"))
            for item in path_nodes
            if isinstance(item.get("path"), str)
        ]
        baseline_text = json.dumps(baseline, sort_keys=True)
        expected_path = [
            relevant_test_path,
            api_path,
            service_path,
            root_path,
        ]
        ripple_ok = (
            baseline.get("liveSourceChecked") is False
            and relevant_test_path not in baseline_text
            and shallow.get("ambiguous") is False
            and shallow.get("liveSourceChecked") is False
            and root_path in shallow_paths
            and service_path in shallow_paths
            and api_path not in shallow_paths
            and relevant_test_path not in shallow_paths
            and expanded.get("ambiguous") is False
            and expanded.get("liveSourceChecked") is False
            and all(
                expected in expanded_paths
                for expected in (
                    root_path,
                    service_path,
                    api_path,
                    relevant_test_path,
                )
            )
            and unrelated_test_path not in expanded_paths
            and path.get("found") is True
            and path.get("ambiguous") is False
            and path.get("liveSourceChecked") is False
            and path_paths == expected_path
            and len(path_edges) == 3
            and all(
                edge.get("kind") == "imports"
                and edge.get("provenance") == "deterministic"
                for edge in path_edges
            )
        )
        scores["ripple_effect_retrieval"] = ripple_ok
        record_downstream_task_contract(
            scores,
            failures,
            text_contract_success(
                json.dumps({"expanded": expanded, "path": path}, sort_keys=True),
                [relevant_test_path],
                [unrelated_test_path],
            )
            and not text_contract_success(
                json.dumps(shallow, sort_keys=True),
                [relevant_test_path],
                [],
            )
            and not text_contract_success(
                baseline_text,
                [relevant_test_path],
                [],
            ),
            "ripple-effect traversal did not surface the transitive impacted test beyond shallow/direct retrieval",
        )
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)],
            [baseline, shallow, expanded, path],
        )
        evidence_text.extend([baseline, shallow, expanded, path])
        if not ripple_ok:
            failures.append(
                "multi-hop import graph did not preserve the expected core -> service -> API -> test impact chain"
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
        captured_hash = "sha256:" + hashlib.sha256(captured_bytes).hexdigest()
        if live_mutation_marker:
            evidence_file.write_text(
                f"{live_mutation_marker}\nThis live file changed after the Ley capture.\n",
                encoding="utf-8",
            )
        live_hash = "sha256:" + hashlib.sha256(evidence_file.read_bytes()).hexdigest()
        session_context = mcp_call(
            project,
            "ley_session_get",
            {"sessionId": session_id, "maxCheckpoints": 5, "maxCharacters": 12000},
        )
        state = mcp_call(
            project,
            "ley_project_state",
            {"maxSessions": 5, "maxKnowledge": 12, "maxCharacters": 12000},
        )
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
        state_verification = next(
            (
                item
                for item in state.get("recentVerification", [])
                if isinstance(item, dict)
                and (
                    not verification_marker
                    or verification_marker in json.dumps(item, sort_keys=True)
                )
            ),
            None,
        )
        state_citation = next(
            (
                item
                for item in (
                    state_verification.get("evidenceArtifacts", [])
                    if isinstance(state_verification, dict)
                    else []
                )
                if isinstance(item, dict) and item.get("artifactPath") == evidence_path
            ),
            None,
        )
        session_text = json.dumps(session_context, sort_keys=True)
        state_text = json.dumps(state, sort_keys=True)
        verification_evidence_ok = (
            matching is not None
            and citation is not None
            and citation.get("contentHash") == captured_hash
            and str(citation.get("artifactSnapshotId", "")).startswith("snp_")
            and int(citation.get("startLine", 0)) >= 1
            and int(citation.get("endLine", 0)) >= int(citation.get("startLine", 0))
            and state_citation is not None
            and state_citation.get("contentHash") == captured_hash
            and captured_hash != live_hash
            and session_context.get("liveSourceChecked") is False
            and state.get("liveSourceChecked") is False
            and (not live_mutation_marker or live_mutation_marker not in session_text)
            and (not live_mutation_marker or live_mutation_marker not in state_text)
            and str(project) not in session_text
            and str(vault) not in session_text
            and str(project) not in state_text
            and str(vault) not in state_text
        )
        scores["verification_evidence_links"] = verification_evidence_ok
        privacy_canaries = [str(project), str(vault)]
        if live_mutation_marker:
            privacy_canaries.append(live_mutation_marker)
        scores["privacy_violation_rate"] = privacy_violation_rate(
            privacy_canaries, [session_context, state]
        )
        evidence_text.extend([session_context, state])
        if not verification_evidence_ok:
            failures.append(
                "verification evidence links did not remain bound to the captured snapshot/hash across live-source drift and derived state"
            )

    multimodal_expectation = scenario.get("expected_multimodal_evidence")
    if isinstance(multimodal_expectation, dict):
        if not session_id:
            raise RuntimeError("multimodal evidence fixture created no structured session")
        evidence_path = str(multimodal_expectation.get("evidence_path", ""))
        expected_media_type = str(multimodal_expectation.get("media_type", ""))
        expected_mime_type = str(multimodal_expectation.get("mime_type", ""))
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

        session_context = mcp_call(
            project,
            "ley_session_get",
            {"sessionId": session_id, "maxCheckpoints": 5, "maxCharacters": 12000},
        )
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
        media_payload, media_result = mcp_call_result(
            project,
            "ley_read_media_evidence",
            {
                "artifactPath": evidence_path,
                "artifactSnapshotId": citation.get("artifactSnapshotId"),
                "contentHash": citation.get("contentHash"),
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
        multimodal_ok = (
            "ley_read_media_evidence" in tool_names
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
            and str(project) not in serialized_media
            and str(vault) not in serialized_media
        )
        scores["multimodal_evidence"] = multimodal_ok
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault), live_marker.decode("ascii")],
            [session_context, media_payload],
        )
        evidence_text.extend([session_context, media_payload])
        if not multimodal_ok:
            failures.append(
                "multimodal evidence did not preserve exact original media, immutable citation provenance, non-text semantics, or bounded MCP image delivery"
            )

    context_utility_expectation = scenario.get("expected_context_utility")
    if isinstance(context_utility_expectation, dict):
        task = str(context_utility_expectation.get("task", ""))
        hidden_marker = str(context_utility_expectation.get("hidden_marker", ""))
        max_results = int(context_utility_expectation.get("max_results", 8))
        max_tokens = int(context_utility_expectation.get("max_tokens", 1_500))
        started = mcp_call(
            project,
            "ley_session_start",
            {
                "requestId": request_id(f"{scenario['id']}:utility:start"),
                "name": "Context utility evaluation",
                "goal": str(scenario["goal"]),
                "host": "codex",
            },
            WRITE_FLAGS,
        )
        utility_session_id = str(started.get("sessionId", ""))
        start_event_id = str(started.get("eventId", ""))
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": task, "maxResults": max_results, "maxTokens": max_tokens},
        )
        context_pack_id = str(compiled.get("contextPackId", ""))
        compiled_text = json.dumps(compiled, sort_keys=True)
        bind_args = {
            "sessionId": utility_session_id,
            "requestId": request_id(f"{scenario['id']}:utility:bind"),
            "expectedEventCount": 1,
            "contextPackId": context_pack_id,
            "task": task,
            "maxResults": max_results,
            "maxTokens": max_tokens,
        }
        bound = mcp_call(
            project,
            "ley_context_utility_bind",
            bind_args,
            WRITE_FLAGS,
        )
        bound_retry = mcp_call(
            project,
            "ley_context_utility_bind",
            bind_args,
            WRITE_FLAGS,
        )
        binding_id = str(bound.get("bindingId", ""))
        checkpoint = mcp_call(
            project,
            "ley_session_checkpoint",
            {
                "sessionId": utility_session_id,
                "requestId": request_id(f"{scenario['id']}:utility:checkpoint"),
                "expectedEventCount": 2,
                "summary": "Applied the bound context to the downstream utility task.",
                "tasks": [
                    {
                        "title": "Complete utility-bound implementation",
                        "status": "completed",
                        "details": "The downstream implementation slice completed.",
                    }
                ],
                "problems": [
                    {
                        "title": "Verify utility-bound outcome",
                        "symptom": "The downstream result needed typed verification evidence.",
                        "expected": "The verification should pass.",
                        "attempts": [
                            {
                                "action": "Run the bounded downstream verification",
                                "outcome": "helped",
                                "evidence": "The typed verification completed.",
                            }
                        ],
                        "resolution": {
                            "rootCause": "Outcome correlation was previously absent.",
                            "change": "Bind the supplied context before the work.",
                            "verification": "The downstream verification passed.",
                        },
                    }
                ],
                "verification": [
                    {
                        "kind": "test",
                        "status": "passed",
                        "summary": "Context utility downstream verification passed.",
                    }
                ],
            },
            WRITE_FLAGS,
        )
        checkpoint_event_id = str(checkpoint.get("eventId", ""))
        finished = mcp_call(
            project,
            "ley_session_finish",
            {
                "sessionId": utility_session_id,
                "requestId": request_id(f"{scenario['id']}:utility:finish"),
                "status": "completed",
                "summary": "The utility-bound downstream task completed.",
                "finalResponse": "Completed and verified the downstream task.",
                "handoff": "",
                "unresolved": [],
            },
            WRITE_FLAGS,
        )
        finish_event_id = str(finished.get("eventId", ""))

        pre_binding_rejected = False
        try:
            mcp_call(
                project,
                "ley_context_utility_observe",
                {
                    "sessionId": utility_session_id,
                    "requestId": request_id(f"{scenario['id']}:utility:invalid-before-binding"),
                    "expectedEventCount": 4,
                    "bindingId": binding_id,
                    "downstreamEventIds": [start_event_id],
                },
                WRITE_FLAGS,
            )
        except RuntimeError as error:
            pre_binding_rejected = "does not occur after its bound context pack" in str(error)

        observe_args = {
            "sessionId": utility_session_id,
            "requestId": request_id(f"{scenario['id']}:utility:observe"),
            "expectedEventCount": 4,
            "bindingId": binding_id,
            "downstreamEventIds": [checkpoint_event_id, finish_event_id],
        }
        observed = mcp_call(
            project,
            "ley_context_utility_observe",
            observe_args,
            WRITE_FLAGS,
        )
        observed_retry = mcp_call(
            project,
            "ley_context_utility_observe",
            observe_args,
            WRITE_FLAGS,
        )
        session_context = mcp_call(
            project,
            "ley_session_get",
            {
                "sessionId": utility_session_id,
                "maxCheckpoints": 5,
                "maxCharacters": 12_000,
            },
        )
        utility_rows = [
            item
            for item in session_context.get("contextUtilityObservations", [])
            if isinstance(item, dict)
        ]
        utility = utility_rows[0] if len(utility_rows) == 1 else None
        outcomes = (
            [
                item
                for item in utility.get("downstreamOutcomes", [])
                if isinstance(item, dict)
            ]
            if isinstance(utility, dict)
            else []
        )
        checkpoint_outcome = next(
            (item for item in outcomes if item.get("kind") == "checkpoint"), None
        )
        finish_outcome = next(
            (item for item in outcomes if item.get("kind") == "session-finish"), None
        )
        session_text = json.dumps(session_context, sort_keys=True)
        context_utility_ok = (
            context_pack_id.startswith("cpk_")
            and len(context_pack_id) == 68
            and (not hidden_marker or hidden_marker in compiled_text)
            and binding_id.startswith("cub_")
            and bound.get("contextPackId") == context_pack_id
            and bound.get("eventCount") == 2
            and bound.get("replayed") is False
            and bound_retry.get("bindingId") == binding_id
            and bound_retry.get("eventCount") == 2
            and bound_retry.get("replayed") is True
            and checkpoint.get("eventCount") == 3
            and finished.get("eventCount") == 4
            and pre_binding_rejected
            and observed.get("eventCount") == 5
            and observed.get("replayed") is False
            and observed_retry.get("eventId") == observed.get("eventId")
            and observed_retry.get("eventCount") == 5
            and observed_retry.get("replayed") is True
            and session_context.get("schemaVersion") == 5
            and session_context.get("projectionSchemaVersion") == 1
            and session_context.get("contextUtilityBindingCount") == 1
            and session_context.get("contextUtilityObservationCount") == 1
            and session_context.get("omittedContextUtilityObservations") == 0
            and isinstance(utility, dict)
            and utility.get("bindingId") == binding_id
            and utility.get("contextPackId") == context_pack_id
            and utility.get("contextPackRevalidated") is True
            and utility.get("contextUsageProven") is False
            and utility.get("causalUtilityProven") is False
            and utility.get("trustChangesApplied") is False
            and utility.get("rankingChangesApplied") is False
            and bool(utility.get("includedRecords"))
            and isinstance(checkpoint_outcome, dict)
            and checkpoint_outcome.get("completedTasks") == 1
            and checkpoint_outcome.get("resolvedProblems") == 1
            and checkpoint_outcome.get("helpedAttempts") == 1
            and checkpoint_outcome.get("passedVerifications") == 1
            and isinstance(finish_outcome, dict)
            and finish_outcome.get("sessionStatus") == "completed"
            and session_context.get("liveSourceChecked") is False
            and (not hidden_marker or hidden_marker not in session_text)
            and str(project) not in session_text
            and str(vault) not in session_text
        )
        scores["context_memory_utility"] = context_utility_ok
        utility_privacy_canaries = [str(project), str(vault)]
        if hidden_marker:
            utility_privacy_canaries.append(hidden_marker)
        scores["privacy_violation_rate"] = privacy_violation_rate(
            utility_privacy_canaries,
            [bound, bound_retry, checkpoint, finished, observed, observed_retry, session_context],
        )
        evidence_text.extend(
            [compiled, bound, bound_retry, checkpoint, finished, observed, observed_retry, session_context]
        )
        if not context_utility_ok:
            failures.append(
                "context/memory utility feedback did not preserve exact pre-work pack binding, downstream-only outcome attribution, metadata-only privacy, retry safety, and non-authority semantics"
            )

    unobserved_utility_expectation = scenario.get(
        "expected_unobserved_context_utility_health"
    )
    if isinstance(unobserved_utility_expectation, dict):
        task = str(unobserved_utility_expectation.get("task", ""))
        hidden_marker = str(unobserved_utility_expectation.get("hidden_marker", ""))
        max_results = int(unobserved_utility_expectation.get("max_results", 8))
        max_tokens = int(unobserved_utility_expectation.get("max_tokens", 1_500))
        started = mcp_call(
            project,
            "ley_session_start",
            {
                "requestId": request_id(f"{scenario['id']}:unobserved:start"),
                "name": "Unobserved context utility evaluation",
                "goal": str(scenario["goal"]),
                "host": "codex",
            },
            WRITE_FLAGS,
        )
        utility_session_id = str(started.get("sessionId", ""))
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {"task": task, "maxResults": max_results, "maxTokens": max_tokens},
        )
        compiled_text = json.dumps(compiled, sort_keys=True)
        context_pack_id = str(compiled.get("contextPackId", ""))
        bound = mcp_call(
            project,
            "ley_context_utility_bind",
            {
                "sessionId": utility_session_id,
                "requestId": request_id(f"{scenario['id']}:unobserved:bind"),
                "expectedEventCount": 1,
                "contextPackId": context_pack_id,
                "task": task,
                "maxResults": max_results,
                "maxTokens": max_tokens,
            },
            WRITE_FLAGS,
        )
        binding_id = str(bound.get("bindingId", ""))
        finished = mcp_call(
            project,
            "ley_session_finish",
            {
                "sessionId": utility_session_id,
                "requestId": request_id(f"{scenario['id']}:unobserved:finish"),
                "status": "completed",
                "summary": "The work ended without attaching a context utility observation.",
                "finalResponse": "Terminal context utility measurement fixture.",
                "handoff": "",
                "unresolved": [],
            },
            WRITE_FLAGS,
        )
        session_context = mcp_call(
            project,
            "ley_session_get",
            {
                "sessionId": utility_session_id,
                "maxCheckpoints": 5,
                "maxCharacters": 12_000,
            },
        )
        health = mcp_call(
            project,
            "ley_memory_health",
            {
                "maxSignals": int(unobserved_utility_expectation.get("max_signals", 100)),
                "maxSessions": int(unobserved_utility_expectation.get("max_sessions", 20)),
                "maxCharacters": int(
                    unobserved_utility_expectation.get("max_characters", 16_000)
                ),
            },
        )
        health_text = json.dumps(health, sort_keys=True)
        signals = [
            item
            for item in health.get("signals", [])
            if isinstance(item, dict)
            and item.get("kind") == "unobserved-context-utility-binding"
        ]
        signal = signals[0] if len(signals) == 1 else {}
        unobserved_binding_rows = [
            item
            for item in session_context.get("unobservedContextUtilityBindings", [])
            if isinstance(item, dict)
        ]
        unobserved_binding = (
            unobserved_binding_rows[0] if len(unobserved_binding_rows) == 1 else {}
        )
        finish_event_id = str(finished.get("eventId", ""))
        finish_context = session_context.get("finish", {})
        health_coverage = health.get("coverage", {})
        unobserved_ok = (
            context_pack_id.startswith("cpk_")
            and binding_id.startswith("cub_")
            and bool(compiled.get("items"))
            and finished.get("eventCount") == 3
            and session_context.get("contextUtilityBindingCount") == 1
            and session_context.get("contextUtilityObservationCount") == 0
            and session_context.get("observedContextUtilityBindingCount") == 0
            and session_context.get("unobservedContextUtilityBindingCount") == 1
            and isinstance(finish_context, dict)
            and finish_context.get("eventId") == finish_event_id
            and len(unobserved_binding_rows) == 1
            and unobserved_binding.get("bindingId") == binding_id
            and unobserved_binding.get("contextPackId") == context_pack_id
            and unobserved_binding.get("terminalFinishEventId") == finish_event_id
            and unobserved_binding.get("contextPackRevalidated") is True
            and unobserved_binding.get("contextUsageProven") is False
            and int(unobserved_binding.get("includedRecordCount", 0)) > 0
            and health.get("schemaVersion") == 5
            and health_coverage.get("contextUtilityBindingsInspected") == 1
            and health_coverage.get("observedContextUtilityBindingsInspected") == 0
            and health_coverage.get("unobservedContextUtilityBindingsInspected") == 1
            and health_coverage.get("procedureApplicationClaimsInspected") == 0
            and health_coverage.get("exactCurrentProcedureApplicationClaimsInspected") == 0
            and health.get("persisted") is False
            and health.get("destructiveActionsTaken") is False
            and health.get("liveSourceChecked") is False
            and len(signals) == 1
            and signal.get("severity") == "review"
            and signal.get("relatedLearningIds") == []
            and signal.get("relatedSessionIds") == [utility_session_id]
            and signal.get("relatedRecordIds") == [binding_id]
            and "does not prove" in str(signal.get("detail", ""))
            and (not hidden_marker or hidden_marker in compiled_text)
            and (not hidden_marker or hidden_marker not in health_text)
            and str(project) not in health_text
            and str(vault) not in health_text
        )
        scores["unobserved_context_utility_health"] = unobserved_ok
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault), hidden_marker],
            [session_context, health],
        )
        evidence_text.extend([compiled, bound, finished, session_context, health])
        if not unobserved_ok:
            failures.append(
                "terminal unobserved context utility binding was not surfaced as body-free measurement coverage and advisory Memory Health attention"
            )

    procedure_history_expectation = scenario.get("expected_procedure_application_history")
    if isinstance(procedure_history_expectation, dict):
        title = str(
            procedure_history_expectation.get(
                "title",
                "Release verification procedure",
            )
        )
        guidance = str(
            procedure_history_expectation.get(
                "guidance",
                "Run release verification before shipping.",
            )
        )
        runs = [
            item
            for item in procedure_history_expectation.get("runs", [])
            if isinstance(item, dict)
        ]
        if len(runs) != 3:
            raise RuntimeError(
                "procedure application history fixture must define exactly three runs"
            )

        evidence_started = mcp_call(
            project,
            "ley_session_start",
            {
                "requestId": request_id(
                    f"{scenario['id']}:procedure-history:evidence:start"
                ),
                "name": "Procedure review evidence",
                "goal": "Create durable evidence for one user-reviewed procedure.",
                "host": "codex",
            },
            WRITE_FLAGS,
        )
        evidence_session_id = str(evidence_started.get("sessionId", ""))
        mcp_call(
            project,
            "ley_session_checkpoint",
            {
                "sessionId": evidence_session_id,
                "requestId": request_id(
                    f"{scenario['id']}:procedure-history:evidence:checkpoint"
                ),
                "expectedEventCount": 1,
                "summary": "Verified the reusable release procedure source.",
                "touchedArtifacts": ["src/release.rs"],
                "verification": [
                    {
                        "kind": "test",
                        "status": "passed",
                        "summary": "Procedure source evidence reviewed.",
                    }
                ],
            },
            WRITE_FLAGS,
        )
        evidence_context = mcp_call(
            project,
            "ley_session_get",
            {
                "sessionId": evidence_session_id,
                "maxCheckpoints": 5,
                "maxCharacters": 8_000,
            },
        )
        evidence_checkpoints = [
            item
            for item in evidence_context.get("checkpoints", [])
            if isinstance(item, dict)
        ]
        if not evidence_checkpoints:
            raise RuntimeError(
                "procedure application history fixture created no checkpoint evidence"
            )
        evidence_record_id = str(evidence_checkpoints[-1].get("checkpointId", ""))
        proposed = mcp_call(
            project,
            "ley_learning_propose",
            {
                "requestId": request_id(
                    f"{scenario['id']}:procedure-history:proposal"
                ),
                "kind": "procedure",
                "title": title,
                "guidance": guidance,
                "confidencePercent": 90,
                "provenance": "agent-authored",
                "evidence": [
                    {
                        "sessionId": evidence_session_id,
                        "recordId": evidence_record_id,
                        "note": "Evaluation evidence for reviewed procedure application history.",
                    }
                ],
            },
            WRITE_FLAGS,
        )
        learning_id = str(proposed.get("learningId", ""))
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
                "Explicitly reviewed procedure for application-history evaluation.",
                "--request-id",
                request_id(f"{scenario['id']}:procedure-history:review"),
                "--json",
            ]
        )
        reviewed_learning = (
            reviewed.get("learning", {}) if isinstance(reviewed, dict) else {}
        )
        reviewed_event_count = int(reviewed_learning.get("eventCount", 0))
        run_outputs: list[object] = [evidence_context, proposed, reviewed]
        run_checks: list[bool] = []
        procedure_run_records: list[dict[str, str]] = []
        invalid_claim_rejected = False

        for index, run_spec in enumerate(runs):
            task = str(run_spec.get("task", ""))
            verification_status = str(run_spec.get("verification_status", ""))
            expected_passed = int(run_spec.get("passed_verifications", 0))
            expected_failed = int(run_spec.get("failed_verifications", 0))
            if not task or verification_status not in {"passed", "failed"}:
                raise RuntimeError(
                    "procedure application history run requires task and passed/failed verification_status"
                )

            started = mcp_call(
                project,
                "ley_session_start",
                {
                    "requestId": request_id(
                        f"{scenario['id']}:procedure-history:{index}:start"
                    ),
                    "name": f"Procedure application run {index + 1}",
                    "goal": task,
                    "host": "codex",
                },
                WRITE_FLAGS,
            )
            work_session_id = str(started.get("sessionId", ""))
            compiled = mcp_call(
                project,
                "ley_compile_context",
                {"task": task, "maxResults": 8, "maxTokens": 1_500},
            )
            learning_items = [
                item
                for item in compiled.get("items", [])
                if isinstance(item, dict)
                and item.get("learningId") == learning_id
            ]
            learning_item = learning_items[0] if len(learning_items) == 1 else {}
            context_pack_id = str(compiled.get("contextPackId", ""))
            bound = mcp_call(
                project,
                "ley_context_utility_bind",
                {
                    "sessionId": work_session_id,
                    "requestId": request_id(
                        f"{scenario['id']}:procedure-history:{index}:bind"
                    ),
                    "expectedEventCount": 1,
                    "contextPackId": context_pack_id,
                    "task": task,
                    "maxResults": 8,
                    "maxTokens": 1_500,
                },
                WRITE_FLAGS,
            )
            binding_id = str(bound.get("bindingId", ""))
            checkpoint = mcp_call(
                project,
                "ley_session_checkpoint",
                {
                    "sessionId": work_session_id,
                    "requestId": request_id(
                        f"{scenario['id']}:procedure-history:{index}:checkpoint"
                    ),
                    "expectedEventCount": 2,
                    "summary": (
                        "Caller claims the reviewed procedure was applied under this run's "
                        "recorded task/condition; preserve the typed outcome separately."
                    ),
                    "verification": [
                        {
                            "kind": "test",
                            "status": verification_status,
                            "summary": f"Procedure application run {index + 1} verification {verification_status}.",
                        }
                    ],
                },
                WRITE_FLAGS,
            )
            checkpoint_event_id = str(checkpoint.get("eventId", ""))

            if index == 0:
                try:
                    mcp_call(
                        project,
                        "ley_context_utility_observe",
                        {
                            "sessionId": work_session_id,
                            "requestId": request_id(
                                f"{scenario['id']}:procedure-history:invalid-claim"
                            ),
                            "expectedEventCount": 3,
                            "bindingId": binding_id,
                            "downstreamEventIds": [checkpoint_event_id],
                            "claimedAppliedLearningIds": [
                                "lrn_" + ("f" * 32)
                            ],
                        },
                        WRITE_FLAGS,
                    )
                except RuntimeError as error:
                    invalid_claim_rejected = (
                        "not an exact active-project procedure" in str(error)
                    )

            observed = mcp_call(
                project,
                "ley_context_utility_observe",
                {
                    "sessionId": work_session_id,
                    "requestId": request_id(
                        f"{scenario['id']}:procedure-history:{index}:observe"
                    ),
                    "expectedEventCount": 3,
                    "bindingId": binding_id,
                    "downstreamEventIds": [checkpoint_event_id],
                    "claimedAppliedLearningIds": [learning_id],
                },
                WRITE_FLAGS,
            )
            session_context = mcp_call(
                project,
                "ley_session_get",
                {
                    "sessionId": work_session_id,
                    "maxCheckpoints": 5,
                    "maxCharacters": 8_000,
                },
            )
            utility_rows = [
                item
                for item in session_context.get("contextUtilityObservations", [])
                if isinstance(item, dict)
            ]
            utility = utility_rows[0] if len(utility_rows) == 1 else {}
            outcomes = [
                item
                for item in utility.get("downstreamOutcomes", [])
                if isinstance(item, dict)
            ]
            outcome = outcomes[0] if len(outcomes) == 1 else {}
            run_checks.append(
                bool(
                    learning_item.get("learningKind") == "procedure"
                    and learning_item.get("learningEventCount") == reviewed_event_count
                    and learning_item.get("trustedForReuse") is True
                    and context_pack_id.startswith("cpk_")
                    and binding_id.startswith("cub_")
                    and observed.get("eventCount") == 4
                    and session_context.get("schemaVersion") == 15
                    and session_context.get("projectionSchemaVersion") == 1
                    and utility.get("claimedAppliedLearningIds") == [learning_id]
                    and outcome.get("passedVerifications") == expected_passed
                    and outcome.get("failedVerifications") == expected_failed
                    and utility.get("contextUsageProven") is False
                    and utility.get("causalUtilityProven") is False
                    and utility.get("trustChangesApplied") is False
                    and utility.get("rankingChangesApplied") is False
                )
            )
            procedure_run_records.append(
                {
                    "session_id": work_session_id,
                    "observation_id": str(utility.get("id", "")),
                    "task": task,
                    "verification_status": verification_status,
                }
            )
            run_outputs.extend(
                [started, compiled, bound, checkpoint, observed, session_context]
            )

        final_learning = mcp_call(
            project,
            "ley_learning_get",
            {
                "learningId": learning_id,
                "maxEvidence": 10,
                "maxHistory": 10,
                "maxArtifactsPerEvidence": 10,
                "maxCharacters": 16_000,
            },
        )
        application_rows = [
            item
            for item in final_learning.get("applicationObservations", [])
            if isinstance(item, dict)
        ]
        applications_by_task = {
            str(item.get("taskExcerpt", "")): item for item in application_rows
        }
        expected_tasks = {str(item.get("task", "")) for item in runs}
        expected_outcomes = {
            str(item.get("task", "")): (
                int(item.get("passed_verifications", 0)),
                int(item.get("failed_verifications", 0)),
            )
            for item in runs
        }
        history_rows_ok = len(application_rows) == 3 and set(
            applications_by_task
        ) == expected_tasks
        if history_rows_ok:
            history_rows_ok = all(
                application.get("learningEventCount") == reviewed_event_count
                and application.get("learningVersionMatchesCurrent") is True
                and application.get("passedVerifications")
                == expected_outcomes[task][0]
                and application.get("failedVerifications")
                == expected_outcomes[task][1]
                and application.get("procedureFollowedProven") is False
                and application.get("conditionApplicabilityProven") is False
                and application.get("contextUsageProven") is False
                and application.get("causalUtilityProven") is False
                and application.get("trustChangesApplied") is False
                and application.get("rankingChangesApplied") is False
                for task, application in applications_by_task.items()
            )
        final_text = json.dumps(final_learning, sort_keys=True)
        procedure_history_ok = (
            learning_id.startswith("lrn_")
            and proposed.get("eventCount") == 1
            and reviewed_event_count == 2
            and final_learning.get("schemaVersion") == 2
            and final_learning.get("projectionSchemaVersion") == 1
            and final_learning.get("eventCount") == 2
            and final_learning.get("state") == "verified"
            and final_learning.get("trustState") == "trusted"
            and final_learning.get("applicationObservationCount") == 3
            and final_learning.get("omittedApplicationObservations") == 0
            and "caller-declared"
            in str(final_learning.get("applicationClaimNotice", ""))
            and invalid_claim_rejected
            and all(run_checks)
            and history_rows_ok
            and str(project) not in final_text
            and str(vault) not in final_text
        )
        scores["procedure_application_history"] = procedure_history_ok
        run_outputs.append(final_learning)
        scores["privacy_violation_rate"] = privacy_violation_rate(
            [str(project), str(vault)],
            run_outputs,
        )
        evidence_text.extend(run_outputs)
        if not procedure_history_ok:
            failures.append(
                "procedure application history did not preserve exact reviewed-version binding, mixed typed outcomes, changed-condition task excerpts, invalid-claim rejection, or non-authority semantics"
            )

        procedure_health_expectation = scenario.get(
            "expected_procedure_outcome_health_attention"
        )
        if isinstance(procedure_health_expectation, dict):
            failed_runs = [
                item
                for item in procedure_run_records
                if item.get("verification_status") == "failed"
            ]
            passed_runs = [
                item
                for item in procedure_run_records
                if item.get("verification_status") == "passed"
            ]
            procedure_health = mcp_call(
                project,
                "ley_memory_health",
                {
                    "maxSignals": int(
                        procedure_health_expectation.get("max_signals", 100)
                    ),
                    "maxSessions": int(
                        procedure_health_expectation.get("max_sessions", 20)
                    ),
                    "maxCharacters": int(
                        procedure_health_expectation.get("max_characters", 16_000)
                    ),
                },
            )
            procedure_health_text = json.dumps(procedure_health, sort_keys=True)
            procedure_health_coverage = procedure_health.get("coverage", {})
            attention_signals = [
                item
                for item in procedure_health.get("signals", [])
                if isinstance(item, dict)
                and item.get("kind")
                == procedure_health_expectation.get(
                    "signal_kind", "procedure-application-outcome-attention"
                )
            ]
            attention = attention_signals[0] if len(attention_signals) == 1 else {}
            detail = str(attention.get("detail", ""))
            required_detail_fragments = [
                str(item)
                for item in procedure_health_expectation.get("detail_fragments", [])
            ]
            forbidden_markers = [
                str(item)
                for item in procedure_health_expectation.get("forbidden_markers", [])
                if str(item)
            ]
            failed_run = failed_runs[0] if len(failed_runs) == 1 else {}
            procedure_health_privacy = privacy_violation_rate(
                [str(project), str(vault), *forbidden_markers],
                [procedure_health],
            )
            expected_unsupported_signal = str(
                procedure_health_expectation.get(
                    "unsupported_signal",
                    "old-procedure-never-successfully-reverified",
                )
            )
            procedure_health_ok = (
                len(procedure_run_records) == 3
                and len(failed_runs) == 1
                and len(passed_runs) == 2
                and bool(failed_run.get("session_id"))
                and bool(failed_run.get("observation_id"))
                and all(
                    attention.get("relatedSessionIds")
                    != [passed_run.get("session_id")]
                    and attention.get("relatedRecordIds")
                    != [passed_run.get("observation_id")]
                    for passed_run in passed_runs
                )
                and procedure_health.get("schemaVersion")
                == int(procedure_health_expectation.get("schema_version", 5))
                and procedure_health_coverage.get("procedureApplicationClaimsInspected")
                == int(
                    procedure_health_expectation.get(
                        "procedure_application_claims_inspected", 3
                    )
                )
                and procedure_health_coverage.get(
                    "exactCurrentProcedureApplicationClaimsInspected"
                )
                == int(
                    procedure_health_expectation.get(
                        "exact_current_procedure_application_claims_inspected", 3
                    )
                )
                and procedure_health.get("persisted") is False
                and procedure_health.get("destructiveActionsTaken") is False
                and procedure_health.get("liveSourceChecked") is False
                and len(attention_signals) == 1
                and attention.get("severity")
                == procedure_health_expectation.get("severity", "review")
                and attention.get("relatedLearningIds") == [learning_id]
                and attention.get("relatedSessionIds") == [failed_run.get("session_id")]
                and attention.get("relatedRecordIds")
                == [failed_run.get("observation_id")]
                and all(fragment in detail for fragment in required_detail_fragments)
                and expected_unsupported_signal
                in {
                    str(item.get("signal"))
                    for item in procedure_health.get("unsupportedSignals", [])
                    if isinstance(item, dict)
                }
                and not any(marker in procedure_health_text for marker in forbidden_markers)
                and procedure_health_privacy == 0.0
            )
            scores["procedure_outcome_health_attention"] = procedure_health_ok
            run_outputs.append(procedure_health)
            evidence_text.append(procedure_health)
            if not procedure_health_ok:
                failures.append(
                    "procedure application outcome Memory Health did not isolate the failed run's attention signal, preserve advisory semantics, or withhold task/path/body details"
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
            and inspection.get("schemaVersion") == 3
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

    health_expectation = scenario.get("expected_memory_health")
    if isinstance(health_expectation, dict):
        if not session_id:
            raise RuntimeError("memory health fixture created no structured session")
        hidden_turn_marker = str(
            health_expectation.get("hidden_turn_marker", "memory_health_hidden_turn")
        )
        run(
            [
                "session",
                "prompt",
                session_id,
                str(project),
                "--stdin",
                "--request-id",
                request_id(f"{scenario['id']}:health:prompt"),
                "--json",
            ],
            stdin=f"{hidden_turn_marker} prompt body",
        )
        run(
            [
                "session",
                "response",
                session_id,
                str(project),
                "--stdin",
                "--request-id",
                request_id(f"{scenario['id']}:health:response"),
                "--json",
            ],
            stdin=f"{hidden_turn_marker} response body",
        )
        arguments = {
            "maxSignals": int(health_expectation.get("max_signals", 100)),
            "maxSessions": int(health_expectation.get("max_sessions", 20)),
            "maxCharacters": int(health_expectation.get("max_characters", 16_000)),
        }
        health = mcp_call(project, "ley_memory_health", arguments)
        rebuilt = mcp_call(project, "ley_memory_health", arguments)
        health_text = json.dumps(health, sort_keys=True)
        signal_kinds = {
            str(signal.get("kind"))
            for signal in health.get("signals", [])
            if isinstance(signal, dict)
        }
        expected_kinds = {
            str(value) for value in health_expectation.get("signal_kinds", [])
        }
        unsupported = {
            str(item.get("signal"))
            for item in health.get("unsupportedSignals", [])
            if isinstance(item, dict)
        }
        expected_unsupported = {
            "old-procedure-never-successfully-reverified",
            "failed-consolidations",
            "chronically-retrieved-but-unhelpful-memory",
        }
        health_ok = (
            health.get("schemaVersion") == 5
            and health.get("projection") == "on-demand-memory-health"
            and health.get("persisted") is False
            and health.get("destructiveActionsTaken") is False
            and health.get("hasActionableSignals") is True
            and str(health.get("healthFingerprint", "")).startswith("sha256:")
            and health.get("healthFingerprint") == rebuilt.get("healthFingerprint")
            and expected_kinds.issubset(signal_kinds)
            and expected_unsupported.issubset(unsupported)
            and health.get("liveSourceChecked") is False
            and int(health.get("coverage", {}).get("textCharacters", 0))
            <= arguments["maxCharacters"]
            and hidden_turn_marker not in health_text
            and str(project) not in health_text
            and str(vault) not in health_text
        )
        scores["memory_health"] = health_ok
        privacy_canaries = [str(project), str(vault)]
        if hidden_turn_marker:
            privacy_canaries.append(hidden_turn_marker)
        scores["privacy_violation_rate"] = privacy_violation_rate(
            privacy_canaries, [health]
        )
        evidence_text.extend([health, rebuilt])
        if not health_ok:
            failures.append(
                "Memory Health did not preserve advisory/non-destructive semantics, typed signal coverage, private turn-body omission, or unsupported-signal honesty"
            )

    legibility_expectation = scenario.get("expected_agent_legibility")
    if isinstance(legibility_expectation, dict):
        arguments = {
            "maxEntriesPerSection": int(
                legibility_expectation.get("max_entries_per_section", 12)
            ),
            "maxSessions": int(legibility_expectation.get("max_sessions", 8)),
            "maxCharacters": int(
                legibility_expectation.get("max_characters", 12_000)
            ),
        }
        legibility = mcp_call(project, "ley_agent_legibility", arguments)
        rebuilt = mcp_call(project, "ley_agent_legibility", arguments)
        legibility_text = json.dumps(legibility, sort_keys=True)
        hidden_spec_marker = str(
            legibility_expectation.get("hidden_spec_marker", "")
        )
        architecture_path = str(
            legibility_expectation.get("architecture_path", "")
        )
        directory_path = str(legibility_expectation.get("directory_path", ""))
        policy_path = str(legibility_expectation.get("policy_path", ""))
        schema_path = str(legibility_expectation.get("schema_path", ""))
        observability_path = str(
            legibility_expectation.get("observability_path", "")
        )
        api_path = str(legibility_expectation.get("api_path", ""))
        declared_command = str(
            legibility_expectation.get("declared_command", "")
        )
        observed_command = str(
            legibility_expectation.get("observed_command", "")
        )
        plan_marker = str(legibility_expectation.get("plan_marker", ""))
        specification_path = str(
            legibility_expectation.get("specification_path", "")
        )
        legibility_ok = (
            legibility.get("schemaVersion") == 1
            and legibility.get("projection") == "on-demand-agent-legibility-map"
            and legibility.get("persisted") is False
            and legibility.get("tableOfContentsNotScore") is True
            and "score" not in legibility
            and str(legibility.get("mapFingerprint", "")).startswith("sha256:")
            and legibility.get("mapFingerprint") == rebuilt.get("mapFingerprint")
            and legibility.get("liveSourceChecked") is False
            and legibility.get("egressTarget") == "cloud"
            and (
                not architecture_path
                or architecture_path
                in json.dumps(legibility.get("architectureDocs", []), sort_keys=True)
            )
            and (
                not directory_path
                or directory_path
                in json.dumps(
                    legibility.get("importantDirectories", []), sort_keys=True
                )
            )
            and (
                not policy_path
                or policy_path
                in json.dumps(legibility.get("projectPolicies", []), sort_keys=True)
            )
            and (
                not schema_path
                or schema_path
                in json.dumps(legibility.get("schemaMigrations", []), sort_keys=True)
            )
            and (
                not observability_path
                or observability_path
                in json.dumps(
                    legibility.get("observabilityReferences", []), sort_keys=True
                )
            )
            and (
                not api_path
                or api_path
                in json.dumps(
                    legibility.get("primaryApiCandidates", []), sort_keys=True
                )
            )
            and (
                not declared_command
                or declared_command
                in json.dumps(legibility.get("declaredCommands", []), sort_keys=True)
            )
            and (
                not observed_command
                or observed_command
                in json.dumps(legibility.get("observedCommands", []), sort_keys=True)
            )
            and (
                not plan_marker
                or plan_marker
                in json.dumps(legibility.get("currentPlans", []), sort_keys=True)
            )
            and (
                not specification_path
                or specification_path
                in json.dumps(
                    legibility.get("importantSpecifications", []), sort_keys=True
                )
            )
            and (
                not hidden_spec_marker or hidden_spec_marker not in legibility_text
            )
            and int(legibility.get("coverage", {}).get("textCharacters", 0))
            <= arguments["maxCharacters"]
            and str(project) not in legibility_text
            and str(vault) not in legibility_text
        )
        scores["agent_legibility"] = legibility_ok
        privacy_canaries = [str(project), str(vault)]
        if hidden_spec_marker:
            privacy_canaries.append(hidden_spec_marker)
        scores["privacy_violation_rate"] = privacy_violation_rate(
            privacy_canaries, [legibility]
        )
        evidence_text.extend([legibility, rebuilt])
        if not legibility_ok:
            failures.append(
                "Agent Legibility Map did not preserve TOC-only semantics, source binding, section coverage, Specification-body omission, or privacy"
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
        if (
            len(iterations) != 10
            or not query
            or not activity_query
            or not current_requirement_marker
            or not legacy_markers
            or not final_handoff_marker
        ):
            raise RuntimeError(
                "long-horizon continuity fixture requires exactly ten iterations, current/legacy markers, query, and final handoff"
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
                host="codex" if index % 2 else "claude-code",
            )
            mcp_call(
                project,
                "ley_session_finish",
                {
                    "sessionId": long_session_id,
                    "requestId": request_id(
                        f"{scenario['id']}:long-horizon:{index}:finish"
                    ),
                    "status": "completed",
                    "summary": result_summary,
                    "handoff": handoff,
                    "finalResponse": "",
                    "unresolved": [],
                },
                WRITE_FLAGS,
            )
            context = mcp_call(
                project,
                "ley_session_get",
                {
                    "sessionId": long_session_id,
                    "maxCheckpoints": 5,
                    "maxCharacters": 8_000,
                },
            )
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

        session_list = mcp_call(
            project,
            "ley_sessions_list",
            {"maxResults": 20},
        )
        resume = mcp_call(
            project,
            "ley_project_resume",
            {
                "maxSessions": 3,
                "maxLearnings": 1,
                "maxCharacters": 12_000,
            },
        )
        activity = mcp_call(
            project,
            "ley_search_activity",
            {
                "query": activity_query,
                "maxResults": 20,
            },
        )
        compiled = mcp_call(
            project,
            "ley_compile_context",
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
            for item in activity.get("decisions", [])
            if isinstance(item, dict)
        ]
        activity_text = json.dumps(activity, sort_keys=True)
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
                    and item.get("checkpointCount") == 1
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
            "history-inspectable":
                activity.get("totalSessions") == 10
                and activity.get("totalMatchingDecisions") == 10
                and len(decisions) == 10
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
                and activity.get("liveSourceChecked") is False
                and compiled.get("liveSourceChecked") is False,
        }
        long_horizon_ok = all(long_horizon_checks.values())
        scores["long_horizon_continuity"] = long_horizon_ok
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
            activity,
            compiled,
            *session_contexts,
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
            mcp_call(
                project,
                "ley_memory_health",
                {
                    "maxSignals": 100,
                    "maxSessions": 20,
                    "maxCharacters": 8_000,
                },
            ),
            mcp_call(
                project,
                "ley_agent_legibility",
                {
                    "maxEntriesPerSection": 12,
                    "maxSessions": 8,
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
        codex_task_context = automatic_hook_context(codex_task)
        claude_task_context = automatic_hook_context(claude_task)
        codex_retry_context = automatic_hook_context(codex_retry)
        claude_retry_context = automatic_hook_context(claude_retry)
        portable = (
            marker in codex_text
            and marker in claude_text
            and codex_session_id.startswith("ses_")
            and claude_session_id.startswith("ses_")
            and codex_task_context.startswith("# Ley task context (automatic)")
            and claude_task_context.startswith("# Ley task context (automatic)")
            and "cpk_" in codex_task_context
            and "cpk_" in claude_task_context
            and portable_session_id in codex_task_context
            and portable_session_id in claude_task_context
            and "Portable prior work" in codex_task_context
            and "Portable prior work" in claude_task_context
            and codex_prompt_marker not in codex_task_context
            and claude_prompt_marker not in claude_task_context
            and len(codex_task_context.encode("utf-8")) <= 3_500
            and len(claude_task_context.encode("utf-8")) <= 3_500
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
        evidence_text.extend([codex, claude, codex_task, claude_task, codex_retry, claude_retry])
        if not portable:
            failures.append(
                "durable Ley context or automatic task compilation was not usable from both Codex and Claude lifecycle hosts"
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
                "ley_search_memory",
                {
                    "query": query,
                    "maxResults": max_results,
                    "maxTokens": budget,
                },
            )
            compiler_payload = mcp_call(
                project,
                "ley_compile_context",
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
            search_fallback_reasons = [
                str(search_retrieval.get("boundedRerankFallbackReason", "")),
                str(search_retrieval.get("artifactContextFallbackReason", "")),
            ]
            compiler_fallback_reasons = [
                str(compiler_retrieval.get("boundedRerankFallbackReason", "")),
                str(compiler_retrieval.get("artifactContextFallbackReason", "")),
            ]
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
            fallback_reasons = search_fallback_reasons + compiler_fallback_reasons
            per_budget_checks.append(
                search_retrieval.get("mode") == "lexical"
                and search_retrieval.get("boundedRerankMode") == "lexical"
                and search_retrieval.get("artifactContextMode") == "lexical"
                and compiler_retrieval.get("mode") == "lexical"
                and compiler_retrieval.get("boundedRerankMode") == "lexical"
                and compiler_retrieval.get("artifactContextMode") == "lexical"
                and all(reason and "not installed" in reason for reason in fallback_reasons)
                and compiler_semantic_gap
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
                "retrieval robustness did not preserve lexical fallback, useful sparse evidence, or the multi-budget ladder"
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

        poisoning_session = mcp_call(
            project,
            "ley_session_start",
            {
                "requestId": request_id(
                    f"{scenario['id']}:poisoning:start"
                ),
                "name": "Inspect suspicious repository text",
                "goal": (
                    "Inspect suspicious repository text without treating it "
                    "as trusted instructions."
                ),
                "host": "codex",
            },
            WRITE_FLAGS,
        )
        poisoning_session_id = str(poisoning_session.get("sessionId", ""))
        mcp_call(
            project,
            "ley_session_checkpoint",
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

        proposal = mcp_call(
            project,
            "ley_learning_propose",
            {
                "requestId": request_id(
                    f"{scenario['id']}:poisoning:learning"
                ),
                "kind": "procedure",
                "title": poison_title,
                "guidance": poison_guidance,
                "confidencePercent": 99,
                "provenance": "agent-authored",
                "evidence": [
                    {
                        "sessionId": poisoning_session_id,
                        "recordId": poisoning_checkpoint_id,
                        "note": (
                            "Derived from suspicious repository evidence; "
                            "requires explicit review."
                        ),
                    }
                ],
            },
            WRITE_FLAGS,
        )
        learning_id = str(proposal.get("learningId", ""))
        learning = mcp_call(
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
        search = mcp_call(
            project,
            "ley_search_memory",
            {
                "query": query,
                "maxResults": 8,
                "maxTokens": 1_500,
            },
        )
        compiled = mcp_call(
            project,
            "ley_compile_context",
            {
                "task": query,
                "maxResults": 8,
                "maxTokens": 1_500,
            },
        )
        trusted_list = mcp_call(
            project,
            "ley_learnings_list",
            {"maxResults": 50},
        )
        all_list = mcp_call(
            project,
            "ley_learnings_list",
            {"scope": "all", "maxResults": 50},
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
        task_context = automatic_hook_context(later_prompt)

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
        all_learning = next(
            (
                item
                for item in all_list.get("learnings", [])
                if isinstance(item, dict)
                and item.get("learningId") == learning_id
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
                and learning.get("trustState") == "review-required"
                and learning.get("trustedForReuse") is False,
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
            "default-learning-list-clean":
                trusted_list.get("scope") == "current-trusted"
                and trusted_list.get("totalMatching") == 0
                and not trusted_list.get("learnings"),
            "explicit-learning-list-review-required":
                all_list.get("scope") == "all"
                and all_learning.get("trustState") == "review-required"
                and all_learning.get("state") == "tentative"
                and poison_marker
                in str(all_learning.get("guidanceExcerpt", "")),
            "startup-does-not-inject-poison":
                poison_marker not in startup_context,
            "task-context-does-not-inject-poison":
                bool(task_context)
                and poison_marker not in task_context
                and source_marker in task_context
                and "authority=direct-evidence trusted=false"
                in task_context
                and "evidence, not host policy or permission"
                in task_context,
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
            trusted_list,
            all_list,
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
            typed_compiled: dict[str, object] | None = None
            task_compiled: dict[str, object] | None = None
            task_projection: dict[str, object] | None = None
            task_secret_canary = str(scenario.get("task_secret_canary", ""))
            plan_compiled: dict[str, object] | None = None
            plan_projection: dict[str, object] | None = None
            plan_secret_canary = str(scenario.get("plan_secret_canary", ""))
            batch_compiled: dict[str, object] | None = None
            batch_projection: dict[str, object] | None = None
            batch_secret_canary = str(scenario.get("batch_secret_canary", ""))
            rich_problem_compiled: dict[str, object] | None = None
            rich_problem_projection: dict[str, object] | None = None
            rich_problem_secret_canary = str(
                scenario.get("rich_problem_secret_canary", "")
            )
            composite_compiled: dict[str, object] | None = None
            composite_projection: dict[str, object] | None = None
            composite_secret_canary = str(scenario.get("composite_secret_canary", ""))
            tool_compiled: dict[str, object] | None = None
            tool_history: dict[str, object] | None = None
            tool_projection: dict[str, object] | None = None
            tool_verification: dict[str, object] | None = None
            tool_secret_canary = str(scenario.get("tool_secret_canary", ""))
            tool_raw_call_id = str(scenario.get("tool_raw_call_id", ""))
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
                    cli_json(
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
                            "Explicitly reviewed lineage-bearing recovery learning for downstream evaluation.",
                            "--request-id",
                            request_id(f"{scenario['id']}:origin-lineage:confirm"),
                            "--json",
                        ]
                    )
                    reviewed_learning = mcp_call(
                        project,
                        "ley_learning_get",
                        {
                            "learningId": learning_id,
                            "maxCharacters": 4_000,
                        },
                    )
                    downstream_lineage_context = mcp_call(
                        project,
                        "ley_compile_context",
                        {
                            "task": prompt_text or "Fix the login bug",
                            "maxResults": 8,
                            "maxTokens": 1_500,
                        },
                    )
                    reviewed_learning_auto_admitted = any(
                        isinstance(item, dict)
                        and item.get("learningId") == learning_id
                        for item in downstream_lineage_context.get("items", [])
                    )
                    lineage_preserved_after_review = (
                        reviewed_learning.get("originLineage")
                        == learning_context.get("originLineage")
                    )
                    reviewed_learning_body_ok = text_contract_success(
                        json.dumps(
                            {
                                "title": reviewed_learning.get("title"),
                                "guidance": reviewed_learning.get("guidance"),
                            },
                            sort_keys=True,
                        ),
                        [prompt_text or "Fix the login bug"],
                        [],
                    )
                    downstream_lineage_checks = {
                        "reviewed-learning-verified":
                            reviewed_learning.get("state") == "verified",
                        "reviewed-learning-trusted":
                            reviewed_learning.get("trustState") == "trusted",
                        "uncited-learning-not-auto-reusable":
                            reviewed_learning.get("freshness") == "uncited"
                            and reviewed_learning.get("trustedForReuse") is False,
                        "lineage-preserved-after-review": lineage_preserved_after_review,
                        "required-task-evidence-present": reviewed_learning_body_ok,
                        "compiler-does-not-auto-admit-uncited-learning":
                            not reviewed_learning_auto_admitted,
                    }
                    failed_downstream_lineage_checks = [
                        label
                        for label, passed in downstream_lineage_checks.items()
                        if not passed
                    ]
                    record_downstream_task_contract(
                        scores,
                        failures,
                        all(downstream_lineage_checks.values()),
                        "origin-lineage learning did not preserve useful progressive disclosure and non-laundering after explicit review: "
                        + ", ".join(failed_downstream_lineage_checks),
                    )
                    evidence_text.extend(
                        [reviewed_learning, downstream_lineage_context]
                    )
                typed_binding_ok = True
                typed_lineage_ok = True
                typed_after_ok = True
                task_binding_ok = True
                task_lineage_ok = True
                task_after_ok = True
                plan_binding_ok = True
                plan_lineage_ok = True
                plan_after_ok = True
                batch_binding_ok = True
                batch_lineage_ok = True
                batch_after_ok = True
                rich_problem_binding_ok = True
                rich_problem_lineage_ok = True
                rich_problem_after_ok = True
                composite_binding_ok = True
                composite_lineage_ok = True
                composite_after_ok = True
                tool_evidence_ok = True
                if scenario.get("expected_typed_recovery"):
                    typed_prompt = "Use SQLite for local-first persistence"
                    hook_call(
                        project,
                        "codex",
                        {
                            "hook_event_name": "UserPromptSubmit",
                            "session_id": "ley-eval-crash-thread",
                            "turn_id": "ley-eval-typed-recovery-turn",
                            "prompt": typed_prompt,
                        },
                    )
                    typed_compiled = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
                    )
                    typed_event_count = int(typed_compiled.get("sessionEventCount", 0))
                    typed_evidence = typed_compiled.get("evidence", [])
                    typed_prompt_record = next(
                        (
                            item
                            for item in typed_evidence
                            if isinstance(item, dict)
                            and item.get("kind") == "user-prompt"
                            and item.get("recordId")
                        ),
                        {},
                    )
                    typed_record_id = str(typed_prompt_record.get("recordId", ""))
                    typed_statement = str(typed_prompt_record.get("text", "")) or typed_prompt
                    typed_transition = mcp_call(
                        project,
                        "ley_session_memory_verify",
                        {
                            "sessionId": session_id,
                            "expectedEventCount": typed_event_count,
                            "claims": [
                                {
                                    "kind": "decision",
                                    "subject": "Persistence engine",
                                    "statement": typed_statement,
                                    "evidenceRecordIds": [typed_record_id],
                                }
                            ],
                            "deferredEvidenceRecordIds": [],
                        },
                    )
                    typed_commit_args = {
                        "sessionId": session_id,
                        "requestId": request_id(f"{scenario['id']}:typed-memory-recovery"),
                        "expectedEventCount": typed_event_count,
                        "candidateFingerprint": typed_transition.get("candidateFingerprint", ""),
                        "kind": "decision",
                        "subject": "Persistence engine",
                        "statement": typed_statement,
                        "evidenceRecordIds": [typed_record_id],
                    }
                    typed_receipt = mcp_call(
                        project,
                        "ley_session_memory_commit_structured",
                        typed_commit_args,
                        WRITE_FLAGS,
                    )
                    typed_retry = mcp_call(
                        project,
                        "ley_session_memory_commit_structured",
                        typed_commit_args,
                        WRITE_FLAGS,
                    )
                    typed_session = mcp_call(
                        project,
                        "ley_session_get",
                        {"sessionId": session_id, "maxCheckpoints": 5, "maxCharacters": 8_000},
                    )
                    typed_checkpoints = typed_session.get("checkpoints", [])
                    typed_checkpoint = (
                        typed_checkpoints[-1]
                        if isinstance(typed_checkpoints, list)
                        and typed_checkpoints
                        and isinstance(typed_checkpoints[-1], dict)
                        else {}
                    )
                    typed_decisions = typed_checkpoint.get("decisions", [])
                    typed_binding_ok = (
                        typed_transition.get("state") == "review-required"
                        and typed_transition.get("semanticFaithfulnessProven") is False
                        and typed_transition.get("liveSourceChecked") is False
                        and bool(typed_transition.get("coverage", {}).get("coverageComplete"))
                        and str(typed_transition.get("candidateFingerprint", "")).startswith("sha256:")
                        and typed_receipt.get("eventCount") == typed_event_count + 1
                        and typed_receipt.get("replayed") is False
                        and typed_retry.get("eventCount") == typed_event_count + 1
                        and typed_retry.get("replayed") is True
                        and typed_session.get("schemaVersion") == 8
                        and isinstance(typed_decisions, list)
                        and len(typed_decisions) == 1
                        and isinstance(typed_decisions[0], dict)
                        and typed_decisions[0].get("title") == "Persistence engine"
                        and typed_decisions[0].get("decision") == typed_statement
                        and typed_checkpoint.get("problems") == []
                        and typed_checkpoint.get("unresolved") == []
                    )
                    if not typed_binding_ok:
                        failures.append(
                            "typed bound recovery did not preserve verifier binding/idempotency/projection"
                        )
                    if scenario.get("expected_origin_lineage"):
                        typed_checkpoint_id = str(typed_checkpoint.get("checkpointId", ""))
                        typed_learning = mcp_call(
                            project,
                            "ley_learning_propose",
                            {
                                "requestId": request_id(f"{scenario['id']}:typed-origin-lineage"),
                                "kind": "fact",
                                "title": "Recovered persistence decision",
                                "guidance": typed_statement,
                                "confidencePercent": 50,
                                "provenance": "inferred",
                                "evidence": [
                                    {
                                        "sessionId": session_id,
                                        "recordId": typed_checkpoint_id,
                                        "note": "Derived only from the typed bound recovery checkpoint.",
                                    }
                                ],
                            },
                            WRITE_FLAGS,
                        )
                        typed_learning_context = mcp_call(
                            project,
                            "ley_learning_get",
                            {
                                "learningId": str(typed_learning.get("learningId", "")),
                                "maxCharacters": 4_000,
                            },
                        )
                        typed_lineage = typed_learning_context.get("originLineage", {})
                        typed_sources = (
                            typed_lineage.get("sources", [])
                            if isinstance(typed_lineage, dict)
                            else []
                        )
                        typed_lineage_ok = (
                            bool(typed_checkpoint_id)
                            and typed_lineage.get("mechanicallyResolved") is True
                            and typed_lineage.get("causalCompletenessProven") is False
                            and typed_lineage.get("automaticAuthorityCeiling") == "review-required"
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "recovery-candidate"
                                and source.get("candidateFingerprint")
                                == typed_transition.get("candidateFingerprint")
                                for source in typed_sources
                            )
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == typed_record_id
                                for source in typed_sources
                            )
                        )
                        if not typed_lineage_ok:
                            failures.append(
                                "typed recovery-derived learning did not preserve the bound origin chain"
                            )
                    typed_after = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
                    )
                    typed_after_ok = (
                        typed_after.get("state") == "no-unconsolidated-evidence"
                        and typed_after.get("totalUnconsolidatedEvidence") == 0
                    )
                    scores["memory_binding"] = binding_ok and typed_binding_ok
                    if scenario.get("expected_origin_lineage"):
                        scores["origin_lineage"] = lineage_ok and typed_lineage_ok
                if scenario.get("expected_task_recovery"):
                    task_prompt = (
                        "Task state: Release build is completed; details: smoke test passed. "
                        f"api_key: {task_secret_canary}"
                    )
                    hook_call(
                        project,
                        "codex",
                        {
                            "hook_event_name": "UserPromptSubmit",
                            "session_id": "ley-eval-crash-thread",
                            "turn_id": "ley-eval-task-recovery-turn",
                            "prompt": task_prompt,
                        },
                    )
                    task_compiled = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
                    )
                    task_event_count = int(task_compiled.get("sessionEventCount", 0))
                    task_evidence = task_compiled.get("evidence", [])
                    task_prompt_record = next(
                        (
                            item
                            for item in task_evidence
                            if isinstance(item, dict)
                            and item.get("kind") == "user-prompt"
                            and item.get("recordId")
                        ),
                        {},
                    )
                    task_record_id = str(task_prompt_record.get("recordId", ""))
                    task_prompt_text = str(task_prompt_record.get("text", ""))
                    task_redaction_ok = (
                        bool(task_secret_canary)
                        and task_secret_canary not in task_prompt_text
                        and "[REDACTED:" in task_prompt_text
                    )
                    if not task_redaction_ok:
                        failures.append(
                            "typed Task recovery evidence did not redact the Task-specific secret canary"
                        )
                    task_transition = mcp_call(
                        project,
                        "ley_session_memory_verify_typed",
                        {
                            "sessionId": session_id,
                            "expectedEventCount": task_event_count,
                            "candidate": {
                                "kind": "task",
                                "title": "Release build",
                                "status": "completed",
                                "details": "Smoke test passed",
                                "evidenceRecordIds": [task_record_id],
                            },
                            "deferredEvidenceRecordIds": [],
                        },
                    )
                    task_commit_args = {
                        "sessionId": session_id,
                        "requestId": request_id(f"{scenario['id']}:task-memory-recovery"),
                        "expectedEventCount": task_event_count,
                        "candidateFingerprint": task_transition.get("candidateFingerprint", ""),
                        "title": "Release build",
                        "status": "completed",
                        "details": "Smoke test passed",
                        "evidenceRecordIds": [task_record_id],
                    }
                    task_receipt = mcp_call(
                        project,
                        "ley_session_memory_commit_task",
                        task_commit_args,
                        WRITE_FLAGS,
                    )
                    task_retry = mcp_call(
                        project,
                        "ley_session_memory_commit_task",
                        task_commit_args,
                        WRITE_FLAGS,
                    )
                    task_session = mcp_call(
                        project,
                        "ley_session_get",
                        {"sessionId": session_id, "maxCheckpoints": 8, "maxCharacters": 12_000},
                    )
                    task_checkpoints = task_session.get("checkpoints", [])
                    task_checkpoint = (
                        task_checkpoints[-1]
                        if isinstance(task_checkpoints, list)
                        and task_checkpoints
                        and isinstance(task_checkpoints[-1], dict)
                        else {}
                    )
                    task_records = task_checkpoint.get("tasks", [])
                    diagnostic = cli_json(["doctor", str(project), "--json"])
                    identity = diagnostic.get("identity", {}) if isinstance(diagnostic, dict) else {}
                    project_id = str(identity.get("projectId", "")) if isinstance(identity, dict) else ""
                    task_projection_path = (
                        vault
                        / ".ley"
                        / "agent-memory"
                        / "projects"
                        / project_id
                        / "sessions"
                        / session_id
                        / "session-v9.json"
                    )
                    if project_id and task_projection_path.is_file():
                        loaded_task_projection = json.loads(
                            task_projection_path.read_text(encoding="utf-8")
                        )
                        if isinstance(loaded_task_projection, dict):
                            task_projection = loaded_task_projection
                    durable_task_checkpoints = (
                        task_projection.get("checkpoints", [])
                        if isinstance(task_projection, dict)
                        else []
                    )
                    durable_task_checkpoint = (
                        durable_task_checkpoints[-1]
                        if isinstance(durable_task_checkpoints, list)
                        and durable_task_checkpoints
                        and isinstance(durable_task_checkpoints[-1], dict)
                        else {}
                    )
                    durable_task_records = durable_task_checkpoint.get("tasks", [])
                    durable_task_ok = (
                        isinstance(durable_task_records, list)
                        and len(durable_task_records) == 1
                        and isinstance(durable_task_records[0], dict)
                        and durable_task_records[0].get("title") == "Release build"
                        and durable_task_records[0].get("status") == "completed"
                        and durable_task_records[0].get("details") == "Smoke test passed"
                        and task_secret_canary not in serialized(task_projection)
                    )
                    if not durable_task_ok:
                        failures.append(
                            "typed Task recovery did not durably preserve exact details or leaked the Task secret canary"
                        )
                    task_binding_ok = (
                        task_transition.get("state") == "review-required"
                        and task_transition.get("semanticFaithfulnessProven") is False
                        and task_transition.get("liveSourceChecked") is False
                        and bool(task_transition.get("coverage", {}).get("coverageComplete"))
                        and str(task_transition.get("candidateFingerprint", "")).startswith("sha256:")
                        and task_receipt.get("eventCount") == task_event_count + 1
                        and task_receipt.get("replayed") is False
                        and task_retry.get("eventCount") == task_event_count + 1
                        and task_retry.get("replayed") is True
                        and task_session.get("schemaVersion") == 9
                        and isinstance(task_records, list)
                        and len(task_records) == 1
                        and isinstance(task_records[0], dict)
                        and task_records[0].get("title") == "Release build"
                        and task_records[0].get("status") == "completed"
                        and durable_task_ok
                        and task_redaction_ok
                        and task_checkpoint.get("decisions") == []
                        and task_checkpoint.get("problems") == []
                        and task_checkpoint.get("unresolved") == []
                    )
                    if not task_binding_ok:
                        failures.append(
                            "typed Task recovery did not preserve verifier binding/idempotency/projection"
                        )
                    if scenario.get("expected_origin_lineage"):
                        task_checkpoint_id = str(task_checkpoint.get("checkpointId", ""))
                        task_learning = mcp_call(
                            project,
                            "ley_learning_propose",
                            {
                                "requestId": request_id(f"{scenario['id']}:task-origin-lineage"),
                                "kind": "fact",
                                "title": "Recovered release Task state",
                                "guidance": "Release build completed after the smoke test passed",
                                "confidencePercent": 50,
                                "provenance": "inferred",
                                "evidence": [
                                    {
                                        "sessionId": session_id,
                                        "recordId": task_checkpoint_id,
                                        "note": "Derived only from the typed Task recovery checkpoint.",
                                    }
                                ],
                            },
                            WRITE_FLAGS,
                        )
                        task_learning_context = mcp_call(
                            project,
                            "ley_learning_get",
                            {
                                "learningId": str(task_learning.get("learningId", "")),
                                "maxCharacters": 4_000,
                            },
                        )
                        task_lineage = task_learning_context.get("originLineage", {})
                        task_sources = (
                            task_lineage.get("sources", [])
                            if isinstance(task_lineage, dict)
                            else []
                        )
                        task_lineage_ok = (
                            bool(task_checkpoint_id)
                            and task_lineage.get("mechanicallyResolved") is True
                            and task_lineage.get("causalCompletenessProven") is False
                            and task_lineage.get("automaticAuthorityCeiling") == "review-required"
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "recovery-candidate"
                                and source.get("candidateFingerprint")
                                == task_transition.get("candidateFingerprint")
                                for source in task_sources
                            )
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == task_record_id
                                for source in task_sources
                            )
                        )
                        if not task_lineage_ok:
                            failures.append(
                                "typed Task recovery-derived learning did not preserve the bound origin chain"
                            )
                    task_after = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
                    )
                    task_after_ok = (
                        task_after.get("state") == "no-unconsolidated-evidence"
                        and task_after.get("totalUnconsolidatedEvidence") == 0
                    )
                    scores["memory_binding"] = (
                        binding_ok and typed_binding_ok and task_binding_ok
                    )
                    if scenario.get("expected_origin_lineage"):
                        scores["origin_lineage"] = (
                            lineage_ok and typed_lineage_ok and task_lineage_ok
                        )
                if scenario.get("expected_plan_recovery"):
                    plan_prompt = (
                        "Plan state: Ship the release is completed. "
                        f"api_key: {plan_secret_canary}"
                    )
                    hook_call(
                        project,
                        "codex",
                        {
                            "hook_event_name": "UserPromptSubmit",
                            "session_id": "ley-eval-crash-thread",
                            "turn_id": "ley-eval-plan-recovery-turn",
                            "prompt": plan_prompt,
                        },
                    )
                    plan_compiled = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
                    )
                    plan_event_count = int(plan_compiled.get("sessionEventCount", 0))
                    plan_evidence = plan_compiled.get("evidence", [])
                    plan_prompt_record = next(
                        (
                            item
                            for item in plan_evidence
                            if isinstance(item, dict)
                            and item.get("kind") == "user-prompt"
                            and item.get("recordId")
                        ),
                        {},
                    )
                    plan_record_id = str(plan_prompt_record.get("recordId", ""))
                    plan_prompt_text = str(plan_prompt_record.get("text", ""))
                    plan_redaction_ok = (
                        bool(plan_secret_canary)
                        and plan_secret_canary not in plan_prompt_text
                        and "[REDACTED:" in plan_prompt_text
                    )
                    if not plan_redaction_ok:
                        failures.append(
                            "typed Plan recovery evidence did not redact the Plan-specific secret canary"
                        )
                    plan_transition = mcp_call(
                        project,
                        "ley_session_memory_verify_typed",
                        {
                            "sessionId": session_id,
                            "expectedEventCount": plan_event_count,
                            "candidate": {
                                "kind": "plan",
                                "text": "Ship the release",
                                "status": "completed",
                                "evidenceRecordIds": [plan_record_id],
                            },
                            "deferredEvidenceRecordIds": [],
                        },
                    )
                    plan_commit_args = {
                        "sessionId": session_id,
                        "requestId": request_id(f"{scenario['id']}:plan-memory-recovery"),
                        "expectedEventCount": plan_event_count,
                        "candidateFingerprint": plan_transition.get("candidateFingerprint", ""),
                        "text": "Ship the release",
                        "status": "completed",
                        "evidenceRecordIds": [plan_record_id],
                    }
                    plan_receipt = mcp_call(
                        project,
                        "ley_session_memory_commit_plan",
                        plan_commit_args,
                        WRITE_FLAGS,
                    )
                    plan_retry = mcp_call(
                        project,
                        "ley_session_memory_commit_plan",
                        plan_commit_args,
                        WRITE_FLAGS,
                    )
                    plan_session = mcp_call(
                        project,
                        "ley_session_get",
                        {"sessionId": session_id, "maxCheckpoints": 10, "maxCharacters": 16_000},
                    )
                    plan_checkpoints = plan_session.get("checkpoints", [])
                    plan_checkpoint = (
                        plan_checkpoints[-1]
                        if isinstance(plan_checkpoints, list)
                        and plan_checkpoints
                        and isinstance(plan_checkpoints[-1], dict)
                        else {}
                    )
                    plan_diagnostic = cli_json(["doctor", str(project), "--json"])
                    plan_identity = (
                        plan_diagnostic.get("identity", {})
                        if isinstance(plan_diagnostic, dict)
                        else {}
                    )
                    plan_project_id = (
                        str(plan_identity.get("projectId", ""))
                        if isinstance(plan_identity, dict)
                        else ""
                    )
                    plan_projection_path = (
                        vault
                        / ".ley"
                        / "agent-memory"
                        / "projects"
                        / plan_project_id
                        / "sessions"
                        / session_id
                        / "session-v10.json"
                    )
                    if plan_project_id and plan_projection_path.is_file():
                        loaded_plan_projection = json.loads(
                            plan_projection_path.read_text(encoding="utf-8")
                        )
                        if isinstance(loaded_plan_projection, dict):
                            plan_projection = loaded_plan_projection
                    durable_plan_checkpoints = (
                        plan_projection.get("checkpoints", [])
                        if isinstance(plan_projection, dict)
                        else []
                    )
                    durable_plan_checkpoint = (
                        durable_plan_checkpoints[-1]
                        if isinstance(durable_plan_checkpoints, list)
                        and durable_plan_checkpoints
                        and isinstance(durable_plan_checkpoints[-1], dict)
                        else {}
                    )
                    durable_plan_records = durable_plan_checkpoint.get("plan", [])
                    durable_plan_ok = (
                        isinstance(durable_plan_records, list)
                        and len(durable_plan_records) == 1
                        and isinstance(durable_plan_records[0], dict)
                        and durable_plan_records[0].get("text") == "Ship the release"
                        and durable_plan_records[0].get("status") == "completed"
                        and plan_secret_canary not in serialized(plan_projection)
                    )
                    if not durable_plan_ok:
                        failures.append(
                            "typed Plan recovery did not durably preserve exact state or leaked the Plan secret canary"
                        )
                    plan_binding_ok = (
                        plan_transition.get("state") == "review-required"
                        and plan_transition.get("semanticFaithfulnessProven") is False
                        and plan_transition.get("liveSourceChecked") is False
                        and bool(plan_transition.get("coverage", {}).get("coverageComplete"))
                        and str(plan_transition.get("candidateFingerprint", "")).startswith("sha256:")
                        and plan_receipt.get("eventCount") == plan_event_count + 1
                        and plan_receipt.get("replayed") is False
                        and plan_retry.get("eventCount") == plan_event_count + 1
                        and plan_retry.get("replayed") is True
                        and plan_session.get("schemaVersion") == 10
                        and plan_checkpoint.get("summary") == "Ship the release"
                        and plan_checkpoint.get("decisions") == []
                        and plan_checkpoint.get("tasks") == []
                        and plan_checkpoint.get("problems") == []
                        and plan_checkpoint.get("unresolved") == []
                        and durable_plan_ok
                        and plan_redaction_ok
                    )
                    if not plan_binding_ok:
                        failures.append(
                            "typed Plan recovery did not preserve verifier binding/idempotency/projection"
                        )
                    if scenario.get("expected_origin_lineage"):
                        plan_checkpoint_id = str(plan_checkpoint.get("checkpointId", ""))
                        plan_learning = mcp_call(
                            project,
                            "ley_learning_propose",
                            {
                                "requestId": request_id(f"{scenario['id']}:plan-origin-lineage"),
                                "kind": "fact",
                                "title": "Recovered release Plan state",
                                "guidance": "Ship the release plan completed",
                                "confidencePercent": 50,
                                "provenance": "inferred",
                                "evidence": [
                                    {
                                        "sessionId": session_id,
                                        "recordId": plan_checkpoint_id,
                                        "note": "Derived only from the typed Plan recovery checkpoint.",
                                    }
                                ],
                            },
                            WRITE_FLAGS,
                        )
                        plan_learning_context = mcp_call(
                            project,
                            "ley_learning_get",
                            {
                                "learningId": str(plan_learning.get("learningId", "")),
                                "maxCharacters": 4_000,
                            },
                        )
                        plan_lineage = plan_learning_context.get("originLineage", {})
                        plan_sources = (
                            plan_lineage.get("sources", [])
                            if isinstance(plan_lineage, dict)
                            else []
                        )
                        plan_lineage_ok = (
                            bool(plan_checkpoint_id)
                            and plan_lineage.get("mechanicallyResolved") is True
                            and plan_lineage.get("causalCompletenessProven") is False
                            and plan_lineage.get("automaticAuthorityCeiling") == "review-required"
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "recovery-candidate"
                                and source.get("candidateFingerprint")
                                == plan_transition.get("candidateFingerprint")
                                for source in plan_sources
                            )
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == plan_record_id
                                for source in plan_sources
                            )
                        )
                        if not plan_lineage_ok:
                            failures.append(
                                "typed Plan recovery-derived learning did not preserve the bound origin chain"
                            )
                    plan_after = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
                    )
                    plan_after_ok = (
                        plan_after.get("state") == "no-unconsolidated-evidence"
                        and plan_after.get("totalUnconsolidatedEvidence") == 0
                    )
                    scores["memory_binding"] = (
                        binding_ok and typed_binding_ok and task_binding_ok and plan_binding_ok
                    )
                    if scenario.get("expected_origin_lineage"):
                        scores["origin_lineage"] = (
                            lineage_ok
                            and typed_lineage_ok
                            and task_lineage_ok
                            and plan_lineage_ok
                        )
                if scenario.get("expected_batch_recovery"):
                    batch_first_prompt = (
                        "Atomic batch storage decision: use SQLite. "
                        f"api_key: {batch_secret_canary}"
                    )
                    batch_second_prompt = (
                        "Atomic batch migration task completed; rollout plan completed."
                    )
                    batch_third_prompt = (
                        "Atomic batch backup behavior remains unresolved and needs verification."
                    )
                    hook_call(
                        project,
                        "codex",
                        {
                            "hook_event_name": "UserPromptSubmit",
                            "session_id": "ley-eval-crash-thread",
                            "turn_id": "ley-eval-batch-recovery-turn-1",
                            "prompt": batch_first_prompt,
                        },
                    )
                    hook_call(
                        project,
                        "codex",
                        {
                            "hook_event_name": "UserPromptSubmit",
                            "session_id": "ley-eval-crash-thread",
                            "turn_id": "ley-eval-batch-recovery-turn-2",
                            "prompt": batch_second_prompt,
                        },
                    )
                    hook_call(
                        project,
                        "codex",
                        {
                            "hook_event_name": "UserPromptSubmit",
                            "session_id": "ley-eval-crash-thread",
                            "turn_id": "ley-eval-batch-recovery-turn-3",
                            "prompt": batch_third_prompt,
                        },
                    )
                    batch_compiled = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 8_000},
                    )
                    batch_event_count = int(batch_compiled.get("sessionEventCount", 0))
                    batch_evidence = [
                        item
                        for item in batch_compiled.get("evidence", [])
                        if isinstance(item, dict)
                        and item.get("kind") == "user-prompt"
                        and item.get("recordId")
                    ]
                    batch_first_record = batch_evidence[0] if len(batch_evidence) >= 1 else {}
                    batch_second_record = batch_evidence[1] if len(batch_evidence) >= 2 else {}
                    batch_third_record = batch_evidence[2] if len(batch_evidence) >= 3 else {}
                    batch_first_record_id = str(batch_first_record.get("recordId", ""))
                    batch_second_record_id = str(batch_second_record.get("recordId", ""))
                    batch_third_record_id = str(batch_third_record.get("recordId", ""))
                    batch_first_text = str(batch_first_record.get("text", ""))
                    batch_redaction_ok = (
                        len(batch_evidence) == 3
                        and bool(batch_secret_canary)
                        and batch_secret_canary not in batch_first_text
                        and "[REDACTED:" in batch_first_text
                        and bool(batch_first_record_id)
                        and bool(batch_second_record_id)
                        and bool(batch_third_record_id)
                    )
                    if not batch_redaction_ok:
                        failures.append(
                            "atomic batch recovery evidence did not preserve three bounded records with secret redaction"
                        )

                    batch_candidates = [
                        {
                            "kind": "decision",
                            "title": "Atomic batch storage engine",
                            "decision": "Use SQLite",
                            "evidenceRecordIds": [batch_first_record_id],
                        },
                        {
                            "kind": "task",
                            "title": "Atomic batch migration",
                            "status": "completed",
                            "details": "Migration completed",
                            "evidenceRecordIds": [batch_second_record_id],
                        },
                        {
                            "kind": "plan",
                            "text": "Atomic batch rollout",
                            "status": "completed",
                            "evidenceRecordIds": [batch_second_record_id],
                        },
                        {
                            "kind": "unresolved",
                            "text": "Verify atomic batch backup behavior",
                            "evidenceRecordIds": [batch_third_record_id],
                        },
                    ]
                    batch_summary = "Recovered atomic persistence batch"
                    batch_transition = mcp_call(
                        project,
                        "ley_session_memory_verify_batch",
                        {
                            "sessionId": session_id,
                            "expectedEventCount": batch_event_count,
                            "checkpointSummary": batch_summary,
                            "candidates": batch_candidates,
                            "deferredEvidenceRecordIds": [],
                        },
                    )
                    batch_commit_args = {
                        "sessionId": session_id,
                        "requestId": request_id(f"{scenario['id']}:batch-memory-recovery"),
                        "expectedEventCount": batch_event_count,
                        "candidateFingerprint": batch_transition.get("candidateFingerprint", ""),
                        "checkpointSummary": batch_summary,
                        "candidates": batch_candidates,
                    }
                    batch_receipt = mcp_call(
                        project,
                        "ley_session_memory_commit_batch",
                        batch_commit_args,
                        WRITE_FLAGS,
                    )
                    batch_retry = mcp_call(
                        project,
                        "ley_session_memory_commit_batch",
                        batch_commit_args,
                        WRITE_FLAGS,
                    )
                    batch_session = mcp_call(
                        project,
                        "ley_session_get",
                        {"sessionId": session_id, "maxCheckpoints": 12, "maxCharacters": 20_000},
                    )
                    batch_checkpoints = batch_session.get("checkpoints", [])
                    batch_checkpoint = (
                        batch_checkpoints[-1]
                        if isinstance(batch_checkpoints, list)
                        and batch_checkpoints
                        and isinstance(batch_checkpoints[-1], dict)
                        else {}
                    )
                    batch_decisions = batch_checkpoint.get("decisions", [])
                    batch_tasks = batch_checkpoint.get("tasks", [])
                    batch_unresolved = batch_checkpoint.get("unresolved", [])
                    batch_unresolved_record_ids = batch_checkpoint.get("unresolvedRecordIds", [])

                    batch_diagnostic = cli_json(["doctor", str(project), "--json"])
                    batch_identity = (
                        batch_diagnostic.get("identity", {})
                        if isinstance(batch_diagnostic, dict)
                        else {}
                    )
                    batch_project_id = (
                        str(batch_identity.get("projectId", ""))
                        if isinstance(batch_identity, dict)
                        else ""
                    )
                    batch_projection_path = (
                        vault
                        / ".ley"
                        / "agent-memory"
                        / "projects"
                        / batch_project_id
                        / "sessions"
                        / session_id
                        / "session-v11.json"
                    )
                    if batch_project_id and batch_projection_path.is_file():
                        loaded_batch_projection = json.loads(
                            batch_projection_path.read_text(encoding="utf-8")
                        )
                        if isinstance(loaded_batch_projection, dict):
                            batch_projection = loaded_batch_projection
                    durable_batch_checkpoints = (
                        batch_projection.get("checkpoints", [])
                        if isinstance(batch_projection, dict)
                        else []
                    )
                    durable_batch_checkpoint = (
                        durable_batch_checkpoints[-1]
                        if isinstance(durable_batch_checkpoints, list)
                        and durable_batch_checkpoints
                        and isinstance(durable_batch_checkpoints[-1], dict)
                        else {}
                    )
                    durable_batch_decisions = durable_batch_checkpoint.get("decisions", [])
                    durable_batch_tasks = durable_batch_checkpoint.get("tasks", [])
                    durable_batch_plans = durable_batch_checkpoint.get("plan", [])
                    durable_batch_unresolved = durable_batch_checkpoint.get("unresolved", [])
                    durable_batch_ok = (
                        isinstance(durable_batch_decisions, list)
                        and len(durable_batch_decisions) == 1
                        and isinstance(durable_batch_decisions[0], dict)
                        and durable_batch_decisions[0].get("title") == "Atomic batch storage engine"
                        and durable_batch_decisions[0].get("decision") == "Use SQLite"
                        and isinstance(durable_batch_tasks, list)
                        and len(durable_batch_tasks) == 1
                        and isinstance(durable_batch_tasks[0], dict)
                        and durable_batch_tasks[0].get("title") == "Atomic batch migration"
                        and durable_batch_tasks[0].get("status") == "completed"
                        and durable_batch_tasks[0].get("details") == "Migration completed"
                        and isinstance(durable_batch_plans, list)
                        and len(durable_batch_plans) == 1
                        and isinstance(durable_batch_plans[0], dict)
                        and durable_batch_plans[0].get("text") == "Atomic batch rollout"
                        and durable_batch_plans[0].get("status") == "completed"
                        and durable_batch_unresolved == ["Verify atomic batch backup behavior"]
                        and batch_secret_canary not in serialized(batch_projection)
                    )
                    if not durable_batch_ok:
                        failures.append(
                            "atomic batch recovery did not durably preserve exact Decision/Task/Plan/Unresolved state or leaked its secret canary"
                        )
                    batch_binding_ok = (
                        batch_transition.get("state") == "review-required"
                        and batch_transition.get("semanticFaithfulnessProven") is False
                        and batch_transition.get("liveSourceChecked") is False
                        and bool(batch_transition.get("coverage", {}).get("coverageComplete"))
                        and batch_transition.get("coverage", {}).get("totalCurrentEvidence") == 3
                        and len(batch_transition.get("claimChecks", [])) == 4
                        and str(batch_transition.get("candidateFingerprint", "")).startswith("sha256:")
                        and batch_receipt.get("eventCount") == batch_event_count + 1
                        and batch_receipt.get("replayed") is False
                        and batch_retry.get("eventCount") == batch_event_count + 1
                        and batch_retry.get("eventId") == batch_receipt.get("eventId")
                        and batch_retry.get("replayed") is True
                        and batch_session.get("schemaVersion") == 11
                        and batch_checkpoint.get("summary") == batch_summary
                        and isinstance(batch_decisions, list)
                        and len(batch_decisions) == 1
                        and isinstance(batch_tasks, list)
                        and len(batch_tasks) == 1
                        and batch_checkpoint.get("problems") == []
                        and batch_unresolved == ["Verify atomic batch backup behavior"]
                        and isinstance(batch_unresolved_record_ids, list)
                        and len(batch_unresolved_record_ids) == 1
                        and str(batch_unresolved_record_ids[0]).startswith("unr_")
                        and durable_batch_ok
                        and batch_redaction_ok
                    )
                    if not batch_binding_ok:
                        failures.append(
                            "atomic batch recovery did not preserve verifier binding/idempotency/schema-v11 projection"
                        )

                    if scenario.get("expected_origin_lineage"):
                        batch_decision_id = (
                            str(batch_decisions[0].get("id", ""))
                            if isinstance(batch_decisions, list)
                            and batch_decisions
                            and isinstance(batch_decisions[0], dict)
                            else ""
                        )
                        batch_learning = mcp_call(
                            project,
                            "ley_learning_propose",
                            {
                                "requestId": request_id(f"{scenario['id']}:batch-origin-lineage"),
                                "kind": "fact",
                                "title": "Atomic batch recovered storage decision",
                                "guidance": "Use SQLite for atomic batch storage.",
                                "confidencePercent": 50,
                                "provenance": "inferred",
                                "evidence": [
                                    {
                                        "sessionId": session_id,
                                        "recordId": batch_decision_id,
                                        "note": "Derived only from the recovered Decision child of the atomic batch.",
                                    }
                                ],
                            },
                            WRITE_FLAGS,
                        )
                        batch_learning_context = mcp_call(
                            project,
                            "ley_learning_get",
                            {
                                "learningId": str(batch_learning.get("learningId", "")),
                                "maxCharacters": 4_000,
                            },
                        )
                        batch_lineage = batch_learning_context.get("originLineage", {})
                        batch_sources = (
                            batch_lineage.get("sources", [])
                            if isinstance(batch_lineage, dict)
                            else []
                        )
                        batch_decision_lineage_ok = (
                            bool(batch_decision_id)
                            and batch_lineage.get("mechanicallyResolved") is True
                            and batch_lineage.get("causalCompletenessProven") is False
                            and batch_lineage.get("automaticAuthorityCeiling") == "review-required"
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "recovery-candidate"
                                and source.get("candidateFingerprint")
                                == batch_transition.get("candidateFingerprint")
                                for source in batch_sources
                            )
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == batch_first_record_id
                                for source in batch_sources
                            )
                            and not any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == batch_second_record_id
                                for source in batch_sources
                            )
                            and not any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == batch_third_record_id
                                for source in batch_sources
                            )
                        )

                        batch_unresolved_record_id = (
                            str(batch_unresolved_record_ids[0])
                            if isinstance(batch_unresolved_record_ids, list)
                            and batch_unresolved_record_ids
                            else ""
                        )
                        batch_unresolved_learning = mcp_call(
                            project,
                            "ley_learning_propose",
                            {
                                "requestId": request_id(
                                    f"{scenario['id']}:batch-unresolved-origin-lineage"
                                ),
                                "kind": "fact",
                                "title": "Atomic batch backup behavior remains unresolved",
                                "guidance": "Keep atomic batch backup behavior unresolved until verified.",
                                "confidencePercent": 50,
                                "provenance": "inferred",
                                "evidence": [
                                    {
                                        "sessionId": session_id,
                                        "recordId": batch_unresolved_record_id,
                                        "note": "Derived only from the recovered unresolved child of the atomic batch.",
                                    }
                                ],
                            },
                            WRITE_FLAGS,
                        )
                        batch_unresolved_learning_context = mcp_call(
                            project,
                            "ley_learning_get",
                            {
                                "learningId": str(
                                    batch_unresolved_learning.get("learningId", "")
                                ),
                                "maxCharacters": 4_000,
                            },
                        )
                        batch_unresolved_lineage = batch_unresolved_learning_context.get(
                            "originLineage", {}
                        )
                        batch_unresolved_sources = (
                            batch_unresolved_lineage.get("sources", [])
                            if isinstance(batch_unresolved_lineage, dict)
                            else []
                        )
                        batch_unresolved_lineage_ok = (
                            bool(batch_unresolved_record_id)
                            and batch_unresolved_lineage.get("mechanicallyResolved") is True
                            and batch_unresolved_lineage.get("causalCompletenessProven") is False
                            and batch_unresolved_lineage.get("automaticAuthorityCeiling")
                            == "review-required"
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "recovery-candidate"
                                and source.get("candidateFingerprint")
                                == batch_transition.get("candidateFingerprint")
                                for source in batch_unresolved_sources
                            )
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == batch_third_record_id
                                for source in batch_unresolved_sources
                            )
                            and not any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == batch_first_record_id
                                for source in batch_unresolved_sources
                            )
                            and not any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == batch_second_record_id
                                for source in batch_unresolved_sources
                            )
                        )
                        batch_lineage_ok = (
                            batch_decision_lineage_ok and batch_unresolved_lineage_ok
                        )
                        if not batch_lineage_ok:
                            failures.append(
                                "atomic batch child learning did not preserve its record-specific recovery lineage"
                            )

                    batch_after = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
                    )
                    batch_after_ok = (
                        batch_after.get("state") == "no-unconsolidated-evidence"
                        and batch_after.get("totalUnconsolidatedEvidence") == 0
                    )
                    scores["memory_binding"] = (
                        binding_ok
                        and typed_binding_ok
                        and task_binding_ok
                        and plan_binding_ok
                        and batch_binding_ok
                    )
                    if scenario.get("expected_origin_lineage"):
                        scores["origin_lineage"] = (
                            lineage_ok
                            and typed_lineage_ok
                            and task_lineage_ok
                            and plan_lineage_ok
                            and batch_lineage_ok
                        )
                if scenario.get("expected_rich_problem_recovery"):
                    rich_problem_prompts = [
                        (
                            "ley-eval-rich-problem-turn-1",
                            "Login refresh failure: refreshing returns 401 although the authenticated session should survive. "
                            f"api_key: {rich_problem_secret_canary}",
                        ),
                        (
                            "ley-eval-rich-problem-turn-2",
                            "Attempt: clear browser cookies. Outcome: no effect; refresh still returned 401.",
                        ),
                        (
                            "ley-eval-rich-problem-turn-3",
                            "Attempt: refresh the access token before navigation. Outcome: helped; refresh kept the session authenticated.",
                        ),
                        (
                            "ley-eval-rich-problem-turn-4",
                            "Resolution: the client reused an expired access token; refresh the token before protected navigation; repeated refreshes remained authenticated.",
                        ),
                    ]
                    for turn_id, prompt_text in rich_problem_prompts:
                        hook_call(
                            project,
                            "codex",
                            {
                                "hook_event_name": "UserPromptSubmit",
                                "session_id": "ley-eval-crash-thread",
                                "turn_id": turn_id,
                                "prompt": prompt_text,
                            },
                        )
                    rich_problem_compiled = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 12_000},
                    )
                    rich_problem_event_count = int(
                        rich_problem_compiled.get("sessionEventCount", 0)
                    )
                    rich_problem_evidence = [
                        item
                        for item in rich_problem_compiled.get("evidence", [])
                        if isinstance(item, dict)
                        and item.get("kind") == "user-prompt"
                        and item.get("recordId")
                    ]
                    rich_problem_record_ids = [
                        str(item.get("recordId", "")) for item in rich_problem_evidence
                    ]
                    rich_problem_first_text = (
                        str(rich_problem_evidence[0].get("text", ""))
                        if rich_problem_evidence
                        else ""
                    )
                    rich_problem_redaction_ok = (
                        len(rich_problem_record_ids) == 4
                        and bool(rich_problem_secret_canary)
                        and rich_problem_secret_canary not in rich_problem_first_text
                        and "[REDACTED:" in rich_problem_first_text
                        and all(rich_problem_record_ids)
                    )
                    if not rich_problem_redaction_ok:
                        failures.append(
                            "rich Problem recovery evidence did not preserve four bounded component records with secret redaction"
                        )

                    rich_problem_candidate = {
                        "title": "Login refresh failure",
                        "symptom": "Refreshing returns 401",
                        "expected": "The authenticated session survives refresh",
                        "evidenceRecordIds": [rich_problem_record_ids[0]],
                        "attempts": [
                            {
                                "action": "Clear browser cookies",
                                "outcome": "no-effect",
                                "evidence": "Refresh still returned 401",
                                "evidenceRecordIds": [rich_problem_record_ids[1]],
                            },
                            {
                                "action": "Refresh the access token before navigation",
                                "outcome": "helped",
                                "evidence": "Refresh kept the session authenticated",
                                "evidenceRecordIds": [rich_problem_record_ids[2]],
                            },
                        ],
                        "resolution": {
                            "rootCause": "The client reused an expired access token",
                            "change": "Refresh the token before protected navigation",
                            "verification": "Repeated refreshes remained authenticated",
                            "evidenceRecordIds": [rich_problem_record_ids[3]],
                        },
                    }
                    rich_problem_transition = mcp_call(
                        project,
                        "ley_session_memory_verify_problem",
                        {
                            "sessionId": session_id,
                            "expectedEventCount": rich_problem_event_count,
                            "candidate": rich_problem_candidate,
                            "deferredEvidenceRecordIds": [],
                        },
                    )
                    rich_problem_commit_args = {
                        "sessionId": session_id,
                        "requestId": request_id(
                            f"{scenario['id']}:rich-problem-memory-recovery"
                        ),
                        "expectedEventCount": rich_problem_event_count,
                        "candidateFingerprint": rich_problem_transition.get(
                            "candidateFingerprint", ""
                        ),
                        "candidate": rich_problem_candidate,
                    }
                    rich_problem_receipt = mcp_call(
                        project,
                        "ley_session_memory_commit_problem",
                        rich_problem_commit_args,
                        WRITE_FLAGS,
                    )
                    rich_problem_retry = mcp_call(
                        project,
                        "ley_session_memory_commit_problem",
                        rich_problem_commit_args,
                        WRITE_FLAGS,
                    )
                    rich_problem_session = mcp_call(
                        project,
                        "ley_session_get",
                        {"sessionId": session_id, "maxCheckpoints": 16, "maxCharacters": 24_000},
                    )
                    rich_problem_checkpoints = rich_problem_session.get("checkpoints", [])
                    rich_problem_checkpoint = (
                        rich_problem_checkpoints[-1]
                        if isinstance(rich_problem_checkpoints, list)
                        and rich_problem_checkpoints
                        and isinstance(rich_problem_checkpoints[-1], dict)
                        else {}
                    )
                    rich_problem_rows = rich_problem_checkpoint.get("problems", [])
                    rich_problem_row = (
                        rich_problem_rows[0]
                        if isinstance(rich_problem_rows, list)
                        and rich_problem_rows
                        and isinstance(rich_problem_rows[0], dict)
                        else {}
                    )
                    rich_problem_attempt_rows = rich_problem_row.get("attempts", [])
                    rich_problem_resolution_row = rich_problem_row.get("resolutionDetail", {})

                    rich_problem_diagnostic = cli_json(["doctor", str(project), "--json"])
                    rich_problem_identity = (
                        rich_problem_diagnostic.get("identity", {})
                        if isinstance(rich_problem_diagnostic, dict)
                        else {}
                    )
                    rich_problem_project_id = (
                        str(rich_problem_identity.get("projectId", ""))
                        if isinstance(rich_problem_identity, dict)
                        else ""
                    )
                    rich_problem_projection_path = (
                        vault
                        / ".ley"
                        / "agent-memory"
                        / "projects"
                        / rich_problem_project_id
                        / "sessions"
                        / session_id
                        / "session-v12.json"
                    )
                    if rich_problem_project_id and rich_problem_projection_path.is_file():
                        loaded_rich_problem_projection = json.loads(
                            rich_problem_projection_path.read_text(encoding="utf-8")
                        )
                        if isinstance(loaded_rich_problem_projection, dict):
                            rich_problem_projection = loaded_rich_problem_projection
                    durable_rich_checkpoints = (
                        rich_problem_projection.get("checkpoints", [])
                        if isinstance(rich_problem_projection, dict)
                        else []
                    )
                    durable_rich_checkpoint = (
                        durable_rich_checkpoints[-1]
                        if isinstance(durable_rich_checkpoints, list)
                        and durable_rich_checkpoints
                        and isinstance(durable_rich_checkpoints[-1], dict)
                        else {}
                    )
                    durable_rich_problems = durable_rich_checkpoint.get("problems", [])
                    durable_rich_problem = (
                        durable_rich_problems[0]
                        if isinstance(durable_rich_problems, list)
                        and durable_rich_problems
                        and isinstance(durable_rich_problems[0], dict)
                        else {}
                    )
                    durable_rich_attempts = durable_rich_problem.get("attempts", [])
                    durable_rich_resolution = durable_rich_problem.get("resolution", {})
                    durable_rich_ok = (
                        durable_rich_problem.get("title") == "Login refresh failure"
                        and durable_rich_problem.get("symptom") == "Refreshing returns 401"
                        and durable_rich_problem.get("expected")
                        == "The authenticated session survives refresh"
                        and isinstance(durable_rich_attempts, list)
                        and len(durable_rich_attempts) == 2
                        and isinstance(durable_rich_attempts[0], dict)
                        and durable_rich_attempts[0].get("action") == "Clear browser cookies"
                        and durable_rich_attempts[0].get("outcome") == "no-effect"
                        and durable_rich_attempts[0].get("evidence")
                        == "Refresh still returned 401"
                        and isinstance(durable_rich_attempts[1], dict)
                        and durable_rich_attempts[1].get("action")
                        == "Refresh the access token before navigation"
                        and durable_rich_attempts[1].get("outcome") == "helped"
                        and durable_rich_resolution.get("rootCause")
                        == "The client reused an expired access token"
                        and durable_rich_resolution.get("change")
                        == "Refresh the token before protected navigation"
                        and durable_rich_resolution.get("verification")
                        == "Repeated refreshes remained authenticated"
                        and rich_problem_secret_canary not in serialized(rich_problem_projection)
                    )
                    if not durable_rich_ok:
                        failures.append(
                            "rich Problem recovery did not durably preserve exact Problem/Attempt/Resolution state or leaked its secret canary"
                        )
                    rich_problem_binding_ok = (
                        rich_problem_transition.get("state") == "review-required"
                        and rich_problem_transition.get("semanticFaithfulnessProven") is False
                        and rich_problem_transition.get("liveSourceChecked") is False
                        and bool(
                            rich_problem_transition.get("coverage", {}).get("coverageComplete")
                        )
                        and rich_problem_transition.get("coverage", {}).get(
                            "totalCurrentEvidence"
                        )
                        == 4
                        and len(rich_problem_transition.get("claimChecks", [])) == 4
                        and str(
                            rich_problem_transition.get("candidateFingerprint", "")
                        ).startswith("sha256:")
                        and rich_problem_receipt.get("eventCount")
                        == rich_problem_event_count + 1
                        and rich_problem_receipt.get("replayed") is False
                        and rich_problem_retry.get("eventCount")
                        == rich_problem_event_count + 1
                        and rich_problem_retry.get("eventId")
                        == rich_problem_receipt.get("eventId")
                        and rich_problem_retry.get("replayed") is True
                        and rich_problem_session.get("schemaVersion") == 12
                        and rich_problem_checkpoint.get("summary") == "Login refresh failure"
                        and rich_problem_row.get("title") == "Login refresh failure"
                        and isinstance(rich_problem_attempt_rows, list)
                        and len(rich_problem_attempt_rows) == 2
                        and rich_problem_attempt_rows[0].get("outcome") == "no-effect"
                        and rich_problem_attempt_rows[1].get("outcome") == "helped"
                        and isinstance(rich_problem_resolution_row, dict)
                        and rich_problem_resolution_row.get("rootCause")
                        == "The client reused an expired access token"
                        and durable_rich_ok
                        and rich_problem_redaction_ok
                    )
                    if not rich_problem_binding_ok:
                        failures.append(
                            "rich Problem recovery did not preserve verifier binding/idempotency/schema-v12 projection"
                        )

                    if scenario.get("expected_origin_lineage"):
                        rich_attempt_id = (
                            str(rich_problem_attempt_rows[0].get("id", ""))
                            if isinstance(rich_problem_attempt_rows, list)
                            and rich_problem_attempt_rows
                            and isinstance(rich_problem_attempt_rows[0], dict)
                            else ""
                        )
                        rich_resolution_id = (
                            str(rich_problem_resolution_row.get("id", ""))
                            if isinstance(rich_problem_resolution_row, dict)
                            else ""
                        )
                        rich_attempt_learning = mcp_call(
                            project,
                            "ley_learning_propose",
                            {
                                "requestId": request_id(
                                    f"{scenario['id']}:rich-problem-attempt-lineage"
                                ),
                                "kind": "pitfall",
                                "title": "Cookie clearing did not fix refresh authentication",
                                "guidance": "Do not treat cookie clearing as the fix for this refresh failure.",
                                "confidencePercent": 50,
                                "provenance": "inferred",
                                "evidence": [
                                    {
                                        "sessionId": session_id,
                                        "recordId": rich_attempt_id,
                                        "note": "Derived only from the failed recovered Attempt.",
                                    }
                                ],
                            },
                            WRITE_FLAGS,
                        )
                        rich_attempt_context = mcp_call(
                            project,
                            "ley_learning_get",
                            {
                                "learningId": str(rich_attempt_learning.get("learningId", "")),
                                "maxCharacters": 4_000,
                            },
                        )
                        rich_attempt_lineage = rich_attempt_context.get("originLineage", {})
                        rich_attempt_sources = (
                            rich_attempt_lineage.get("sources", [])
                            if isinstance(rich_attempt_lineage, dict)
                            else []
                        )
                        rich_attempt_lineage_ok = (
                            bool(rich_attempt_id)
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "recovery-candidate"
                                and source.get("candidateFingerprint")
                                == rich_problem_transition.get("candidateFingerprint")
                                for source in rich_attempt_sources
                            )
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == rich_problem_record_ids[1]
                                for source in rich_attempt_sources
                            )
                            and not any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") in {
                                    rich_problem_record_ids[0],
                                    rich_problem_record_ids[2],
                                    rich_problem_record_ids[3],
                                }
                                for source in rich_attempt_sources
                            )
                        )

                        rich_resolution_learning = mcp_call(
                            project,
                            "ley_learning_propose",
                            {
                                "requestId": request_id(
                                    f"{scenario['id']}:rich-problem-resolution-lineage"
                                ),
                                "kind": "procedure",
                                "title": "Refresh the token before protected navigation",
                                "guidance": "Refresh the access token before protected navigation when the previous token is expired.",
                                "confidencePercent": 50,
                                "provenance": "inferred",
                                "evidence": [
                                    {
                                        "sessionId": session_id,
                                        "recordId": rich_resolution_id,
                                        "note": "Derived only from the recovered Resolution.",
                                    }
                                ],
                            },
                            WRITE_FLAGS,
                        )
                        rich_resolution_context = mcp_call(
                            project,
                            "ley_learning_get",
                            {
                                "learningId": str(
                                    rich_resolution_learning.get("learningId", "")
                                ),
                                "maxCharacters": 4_000,
                            },
                        )
                        rich_resolution_lineage = rich_resolution_context.get(
                            "originLineage", {}
                        )
                        rich_resolution_sources = (
                            rich_resolution_lineage.get("sources", [])
                            if isinstance(rich_resolution_lineage, dict)
                            else []
                        )
                        rich_resolution_lineage_ok = (
                            bool(rich_resolution_id)
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "recovery-candidate"
                                and source.get("candidateFingerprint")
                                == rich_problem_transition.get("candidateFingerprint")
                                for source in rich_resolution_sources
                            )
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == rich_problem_record_ids[3]
                                for source in rich_resolution_sources
                            )
                            and not any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") in {
                                    rich_problem_record_ids[0],
                                    rich_problem_record_ids[1],
                                    rich_problem_record_ids[2],
                                }
                                for source in rich_resolution_sources
                            )
                        )
                        rich_problem_lineage_ok = (
                            rich_attempt_lineage_ok and rich_resolution_lineage_ok
                        )
                        if not rich_problem_lineage_ok:
                            failures.append(
                                "rich Problem Attempt/Resolution learning did not preserve component-specific recovery lineage"
                            )

                    rich_problem_after = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
                    )
                    rich_problem_after_ok = (
                        rich_problem_after.get("state") == "no-unconsolidated-evidence"
                        and rich_problem_after.get("totalUnconsolidatedEvidence") == 0
                    )
                    scores["memory_binding"] = (
                        binding_ok
                        and typed_binding_ok
                        and task_binding_ok
                        and plan_binding_ok
                        and batch_binding_ok
                        and rich_problem_binding_ok
                    )
                    if scenario.get("expected_origin_lineage"):
                        scores["origin_lineage"] = (
                            lineage_ok
                            and typed_lineage_ok
                            and task_lineage_ok
                            and plan_lineage_ok
                            and batch_lineage_ok
                            and rich_problem_lineage_ok
                        )
                if scenario.get("expected_composite_recovery"):
                    composite_prompts = [
                        (
                            "ley-eval-composite-recovery-turn-1",
                            "Composite login symptom: refresh returns 401 although authentication should survive. "
                            f"api_key: {composite_secret_canary}",
                        ),
                        (
                            "ley-eval-composite-recovery-turn-2",
                            "Composite failed attempt: clearing browser cookies had no effect on the 401.",
                        ),
                        (
                            "ley-eval-composite-recovery-turn-3",
                            "Composite resolution: an expired access token was the root cause; refreshing before navigation fixed repeated refreshes.",
                        ),
                        (
                            "ley-eval-composite-recovery-turn-4",
                            "Composite follow-up: adopt refresh-before-navigation and mark the refresh-flow migration completed.",
                        ),
                    ]
                    for turn_id, prompt_text in composite_prompts:
                        hook_call(
                            project,
                            "codex",
                            {
                                "hook_event_name": "UserPromptSubmit",
                                "session_id": "ley-eval-crash-thread",
                                "turn_id": turn_id,
                                "prompt": prompt_text,
                            },
                        )
                    composite_compiled = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 8_000},
                    )
                    composite_event_count = int(composite_compiled.get("sessionEventCount", 0))
                    composite_evidence = [
                        item
                        for item in composite_compiled.get("evidence", [])
                        if isinstance(item, dict)
                        and item.get("kind") == "user-prompt"
                        and item.get("recordId")
                    ]
                    composite_record_ids = [
                        str(item.get("recordId", "")) for item in composite_evidence
                    ]
                    composite_first_text = (
                        str(composite_evidence[0].get("text", ""))
                        if composite_evidence
                        else ""
                    )
                    composite_redaction_ok = (
                        len(composite_record_ids) == 4
                        and bool(composite_secret_canary)
                        and composite_secret_canary not in composite_first_text
                        and "[REDACTED:" in composite_first_text
                        and all(composite_record_ids)
                    )
                    if not composite_redaction_ok:
                        failures.append(
                            "composite recovery evidence did not preserve four bounded records with secret redaction"
                        )

                    composite_rich_problem = {
                        "title": "Composite login refresh failure",
                        "symptom": "Refreshing returns 401",
                        "expected": "The authenticated session survives refresh",
                        "evidenceRecordIds": [composite_record_ids[0]],
                        "attempts": [
                            {
                                "action": "Clear browser cookies",
                                "outcome": "no-effect",
                                "evidence": "Refresh still returned 401",
                                "evidenceRecordIds": [composite_record_ids[1]],
                            }
                        ],
                        "resolution": {
                            "rootCause": "The client reused an expired access token",
                            "change": "Refresh the token before protected navigation",
                            "verification": "Repeated refreshes remained authenticated",
                            "evidenceRecordIds": [composite_record_ids[2]],
                        },
                    }
                    composite_siblings = [
                        {
                            "kind": "decision",
                            "title": "Composite token refresh policy",
                            "decision": "Refresh before protected navigation",
                            "evidenceRecordIds": [composite_record_ids[3]],
                        },
                        {
                            "kind": "task",
                            "title": "Composite refresh-flow migration",
                            "status": "completed",
                            "details": "Migration completed",
                            "evidenceRecordIds": [composite_record_ids[3]],
                        },
                    ]
                    composite_summary = "Recovered composite login debugging and migration"
                    composite_transition = mcp_call(
                        project,
                        "ley_session_memory_verify_composite",
                        {
                            "sessionId": session_id,
                            "expectedEventCount": composite_event_count,
                            "checkpointSummary": composite_summary,
                            "richProblem": composite_rich_problem,
                            "siblings": composite_siblings,
                            "deferredEvidenceRecordIds": [],
                        },
                    )
                    composite_commit_args = {
                        "sessionId": session_id,
                        "requestId": request_id(
                            f"{scenario['id']}:composite-memory-recovery"
                        ),
                        "expectedEventCount": composite_event_count,
                        "candidateFingerprint": composite_transition.get(
                            "candidateFingerprint", ""
                        ),
                        "checkpointSummary": composite_summary,
                        "richProblem": composite_rich_problem,
                        "siblings": composite_siblings,
                    }
                    composite_receipt = mcp_call(
                        project,
                        "ley_session_memory_commit_composite",
                        composite_commit_args,
                        WRITE_FLAGS,
                    )
                    composite_retry = mcp_call(
                        project,
                        "ley_session_memory_commit_composite",
                        composite_commit_args,
                        WRITE_FLAGS,
                    )
                    composite_session = mcp_call(
                        project,
                        "ley_session_get",
                        {"sessionId": session_id, "maxCheckpoints": 16, "maxCharacters": 24_000},
                    )
                    composite_checkpoints = composite_session.get("checkpoints", [])
                    composite_checkpoint = (
                        composite_checkpoints[-1]
                        if isinstance(composite_checkpoints, list)
                        and composite_checkpoints
                        and isinstance(composite_checkpoints[-1], dict)
                        else {}
                    )
                    composite_decisions = composite_checkpoint.get("decisions", [])
                    composite_tasks = composite_checkpoint.get("tasks", [])
                    composite_problems = composite_checkpoint.get("problems", [])
                    composite_problem = (
                        composite_problems[0]
                        if isinstance(composite_problems, list)
                        and composite_problems
                        and isinstance(composite_problems[0], dict)
                        else {}
                    )
                    composite_attempts = composite_problem.get("attempts", [])
                    composite_resolution = composite_problem.get("resolutionDetail", {})

                    composite_diagnostic = cli_json(["doctor", str(project), "--json"])
                    composite_identity = (
                        composite_diagnostic.get("identity", {})
                        if isinstance(composite_diagnostic, dict)
                        else {}
                    )
                    composite_project_id = (
                        str(composite_identity.get("projectId", ""))
                        if isinstance(composite_identity, dict)
                        else ""
                    )
                    composite_projection_path = (
                        vault
                        / ".ley"
                        / "agent-memory"
                        / "projects"
                        / composite_project_id
                        / "sessions"
                        / session_id
                        / "session-v13.json"
                    )
                    if composite_project_id and composite_projection_path.is_file():
                        loaded_composite_projection = json.loads(
                            composite_projection_path.read_text(encoding="utf-8")
                        )
                        if isinstance(loaded_composite_projection, dict):
                            composite_projection = loaded_composite_projection
                    durable_composite_checkpoints = (
                        composite_projection.get("checkpoints", [])
                        if isinstance(composite_projection, dict)
                        else []
                    )
                    durable_composite_checkpoint = (
                        durable_composite_checkpoints[-1]
                        if isinstance(durable_composite_checkpoints, list)
                        and durable_composite_checkpoints
                        and isinstance(durable_composite_checkpoints[-1], dict)
                        else {}
                    )
                    durable_composite_decisions = durable_composite_checkpoint.get(
                        "decisions", []
                    )
                    durable_composite_tasks = durable_composite_checkpoint.get("tasks", [])
                    durable_composite_problems = durable_composite_checkpoint.get(
                        "problems", []
                    )
                    durable_composite_problem = (
                        durable_composite_problems[0]
                        if isinstance(durable_composite_problems, list)
                        and durable_composite_problems
                        and isinstance(durable_composite_problems[0], dict)
                        else {}
                    )
                    durable_composite_attempts = durable_composite_problem.get("attempts", [])
                    durable_composite_resolution = durable_composite_problem.get(
                        "resolution", {}
                    )
                    durable_composite_ok = (
                        durable_composite_checkpoint.get("summary") == composite_summary
                        and isinstance(durable_composite_decisions, list)
                        and len(durable_composite_decisions) == 1
                        and isinstance(durable_composite_decisions[0], dict)
                        and durable_composite_decisions[0].get("title")
                        == "Composite token refresh policy"
                        and durable_composite_decisions[0].get("decision")
                        == "Refresh before protected navigation"
                        and isinstance(durable_composite_tasks, list)
                        and len(durable_composite_tasks) == 1
                        and isinstance(durable_composite_tasks[0], dict)
                        and durable_composite_tasks[0].get("title")
                        == "Composite refresh-flow migration"
                        and durable_composite_tasks[0].get("status") == "completed"
                        and durable_composite_problem.get("title")
                        == "Composite login refresh failure"
                        and durable_composite_problem.get("symptom") == "Refreshing returns 401"
                        and durable_composite_problem.get("expected")
                        == "The authenticated session survives refresh"
                        and isinstance(durable_composite_attempts, list)
                        and len(durable_composite_attempts) == 1
                        and isinstance(durable_composite_attempts[0], dict)
                        and durable_composite_attempts[0].get("action")
                        == "Clear browser cookies"
                        and durable_composite_attempts[0].get("outcome") == "no-effect"
                        and durable_composite_resolution.get("rootCause")
                        == "The client reused an expired access token"
                        and durable_composite_resolution.get("change")
                        == "Refresh the token before protected navigation"
                        and composite_secret_canary not in serialized(composite_projection)
                    )
                    if not durable_composite_ok:
                        failures.append(
                            "composite recovery did not durably preserve exact rich Problem/sibling state or leaked its secret canary"
                        )

                    composite_binding_ok = (
                        composite_transition.get("state") == "review-required"
                        and composite_transition.get("semanticFaithfulnessProven") is False
                        and composite_transition.get("liveSourceChecked") is False
                        and bool(
                            composite_transition.get("coverage", {}).get("coverageComplete")
                        )
                        and composite_transition.get("coverage", {}).get(
                            "totalCurrentEvidence"
                        )
                        == 4
                        and len(composite_transition.get("claimChecks", [])) == 5
                        and str(
                            composite_transition.get("candidateFingerprint", "")
                        ).startswith("sha256:")
                        and composite_receipt.get("eventCount")
                        == composite_event_count + 1
                        and composite_receipt.get("replayed") is False
                        and composite_retry.get("eventCount")
                        == composite_event_count + 1
                        and composite_retry.get("eventId")
                        == composite_receipt.get("eventId")
                        and composite_retry.get("replayed") is True
                        and composite_session.get("schemaVersion") == 13
                        and composite_checkpoint.get("summary") == composite_summary
                        and isinstance(composite_decisions, list)
                        and len(composite_decisions) == 1
                        and isinstance(composite_tasks, list)
                        and len(composite_tasks) == 1
                        and composite_problem.get("title")
                        == "Composite login refresh failure"
                        and isinstance(composite_attempts, list)
                        and len(composite_attempts) == 1
                        and composite_attempts[0].get("outcome") == "no-effect"
                        and isinstance(composite_resolution, dict)
                        and composite_resolution.get("rootCause")
                        == "The client reused an expired access token"
                        and durable_composite_ok
                        and composite_redaction_ok
                    )
                    if not composite_binding_ok:
                        failures.append(
                            "composite recovery did not preserve verifier binding/idempotency/schema-v13 projection"
                        )

                    if scenario.get("expected_origin_lineage"):
                        composite_checkpoint_id = str(
                            composite_checkpoint.get("checkpointId", "")
                        )
                        composite_attempt_id = (
                            str(composite_attempts[0].get("id", ""))
                            if isinstance(composite_attempts, list)
                            and composite_attempts
                            and isinstance(composite_attempts[0], dict)
                            else ""
                        )
                        composite_decision_id = (
                            str(composite_decisions[0].get("id", ""))
                            if isinstance(composite_decisions, list)
                            and composite_decisions
                            and isinstance(composite_decisions[0], dict)
                            else ""
                        )
                        composite_checkpoint_learning = mcp_call(
                            project,
                            "ley_learning_propose",
                            {
                                "requestId": request_id(
                                    f"{scenario['id']}:composite-checkpoint-lineage"
                                ),
                                "kind": "fact",
                                "title": "Composite recovery window remained attributable",
                                "guidance": "Treat the composite checkpoint as provenance over the complete recovery window.",
                                "confidencePercent": 50,
                                "provenance": "inferred",
                                "evidence": [
                                    {
                                        "sessionId": session_id,
                                        "recordId": composite_checkpoint_id,
                                        "note": "Derived from the complete composite recovery checkpoint.",
                                    }
                                ],
                            },
                            WRITE_FLAGS,
                        )
                        composite_checkpoint_context = mcp_call(
                            project,
                            "ley_learning_get",
                            {
                                "learningId": str(
                                    composite_checkpoint_learning.get("learningId", "")
                                ),
                                "maxCharacters": 4_000,
                            },
                        )
                        composite_checkpoint_lineage = composite_checkpoint_context.get(
                            "originLineage", {}
                        )
                        composite_checkpoint_sources = (
                            composite_checkpoint_lineage.get("sources", [])
                            if isinstance(composite_checkpoint_lineage, dict)
                            else []
                        )
                        composite_checkpoint_lineage_ok = (
                            bool(composite_checkpoint_id)
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "recovery-candidate"
                                and source.get("candidateFingerprint")
                                == composite_transition.get("candidateFingerprint")
                                for source in composite_checkpoint_sources
                            )
                            and all(
                                any(
                                    isinstance(source, dict)
                                    and source.get("kind") == "turn-evidence"
                                    and source.get("recordId") == record_id
                                    for source in composite_checkpoint_sources
                                )
                                for record_id in composite_record_ids
                            )
                        )

                        composite_attempt_learning = mcp_call(
                            project,
                            "ley_learning_propose",
                            {
                                "requestId": request_id(
                                    f"{scenario['id']}:composite-attempt-lineage"
                                ),
                                "kind": "pitfall",
                                "title": "Composite cookie clearing did not fix refresh authentication",
                                "guidance": "Do not treat cookie clearing as the fix for this composite refresh failure.",
                                "confidencePercent": 50,
                                "provenance": "inferred",
                                "evidence": [
                                    {
                                        "sessionId": session_id,
                                        "recordId": composite_attempt_id,
                                        "note": "Derived only from the recovered composite Attempt.",
                                    }
                                ],
                            },
                            WRITE_FLAGS,
                        )
                        composite_attempt_context = mcp_call(
                            project,
                            "ley_learning_get",
                            {
                                "learningId": str(
                                    composite_attempt_learning.get("learningId", "")
                                ),
                                "maxCharacters": 4_000,
                            },
                        )
                        composite_attempt_lineage = composite_attempt_context.get(
                            "originLineage", {}
                        )
                        composite_attempt_sources = (
                            composite_attempt_lineage.get("sources", [])
                            if isinstance(composite_attempt_lineage, dict)
                            else []
                        )
                        composite_attempt_lineage_ok = (
                            bool(composite_attempt_id)
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "recovery-candidate"
                                and source.get("candidateFingerprint")
                                == composite_transition.get("candidateFingerprint")
                                for source in composite_attempt_sources
                            )
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == composite_record_ids[1]
                                for source in composite_attempt_sources
                            )
                            and not any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") in {
                                    composite_record_ids[0],
                                    composite_record_ids[2],
                                    composite_record_ids[3],
                                }
                                for source in composite_attempt_sources
                            )
                        )

                        composite_decision_learning = mcp_call(
                            project,
                            "ley_learning_propose",
                            {
                                "requestId": request_id(
                                    f"{scenario['id']}:composite-decision-lineage"
                                ),
                                "kind": "procedure",
                                "title": "Composite protected routes refresh before navigation",
                                "guidance": "Refresh the access token before protected navigation.",
                                "confidencePercent": 50,
                                "provenance": "inferred",
                                "evidence": [
                                    {
                                        "sessionId": session_id,
                                        "recordId": composite_decision_id,
                                        "note": "Derived only from the recovered composite Decision.",
                                    }
                                ],
                            },
                            WRITE_FLAGS,
                        )
                        composite_decision_context = mcp_call(
                            project,
                            "ley_learning_get",
                            {
                                "learningId": str(
                                    composite_decision_learning.get("learningId", "")
                                ),
                                "maxCharacters": 4_000,
                            },
                        )
                        composite_decision_lineage = composite_decision_context.get(
                            "originLineage", {}
                        )
                        composite_decision_sources = (
                            composite_decision_lineage.get("sources", [])
                            if isinstance(composite_decision_lineage, dict)
                            else []
                        )
                        composite_decision_lineage_ok = (
                            bool(composite_decision_id)
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "recovery-candidate"
                                and source.get("candidateFingerprint")
                                == composite_transition.get("candidateFingerprint")
                                for source in composite_decision_sources
                            )
                            and any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") == composite_record_ids[3]
                                for source in composite_decision_sources
                            )
                            and not any(
                                isinstance(source, dict)
                                and source.get("kind") == "turn-evidence"
                                and source.get("recordId") in {
                                    composite_record_ids[0],
                                    composite_record_ids[1],
                                    composite_record_ids[2],
                                }
                                for source in composite_decision_sources
                            )
                        )
                        composite_lineage_ok = (
                            composite_checkpoint_lineage_ok
                            and composite_attempt_lineage_ok
                            and composite_decision_lineage_ok
                        )
                        if not composite_lineage_ok:
                            failures.append(
                                "composite recovery learning did not preserve checkpoint-union and child-specific origin lineage"
                            )

                    composite_after = mcp_call(
                        project,
                        "ley_session_memory_compile",
                        {"sessionId": session_id, "maxResults": 20, "maxCharacters": 4_000},
                    )
                    composite_after_ok = (
                        composite_after.get("state") == "no-unconsolidated-evidence"
                        and composite_after.get("totalUnconsolidatedEvidence") == 0
                    )
                    if scenario.get("expected_tool_evidence"):
                        checkpoint_count_before_tool = len(
                            composite_projection.get("checkpoints", [])
                            if isinstance(composite_projection, dict)
                            else []
                        )
                        hook_call(
                            project,
                            "codex",
                            {
                                "hook_event_name": "PostToolUse",
                                "session_id": "ley-eval-crash-thread",
                                "turn_id": "ley-eval-tool-evidence-turn",
                                "tool_name": "Bash",
                                "tool_use_id": tool_raw_call_id,
                                "tool_input": {
                                    "command": (
                                        "cargo test -p ley-core "
                                        f"api_key={tool_secret_canary}"
                                    )
                                },
                                "tool_response": {
                                    "output": (
                                        "command returned with non-zero-like output\n"
                                        f"api_key={tool_secret_canary}"
                                    ),
                                    "metadata": {"exit_code": 1},
                                },
                            },
                        )
                        tool_compiled = mcp_call(
                            project,
                            "ley_session_memory_compile",
                            {
                                "sessionId": session_id,
                                "maxResults": 20,
                                "maxCharacters": 8_000,
                            },
                        )
                        tool_history = mcp_call(
                            project,
                            "ley_session_turns_get",
                            {
                                "sessionId": session_id,
                                "maxResults": 100,
                                "maxCharacters": 64_000,
                            },
                        )
                        tool_rows = tool_compiled.get("supportingToolEvidence", [])
                        tool_row = (
                            tool_rows[-1]
                            if isinstance(tool_rows, list)
                            and tool_rows
                            and isinstance(tool_rows[-1], dict)
                            else {}
                        )
                        history_rows = tool_history.get("toolObservations", [])
                        history_row = (
                            history_rows[-1]
                            if isinstance(history_rows, list)
                            and history_rows
                            and isinstance(history_rows[-1], dict)
                            else {}
                        )
                        tool_project_id = str(tool_history.get("projectId", ""))
                        tool_projection_path = (
                            vault
                            / ".ley"
                            / "agent-memory"
                            / "projects"
                            / tool_project_id
                            / "sessions"
                            / session_id
                            / "session-v14.json"
                        )
                        if tool_project_id and tool_projection_path.is_file():
                            loaded_tool_projection = json.loads(
                                tool_projection_path.read_text(encoding="utf-8")
                            )
                            if isinstance(loaded_tool_projection, dict):
                                tool_projection = loaded_tool_projection
                        durable_tool_rows = (
                            tool_projection.get("toolObservations", [])
                            if isinstance(tool_projection, dict)
                            else []
                        )
                        durable_tool_row = (
                            durable_tool_rows[-1]
                            if isinstance(durable_tool_rows, list)
                            and durable_tool_rows
                            and isinstance(durable_tool_rows[-1], dict)
                            else {}
                        )
                        tool_record_id = str(tool_row.get("recordId", ""))
                        command_candidates = tool_compiled.get(
                            "automaticCommandCandidates", []
                        )
                        command_candidate = (
                            command_candidates[-1]
                            if isinstance(command_candidates, list)
                            and command_candidates
                            and isinstance(command_candidates[-1], dict)
                            else {}
                        )
                        tool_verification = mcp_call(
                            project,
                            "ley_session_memory_verify_observed_command",
                            {
                                "sessionId": session_id,
                                "expectedEventCount": int(
                                    tool_compiled.get("sessionEventCount", 0)
                                ),
                                "sourceRecordId": tool_record_id,
                            },
                        )
                        durable_checkpoint_count = len(
                            tool_projection.get("checkpoints", [])
                            if isinstance(tool_projection, dict)
                            else []
                        )
                        tool_payload_text = serialized(
                            [tool_compiled, tool_history, tool_projection or {}]
                        )
                        tool_checks = {
                            "compiler-state": tool_compiled.get("state")
                            == "no-unconsolidated-evidence",
                            "compiler-old-count": tool_compiled.get("totalUnconsolidatedEvidence")
                            == 0,
                            "compiler-tool-count": tool_compiled.get("totalSupportingToolEvidence")
                            == 1,
                            "compiler-returned-tool-count": tool_compiled.get(
                                "returnedSupportingToolEvidence"
                            )
                            == 1,
                            "compiler-binding-disabled": tool_compiled.get(
                                "toolEvidenceCandidateBindingAllowed"
                            )
                            is False,
                            "command-candidate-source-count": tool_compiled.get(
                                "totalAutomaticCommandCandidateSources"
                            )
                            == 1,
                            "command-candidate-returned-count": tool_compiled.get(
                                "returnedAutomaticCommandCandidates"
                            )
                            == 1,
                            "command-candidate-omitted-count": tool_compiled.get(
                                "omittedAutomaticCommandCandidateSources"
                            )
                            == 0,
                            "command-candidate-suppressed-count": tool_compiled.get(
                                "suppressedAutomaticCommandCandidateSources"
                            )
                            == 0,
                            "command-candidate-ineligible-count": tool_compiled.get(
                                "ineligibleAutomaticCommandObservations"
                            )
                            == 0,
                            "command-candidate-binding-disabled": tool_compiled.get(
                                "automaticCommandCandidateBindingAllowed"
                            )
                            is False,
                            "command-candidate-write-disabled": tool_compiled.get(
                                "automaticCommandWriteAllowed"
                            )
                            is False,
                            "compiler-one-row": isinstance(tool_rows, list)
                            and len(tool_rows) == 1,
                            "opaque-record-id": tool_record_id.startswith("toe_"),
                            "opaque-tool-call-id": str(
                                tool_row.get("toolCallReference", "")
                            ).startswith("tol_"),
                            "tool-name": tool_row.get("toolName") == "Bash",
                            "returned-not-success": tool_row.get("observationKind") == "returned",
                            "row-binding-disabled": tool_row.get("candidateBindingAllowed") is False,
                            "row-command-candidate-eligible": tool_row.get(
                                "automaticCommandCandidateEligibility"
                            )
                            == "eligible",
                            "command-redacted": "[REDACTED:"
                            in str(tool_row.get("command", "")),
                            "result-redacted": "[REDACTED:"
                            in str(tool_row.get("result", "")),
                            "one-derived-command-candidate": isinstance(
                                command_candidates, list
                            )
                            and len(command_candidates) == 1,
                            "candidate-provenance": command_candidate.get(
                                "sourceRecordId"
                            )
                            == tool_record_id,
                            "candidate-command-reference": command_candidate.get(
                                "commandField"
                            )
                            == "supportingToolEvidence.command",
                            "candidate-has-fingerprint": str(
                                command_candidate.get("candidateFingerprint", "")
                            ).startswith("sha256:"),
                            "candidate-verification-allowed": command_candidate.get(
                                "verificationAllowed"
                            )
                            is True,
                            "candidate-explicit-null-exit": "exitCode"
                            in command_candidate
                            and command_candidate.get("exitCode") is None,
                            "candidate-not-persisted": command_candidate.get("persisted")
                            is False,
                            "candidate-not-bindable": command_candidate.get(
                                "candidateBindingAllowed"
                            )
                            is False,
                            "candidate-no-write": command_candidate.get(
                                "automaticWriteAllowed"
                            )
                            is False,
                            "candidate-no-verification": command_candidate.get(
                                "verificationClaimed"
                            )
                            is False,
                            "candidate-no-outcome-proof": command_candidate.get(
                                "outcomeProven"
                            )
                            is False,
                            "candidate-verifier-review-required": tool_verification.get(
                                "state"
                            )
                            == "review-required",
                            "candidate-verifier-source": tool_verification.get(
                                "sourceRecordId"
                            )
                            == tool_record_id,
                            "candidate-verifier-fingerprint": tool_verification.get(
                                "candidateFingerprint"
                            )
                            == command_candidate.get("candidateFingerprint"),
                            "candidate-verifier-null-exit": "exitCode"
                            in tool_verification
                            and tool_verification.get("exitCode") is None,
                            "candidate-verifier-no-write": tool_verification.get(
                                "automaticWriteAllowed"
                            )
                            is False,
                            "candidate-verifier-no-verification": tool_verification.get(
                                "verificationClaimed"
                            )
                            is False,
                            "candidate-verifier-no-outcome": tool_verification.get(
                                "outcomeProven"
                            )
                            is False,
                            "candidate-verifier-no-semantic-proof": tool_verification.get(
                                "semanticFaithfulnessProven"
                            )
                            is False,
                            "history-schema-v14": tool_history.get("schemaVersion") == 14,
                            "history-count": tool_history.get("toolObservationCount", 0) >= 1,
                            "history-record": history_row.get("recordId") == tool_record_id,
                            "history-kind": history_row.get("observationKind") == "returned",
                            "durable-record": durable_tool_row.get("recordId") == tool_record_id,
                            "durable-kind": durable_tool_row.get("observationKind") == "returned",
                            "candidate-not-durable": "automaticCommandCandidates"
                            not in (tool_projection or {}),
                            "no-checkpoint-authority": durable_checkpoint_count
                            == checkpoint_count_before_tool,
                            "secret-absent": tool_secret_canary not in tool_payload_text,
                            "raw-id-absent": tool_raw_call_id not in tool_payload_text,
                        }
                        tool_evidence_ok = all(tool_checks.values())
                        if not tool_evidence_ok:
                            failed_tool_checks = [
                                label for label, passed in tool_checks.items() if not passed
                            ]
                            failures.append(
                                "host Bash tool evidence did not preserve supporting-only schema-v14 provenance/privacy semantics: "
                                + ", ".join(failed_tool_checks)
                            )
                    scores["memory_binding"] = (
                        binding_ok
                        and typed_binding_ok
                        and task_binding_ok
                        and plan_binding_ok
                        and batch_binding_ok
                        and rich_problem_binding_ok
                        and composite_binding_ok
                        and tool_evidence_ok
                    )
                    if scenario.get("expected_origin_lineage"):
                        scores["origin_lineage"] = (
                            lineage_ok
                            and typed_lineage_ok
                            and task_lineage_ok
                            and plan_lineage_ok
                            and batch_lineage_ok
                            and rich_problem_lineage_ok
                            and composite_lineage_ok
                        )
                recovery_ok = (
                    state_ok
                    and transition_ok
                    and binding_ok
                    and lineage_ok
                    and after.get("state") == "no-unconsolidated-evidence"
                    and after.get("totalUnconsolidatedEvidence") == 0
                    and typed_binding_ok
                    and typed_lineage_ok
                    and typed_after_ok
                    and task_binding_ok
                    and task_lineage_ok
                    and task_after_ok
                    and plan_binding_ok
                    and plan_lineage_ok
                    and plan_after_ok
                    and batch_binding_ok
                    and batch_lineage_ok
                    and batch_after_ok
                    and rich_problem_binding_ok
                    and rich_problem_lineage_ok
                    and rich_problem_after_ok
                    and composite_binding_ok
                    and composite_lineage_ok
                    and composite_after_ok
                    and tool_evidence_ok
                )
            scores["memory_recovery"] = recovery_ok
            privacy_payloads = [compiled]
            if typed_compiled is not None:
                privacy_payloads.append(typed_compiled)
            if task_compiled is not None:
                privacy_payloads.append(task_compiled)
            if task_projection is not None:
                privacy_payloads.append(task_projection)
            if plan_compiled is not None:
                privacy_payloads.append(plan_compiled)
            if plan_projection is not None:
                privacy_payloads.append(plan_projection)
            if batch_compiled is not None:
                privacy_payloads.append(batch_compiled)
            if batch_projection is not None:
                privacy_payloads.append(batch_projection)
            if rich_problem_compiled is not None:
                privacy_payloads.append(rich_problem_compiled)
            if rich_problem_projection is not None:
                privacy_payloads.append(rich_problem_projection)
            if composite_compiled is not None:
                privacy_payloads.append(composite_compiled)
            if composite_projection is not None:
                privacy_payloads.append(composite_projection)
            if tool_compiled is not None:
                privacy_payloads.append(tool_compiled)
            if tool_history is not None:
                privacy_payloads.append(tool_history)
            if tool_projection is not None:
                privacy_payloads.append(tool_projection)
            if tool_verification is not None:
                privacy_payloads.append(tool_verification)
            privacy_canaries = [str(project), str(vault)]
            if task_secret_canary:
                privacy_canaries.append(task_secret_canary)
            if plan_secret_canary:
                privacy_canaries.append(plan_secret_canary)
            if batch_secret_canary:
                privacy_canaries.append(batch_secret_canary)
            if rich_problem_secret_canary:
                privacy_canaries.append(rich_problem_secret_canary)
            if composite_secret_canary:
                privacy_canaries.append(composite_secret_canary)
            if tool_secret_canary:
                privacy_canaries.append(tool_secret_canary)
            if tool_raw_call_id:
                privacy_canaries.append(tool_raw_call_id)
            scores["privacy_violation_rate"] = privacy_violation_rate(
                privacy_canaries, privacy_payloads
            )
            if scenario.get("expected_tool_evidence"):
                privacy_text = serialized(privacy_payloads)
                privacy_labels = [
                    "project-path",
                    "vault-path",
                    "task-secret",
                    "plan-secret",
                    "batch-secret",
                    "rich-problem-secret",
                    "composite-secret",
                    "tool-secret",
                    "tool-raw-call-id",
                ]
                leaked_labels = [
                    label
                    for label, canary in zip(privacy_labels, privacy_canaries, strict=True)
                    if canary and canary in privacy_text
                ]
                if leaked_labels:
                    failures.append(
                        "privacy canary leakage labels: " + ", ".join(leaked_labels)
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
