#!/usr/bin/env python3
"""Read-only check of batch scope, frozen policy bytes, and report portability."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[3]
def git(*args):
    result = subprocess.run(['git',*args],cwd=root,capture_output=True,check=True)
    return result.stdout
assert git('branch','--show-current').decode().strip() == 'v2/batch-5'
# The baseline is the protected-scope reference; committed checkouts are its descendants.
subprocess.run(['git','merge-base','--is-ancestor','126567f','HEAD'],cwd=root,check=True)
protected = ['docs/reports/batch-1','docs/reports/batch-2','docs/reports/batch-3','docs/reports/batch-4','examples/mutants',
    'ergo-sandbox/tests/fixtures/mapping/manifest.json','ergo-sandbox/tests/fixtures/properties']
assert not git('diff','126567f','--',*protected)
pattern = rb'<!-- roadmap-policy:v1 -->.*?<!-- /roadmap-policy:v1 -->'
base = re.search(pattern,git('show','126567f:docs/ROADMAP.md'),re.S)[0]
current = re.search(pattern,(root/'docs/ROADMAP.md').read_bytes(),re.S)[0]
assert base == current
print('roadmap-policy:v1 unchanged; SHA-256:',hashlib.sha256(current).hexdigest())
print('Protected report folders, mutants and manifests unchanged.')
for path in (root/'docs/reports/batch-5').rglob('*'):
    if path.is_file():
        text = path.read_text()
        # Build paths use the portable marker. Test fixtures may contain source
        # strings with slashes but never an absolute workstation path.
        assert not re.search(r'/(?:home|Users)/[^\s"\']+|/tmp/tmp[\w-]+',text), str(path.relative_to(root))
for name in ['W02','W03']:
    report = json.loads((root/f'docs/reports/batch-5/{name}.json').read_text())
    assert report['passed']
    assert all(record['exitCode'] == 0 for record in report['commands'])
    print(name,'gate commands:',len(report['commands']),'all exited 0')
print('Branch, protected scope and report portability checks passed.')
