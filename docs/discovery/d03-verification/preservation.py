"""Read-only preservation audit of the committed D02 prefix.

New files are outside this prefix, including the audit itself. This is not a
correctness check for new files; the Rust and roadmap gates check those.
"""
import hashlib
import json
from pathlib import Path
import subprocess

BASE = 'a996b4eacd9767b067d22a0eab82cbd0e529c9a3'
ROOT = Path(__file__).resolve().parents[3]

def old(path):
    return subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=ROOT)

def policy(text):
    return json.loads(text.split('<!-- roadmap-policy:v1 -->')[1].split('```json')[1].split('```')[0])

before = policy(old('docs/ROADMAP.md').decode())
after = policy((ROOT / 'docs/ROADMAP.md').read_text())
registered = after['units'].pop()
assert registered == {'id': 'D03', 'depends': ['D02'], 'days': 3, 'package': 'ergo-sandbox', 'target': 'property_replay', 'tests': ['all_registered_property_dispositions_match', 'property_replay_rejects_tampered_claims_and_premises', 'legacy_replay_bytes_and_semantics_remain_unchanged', 'registered_author_cases_show_usefulness_beyond_extraction'], 'implemented': False}
assert after == before
stops = json.loads((ROOT / 'docs/roadmap-stops.json').read_text())
assert stops[-1]['unitOrProposal'] == 'D03'
assert stops[:-1] == json.loads(old('docs/roadmap-stops.json'))
allowed = {'docs/ROADMAP.md', 'docs/roadmap-stops.json', 'ergo-sandbox/tests/mapping_inventory.rs', 'ergo-sandbox/src/properties/mod.rs', 'docs/superpowers/specs/2026-09-11-discovery-capability-design.md'}
tracked = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', BASE], cwd=ROOT, text=True).splitlines()
changed = subprocess.check_output(['git', 'diff', '--no-renames', '--name-only', BASE], cwd=ROOT, text=True).splitlines()
# Check all paths that existed at D02, whether modified or deleted. New files
# must not become forbidden merely because they are committed. The design
# record allows the date-convention annotation; historical substance is retained.
unexpected = (set(changed) & set(tracked)) - allowed
assert not unexpected, sorted(unexpected)
pins = {}
for path in tracked:
    if path.startswith('ergo-sandbox/tests/fixtures/properties/') or path in ['ergo-sandbox/src/properties/schema.rs', 'ergo-sandbox/src/properties/evaluate.rs', 'ergo-sandbox/src/properties/trace.rs', 'ergo-sandbox/tests/fixtures/mapping/manifest.json', 'docs/roadmap-metrics.json', 'Cargo.lock']:
        data = (ROOT / path).read_bytes()
        assert data == old(path), path
        pins[path] = hashlib.sha256(data).hexdigest()
print(json.dumps({'baselineCommit': BASE, 'allEarlierPolicyFieldsUnchanged': True, 'allEarlierStopsUnchanged': True, 'registeredOnly': registered, 'pinnedFilesUnchanged': pins, 'M00Anchor': '41729240759244bd3edc833e31359bfe50d8430f0b5a2e27ebc727706363cd10'}, indent=2))
