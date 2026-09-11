"""Capture the authorized consolidation checks without changing policy or Git."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
COMMANDS = [
    ('fmt', ['cargo', 'fmt', '--all', '--', '--check']),
    ('clippy', ['cargo', 'clippy', '--workspace', '--all-targets', '--', '-D', 'warnings']),
    ('python', ['python3', '-m', 'unittest', 'discover', '-s', 'scripts', '-p', 'test_*.py']),
    ('workspace', ['cargo', 'test', '--workspace', '--release']),
    ('through-completed', ['python3', 'scripts/roadmap_gate.py', '--through-completed', '--report', str(OUT / 'through-completed.json')]),
    ('ci', ['python3', 'scripts/roadmap_gate.py', '--ci', '--report', str(OUT / 'ci.json')]),
]

def main():
    env = dict(os.environ, CARGO_TARGET_DIR='./target-p00')
    report = {'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), 'uncommitted': True, 'startedAt': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'commands': []}
    for name, command in COMMANDS:
        with (OUT / f'{name}.log').open('w') as log:
            result = subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, check=False)
        raw = (OUT / f'{name}.log').read_bytes()
        row = {'name': name, 'command': command, 'exitCode': result.returncode, 'outputSha256': hashlib.sha256(raw).hexdigest()}
        if name == 'workspace':
            totals = re.findall(rb'test result: .*? (\d+) passed; (\d+) failed; (\d+) ignored;', raw)
            row['totals'] = dict(zip(['passed', 'failed', 'ignored'], [sum(int(t[i]) for t in totals) for i in range(3)]))
        report['commands'].append(row)
        (OUT / 'verification.json').write_text(json.dumps(report, indent=2) + '\n')
        print(name, result.returncode, flush=True)
    return int(any(r['exitCode'] for r in report['commands']))

if __name__ == '__main__':
    raise SystemExit(main())
