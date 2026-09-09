import copy
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
        for unit in self.policy['units']:
            if not unit['implemented']:
                self.assert_status('unimplemented', lambda: gate.run_unit(unit, []))

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


if __name__ == '__main__':
    unittest.main()
