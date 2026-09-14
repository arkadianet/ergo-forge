"""Verify the batch's protected artifacts, measured identities and command logs."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

REPORT = Path(__file__).resolve().parent
ROOT = REPORT.parents[2]
BASE = 'a18999a'

def check(condition, message='condition was false'):
    """Explicit gate: unlike `assert`, this survives `python -O`."""
    if not condition:
        raise SystemExit(f'integrity check failed: {message}')


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)

def original(path):
    return git('show', f'{BASE}:{path}')

check(git('branch', '--show-current').decode().strip() == 'v2/batch-10')
for batch in range(1, 10):
    check(not git('diff', BASE, '--', f'docs/reports/batch-{batch}'))
old = original('docs/ROADMAP.md').decode()
new = (ROOT / 'docs/ROADMAP.md').read_text()
pattern = r'<!-- roadmap-policy:v1 -->.*?<!-- /roadmap-policy:v1 -->'
check(re.search(pattern, old, re.S)[0] == re.search(pattern, new, re.S)[0])
for path in ['ergo-sandbox/tests/fixtures/mapping', 'ergo-sandbox/tests/fixtures/properties',
             'ergo-sandbox/tests/fixtures/evidence/manifest.json',
             'ergo-sandbox/tests/fixtures/evidence/node-vectors',
             'ergo-sandbox/tests/fixtures/compile_corpus_subset.json',
             'ergo-sandbox/tests/fixtures/decompile_corpus_expectations.json',
             'ergo-sandbox/tests/fixtures/w05_decompile_expectations.json',
             'ergo-sandbox/src/decompile.rs', 'ergo-sandbox/src/decompile',
             'ergo-sandbox/src/evidence/claim.rs', 'ergo-sandbox/src/evidence/replay.rs']:
    check(not git('diff', BASE, '--', path), path)
changed = set(git('diff', '--name-only', BASE).decode().splitlines())
production = {p for p in changed if p.startswith(('ergo-sandbox/src/', 'ergo-web/src/'))}
check(production == {'ergo-sandbox/src/evidence/validate.rs', 'ergo-sandbox/src/hunt.rs', 'ergo-sandbox/src/map/check_relation.rs'})
old_lock = tomllib.loads(original('Cargo.lock').decode())['package']
new_lock = tomllib.loads((ROOT / 'Cargo.lock').read_text())['package']
check(len(old_lock) == len(new_lock))
for before, after in zip(old_lock, new_lock):
    if before['name'] in ['ergo-sandbox', 'ergo-web']:
        check(after == {**before, 'version': '0.5.0'})
    elif before.get('source', '').startswith('git+https://github.com/arkadianet/ergo?'):
        check(after == {**before, 'version': '0.7.0', 'source': before['source'].replace('9468043396e5daa2828211bcff4234bc70fae4f0', '016533194f94ad95b1a87df70bb9bfce493922e2')})
    else:
        check(before == after, before['name'])
# The stop is historical: only its source-evidence locator moved, never its digest.
stops = json.loads((ROOT / 'docs/roadmap-stops.json').read_text())
for r in stops:
    if r['unitOrProposal'] == 'M05':
        hashes = r['gateResults'][0]['evidenceHashes']
        archived = 'docs/reports/batch-10/baseline/ergo-sandbox/tests/mapping_context_boundary.rs'
        digest = hashes.pop(archived)
        check(hashlib.sha256((ROOT / archived).read_bytes()).hexdigest() == digest)
        hashes['ergo-sandbox/tests/mapping_context_boundary.rs'] = digest
check(stops == json.loads(original('docs/roadmap-stops.json')), "integrity check failed: stops == json.loads(original('docs/roadmap-stops.json'))")
for file in (REPORT / 'baseline').rglob('*'):
    if file.is_file():
        check(file.read_bytes() == original(str(file.relative_to(REPORT / 'baseline'))), "integrity check failed: file.read_bytes() == original(str(file.relative_to(REPORT / 'baseline')))")
metrics = json.loads((ROOT / 'docs/roadmap-metrics.json').read_text())
x01 = metrics.pop('scoreboard')['X01']
check(metrics == json.loads(original('docs/roadmap-metrics.json')), "integrity check failed: metrics == json.loads(original('docs/roadmap-metrics.json'))")
for name, digest in x01['measurements'].items():
    check(hashlib.sha256((REPORT / name).read_bytes()).hexdigest() == digest)
ledger = json.loads((REPORT / 'commands.json').read_text())
for row in ledger['commands']:
    check(row['exitStatus'] == str(row['exitCode']), "integrity check failed: row['exitStatus'] == str(row['exitCode'])")
    check(hashlib.sha256((REPORT / row['log']).read_bytes()).hexdigest() == row['logSha256'], row['log'])
logs = {str(p.relative_to(REPORT)) for p in (REPORT / 'logs').glob('*.log')}
post_logs = {r['log'].removeprefix('docs/reports/batch-10/') for r in ledger.get('postSession', [])}
check(logs == {row['log'] for row in ledger['commands']} | post_logs, 'every command log must have a ledger row')
for file in REPORT.rglob('*'):
    if file.is_file() and file.suffix in ['.json', '.md', '.log', '.txt', '.rs', '.py']:
        text = file.read_text()
        check(not re.search(r'/(?:home|tmp)/', text), f'absolute local path: {file.relative_to(REPORT)}')
print('v1 policy, batch 1–9 reports, manifests, original vectors, all baseline digests, and unrelated dependencies retained; exact API/pin/version edits and portable logs verified.')

# The M05 stop record's evidence for mapping_context_boundary.rs points at an
# archived copy; prove that copy is byte-for-byte the file at the record's own
# attemptCommit, so the relocation cannot hide a change to what was measured.
stops = json.loads((ROOT / 'docs/roadmap-stops.json').read_text())
for record in stops:
    for result in record['gateResults']:
        for path, digest in result['evidenceHashes'].items():
            if path.startswith('docs/reports/batch-10/baseline/'):
                original_path = path.removeprefix('docs/reports/batch-10/baseline/')
                archived = (ROOT / path).read_bytes()
                # The record's attemptCommit lives on the abandoned M05 attempt branch and
                # is not guaranteed in a clone's history; the batch base commit is the last
                # main state whose working tree the governance verified against this digest.
                at_base = original(original_path)
                check(hashlib.sha256(archived).hexdigest() == digest, f'{path}: archived digest differs from the record')
                check(archived == at_base, f'{path}: archived copy differs from {original_path} at {BASE}')
print('Archived stop evidence matches the recorded digest and the file at the batch base commit.')
