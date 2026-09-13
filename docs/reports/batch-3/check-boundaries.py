import subprocess, re, hashlib, json
from pathlib import Path
base='137fc8b'
assert subprocess.check_output(['git','branch','--show-current'], text=True).strip()=='v2/batch-3'
changed=subprocess.check_output(['git','diff','--name-only',base],text=True).splitlines()
for prefix in ['docs/reports/batch-2/','examples/mutants/','ergo-sandbox/src/audit/lints/','ergo-sandbox/src/evidence/']:
 assert not any(p.startswith(prefix) for p in changed),prefix
for p in ['ergo-sandbox/src/claim.rs','ergo-sandbox/tests/fixtures/mapping/manifest.json','ergo-sandbox/tests/fixtures/properties/manifest.json','docs/security/vectors.json','docs/security/VECTORS.md']:
 assert Path(p).read_bytes()==subprocess.check_output(['git','show',base+':'+p]),p
pattern=rb'<!-- roadmap-policy:v1 -->.*?<!-- /roadmap-policy:v1 -->'
old=re.search(pattern,subprocess.check_output(['git','show',base+':docs/ROADMAP.md']),re.S)[0]
new=re.search(pattern,Path('docs/ROADMAP.md').read_bytes(),re.S)[0]
assert new==old
print('Branch v2/batch-3; protected paths unchanged against 137fc8b.')
print('roadmap-policy:v1 SHA-256:',hashlib.sha256(new).hexdigest())
print('No claim, replay, promotion, lint, catalogue, mutant, batch-2 report or manifest changes.')
