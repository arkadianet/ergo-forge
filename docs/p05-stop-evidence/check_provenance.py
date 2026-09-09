"""Audit the rejected P05 candidate; exit 1 means it lacks incident provenance.

This is stop evidence, not a replacement acceptance gate or a replay producer.
It deliberately does not infer historical validation inputs from node defaults.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
manifest = json.loads((ROOT / 'sources.json').read_text())
for name, entry in manifest['artifacts'].items():
    if hashlib.sha256((ROOT / name).read_bytes()).hexdigest() != entry['sha256']:
        sys.exit(f'corrupt stop evidence: {name}')
candidate = json.loads((ROOT / 'rejected-use-candidate.fixture').read_text())['execution']
block = json.loads((ROOT / 'public-block.fixture').read_text())['block']
tx = json.loads((ROOT / 'public-transaction.fixture').read_text())
assert tx['blockId'] == block['header']['id']
assert block['blockTransactions'][tx['index']]['id'] == tx['id']
print(f"incident: {tx['id']}; block height: {block['header']['height']}; index: {tx['index']}")
missing = []
for name in ('blockContext', 'parameters', 'networkRules', 'headers', 'priorBlockCost'):
    value = candidate[name]
    print(f"{name}: {value['status']}; origin={value.get('origin', 'MISSING')}")
    if value.get('status') != 'present' or value.get('origin') != 'source-recorded':
        missing.append(name)
for field, source in [('preHeaderVersion', 'version'), ('preHeaderTimestamp', 'timestamp'), ('preHeaderParentId', 'parentId')]:
    supplied = candidate['blockContext']['value'][field]
    actual = block['header'][source]
    print(f'{field}: supplied={supplied}; retrieved={actual}; equal={supplied == actual}')
print('Retrieved transaction records expose no preceding transaction execution costs:',
      not any('cost' in k.lower() for row in block['blockTransactions'] for k in row))
print('This audit does not establish historical UTXO membership or authenticate the explorer.')
if missing:
    print('missing-provenance: ' + ', '.join(missing))
    sys.exit(1)
print('Source labels alone would still need value/source validation before admission.')
