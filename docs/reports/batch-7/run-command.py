"""Record observed validation exits and portable output for this session."""
import json, os, pathlib, shlex, subprocess, sys, time
root = pathlib.Path(__file__).resolve().parents[3]
report = root / 'docs/reports/batch-7'
name, *command = sys.argv[1:]
env = dict(os.environ, CARGO_TARGET_DIR=str(root.parent / 'ergo-forge/target-b7'))
cargo_home = str(pathlib.Path.home() / '.cargo')
worktree_git_dir = str(root.parent / 'ergo-forge/.git/worktrees' / root.name)
start = time.monotonic()
log = report / 'logs' / (name + '.log')
row = dict(command=shlex.join(command), exitStatus='no observed exit', log='logs/' + log.name)
ledger = report / os.environ.get('B7_COMMAND_LEDGER', 'commands.json')
data = json.loads(ledger.read_text()) if ledger.exists() else dict(formatVersion=1, baseRevision='1d53b2f', branch='v2/batch-7', cargoTargetDir='$CARGO_TARGET_DIR', commands=[])
data['commands'].append(row)
ledger.write_text(json.dumps(data, indent=2) + '\n')
with log.open('w') as output:
    process = subprocess.Popen(command, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    for line in process.stdout:
        line = line.replace(env['CARGO_TARGET_DIR'], '$CARGO_TARGET_DIR').replace(str(root), '$WORKTREE').replace(cargo_home, '$CARGO_HOME').replace(worktree_git_dir, '$WORKTREE_GIT_DIR').replace(str(pathlib.Path.home() / '.rustup'), '$RUSTUP_HOME')
        output.write(line)
        output.flush()
    code = process.wait()
row.update(exitStatus=str(code), exitCode=code, elapsedSeconds=round(time.monotonic()-start, 2))
data = json.loads(ledger.read_text())
for index, saved in enumerate(data['commands']):
    if saved['log'] == row['log']:
        data['commands'][index] = row
        break
ledger.write_text(json.dumps(data, indent=2) + '\n')
print(f'{name}: exit {code}; {row["log"]}', flush=True)
sys.exit(code)
