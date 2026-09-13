"""Read-only checks against batch 4's base; never re-bless fixture digests."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = 'a235eed'

def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)

assert git('branch', '--show-current').decode().strip() == 'v2/batch-4'
protected = [
    'docs/reports/batch-3', 'docs/reports/batch-2', 'examples/mutants',
    'ergo-sandbox/src/identity.rs', 'ergo-sandbox/src/claim.rs',
    'ergo-sandbox/src/evidence/replay.rs', 'ergo-sandbox/src/evidence/promotion.rs',
    'ergo-sandbox/src/audit',
    'ergo-sandbox/tests/fixtures/mapping', 'ergo-sandbox/tests/fixtures/properties',
    'ergo-sandbox/tests/fixtures/evidence',
    'Cargo.toml', 'Cargo.lock', 'ergo-sandbox/Cargo.toml',
]
checked = {}
for prefix in protected:
    paths = git('ls-tree', '-r', '--name-only', BASE, '--', prefix).decode().splitlines()
    assert paths, f'protected path has no baseline: {prefix}'
    for path in paths:
        before = git('show', f'{BASE}:{path}')
        now = (ROOT / path).read_bytes()
        assert now == before, f'protected bytes changed: {path}'
        checked[path] = hashlib.sha256(now).hexdigest()
pattern = rb'<!-- roadmap-policy:v1 -->.*?<!-- /roadmap-policy:v1 -->'
before = re.search(pattern, git('show', f'{BASE}:docs/ROADMAP.md'), re.S)[0]
after = re.search(pattern, (ROOT / 'docs/ROADMAP.md').read_bytes(), re.S)[0]
assert before == after, 'hash-frozen roadmap-policy:v1 changed'
print(json.dumps({'baseRevision': BASE, 'branch': 'v2/batch-4', 'protectedFilesChecked': len(checked), 'roadmapPolicyV1Sha256': hashlib.sha256(after).hexdigest(), 'protectedFilesSha256': checked}, indent=2))
