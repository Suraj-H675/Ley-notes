#!/usr/bin/env python3

import copy
import json
import stat
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


sys.path.insert(0, str(Path(__file__).resolve().parent))

import run_agent_task_eval as agent_eval
from run_eval import write_project_files


class AgentTaskEvalTests(unittest.TestCase):
    def fixture(self, fixture_id: str) -> dict[str, object]:
        return next(
            fixture
            for fixture in agent_eval.load_fixtures()
            if fixture["id"] == fixture_id
        )

    def test_secret_retry_fixture_materializes_without_checked_in_answer(self) -> None:
        raw = self.fixture("prior-retry-delay-contract")
        raw_text = str(raw)
        self.assertIn("{retry_schedule}", raw_text)
        self.assertNotIn("_oracle_expected", raw_text)
        self.assertNotIn("oracle_expected", raw_text)

        first = agent_eval.materialize_fixture(raw, bytes.fromhex("11" * 32))
        second = agent_eval.materialize_fixture(raw, bytes.fromhex("22" * 32))

        self.assertNotIn("{retry_schedule}", str(first["prior_memory"]))
        self.assertNotEqual(
            first["_oracle_expected"],
            second["_oracle_expected"],
        )
        expected = first["_oracle_expected"]
        self.assertIsInstance(expected, list)
        schedule = expected[2]
        self.assertEqual(schedule, sorted(schedule))
        self.assertEqual(len(schedule), 4)
        self.assertEqual(expected[3], schedule + [schedule[-1], schedule[-1]])

    def test_context_marker_leakage_in_task_is_rejected(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("prior-retry-delay-contract"),
            bytes.fromhex("33" * 32),
        )
        marker = str(fixture["context_markers"][0])
        fixture["task"] = str(fixture["task"]) + " " + marker
        with self.assertRaisesRegex(RuntimeError, "historical context"):
            agent_eval.validate_fixture_does_not_leak_context(fixture)

    def test_context_marker_leakage_in_filename_is_rejected(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("prior-retry-delay-contract"),
            bytes.fromhex("34" * 32),
        )
        marker = str(fixture["context_markers"][0])
        fixture["project_files"][marker] = "visible\n"
        with self.assertRaisesRegex(RuntimeError, "historical context"):
            agent_eval.validate_fixture_does_not_leak_context(fixture)

    def test_fixture_schema_rejects_unknown_allowed_file(self) -> None:
        fixture = copy.deepcopy(self.fixture("prior-label-normalization-contract"))
        agent_eval.validate_fixture_schema(fixture, 1)
        fixture["allowed_changed_files"] = ["missing.py"]
        with self.assertRaisesRegex(RuntimeError, "not initial project files"):
            agent_eval.validate_fixture_schema(fixture, 1)

    def test_fixture_schema_requires_task_family(self) -> None:
        fixture = copy.deepcopy(self.fixture("prior-label-normalization-contract"))
        fixture.pop("task_family")
        with self.assertRaisesRegex(RuntimeError, "task_family"):
            agent_eval.validate_fixture_schema(fixture, 1)

    def test_fixture_schema_rejects_invalid_revision_state_or_marker_overlap(self) -> None:
        fixture = copy.deepcopy(self.fixture("prior-label-normalization-contract"))
        fixture["prior_revision_state"] = "future"
        with self.assertRaisesRegex(RuntimeError, "prior_revision_state"):
            agent_eval.validate_fixture_schema(fixture, 1)

        fixture = copy.deepcopy(self.fixture("prior-label-normalization-contract"))
        marker = fixture["context_markers"][0]
        fixture["forbidden_context_markers"] = [marker]
        with self.assertRaisesRegex(RuntimeError, "both required and forbidden"):
            agent_eval.validate_fixture_schema(fixture, 1)

    def test_divergent_fixture_git_state_is_two_sided_without_source_changes(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("divergent-feature-flag-contract"),
            bytes.fromhex("4c" * 32),
        )
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            write_project_files(project, fixture["project_files"])
            agent_eval.git_run(project, ["init", "-b", "main"])
            base = agent_eval.git_commit_all(project, "fixture")
            before = agent_eval.snapshot_project_tree(project)

            agent_eval.prepare_fixture_git_state(project, fixture)

            self.assertEqual(agent_eval.git_run(project, ["branch", "--show-current"]), "main")
            main = agent_eval.git_run(project, ["rev-parse", "main"])
            experiment = agent_eval.git_run(
                project, ["rev-parse", "ley-eval-prior-divergent"]
            )
            self.assertNotEqual(main, experiment)
            self.assertEqual(
                agent_eval.git_run(project, ["merge-base", main, experiment]),
                base,
            )
            self.assertEqual(agent_eval.snapshot_project_tree(project), before)

    def test_fixture_schema_requires_exactly_one_hidden_oracle_mode(self) -> None:
        fixture = copy.deepcopy(self.fixture("prior-label-normalization-contract"))
        fixture["oracle_script"] = "raise SystemExit(0)\n"
        with self.assertRaisesRegex(RuntimeError, "exactly one"):
            agent_eval.validate_fixture_schema(fixture, 1)

        fixture.pop("oracle_probe")
        fixture.pop("oracle_script")
        fixture.pop("oracle_expected")
        with self.assertRaisesRegex(RuntimeError, "exactly one"):
            agent_eval.validate_fixture_schema(fixture, 1)

    def test_fixture_schema_bounds_hidden_oracle_script(self) -> None:
        fixture = copy.deepcopy(self.fixture("prior-label-normalization-contract"))
        fixture.pop("oracle_probe")
        fixture.pop("oracle_expected")
        fixture["oracle_script"] = "#" * (agent_eval.MAX_ORACLE_SCRIPT_BYTES + 1)
        with self.assertRaisesRegex(RuntimeError, "exceeds"):
            agent_eval.validate_fixture_schema(fixture, 1)

    def test_duplicate_fixture_ids_are_rejected(self) -> None:
        fixture = self.fixture("prior-label-normalization-contract")
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "fixtures.jsonl"
            line = json.dumps(fixture, separators=(",", ":"))
            path.write_text(line + "\n" + line + "\n", encoding="utf-8")
            original = agent_eval.FIXTURES
            agent_eval.FIXTURES = path
            try:
                with self.assertRaisesRegex(RuntimeError, "duplicates fixture id"):
                    agent_eval.load_fixtures()
            finally:
                agent_eval.FIXTURES = original

    def test_snapshot_diff_detects_unauthorized_file_even_after_git_commit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            write_project_files(project, {"tracked.txt": "base\n"})
            agent_eval.git_run(project, ["init", "-b", "main"])
            agent_eval.git_commit_all(project, "base")
            before = agent_eval.snapshot_project_tree(project)
            (project / "extra.txt").write_text("first\n", encoding="utf-8")
            agent_eval.git_run(project, ["add", "extra.txt"])
            agent_eval.git_run(
                project,
                [
                    "-c",
                    "user.name=Ley Eval",
                    "-c",
                    "user.email=ley-eval@example.invalid",
                    "commit",
                    "-m",
                    "hide unauthorized file",
                ],
            )
            after = agent_eval.snapshot_project_tree(project)
            files, material = agent_eval.compare_project_snapshots(before, after)
            self.assertEqual(files, ["extra.txt"])
            self.assertIn(b"first\n", material)

    def test_task_constraints_reject_non_target_changes(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("prior-retry-delay-contract"),
            bytes.fromhex("44" * 32),
        )
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            write_project_files(
                project,
                {
                    str(path): str(body)
                    for path, body in fixture["project_files"].items()
                },
            )
            before = agent_eval.snapshot_project_tree(project)
            (project / "test_retry.py").write_text("# altered\n", encoding="utf-8")
            after = agent_eval.snapshot_project_tree(project)
            changed, _ = agent_eval.compare_project_snapshots(before, after)
            result = agent_eval.evaluate_task_constraints(
                fixture,
                project,
                before,
                after,
                changed,
                10,
            )
            self.assertFalse(result["passed"])
            self.assertEqual(result["unexpectedChangedFiles"], ["test_retry.py"])
            self.assertEqual(result["unchangedFileMismatches"], ["test_retry.py"])

    def test_allowed_symlink_is_rejected_before_visible_tests(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("prior-retry-delay-contract"),
            bytes.fromhex("45" * 32),
        )
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = root / "project"
            project.mkdir()
            write_project_files(
                project,
                {
                    str(path): str(body)
                    for path, body in fixture["project_files"].items()
                },
            )
            before = agent_eval.snapshot_project_tree(project)
            outside = root / "outside.py"
            outside.write_text("def retry_delays(attempts): return []\n", encoding="utf-8")
            (project / "retry.py").unlink()
            (project / "retry.py").symlink_to(outside)
            after = agent_eval.snapshot_project_tree(project)
            changed, _ = agent_eval.compare_project_snapshots(before, after)
            result = agent_eval.evaluate_task_constraints(
                fixture,
                project,
                before,
                after,
                changed,
                10,
            )
            self.assertFalse(result["passed"])
            self.assertEqual(result["symlinkPaths"], ["retry.py"])
            self.assertTrue(result["visibleTestsSkipped"])

    def test_oracle_probe_runs_in_project_only_sandbox(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("prior-retry-delay-contract"),
            bytes.fromhex("46" * 32),
        )
        schedule = fixture["_oracle_expected"][2]
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            write_project_files(
                project,
                {
                    "retry.py": (
                        "def retry_delays(attempts: int) -> list[int]:\n"
                        f"    schedule = {schedule!r}\n"
                        "    return schedule[:attempts] if attempts <= 4 "
                        "else schedule + [schedule[-1]] * (attempts - 4)\n"
                    )
                },
            )
            result = agent_eval.run_oracle_probe(
                fixture,
                project,
                10,
                False,
            )
            self.assertTrue(result["passed"])

            (project / "retry.py").write_text(
                "from pathlib import Path\n"
                "def retry_delays(attempts: int) -> list[int]:\n"
                "    leaked = Path('/etc/passwd').exists()\n"
                "    return [1] if leaked else []\n",
                encoding="utf-8",
            )
            result = agent_eval.run_oracle_probe(
                fixture,
                project,
                10,
                False,
            )
            self.assertFalse(result["passed"])

    def test_oracle_script_verifies_multi_file_behavior_in_project_only_sandbox(self) -> None:
        fixture = copy.deepcopy(self.fixture("prior-label-normalization-contract"))
        fixture.pop("oracle_probe")
        fixture.pop("oracle_expected")
        fixture["project_files"] = {
            "math_ops.py": "def add(left, right): return left - right\n",
            "service.py": "from math_ops import add\ndef total(values): return add(values[0], values[1])\n",
        }
        fixture["allowed_changed_files"] = ["math_ops.py", "service.py"]
        fixture["visible_test_command"] = ["python3", "-c", "import service"]
        fixture["oracle_script"] = (
            "from pathlib import Path\n"
            "assert not Path('/etc/passwd').exists()\n"
            "import math_ops, service\n"
            "assert math_ops.add(3, 4) == 7\n"
            "assert service.total([5, 8]) == 13\n"
        )
        fixture["oracle_solution_files"] = {
            "math_ops.py": "def add(left, right): return left + right\n",
        }
        agent_eval.validate_fixture_schema(fixture, 1)

        with tempfile.TemporaryDirectory() as temporary:
            validation = agent_eval.validate_script_oracle_reference(
                fixture,
                Path(temporary),
                10,
            )
            self.assertEqual(validation["initialStatus"], "failed")
            self.assertEqual(validation["referenceStatus"], "passed")
            self.assertEqual(validation["referenceSolutionFileCount"], 1)

        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            write_project_files(project, fixture["project_files"])
            write_project_files(project, fixture["oracle_solution_files"])
            result = agent_eval.run_hidden_oracle(fixture, project, 10, False)
            self.assertTrue(result["passed"])
            self.assertNotIn("assert service.total", "\n".join(
                path.read_text(encoding="utf-8")
                for path in project.rglob("*.py")
            ))

            (project / "math_ops.py").write_text(
                "def add(left, right): return left - right\n",
                encoding="utf-8",
            )
            result = agent_eval.run_hidden_oracle(fixture, project, 10, False)
            self.assertFalse(result["passed"])

    def test_nonzero_runner_exit_is_a_failed_attempt_not_an_exception(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            result = agent_eval.run_external_agent(
                [sys.executable, "-c", "raise SystemExit(7)"],
                project,
                "task",
                10,
                "baseline",
                [],
                [],
                False,
            )
            self.assertFalse(result["completed"])
            self.assertFalse(result["timedOut"])
            self.assertEqual(result["exitCode"], 7)

    def test_runner_environment_does_not_inherit_home_by_default(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            result = agent_eval.run_external_agent(
                [
                    sys.executable,
                    "-c",
                    (
                        "import os; "
                        "print(os.environ.get('HOME', '<missing>')); "
                        "print(os.environ.get('PWD', ''))"
                    ),
                ],
                project,
                "task",
                10,
                "baseline",
                [],
                [],
                True,
            )
            lines = bytes(result["_stdout"]).decode("utf-8").splitlines()
            self.assertEqual(lines[0], "/home/runner")
            self.assertEqual(lines[1], "/workspace")

    def test_runner_timeout_is_counted_as_failed_attempt(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            result = agent_eval.run_external_agent(
                [sys.executable, "-c", "import time; time.sleep(2)"],
                project,
                "task",
                1,
                "baseline",
                [],
                [],
                False,
            )
            self.assertFalse(result["completed"])
            self.assertTrue(result["timedOut"])

    def test_runner_pid_namespace_kills_setsid_descendant_on_timeout(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            code = (
                "import os,pathlib,time\n"
                "pid=os.fork()\n"
                "if pid==0:\n"
                "    pid2=os.fork()\n"
                "    if pid2>0: os._exit(0)\n"
                "    os.setsid()\n"
                "    time.sleep(1.5)\n"
                "    pathlib.Path('escaped.txt').write_text('escaped', encoding='utf-8')\n"
                "    os._exit(0)\n"
                "time.sleep(5)\n"
            )
            result = agent_eval.run_external_agent(
                [sys.executable, "-c", code],
                project,
                "task",
                1,
                "baseline",
                [],
                [],
                False,
            )
            self.assertTrue(result["timedOut"])
            import time
            time.sleep(2)
            self.assertFalse((project / "escaped.txt").exists())

    def test_runner_cannot_see_host_repository(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            host_repo = str(agent_eval.REPO_ROOT)
            code = (
                "import pathlib,sys; "
                "pathlib.Path('visibility.txt').write_text("
                "'visible' if pathlib.Path(sys.argv[1]).exists() else 'hidden', "
                "encoding='utf-8')"
            )
            result = agent_eval.run_external_agent(
                [sys.executable, "-c", code, host_repo],
                project,
                "task",
                10,
                "baseline",
                [],
                [],
                False,
            )
            self.assertTrue(result["completed"])
            self.assertEqual(
                (project / "visibility.txt").read_text(encoding="utf-8"),
                "hidden",
            )

    def test_sandbox_output_limit_is_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            result = agent_eval.run_sandboxed_project_command(
                [
                    "python3",
                    "-c",
                    f"import sys; sys.stdout.write('x'*{agent_eval.MAX_SANDBOX_OUTPUT_BYTES + 65536})",
                ],
                project,
                10,
                False,
            )
            self.assertFalse(result["completed"])
            self.assertTrue(result["outputLimitExceeded"])

    def test_external_runner_sandbox_mounts_only_public_tls_trust_material(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            command, _ = agent_eval.runner_sandbox_command(
                ["python3", "-c", "pass"],
                Path(temporary),
                [],
                [],
            )
        joined = "\n".join(command)
        for path in (
            "/etc/ssl/certs",
            "/etc/ssl/cert.pem",
            "/etc/ca-certificates/extracted",
            "/etc/pki/ca-trust/extracted",
            "/etc/pki/tls/certs",
        ):
            self.assertIn(f"--ro-bind-try\n{path}\n{path}", joined)
        for broad_path in ("/etc/ssl", "/etc/ca-certificates", "/etc/pki"):
            self.assertNotIn(
                f"--ro-bind-try\n{broad_path}\n{broad_path}",
                joined,
            )

    def test_snapshot_does_not_follow_directory_symlink(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = root / "project"
            outside = root / "outside"
            project.mkdir()
            outside.mkdir()
            (outside / "secret.txt").write_text("secret", encoding="utf-8")
            (project / "linked").symlink_to(outside, target_is_directory=True)
            snapshot = agent_eval.snapshot_project_tree(project)
            self.assertEqual(snapshot["linked"][0], "symlink")
            self.assertNotIn("linked/secret.txt", snapshot)

    def test_normal_result_redacts_secret_bearing_changed_path(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("prior-label-normalization-contract"),
            bytes.fromhex("49" * 32),
        )
        secret = "runtime-secret-91-37"

        def fake_runner(
            command,
            project,
            prompt,
            timeout_seconds,
            variant,
            inherited_env_names,
            read_only_mounts,
            capture_raw,
        ):
            (project / f"{secret}.txt").write_text("leak", encoding="utf-8")
            return {
                "completed": False,
                "timedOut": False,
                "outputLimitExceeded": False,
                "exitCode": 1,
                "seconds": 0.0,
                "stdoutBytes": 0,
                "stderrBytes": 0,
                "stdoutSha256": agent_eval.sha256_bytes(b""),
                "stderrSha256": agent_eval.sha256_bytes(b""),
            }

        with tempfile.TemporaryDirectory() as temporary:
            with mock.patch.object(
                agent_eval,
                "run_external_agent",
                side_effect=fake_runner,
            ):
                result = agent_eval.execute_variant(
                    fixture,
                    Path(temporary),
                    ["ignored"],
                    [],
                    [],
                    "baseline",
                    10,
                    8,
                    500,
                    False,
                )
        rendered = json.dumps(result, sort_keys=True)
        self.assertNotIn(secret, rendered)
        self.assertNotIn("changedFiles", result)
        self.assertGreaterEqual(result["changedFileCount"], 1)

    def test_variant_order_balances_four_arm_comparison(self) -> None:
        self.assertEqual(
            agent_eval.variants_for_repetition("all", "baseline", 1),
            ("baseline", "handoff", "minimal", "ley"),
        )
        self.assertEqual(
            agent_eval.variants_for_repetition("all", "baseline", 2),
            ("handoff", "minimal", "ley", "baseline"),
        )
        self.assertEqual(
            agent_eval.variants_for_repetition("all", "ley", 1),
            ("ley", "baseline", "handoff", "minimal"),
        )
        self.assertEqual(
            agent_eval.variants_for_repetition("both", "baseline", 2),
            ("ley", "baseline"),
        )
        self.assertEqual(
            agent_eval.variants_for_repetition("minimal", "baseline", 3),
            ("minimal",),
        )

    def test_task_schedule_rotates_each_task_across_repetitions_independent_of_suite_size(self) -> None:
        starts = []
        for repetition in range(1, 5):
            schedule_index = agent_eval.comparison_schedule_index(1, repetition)
            starts.append(
                agent_eval.variants_for_repetition(
                    "all", "baseline", schedule_index
                )[0]
            )
        self.assertEqual(starts, ["baseline", "handoff", "minimal", "ley"])

        fourth_task_starts = []
        for repetition in range(1, 5):
            schedule_index = agent_eval.comparison_schedule_index(4, repetition)
            fourth_task_starts.append(
                agent_eval.variants_for_repetition(
                    "all", "baseline", schedule_index
                )[0]
            )
        self.assertEqual(
            fourth_task_starts,
            ["ley", "baseline", "handoff", "minimal"],
        )

    def test_select_fixtures_preserves_requested_order_and_validation_default(self) -> None:
        fixtures = [
            {"id": "alpha"},
            {"id": "beta"},
            {"id": "gamma"},
        ]
        self.assertEqual(
            [item["id"] for item in agent_eval.select_fixtures(
                fixtures,
                ["gamma", "alpha"],
                False,
                default_all=False,
            )],
            ["gamma", "alpha"],
        )
        self.assertEqual(
            [item["id"] for item in agent_eval.select_fixtures(
                fixtures,
                [],
                False,
                default_all=True,
            )],
            ["alpha", "beta", "gamma"],
        )

    def test_select_fixtures_rejects_ambiguous_or_duplicate_selection(self) -> None:
        fixtures = [{"id": "alpha"}, {"id": "beta"}]
        with self.assertRaisesRegex(RuntimeError, "cannot be combined"):
            agent_eval.select_fixtures(
                fixtures,
                ["alpha"],
                True,
                default_all=False,
            )
        with self.assertRaisesRegex(RuntimeError, "duplicate"):
            agent_eval.select_fixtures(
                fixtures,
                ["alpha", "alpha"],
                False,
                default_all=False,
            )
        with self.assertRaisesRegex(RuntimeError, "unknown agent task"):
            agent_eval.select_fixtures(
                fixtures,
                ["missing"],
                False,
                default_all=False,
            )

    def test_simple_context_baselines_preserve_required_historical_markers(self) -> None:
        seed = bytes.fromhex("45" * 32)
        for raw in agent_eval.load_fixtures():
            fixture = agent_eval.materialize_fixture(raw, seed)
            with self.subTest(task=fixture["id"]):
                markers = [str(value) for value in fixture["context_markers"]]
                forbidden = [
                    str(value)
                    for value in fixture.get("forbidden_context_markers", [])
                ]
                handoff = agent_eval.render_handoff(fixture)
                minimal = agent_eval.render_minimal_brief(fixture)
                for marker in markers:
                    self.assertIn(marker.lower(), handoff.lower())
                    self.assertIn(marker.lower(), minimal.lower())
                for marker in forbidden:
                    self.assertTrue(
                        marker.lower() in handoff.lower()
                        or marker.lower() in minimal.lower()
                    )
                if forbidden:
                    self.assertTrue(
                        any(marker.lower() in handoff.lower() for marker in forbidden)
                    )
                    self.assertTrue(
                        any(marker.lower() in minimal.lower() for marker in forbidden)
                    )
                self.assertIn("human handoff", handoff.lower())
                self.assertIn("benchmark baseline", minimal.lower())
                self.assertLessEqual(len(minimal), len(handoff))

    def test_task_pass_requires_all_gates(self) -> None:
        good_runner = {"completed": True}
        good_constraints = {"passed": True}
        good_oracle = {"passed": True}
        self.assertTrue(
            agent_eval.task_attempt_passed(
                good_runner,
                good_constraints,
                good_oracle,
                True,
            )
        )
        for runner, constraints, oracle, stable in (
            ({"completed": False}, good_constraints, good_oracle, True),
            (good_runner, {"passed": False}, good_oracle, True),
            (good_runner, good_constraints, {"passed": False}, True),
            (good_runner, good_constraints, good_oracle, False),
        ):
            self.assertFalse(
                agent_eval.task_attempt_passed(
                    runner,
                    constraints,
                    oracle,
                    stable,
                )
            )

    def test_oracle_pass_and_overall_task_failure_are_recorded_separately(self) -> None:
        fields = agent_eval.evaluation_outcome_fields(
            task_passed=False,
            hidden_oracle_status="passed",
        )
        self.assertEqual(fields["taskStatus"], "blocked")
        self.assertEqual(fields["sessionStatus"], "paused")
        verifications = {
            item["kind"]: item
            for item in fields["verifications"]
        }
        self.assertEqual(verifications["agent-task-eval"]["status"], "failed")
        self.assertEqual(verifications["hidden-oracle"]["status"], "passed")
        self.assertIn(
            "hidden oracle passed",
            verifications["hidden-oracle"]["summary"].lower(),
        )
        self.assertNotIn(
            "oracle failed",
            verifications["agent-task-eval"]["summary"].lower(),
        )

    def test_skipped_oracle_is_not_recorded_as_failed(self) -> None:
        fields = agent_eval.evaluation_outcome_fields(
            task_passed=False,
            hidden_oracle_status="skipped",
        )
        verifications = {
            item["kind"]: item
            for item in fields["verifications"]
        }
        self.assertEqual(verifications["agent-task-eval"]["status"], "failed")
        self.assertEqual(verifications["hidden-oracle"]["status"], "skipped")
        self.assertIn(
            "skipped",
            verifications["hidden-oracle"]["summary"].lower(),
        )
        self.assertNotIn(
            "failed",
            verifications["hidden-oracle"]["summary"].lower(),
        )

    def test_hidden_oracle_aggregate_separates_skipped_from_failed(self) -> None:
        summary = agent_eval.summarize_hidden_oracles(
            [
                {"hiddenOracleStatus": "passed"},
                {"hiddenOracleStatus": "failed"},
                {"hiddenOracleStatus": "skipped"},
            ]
        )
        self.assertEqual(summary["attemptedCount"], 2)
        self.assertEqual(summary["passedCount"], 1)
        self.assertEqual(summary["failedCount"], 1)
        self.assertEqual(summary["skippedCount"], 1)
        self.assertEqual(summary["passRateAmongAttempted"], 0.5)

    def test_audit_bundle_contains_expected_review_artifacts(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("prior-retry-delay-contract"),
            bytes.fromhex("47" * 32),
        )
        report = {"schemaVersion": 1, "taskId": fixture["id"]}
        payload = {
            "prompt": "prompt\n",
            "context": "context\n",
            "runnerStdout": b"runner-out\n",
            "runnerStderr": b"",
            "oracleStdout": "oracle-out\n",
            "oracleStderr": "",
            "diffMaterial": b"diff",
            "projectSnapshot": b"snapshot",
            "changedFiles": ["retry.py"],
            "constraintDetails": {"passed": True},
        }
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "audit"
            agent_eval.write_audit_bundle(
                destination,
                [fixture],
                ["runner", "--flag"],
                report,
                [
                    {
                        "taskId": fixture["id"],
                        "taskFamily": fixture["task_family"],
                        "repetition": 1,
                        "variant": "ley",
                        "payload": payload,
                    }
                ],
            )
            self.assertTrue((destination / "materialized-fixture.json").is_file())
            self.assertTrue((destination / "runner-command.json").is_file())
            self.assertTrue((destination / "report.json").is_file())
            run_dir = destination / "repetition-001-ley"
            for name in (
                "prompt.txt",
                "context.txt",
                "runner.stdout",
                "runner.stderr",
                "oracle.stdout",
                "oracle.stderr",
                "diff-material.bin",
                "project-after.tar.gz",
                "changed-files.json",
                "constraint-details.json",
            ):
                self.assertTrue((run_dir / name).is_file(), name)

    def test_multi_task_audit_bundle_uses_numbered_task_directories(self) -> None:
        seed = bytes.fromhex("4b" * 32)
        fixtures = [
            agent_eval.materialize_fixture(
                self.fixture("changed-display-name-requirement"), seed
            ),
            agent_eval.materialize_fixture(
                self.fixture("resume-cache-key-migration"), seed
            ),
        ]
        payload = {
            "prompt": "prompt\n",
            "context": "",
            "runnerStdout": b"",
            "runnerStderr": b"",
            "oracleStdout": "",
            "oracleStderr": "",
            "diffMaterial": b"diff",
            "projectSnapshot": b"snapshot",
            "changedFiles": [],
            "constraintDetails": {"passed": True},
        }
        records = [
            {
                "taskId": fixtures[0]["id"],
                "taskFamily": fixtures[0]["task_family"],
                "repetition": 1,
                "variant": "baseline",
                "payload": payload,
            },
            {
                "taskId": fixtures[1]["id"],
                "taskFamily": fixtures[1]["task_family"],
                "repetition": 1,
                "variant": "baseline",
                "payload": payload,
            },
        ]
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "audit"
            agent_eval.write_audit_bundle(
                destination,
                fixtures,
                ["runner"],
                {"schemaVersion": 3},
                records,
            )
            self.assertTrue((destination / "materialized-fixtures.json").is_file())
            for index, fixture in enumerate(fixtures, start=1):
                task_dir = destination / f"task-{index:03d}"
                self.assertEqual(
                    (task_dir / "task-id.txt").read_text(encoding="utf-8").strip(),
                    fixture["id"],
                )
                self.assertTrue(
                    (task_dir / "repetition-001-baseline" / "prompt.txt").is_file()
                )

    def test_grouped_summaries_expose_family_regressions(self) -> None:
        results = [
            {
                "taskId": "stale-1",
                "taskFamily": "stale-memory",
                "variant": "baseline",
                "taskPassed": True,
                "hiddenOracleStatus": "passed",
                "runner": {"seconds": 1.0},
                "context": None,
            },
            {
                "taskId": "stale-1",
                "taskFamily": "stale-memory",
                "variant": "ley",
                "taskPassed": False,
                "hiddenOracleStatus": "failed",
                "runner": {"seconds": 1.5},
                "context": {"contextCharacters": 120},
            },
            {
                "taskId": "resume-1",
                "taskFamily": "resume",
                "variant": "baseline",
                "taskPassed": False,
                "hiddenOracleStatus": "failed",
                "runner": {"seconds": 2.0},
                "context": None,
            },
            {
                "taskId": "resume-1",
                "taskFamily": "resume",
                "variant": "ley",
                "taskPassed": True,
                "hiddenOracleStatus": "passed",
                "runner": {"seconds": 1.0},
                "context": {"contextCharacters": 80},
            },
        ]
        per_family = agent_eval.grouped_variant_summaries(results, "taskFamily")
        self.assertEqual(agent_eval.regressed_groups(per_family), ["stale-memory"])
        self.assertEqual(per_family["resume"]["ley"]["taskPassRate"], 1.0)
        self.assertEqual(per_family["resume"]["baseline"]["taskPassRate"], 0.0)

    def test_ley_advantage_requires_beating_every_included_simpler_arm(self) -> None:
        summaries = {
            variant: agent_eval.summarize_variant_results([])
            for variant in agent_eval.COMPARISON_VARIANTS
        }
        summaries["baseline"]["taskPassRate"] = 0.25
        summaries["handoff"]["taskPassRate"] = 0.5
        summaries["minimal"]["taskPassRate"] = 0.75
        summaries["ley"]["taskPassRate"] = 1.0
        self.assertTrue(agent_eval.ley_advantage_observed(summaries))

        summaries["handoff"]["taskPassRate"] = 1.0
        self.assertFalse(agent_eval.ley_advantage_observed(summaries))

    def test_ley_advantage_assertion_rejects_hidden_task_or_family_regression(self) -> None:
        summaries = {
            variant: agent_eval.summarize_variant_results([])
            for variant in agent_eval.COMPARISON_VARIANTS
        }
        summaries["baseline"]["taskPassRate"] = 0.25
        summaries["handoff"]["taskPassRate"] = 0.5
        summaries["minimal"]["taskPassRate"] = 0.5
        summaries["ley"]["taskPassRate"] = 0.75
        self.assertTrue(
            agent_eval.ley_advantage_assertion_passed(summaries, [], [])
        )
        self.assertFalse(
            agent_eval.ley_advantage_assertion_passed(
                summaries,
                ["one-regressed-task"],
                [],
            )
        )
        self.assertFalse(
            agent_eval.ley_advantage_assertion_passed(
                summaries,
                [],
                ["stale-memory"],
            )
        )

    def test_main_runs_selected_multi_task_suite_with_balanced_arm_rotation(self) -> None:
        calls: list[tuple[str, str]] = []

        def fake_execute(
            fixture,
            root,
            command,
            inherited_env_names,
            read_only_mounts,
            variant,
            timeout_seconds,
            max_results,
            max_tokens,
            capture_audit,
        ):
            del root, command, inherited_env_names, read_only_mounts
            del timeout_seconds, max_results, max_tokens, capture_audit
            calls.append((str(fixture["id"]), variant))
            return {
                "variant": variant,
                "taskPassed": variant == "ley",
                "hiddenOracleAttempted": True,
                "hiddenOracleStatus": "passed" if variant == "ley" else "failed",
                "hiddenOraclePassed": variant == "ley",
                "hiddenOracleExitCode": 0 if variant == "ley" else 1,
                "postCheckTreeStable": True,
                "runner": {"completed": True, "seconds": 0.01},
                "constraints": {"passed": True},
                "changedFileCount": 1,
                "changedPathsSha256": "0" * 64,
                "diffBytes": 1,
                "diffSha256": "1" * 64,
                "promptSha256": "2" * 64,
                "promptCharacters": 10,
                "context": (
                    {"contextCharacters": 50}
                    if variant != "baseline"
                    else None
                ),
                "utilityObservation": None,
            }

        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "report.json"
            with mock.patch.object(agent_eval, "execute_variant", side_effect=fake_execute):
                exit_code = agent_eval.main(
                    [
                        "--task",
                        "changed-display-name-requirement",
                        "--task",
                        "resume-cache-key-migration",
                        "--runner-command",
                        "fake-runner",
                        "--output",
                        str(output),
                    ]
                )
            self.assertEqual(exit_code, 0)
            self.assertEqual(
                calls,
                [
                    ("changed-display-name-requirement", "baseline"),
                    ("changed-display-name-requirement", "handoff"),
                    ("changed-display-name-requirement", "minimal"),
                    ("changed-display-name-requirement", "ley"),
                    ("resume-cache-key-migration", "handoff"),
                    ("resume-cache-key-migration", "minimal"),
                    ("resume-cache-key-migration", "ley"),
                    ("resume-cache-key-migration", "baseline"),
                ],
            )
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(report["schemaVersion"], 3)
            self.assertEqual(report["selectedTaskCount"], 2)
            self.assertEqual(report["plannedAgentAttempts"], 8)
            self.assertEqual(
                report["taskIds"],
                ["changed-display-name-requirement", "resume-cache-key-migration"],
            )
            self.assertIsNone(report["taskId"])
            self.assertEqual(len(report["results"]), 8)
            self.assertIn(
                "changed-requirement-vs-stale-memory",
                report["comparison"]["perFamily"],
            )
            self.assertTrue(report["comparison"]["leyTaskAdvantageObserved"])

    def test_private_ley_state_is_removed_before_external_runner(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("prior-retry-delay-contract"),
            bytes.fromhex("48" * 32),
        )

        def fake_runner(
            command,
            project,
            prompt,
            timeout_seconds,
            variant,
            inherited_env_names,
            read_only_mounts,
            capture_raw,
        ):
            self.assertEqual(variant, "ley")
            self.assertFalse((project / ".ley").exists())
            config = Path(agent_eval.EVAL_ENV["XDG_CONFIG_HOME"])
            self.assertFalse(config.exists())
            return {
                "completed": False,
                "timedOut": False,
                "exitCode": 1,
                "seconds": 0.0,
                "stdoutBytes": 0,
                "stderrBytes": 0,
                "stdoutSha256": agent_eval.sha256_bytes(b""),
                "stderrSha256": agent_eval.sha256_bytes(b""),
            }

        with tempfile.TemporaryDirectory() as temporary:
            with mock.patch.object(agent_eval, "run_external_agent", side_effect=fake_runner):
                result = agent_eval.execute_variant(
                    fixture,
                    Path(temporary),
                    ["ignored"],
                    [],
                    [],
                    "ley",
                    10,
                    8,
                    500,
                    False,
                )
        self.assertFalse(result["taskPassed"])

    def test_handoff_variant_delivers_history_as_repository_file_only(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("prior-label-normalization-contract"),
            bytes.fromhex("49" * 32),
        )

        def fake_runner(
            command,
            project,
            prompt,
            timeout_seconds,
            variant,
            inherited_env_names,
            read_only_mounts,
            capture_raw,
        ):
            del command, timeout_seconds, inherited_env_names, read_only_mounts, capture_raw
            self.assertEqual(variant, "handoff")
            handoff = (project / "HANDOFF.md").read_text(encoding="utf-8")
            self.assertIn("preserve internal spaces and tabs exactly", handoff)
            self.assertNotIn("# HANDOFF", prompt)
            return {
                "completed": False,
                "timedOut": False,
                "exitCode": 1,
                "seconds": 0.0,
                "stdoutBytes": 0,
                "stderrBytes": 0,
                "stdoutSha256": agent_eval.sha256_bytes(b""),
                "stderrSha256": agent_eval.sha256_bytes(b""),
            }

        with tempfile.TemporaryDirectory() as temporary:
            with mock.patch.object(agent_eval, "run_external_agent", side_effect=fake_runner):
                result = agent_eval.execute_variant(
                    fixture,
                    Path(temporary),
                    ["ignored"],
                    [],
                    [],
                    "handoff",
                    10,
                    8,
                    500,
                    False,
                )
        self.assertEqual(result["context"]["kind"], "human-handoff-file")

    def test_minimal_variant_delivers_tiny_fixture_derived_prompt_context(self) -> None:
        fixture = agent_eval.materialize_fixture(
            self.fixture("prior-label-normalization-contract"),
            bytes.fromhex("4a" * 32),
        )

        def fake_runner(
            command,
            project,
            prompt,
            timeout_seconds,
            variant,
            inherited_env_names,
            read_only_mounts,
            capture_raw,
        ):
            del command, timeout_seconds, inherited_env_names, read_only_mounts, capture_raw
            self.assertEqual(variant, "minimal")
            self.assertFalse((project / "HANDOFF.md").exists())
            self.assertIn("# Minimal continuity brief (benchmark baseline)", prompt)
            self.assertIn("preserve internal spaces and tabs exactly", prompt)
            return {
                "completed": False,
                "timedOut": False,
                "exitCode": 1,
                "seconds": 0.0,
                "stdoutBytes": 0,
                "stderrBytes": 0,
                "stdoutSha256": agent_eval.sha256_bytes(b""),
                "stderrSha256": agent_eval.sha256_bytes(b""),
            }

        with tempfile.TemporaryDirectory() as temporary:
            with mock.patch.object(agent_eval, "run_external_agent", side_effect=fake_runner):
                result = agent_eval.execute_variant(
                    fixture,
                    Path(temporary),
                    ["ignored"],
                    [],
                    [],
                    "minimal",
                    10,
                    8,
                    500,
                    False,
                )
        self.assertEqual(
            result["context"]["kind"],
            "fixture-derived-minimal-brief-baseline",
        )

    def test_seed_export_is_private_and_exclusive(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "seed.hex"
            seed = bytes.fromhex("55" * 32)
            agent_eval.write_master_seed(path, seed)
            self.assertEqual(agent_eval.load_master_seed(path), seed)
            mode = stat.S_IMODE(path.stat().st_mode)
            self.assertEqual(mode, 0o600)
            with self.assertRaisesRegex(RuntimeError, "could not create"):
                agent_eval.write_master_seed(path, seed)


if __name__ == "__main__":
    unittest.main()
