import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).parents[1] / 'scripts/verify-project-orientation-native'
LOADER = importlib.machinery.SourceFileLoader('ley_native_acceptance', str(SCRIPT))
SPEC = importlib.util.spec_from_loader(LOADER.name, LOADER)
HARNESS = importlib.util.module_from_spec(SPEC)
LOADER.exec_module(HARNESS)


class NativeRuntimeTests(unittest.TestCase):
    def test_plugin_qualified_ley_skill_is_selected_for_native_invocation(self):
        with tempfile.TemporaryDirectory() as scratch:
            home = Path(scratch) / 'codex-home'
            skill = home / 'plugins/cache/ley/ley-memory/0.2.1/skills/ley/SKILL.md'
            skill.parent.mkdir(parents=True)
            skill.write_text('name: ley\n')
            response = {'data': [{'skills': [{
                'name': 'ley-memory:ley',
                'pluginId': 'ley-memory@ley',
                'enabled': True,
                'path': str(skill),
            }]}]}

            selected = HARNESS.select_ley_skill(response, home)

            self.assertEqual(selected, {
                'name': 'ley-memory:ley',
                'path': str(skill.resolve()),
            })

    def test_ley_skill_selection_requires_the_exact_plugin_and_private_path(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            home = root / 'codex-home'
            skill = home / 'plugins/cache/ley/ley-memory/0.2.1/skills/ley/SKILL.md'
            skill.parent.mkdir(parents=True)
            skill.write_text('name: ley\n')
            unrelated = {
                'name': 'other:ley',
                'pluginId': 'other@ley',
                'enabled': True,
                'path': str(skill),
            }
            with self.assertRaisesRegex(RuntimeError, 'found 0'):
                HARNESS.select_ley_skill({'data': [{'skills': [unrelated]}]}, home)

            escaped = {
                'name': 'ley-memory:ley',
                'pluginId': 'ley-memory@ley',
                'enabled': True,
                'path': str(root / 'outside' / 'SKILL.md'),
            }
            (root / 'outside').mkdir()
            (root / 'outside/SKILL.md').write_text('name: ley\n')
            with self.assertRaisesRegex(RuntimeError, 'unavailable'):
                HARNESS.select_ley_skill({'data': [{'skills': [escaped]}]}, home)

    def test_existing_ipc_tree_is_private_before_launch(self):
        with tempfile.TemporaryDirectory() as scratch:
            home = Path(scratch) / 'codex-home'
            old_socket_directory = home / 'tmp/old-socket'
            old_socket_directory.mkdir(parents=True)
            for directory in [home, home / 'tmp', old_socket_directory]:
                directory.chmod(0o755)
            records = HARNESS.prepare_private_codex_home(home)
            for record in records:
                metadata = Path(record['path']).stat()
                self.assertEqual(metadata.st_uid, os.getuid())
                self.assertEqual(stat.S_IMODE(metadata.st_mode), 0o700)
            self.assertEqual(stat.S_IMODE(old_socket_directory.stat().st_mode), 0o700)

    def test_symlink_runtime_is_rejected_without_changing_target(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            home = root / 'codex-home'
            home.mkdir()
            target = root / 'unrelated'
            target.mkdir(mode=0o755)
            target.chmod(0o755)
            (home / 'tmp').symlink_to(target, target_is_directory=True)
            with self.assertRaises(OSError):
                HARNESS.prepare_private_codex_home(home)
            self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o755)
            alias = root / 'home-alias'
            alias.symlink_to(home, target_is_directory=True)
            with self.assertRaises(OSError):
                HARNESS.prepare_private_codex_home(alias)

    def test_host_children_create_private_temp_directories(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            home = root / 'codex-home'
            proof = root / 'proof'
            proof.mkdir()
            command = [sys.executable, '-c',
                       'import os; from pathlib import Path; '
                       'Path(os.environ["TMPDIR"], "new-socket").mkdir(mode=0o777); '
                       'Path(os.environ["XDG_RUNTIME_DIR"], "new-runtime").mkdir(mode=0o777)']
            masked_socket = {'path': '/tmp/codex-daemon-test', 'exists': True,
                             'uid': os.getuid(), 'mode': '0000', 'isDirectory': True}
            with patch.object(HARNESS, 'shared_runtime_status', return_value=masked_socket):
                host = HARNESS.Host(command, {**os.environ, 'CODEX_HOME': str(home)}, root, proof)
            host.close()
            self.assertEqual(host.proc.returncode, 0)
            self.assertEqual(json.loads((proof / 'shared-runtime-preflight.json').read_text()),
                             masked_socket)
            for directory in [home / 'tmp/new-socket', home / 'tmp/xdg-runtime/new-runtime']:
                metadata = directory.stat()
                self.assertEqual(metadata.st_uid, os.getuid())
                self.assertEqual(stat.S_IMODE(metadata.st_mode), 0o700)
            receipt = json.loads((proof / 'runtime-before-launch.json').read_text())
            self.assertEqual(receipt['childUmask'], '0077')
            self.assertEqual(receipt['turnSandboxPolicy']['type'], 'workspaceWrite')
            self.assertTrue(all(row['mode'] == '0700' for row in receipt['directories']))

    def test_shared_socket_diagnostic_is_independent_of_the_nested_command_boundary(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            masked = root / 'shared-socket'
            masked.mkdir(mode=0o700)
            masked.chmod(0)
            original = HARNESS.shared_runtime_status(masked)
            self.assertEqual(original['uid'], os.getuid())
            self.assertEqual(original['mode'], '0000')
            self.assertEqual(stat.S_IMODE(masked.stat().st_mode), 0)
            masked.chmod(0o700)

    def test_shared_socket_symlink_is_rejected(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            target = root / 'target'
            target.mkdir(mode=0o700)
            alias = root / 'alias'
            alias.symlink_to(target, target_is_directory=True)
            self.assertTrue(HARNESS.shared_runtime_status(alias)['isSymlink'])
            self.assertFalse(HARNESS.shared_runtime_status(target)['isSymlink'])

    def test_native_turn_uses_external_sandbox_without_a_second_bubblewrap(self):
        host = object.__new__(HARNESS.Host)
        host.model = 'gpt-6-luna'
        host.effort = 'xhigh'
        host.turn_sandbox_policy = {'type': 'externalSandbox', 'networkAccess': 'restricted'}
        requests = []

        def request(method, params):
            requests.append((method, params))
            return {'turn': {'id': 'test-turn'}}

        host.request = request
        host.receive = lambda deadline: {
            'method': 'turn/completed',
            'params': {'turn': {'id': 'test-turn', 'status': 'completed'}},
        }
        skill = {'name': 'ley-memory:ley', 'path': '/private/codex-home/skills/ley/SKILL.md'}
        host.turn('test-thread', '$ley', skill)
        self.assertEqual(len(requests), 1)
        method, params = requests[0]
        self.assertEqual(method, 'turn/start')
        self.assertEqual(params['input'], [
            {'type': 'text', 'text': '$ley'},
            {'type': 'skill', 'name': 'ley-memory:ley', 'path': '/private/codex-home/skills/ley/SKILL.md'},
        ])
        self.assertEqual(params['sandboxPolicy'], {
            'type': 'externalSandbox', 'networkAccess': 'restricted',
        })
        self.assertEqual(params['approvalPolicy'], 'never')

    def test_external_sandbox_requires_a_real_readonly_socket_mask(self):
        snapshot = {'path': '/tmp/codex-daemon-1000', 'exists': True,
                    'uid': os.geteuid(), 'mode': '0000',
                    'isDirectory': True, 'isSymlink': False}
        readonly = '100 99 0:42 / /tmp/codex-daemon-1000 ro,nosuid - tmpfs tmpfs rw\n'
        writable = '100 99 0:42 / /tmp/codex-daemon-1000 rw,nosuid - tmpfs tmpfs rw\n'
        self.assertEqual(HARNESS.nested_turn_sandbox_policy(snapshot, readonly), {
            'type': 'externalSandbox', 'networkAccess': 'restricted',
        })
        for evidence in (writable, '', '100 99 0:42 / /tmp ro - tmpfs tmpfs rw\n'):
            self.assertEqual(HARNESS.nested_turn_sandbox_policy(snapshot, evidence), {
                'type': 'workspaceWrite', 'networkAccess': False,
            })
        self.assertEqual(HARNESS.nested_turn_sandbox_policy({**snapshot, 'mode': '0700'}, readonly), {
            'type': 'workspaceWrite', 'networkAccess': False,
        })


if __name__ == '__main__':
    unittest.main()
