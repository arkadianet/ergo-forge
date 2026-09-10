import copy
import json
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
                with patch.object(gate, 'load_policy', side_effect=(
                    lambda: gate.policy_from(malformed_doc)
                ) if failure == 'policy' else lambda: self.policy), patch.object(
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
        self.assertEqual(gate.stop_records(self.policy), [])
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
            self.assertEqual(gate.stop_records(self.policy), [new_stop])


if __name__ == '__main__':
    unittest.main()
