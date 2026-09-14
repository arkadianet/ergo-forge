"""Check the portable evidence ledger and unchanged gate output hashes."""
import hashlib, json, pathlib, re
root = pathlib.Path(__file__).resolve().parent
ledger = json.loads((root/'commands.json').read_text())
assert len(ledger['commands']) == len({c['log'] for c in ledger['commands']})
for c in ledger['commands']:
    assert (root/c['log']).is_file()
    if 'exitCode' in c:
        assert c['exitStatus'] == str(c['exitCode'])
repo = root.parents[2]
post = ledger.get('postSession')
assert post, 'postSession re-verification entries are required'
for c in post:
    assert c['exitCode'] == 0, c['command']
    assert (repo/c['log']).is_file(), c['log']
for name in ['S04.json', 'I04.json']:
    gate = json.loads((root/name).read_text())
    assert gate['passed']
    for c in gate['commands']:
        assert c['exitCode'] == 0
        assert c['outputSha256'] == hashlib.sha256(c['output'].encode()).hexdigest()
for p in root.rglob('*'):
    if p.is_file() and '__pycache__' not in p.parts:
        # The gate runner records the interpreter it ran (/usr/bin/python3);
        # that is not a workstation location.
        text = p.read_text().replace('/usr/bin/python3', 'python3')
        assert not re.search('/' + '(?:home|tmp|usr|opt)/', text), str(p)
print('Gate outputs and hashes match; all ledger logs exist; report paths are portable.')
