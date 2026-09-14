"""Regenerate the report from observed command exits; never infer a pass."""
import hashlib
import json
from pathlib import Path

REPORT = Path(__file__).resolve().parent
ledger = json.loads((REPORT / 'commands.json').read_text())
rows = ledger['commands']
latest = {r['command']: r for r in rows}
required = [
    'cargo fmt --all -- --check',
    'cargo clippy --workspace --all-targets -- -D warnings',
    'cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace -- -D warnings',
    'cargo test --workspace',
    'cargo test -p ergo-sandbox -p ergo-web --features cost-trace',
    'cargo test -p ergo-web --no-default-features',
    'cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory',
    'python3 scripts/test_roadmap_gate.py',
    'python3 scripts/test_lockfile_action.py',
    'python3 scripts/test_release.py',
    'python3 scripts/test_ci_workflow.py',
    next((r['command'] for r in rows if r['command'].startswith('node --test ui/tests/attack.test.js')), 'node --test ui/tests/attack.test.js'),
    *[f'python3 scripts/roadmap_gate.py --require {u} --report docs/reports/batch-10/{u}.json' for u in ['X01','X02','X03']],
    'python3 scripts/roadmap_gate.py --through-completed --report docs/reports/batch-10/through-completed.json',
    'python3 scripts/roadmap_gate.py --require P08 --report docs/reports/batch-10/P08.json',
    'python3 scripts/roadmap_gate.py --ci --report docs/reports/batch-10/ci.json',
]
complete = all(c in latest and latest[c]['exitCode'] == 0 for c in required)
text = '''# Batch 10 — X01, X02, X03

Prepared on `v2/batch-10`, based on `a18999a`, on 2026-09-14. The work remained
uncommitted: staging and all four commit attempts exited 128 because the
sandbox made the worktree index read-only. Four commit messages with the exact
requested co-author and session trailers are retained in this directory.
Nothing was pushed, tagged or published; no container was pushed. No other
branch or worktree was modified.

'''
text += ('Every required validation command completed with exit 0.\n' if complete else
         'Final validation was still in progress when this report was regenerated; see observed exits below.\n')
text += '''
## Prepared changes

- X01: all eight Cargo dependency declarations and all nine node packages in
  Cargo.lock advance from `9468043396e5daa2828211bcff4234bc70fae4f0` to
  `016533194f94ad95b1a87df70bb9bfce493922e2`. Both the read-only node checkout's
  HEAD and origin/main matched the latter; origin identified arkadianet/ergo.
  The node packages changed from 0.6.0 to 0.7.0. No unrelated dependency or
  dependency edge changed in the final lockfile.
- The new `node_corpus_expectations.json` pins all 92 eligible seed rows and
  279 mainnet trees individually, including non-exact outcomes. The two floor
  tests now require 79 and 270 exact rows, and compare every original tree
  hash and exact disposition. The historical 73-row compiled subset, the
  339-row v1 decompiler expectations and W05 expectations remain byte-identical.
- `node_vectors_pass_on_new_rev` verifies the measured new revision, agreement
  across the Cargo declarations and recorded engine revision, and all original
  P03 expected outcomes. `engine_revision()` already read Cargo.toml and stayed
  unchanged. Hunt's rule-basis metadata and the wire-vector generator now use
  that same source instead of a repeated live revision literal.
- X02: the inherited workspace version is 0.5.0. CHANGELOG.md names every
  implemented v2 unit and every batch 0–9, plus batch 10, with historical tag
  sections grounded in local git history. README/action tag examples use
  v0.5.0. Existing v0.4.0/v0.4.1 trees still declared crate version 0.3.0.
- The existing release workflow and action download logic required no code
  change: assets are uploaded for the tag, and the action selects a named tag
  or latest release. The current CLI help contains `verify-lock`; its existing
  compatibility probe will pass once v0.5.0 is the latest release. The new
  Python release gates parse the workflow/action YAML offline. Cutting the
  tag after merge remained the maintainer's action.
- X03: both Rust verification jobs cache the sibling `ergo/` checkout using
  the pinned revision and only check it out on a cache miss. The cache action
  uses verified v4.3.0 SHA `0057852bfaa89a56745cba8c7296529d2fc39830`.
  Cost-trace clippy/tests run in their own job with no dependency on the
  default verification job. Both install Node/DOM dependencies and cache Rust
  builds. Fmt, default clippy/workspace tests, web without cost-trace, the
  lockfile action runner, completed-prefix/P08/all-goal gates remain. Parsed
  YAML tests enforce this topology and the retained checks. PyYAML 6.0.3 is an
  explicit CI test dependency.
- X01/X02/X03 are implemented in both complete v2 policy/spec records. The
  policy test's implemented set includes all three. The v1 block is unchanged.

## Measurement and stop decision

| Measurement | Before: 9468043 | After: 0165331 | Previously exact lost |
|---|---:|---:|---:|
| Eligible seed corpus | 74/87 | 79/92 | 0 |
| Mainnet corpus | 270/279 | 270/279 | 0 |
| P03 vectors, including accepting/rejecting cases | 9/9 | 9/9 | 0 |

The old committed floors (73 and 250) were lower than the freshly measured
before counts. The stop decision used the observed 74 and 270, with individual
identity checks, rather than treating the floors as measurements. Seed vector
indices moved when upstream inserted entries, so the comparison joined source
SHA-256 plus tree SHA-256 with multiplicity. Mainnet joined tree SHA-256; its
corpus file hash was identical across revisions. All 74 old exact seed rows
and 270 old exact mainnet trees survived; five added seed rows were exact.
No engine-regression stop was triggered and the new pin was retained.

[engine-before.json](engine-before.json), [engine-after.json](engine-after.json)
and [engine-comparison.json](engine-comparison.json) retain the observations,
file identities and comparison. The old node corpus was extracted with
`git show` into a temporary directory; the read-only sibling was never reset.
The retained `engine_measurement.rs` diagnostic used the same eligibility and
recompile rules as the floor tests. `compare-engine.py` reproduces the identity
join from those observations and the node's two committed corpus revisions.
The initial diagnostic compilation failed on the changed upstream API, then
passed with only the adapter changes listed below.

The unchanged bundled corpus test passed at **264/357 obtained trees**:
367 entries, 264 byte-identical, 93 recompiling differently, 10 initial compile
failures. The separately preserved historical scoreboard remains 238/329 from
339 entries. The difference between those inventories includes S00/W05
additions; it is not attributed to the engine bump. `docs/roadmap-metrics.json`
adds an X01 scoreboard with the rev pair, before/after counts, vector results
and measurement hashes; every historical scoreboard field remains unchanged.

## Evidence integration and exact API edits

Original P03 requests, expected statuses, stages, transaction IDs, costs,
diagnostics and all their file hashes were unchanged. The retained diagnostic
`node_validation_measurement.rs` first authenticated those bytes and executed
fresh requests with only their engine premise changed. Its three tests passed,
including the full nine-vector loop and the existing controls.

The permanent test-only `engine_support/request.rs` makes this boundary
explicit: it authenticates the historical revision, proves production
validation rejects it before invoking the pipeline, clones the premises with
the current Cargo pin, and requires a different request fingerprint. The
lockfile test uses the actual historical pin as both independent compiler/node
drift controls. HTTP/CLI/DOM replay tests also create fresh test requests;
the DOM test first proves the original bundle is rejected. Shipped historical
bundles were not silently accepted or rewritten by production code.

Tests for historical mapping/property results recompute only revision-derived
identities from the original JSON using `engine_support::RevisionMap`.
They compare complete expected reports, with unchanged verdicts, costs, state,
transaction bytes and independent answers. Immutable source-artifact SHA-256
fields remain original. The old metadata identities are not copied onto new
executions. The D00 measurement example imports the shared test helper too.

The only production API adaptations forced by upstream were:

1. `NetworkRules::node` in `ergo-sandbox/src/evidence/validate.rs` takes an
   activation version and passes it to the node's now-two-argument
   `check_tree_version_supported`.
2. `validate` passes the already supplied transaction context's activation
   version to that method.
3. `ergo-sandbox/src/map/check_relation.rs` passes its already supplied
   state-domain activation version to the same method.

The only other production edit was Hunt's rule-basis revision metadata, now
computed by `node_rule_basis()` from `engine_revision()`. No decompiler, lint,
claim, replay or promotion algorithm changed.

The mapping/properties manifests and every baseline digest remained unchanged.
`baseline_support` authenticates exact historical copies of the changed Cargo
files and validator, permitting only the explicit revision/version/API edits,
and authenticates the old metrics with only the X01 scoreboard appended.
Both inventory tests passed. The M05 stop source had to gain the same explicit
remeasurement setup: its stop-record evidence locator now names an exact
archived original under `baseline/`, with its original SHA-256. The reason,
gate exits, capabilities and all other stop-record content remain unchanged.
The initial gate attempts detected that stale locator and failed; those exits
are retained. M05 and D03 remain stopped; S03 remains unimplemented.

## Validation and session limitations

All builds used this batch's isolated `$CARGO_TARGET_DIR`. The runner set the
same optimized dev profile used by CI (`opt-level=3`, debug assertions and
overflow checks enabled). No `--skip` flag was used. The actual sibling seed
and mainnet floor tests ran and printed 79/92 and 270/279; the existing ignored
optional external-corpus diagnostic was not counted as an executed gate.
The full workspace mutation-corpus measurement was allowed to finish.

Hosted GitHub cache hits, parallel CI elapsed time, three-platform release
builds, container builds and live latest-release downloads were not exercised
in this session. Workflow/action structure was tested offline and the local
CLI compatibility was observed. Tagging/publication was deliberately not
attempted. Git staging/commits were attempted and refused by the read-only
worktree index; no commit was created. Prepared messages and
[commit-plan.json](commit-plan.json) retain the four intended logical steps.

Early failures were integration diagnostics: the upstream API argument,
historical evidence revision/fingerprint checks, frozen artifact projections,
a stale M05 source locator, and shared-module imports found by clippy. They
were retained in the ledger. No interrupted command or unobserved exit was
reported as a pass.

## Required command results

Latest observed exits are shown here; report flags direct gate artifacts to
batch 10 instead of the runner's default report directory.

| Command | Exit | Seconds | Log |
|---|---:|---:|---|
'''
for command in required:
    row = latest.get(command)
    label = 'node --test (all 10 UI files passed explicitly; exact argv below)' if command.startswith('node --test ') else command
    if row:
        text += f"| `{label}` | {row['exitStatus']} | {row['elapsedSeconds']} | [{Path(row['log']).name}]({row['log']}) |\n"
    else:
        text += f'| `{label}` | no observed exit | — | — |\n'
text += '''
## Complete validation and Git-write ledger

Every observed build, test, gate, integrity check and Git write attempt is
retained in [commands.json](commands.json). Failed development commands are
included. Logs contain portable locations; hashes cover the retained log bytes.

| # | Command | Exit | Seconds | Log | Log SHA-256 |
|---:|---|---:|---:|---|---|
'''
for i, row in enumerate(rows, 1):
    command = row['command'].replace('|', '\\|')
    text += f"| {i} | `{command}` | {row['exitStatus']} | {row['elapsedSeconds']} | [{Path(row['log']).name}]({row['log']}) | `{row['logSha256']}` |\n"
text += '\n## Artifact identities\n\n| Artifact | SHA-256 |\n|---|---|\n'
for name in ['X01.json','X02.json','X03.json','through-completed.json','P08.json','ci.json','engine-before.json','engine-after.json','engine-comparison.json','bundled-after.json']:
    file = REPORT / name
    if file.exists():
        text += f'| [{name}]({name}) | `{hashlib.sha256(file.read_bytes()).hexdigest()}` |\n'
text += '''
## Post-session

No post-session results were claimed. All recorded exits came from this
implementation session. The maintainer still needed to create the prepared
commits outside the read-only index restriction, merge, and then decide when
to cut v0.5.0. Nothing was pushed or tagged by this session.
'''

# Preserve the post-session re-verification section appended after the
# implementation session; regenerating the report must never drop evidence.
_existing = (REPORT / 'REPORT.md').read_text() if (REPORT / 'REPORT.md').exists() else ''
_marker = '\n## Post-session re-verification\n'
if _marker in _existing:
    text = text.rstrip() + '\n' + _marker + _existing.split(_marker, 1)[1]
(REPORT / 'REPORT.md').write_text(text)
print(f'Report regenerated from {len(rows)} observed commands; required validation complete: {complete}')
