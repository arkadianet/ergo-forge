"""Check frozen assessment accounting; exit 1 means the entry gate is unmet."""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]

def read(name):
    return json.loads((HERE / name).read_text())

def main():
    candidate_bytes = (HERE / 'candidates.json').read_bytes()
    assert hashlib.sha256(candidate_bytes).hexdigest() == (HERE / 'candidates.sha256').read_text().split()[0]
    frozen = read('candidates.json')
    candidates = frozen['candidates']
    attempts = read('attempts.json')
    parsed = read('parser-output.json')
    reviewed = read('review.json')['rows']
    assert len(candidates) == 20
    assert len({c['family'] for c in candidates}) >= 5
    ids = [c['id'] for c in candidates]
    assert len(set(ids)) == 20
    for rows in [attempts, parsed['results'], reviewed]:
        assert [r['id'] for r in rows] == ids
    for c, a, p, r in zip(candidates, attempts, parsed['results'], reviewed, strict=True):
        assert hashlib.sha256((ROOT / c['source']).read_bytes()).hexdigest() == c['sourceSha256']
        assert a['sourceSha256'] == p['sourceSha256'] == c['sourceSha256']
        assert r['family'] == c['family']
        assert r['qualifies'] == (r['soleArithmetic'] is True)
        assert r['syntaxAccepted'] == p['syntaxAccepted']
    assert parsed['attemptsSha256'] == hashlib.sha256((HERE / 'attempts.json').read_bytes()).hexdigest()
    qualified = [r for r in reviewed if r['qualifies']]
    families = sorted({r['family'] for r in qualified})
    passed = len(qualified) >= 6 and len(families) >= 3
    print(json.dumps({'qualifyingIds': [r['id'] for r in qualified], 'qualifyingFamilies': families, 'denominator': 20, 'threshold': frozen['threshold'], 'passed': passed, 'exitMeaning': '1 = entry gate unmet; integrity assertions must also pass'}, indent=2))
    return 0 if passed else 1

if __name__ == '__main__':
    raise SystemExit(main())
