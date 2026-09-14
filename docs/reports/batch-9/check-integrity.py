"""Check batch scope against its base without rewriting any baseline."""
import hashlib
import json
import re
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[3]
base = 'a69af13'

def original(path):
    return subprocess.check_output(['git', 'show', f'{base}:{path}'], cwd=root)

def unchanged(path):
    assert (root / path).read_bytes() == original(path), path

for name in ['claim', 'replay', 'promotion', 'identity', 'compile', 'eval']:
    paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', base, 'ergo-sandbox/src'], cwd=root, text=True).splitlines()
    for path in paths:
        if Path(path).name == f'{name}.rs':
            unchanged(path)
for prefix in ['ergo-sandbox/src/audit', 'examples/incidents', 'examples/contracts/recipes',
               'ergo-sandbox/tests/fixtures', 'examples/mutants', 'examples/tests']:
    paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', base, prefix], cwd=root, text=True).splitlines()
    for path in paths:
        unchanged(path)
for batch in range(1, 9):
    paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', base, f'docs/reports/batch-{batch}'], cwd=root, text=True).splitlines()
    for path in paths:
        unchanged(path)
for path in ['ergo-sandbox/src/compose.rs', 'ergo-sandbox/src/hunt.rs', 'ergo-sandbox/src/rent.rs',
             'ergo-sandbox/src/lockfile.rs', 'ergo-sandbox/tests/recognize.rs', 'ui/index.html']:
    unchanged(path)
roadmap = (root / 'docs/ROADMAP.md').read_bytes()
v1 = rb'<!-- roadmap-policy:v1 -->.*?<!-- /roadmap-policy:v1 -->'
assert re.search(v1, roadmap, re.S)[0] == re.search(v1, original('docs/ROADMAP.md'), re.S)[0]
branch = subprocess.check_output(['git', 'branch', '--show-current'], cwd=root, text=True).strip()
assert branch == 'v2/batch-9', branch
for p in (root / 'docs/reports/batch-9').rglob('*'):
    if p.is_file() and p.suffix in ('.json', '.md', '.log', '.txt'):
        assert '/home/' not in p.read_text(), p.relative_to(root)
print('Protected sources, historical recipes/suites/mutants/fixtures, batch 1–8 reports and v1 policy are byte-identical to a69af13.')
print('Branch v2/batch-9; reports contain no absolute workstation paths.')
print('v1 policy SHA-256:', hashlib.sha256(re.search(v1, roadmap, re.S)[0]).hexdigest())
