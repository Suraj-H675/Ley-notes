#!/usr/bin/env python3
"""Probe installed Codex/Claude MCP compatibility with Ley without a model turn.

This is a host-version-sensitive validation lane, not deterministic CI. It creates a disposable Ley
project plus isolated host configuration, captures the real stdio MCP initialize/inventory traffic,
prints a compact JSON report, and deletes the temporary state on exit.
"""

from __future__ import annotations

import argparse
import json
import os
import select
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_LEY_BIN = REPO_ROOT / "target" / "debug" / "ley"


def run(
    args: list[str],
    *,
    env: dict[str, str] | None = None,
    timeout: int = 30,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        args,
        env=env,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )
    if check and result.returncode != 0:
        raise RuntimeError(
            f"command failed ({result.returncode}): {' '.join(args)}\n"
            f"stdout: {result.stdout.strip()}\n"
            f"stderr: {result.stderr.strip()}"
        )
    return result


def proxy_main() -> int:
    """Transparent line-oriented stdio proxy used only by the disposable host probes."""

    log_path = Path(os.environ["LEY_MCP_PROBE_LOG"])
    ley_bin = os.environ["LEY_MCP_PROBE_LEY"]
    project = os.environ["LEY_MCP_PROBE_PROJECT"]
    lock = threading.Lock()

    def log(prefix: str, line: str) -> None:
        with lock:
            with log_path.open("a", encoding="utf-8") as handle:
                handle.write(prefix)
                handle.write(line)
                if not line.endswith("\n"):
                    handle.write("\n")
                handle.flush()

    child = subprocess.Popen(
        [ley_bin, "mcp", project],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
        env=os.environ.copy(),
    )

    def client_to_server() -> None:
        assert child.stdin is not None
        for line in sys.stdin:
            log("C> ", line)
            child.stdin.write(line)
            child.stdin.flush()
        try:
            child.stdin.close()
        except BrokenPipeError:
            pass

    def stderr_to_log() -> None:
        assert child.stderr is not None
        for line in child.stderr:
            log("E> ", line)

    threading.Thread(target=client_to_server, daemon=True).start()
    threading.Thread(target=stderr_to_log, daemon=True).start()
    assert child.stdout is not None
    for line in child.stdout:
        log("S> ", line)
        sys.stdout.write(line)
        sys.stdout.flush()
    return child.wait()


def parse_traffic(path: Path) -> dict[str, Any]:
    records: list[tuple[str, dict[str, Any]]] = []
    if path.exists():
        for raw in path.read_text(encoding="utf-8").splitlines():
            if len(raw) < 4 or raw[1:3] != "> ":
                continue
            try:
                payload = json.loads(raw[3:])
            except json.JSONDecodeError:
                continue
            if isinstance(payload, dict):
                records.append((raw[0], payload))

    initialize = next(
        (
            payload
            for direction, payload in records
            if direction == "C" and payload.get("method") == "initialize"
        ),
        {},
    )
    initialize_id = initialize.get("id")
    initialize_response = next(
        (
            payload
            for direction, payload in records
            if direction == "S" and payload.get("id") == initialize_id
        ),
        {},
    )
    methods = [
        str(payload.get("method"))
        for direction, payload in records
        if direction == "C" and isinstance(payload.get("method"), str)
    ]

    def inventory_result(method: str, key: str) -> tuple[bool, int | None]:
        request = next(
            (
                payload
                for direction, payload in records
                if direction == "C" and payload.get("method") == method
            ),
            None,
        )
        if not isinstance(request, dict) or request.get("id") is None:
            return False, None
        response = next(
            (
                payload
                for direction, payload in records
                if direction == "S" and payload.get("id") == request.get("id")
            ),
            None,
        )
        if not isinstance(response, dict) or "error" in response:
            return False, None
        response_result = response.get("result")
        if not isinstance(response_result, dict):
            return False, None
        values = response_result.get(key)
        if not isinstance(values, list):
            return False, None
        return True, len(values)

    tools_ok, tools_count = inventory_result("tools/list", "tools")
    resources_ok, resources_count = inventory_result("resources/list", "resources")
    templates_ok, templates_count = inventory_result(
        "resources/templates/list", "resourceTemplates"
    )
    result = initialize_response.get("result", {})
    if not isinstance(result, dict):
        result = {}
    initialize_succeeded = (
        bool(initialize)
        and isinstance(initialize_response, dict)
        and "error" not in initialize_response
        and bool(result)
        and isinstance(result.get("protocolVersion"), str)
        and bool(result.get("protocolVersion"))
    )
    params = initialize.get("params", {})
    if not isinstance(params, dict):
        params = {}
    client_info = params.get("clientInfo", {})
    if not isinstance(client_info, dict):
        client_info = {}
    server_info = result.get("serverInfo", {})
    if not isinstance(server_info, dict):
        server_info = {}
    return {
        "initializeObserved": bool(initialize),
        "initializeSucceeded": initialize_succeeded,
        "clientProtocolVersion": params.get("protocolVersion"),
        "serverProtocolVersion": result.get("protocolVersion"),
        "clientInfo": client_info,
        "serverInfo": server_info,
        "initializedNotificationObserved": "notifications/initialized" in methods,
        "toolsListObserved": "tools/list" in methods,
        "toolsListSucceeded": tools_ok,
        "toolCount": tools_count,
        "resourcesListObserved": "resources/list" in methods,
        "resourcesListSucceeded": resources_ok,
        "resourceCount": resources_count,
        "resourceTemplatesListObserved": "resources/templates/list" in methods,
        "resourceTemplatesListSucceeded": templates_ok,
        "resourceTemplateCount": templates_count,
    }


def create_probe_project(base: Path, ley_bin: Path) -> tuple[Path, dict[str, str]]:
    project = base / "project"
    vault = base / "vault"
    config = base / "ley-xdg-config"
    cache = base / "ley-xdg-cache"
    project.mkdir()
    vault.mkdir()
    config.mkdir()
    cache.mkdir()
    (project / "README.md").write_text(
        "# MCP host compatibility probe\n\nhost_compat_probe_marker\n",
        encoding="utf-8",
    )
    env = os.environ.copy()
    env["XDG_CONFIG_HOME"] = str(config)
    env["XDG_CACHE_HOME"] = str(cache)
    run(
        [
            str(ley_bin),
            "init",
            str(project),
            "--name",
            "MCP host compatibility probe",
            "--capture",
            "structured",
            "--json",
        ],
        env=env,
    )
    run(
        [str(ley_bin), "bind", str(project), "--vault", str(vault), "--json"],
        env=env,
    )
    run([str(ley_bin), "ingest", str(project), "--json"], env=env)
    return project, {
        "XDG_CONFIG_HOME": str(config),
        "XDG_CACHE_HOME": str(cache),
    }


def server_env(
    *,
    log: Path,
    ley_bin: Path,
    project: Path,
    ley_env: dict[str, str],
) -> dict[str, str]:
    return {
        "LEY_MCP_PROBE_LOG": str(log),
        "LEY_MCP_PROBE_LEY": str(ley_bin),
        "LEY_MCP_PROBE_PROJECT": str(project),
        **ley_env,
    }


def probe_claude(
    *,
    base: Path,
    claude_bin: str,
    ley_bin: Path,
    project: Path,
    ley_env: dict[str, str],
) -> dict[str, Any]:
    version = run([claude_bin, "--version"], check=False).stdout.strip()
    home = base / "claude-home"
    xdg_config = base / "claude-xdg-config"
    xdg_cache = base / "claude-xdg-cache"
    claude_config = base / "claude-config"
    home.mkdir()
    xdg_config.mkdir()
    xdg_cache.mkdir()
    claude_config.mkdir()
    env = os.environ.copy()
    env.update(
        {
            "HOME": str(home),
            "XDG_CONFIG_HOME": str(xdg_config),
            "XDG_CACHE_HOME": str(xdg_cache),
            "CLAUDE_CONFIG_DIR": str(claude_config),
        }
    )
    log = base / "claude-traffic.log"
    child_env = server_env(
        log=log, ley_bin=ley_bin, project=project, ley_env=ley_env
    )
    add = [claude_bin, "mcp", "add", "--scope", "user", "leyprobe"]
    for key, value in child_env.items():
        add.extend(["-e", f"{key}={value}"])
    add.extend(["--", sys.executable, str(Path(__file__).resolve()), "--proxy"])
    run(add, env=env)
    log.write_text("", encoding="utf-8")
    health = run([claude_bin, "mcp", "get", "leyprobe"], env=env, check=False)
    traffic = parse_traffic(log)
    connected = (
        health.returncode == 0
        and "Connected" in health.stdout
        and traffic["initializeObserved"]
        and traffic["initializeSucceeded"]
        and isinstance(traffic["clientProtocolVersion"], str)
        and bool(traffic["clientProtocolVersion"])
        and isinstance(traffic["serverProtocolVersion"], str)
        and bool(traffic["serverProtocolVersion"])
        and traffic["toolsListSucceeded"]
        and int(traffic["toolCount"] or 0) > 0
    )
    return {
        "available": True,
        "version": version,
        "connected": connected,
        "healthCheckReturnCode": health.returncode,
        "traffic": traffic,
    }


def app_server_read_id(
    process: subprocess.Popen[str], wanted: int, *, timeout: int = 30
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    deadline = time.monotonic() + timeout
    seen: list[dict[str, Any]] = []
    assert process.stdout is not None
    while time.monotonic() < deadline:
        ready, _, _ = select.select(
            [process.stdout], [], [], max(0.0, deadline - time.monotonic())
        )
        if not ready:
            break
        line = process.stdout.readline()
        if not line:
            break
        try:
            payload = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(payload, dict):
            continue
        seen.append(payload)
        if payload.get("id") == wanted:
            return payload, seen
    raise RuntimeError(f"Codex app-server timed out waiting for request id {wanted}")


def app_server_send(process: subprocess.Popen[str], payload: dict[str, Any]) -> None:
    assert process.stdin is not None
    process.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
    process.stdin.flush()


def probe_codex(
    *,
    base: Path,
    codex_bin: str,
    ley_bin: Path,
    project: Path,
    ley_env: dict[str, str],
) -> dict[str, Any]:
    version = run([codex_bin, "--version"], check=False).stdout.strip()
    codex_home = base / "codex-home"
    user_home = base / "codex-user-home"
    codex_home.mkdir()
    user_home.mkdir()
    env = os.environ.copy()
    env.update({"CODEX_HOME": str(codex_home), "HOME": str(user_home)})
    log = base / "codex-traffic.log"
    child_env = server_env(
        log=log, ley_bin=ley_bin, project=project, ley_env=ley_env
    )
    add = [codex_bin, "mcp", "add"]
    for key, value in child_env.items():
        add.extend(["--env", f"{key}={value}"])
    add.extend(
        ["leyprobe", "--", sys.executable, str(Path(__file__).resolve()), "--proxy"]
    )
    run(add, env=env)
    log.write_text("", encoding="utf-8")

    process = subprocess.Popen(
        [codex_bin, "app-server", "--stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
        env=env,
    )
    try:
        app_server_send(
            process,
            {
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": {"name": "ley-mcp-host-compat", "version": "1.0"},
                    "capabilities": {"experimentalApi": True},
                },
            },
        )
        app_server_initialize, initialize_seen = app_server_read_id(process, 1)
        app_server_send(
            process,
            {
                "id": 2,
                "method": "mcpServerStatus/list",
                "params": {"detail": "full", "limit": 20},
            },
        )
        status, status_seen = app_server_read_id(process, 2)
    finally:
        try:
            if process.stdin is not None:
                process.stdin.close()
        except BrokenPipeError:
            pass
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()

    data = status.get("result", {}).get("data", [])
    row = next(
        (
            item
            for item in data
            if isinstance(item, dict) and item.get("name") == "leyprobe"
        ),
        {},
    )
    traffic = parse_traffic(log)
    tools = row.get("tools", {}) if isinstance(row, dict) else {}
    resources = row.get("resources", {}) if isinstance(row, dict) else {}
    templates = row.get("resourceTemplates", {}) if isinstance(row, dict) else {}
    app_server_initialized = "result" in app_server_initialize
    connected = (
        bool(row)
        and app_server_initialized
        and traffic["initializeObserved"]
        and traffic["initializeSucceeded"]
        and isinstance(traffic["clientProtocolVersion"], str)
        and bool(traffic["clientProtocolVersion"])
        and isinstance(traffic["serverProtocolVersion"], str)
        and bool(traffic["serverProtocolVersion"])
        and traffic["toolsListSucceeded"]
        and int(traffic["toolCount"] or 0) > 0
        and isinstance(tools, dict)
        and len(tools) > 0
    )
    return {
        "available": True,
        "version": version,
        "connected": connected,
        "appServerInitialized": app_server_initialized,
        "statusNotificationsObserved": len(
            [
                item
                for item in initialize_seen + status_seen
                if item.get("method") == "mcpServer/status/updated"
            ]
        ),
        "toolCount": len(tools) if isinstance(tools, dict) else 0,
        "resourceCount": traffic["resourceCount"],
        "resourceTemplateCount": traffic["resourceTemplateCount"],
        "traffic": traffic,
    }


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description=(
            "Probe installed Codex/Claude MCP negotiation with a disposable Ley project without "
            "starting an LLM/model turn."
        )
    )
    result.add_argument("--ley-bin", default=str(DEFAULT_LEY_BIN))
    result.add_argument("--codex-bin", default="codex")
    result.add_argument("--claude-bin", default="claude")
    result.add_argument("--output", help="Optional JSON report path")
    result.add_argument(
        "--require-all",
        action="store_true",
        help="Exit non-zero unless both installed host probes connect and inventory Ley.",
    )
    return result


def main() -> int:
    args = parser().parse_args()
    ley_bin = Path(args.ley_bin).resolve()
    if not ley_bin.is_file():
        raise RuntimeError(
            f"Ley binary not found at {ley_bin}; build it before running this host probe"
        )

    codex = shutil.which(args.codex_bin)
    claude = shutil.which(args.claude_bin)
    with tempfile.TemporaryDirectory(prefix="ley-mcp-host-compat-") as directory:
        base = Path(directory)
        project, ley_env = create_probe_project(base, ley_bin)
        report: dict[str, Any] = {
            "schemaVersion": 1,
            "modelInvocationAttempted": False,
            "temporaryConfigurationOnly": True,
            "leyBinary": str(ley_bin),
            "codex": (
                probe_codex(
                    base=base,
                    codex_bin=codex,
                    ley_bin=ley_bin,
                    project=project,
                    ley_env=ley_env,
                )
                if codex
                else {"available": False, "connected": False}
            ),
            "claude": (
                probe_claude(
                    base=base,
                    claude_bin=claude,
                    ley_bin=ley_bin,
                    project=project,
                    ley_env=ley_env,
                )
                if claude
                else {"available": False, "connected": False}
            ),
        }

    output = json.dumps(report, indent=2, sort_keys=True)
    print(output)
    if args.output:
        Path(args.output).write_text(output + "\n", encoding="utf-8")
    if args.require_all and not (
        report["codex"].get("connected") and report["claude"].get("connected")
    ):
        return 1
    return 0


if __name__ == "__main__":
    if "--proxy" in sys.argv:
        raise SystemExit(proxy_main())
    raise SystemExit(main())
