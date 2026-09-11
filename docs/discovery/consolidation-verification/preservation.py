"""Authenticate the unchanged tracked prefix against the actual starting revision."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = 'c717f82'
ALLOWED = {'README.md', 'docs/superpowers/specs/2026-09-11-discovery-capability-design.md'}

def main():
    revision = subprocess.check_output(['git', 'rev-parse', BASE], cwd=ROOT, text=True).strip()
    files = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', revision], cwd=ROOT, text=True).splitlines()
    hashes = {}
    for name in files:
        if name in ALLOWED:
            continue
        before = subprocess.check_output(['git', 'show', f'{revision}:{name}'], cwd=ROOT)
        after = (ROOT / name).read_bytes()
        assert after == before, name
        hashes[name] = hashlib.sha256(after).hexdigest()
    text = (ROOT / 'docs/ROADMAP.md').read_text()
    policy = json.loads(text.split('<!-- roadmap-policy:v1 -->')[1].split('```json')[1].split('```')[0])
    assert policy['completedThrough'] == 'D02'
    assert next(u for u in policy['units'] if u['id'] == 'D03')['implemented'] is False
    assert not any(u['id'] == 'D04' for u in policy['units'])
    manifest = json.loads((ROOT / 'ergo-sandbox/tests/fixtures/mapping/manifest.json').read_text())
    anchor = '41729240759244bd3edc833e31359bfe50d8430f0b5a2e27ebc727706363cd10'
    assert anchor in json.dumps(manifest)
    print(json.dumps({'baselineRevision': revision, 'unchangedTrackedFiles': len(hashes), 'exceptions': sorted(ALLOWED), 'completedThrough': 'D02', 'D03Implemented': False, 'D04Registered': False, 'M00Anchor': anchor, 'sha256': hashes}, indent=2))

if __name__ == '__main__':
    main()
