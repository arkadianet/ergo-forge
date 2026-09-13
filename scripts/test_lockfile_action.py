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
            with patch.object(locks.subprocess, 'run', side_effect=[SimpleNamespace(returncode=5), SimpleNamespace(returncode=0)]) as run:
                self.assertEqual(locks.verify(str(root / '*.lock.json')), 1)
                self.assertEqual(run.call_count, 2)
                first = run.call_args_list[0].args[0]
                self.assertEqual(first[:2], ['ergo-es', 'verify-lock'])
                self.assertIn('testnet', first)
                self.assertIn(str(root / 'params.json'), first)
                self.assertIn('mainnet', run.call_args_list[1].args[0])

    def test_absent_globs_sources_and_invalid_options_fail(self):
        with tempfile.TemporaryDirectory() as tmp, contextlib.redirect_stderr(io.StringIO()):
            root = Path(tmp)
            with patch.object(locks.subprocess, 'run') as run:
                self.assertEqual(locks.verify(str(root / '*.lock.json')), 1)
                lock = root / 'contract.lock.json'; lock.write_text('{}')
                self.assertEqual(locks.verify(str(lock)), 1)
                (root / 'contract.es').write_text('sigmaProp(true)')
                (root / 'contract.test.json').write_text('{"network":"moon"}')
                self.assertEqual(locks.verify(str(lock)), 1)
                run.assert_not_called()

    def test_action_keeps_latest_release_semantics(self):
        action = (ROOT / '.github/actions/test/action.yml').read_text()
        self.assertIn('default: "latest"', action)
        self.assertIn('if [ "$VERSION" = "latest" ]; then\n          gh release download --repo', action)
        self.assertIn("if: ${{ inputs.lockfile != '' }}", action)
        self.assertIn('python3 "$LOCK_ACTION_PATH/verify-lockfiles.py" "$LOCKFILES"', action)


if __name__ == '__main__':
    unittest.main()
