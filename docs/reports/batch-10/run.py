"""Run one argv command and retain its observed exit and portable output."""
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import time
import tempfile

ROOT = Path(__file__).resolve().parents[3]
REPORT = Path(__file__).resolve().parent

def portable(text):
    for value, name in [(os.environ['CARGO_TARGET_DIR'], '$CARGO_TARGET_DIR'),
                        (str(ROOT), '$REPO'), (str(ROOT.parent / 'ergo-forge' / '.git'), '$GIT_DIR'),
                        (str(ROOT.parent / 'ergo'), '$NODE_CHECKOUT'),
                        (str(Path(tempfile.gettempdir()) / 'batch-10-node-before'), '$BEFORE_CORPUS'),
                        (str(Path.home()), '$USER_HOME')]:
        text = text.replace(value, name)
    return text.replace(tempfile.gettempdir() + os.sep, '<tmp>/')

if __name__ == '__main__':
    name, *args = sys.argv[1:]
    env = os.environ.copy()
    env.update(CARGO_PROFILE_DEV_OPT_LEVEL='3', CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true',
               CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true', CARGO_TERM_COLOR='never')
    overrides = []
    while args and args[0] == '--env':
        key, value = args[1].split('=', 1)
        env[key] = value
        overrides.append(args[1])
        args = args[2:]
    start = time.monotonic()
    result = subprocess.run(args, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    output = portable(result.stdout.decode(errors='replace'))
    log = REPORT / 'logs' / (name + '.log')
    log.write_text(output)
    ledger_path = REPORT / 'commands.json'
    ledger = json.loads(ledger_path.read_text()) if ledger_path.exists() else dict(
        formatVersion=1, baseRevision='a18999a', branch='v2/batch-10', cargoTargetDir='$CARGO_TARGET_DIR',
        profile=dict(optLevel=3, debugAssertions=True, overflowChecks=True), commands=[])
    row = dict(command=portable(' '.join(overrides + [shlex.join(args)])), exitCode=result.returncode,
               exitStatus=str(result.returncode), elapsedSeconds=round(time.monotonic()-start, 2),
               log=str(log.relative_to(REPORT)), logSha256=hashlib.sha256(log.read_bytes()).hexdigest())
    ledger['commands'].append(row)
    ledger_path.write_text(json.dumps(ledger, indent=2)+'\n')
    print(json.dumps(row))
    print(output[-5000:])
    sys.exit(result.returncode)
