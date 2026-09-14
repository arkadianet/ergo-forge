#!/usr/bin/env python3
"""Record an observed command exit and portable output; run from repository root."""
import fcntl
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import time

root = Path.cwd()
report = root / 'docs/reports/batch-8'
name, *args = sys.argv[1:]
target = os.environ['CARGO_TARGET_DIR']
def portable(text):
    return text.replace(target, '$CARGO_TARGET_DIR').replace(str(root), '<repo>').replace(str(Path.home()), '<home>').replace(tempfile.gettempdir() + '/', '<tmp>/')
log = report / 'logs' / (name + '.log')
start = time.monotonic()
with log.open('w') as output:
    process = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, env={**os.environ, 'CARGO_TERM_COLOR': 'never'})
    for line in process.stdout:
        output.write(portable(line))
        output.flush()
    code = process.wait()
entry = dict(command=portable(shlex.join(args)), exitCode=code, exitStatus=str(code), log='logs/' + log.name, elapsedSeconds=round(time.monotonic()-start, 2))
with open(Path(tempfile.gettempdir()) / 'forge-b8-command-ledger.lock', 'w') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    ledger = report / 'commands.json'
    data = json.loads(ledger.read_text()) if ledger.exists() else {'environment': {'CARGO_TARGET_DIR': '$CARGO_TARGET_DIR'}, 'commands': [], 'postSession': []}
    data['commands'].append(entry)
    ledger.write_text(json.dumps(data, indent=2) + '\n')
print(json.dumps(entry))
print(''.join(log.read_text().splitlines(keepends=True)[-18:])[-4000:])
sys.exit(code)
