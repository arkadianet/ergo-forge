#!/usr/bin/env python3
"""Read-only scope and frozen-artifact checks for I03."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

root = Path.cwd()
def git(*args):
    return subprocess.check_output(['git', *args], text=True).strip()
assert git('branch', '--show-current') == 'v2/batch-8'
assert git('rev-parse', 'HEAD') == git('rev-parse', '1d53b2f'), 'unexpected HEAD; commits were blocked in this session'
base = '1d53b2f'
protected = [
    'Cargo.lock', 'ergo-web/Cargo.toml',
    'ergo-sandbox/src/claim.rs', 'ergo-sandbox/src/replay.rs',
    'ergo-sandbox/src/promotion.rs', 'ergo-sandbox/src/identity.rs',
    'ergo-sandbox/src/compile.rs', 'ergo-sandbox/src/eval.rs',
    'ergo-sandbox/src/lockfile.rs', 'ergo-sandbox/src/audit',
    'ergo-sandbox/src/hunt.rs', 'ergo-sandbox/src/rent.rs',
    'ergo-sandbox/tests/fixtures/mapping', 'ergo-sandbox/tests/fixtures/properties',
    'ergo-sandbox/tests/fixtures/decompile_corpus_expectations.json',
    'ergo-web/src/routes/eval.rs', 'ergo-web/src/routes/hunt.rs',
    '.github/workflows/ci.yml',
    *[f'docs/reports/batch-{i}' for i in range(1, 8)],
]
for directory in ['ergo-sandbox/src', 'ergo-web/src']:
    for name in ['claim.rs', 'replay.rs', 'promotion.rs', 'identity.rs', 'compile.rs', 'eval.rs']:
        protected.extend(str(path) for path in Path(directory).rglob(name))
assert not git('diff', '--name-only', base, '--', *protected), 'protected tracked files changed'
assert not git('ls-files', '--others', '--exclude-standard', '--', *protected), 'new file in a protected area'
pattern = r'<!-- roadmap-policy:v1 -->.*?<!-- /roadmap-policy:v1 -->'
original = git('show', base + ':docs/ROADMAP.md')
current = (root / 'docs/ROADMAP.md').read_text()
v1 = re.search(pattern, current, re.S)[0]
assert re.search(pattern, original, re.S)[0] == v1
print('Branch v2/batch-8, HEAD 1d53b2f; Git commits were blocked by read-only metadata.')
print('Protected source, manifests, decompiler baseline and batch-1..7 reports unchanged.')
print('roadmap-policy:v1 SHA-256:', hashlib.sha256(v1.encode()).hexdigest())
report = json.loads((root / 'docs/reports/batch-8/I03.json').read_text())
assert report['passed']
for command in report['commands']:
    assert command['exitCode'] == 0
    assert hashlib.sha256(command['output'].encode()).hexdigest() == command['outputSha256']
print('I03 gate command outputs match their recorded hashes; every gate command exited 0.')
for file in (root / 'docs/reports/batch-8').rglob('*'):
    if file.is_file() and file.suffix in {'.json', '.md', '.log', '.txt'}:
        text = file.read_text()
        assert str(root) not in text and str(Path.home()) not in text, f'local path in {file.name}'
ownership = json.loads((root / 'docs/reports/batch-8/commit-files.json').read_text())
owned = [path for paths in ownership.values() for path in paths]
assert len(owned) == len(set(owned)), 'a file was assigned to multiple steps'
changed = git('diff', '--name-only', base).splitlines() + git('ls-files', '--others', '--exclude-standard').splitlines()
assert all(path in owned or path.startswith('docs/reports/batch-8/') for path in changed), 'unassigned change'
print('Reports contain no absolute repository or home paths; every change has one commit owner.')
