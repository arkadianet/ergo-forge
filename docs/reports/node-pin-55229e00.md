# Node pin 55229e00

Prepared on `chore/node-pin-55229e00` from `main` at `211c5b5` on 2026-10-21. The branch
moves forge's engine from node `a203cc02f585bcbe13c743aee008222b208f3c82` (0.8.0) to
`55229e0040b83b57521334030fe2842e85d43422` (0.13.1, the node's `release: prepare 0.13.1`
commit on `release/0.13.1`). It is the same kind of change as
[node pin a203cc02](node-pin-a203cc02.md). Nothing is tagged or released by it.

The work was first done against node `a80f54c8e03eb8239fe7491e28e85dd3815a5ee8`, on
`main` after 0.13.0. 0.13.1 contains it, and between the two the node changes only release
metadata, CI workflows, its changelog, `ergo-api` (which forge does not use) and dependency
versions (`dns-lookup` 3 → 4, `futures-util`, `proptest`, `rayon`, `rustix`). No crate forge
depends on changes code. The Scala probes below date from that work. Every measurement and
check in this report was rerun at `55229e00`.

The rule for every changed expectation is that the Scala reference node decides. Each one
below was checked against the Scala 6.0.8 node (ergo-core and ergo-wallet built from the
`v6.0.8` tag, sigma-state 6.0.7, JDK 17.0.17, scala-cli 1.12.1), and in each case node
55229e00 now agrees with it. The probes and their output are in
[`node-pin-55229e00/scala/`](node-pin-55229e00/scala/).

## What changes

- **Pins.** All eight node declarations (six in `Cargo.toml`, the two dev dependencies in
  `ergo-sandbox/Cargo.toml`), the revision `node_vectors_pass_on_new_rev` expects and the
  `engineRevision` of `node_corpus_expectations.json` name the new revision. The node
  moved to `num-bigint` 0.5, so the workspace's `num-bigint` does too. Cargo re-resolved
  once, and the node's dependency graph moved with it.
  [`Cargo.lock.sha256`](node-pin-55229e00/Cargo.lock.sha256) records the result, and the
  last check below confirms that no later command moved it.
- **Toolchain.** The node declares `rust-version = "1.99.0"`. Every check below ran on Rust
  1.99.0. CI's `dtolnay/rust-toolchain` installs current stable and needs no change.
- **API adaptations** the node forced, and no other production change:
  1. `ergo-sandbox/src/evidence/validate.rs`: `ProtocolParams` now carries the block
     version (the node takes the activated script version from it, block version − 1) and
     its rule state. `Parameters::node` takes the block version, and the caller passes the
     request's activated script version + 1. The rule state is the default: every rule
     enabled, no missing core parameters, no block-rule inputs. `ReemissionRuleInputs`
     gains `check_rules` and `emission`; see *Emission-box allocation* below.
  2. `ergo-sandbox/src/box_build.rs`: `EvalBox` has a `lazy_vals` cache. A parsed box gets
     the node's `EvalBoxLazyVals::from_candidate`, an unparsed one the default.
  3. `ergo-sandbox/src/map/check_relation.rs` and `ergo-sandbox/src/properties/evaluate.rs`:
     a candidate's registers are read through `additional_registers()`, and an `ErgoTree`
     literal names `reserved_header_bits: 0`.
  4. `ergo-web/src/routes/explain.rs`: `Cand`/`Cor` take the node's `SigmaChildren`
     (`.into()`).
  5. Tests: the same API in `decompile_ast.rs` and `decompile_robustness.rs`.
     `evidence_wire.rs` builds its stale-cache candidate with the node's
     `ErgoBoxCandidate::from_trusted_raw_parts`, because the fields it used to mutate are now
     private. It checks the same rejection.
- **Expectations that move to the reference**, each below with its evidence.
- **Baseline allow-list** (`ergo-sandbox/tests/baseline_support/mod.rs`). It now admits this
  migration's edits: the `num-bigint` line in `Cargo.toml`, and the `validate.rs` edits of
  item 1 above, each spelled out. For `Cargo.lock`, the node's new dependency graph is too
  large to admit entry by entry, so the lockfile must be byte for byte the reviewed one:
  its SHA-256 must equal `node-pin-55229e00/Cargo.lock.sha256`.
- **Clippy (toolchain).** Rust 1.99's clippy adds `needless_borrows_for_generic_args`.
  `ergo-sandbox/src/drain.rs:839` passes `hit_total` rather than `&hit_total`. This is
  forced by the toolchain the pin needs, not by the node's API.
- **`CHANGELOG.md`**: the 0.5.0 section names the new pin.

## Measurements

| Measurement | a203cc02 | 55229e00 | Previously exact lost |
|---|---:|---:|---:|
| Eligible seed corpus, exact | 79/92 | 79/92 | 0 |
| Mainnet corpus, exact | 270/279 | 270/279 | 0 |
| External compiled fixtures (ignored diagnostic) | 373 byte-identical, 27 differ, 1 fails | identical | 0 |
| Bundled decompile corpus | 367 rows | 11 rows now fail to compile (below) | 7 byte-identical, 4 recompiles-but-differs |
| P03 vectors, outcome | 9/9 | 8 as recorded; `canonical-encoding-rejection` moves to the reference (below) | — |

[`engine-after.json`](node-pin-55229e00/engine-after.json) is the copy-in diagnostic
(`docs/reports/batch-10/engine_measurement.rs`) at the new pin, and
[`external.json`](node-pin-55229e00/external.json) is the ignored
`external_compiled_fixtures_measurement`. The corpora are the node's own `test-vectors` at
the pin, checked out beside forge as CI does.

The node reordered its `compile_seed.json` between the two pins: the file's bytes differ,
but its eligible rows are the same trees. Measured as a multiset of (tree SHA-256, exact),
the seed and mainnet corpora are identical at both pins. So
`node_corpus_expectations.json` is re-recorded in the new order, with the same 92 + 279
(tree, exact) pairs. That test zips rows by position. The external fixtures are identical
as a multiset of (source, tree, outcome).

## Expectations that move to the reference

### 1. The typer rejects eleven bundled sources

Node 55229e00's ErgoScript typer rejects a declared type that the bound expression does not
have. The Scala compiler (sigma-state 6.0.7 `SigmaCompiler`, tree version 3, forge's
parameter substitution) rejects the same eleven sources with the same error at the same
line ([`scala/typer`](node-pin-55229e00/scala/typer/)). As a control, eight neighbouring
Dexy/Phoenix sources compile under Scala byte for byte to forge's trees.

| Source (`examples/contracts/…`) | Before | Error (node and Scala) |
|---|---|---|
| `dexy/bank/payout.es` | byte-identical | Expected SInt, got SLong |
| `dexy/lp/pool/mint.es` | byte-identical | Expected SInt, got SLong |
| `hodlcoin/phoenix/phoenix_v1_hodlcoin_bank.es` | recompiles-but-differs | Expected SLong, got SBigInt |
| `hodlcoin/phoenix/phoenix_v1_hodlcoin_proxy.es` | recompiles-but-differs | Expected SLong, got SBigInt |
| `hodlcoin/phoenix/phoenix_v1_hodltoken_bank.es` | byte-identical | Expected SLong, got SBigInt |
| `hodlcoin/phoenix/phoenix_v1_hodltoken_fee.es` | byte-identical | Expected SBoolean, got SSigmaProp |
| `hodlcoin/phoenix/phoenix_v1_hodltoken_feeTest_mainnet.es` | byte-identical | Expected SBoolean, got SSigmaProp |
| `hodlcoin/phoenix/phoenix_v1_hodltoken_feeTest_testnet.es` | byte-identical | Expected SBoolean, got SSigmaProp |
| `hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodlerg_bank.es` | recompiles-but-differs | Expected SLong, got SBigInt |
| `hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodlerg_proxy.es` | recompiles-but-differs | Expected SLong, got SBigInt |
| `hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodltoken_bank.es` | byte-identical | Expected SLong, got SBigInt |

For example, `phoenix_v1_hodlcoin_bank.es` writes
`val price: Long = (reserveIn.toBigInt * precisionFactor) / hodlCoinsCircIn`. The deployed
contracts were compiled by an older, laxer compiler. The sources are kept as they are,
because they are copies of deployed contracts.

Where this moves coverage:

- **Decompile corpus**: the eleven rows are now expected as `initial-compile-failed`. The
  expectations file is a frozen roadmap baseline (`roadmap_gate.py` requires its bytes
  unchanged), so it stays as recorded, and `decompile_corpus.rs` corrects the eleven rows
  by id, each only from a compiled outcome. No compiled tree changed: every other row is
  identical.
- **Deployed-contract sweeps** ([S02](batch-2/audit-sweep.json),
  [I04](batch-7/audit-sweep.json)). The sweep's own parameters give `payout.es` and
  `mint.es` a Long, so those two still compile, unchanged. The other nine are now failed
  records with the node's exact error, so the population is 105 complete and 19 failed.
  Their findings are withdrawn, not reclassified. The nine rows are taken from a fresh
  `audit_sweep` run at the pin, and no other row changed. Every other I04 row equals that
  run. So does every other S02 row but one, `vectors/mutable-upgrade-digest/vulnerable.es`,
  whose `upgrade-hook` finding came with I04 after S02 was recorded. Each site review
  re-pins its sweep's SHA-256. `docs/audit-sweep.md` gains a dated section
  listing all of it.

  | Contract | Findings withdrawn (S02 / I04) | S02 reviewed sites withdrawn |
  |---|---:|---|
  | `hodlcoin/phoenix/phoenix_v1_hodlcoin_bank.es` | 12 / 12 | `unconstrained-outputs` node 8, benign |
  | `hodlcoin/phoenix/phoenix_v1_hodlcoin_proxy.es` | 15 / 15 | — |
  | `hodlcoin/phoenix/phoenix_v1_hodltoken_bank.es` | 11 / 11 | `unconstrained-outputs` node 8, benign |
  | `hodlcoin/phoenix/phoenix_v1_hodltoken_fee.es` | 0 / 0 | — |
  | `hodlcoin/phoenix/phoenix_v1_hodltoken_feeTest_mainnet.es` | 0 / 0 | — |
  | `hodlcoin/phoenix/phoenix_v1_hodltoken_feeTest_testnet.es` | 0 / 0 | — |
  | `hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodlerg_bank.es` | 12 / 12 | `unconstrained-outputs` node 8, benign |
  | `hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodlerg_proxy.es` | 15 / 15 | — |
  | `hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodltoken_bank.es` | 11 / 11 | `unconstrained-outputs` node 8, benign |
  | **Total** | **76 / 76** | **4** |

- **`identity.rs` `phoenix_deployments_and_near_miss_siblings`**: of the four sources it
  compiles, only `phoenix_v1_hodltoken_proxy.es` still compiles, and it still matches the
  deployed proxy (same program, differing constants). For the other three, the test now
  asserts the exact typer rejection. The bank's deployed-tree match and both sibling
  near-miss checks are lost.
- **`audit_context.rs` `phoenix_bank_binds_the_proxy_bank_successor`**: the bank companion
  is now the bank tree deployed on mainnet (`fixtures/identity_phoenix.json`, HodlCOMET
  box `f7798e12…`), not a compilation of the source. Every assertion holds unchanged: one
  discharge, the bank successor at `OUTPUTS(0)`, bound as `SelfSuccessor` via
  `SELF.propositionBytes`.

Restoring source-level coverage for these contracts is a follow-up for the maintainer,
either by auditing the deployed trees or by pinning the historical trees.

### 2. An overlong input count is accepted

`node-vectors/canonical-encoding-rejection` writes the input count as the overlong VLQ
`0x81 0x00`, and expected `node-rejected` / `NonCanonical`. The Scala node parses the
transaction, passes `statelessValidity` and accepts it in `statefulValidity` at block cost
12,101. Its re-serialization is canonical, and its id is `2241d787…`: this is the
canonically encoded `accepted-keyless-spend`. Node 55229e00 accepts it with the same id and
cost. The vector and its manifest stay as recorded. Like #115's cost corrections,
`engine_support::OUTCOME_CORRECTIONS` corrects the expectation by key, and only where the
row holds exactly the historical one.

### 3. A Boolean-rooted tree is refused

`sandbox_eval.rs` `standalone_tree_input_must_consume_all_bytes` evaluated the tree
`10810001d17300` and expected Pass. That tree has a segregated constant (count written
`0x81 0x00`), no size bit, and a `Boolean` root. Scala's `ErgoTreeSerializer` refuses it:
rule 1001, the deserialized script must be a SigmaProp, and the failure is fatal because
there is no size bit. So a box holding it cannot be parsed, and a transaction creating such
an output is refused. Node 55229e00 refuses that transaction at the same stage ("sizeless
ErgoTree root has type SBoolean, expected SigmaProp (CheckDeserializedScriptIsSigmaProp,
rule 1001)", node commit `d70685e7`). The sandbox's evaluation of the standalone tree now
ends in `error` ("expected SigmaProp script result, got Bool(true)"), and the test expects
that. Forge's own `inspect::parse_tree` and `parse_tree_complete` still accept the tree,
more leniently than either node; that is left as a follow-up.

### 4. `maxTokensPerBox` is 255

The USE incident's replay re-derives its epoch parameters from the chain
(`docs/p05-recovery`). The node now gives `maxTokensPerBox` 255 where it gave 122 (node
commit `7642ac1d`). Sigma-state 6.0.7's `SigmaConstants.MaxTokens` is 255: the pinned jar's
bytecode reads `sipush 255`. The recorded bundle keeps 122, and `claim_replay.rs` corrects
it by key with `engine_support::with_corrected_max_tokens`, which applies only where the
record holds exactly 122.

## Emission-box allocation is not checked

Node 55229e00's re-emission rules can also check the emission box's allocation. To do that
it needs the emission constants (`ReemissionRuleInputs::emission`). A forge network-rule
description does not carry them, so forge passes `emission: None` and does not run that
check. It still asks the node to check the re-emission rules it was given
(`check_rules: true`), as before. `validate.rs` says so where it builds the inputs.
Carrying the emission constants is a follow-up for the maintainer.

## For the maintainer's decision

1. **The four reference moves.** Recommended: accept. Each one states what the Scala
   reference node does, and keeps its historical record.
2. **Lost source-level coverage** of the nine swept and eleven bundled Dexy/Phoenix
   sources. Recommended: accept here, and restore it in a follow-up.
3. **The lockfile digest in the baseline allow-list.** Recommended: accept. It admits
   exactly one lockfile, the one reviewed here.
4. **Emission-box allocation.** Recommended: accept the stated gap, and carry the emission
   constants in a follow-up.
5. **Roadmap v2's `engine-regression` rule** ("X01 loses exact round-trips or P03 vectors.
   Stay on the old rev."). Read literally, it trips, in two places:
   - Seven bundled sources that round-tripped byte for byte no longer compile.
   - One P03 vector changes outcome.

   Both changes are the engine moving to the reference, not getting worse:
   - No decompiler round trip of a compilable tree changed.
   - Seed, mainnet and the external fixtures are identical.
   - The P03 vector's new outcome is the Scala node's.

   Recommended: keep the pin, and read the rule as guarding against engine regressions
   measured against the reference.

## Checks

The branch was checked with every command forge's CI runs, each on its own with its exit
status recorded ([`checks.txt`](node-pin-55229e00/checks.txt), one log per check), under
CI's build settings: `RUSTFLAGS=-D warnings` and the dev profile at `opt-level=3` with debug
assertions and overflow checks. Rust 1.99.0, Node 22, `npm ci --prefix ui`,
`scripts/requirements-test.txt`, and the node checked out beside forge at the pin, as CI
provides it. Every cargo command passed `--locked`. The run was one script, at `55229e00`,
after every edit on the branch (6 build jobs and 4 test threads, in a 7 GB memory scope, to
share the machine).

| Check | Command | Exit | Log |
|---|---|---:|---|
| `fmt` | `cargo fmt --all -- --check` | 0 | [fmt.log](node-pin-55229e00/fmt.log) |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | [clippy.log](node-pin-55229e00/clippy.log) |
| `clippy-cost-trace` | `cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace --locked -- -D warnings` | 0 | [clippy-cost-trace.log](node-pin-55229e00/clippy-cost-trace.log) |
| `tests-default` | `cargo test --workspace --locked --no-fail-fast` | 0 | [tests-default.log](node-pin-55229e00/tests-default.log) |
| `tests-web-no-default` | `cargo test -p ergo-web --no-default-features --locked --no-fail-fast` | 0 | [tests-web-no-default.log](node-pin-55229e00/tests-web-no-default.log) |
| `tests-cost-trace` | `cargo test -p ergo-sandbox -p ergo-web --features cost-trace --locked --no-fail-fast` | 0 | [tests-cost-trace.log](node-pin-55229e00/tests-cost-trace.log) |
| `release-gate` | `python3 scripts/test_release.py` | 0 | [release-gate.log](node-pin-55229e00/release-gate.log) |
| `ci-workflow-gate` | `python3 scripts/test_ci_workflow.py` | 0 | [ci-workflow-gate.log](node-pin-55229e00/ci-workflow-gate.log) |
| `lockfile-action` | `python3 scripts/test_lockfile_action.py` | 0 | [lockfile-action.log](node-pin-55229e00/lockfile-action.log) |
| `roadmap-completed` | `python3 scripts/roadmap_gate.py --through-completed` | 0 | [roadmap-completed.log](node-pin-55229e00/roadmap-completed.log) |
| `roadmap-p08` | `python3 scripts/roadmap_gate.py --require P08` | 0 | [roadmap-p08.log](node-pin-55229e00/roadmap-p08.log) |
| `roadmap-ci` | `python3 scripts/roadmap_gate.py --ci` | 0 | [roadmap-ci.log](node-pin-55229e00/roadmap-ci.log) |
| `contract-suites` | `ergo-es test on every examples/tests and examples/incidents suite (contract-tests.yml)` | 0 | [contract-suites.log](node-pin-55229e00/contract-suites.log) |
| `lockfile` | `sha256sum -c --quiet docs/reports/node-pin-55229e00/Cargo.lock.sha256` | 0 | [lockfile.log](node-pin-55229e00/lockfile.log) |

The workspace and `cost-trace` suites each ran 661 tests, none failed, one ignored (the
external measurement above); `contract-suites` ran all 22 example suites. Both measurements
are identical to the ones taken at `a80f54c8`: `engine-after.json` row for row (371 rows, seed
79/92 and mainnet 270/279 exact), with only its `engineRevision` changed, and `external.json`
(401 rows) byte for byte. `node_corpus_expectations.json` equals `engine-after.json`'s rows
with `error` removed.
