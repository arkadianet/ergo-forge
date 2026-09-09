#!/usr/bin/env python3
"""Run the governing roadmap, without turning missing work into product evidence."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
MARKER = '<!-- roadmap-policy:v1 -->'


class GateError(Exception):
    def __init__(self, status, message):
        self.status = status
        super().__init__(message)


def require(condition, message, status='failed'):
    if not condition:
        raise GateError(status, message)


def read(path):
    try:
        return path.read_bytes()
    except OSError as e:
        raise GateError('missing-gate', f'{path}: {e}') from e


def load_json(path):
    try:
        return json.loads(read(path))
    except ValueError as e:
        raise GateError('missing-gate', f'invalid JSON in {path}: {e}') from e


def policy_from(text):
    require(text.count(MARKER) == 1, 'expected one roadmap policy block', 'missing-gate')
    match = re.search(re.escape(MARKER) + r'\s*```json\s*(.*?)\s*```\s*<!-- /roadmap-policy:v1 -->', text, re.S)
    require(match is not None, 'malformed roadmap policy block', 'missing-gate')
    try:
        policy = json.loads(match.group(1))
        require(policy['schemaVersion'] == 1, 'unknown policy version', 'missing-gate')
        units = policy['units']
        seen = set()
        for unit in units:
            require(unit['id'] not in seen, 'duplicate unit', 'missing-gate')
            require(set(unit['depends']) <= seen, 'non-prefix dependency', 'missing-gate')
            require(bool(unit['tests']) and len(set(unit['tests'])) == len(unit['tests']), 'empty/duplicate test names', 'missing-gate')
            require(all(re.fullmatch(r'[a-zA-Z0-9_]+', t) for t in unit['tests']), 'invalid test name', 'missing-gate')
            require(type(unit['implemented']) is bool and unit['days'] > 0, 'invalid unit registration', 'missing-gate')
            require(bool(unit['package']) and bool(unit['target']), 'missing target', 'missing-gate')
            seen.add(unit['id'])
        require(policy['completedThrough'] is None or policy['completedThrough'] in seen, 'unknown completed unit', 'missing-gate')
        require(all(type(v) is int and v >= 0 for v in policy['thresholds'].values()), 'invalid thresholds', 'missing-gate')
        require(re.fullmatch(r'[0-9a-f]{40}', policy['baselineRev']) is not None, 'invalid baseline revision', 'missing-gate')
        for key in ['frozenPreflight', 'scoreboard', 'stopRecords']:
            require(bool(policy[key]), f'missing {key}', 'missing-gate')
        return policy
    except (KeyError, TypeError, ValueError) as e:
        raise GateError('missing-gate', f'invalid policy: {e}') from e


def load_policy(root=ROOT):
    return policy_from(read(root / 'docs/ROADMAP.md').decode())


def select_units(policy, requested=None, all_goals=False):
    units = {u['id']: u for u in policy['units']}
    if all_goals:
        return list(units.values())
    require(requested in units, f'unknown unit: {requested}', 'missing-gate')
    needed = set()
    def visit(name):
        for dep in units[name]['depends']:
            visit(dep)
        needed.add(name)
    visit(requested)
    return [u for u in units.values() if u['id'] in needed]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def baseline_bytes(root, rev, path):
    result = subprocess.run(['git', 'show', f'{rev}:{path}'], cwd=root, capture_output=True)
    require(result.returncode == 0, f'missing committed baseline {rev}:{path}', 'missing-gate')
    return result.stdout


def scoreboard(policy, root=ROOT):
    """Read artifacts only: no compiler, detector, or benchmark execution."""
    paths = policy['scoreboard']
    metrics = load_json(root / paths['path'])
    rev = policy['baselineRev']
    require(metrics['sourceCommit'] == rev, 'scoreboard source commit differs')
    require(metrics['harnessVersion'] == 'roadmap-policy:v1', 'unknown scoreboard producer')
    frozen = policy['frozenPreflight']
    evidence = {}
    baselines = {}
    for path in [paths['ingestion'], paths['decompiler'], paths['history'], frozen['artifact']]:
        original = baseline_bytes(root, rev, path)
        require(metrics['baselineArtifactSha256'].get(path) == sha(original), f'wrong baseline identity: {path}')
        current = read(root / path)
        evidence[path] = sha(current)
        baselines[path] = json.loads(original)
        if path != frozen['artifact']:
            require(current == original, f'frozen baseline changed: {path}')
    corpus = load_json(root / frozen['artifact'])
    # The only mutable fields are explicitly identified narratives. Everything
    # else (including every numeric value, verdict, cap, objective, and witness)
    # must equal the committed measurement.
    def measured(value):
        value = json.loads(json.dumps(value))
        for key in ['denominatorNote', 'foundNote', 'negativeControlsNote', 'missTable']:
            value.pop(key, None)
        for row in value.get('escalatedFindings', []):
            row.pop('disposition', None)
            row.pop('consequence', None)
        for row in value.get('baselineRepairs', {}).values():
            row.pop('note', None)
        return value
    require(measured(corpus) == measured(baselines[frozen['artifact']]), 'frozen corpus data changed')
    rows = corpus['perMutant']
    controls = corpus['negativeControls']
    require(len(rows) == frozen['cases'], 'corpus case count')
    ids = [r['id'] for r in rows]
    require(len(set(ids)) == len(ids) and set(ids) == {r['id'] for r in controls}, 'corpus/control membership')
    proven = sum(r['proven'] for r in rows)
    attributable = sum(r['attributable'] for r in rows)
    confounded = sum(r['confounded'] for r in rows)
    require(proven == frozen['proven'] == corpus['provenMutants'], 'proven denominator')
    require(attributable == frozen['attributable'] == corpus['attributableDetections'], 'attributable numerator')
    require(confounded == frozen['confounded'] == len(corpus['confounded']), 'confounded count')
    require(corpus['detectionRate'] == attributable / proven, 'detection rate')
    require(all(corpus['caps'][k] == frozen[k] for k in ['maxProbes', 'maxPermutations']), 'frozen caps')
    ing = load_json(root / paths['ingestion'])
    require(len(ing['files']) == policy['thresholds']['ingestRows'], 'ingest denominator')
    require(ing['after']['compiled'] >= policy['thresholds']['ingestCompiledFloor'], 'ingest floor')
    require(len({r['path'] for r in ing['files']}) == len(ing['files']), 'duplicate ingest members')
    require(all(re.fullmatch(r'[0-9a-f]{64}', r['sha256']) for r in ing['files']), 'source hashes missing', 'missing-gate')
    require(metrics['ingest'] == dict(compiled=ing['after']['compiled'], rows=len(ing['files']), baselineCompiled=ing['baseline']['compiled'], sourceHashes=len(ing['files']), corpusRevision=ing['corpus_revision'], compilerRevision=ing['compiler_revision'], members=ing['files']), 'ingest scoreboard differs')
    pre = metrics['preflight']
    expected = dict(cases=len(rows), proven=proven, attributable=attributable, confounded=confounded, rate=corpus['detectionRate'], members=ids, excluded=[r['id'] for r in rows if not r['proven']], invalidProvenSynthesisOn=sum(r['synthesisOn']['rejections']['invalid'] for r in rows if r['proven']), excludedInvalidProbes={r['id']:r['synthesisOn']['rejections']['invalid'] for r in rows if not r['proven']}, capped=[r['id'] for r in rows if r['synthesisOn']['capped']])
    require(all(pre.get(k) == v for k, v in expected.items()), 'preflight scoreboard differs')
    require(pre['objective'] == 'recognized-attacker-receipts-v1' and bool(pre['denominatorCaveat']), 'missing objective/denominator caveat')
    dec = load_json(root / paths['decompiler'])
    counts = dict(sorted(Counter(r['bucket'] for r in dec.values()).items()))
    require(metrics['decompiler'] == dict(entries=len(dec), buckets=counts, obtainedTrees=sum(v for k, v in counts.items() if k != 'initial-compile-failed'), exactTrees=counts['byte-identical']), 'decompiler scoreboard differs')
    for reference in dec:
        file, _, pointer = reference.partition('#')
        data = read(root / file)
        if pointer:
            value = json.loads(data)
            for part in pointer.strip('/').split('/'):
                value = value[int(part)] if isinstance(value, list) else value[part.replace('~1', '/').replace('~0', '~')]
    for key in ['nodeValidatedClaims', 'canonicalAcceptedBundles', 'promotedSearchCandidates']:
        require(metrics[key]['value'] == 0 and bool(metrics[key]['producer']), f'{key}: no producer at baseline')
    for key in ['deployedSourceEquivalence', 'realAuditActionablePrecision', 'obligationReviewDuplication']:
        require(metrics[key]['value'] is None and bool(metrics[key]['producer']), f'{key}: unknown is not zero')
    # Retire prose-only authority: verify the baseline figures and artifact links
    # in the human scoreboard against the same rows, not copied test thresholds.
    doc = read(root / 'docs/ROADMAP.md').decode().split('## 5. Scoreboard')[1].split('## 6. Stop rules')[0]
    for token in [f"**{ing['after']['compiled']}/{len(ing['files'])}", f"**{ing['baseline']['compiled']}/{len(ing['files'])}**", f'**{attributable}/{proven}**', f"**{counts['byte-identical']}/{metrics['decompiler']['obtainedTrees']}**", f"**{len(dec)} entries: {counts['byte-identical']} byte-identical, {counts['recompiles-but-differs']} recompiling with different bytes, {counts['initial-compile-failed']} initial compilation failures**"]:
        require(token in doc, f'roadmap prose disagrees with recorded rows: {token}')
    for link in re.findall(r'\]\(([^)]+)\)', doc):
        if '://' not in link and not link.startswith('#'):
            read(root / 'docs' / link.split('#')[0])
    evidence[paths['path']] = sha(read(root / paths['path']))
    return evidence


def stop_records(policy, root=ROOT):
    path = root / policy['stopRecords']
    if not path.exists():
        require(not policy.get('resolvedStopRecords'), 'resolved stop record is missing', 'missing-gate')
        return []
    records = load_json(path)
    require(isinstance(records, list), 'stop records must be an array', 'missing-gate')
    required = {'unitOrProposal', 'reasonCode', 'attemptCommit', 'daysSpent', 'gateResults', 'retainedCapability', 'unsupportedClass', 'reopenEvidenceRequired'}
    for record in records:
        require(required <= record.keys(), 'incomplete stop record', 'missing-gate')
        require(all(record[k] for k in required - {'daysSpent'}), 'empty stop evidence', 'missing-gate')
        require(isinstance(record['daysSpent'], (int, float)) and record['daysSpent'] >= 0, 'invalid daysSpent', 'missing-gate')
        require(re.fullmatch(r'[0-9a-f]{40}', record['attemptCommit']) is not None, 'invalid attemptCommit', 'missing-gate')
        for result in record['gateResults']:
            require(isinstance(result.get('command'), list) and bool(result['command']) and type(result.get('exitCode')) is int and bool(result.get('evidenceHashes')), 'incomplete stop gate result', 'missing-gate')
            for file, digest in result['evidenceHashes'].items():
                require(sha(read(root / file)) == digest, 'stop evidence hash mismatch', 'missing-gate')
    resolutions = policy.get('resolvedStopRecords', [])
    require(isinstance(resolutions, list), 'invalid stop resolutions', 'missing-gate')
    by_hash = {}
    for resolution in resolutions:
        require(isinstance(resolution, dict) and {'unit', 'recordSha256', 'decision', 'decisionSha256'} <= resolution.keys(), 'incomplete stop resolution', 'missing-gate')
        digest = resolution['recordSha256']
        require(digest not in by_hash, 'duplicate stop resolution', 'missing-gate')
        by_hash[digest] = resolution
    open_records = []
    used = set()
    for record in records:
        digest = sha(json.dumps(record, sort_keys=True, separators=(',', ':')).encode())
        resolution = by_hash.get(digest)
        if resolution is None:
            open_records.append(record)
            continue
        require(resolution['unit'] == record['unitOrProposal'], 'stop resolution unit mismatch', 'missing-gate')
        require(record.get('resolution', {}).get('governingDecision') == resolution['decision'], 'stop resolution lacks governing decision', 'missing-gate')
        require(sha(read(root / resolution['decision'])) == resolution['decisionSha256'], 'governing decision hash mismatch', 'missing-gate')
        used.add(digest)
    require(used == set(by_hash), 'stop resolution does not match the recorded stop', 'missing-gate')
    return open_records


def check_discovery(output, names):
    found = set(re.findall(r'^(.+): test$', output, re.M))
    require(bool(found), 'zero-test discovery/filter', 'missing-gate')
    require(set(names) <= found, f'required tests not discovered: {sorted(set(names) - found)}', 'missing-gate')


def check_test_output(output, names, kind='rust'):
    if kind == 'rust':
        summaries = re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;', output)
        require(bool(summaries), 'missing test execution evidence', 'missing-gate')
        require(all(int(p) + int(f) > 0 for _, p, f, _ in summaries), 'zero-test filter', 'missing-gate')
        require(all(int(i) == 0 for _, _, _, i in summaries), 'skipped/ignored test', 'missing-gate')
        require(all(s == 'ok' and int(f) == 0 for s, _, f, _ in summaries), 'product test failure')
        passed = set(re.findall(r'^test (\S+) \.\.\. ok$', output, re.M))
        require(set(names) <= passed, 'required test did not execute successfully', 'missing-gate')
    elif kind == 'node':
        counts = {k:int(v) for k,v in re.findall(r'^# (tests|pass|fail|skipped|cancelled|todo) (\d+)$', output, re.M)}
        require(counts.get('tests', 0) > 0 and len(counts) == 6, 'missing/zero Node test evidence', 'missing-gate')
        require(not any(counts[k] for k in ['skipped','cancelled','todo']), 'skipped Node test', 'missing-gate')
        require(counts['fail'] == 0 and counts['pass'] == counts['tests'], 'Node product failure')
    else:
        match = re.search(r'Ran (\d+) tests?', output)
        require(match is not None and int(match.group(1)) > 0, 'missing/zero Python test evidence', 'missing-gate')
        require('skipped=' not in output and 'expected failures=' not in output, 'skipped Python test', 'missing-gate')
        require(re.search(r'^OK$', output, re.M) is not None, 'Python product failure')


def command(args, records, root=ROOT):
    try:
        result = subprocess.run(args, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env={**os.environ, 'CARGO_TARGET_DIR': os.environ.get('CARGO_TARGET_DIR', './target-p00'), 'CARGO_TERM_COLOR':'never'})
    except OSError as e:
        raise GateError('missing-gate', f'{args[0]} unavailable: {e}') from e
    records.append(dict(command=args, exitCode=result.returncode, output=result.stdout, outputSha256=sha(result.stdout.encode())))
    print(result.stdout, end='', flush=True)
    return result


def extras(unit):
    if unit == 'P00':
        return [('python', [sys.executable, '-m', 'unittest', 'discover', '-s', 'scripts', '-p', 'test_roadmap_gate.py']), ('node', ['node', '--test', 'ui/tests/claim-labels.test.js'])]
    if unit == 'P08':
        return [('node', ['node', '--test', 'ui/tests/evidence-replay.test.js'])]
    return []


def run_unit(unit, records, root=ROOT):
    require(unit['implemented'], 'registered product unit is not implemented', 'unimplemented')
    target = root / unit['package'] / 'tests' / (unit['target'] + '.rs')
    read(target)
    cargo = ['cargo', 'test', '--release', '-p', unit['package'], '--test', unit['target'], '--']
    listed = command(cargo + ['--list'], records, root)
    require(listed.returncode == 0, 'test discovery/build failed')
    check_discovery(listed.stdout, unit['tests'])
    # Captured diagnostics follow intact status lines; --nocapture can split them.
    result = command(cargo + ['--show-output'], records, root)
    if result.returncode:
        raise GateError('failed', 'product test command failed')
    check_test_output(result.stdout, unit['tests'])
    for kind, args in extras(unit['id']):
        if kind == 'node':
            read(root / args[-1])
            require((root / 'ui/node_modules/linkedom/package.json').exists(), 'DOM test dependency missing; npm ci --prefix ui', 'missing-gate')
        else:
            read(root / 'scripts/test_roadmap_gate.py')
        result = command(args, records, root)
        require(result.returncode == 0, f'{kind} test command failed')
        check_test_output(result.stdout, [], kind)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--require')
    group.add_argument('--all-goals', action='store_true')
    group.add_argument('--through-completed', action='store_true')
    group.add_argument('--scoreboard-only', action='store_true')
    parser.add_argument('--report', type=Path)
    args = parser.parse_args(argv)
    report = {'formatVersion':1, 'results':[], 'commands':[], 'evidenceHashes':{}}
    try:
        policy = load_policy()
        selected = [] if args.scoreboard_only else select_units(policy, args.require if args.require else policy['completedThrough'], args.all_goals)
        report['policySha256'] = sha(json.dumps(policy, sort_keys=True).encode())
        report['evidenceHashes'] = scoreboard(policy)
        stops = stop_records(policy)
        report['results'].append(dict(unit='scoreboard', status='passed'))
        states = {}
        for unit in selected:
            try:
                require(not any(r['unitOrProposal'] == unit['id'] for r in stops), 'recorded stop blocks this unit', 'stopped')
                require(unit['implemented'], 'registered product unit is not implemented', 'unimplemented')
                require(all(states[d] == 'passed' for d in unit['depends']), 'dependency did not pass', 'stopped')
                run_unit(unit, report['commands'])
                status, detail = 'passed', 'required tests executed'
            except GateError as e:
                status, detail = e.status, str(e)
            states[unit['id']] = status
            report['results'].append(dict(unit=unit['id'], status=status, detail=detail))
            print(f"{unit['id']}: {status}: {detail}", flush=True)
    except (GateError, KeyError, TypeError, ValueError, IndexError) as e:
        report['results'].append(dict(unit='policy/evidence', status=e.status if isinstance(e, GateError) else 'missing-gate', detail=str(e)))
    report['passed'] = bool(report['results']) and all(r['status'] == 'passed' for r in report['results'])
    output = args.report or ROOT / 'target-p00/roadmap-gates' / ((args.require or ('all-goals' if args.all_goals else 'scoreboard' if args.scoreboard_only else 'through-completed')) + '.json')
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + '\n')
    for result in report['results']:
        print(f"{result['unit']}: {result['status']}" + (f": {result['detail']}" if 'detail' in result else ''))
    print(f'report: {output}')
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
