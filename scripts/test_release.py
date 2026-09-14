"""Offline release wiring and policy-derived changelog gates."""
from pathlib import Path
import re
import unittest

import yaml
import roadmap_gate

ROOT = Path(__file__).resolve().parent.parent


def workflow(path):
    # BaseLoader preserves GitHub's `on` key (YAML 1.1 treats it as Boolean).
    return yaml.load((ROOT / path).read_text(), Loader=yaml.BaseLoader)


class ReleaseTests(unittest.TestCase):
    def test_release_action_points_at_tagged_binary(self):
        release = workflow('.github/workflows/release.yml')
        action = workflow('.github/actions/test/action.yml')
        self.assertEqual(release['on']['push']['tags'], ['v*'])
        cli = release['jobs']['cli']
        matrix = cli['strategy']['matrix']['include']
        self.assertEqual({r['target'] for r in matrix}, {
            'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu', 'aarch64-apple-darwin'})
        self.assertEqual(cli['runs-on'], '${{ matrix.os }}')
        commands = '\n'.join(s.get('run', '') for s in cli['steps'])
        self.assertIn('cargo build --release --locked -p ergo-sandbox --bin ergo-es', commands)
        self.assertIn('cp target/release/ergo-es dist/', commands)
        self.assertIn('tar -C dist -czf "${{ matrix.asset }}" ergo-es', commands)
        self.assertIn('gh release upload "${{ github.ref_name }}" "${{ matrix.asset }}"', commands)
        self.assertTrue(any(s.get('uses', '').startswith('actions/checkout@') for s in cli['steps']))
        self.assertEqual(action['inputs']['version']['default'], 'latest')
        install = next(s for s in action['runs']['steps'] if s.get('name') == 'Install ergo-es')
        self.assertEqual(install['env']['VERSION'], '${{ inputs.version }}')
        self.assertEqual(install['if'], "${{ inputs.version != 'source' }}")
        script = install['run']
        for row in matrix:
            self.assertIn('asset="' + row['asset'] + '"', script)
        self.assertIn('if [ "$VERSION" = "latest" ]; then\n  gh release download --repo arkadianet/ergo-forge --pattern "$asset"', script)
        self.assertIn('gh release download "$VERSION" --repo arkadianet/ergo-forge --pattern "$asset"', script)
        self.assertIn('tar -xzf "$RUNNER_TEMP/ergo-es/$asset" -C "$RUNNER_TEMP/ergo-es"', script)
        self.assertIn('echo "$RUNNER_TEMP/ergo-es" >> "$GITHUB_PATH"', script)
        verify = next(s for s in action['runs']['steps'] if s.get('name') == 'Verify lockfiles')
        self.assertIn('verify-lockfiles.py', verify['run'])
        self.assertIn('verify-lock <', (ROOT / 'ergo-sandbox/src/bin/ergo-es.rs').read_text())
        self.assertRegex((ROOT / 'Cargo.toml').read_text(), r'version\s*=\s*"0\.5\.0"')

    def test_changelog_lists_every_batch(self):
        changelog = (ROOT / 'CHANGELOG.md').read_text()
        section = re.search(r'^## 0\.5\.0[^\n]*\n(.*?)(?=^## |\Z)', changelog, re.M | re.S)[1]
        policy = roadmap_gate.load_policies()[1]
        implemented = {u['id'] for u in policy['units'] if u['implemented']}
        self.assertLessEqual(implemented, set(re.findall(r'\b[WSIXE]\d{2}\b', section)))
        spec = (ROOT / 'docs/superpowers/specs/2026-09-13-forge-roadmap-v2.md').read_text()
        for batch in range(10):
            pattern = rf'^\| {batch} \| ([^|]+) \|'
            expected = re.search(pattern, spec, re.M)
            actual = re.search(pattern, section, re.M)
            self.assertIsNotNone(actual, f'batch {batch} missing')
            units = set(re.findall(r'\b[WSIXE]\d{2}\b', expected[1]))
            self.assertLessEqual(units, implemented)
            self.assertEqual(set(re.findall(r'\b[WSIXE]\d{2}\b', actual[1])), units)
        for tag in ['0.4.1', '0.4.0', '0.3.0']:
            self.assertIn('## ' + tag, changelog)


if __name__ == '__main__':
    unittest.main()
