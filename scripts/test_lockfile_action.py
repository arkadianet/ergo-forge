import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('locks', ROOT / '.github/actions/test/verify-lockfiles.py')
locks = importlib.util.module_from_spec(spec)
previous_bytecode = sys.dont_write_bytecode
sys.dont_write_bytecode = True
spec.loader.exec_module(locks)
sys.dont_write_bytecode = previous_bytecode


class LockfileActionTests(unittest.TestCase):
    def test_multiple_locks_use_external_inputs_and_fail_on_drift(self):
        with tempfile.TemporaryDirectory() as tmp, contextlib.redirect_stdout(io.StringIO()):
            root = Path(tmp)
            for name in ['contract', 'second']:
                (root / (name + '.lock.json')).write_text('{}')
                (root / (name + '.es')).write_text('sigmaProp(true)')
            (root / 'params.json').write_text('{}')
            (root / 'contract.test.json').write_text(json.dumps({'network': 'testnet', 'treeVersion': 3}))
            help_ok = SimpleNamespace(returncode=0, stdout='USAGE:\n  ergo-es verify-lock <contract.lock.json>', stderr='')
            with patch.object(locks.subprocess, 'run', side_effect=[help_ok, SimpleNamespace(returncode=5), SimpleNamespace(returncode=0)]) as run:
                self.assertEqual(locks.verify(str(root / '*.lock.json')), 1)
                self.assertEqual(run.call_count, 3)
                self.assertEqual(run.call_args_list[0].args[0], ['ergo-es', '--help'])
                first = run.call_args_list[1].args[0]
                self.assertEqual(first[:2], ['ergo-es', 'verify-lock'])
                self.assertIn('testnet', first)
                self.assertIn(str(root / 'params.json'), first)
                self.assertIn('mainnet', run.call_args_list[2].args[0])

    def test_binary_without_verify_lock_fails_with_compatibility_error(self):
        with tempfile.TemporaryDirectory() as tmp, contextlib.redirect_stderr(io.StringIO()) as err:
            root = Path(tmp)
            (root / 'contract.lock.json').write_text('{}')
            (root / 'contract.es').write_text('sigmaProp(true)')
            old_help = SimpleNamespace(returncode=0, stdout='USAGE:\n  ergo-es match <source.es|treeHex> <treeHex>', stderr='')
            with patch.object(locks.subprocess, 'run', side_effect=[old_help]) as run:
                self.assertEqual(locks.verify(str(root / 'contract.lock.json')), 1)
                self.assertEqual(run.call_count, 1)
            self.assertIn('version: source', err.getvalue())
            with patch.object(locks.subprocess, 'run', side_effect=OSError('not found')):
                self.assertEqual(locks.verify(str(root / 'contract.lock.json')), 1)

    def test_absent_globs_sources_and_invalid_options_fail(self):
        with tempfile.TemporaryDirectory() as tmp, contextlib.redirect_stderr(io.StringIO()):
            root = Path(tmp)
            help_ok = SimpleNamespace(returncode=0, stdout='ergo-es verify-lock', stderr='')
            with patch.object(locks.subprocess, 'run', return_value=help_ok) as run:
                self.assertEqual(locks.verify(str(root / '*.lock.json')), 1)
                lock = root / 'contract.lock.json'; lock.write_text('{}')
                self.assertEqual(locks.verify(str(lock)), 1)
                (root / 'contract.es').write_text('sigmaProp(true)')
                (root / 'contract.test.json').write_text('{"network":"moon"}')
                self.assertEqual(locks.verify(str(lock)), 1)
                for call in run.call_args_list:
                    self.assertEqual(call.args[0], ['ergo-es', '--help'])

    def test_action_keeps_latest_release_semantics(self):
        action = (ROOT / '.github/actions/test/action.yml').read_text()
        self.assertIn('default: "latest"', action)
        self.assertIn('if [ "$VERSION" = "latest" ]; then\n          gh release download --repo', action)
        self.assertIn("if: ${{ inputs.lockfile != '' }}", action)
        self.assertIn('python3 "$LOCK_ACTION_PATH/verify-lockfiles.py" "$LOCKFILES"', action)


if __name__ == '__main__':
    unittest.main()
