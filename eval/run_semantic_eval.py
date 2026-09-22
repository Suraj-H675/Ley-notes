#!/usr/bin/env python3
"""Run Ley's opt-in model-enabled semantic retrieval evaluation.

This lane deliberately uses the caller's verified local semantic-model cache.
It never downloads a model. The normal deterministic run_eval.py corpus remains
cache-isolated and does not depend on this environment-specific capability.
"""

from __future__ import annotations

import json
import os
import tempfile
from pathlib import Path

import run_eval as base


FIXTURE = Path(__file__).parent / "fixtures" / "semantic_model_scenario.json"


def memory_search(project: Path, query: str, max_results: int, max_tokens: int) -> dict[str, object]:
    result = base.mcp_call(
        project,
        "ley_search_memory",
        {
            "query": query,
            "maxResults": max_results,
            "maxTokens": max_tokens,
        },
    )
    if not isinstance(result, dict):
        raise RuntimeError("ley_search_memory returned no structured result")
    return result


def result_for_path(payload: dict[str, object], artifact_path: str) -> dict[str, object]:
    for item in payload.get("results", []):
        if not isinstance(item, dict):
            continue
        citation = item.get("citation", {})
        if isinstance(citation, dict) and citation.get("artifactPath") == artifact_path:
            return item
    return {}


def retrieval(payload: dict[str, object]) -> dict[str, object]:
    value = payload.get("retrieval", {})
    return value if isinstance(value, dict) else {}


def semantic_index_files(vault: Path, project_id: str) -> list[Path]:
    directory = (
        vault
        / ".ley"
        / "agent-memory"
        / "projects"
        / project_id
        / "semantic-index"
    )
    return sorted(directory.glob("semantic-index-v1-*.json"))


def main() -> int:
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    status = base.cli_json(["semantic", "status", "--json"])
    if not isinstance(status, dict) or status.get("state") != "ready":
        raise SystemExit(
            "model-enabled semantic eval requires Ley's exact pinned local model to be "
            "already installed and checksum-verified; no download was attempted"
        )

    with tempfile.TemporaryDirectory(prefix="ley-semantic-eval-") as temporary:
        root = Path(temporary)
        project = root / "project"
        vault = root / "vault"
        config = root / "config"
        empty_cache = root / "empty-cache"
        for path in (project, vault, config, empty_cache):
            path.mkdir()

        base.EVAL_ENV["XDG_CONFIG_HOME"] = str(config)
        base.write_project_files(project, fixture["project_files"])
        base.init_project(project, str(fixture["goal"]), vault)

        query = str(fixture["query"])
        gold_path = str(fixture["gold_path"])
        gold_marker = str(fixture["gold_marker"])
        max_results = int(fixture["max_results"])
        max_tokens = int(fixture["max_tokens"])
        minimum_similarity = float(fixture["minimum_gold_similarity"])

        base.EVAL_ENV["XDG_CACHE_HOME"] = str(empty_cache)
        try:
            lexical = memory_search(project, query, max_results, max_tokens)
        finally:
            base.EVAL_ENV.pop("XDG_CACHE_HOME", None)

        hybrid = memory_search(project, query, max_results, max_tokens)
        lexical_gold = result_for_path(lexical, gold_path)
        hybrid_gold = result_for_path(hybrid, gold_path)
        hybrid_ranking = (
            hybrid_gold.get("ranking", {})
            if isinstance(hybrid_gold.get("ranking"), dict)
            else {}
        )

        doctor = base.cli_json(["doctor", str(project), "--json"])
        identity = doctor.get("identity", {}) if isinstance(doctor, dict) else {}
        project_id = str(identity.get("projectId", ""))
        if not project_id.startswith("prj_"):
            raise RuntimeError("doctor returned no stable project ID")
        index_files = semantic_index_files(vault, project_id)
        if len(index_files) != 1:
            raise RuntimeError(
                f"expected one derived semantic index after first hybrid search, found {len(index_files)}"
            )
        index_path = index_files[0]
        index_directory = index_path.parent
        original_index_document = json.loads(index_path.read_text(encoding="utf-8"))

        index_path.write_bytes(b"{corrupt-derived-index")
        os.chmod(index_path, 0o600)
        os.chmod(index_directory, 0o500)
        try:
            corrupt_fallback = memory_search(project, query, max_results, max_tokens)
        finally:
            os.chmod(index_directory, 0o700)

        repaired = memory_search(project, query, max_results, max_tokens)
        repaired_index_document = json.loads(index_path.read_text(encoding="utf-8"))
        repaired_gold = result_for_path(repaired, gold_path)

        storage_path = project / gold_path
        storage_path.write_text(
            storage_path.read_text(encoding="utf-8")
            + str(fixture["updated_gold_suffix"]),
            encoding="utf-8",
        )
        base.run(["ingest", str(project), "--json"])
        updated = memory_search(project, query, max_results, max_tokens)
        updated_gold = result_for_path(updated, gold_path)
        updated_indexes = semantic_index_files(vault, project_id)

        hybrid_citation = (
            hybrid_gold.get("citation", {})
            if isinstance(hybrid_gold.get("citation"), dict)
            else {}
        )
        updated_citation = (
            updated_gold.get("citation", {})
            if isinstance(updated_gold.get("citation"), dict)
            else {}
        )
        fallback_retrieval = retrieval(corrupt_fallback)
        checks = {
            "verified-model-ready": status.get("state") == "ready",
            "lexical-mode-explicit":
                retrieval(lexical).get("mode") == "lexical"
                and retrieval(lexical).get("artifactContextMode") == "lexical"
                and "not installed"
                in str(retrieval(lexical).get("artifactContextFallbackReason", "")),
            "lexical-top-budget-misses-gold": not lexical_gold,
            "hybrid-mode-explicit":
                retrieval(hybrid).get("mode") == "hybrid"
                and retrieval(hybrid).get("artifactContextMode") == "hybrid",
            "hybrid-recovers-gold":
                bool(hybrid_gold)
                and hybrid_ranking.get("semanticRank") == 1
                and float(hybrid_ranking.get("semanticSimilarity", -1.0))
                >= minimum_similarity
                and gold_marker in json.dumps(hybrid_gold),
            "corrupt-index-falls-back":
                fallback_retrieval.get("artifactContextMode") == "lexical"
                and "snapshot-bound semantic index could not be built"
                in str(fallback_retrieval.get("artifactContextFallbackReason", "")),
            "repaired-index-restores-hybrid":
                retrieval(repaired).get("artifactContextMode") == "hybrid"
                and bool(repaired_gold)
                and repaired_index_document.get("binding")
                == original_index_document.get("binding"),
            "stale-index-not-reused":
                retrieval(updated).get("artifactContextMode") == "hybrid"
                and bool(updated_gold)
                and len(updated_indexes) == 2
                and hybrid_citation.get("artifactSnapshotId")
                != updated_citation.get("artifactSnapshotId"),
        }

        outputs = [status, lexical, hybrid, corrupt_fallback, repaired, updated]
        serialized = json.dumps(outputs, sort_keys=True)
        privacy_ok = all(
            str(path) not in serialized
            for path in (
                root,
                project,
                vault,
                config,
                empty_cache,
                index_directory,
                index_path,
            )
        )
        checks["privacy-paths-hidden"] = privacy_ok

        failures = [name for name, passed in checks.items() if not passed]
        print("=== Ley model-enabled semantic retrieval evaluation ===")
        for name, passed in checks.items():
            print(f"{name}: {'PASS' if passed else 'FAIL'}")
        print(
            "lexical-results:",
            [item.get("title") for item in lexical.get("results", []) if isinstance(item, dict)],
        )
        print(
            "hybrid-results:",
            [item.get("title") for item in hybrid.get("results", []) if isinstance(item, dict)],
        )
        print("derived-index-count-after-reingest:", len(updated_indexes))
        if failures:
            print("FAILED:", ", ".join(failures))
            return 1
        print("PASS: real local hybrid retrieval, corrupt-index fallback/repair, and stale-index rebuild")
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
