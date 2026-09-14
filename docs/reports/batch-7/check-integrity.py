"""Read-only check of batch scope, frozen files and append-only namespaces."""
import hashlib, json, pathlib, re, subprocess
root = pathlib.Path(__file__).resolve().parents[3]
def git(*args):
    return subprocess.check_output(['git', *args], cwd=root)
def old(path):
    return git('show', '1d53b2f:' + path)
assert git('branch', '--show-current').decode().strip() == 'v2/batch-7'
assert git('rev-parse', '--short=7', 'HEAD').decode().strip() == '1d53b2f'  # session could not commit
changed = git('diff', '--name-only', '1d53b2f').decode().splitlines()
for p in changed:
    assert not re.match(r'docs/reports/batch-[1-6]/', p), p
    assert pathlib.Path(p).name not in {'claim.rs','replay.rs','promotion.rs','identity.rs','compile.rs','eval.rs'}, p
    assert p not in {'.github/workflows/ci.yml', 'ergo-web/Cargo.toml'}, p
for p in ['ergo-sandbox/tests/fixtures/mapping/manifest.json', 'ergo-sandbox/tests/fixtures/properties/manifest.json']:
    assert (root / p).read_bytes() == old(p), p
pattern = rb'<!-- roadmap-policy:v1 -->.*?<!-- /roadmap-policy:v1 -->'
p = 'docs/ROADMAP.md'
policy = re.search(pattern, (root/p).read_bytes(), re.S)[0]
assert policy == re.search(pattern, old(p), re.S)[0]
print('Frozen v1 policy SHA-256:', hashlib.sha256(policy).hexdigest())
for name, key in [('mutants', 'pairs'), ('answer-key', 'perMutant')]:
    p = 'examples/mutants/' + name + '.json'
    before, after = json.loads(old(p)), json.loads((root/p).read_text())
    prior, new = before.pop('staticLintPairs'), after.pop('staticLintPairs')
    assert before == after, p + ': historical fields changed'
    prior_rows, new_rows = prior.pop(key), new.pop(key)
    assert prior == new
    assert new_rows[:-1] == prior_rows
    assert new_rows[-1]['id'] == 'I04-01-v1'
for p in git('ls-tree', '-r', '--name-only', '1d53b2f', 'examples/mutants/s02/v1').decode().splitlines():
    assert (root/p).read_bytes() == old(p)
for p in git('ls-tree', '-r', '--name-only', '1d53b2f', 'ergo-sandbox/src/audit/lints').decode().splitlines():
    if pathlib.Path(p).name != 'mod.rs':
        assert (root/p).read_bytes() == old(p), 'existing lint changed: ' + p
print('Protected reports, code, manifests and prior lint pairs unchanged; I04 appends one pair.')
