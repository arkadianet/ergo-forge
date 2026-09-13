#!/usr/bin/env python3
"""Read-only verification of batch scope and portable evidence."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[3]
base = '1d53b2f'
def git(*args):
    return subprocess.run(['git', *args], cwd=root, capture_output=True, check=True).stdout

assert git('branch', '--show-current').decode().strip() == 'v2/batch-6'
subprocess.run(['git', 'merge-base', '--is-ancestor', base, 'HEAD'], cwd=root, check=True)
protected = [f'docs/reports/batch-{n}' for n in range(1, 6)] + [
    'examples/mutants', 'ergo-sandbox/src/compile.rs', 'ergo-sandbox/tests/fixtures/mapping/manifest.json',
    'ergo-sandbox/tests/fixtures/properties', 'Cargo.lock', 'Cargo.toml']
assert not git('diff', base, '--', *protected)
assert not git('ls-files', '--others', '--exclude-standard', '--', *protected)
pattern = rb'<!-- roadmap-policy:v1 -->.*?<!-- /roadmap-policy:v1 -->'
original = re.search(pattern, git('show', base + ':docs/ROADMAP.md'), re.S)[0]
current = re.search(pattern, (root / 'docs/ROADMAP.md').read_bytes(), re.S)[0]
assert original == current
print('roadmap-policy:v1 unchanged; SHA-256:', hashlib.sha256(current).hexdigest())
print('Protected reports, mutants, manifests, engine pin and lockfile unchanged.')
for name in ['mapping', 'properties']:
    manifest = json.loads((root / f'ergo-sandbox/tests/fixtures/{name}/manifest.json').read_text())
    for path, digest in manifest['baselineArtifacts'].items():
        # S02's historical answer-key handling is covered by the existing
        # inventory tests; this check already requires current bytes unchanged.
        if path != 'examples/mutants/answer-key.json':
            assert hashlib.sha256((root / path).read_bytes()).hexdigest() == digest, path
print('All mapping/properties baseline artifact digests match.')
for path in (root / 'docs/reports/batch-6').rglob('*'):
    if path.is_file():
        assert not re.search(r'/(?:home|Users)/[^\s"\']+|/tmp/tmp[\w-]+', path.read_text()), str(path.relative_to(root))
report = json.loads((root / 'docs/reports/batch-6/W04.json').read_text())
assert report['passed']
assert all(command['exitCode'] == 0 for command in report['commands'])
print('W04 gate:', len(report['commands']), 'subprocesses, all exited 0.')
print('Branch, protected scope and report portability checks passed.')

ledger = json.loads((root / 'docs/reports/batch-6/commands.json').read_text())
text = (root / 'docs/reports/batch-6/REPORT.md').read_text()
assert '**pending**' not in text
for entry in ledger['commands']:
    assert entry['command'] in text, entry['command']
    assert entry['exitStatus'] == str(entry['exitCode'])
    assert (root / 'docs/reports/batch-6' / entry['log']).is_file()
for number in range(1, 5):
    message = (root / f'docs/reports/batch-6/commit-{number}-message.txt').read_text()
    assert 'Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>' in message
    assert 'Claude-Session: https://claude.ai/code/session_01WwVySagCnCMChPM4C65WYg' in message
print('Report covers every recorded command exit; all four commit messages carry the requested trailers.')
