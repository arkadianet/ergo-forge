"""Join measured seed rows by source/tree identity; vector indices can move."""
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess

REPORT = Path(__file__).resolve().parent
ROOT = REPORT.parents[2]
NODE = Path(os.environ.get('DC_NODE_CHECKOUT', ROOT.parent / 'ergo'))
SEED = 'test-vectors/ergoscript/compile/compile_seed.json'
MAINNET = 'test-vectors/mainnet/scala_tx_json/diff_corpus.json'

def sha(data):
    return hashlib.sha256(data).hexdigest()

measurements = {}
identities = {}
for label in ['before', 'after']:
    measurement = json.loads((REPORT / f'engine-{label}.json').read_text())
    rev = measurement['engineRevision']
    raw = subprocess.check_output(['git', '-C', str(NODE), 'show', f'{rev}:{SEED}'])
    vectors = json.loads(raw)['vectors']
    totals = {}
    for corpus in ['seed', 'mainnet']:
        rows = [r for r in measurement['rows'] if r['corpus'] == corpus]
        totals[corpus] = {'entries': len(rows), 'exact': sum(r['exact'] for r in rows)}
        exact = Counter()
        for row in rows:
            source = ''
            if corpus == 'seed':
                v = vectors[row['index']]
                assert sha(bytes.fromhex(v['tree_hex'])) == row['treeSha256']
                source = sha(v['source'].encode())
            if row['exact']:
                exact[(source, row['treeSha256'])] += 1
        identities[label, corpus] = exact
    measurements[label] = {'revision': rev, 'counts': totals,
                           'seedCorpusSha256': sha(raw),
                           'mainnetCorpusSha256': sha(subprocess.check_output(['git', '-C', str(NODE), 'show', f'{rev}:{MAINNET}'])),
                           'measurementSha256': sha((REPORT / f'engine-{label}.json').read_bytes())}
comparison = {}
for corpus in ['seed', 'mainnet']:
    before, after = identities['before', corpus], identities['after', corpus]
    lost = before - after
    gained = after - before
    comparison[corpus] = {'retainedExactRows': sum((before & after).values()),
                          'lostExactRows': sum(lost.values()), 'gainedExactRows': sum(gained.values()),
                          'lostIdentities': [list(k) for k in lost],
                          'gainedIdentities': [list(k) for k in gained]}
    assert not lost, f'engine-regression: {corpus}: {lost}'
output = {'measurements': measurements, 'comparison': comparison,
          'identity': 'seed: source SHA-256 + tree SHA-256, preserving multiplicity; mainnet: tree SHA-256',
          'decision': 'advance-pin; no seed or mainnet exact loss'}
(REPORT / 'engine-comparison.json').write_text(json.dumps(output, indent=2)+'\n')
print(json.dumps(output, indent=2))
