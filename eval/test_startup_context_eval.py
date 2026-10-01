from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest import mock

import run_startup_context_eval as startup_eval


class StartupContextEvalTests(unittest.TestCase):
    def test_fixture_manifest_resolves_referenced_and_native_cases(self) -> None:
        fixtures = startup_eval.load_fixtures()
        self.assertEqual(len(fixtures), 6)
        self.assertEqual(len({str(item["id"]) for item in fixtures}), 6)
        by_id = {str(item["id"]): item for item in fixtures}
        referenced = by_id["startup-prior-label-contract"]
        self.assertEqual(
            referenced["sourceTaskId"], "prior-label-normalization-contract"
        )
        self.assertIn("labels.py", referenced["project_files"])
        trusted = by_id["startup-trusted-learning-contract"]
        self.assertEqual(trusted["startupCase"], "trusted-learning")
        self.assertIn("trusted_learning", trusted)
        recovery = by_id["startup-current-session-recovery"]
        self.assertEqual(recovery["startupCase"], "interrupted-current-session")
        self.assertIn("interrupted_prompt", recovery)

    def test_custom_probe_oracles_match_the_documented_contracts(self) -> None:
        by_id = {str(item["id"]): item for item in startup_eval.load_fixtures()}
        trusted = by_id["startup-trusted-learning-contract"]
        trusted_inputs = trusted["oracle_probe"]["inputs"]
        self.assertEqual(
            trusted["oracle_expected"],
            ["acct::" + value.strip().lower() for value in trusted_inputs],
        )
        recovery = by_id["startup-current-session-recovery"]
        recovery_inputs = recovery["oracle_probe"]["inputs"]
        self.assertEqual(
            recovery["oracle_expected"],
            ["sha256::" + value.strip().lower() for value in recovery_inputs],
        )

    def test_stale_same_lineage_manifest_measures_old_history_as_exposure(self) -> None:
        fixture = next(
            item
            for item in startup_eval.load_fixtures()
            if item["id"] == "startup-changed-display-name"
        )
        self.assertEqual(fixture["context_markers"], [])
        self.assertEqual(
            fixture["forbidden_context_markers"],
            [
                "Trim only leading and trailing whitespace; preserve interior formatting byte-for-byte.",
                "The previous release said to preserve interior tabs and repeated spaces after trimming the edges.",
            ],
        )
        expectations = fixture["startupExpectations"]
        self.assertEqual(
            expectations["contentful"]["forbiddenMarkerLeakCount"], 2
        )
        self.assertEqual(
            expectations["guidance-only"]["forbiddenMarkerLeakCount"], 0
        )

    def test_stable_host_request_id_matches_host_adapter_algorithm(self) -> None:
        self.assertEqual(
            startup_eval.stable_host_request_id("ley-c4-debug"),
            "req_103669ed217e34907826ca98f046ff79",
        )

    def test_variant_order_alternates_without_changing_the_pair(self) -> None:
        self.assertEqual(
            startup_eval.variants_for_repetition("contentful", 1),
            ("contentful", "guidance-only"),
        )
        self.assertEqual(
            startup_eval.variants_for_repetition("contentful", 2),
            ("guidance-only", "contentful"),
        )
        self.assertEqual(
            startup_eval.variants_for_repetition("guidance-only", 1),
            ("guidance-only", "contentful"),
        )

    def test_guidance_only_startup_contains_identity_but_no_history_body(self) -> None:
        session_id = "ses_" + "1" * 32
        rendered = startup_eval.guidance_only_startup_context(session_id)
        self.assertIn(session_id, rendered)
        self.assertIn("Call ley_brief", rendered)
        self.assertIn("was not auto-injected", rendered)
        self.assertNotIn("Recent work", rendered)
        self.assertNotIn("Reviewed project learnings", rendered)

    def test_guidance_only_recovery_notice_is_body_free(self) -> None:
        session_id = "ses_" + "2" * 32
        rendered = startup_eval.guidance_only_startup_context(
            session_id,
            recovery_records=1,
        )
        self.assertIn("Recovery signal", rendered)
        self.assertIn("1 prompt/response record(s)", rendered)
        self.assertIn("Their bodies were not injected here", rendered)
        self.assertNotIn("interrupted_recovery_body_marker_8f31", rendered)

    def test_relay_telemetry_records_names_not_arguments(self) -> None:
        relay = startup_eval.McpRelay.__new__(startup_eval.McpRelay)
        relay.lock = __import__("threading").Lock()
        relay.methods = []
        relay.tool_calls = []
        relay.server_failures = 0
        relay._record(
            '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"ley_brief","arguments":{"task":"private-marker"}}}\n'
        )
        summary = relay.summary()
        self.assertEqual(summary["toolCallCounts"], {"ley_brief": 1})
        self.assertTrue(summary["briefCalled"])
        self.assertNotIn("private-marker", str(summary))

    def test_codex_mounts_contain_only_benchmark_mcp_config_and_bridge(self) -> None:
        relay = startup_eval.McpRelay.__new__(startup_eval.McpRelay)
        relay.port = 43123
        relay.token = "a" * 64
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            mounts = startup_eval.make_codex_mounts(root, relay, [])
            by_destination = {destination: source for source, destination in mounts}
            self.assertEqual(
                set(by_destination),
                {
                    "/home/runner/.codex/config.toml",
                    "/home/runner/.ley-c4/bridge.py",
                },
            )
            config = by_destination["/home/runner/.codex/config.toml"].read_text(
                encoding="utf-8"
            )
            self.assertIn("[mcp_servers.ley]", config)
            self.assertIn('command = "python3"', config)
            self.assertIn('"127.0.0.1"', config)
            self.assertIn('"43123"', config)
            self.assertIn('"' + "a" * 64 + '"', config)
            self.assertNotIn("mcp_servers.github", config)
            self.assertNotIn("mcp_servers.devspace", config)

    def test_codex_mounts_reject_operator_collision(self) -> None:
        relay = startup_eval.McpRelay.__new__(startup_eval.McpRelay)
        relay.port = 43123
        relay.token = "b" * 64
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            operator_file = root / "operator-config.toml"
            operator_file.write_text("operator", encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "collides with C4 mount"):
                startup_eval.make_codex_mounts(
                    root,
                    relay,
                    [
                        (
                            operator_file,
                            "/home/runner/.codex/config.toml",
                        )
                    ],
                )

    def test_study_source_metadata_rejects_dirty_model_run(self) -> None:
        with mock.patch.object(
            startup_eval.agent_eval,
            "git_run",
            side_effect=["1" * 40, " M eval/run_startup_context_eval.py"],
        ):
            with self.assertRaisesRegex(RuntimeError, "clean committed repository"):
                startup_eval.study_source_metadata(require_clean=True)

    def test_compact_console_report_omits_attempt_payloads(self) -> None:
        report = {
            "study": "session-start-content-vs-guidance-only",
            "source": {"headSha": "1" * 40},
            "selectedTaskCount": 2,
            "repetitions": 1,
            "plannedAgentAttempts": 4,
            "results": [{"private": "attempt-detail"}],
            "comparison": {
                "taskPassRateDeltaContentfulMinusGuidance": 0.5,
                "variantSummaries": {"contentful": {}, "guidance-only": {}},
            },
        }
        summary = startup_eval.compact_console_report(
            report,
            output=Path("/tmp/report.json"),
            audit_dir=None,
        )
        self.assertNotIn("results", summary)
        self.assertEqual(summary["output"], "/tmp/report.json")
        self.assertEqual(summary["taskPassRateDeltaContentfulMinusGuidance"], 0.5)


if __name__ == "__main__":
    unittest.main()
