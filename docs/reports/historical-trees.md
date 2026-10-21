# Historical trees

Prepared on `feat/historical-trees` from `main` at `d0e7f10` on 2026-10-21, and rebased onto
`main` at `4dd4428` (#121, #122), with the affected tests rerun there. It is route C
of issue #119: restoring the source-level audit coverage that
[node pin 55229e00](node-pin-55229e00.md) (section 1) withdrew for eleven bundled
Dexy/Phoenix sources. Nothing is tagged or released by it.

## What is pinned, and why

Node 55229e00's typer, like the Scala reference's (sigma-state 6.0.7), rejects eleven
bundled sources, for example `val price: Long = <BigInt expression>`. The deployed contracts
were compiled by an older, laxer compiler, so the sources are kept verbatim and the current
compiler cannot make their trees. This branch pins the trees that older compiler made: node
`a203cc02f585bcbe13c743aee008222b208f3c82` with forge `main` at
`211c5b517d4aedd3a5a6e525999d4134af27fa7a`. Each row records that revision.

| Input | Rows | Parameters | Used by |
|---|---:|---|---|
| [`historical_trees.json`](../../ergo-sandbox/tests/fixtures/historical_trees.json) | 11 | the decompile corpus's | `decompile_corpus.rs`, `identity.rs` |
| [`2026-10-21-historical-sweep/trees.json`](2026-10-21-historical-sweep/trees.json) | 9 | the S02 and I04 sweeps' | `audit_sweep --tree-rows`, `mutation_corpus.rs` |

The sweeps' parameters give `payout.es` and `mint.es` a Long, so they still compile and are
still swept from source; they are in the fixture only. Each row holds the source path and
SHA-256, the parameters, the network, the compiler revision, and the tree as hex with its
SHA-256. The fixture names the tree `treeHex`, not `tree`: the decompile corpus measures
every `tree` key it finds in a fixture, and these rows are not new corpus members.

## What they are not

- **Not the deployed bytes.** The deployed HodlCOMET bank (box `f7798e12…`) matches the
  pinned `phoenix_v1_hodltoken_bank.es` tree as the same program with differing constants,
  not as the same bytes.
- **Not what any current compiler emits.** The current compiler rejects these sources.
  Batch-2 and batch-7 still record them as failed, and
  `decompile_corpus_expectations.json` still carries its `initial-compile-failed`
  correction, which is true for the current compiler.
- **No compile map.** A tree row has no source positions, so a finding has none. A node id is
  a lift-local anchor, as in every sweep.
- **Placeholder parameters.** They are the corpus's and the sweeps' structural values, never
  deployment constants.

## Evidence

1. **The trees are the frozen ones.** All eleven fixture trees come from
   [`batch-10/bundled-after.json`](batch-10/bundled-after.json), which equals
   `batch-9/decompile-measurement.json`. Each hashes to the `tree_sha256` frozen in
   `decompile_corpus_expectations.json`.
2. **The old compiler makes them again.** Forge `211c5b5` was built in its own worktree and
   target dir (Rust 1.95.0, the toolchain the node pins at a203cc02; release profile; 156
   crates in 40 s). [`regenerate.py`](historical-trees/regenerate.py) compiled every source
   with that build's `ergo-es compile` and the recorded parameters and network, and stopped
   unless the tree was the recorded one. All 11 equal their frozen `tree_sha256`; all 9
   sweep trees equal the `treeSha256` of the withdrawn S02 and I04 rows. That is why their
   snippets and messages can be byte for byte the withdrawn ones.
3. **The findings are the withdrawn findings.** [`compare_withdrawn.py`](historical-trees/compare_withdrawn.py)
   compares the new sweep with the S02 and I04 rows at `211c5b5`
   ([output](historical-trees/withdrawn-comparison.txt)). For all nine contracts every field
   is equal but the added `provenance` and `compiler` and the old compile's `attemptErrors`:
   the tree, the parameters, the network, the source hash and every finding with its snippet
   and message. The (lint, node id) sets, `unconstrained-outputs` node 8 in the four banks
   included, are the same. The tree rows are parsed by the current engine (node 55229e00),
   so the finding code is the current one.

| | Withdrawn (S02 / I04) | Restored |
|---|---:|---:|
| Findings | 76 / 76 | 76 (`unchecked-get` 60, `unbound-box-reserves` 12, `unconstrained-outputs` 4) |
| Reviewed S02 sites | 4 | 4 |
| Contracts | 9 | 9 (all complete) |

The findings are identical, so the four reviews stand: `site-review.json` copies the sites,
outcomes and reasons from `git show 211c5b5:docs/reports/batch-2/site-review.json` and
needed no new review.

## What each restored item asserts

- **`decompile_corpus.rs` `historical_trees_are_the_frozen_corpus_trees`**: the fixture's
  rows are the eleven rejected sources, in order. Each tree hashes to its row and to the
  frozen `tree_sha256`; each source hash matches the bundled source; each row's parameters
  are the ones the corpus derives from that source.
- **`decompile_corpus.rs` `historical_trees_round_trip_to_the_frozen_buckets`**: the current
  decompiler and compiler take each pinned tree to the frozen bucket, placeholder count,
  truncation flag and tree hash: 7 byte-identical, 4 recompiles-but-differs. The existing
  `initial-compile-failed` correction is kept.
- **`identity.rs` `phoenix_deployments_and_near_miss_siblings`**: against the deployed bank,
  the pinned `hodltoken_bank` tree is `same_program_with_differing_constants`, and the pinned
  `hodlcoin_bank` tree is `different_program`. Against the deployed proxy, the pinned
  `hodlcoin_proxy` tree is `different_program`. The `hodltoken_proxy` match still compiles
  from source. The test still asserts the exact typer rejection of the three sources it
  matches through a pinned tree.
- **`mutation_corpus.rs` `historical_sweep_is_recorded`**: beside `check_sweep`, which now
  shares its site-review check with it (`check_reviews`). It re-parses and re-audits every
  tree row with all registered lints and compares each record, finding, source hash and tree
  hash. The rows must be exactly the records the typer withdrew from batch-2 and batch-7
  (their `type error:` failures), with the withdrawn finding counts per contract (76), and
  every reported site of the four S02 lints must be reviewed once, with its reason and
  decision in `docs/audit-sweep.md`.

`audit_sweep` takes `--tree-rows rows.json` and audits recorded trees instead of compiling
sources, labelling each row `historical-compile` with its compiler revision. The source path
is unchanged, and so is its output.

`ergo-sandbox/tests/fixtures/decompile_corpus_expectations.json`, the batch-2 and batch-7
sweeps and sites, and every file `manifest.json` or `roadmap_gate.py` pins are untouched.

## Regenerating

```sh
# forge main@211c5b5 (node a203cc02), built in its own worktree and target dir
cargo build --release --locked -p ergo-sandbox --bin ergo-es
python3 docs/reports/historical-trees/regenerate.py --old-ergo-es <that build>/release/ergo-es

# the sweep, then its comparison with the withdrawn rows
cargo run --release --locked -p ergo-sandbox --example audit_sweep -- \
  --tree-rows docs/reports/2026-10-21-historical-sweep/trees.json \
  > docs/reports/2026-10-21-historical-sweep/audit-sweep.json
python3 docs/reports/historical-trees/compare_withdrawn.py
```

The release build of the sweep is byte for byte the committed `audit-sweep.json`.
`site-review.json` carries that file's SHA-256. If the engine ever changes what these trees
lift to, `historical_sweep_is_recorded` fails: record a new dated sweep, do not edit this one.

## For the maintainer

- **What this does not decide.** It audits trees nobody deployed. Route B (auditing the
  deployed mainnet trees, as `identity_phoenix.json` does for the Phoenix bank and proxy)
  remains open, and would need its own reviews wherever its findings differ.
- **The pins can go.** If the node's typer ever accepts these sources again, the
  `initial-compile-failed` correction and these pins are the two things to retire.
- **Not restored.** `payout.es` and `mint.es` have no restored sweep rows because they never
  lost them. The fixture holds only their decompile-corpus trees.

## Checks

Every command forge's CI runs was run on its own, with its exit status recorded, under CI's
build settings: `RUSTFLAGS=-D warnings` and the dev profile at `opt-level=3` with debug
assertions and overflow checks. Rust 1.99.0, Node 22, `npm ci --prefix ui`,
`scripts/requirements-test.txt`, and the node checked out beside forge at 55229e00. Every
cargo command passed `--locked`. Logs are not committed. The run was one script, in a 6 GB
memory scope with 6 build jobs and 4 test threads.

| Check | Command | Exit |
|---|---|---:|
| `fmt` | `cargo fmt --all -- --check` | 0 |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `clippy-cost-trace` | `cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace --locked -- -D warnings` | 0 |
| `tests-default` | `cargo test --workspace --locked --no-fail-fast` | 0 |
| `tests-web-no-default` | `cargo test -p ergo-web --no-default-features --locked --no-fail-fast` | 0 |
| `tests-cost-trace` | `cargo test -p ergo-sandbox -p ergo-web --features cost-trace --locked --no-fail-fast` | 0 |
| `release-gate` | `python3 scripts/test_release.py` | 0 |
| `ci-workflow-gate` | `python3 scripts/test_ci_workflow.py` | 0 |
| `lockfile-action` | `python3 scripts/test_lockfile_action.py` | 0 |
| `roadmap-completed` | `python3 scripts/roadmap_gate.py --through-completed` | 0 |
| `roadmap-p08` | `python3 scripts/roadmap_gate.py --require P08` | 0 |
| `roadmap-ci` | `python3 scripts/roadmap_gate.py --ci` | 0 |
| `contract-suites` | `ergo-es test` on every `examples/tests` and `examples/incidents` suite | 0 |
| `lockfile` | `sha256sum -c --quiet docs/reports/node-pin-55229e00/Cargo.lock.sha256` | 0 |

The workspace and `cost-trace` suites each ran 666 tests, none failed, one ignored (the
external measurement of the decompile corpus); `tests-web-no-default` ran 97; `contract-suites`
ran all 22 suites. After the run, `audit_sweep.rs` lost one redundant rebinding. `fmt` and
`clippy` were run again, and the release build of the sweep is byte for byte the committed
`audit-sweep.json`. The new tests passed in the run: both `decompile_corpus` tests,
`phoenix_deployments_and_near_miss_siblings` and `historical_sweep_is_recorded`
(`roadmap_gate.py --ci` lists it among `mutation_corpus`'s seven).
