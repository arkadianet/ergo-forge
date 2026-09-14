"""Offline CI topology and retained-check gates."""
import re
import unittest

from test_release import workflow


class CiWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.ci = workflow('.github/workflows/ci.yml')
        self.jobs = self.ci['jobs']

    def job_for(self, command):
        jobs = [name for name, job in self.jobs.items()
                if any(command in step.get('run', '').splitlines() for step in job['steps'])]
        self.assertEqual(len(jobs), 1, f'{command}: expected exactly one job, got {jobs}')
        return jobs[0]

    def test_node_checkout_is_cached(self):
        for command in ['cargo test --workspace',
                        'cargo test -p ergo-sandbox -p ergo-web --features cost-trace']:
            steps = self.jobs[self.job_for(command)]['steps']
            pin = next(s for s in steps if s.get('id') == 'pin')
            self.assertIn('Cargo.toml', pin['run'])
            self.assertIn('rev=$rev', pin['run'])
            cache = next(s for s in steps if s.get('id') == 'node-cache')
            self.assertRegex(cache['uses'], r'^actions/cache@[0-9a-f]{40}$')
            self.assertEqual(cache['with']['path'].rstrip('/'), 'ergo')
            self.assertIn('${{ steps.pin.outputs.rev }}', cache['with']['key'])
            self.assertNotIn('restore-keys', cache['with'])
            node = next(s for s in steps if s.get('with', {}).get('repository') == 'arkadianet/ergo')
            self.assertEqual(node['with']['path'], 'ergo')
            self.assertEqual(node['with']['ref'], '${{ steps.pin.outputs.rev }}')
            self.assertEqual(node['if'], "steps.node-cache.outputs.cache-hit != 'true'")
            self.assertLess(steps.index(pin), steps.index(cache))
            self.assertLess(steps.index(cache), steps.index(node))
            self.assertLess(steps.index(node), next(i for i, s in enumerate(steps) if s.get('run') == command))
            forge = next(s for s in steps if s.get('name') == 'Checkout ergo-forge')
            self.assertEqual(forge['with']['path'], 'ergo-forge')

    def test_cost_trace_runs_in_its_own_job(self):
        default = self.job_for('cargo test --workspace')
        traced = self.job_for('cargo test -p ergo-sandbox -p ergo-web --features cost-trace')
        self.assertNotEqual(default, traced)
        self.assertEqual(self.job_for('cargo clippy --workspace --all-targets -- -D warnings'), default)
        self.assertEqual(self.job_for('cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace -- -D warnings'), traced)
        for name in [default, traced]:
            self.assertFalse(self.jobs[name].get('needs'), 'verification jobs must run in parallel')
            steps = self.jobs[name]['steps']
            self.assertTrue(any(s.get('uses', '').startswith('actions/setup-node@') for s in steps))
            self.assertTrue(any(s.get('run') == 'npm ci --prefix ui' for s in steps))
            self.assertTrue(any(s.get('uses', '').startswith('Swatinem/rust-cache@') for s in steps))
            for step in steps:
                if 'uses' in step:
                    self.assertRegex(step['uses'], r'^[^@]+@[0-9a-f]{40}$')
                if 'run' in step:
                    self.assertEqual(step['working-directory'], 'ergo-forge')
        for command in ['cargo fmt --all -- --check',
                        'cargo test -p ergo-web --no-default-features',
                        'python3 scripts/test_lockfile_action.py',
                        'python3 scripts/roadmap_gate.py --through-completed',
                        'python3 scripts/roadmap_gate.py --require P08',
                        'python3 scripts/roadmap_gate.py --ci',
                        'python3 scripts/test_release.py',
                        'python3 scripts/test_ci_workflow.py']:
            self.assertEqual(self.job_for(command), default)
        gate = next(s for s in self.jobs[default]['steps'] if s.get('run') == 'python3 scripts/roadmap_gate.py --ci')
        self.assertEqual(gate['if'], 'always()')
        self.assertEqual(self.ci['env']['CARGO_PROFILE_DEV_DEBUG_ASSERTIONS'], 'true')
        self.assertEqual(self.ci['env']['CARGO_PROFILE_DEV_OVERFLOW_CHECKS'], 'true')


if __name__ == '__main__':
    unittest.main()
