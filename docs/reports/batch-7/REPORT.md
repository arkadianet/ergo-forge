# Batch 7 — S04 and I04

S04 and I04 were implemented in the `v2/batch-7` worktree based on `1d53b2f`.
The implementation session could not create commits: Git staging and commit
attempts exited **128** because `$WORKTREE_GIT_DIR` was mounted read-only.
The changes remained in this worktree; no push was attempted. Four prepared
`commit-N-message.txt` files retain the requested co-author/session trailers.

## Implementation

- Hunt retains its six height/output samples and adds six immobilisation
  samples: absent registers, minimum output value, and explicit block-budget
  reduction, each with attacker and preserve outputs. Results record the
  12-probe set/cap, a 16-box cap per collection, truncation, output values,
  consumed cost/budget, exhaustion, recovered register reads, and reached
  absent reads when the engine reports `Option.get` failure. SELF and positional
  input/output/data-input registers are cleared in the absence samples.
- Literal positional accesses are scaffolded within the cap. Computed or
  over-cap positions and partial recovery set `truncated`; dynamic search is
  outside this unit. Supplied data inputs beyond the cap are omitted with
  truncation recorded. Every result remains synthetic and `nodeValidated: false`.
  `unspendable-under-probe` is a scoped error observation. When nothing passes,
  the strongest aggregate statement is **not spendable under these probes**.
- Minimum output values use the pinned node's serialized box size, including
  output index and the value's variable-length encoding, times its default
  `min_value_per_byte` (360). The least fixed point is used, rather than a
  hard-coded dust amount. The test sends the outputs through the node's
  `validate_structural`, then verifies that one nanoERG less is refused.
- Cost samples use `eval::DEFAULT_COST_LIMIT` (8,001,091 block units), checked
  against `ProtocolParams::mainnet_default`, and the existing cost accumulator.
  An additional synthetic nested-fold case exhausts that budget. This measures
  reduction only; full transaction overhead and aggregate validation are not
  inferred. No engine limit or cost accounting was changed.
- Read displays a **static** rent estimate for the supplied box or synthetic
  SELF via `rent::estimate`, preserving the frozen storage period (1,051,200)
  and fee factor (1,250,000 nanoERG/byte). The line says “after four years,
  anyone may claim this box for the rent” through the miner storage-rent path,
  cites both constants and **P03:storage-rent-acceptance**, and states that
  collection is not predicted. The P03 case is the node-validated basis;
  the new estimate itself is not node-validated.
- The independent `upgrade-hook` sibling lint was smaller in behavioral scope
  than extending `trust-assumptions`: all nine existing lint implementations
  stayed unchanged. It recognises an output bytes/blake2b256 comparison to a
  SELF or positional-box byte register and a same-script continuation that
  omits that register's equality on its path. It names recognised sigma
  keys/register anchors, or reports “unguarded (no recognised sigma guard on
  this path)”. LOW is review priority, not evidence of an exploitable upgrade.
- `I04-01-v1` adds one mechanical pair under `examples/mutants/i04/v1/`, solely
  within the append-only `staticLintPairs` namespaces. Removing the continuation's
  register equality produces exactly one finding; the control has zero for
  this lint in both lift modes. Additional precision tests cover bytes, hashes,
  positional boxes, key/register guards, wrong-output and wrong-branch equality,
  unused hooks, and token transfers into a hook destination.
- The class-12 mutable-upgrade row appends `lint:upgrade-hook`; the catalogue
  was regenerated and checklist reports findings with static provenance and
  recovery anchors. Prior catalogue fields/examples were retained, with
  appended notes distinguishing the S00 baseline from the S04 and I04 additions.
- Read's existing hunt panel renders families, caps, truncation, output values,
  absent reads, cost/budget and exhaustion. A small `HuntRead` renderer keeps
  changes to `app.js` and `index.html` local. DOM tests cover static rent,
  its P03 citation, synthetic labels, scoped misses, escaping and clearing.
  S04's DOM extra uses the existing gate test for failed/empty/skipped runs.
- Both complete v2 policy records are `implemented: true` in the roadmap and
  spec; the implemented-set assertion adds only S04 and I04. W04, Write Run,
  eval/explain routes, Cargo features, CI and protected evidence modules were
  not changed.

## Sweep and review

The final release sweep covered **124 files, 114 complete analyses, zero partial
analyses, and 10 existing compile failures**. This bundled population includes
teaching vectors and recipes; it is not the spec's historical 79-deployment
population. No live deployment identity was verified.

The initial recogniser reported two sites. Review found that Rosen
`Commitment.es` transferred X-RWT into a permit whose script is selected by R7;
that token transfer did not establish continuing commitment state. The
recogniser was narrowed to same-script continuations, and a negative control
was added. The final sweep reports **one authored mutable-upgrade site**, reviewed
as a true-positive static pattern. No precision-regression stop was needed after
narrowing. The unchanged ten compile failures remained outside analysis.

Both sweeps and every reported site review are retained:
[initial sweep](initial-audit-sweep.json), [initial review](initial-site-review.json),
[final sweep](audit-sweep.json), [final review](site-review.json), and
[the appended I04 section](../../audit-sweep.md). The extended
`deployed_corpus_sweep_is_recorded` checks both S02's unchanged report and I04's
report against source/tree hashes, compilation outcomes and exact current sites.

## Verification and command ledger

Every Cargo command used this batch's `$CARGO_TARGET_DIR`. The wrapper retained
observed exits and portable stdout/stderr in [commands.json](commands.json) and
`logs/`. Read-only inspection and file editing were not counted as test passes.
No interrupted command is counted as a pass. Required gate subprocess commands,
outputs and hashes are retained in [S04.json](S04.json) and [I04.json](I04.json).
Python executable paths in their command arrays are rendered as `python3` for
portability; captured output and its hashes are unchanged.

The initial S04 builds exited 101 on signed/unsigned scenario-value types and
then on a JSON-vs-Option test assertion. Both were corrected before behavior
verification. The first authority precision test used a testnet key under
mainnet compilation; its fixture network was corrected. The first cost stress
source used a nested fold directly inside arithmetic, which the pinned typer
refused; binding the subtotal to a local value made that synthetic test compile.
These failures are retained, not reported as passes. A draft inline report-path
audit also flagged the two standard interpreter shebangs. Its enclosing shell
continued, so its standalone exit was not captured; the helpers lost their
unneeded shebangs. The first recorded report audit then found Rust toolchain
paths in the retained failed-build log (exit 1); those paths were rendered as
`$RUSTUP_HOME`, and the corrected audit passed.

Existing expectation changes were limited to the expanded hunt sample counts
(six to twelve in sandbox, CLI and HTTP tests), the scoped CLI miss wording,
and the production DOM fixture's additional renderer/response fields. No
existing audit fixture expectation was changed. No required named roadmap
gate was run with ignored or zero selected tests accepted as evidence.

<!-- command-ledger -->

All nine required command forms finished with observed exit **0**, including
the complete workspace run with the one requested skip (**973.12 seconds**).
The final DOM run passed 34 tests. Both S04 and I04 gate reports have
`passed: true`; their seven and eight subprocesses, respectively, exited 0.

| Command | Observed exit | Evidence |
| --- | ---: | --- |
| `cargo fmt --all` | 0 | [fmt-step1](logs/fmt-step1.log) |
| `cargo test -p ergo-sandbox --test immobilisation --test hunt --test cli_hunt` | 101 | [s04-target](logs/s04-target.log) |
| `cargo test -p ergo-sandbox --test immobilisation --test hunt --test cli_hunt` | 101 | [s04-types](logs/s04-types.log) |
| `cargo test -p ergo-sandbox --test immobilisation --test hunt --test cli_hunt` | 0 | [s04-behavior](logs/s04-behavior.log) |
| `cargo fmt --all` | 0 | [fmt-step2](logs/fmt-step2.log) |
| `cargo test -p ergo-sandbox --test upgrade_hook` | 101 | [i04-precision](logs/i04-precision.log) |
| `cargo test -p ergo-sandbox --test mutation_corpus upgrade_hook_mutant_is_caught -- --exact` | 0 | [pairs](logs/pairs.log) |
| `cargo test -p ergo-sandbox --test upgrade_hook --test audit --test more_lints --test delegated_reserves` | 0 | [lint-precision](logs/lint-precision.log) |
| `bash -c 'cargo run --release -p ergo-sandbox --example audit_sweep > docs/reports/batch-7/audit-sweep.json'` | 0 | [sweep](logs/sweep.log) |
| `bash -c 'cargo run --release -p ergo-sandbox --example audit_sweep > docs/reports/batch-7/audit-sweep.json'` | 0 | [sweep-narrowed](logs/sweep-narrowed.log) |
| `cargo test -p ergo-sandbox --test immobilisation` | 101 | [s04-boundaries](logs/s04-boundaries.log) |
| `python3 scripts/render_vectors.py` | 0 | [vectors](logs/vectors.log) |
| `cargo test -p ergo-web --test http` | 0 | [http](logs/http.log) |
| `cargo test -p ergo-sandbox --test immobilisation cost_probe_reports_engine_exhaustion_at_the_existing_block_budget -- --exact` | 0 | [s04-cost](logs/s04-cost.log) |
| `cargo fmt --all` | 0 | [fmt-final-edit](logs/fmt-final-edit.log) |
| `cargo fmt --all -- --check` | 0 | [fmt](logs/fmt.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy](logs/clippy.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/evidence-replay.test.js ui/tests/hunt-read.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 0 | [dom](logs/dom.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | [roadmap-tests](logs/roadmap-tests.log) |
| `python3 scripts/test_lockfile_action.py` | 0 | [lockfile-tests](logs/lockfile-tests.log) |
| `git add ergo-sandbox/src/hunt.rs ergo-sandbox/src/rent.rs ergo-sandbox/src/bin/ergo-es.rs ergo-sandbox/tests/hunt.rs ergo-sandbox/tests/cli_hunt.rs ergo-sandbox/tests/immobilisation.rs ergo-web/src/dto.rs ergo-web/tests/http.rs` | 128 | [stage-step1](logs/stage-step1.log) |
| `git commit -F docs/reports/batch-7/commit-1-message.txt` | 128 | [commit-step1](logs/commit-step1.log) |
| `git add ergo-sandbox/src/audit/lints/upgrade_hook.rs ergo-sandbox/src/audit/lints/mod.rs ergo-sandbox/src/audit/mod.rs ergo-sandbox/tests/upgrade_hook.rs ergo-sandbox/tests/mutation_corpus.rs examples/mutants/i04 examples/mutants/mutants.json examples/mutants/answer-key.json examples/mutants/README.md docs/security/vectors.json docs/security/VECTORS.md docs/audit-sweep.md docs/reports/batch-7/audit-sweep.json docs/reports/batch-7/site-review.json docs/reports/batch-7/initial-audit-sweep.json docs/reports/batch-7/initial-site-review.json` | 128 | [stage-step2](logs/stage-step2.log) |
| `git commit -F docs/reports/batch-7/commit-2-message.txt` | 128 | [commit-step2](logs/commit-step2.log) |
| `git add ui/app.js ui/index.html ui/hunt-read.js ui/claim-labels.js ui/tests/hunt-read.test.js ui/tests/claim-labels.test.js scripts/roadmap_gate.py` | 128 | [stage-step3](logs/stage-step3.log) |
| `git commit -F docs/reports/batch-7/commit-3-message.txt` | 128 | [commit-step3](logs/commit-step3.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 | [workspace](logs/workspace.log) |
| `python3 scripts/roadmap_gate.py --require S04 --report docs/reports/batch-7/S04.json` | 0 | [gate-s04](logs/gate-s04.log) |
| `python3 scripts/roadmap_gate.py --require I04 --report docs/reports/batch-7/I04.json` | 0 | [gate-i04](logs/gate-i04.log) |
| `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory` | 0 | [inventories](logs/inventories.log) |
| `python3 docs/reports/batch-7/check-integrity.py` | 0 | [integrity](logs/integrity.log) |
| `git diff --check` | 0 | [whitespace](logs/whitespace.log) |
| `python3 scripts/render_vectors.py --check` | 0 | [vectors-final](logs/vectors-final.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/evidence-replay.test.js ui/tests/hunt-read.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 0 | [dom-final](logs/dom-final.log) |
| `python3 scripts/roadmap_gate.py --require S04 --report docs/reports/batch-7/S04.json` | 0 | [gate-s04-final](logs/gate-s04-final.log) |
| `git add docs/ROADMAP.md docs/superpowers/specs/2026-09-13-forge-roadmap-v2.md scripts/test_roadmap_gate.py docs/reports/batch-7` | 128 | [stage-step4](logs/stage-step4.log) |
| `git add -f docs/reports/batch-7/logs` | 128 | [stage-logs](logs/stage-logs.log) |
| `git commit -F docs/reports/batch-7/commit-4-message.txt` | 128 | [commit-step4](logs/commit-step4.log) |
| `python3 docs/reports/batch-7/check-integrity.py` | 0 | [final-integrity](logs/final-integrity.log) |
| `git diff --check` | 0 | [final-whitespace](logs/final-whitespace.log) |
| `python3 (draft inline report-path audit)` | no observed exit | [draft-report-path-audit](logs/draft-report-path-audit.log) |
| `python3 docs/reports/batch-7/check-report.py` | 1 | [final-report-audit](logs/final-report-audit.log) |
| `python3 docs/reports/batch-7/check-report.py` | 0 | [final-report-audit-corrected](logs/final-report-audit-corrected.log) |
| `git diff --check` | 0 | [report-whitespace](logs/report-whitespace.log) |

Gate reports (after command-path portability normalization):

| Report | SHA-256 |
| --- | --- |
| [S04.json](S04.json) | `01c2eee56e71a2e5708b7b5efa45536b8715e54c363546ab0a2f0a7cd91c0f36` |
| [I04.json](I04.json) | `78de3306b2c791bb5b444886851f5bd65d19702a0e9503722441718eeee52b83` |

The integrity check confirmed the frozen v1 policy SHA-256 remained
`45c68423f26d61f290b8b36b44a747015381fd840e86368c9b9a26b4acb2c491`.
All staging/commit attempts, including the final report/log step, exited 128.

<!-- /command-ledger -->

## Session limitations and preserved boundaries

Git could not create `index.lock` in the read-only worktree metadata. No alternate
index, Git directory, other branch or worktree was used to bypass that restriction.
The staging lists in the command ledger define the prepared logical steps.

The known local seed-corpus mismatch test was skipped exactly as requested,
with its source unchanged. Existing ignored workspace tests remained governed
by their existing annotations. Browser behavior was checked with linkedom DOM
tests; a Chromium session was not run. No live node/explorer acceptance was
performed or claimed by these units. The existing P03 fixture remains the basis
for the rent explanation.

The lint could not decide reachability, signing ability, arbitrary predicates,
computed boxes, dynamic registers, token-only continuations or external-contract
constraints. Its path walk is bounded to 64 alternatives and depth 128; a miss
remains unchecked. Hunt could not establish universal liveness, valid creation,
worst-case transaction cost or full transaction acceptance. Its malformed and
cost-heavy synthetic contexts were experiments, not valid-chain box claims.

The frozen `roadmap-policy:v1`, mapping/property manifests, answer-key history,
prior static pairs, and `docs/reports/batch-1..6/` were preserved. The integrity
script records these checks independently of the unchanged inventory guards.

## Post-session verification

No post-session verification was claimed by this implementation session. The
retained commit messages and logs are available for the later authorised commit
step outside the read-only Git metadata restriction.

## Post-session re-verification

After the implementation session ended, the same tree was re-verified outside
its sandbox with this batch's build directory (`$CARGO_TARGET_DIR`), before the
commits were created from the four prepared messages. Observed exits:

| Command | Exit |
|---|---:|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `python3 scripts/test_roadmap_gate.py` | 0 |
| `python3 scripts/test_lockfile_action.py` | 0 |
| `node --test ui/tests/*.test.js` (34 passed) | 0 |
| `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory` | 0 |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 |
| `python3 scripts/roadmap_gate.py --require S04 --report docs/reports/batch-7/S04.json` | 0 |
| `python3 scripts/roadmap_gate.py --require I04 --report docs/reports/batch-7/I04.json` | 0 |

The filtered output of the workspace run and both gate runs is committed as
[`logs/workspace-post-session.log`](logs/workspace-post-session.log) and
recorded as the `postSession` entry of `commands.json`. After the branch was
rebased onto main (batch 6 merged), clippy, the workspace suite and both gates
were run again on the rebased tree and exited 0; the committed gate reports
are from that rebased run and hash as:

| Report | SHA-256 |
|---|---|
| `S04.json` | `feec73e2cae84ae9c488d38023098ac43778cf3903c024d56d9b64df4bb5a252` |
| `I04.json` | `c7b272ae13a9509f1bd905baa24a9b99348f2b89fccfb7dc1342e606a42d72ef` |
