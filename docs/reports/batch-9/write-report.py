"""Render the batch report from observed command exits."""
import hashlib
import json
from pathlib import Path

report = Path(__file__).resolve().parent
ledger = json.loads((report / 'commands.json').read_text())
rows = ledger['commands']
required = [
    'cargo fmt --all -- --check',
    'cargo clippy --workspace --all-targets -- -D warnings',
    'cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present',
    'python3 scripts/test_roadmap_gate.py',
    'python3 scripts/test_lockfile_action.py',
    'node --test ui/tests/attack.test.js',
    'python3 scripts/roadmap_gate.py --require W05 --report docs/reports/batch-9/W05.json',
    'python3 scripts/roadmap_gate.py --require S05 --report docs/reports/batch-9/S05.json',
    'cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory',
]
status = []
for command in required:
    matches = [r for r in rows if r['command'].startswith(command)]
    row = matches[-1] if matches else {'exitStatus': 'no observed exit', 'log': ''}
    label = 'node --test (all 10 UI test files, passed explicitly)' if command.startswith('node ') else command
    status.append(f"| `{label}` | {row['exitStatus']} | [{Path(row['log']).name}]({row['log']}) |")
all_commands = [
    f"| {i} | `{r['command'].replace('|', '&#124;')}` | {r['exitStatus']} | {r.get('elapsedSeconds', '')} "
    f"| [{Path(r['log']).name}]({r['log']}) | `{r.get('logSha256', '')}` |"
    for i, r in enumerate(rows, 1)
]
hashes = []
for name in ['W05.json', 'S05.json', 'decompile-w05.json']:
    p = report / name
    if p.exists():
        hashes.append(f"| [{name}]({name}) | `{hashlib.sha256(p.read_bytes()).hexdigest()}` |")
text = '''# Batch 9 — W05 and S05

Implementation on `v2/batch-9`, based on `a69af13`, 2026-09-14. Git staging
and commits were refused by the sandbox's read-only worktree index (exit 128).
The work remained uncommitted. No push, branch switch, other-worktree edit,
explorer request or broadcast occurred. All builds used this batch's isolated
`$CARGO_TARGET_DIR`.

All required validation commands finished with exit 0. The final workspace run
completed in 873.65 seconds, including the full mutation-corpus measurement and
the requested seed-corpus skip. Earlier failures are retained in the ledger.

## Implemented

- W05: `pool-bound-swap.es` and `successor-locked-vault.es`, using the existing
  EIP-5 recipe format discovered in the checkout. Both compile at the default
  tree version 3, appear in Build, and carry an authored walkthrough in their
  doc header. Build's production edit was two title entries; index.html was
  unchanged. The DOM test drives the existing picker/question flow.
- Independent source+params suites beside the recipes: 5 swap scenarios and
  8 vault scenarios. Honest cases pass; decoy pool/NFT, falling reserve product,
  successor script/value/register changes and extra outputs fail as specified.
  Verdicts were authored independently, never populated from evaluator output.
- Two versioned source mutants under `examples/mutants/w05/v1/`, with their
  recipes as controls. `examples/mutants/recipes.json` registers them and a
  separate in-memory output-count mutation. The three defects individually
  flip their binding case from required fail to actual pass; the suite catches
  each. Their respective static observations are unbound-box-reserves,
  successor-field-drift and unconstrained-outputs. Controls are clean under
  every current lint in both lift modes. The separate manifest avoided changing
  either historical mutant JSON file or parallel staticLintPairs appends.
- S05: `ergo_sandbox::incident::scaffold` and `ergo-es incident <txid>
  --explorer <url> [--out <dir>] [--network mainnet|testnet]`. A new directory
  receives README.md, empty triage.json and one `input-N/contract.test.json`
  per distinct spent script, with one case per matching SELF position.
- TxBoxes now carries optional transaction inclusion height and ordered data
  inputs. The explorer transaction adapter hydrates incomplete references via
  read-only box lookups and preserves hex case. Fixtures and CLI share the
  scaffold; offline adapter tests also exercise hydration and exact raw hex.
- Both units were marked implemented in v2 policy and spec; the implemented-set
  assertion adds only W05/S05. W05's DOM test is required by extras(), including
  failure/empty/ignored-run checks in the gate runner unit test.

## Exact box reproduction and evidence boundary

The test constructs and JSON-round-trips a Fixture archive from the committed
USE boxes, then passes it through the production scaffold. It matches the swap
and pool suites by script, preserving selfIndex 1/2 and inclusion height 1868204.
All projected input/output JSON bytes match after common JSON serialization:
fields, array/token order, integer precision and exact hex characters.

The committed suites lack every boxId and every input creationHeight. Their
outputs carry creationHeight 1868202, which is retained and compared. The test
supplies conspicuous synthetic values only for the missing metadata and removes
precisely those additions for equality. It never changes the original suites.
Data inputs and mixed-case hex are covered separately. There was no recorded
live transaction fixture added: the committed USE boxes supply the Fixture.

Every generated expectation is REPLACE-ME, rejected by the real CLI test parser.
Missing inclusion height is another flagged placeholder; the chain tip and
box creation heights never replace it. Triage selection, objective and bounds
remain empty for the author. The scaffold makes no exploitability claim.
Recipe results are synthetic with nodeValidated: false; static findings mean
review priority. See [incident-scaffold.md](../../incident-scaffold.md).

## Integration and retained limitations

The existing every-recipe recognition test exposed unsupported guarded branches,
reserve-product comparisons, non-SELF script comparisons and empty collection
literals. Small additions to Read's literal description renderer cover those
shapes while retaining its incomplete/quoted fallback. The Build combinator DSL,
compiler, evaluator, lints and existing recipe tests were unchanged.

The old decompiler corpus inventory is frozen. A separate
`w05_decompile_expectations.json` adds four measured records, with duplicate
baseline keys rejected. The two recipe trees lift completely and recompile,
but their decompiled source did not round-trip to identical tree bytes on the
pinned engine. Those outcomes were retained as recompiles-but-differs, not
reclassified as exact. The two embedded context-tree records are byte-identical.
[decompile-w05.json](decompile-w05.json) records the new measurements.
The immutable S02 sweep delegates only these two new recipes to W05's all-lint
checks; every historical source, finding and record remains pinned.

The live explorer path was unverified in this session, as required by the
explorer-dependency boundary. Fixtures supplied the required box projection,
so no capability stop was recorded and no unit was widened to obtain live data.
The teaching recipes constrain exactly two/one outputs; they did not implement
fee/change assembly, wallet signing or node-validated transaction acceptance.

Initial development runs exposed aliased synthetic input/output objects in
fixture authoring, an unfinished import, an overbroad creationHeight projection,
missing corpus registration and unsupported descriptions. An intermediate
vault simplification also let compilation eliminate the SELF R4 read from its
mutant; the explicit accounting-unit condition was restored. These failures
and the intentional measurements before registering the new decompiler rows
are retained below. Required unit gate runs passed; no interrupted command was
reported as a pass. The first workspace attempt exited at the corpus inventory
check; the final run follows the separate registration.

## Required command results

The latest observed exit is shown below. The known local seed-corpus mismatch
was skipped exactly as requested; existing ignored tests were left unchanged.

| Command | Exit | Log |
|---|---:|---|
''' + '\n'.join(status) + '''

## Complete command ledger

Every validation and commit attempt is recorded in [commands.json](commands.json)
with exit status, elapsed time and its log. Earlier failures remain visible.
The adapter-only filter run executed one library test and zero integration
tests; it was not counted as integration or gate evidence.

| # | Command | Exit | Seconds | Log | Log SHA-256 |
|---:|---|---:|---:|---|---|
''' + '\n'.join(all_commands) + '''

## Integrity and prepared commits

[check-integrity.py](check-integrity.py) checks all forbidden source files,
all historical fixtures/recipes/suites/mutants, batch 1–8 reports, v1 policy,
the active branch and portable report paths against a69af13. Both inventory
guards additionally verify their unchanged baseline and policy hashes.

[commit-plan.json](commit-plan.json) assigns the final files to four logical
steps. [commit-1-message.txt](commit-1-message.txt),
[commit-2-message.txt](commit-2-message.txt),
[commit-3-message.txt](commit-3-message.txt) and
[commit-4-message.txt](commit-4-message.txt) retain the requested co-author and
session trailers. No commit was created because Git could not write index.lock.

| Artifact | SHA-256 |
|---|---|
''' + '\n'.join(hashes) + '''

## Post-session

No post-session checks or externally supplied passes were claimed by the
implementation session itself. All observed results above came from that
session. The live explorer path remained unverified for the reasons recorded
above; the commits were created afterwards from the prepared messages.
'''
# Post-session evidence lives in the ledger's `postSession` entries and in the
# re-verification section appended to REPORT.md after the session. Regenerating
# the report must render the former and preserve the latter, never drop them.
import json as _json
_ledger = _json.loads((report / 'commands.json').read_text())
for _entry in _ledger.get('postSession', []):
    text += f"\nPost-session ledger entry: `{_entry['command']}` exited **{_entry['exitCode']}** ([log]({_entry['log'].removeprefix('docs/reports/batch-9/')})). {_entry.get('note', '')}\n"
_existing = (report / 'REPORT.md').read_text() if (report / 'REPORT.md').exists() else ''
_marker = '\n## Post-session re-verification\n'
if _marker in _existing:
    text = text.rstrip() + '\n' + _marker + _existing.split(_marker, 1)[1]
(report / 'REPORT.md').write_text(text)
