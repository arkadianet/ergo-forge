"""Run the preserved strict D03 target; it must fail utility, not pass it.

Restores only the new test target temporarily. Does not alter policy, frozen
inputs, stops or first-measurement artifacts. Never overwrites an existing target.
"""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
target = root / 'ergo-sandbox/tests/property_replay.rs'
source = Path(__file__).with_name('property_replay.rs')
with target.open('xb') as stream:
    stream.write(source.read_bytes())
try:
    env = dict(os.environ, CARGO_TARGET_DIR='./target-p00')
    env.pop('D03_MEASUREMENT_DIR', None)
    result = subprocess.run(
        ['cargo', 'test', '-p', 'ergo-sandbox', '--release', '--test',
         'property_replay', '--', '--show-output'], cwd=root, env=env)
finally:
    target.unlink()
raise SystemExit(result.returncode)
