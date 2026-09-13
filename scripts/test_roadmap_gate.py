import copy
import contextlib
import io
import json
import re
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import roadmap_gate as gate


class RoadmapGateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy = gate.load_policy()

    def assert_status(self, status, call):
        with self.assertRaises(gate.GateError) as caught:
            call()
        self.assertEqual(caught.exception.status, status)

    def test_policy_v2_block_parses(self):
        v1, v2 = gate.load_policies()
        original_doc = gate.baseline_bytes(gate.ROOT, v2['baselineRev'], 'docs/ROADMAP.md').decode()
        current_doc = (gate.ROOT / 'docs/ROADMAP.md').read_text()
        pattern = r'<!-- roadmap-policy:v1 -->.*?<!-- /roadmap-policy:v1 -->'
        self.assertEqual(re.search(pattern, original_doc, re.S)[0], re.search(pattern, current_doc, re.S)[0])
        spec = (gate.ROOT / 'docs/superpowers/specs/2026-09-13-forge-roadmap-v2.md').read_text()
        expected = json.loads(re.search(r'```json\n(.*?)\n```', spec, re.S)[1])['newUnits']
        self.assertEqual(len(expected), 20)
        self.assertEqual(v2['baselineRev'], '11d9a1c0986c5943d5b421e42271e1277d23528c')
        self.assertIsNone(v2['completedThrough'])
        self.assertEqual(v1['completedThrough'], 'D02')
        self.assertTrue(all(isinstance(u.get('implemented'), bool) for u in expected))
        self.assertEqual(v2['units'], expected)
        for key in ['schemaVersion', 'maxActiveImplementationBranches', 'maxOpenImplementationPrs',
                    'thresholds', 'frozenPreflight', 'scoreboard', 'stopRecords']:
            self.assertEqual(v2[key], v1[key])
        union = gate.union_policy([v1, v2])
        for unit in v2['units']:
            self.assertEqual(gate.select_units(union, unit['id'])[-1], unit)
        self.assertEqual({u['id'] for u in v2['units'] if u['implemented']}, {'W00', 'S00', 'W01', 'S02', 'S01', 'W06', 'I01', 'I02'})
        archived = (gate.ROOT / 'docs/reports/ROADMAP-v1-queue.md').read_text()
        self.assertIn(original_doc[original_doc.index('## 2. '):original_doc.index('## 4. ')], archived)
        self.assert_status('missing-gate', lambda: gate.policy_v2_from(current_doc + gate.V2_MARKER, v1))
        cross = current_doc.replace('"depends": [],\n      "days": 1', '"depends": ["P08"],\n      "days": 1')
        parsed = gate.policy_v2_from(cross, v1)
        self.assertEqual(parsed['units'][0]['depends'], ['P08'])
        broken = cross.replace('"depends": ["P08"],\n      "days": 1', '"depends": ["absent"],\n      "days": 1')
        self.assert_status('missing-gate', lambda: gate.policy_v2_from(broken, v1))
        self.assertEqual(gate.completed_units([v1, v2]), gate.select_units(v1, 'D02'))
        completed = copy.deepcopy(v2)
        completed['completedThrough'] = 'S00'
        self.assertEqual([u['id'] for u in gate.completed_units([v1, completed])][-2:], ['W00', 'S00'])

    def test_scoreboard_reads_every_baseline_artifact(self):
        metrics = gate.load_json(gate.ROOT / self.policy['scoreboard']['path'])
        for policy in gate.load_policies():
            with patch.object(gate, 'baseline_bytes', wraps=gate.baseline_bytes) as baseline:
                evidence = gate.scoreboard(policy)
            for rev in [policy['baselineRev'], metrics['sourceCommit']]:
                for path in metrics['baselineArtifactSha256']:
                    baseline.assert_any_call(gate.ROOT, rev, path)
                    self.assertIn(path, evidence)
            for path in metrics['baselineArtifactSha256']:
                with self.subTest(missing=path), patch.object(gate, 'baseline_bytes', side_effect=lambda root, rev, p: (
                    gate.read(Path('/nonexistent-batch-0-artifact')) if p == path else baseline(root, rev, p)
                )):
                    self.assert_status('missing-gate', lambda: gate.scoreboard(policy))

    def test_s01_extras_require_the_checklist_dom_test(self):
        self.assertEqual(gate.extras('S01'), [('node', ['node', '--test', 'ui/tests/checklist.test.js'])])
        from types import SimpleNamespace
        unit = next(u for u in gate.load_policies()[1]['units'] if u['id'] == 'S01')
        names = unit['tests']
        listing = '\n'.join(f'{n}: test' for n in names)
        output = '\n'.join(f'test {n} ... ok' for n in names) + '\ntest result: ok. 3 passed; 0 failed; 0 ignored;'
        node = '# tests 1\n# pass 1\n# fail 0\n# skipped 0\n# cancelled 0\n# todo 0'
        for code, status in [(0, None), (1, 'failed')]:
            with patch.object(gate, 'read'), patch.object(Path, 'exists', return_value=True), patch.object(gate, 'command', side_effect=[
                SimpleNamespace(returncode=0, stdout=listing),
                SimpleNamespace(returncode=0, stdout=output),
                SimpleNamespace(returncode=code, stdout=node),
            ]) as command:
                if status:
                    self.assert_status(status, lambda: gate.run_unit(unit, []))
                else:
                    gate.run_unit(unit, [])
                self.assertEqual(command.call_args_list[-1].args[0], gate.extras('S01')[0][1])

    def test_i01_extras_require_the_verify_dom_test(self):
        self.assertEqual(gate.extras('I01'), [('node', ['node', '--test', 'ui/tests/verify.test.js'])])
        from types import SimpleNamespace
        unit = next(u for u in gate.load_policies()[1]['units'] if u['id'] == 'I01')
        names = unit['tests']
        listing = '\n'.join(f'{n}: test' for n in names)
        output = '\n'.join(f'test {n} ... ok' for n in names) + '\ntest result: ok. 2 passed; 0 failed; 0 ignored;'
        node = '# tests 1\n# pass 1\n# fail 0\n# skipped 0\n# cancelled 0\n# todo 0'
        for code, status in [(0, None), (1, 'failed')]:
            with patch.object(gate, 'read'), patch.object(Path, 'exists', return_value=True), patch.object(gate, 'command', side_effect=[
                SimpleNamespace(returncode=0, stdout=listing),
                SimpleNamespace(returncode=0, stdout=output),
                SimpleNamespace(returncode=code, stdout=node),
            ]) as command:
                if status:
                    self.assert_status(status, lambda: gate.run_unit(unit, []))
                else:
                    gate.run_unit(unit, [])
                self.assertEqual(command.call_args_list[-1].args[0], gate.extras('I01')[0][1])

    def test_absolute_cargo_target_is_normalized_before_report_hashing(self):
        with patch.dict(gate.os.environ, {'CARGO_TARGET_DIR': '/tmp/batch-build'}):
            self.assertEqual(gate.portable_output('/tmp/batch-build/release/deps/checklist'),
                             '$CARGO_TARGET_DIR/release/deps/checklist')

    def test_python_targets_require_discovered_executed_unskipped_names(self):
        with tempfile.TemporaryDirectory() as tmp, contextlib.redirect_stdout(io.StringIO()):
            root = Path(tmp)
            (root / 'scripts').mkdir()
            target = root / 'scripts/test_sample.py'
            unit = dict(id='sample', implemented=True, package='scripts', target='test_sample', tests=['sample'])
            for source, status in [
                ('def test_sample(self): pass', None),
                ('def test_unrelated(self): pass', 'missing-gate'),
                ('@unittest.skip("no")\n    def test_sample(self): pass', 'missing-gate'),
                ('def test_sample(self): self.fail("broken")', 'failed'),
            ]:
                target.write_text('import unittest\nclass Sample(unittest.TestCase):\n    ' + source + '\n')
                # Different file contents must not reuse an import cache.
                for cached in (root / 'scripts/__pycache__').glob('*'):
                    cached.unlink()
                with self.subTest(source=source):
                    if status:
                        self.assert_status(status, lambda: gate.run_unit(unit, [], root))
                    else:
                        gate.run_unit(unit, [], root)
        self.assert_status('missing-gate', lambda: gate.check_test_output(
            'test_unrelated (sample.Sample.test_unrelated) ... ok\nRan 1 test\nOK\n', ['sample'], 'python'))

    def test_static_lint_namespace_does_not_relax_frozen_hunt_measurements(self):
        real_load = gate.load_json
        path = gate.ROOT / self.policy['frozenPreflight']['artifact']
        current = real_load(path)
        self.assertIn('staticLintPairs', current)
        for change in ['static-only', 'cap', 'verdict', 'membership', 'unknown-field']:
            mutated = copy.deepcopy(current)
            if change == 'static-only':
                # Static counts are independently enforced by S02; the legacy
                # scoreboard must not treat them as a changed hunt measurement.
                mutated['staticLintPairs']['perMutant'][0]['mutantFindings'] = 99
            elif change == 'cap':
                mutated['caps']['maxProbes'] += 1
            elif change == 'verdict':
                mutated['perMutant'][0]['synthesisOn']['verdict'] = 'changed'
            elif change == 'membership':
                mutated['perMutant'].pop()
            else:
                mutated['unregisteredMeasurement'] = {}
            with self.subTest(change=change), patch.object(gate, 'load_json', side_effect=lambda p: (
                mutated if p == path else real_load(p)
            )):
                if change == 'static-only':
                    gate.scoreboard(self.policy)
                else:
                    self.assert_status('failed', lambda: gate.scoreboard(self.policy))

    def test_unknown_unit_is_rejected(self):
        self.assert_status('missing-gate', lambda: gate.select_units(self.policy, 'NOT-A-UNIT'))

    def test_missing_evidence_is_distinct_from_product_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assert_status('missing-gate', lambda: gate.read(Path(tmp) / 'absent.json'))
            self.assert_status('missing-gate', lambda: gate.run_unit(self.policy['units'][0], [], Path(tmp)))
        self.assert_status('failed', lambda: gate.check_test_output('test result: FAILED. 0 passed; 1 failed; 0 ignored;', []))

    def test_missing_scoreboard_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assert_status('missing-gate', lambda: gate.scoreboard(self.policy, Path(tmp)))

    def test_zero_test_filter_is_rejected(self):
        self.assert_status('missing-gate', lambda: gate.check_test_output('test result: ok. 0 passed; 0 failed; 0 ignored;', []))
        self.assert_status('missing-gate', lambda: gate.check_discovery('0 tests, 0 benchmarks', []))

    def test_skipped_tests_are_rejected(self):
        self.assert_status('missing-gate', lambda: gate.check_test_output('test result: ok. 1 passed; 0 failed; 1 ignored;', []))
        self.assert_status('missing-gate', lambda: gate.check_test_output('Ran 1 test\nOK (skipped=1)', [], 'python'))
        self.assert_status('missing-gate', lambda: gate.check_test_output('# tests 1\n# pass 0\n# fail 0\n# skipped 1\n# cancelled 0\n# todo 0', [], 'node'))

    def test_required_names_must_be_discovered_and_executed(self):
        names = self.policy['units'][0]['tests']
        listing = '\n'.join(f'{n}: test' for n in names)
        gate.check_discovery(listing, names)
        self.assert_status('missing-gate', lambda: gate.check_discovery(listing.replace(names[0], 'unrelated'), names))
        output = '\n'.join(f'test {n} ... ok' for n in names) + f'\ntest result: ok. {len(names)} passed; 0 failed; 0 ignored;'
        gate.check_test_output(output, names)
        self.assert_status('missing-gate', lambda: gate.check_test_output(output.replace(names[0], 'unrelated'), names))

    def test_runner_captures_diagnostics_without_weakening_execution_checks(self):
        from types import SimpleNamespace
        unit = copy.deepcopy(next(u for u in self.policy['units'] if u['id'] == 'P03'))
        unit['implemented'] = True  # Exercise the runner independently of completion status.
        names = unit['tests']
        listing = '\n'.join(f'{n}: test' for n in names)
        output = '\n'.join(f'test {n} ... ok' for n in names)
        output += f'\ntest result: ok. {len(names)} passed; 0 failed; 0 ignored;'
        output += '\n---- captured diagnostic ----\nnode outcome detail\n'
        with patch.object(gate, 'read'), patch.object(gate, 'command', side_effect=[
            SimpleNamespace(returncode=0, stdout=listing),
            SimpleNamespace(returncode=0, stdout=output),
        ]) as command:
            gate.run_unit(unit, [])
            self.assertEqual(command.call_args_list[0].args[0][-1], '--list')
            self.assertEqual(command.call_args_list[1].args[0][-1], '--show-output')
        self.assert_status('missing-gate', lambda: gate.check_test_output(
            output.replace(names[0], 'unrelated'), names))

    def test_thresholds_are_read_from_policy(self):
        # Mutating the governing threshold must fail without any copied fixture floor.
        changed = copy.deepcopy(self.policy)
        changed['thresholds']['ingestRows'] += 1
        self.assert_status('failed', lambda: gate.scoreboard(changed))
        changed = copy.deepcopy(self.policy)
        changed['frozenPreflight']['attributable'] += 1
        self.assert_status('failed', lambda: gate.scoreboard(changed))

    def test_scoreboard_cannot_self_attest_changed_measurements(self):
        real_load = gate.load_json
        metrics_path = gate.ROOT / self.policy['scoreboard']['path']
        def altered(path):
            value = real_load(path)
            if path == metrics_path:
                value['preflight']['attributable'] += 1
            return value
        with patch.object(gate, 'load_json', side_effect=altered):
            self.assert_status('failed', lambda: gate.scoreboard(self.policy))

    def test_future_units_fail_as_unimplemented_not_missing_gate(self):
        for unit in copy.deepcopy(self.policy['units']):
            unit['implemented'] = False
            with self.subTest(unit=unit['id']), patch.object(gate, 'command') as command:
                self.assert_status('unimplemented', lambda unit=unit: gate.run_unit(unit, []))
                command.assert_not_called()

    def test_malformed_inputs_still_write_a_failing_report(self):
        real_load = gate.load_json
        malformed_policy = copy.deepcopy(self.policy)
        malformed_policy['thresholds'] = []
        malformed_doc = (gate.MARKER + '\n```json\n' + json.dumps(malformed_policy)
                         + '\n```\n<!-- /roadmap-policy:v1 -->')
        stop_path = gate.ROOT / self.policy['stopRecords']
        for failure in ['policy', 'stop-record', 'arithmetic']:
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as tmp:
                output = Path(tmp) / 'nested/report.json'
                with patch.object(gate, 'load_policies', side_effect=(
                    lambda: [gate.policy_from(malformed_doc)]
                ) if failure == 'policy' else lambda: [self.policy]), patch.object(
                    gate, 'scoreboard', side_effect=ZeroDivisionError('division by zero')
                    if failure == 'arithmetic' else lambda policy: {}
                ), patch.object(gate, 'load_json', side_effect=lambda path: (
                    [None] if path == stop_path else real_load(path)
                )):
                    self.assertEqual(gate.main(['--scoreboard-only', '--report', str(output)]), 1)
                report = json.loads(output.read_text())
                self.assertFalse(report['passed'])
                self.assertEqual(report['results'][-1]['unit'], 'policy/evidence')
                self.assertEqual(report['results'][-1]['status'], 'missing-gate')
                self.assertTrue(report['results'][-1]['detail'])

    def test_dependencies_are_in_policy_order(self):
        for unit in self.policy['units']:
            chosen = gate.select_units(self.policy, unit['id'])
            self.assertEqual(chosen[-1]['id'], unit['id'])
            seen = set()
            for row in chosen:
                self.assertLessEqual(set(row['depends']), seen)
                seen.add(row['id'])

    def test_policy_requires_unique_block_and_known_completion(self):
        doc = (gate.ROOT / 'docs/ROADMAP.md').read_text()
        self.assert_status('missing-gate', lambda: gate.policy_from(doc + gate.MARKER))
        self.assert_status('missing-gate', lambda: gate.policy_from(doc.replace('"schemaVersion": 1', '"schemaVersion": -1')))

    def test_extra_commands_cannot_be_omitted(self):
        self.assertEqual([kind for kind, _ in gate.extras('P00')], ['python', 'node'])
        self.assertIn('test_roadmap_gate.py', gate.extras('P00')[0][1])
        self.assertIn('ui/tests/claim-labels.test.js', gate.extras('P00')[1][1])

    def test_incomplete_stop_record_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / self.policy['stopRecords']
            path.parent.mkdir(parents=True)
            path.write_text('[{"reasonCode":"timebox"}]')
            self.assert_status('missing-gate', lambda: gate.stop_records(self.policy, Path(tmp)))

    def test_resolution_requires_exact_policy_record_and_decision(self):
        # Historical resolutions do not imply that every later goal has no stop.
        original = gate.load_json(gate.ROOT / self.policy['stopRecords'])
        resolved = {r['recordSha256'] for r in self.policy['resolvedStopRecords']}
        pending = [r for r in original if gate.sha(
            json.dumps(r, sort_keys=True, separators=(',', ':')).encode()) not in resolved]
        self.assertEqual(gate.stop_records(self.policy), pending)
        with tempfile.TemporaryDirectory() as tmp:
            self.assert_status('missing-gate', lambda: gate.stop_records(self.policy, Path(tmp)))
        without = copy.deepcopy(self.policy)
        without.pop('resolvedStopRecords', None)
        self.assertTrue(gate.stop_records(without))
        altered = copy.deepcopy(self.policy)
        altered['resolvedStopRecords'][0]['recordSha256'] = '0' * 64
        self.assert_status('missing-gate', lambda: gate.stop_records(altered))
        altered = copy.deepcopy(self.policy)
        altered['resolvedStopRecords'][0]['decisionSha256'] = '0' * 64
        self.assert_status('missing-gate', lambda: gate.stop_records(altered))
        # A later stop remains blocking even when the historical stop is resolved.
        original = gate.load_json(gate.ROOT / self.policy['stopRecords'])
        new_stop = copy.deepcopy(original[0])
        new_stop.pop('resolution')
        new_stop['reasonCode'] = 'new-evidence-failure'
        real_load = gate.load_json
        def appended(path):
            if path == gate.ROOT / self.policy['stopRecords']:
                return original + [new_stop]
            return real_load(path)
        with patch.object(gate, 'load_json', side_effect=appended):
            self.assertEqual(gate.stop_records(self.policy), pending + [new_stop])


class CiGovernanceTests(unittest.TestCase):
    def run_report(self, mode, units, stops, failure=None, v2=False):
        policy = copy.deepcopy(gate.load_policy())
        policy['units'] = units
        policy['completedThrough'] = units[0]['id']
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'report.json'
            with patch.object(gate, 'load_policies', return_value=(
                [{**policy, 'units': [], 'completedThrough': None}, policy] if v2 else [policy])), patch.object(
                gate, 'scoreboard', return_value={}
            ), patch.object(gate, 'stop_records', return_value=stops), patch.object(
                gate, 'run_unit', side_effect=failure
            ):
                code = gate.main([mode, '--report', str(output)])
            return code, json.loads(output.read_text())

    def test_ci_accepts_only_validated_recorded_stops_not_completion(self):
        unit = dict(id='M05', depends=[], implemented=False)
        stops = [dict(unitOrProposal='M05')]
        for mode, expected in [('--ci', 0), ('--all-goals', 1), ('--through-completed', 1)]:
            code, report = self.run_report(mode, [unit], stops)
            self.assertEqual(code, expected)
            self.assertFalse(report['passed'])
            self.assertEqual(report['results'][-1]['status'], 'stopped')
        code, report = self.run_report('--ci', [unit], [])
        self.assertEqual(code, 1)  # Deleting the stop cannot make CI green.
        self.assertEqual(report['results'][-1]['status'], 'unimplemented')

    def test_ci_reports_future_units_as_not_started_without_running_them(self):
        units = [dict(id='future', depends=[], implemented=False),
                 dict(id='later', depends=['future'], implemented=False)]
        code, report = self.run_report('--ci', units, [], AssertionError('must not run'), v2=True)
        self.assertEqual(code, 0)
        self.assertTrue(report['ciAccepted'])
        self.assertFalse(report['passed'])
        self.assertEqual([r['status'] for r in report['results'][1:]], ['not-started', 'not-started'])
        for mode in ['--all-goals', '--through-completed']:
            code, report = self.run_report(mode, units, [], AssertionError('must not run'))
            self.assertEqual(code, 1)
            self.assertEqual(report['results'][-1]['status'], 'unimplemented')

    def test_ci_rejects_failed_missing_and_blocked_units(self):
        unit = dict(id='A', depends=[], implemented=True)
        for status in ['failed', 'missing-gate']:
            code, report = self.run_report('--ci', [unit], [], gate.GateError(status, 'test failure'))
            self.assertEqual(code, 1)
            self.assertEqual(report['results'][-1]['status'], status)
        for implemented, status, expected in [(False, 'unimplemented', 1), (True, 'blocked', 1)]:
            dependent = dict(id='B', depends=['A'], implemented=implemented)
            code, report = self.run_report('--ci', [unit, dependent], [dict(unitOrProposal='A')])
            self.assertEqual(code, expected)
            self.assertEqual(report['results'][-1]['status'], status)
        self.assertEqual(self.run_report('--ci', [unit], [])[0], 0)

    def test_ci_rejects_corrupt_stop_evidence(self):
        policy = gate.load_policy()
        actual_read = gate.read
        evidence = gate.ROOT / 'docs/mapping/m05-stop-evidence/boundary.log'
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'ci.json'
            with patch.object(gate, 'scoreboard', return_value={}), patch.object(
                gate, 'read', side_effect=lambda path: b'changed' if path == evidence else actual_read(path)
            ), patch.object(gate, 'run_unit') as run:
                self.assertEqual(gate.main(['--ci', '--report', str(output)]), 1)
                run.assert_not_called()
            report = json.loads(output.read_text())
            self.assertFalse(report['ciAccepted'])
            self.assertEqual(report['results'][-1]['status'], 'missing-gate')
            self.assertEqual(report['results'][-1]['detail'], 'stop evidence hash mismatch')

    def test_command_normalizes_before_printing_recording_and_hashing(self):
        import contextlib
        import io
        import subprocess
        raw = f'Checking ({gate.ROOT}/ergo-sandbox) {Path.home()}/.cargo/source\n'
        records = []
        out = io.StringIO()
        with patch.object(gate.subprocess, 'run', return_value=subprocess.CompletedProcess(['cargo'], 7, raw)), contextlib.redirect_stdout(out):
            result = gate.command(['cargo'], records)
        self.assertEqual(result.returncode, 7)
        self.assertEqual(out.getvalue(), result.stdout)
        self.assertEqual(records[0]['output'], result.stdout)
        self.assertEqual(records[0]['outputSha256'], gate.sha(result.stdout.encode()))
        self.assertNotIn(str(gate.ROOT), result.stdout)
        self.assertNotIn(str(Path.home()), result.stdout)
        self.assertIn('<repo>/ergo-sandbox', result.stdout)


if __name__ == '__main__':
    unittest.main()
