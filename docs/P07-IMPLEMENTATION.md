# P07 — obligation reports

The five existing detectors and their observations are unchanged. `Audit`,
`StaticAnalysis`, the inspect/compile DTOs, and the CLI audit report now expose an
obligation queue. Each group retains full findings, including every lift/IR
anchor and any legacy scenario reproduction. Typed literal context-variable
reads share a witness-schema key within the target; unrecognized expressions
remain separate. Keys are versioned and scoped by the report's target premises.

`ReviewPriority` names the static ranking; `Severity` and the serialized
`severity` field remain compatibility aliases for review priority. They do not
supply impact to a property claim. Confirmed claims retain the declared
extraction accounting and report unassessed impact as unknown.

`audit_with_contracts` exports all supplied scripts, execution positions,
completeness, recovered code, singleton evidence and the explicit conditional
scope. Input order is canonicalized without inferring membership. Each
conditional discharge names its supporting anchor, companion, binding and
reason. Exact deployment identity remains explicitly unknown at this lifted
code boundary; callers can retain exact companion/state/policy provenance in
the evidence case. Missing premises are not repaired or inferred.

`evidence::claim::obligation_report(case, context, decisions, bundles)` retains
that context and the full evidence case. Suppressions require a nonempty reason,
obligation key and current premise fingerprint. Changed constants, code,
source identity, state, policy or replay bundles invalidate saved decisions;
stale decisions and every observation remain visible. This is a caller review
decision, never a proof of safety. Conditional discharges and suppressions do
not remove anchors.

Property results occupy a separate array and are always derived by fresh P05
replay. An associated observation does not inherit a claim's status. Stored
reports cannot substitute for replay bundles.

The hashed curated manifest is
`ergo-sandbox/tests/fixtures/evidence/precision.json`. Its six benign labels
cover the six P07 acceptance cases; its two positives reference the unchanged
P05 bundles. Rejection/accepted-behavior examples use explicitly synthetic
scenario execution; the fixed paid control and both positives use node replay.
This measures curated report precision, not live-protocol precision.
The duplicate fixture explicitly authors twelve lifted occurrences because
source compilation eliminates repeated reads. All twelve lift-local anchors
must survive as one group; it makes no fabricated IR-anchor claim.

The roadmap clarifies that `precision.json` is P07's separate manifest,
preserving P03's exact node-vector inventory as P04/P05 already do. No policy
threshold, frozen baseline, detector semantics, corpus row or verdict was amended. P08 is not implemented. Work remains uncommitted.

## Verification

All commands used `CARGO_TARGET_DIR=./target-p00`. The blocks below reproduce Cargo per-target summaries and the roadmap runner's final summaries; full captured stdout/stderr is in the local `target-p00/p07-verification/transcript.txt`.

`cargo fmt --all -- --check` — exit **0**

No output.

`cargo clippy --workspace --all-targets -- -D warnings` — exit **0**

```text
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
```

`cargo test --release -p ergo-sandbox --test obligation_reports` — exit **0**

```text
   Compiling ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `release` profile [optimized] target(s) in 2.03s
     Running tests/obligation_reports.rs (target-p00/release/deps/obligation_reports-c0f2e7b0ec212eb3)

running 4 tests
test duplicate_observations_keep_all_anchors_in_one_obligation ... ok
test conditional_discharge_exports_the_full_premise_set ... ok
test suppression_invalidates_on_premise_change ... ok
test curated_precision_has_positive_and_negative_controls ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`cargo test --workspace` — exit **0**

```text
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-76bc11a3ccec096d)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-3884ae02c3af3345)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-195b124d96f08bf4)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-9e780f962f2cc987)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-867a538af62c8c59)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
     Running tests/claim_replay.rs (target-p00/debug/deps/claim_replay-1eeca24b641ba3d1)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-96f38a1b20e781ff)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s
     Running tests/cli_json.rs (target-p00/debug/deps/cli_json-78217134dfb35b8f)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli_match.rs (target-p00/debug/deps/cli_match-99b415b5dbad97d4)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compile_params.rs (target-p00/debug/deps/compile_params-04d6d589f9b5181b)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/compose.rs (target-p00/debug/deps/compose-0c1763dbe7a1c00b)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_ast.rs (target-p00/debug/deps/decompile_ast-b97b3cbe689ccef2)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_corpus.rs (target-p00/debug/deps/decompile_corpus-fd4b9d025a16633b)
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.33s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-b90d7ecfb604ed9d)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-05385b0f9ce21643)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-0049cecfe2605568)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/drain.rs (target-p00/debug/deps/drain-a587fae7e43b0e13)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 49.16s
     Running tests/drain_promotion.rs (target-p00/debug/deps/drain_promotion-1a4eafd2d804d913)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.37s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-6279ded19698364d)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
     Running tests/evidence_wire.rs (target-p00/debug/deps/evidence_wire-f9ca542c54b2ac21)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hot_spots.rs (target-p00/debug/deps/hot_spots-f639f4dc7fb50cb7)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hunt.rs (target-p00/debug/deps/hunt-8038c0be4d44b697)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/identity.rs (target-p00/debug/deps/identity-d7eaadb79846ffcd)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/ingest.rs (target-p00/debug/deps/ingest-9a59f52ffb9032b7)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/map.rs (target-p00/debug/deps/map-c3de162f75160169)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-91836e4d1ac72d18)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-daff6d8031bc25ad)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-d249f4aea675b5ca)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 813.29s
     Running tests/node_validation.rs (target-p00/debug/deps/node_validation-2db5a0b67406d036)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/obligation_reports.rs (target-p00/debug/deps/obligation_reports-faafc363a9c19b1a)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-3626131eca09286f)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/play.rs (target-p00/debug/deps/play-432bab585fd5d216)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-a556daeceeb5f2b7)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-f4287d780e542da1)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-65e9a0bc6b1d0069)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-75ae46c51f516316)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-22e8472d542ae703)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/transaction_proofs.rs (target-p00/debug/deps/transaction_proofs-b597196e4304bbf9)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/tree.rs (target-p00/debug/deps/tree-29fee86d46fc2c00)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-fcd4913da558a07b)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-4136b3ac20645e9f)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-00f71cf05665f353)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_web-58a61400bbf06183)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
     Running unittests src/main.rs (target-p00/debug/deps/ergo_web-5f03d3a11e4efa70)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/http.rs (target-p00/debug/deps/http-de5d928124428075)
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
     Running tests/map.rs (target-p00/debug/deps/map-3720d2a9f2f3f122)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
   Doc-tests ergo_sandbox
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
   Doc-tests ergo_web
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`python3 scripts/roadmap_gate.py --require P07` — exit **0**

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: passed: required tests executed
P05: passed: required tests executed
P06: passed: required tests executed
P07: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/P07.json
```

`python3 scripts/roadmap_gate.py --through-completed` — exit **0**

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: passed: required tests executed
P05: passed: required tests executed
P06: passed: required tests executed
P07: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/through-completed.json
```

`python3 scripts/roadmap_gate.py --all-goals` — exit **1**

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: passed: required tests executed
P05: passed: required tests executed
P06: passed: required tests executed
P07: passed: required tests executed
P08: unimplemented: registered product unit is not implemented
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/all-goals.json
```

Sum of actual workspace per-target summaries: **455 passed / 0 failed**. The existing external-node decompiler measurement remains ignored; no test was newly ignored.

P00–P07 pass. P08 is the only unimplemented unit in the full-goal report; there is no `missing-gate` status. No P07 work remains incomplete and no stop record was required.
