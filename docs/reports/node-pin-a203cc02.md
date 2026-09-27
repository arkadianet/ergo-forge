# Node pin a203cc02

Prepared on `chore/node-pin-a203cc02` from `main` at `188bdd5` on 2026-09-27. The branch
moves forge's engine from node `016533194f94ad95b1a87df70bb9bfce493922e2` to
`a203cc02f585bcbe13c743aee008222b208f3c82`, the node's 0.8.0 release commit on its
`main`. Nothing is merged, tagged or released by it. The resolutions below are for the
maintainer's decision.

## What changes

- **Pins.** All eight node declarations (six in `Cargo.toml`, the two dev dependencies in
  `ergo-sandbox/Cargo.toml`) and the revision `node_vectors_pass_on_new_rev` expects name
  the new revision. Cargo re-resolved once: the nine node packages move to it and from
  0.7.0 to 0.8.0, and `ergo-state` gains a `serde_json` dependency. No other lockfile
  entry changes. [`Cargo.lock.sha256`](node-pin-a203cc02/Cargo.lock.sha256) records the
  result; the last check below confirms that no later command moved it.
- **API adaptations** the node forced, and no other production change:
  1. `ergo-sandbox/src/eval.rs`: `ReductionContext` has a new `validation_settings`
     field. The sandbox passes its default, every rule enabled, as the node's own
     `minimal()` does.
  2. `ergo-sandbox/src/evidence/validate.rs`: the same field in `ProtocolParams`. Forge's
     serialized `Parameters` keep their fields, so no evidence file changes.
  3. `ergo-sandbox/src/inspect.rs` and 4. `ergo-sandbox/src/decompile/lift.rs`:
     `Expr::Unparsed` now wraps `UnparsedErgoTree { bytes, validation_error }`, so the
     byte count reads `bytes.bytes.len()`.
- **Corpus expectations.** `node_corpus_expectations.json` takes the new
  `engineRevision`; every row is unchanged.
- **Baseline allow-list** (`ergo-sandbox/tests/baseline_support/mod.rs`). The edits it
  admits on the Cargo files and the validator now include this migration's: the node
  packages at 0.8.0 (X01 admitted 0.7.0), `ergo-state`'s `serde_json` line and the
  `validation_settings` line. Nothing else is admitted.
- **Cost corrections** (`ergo-sandbox/tests/engine_support/mod.rs`, used by
  `node_validation.rs`, `mapping_context_boundary.rs` and `claim_replay.rs`), below.
- **The registry example's two refused inserts** (`examples/tests/registry.test.json` and its
  generator `examples/tests/gen/proofs.py`), below.
- **`CHANGELOG.md`**: the 0.5.0 section names the new pin.

## Measurements

| Measurement | 0165331 | a203cc02 | Previously exact lost |
|---|---:|---:|---:|
| Eligible seed corpus, exact | 79/92 | 79/92 | 0 |
| Mainnet corpus, exact | 270/279 | 270/279 | 0 |
| External compiled fixtures (ignored diagnostic) | 373 byte-identical, 27 differ, 1 fails | identical, row for row | 0 |
| P03 vectors, outcome | 9/9 | 9/9 | 0 |

[`engine-after.json`](node-pin-a203cc02/engine-after.json) is the copy-in diagnostic
(`docs/reports/batch-10/engine_measurement.rs`) at the new pin. With each row's `error`
removed it equals the committed expectations row for row, so only the fixture's revision
changes. [`external-before.json`](node-pin-a203cc02/external-before.json) and
[`external.json`](node-pin-a203cc02/external.json) are the ignored
`external_compiled_fixtures_measurement` at each pin, and they are identical. The corpora
are the node's own `test-vectors` at each pin, as in CI.

Under roadmap v2's `engine-regression` rule (an exact round trip or a P03 vector lost),
nothing is lost: every exact row stays exact and every P03 vector keeps its outcome. One
P03 vector's recorded cost moves, with three other historical costs.

## Cost changes and their Scala evidence

| Record | Transaction | Recorded (0165331) | a203cc02 | Scala 6.0.6 |
|---|---|---:|---:|---:|
| P03 `storage-rent-acceptance` | `6ea6db3f…` | 12,105 | 12,150 | 12,150 |
| M05 `original` | `61deab92…` | 14,175 | 14,176 | 14,176 |
| M05 `different-true-expression` | `86f149da…` | 14,180 | 14,185 | 14,185 |
| USE incident: prior block cost in block 1,868,204 | the five below | 194,317 | 195,757 | 195,757 |

The USE incident's five preceding transactions, the recorded costs being the differences
of the bundle's source-recorded cumulative costs (0, 14,059, 100,833, 123,759, 136,478,
194,317) and the new ones the replay's at the new pin:

| Transaction | Recorded | a203cc02 | Scala 6.0.6 |
|---|---:|---:|---:|
| `10ff5c82…` | 14,059 | 14,059 | 14,059 |
| `ce0f75d6…` | 86,774 | 88,034 | 88,034 |
| `2b5a56dc…` | 22,926 | 23,106 | 23,106 |
| `6a4d184c…` | 12,719 | 12,719 | 12,719 |
| `30c1cacc…` | 57,839 | 57,839 | 57,839 |

**Why they move.** Between the pins the node brought its costing to Scala parity (its
JIT-cost conformance series, node PRs #337, #339–#345 and #348). The P03 change is node commit
`69337be6`: the storage-rent shortcut is now charged 50 in block-cost units, Scala's
`StorageContractCost`, where 0165331 charged 50 JIT units, 5 in block cost; hence +45.
The M05 and USE changes come from the same series' interpreter charges; they were not
bisected to single commits. In every case the new value is exactly what the Scala
reference node charges for the same transaction in the same context, so each change
corrects the older engine.

**How the Scala values were obtained.** Each transaction, with its input boxes and
context, was taken from forge's own records: P03's request, the M05 stop results and the
USE bundle's source-recorded transactions. Every transaction id matches the record. Each
was validated with the Scala reference node's own stateful transaction validation
(`ErgoTransaction.statefulValidity` with `ErgoInterpreter`): ergo-core, ergo-wallet and
avldb 6.0.6 built from the Scala node's `v6.0.6` tag (ergoplatform/ergo, commit `23aabead`),
sigma-state 6.0.6 from Maven Central, JDK 17.0.17. Every
transaction was accepted. [`scala/`](node-pin-a203cc02/scala/) holds the evidence:
- `<set>/input/` holds the vectors. Each vector's `params.fixture` names its parameter
  file, now under `params/`, by the path the harness used, with its blake2b-256.
- `<set>/scala/` holds the results, with a manifest of the jars loaded and their sha256.

P03 and M05 ran with the parameters their requests record. USE ran with mainnet's
parameters. Their cost fields (input 2,407, data input 100, output 298, token access 100,
maximum block cost 8,001,091) equal the bundle's source-recorded epoch parameters.

**How the tests apply them.** The historical records are not rewritten.
`engine_support::COST_CORRECTIONS` lists the four, keyed, each with its historical and its
corrected cost. `corrected_cost` returns the corrected cost only when the record holds
exactly the historical one, panics on any other value, and leaves every other record as it
is:
- `node_validation.rs` compares each P03 `totalBlockCost` with the corrected cost;
- `mapping_context_boundary.rs` corrects the two M05 executions' expected costs;
- `claim_replay.rs` corrects the USE bundle's `priorBlockCost` before replaying it.

So these tests pass only if the engine charges exactly the Scala value.

## The registry example's refused inserts

Two scenarios of `examples/tests/registry.test.json` offer the registry a proof that
authenticates no insert: a lookup proof for a new name, and an insert of a name that already
exists. They expected `needsProof`, the registrar's key left after the insert branch fails; at
the new pin both end in a runtime error, `AvlTree.insert failed on a pre-v3 ErgoTree`.

That is the Scala node's rule. The registry's tree is pre-v3 (it uses no v6 method, so nothing
stamps it v3), and on a pre-v3 tree a refused `AvlTree.insert` throws: sigma-state 6.0.6
`CErgoTreeEvaluator.insert_eval` raises `syntax.error` when `insertRes.isFailure` and the tree is
not v3 or later (sigmastate-interpreter#908); only a v3 tree gets `None`. The node follows it
since commit `b99df6ae`, whose JVM conformance fixtures map the error to the JVM's
`InterpreterException` (`ergo-sigma/tests/it/cost_ledger_fixtures.rs`). The old engine returned
`None`, so the script fell through to the registrar's key; on mainnet such a spend fails outright.

The two scenarios now expect `error`, and their names say why. The generator writes the same, and
the contract's source is unchanged. The other four scenarios keep their verdicts.

## For the maintainer's decision

1. **The cost corrections.** Recommended: accept. The alternative, re-recording the four
   records at the new pin, rewrites historical evidence, which forge's evidence rules
   avoid. The correction keeps each record and states the change with its evidence.
2. **The baseline allow-list extension.** Recommended: accept. It admits exactly this
   migration's Cargo and validator edits.
3. **The registry example's two verdicts.** Recommended: accept. They now state what the
   reference node does. The alternative, keeping `needsProof` by making the example a v3 tree,
   would need a v6 method in its source, changing the contract and its decompile fixtures.
4. **Roadmap metrics.** X01 appended an `X01` scoreboard to `docs/roadmap-metrics.json`,
   and `baseline_support` admits only the `X01` and `S03` keys. This bump is outside the
   roadmap's units, so the branch adds no scoreboard; the measurements are in this report.
   Adding one would mean extending the admitted keys. Recommended: leave as is.
5. **`docs/immutability.md:51-52`** names `9468043…` as the revision "all node crates in
   this batch share". It describes batch 4's lock files; X01 left it, and so does this
   branch.

## Checks

The branch was checked with every command forge's CI runs, each on its own with its exit
status recorded ([`checks.txt`](node-pin-a203cc02/checks.txt), one log per check), under
CI's build settings: `RUSTFLAGS=-D warnings` and the dev profile at `opt-level=3` with debug
assertions and overflow checks. Every cargo command passed `--locked`, and the node
checkout beside forge was at the pin, as CI provides it.

| Check | Command | Exit | Log |
|---|---|---:|---|
| `fmt` | `cargo fmt --all -- --check` | 0 | [fmt.log](node-pin-a203cc02/fmt.log) |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | [clippy.log](node-pin-a203cc02/clippy.log) |
| `clippy-cost-trace` | `cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace --locked -- -D warnings` | 0 | [clippy-cost-trace.log](node-pin-a203cc02/clippy-cost-trace.log) |
| `tests-default` | `cargo test --workspace --locked --no-fail-fast` | 0 | [tests-default.log](node-pin-a203cc02/tests-default.log) |
| `tests-web-no-default` | `cargo test -p ergo-web --no-default-features --locked --no-fail-fast` | 0 | [tests-web-no-default.log](node-pin-a203cc02/tests-web-no-default.log) |
| `tests-cost-trace` | `cargo test -p ergo-sandbox -p ergo-web --features cost-trace --locked --no-fail-fast` | 0 | [tests-cost-trace.log](node-pin-a203cc02/tests-cost-trace.log) |
| `release-gate` | `python3 scripts/test_release.py` | 0 | [release-gate.log](node-pin-a203cc02/release-gate.log) |
| `ci-workflow-gate` | `python3 scripts/test_ci_workflow.py` | 0 | [ci-workflow-gate.log](node-pin-a203cc02/ci-workflow-gate.log) |
| `lockfile-action` | `python3 scripts/test_lockfile_action.py` | 0 | [lockfile-action.log](node-pin-a203cc02/lockfile-action.log) |
| `roadmap-completed` | `python3 scripts/roadmap_gate.py --through-completed` | 0 | [roadmap-completed.log](node-pin-a203cc02/roadmap-completed.log) |
| `roadmap-p08` | `python3 scripts/roadmap_gate.py --require P08` | 0 | [roadmap-p08.log](node-pin-a203cc02/roadmap-p08.log) |
| `roadmap-ci` | `python3 scripts/roadmap_gate.py --ci` | 0 | [roadmap-ci.log](node-pin-a203cc02/roadmap-ci.log) |
| `contract-suites` | `ergo-es test on every examples/tests and examples/incidents suite (contract-tests.yml)` | 0 | [contract-suites.log](node-pin-a203cc02/contract-suites.log) |
| `lockfile` | `sha256sum -c node-pin-a203cc02/Cargo.lock.sha256` | 0 | [lockfile.log](node-pin-a203cc02/lockfile.log) |

The workspace and `cost-trace` suites each ran 76 test binaries and 580 tests, none failed, one ignored
(the external measurement above); `contract-suites` ran all 22 example suites.

Logs name portable locations: `<repo>` for the checkout, `$CARGO_TARGET_DIR`, `<tmp>` and
`<home>`.
