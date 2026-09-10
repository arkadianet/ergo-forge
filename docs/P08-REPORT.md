# P08 — browser evidence replay

P08 adds a saved-evidence import/replay operation in Read. `POST /api/v2/replay`
accepts the existing P05 `ReplayBundle`, calls its pure replay runner under the
shared engine budget, and returns `{apiVersion: 2, result: <CLI report>}`. The
nested report schema and all execution/property decisions remain unchanged.

Read accepts a bundle or extracts only the bundle from a saved report. It renders
exact result, execution and property-claim scopes, input and per-premise origins,
and every recorded missing premise, including residual gaps on accepted results.
Source-recorded inputs do not establish historical unspentness; hypothetical
inputs remain visibly labelled. Legacy preflight keeps its existing labels.

The real HTTP/CLI tests and production DOM import tests reuse the P05 manifest's
public USE positive and authored paid-sale control. The incomplete test copy
explicitly removes headers and parameters. No frozen fixture is edited. A real
configured-explorer trap receives no connection during complete or incomplete
replay. Malformed/stored-result requests cannot import acceptance authority.

The existing CI workflow now checks P08 and all goals. No gate correction or
threshold amendment was needed. The policy changes only P08's `implemented`
flag and `completedThrough`; the appended roadmap note records completion.
No numeric baseline, corpus row, cap, verdict or denominator was changed.
Everything is left uncommitted as requested.

`python3 scripts/roadmap_gate.py --all-goals` exits **0**. This is the whole
plan's gate passing for the first time: scoreboard and P00–P08 all pass.
`--require P08` and `--through-completed` also exit 0.

All commands use `CARGO_TARGET_DIR=./target-p00`. Actual output excerpts:


```text
$ cargo fmt --all -- --check
[exit 0]
```

```text
$ cargo clippy --workspace --all-targets -- -D warnings
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Checking ergo-web v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-web)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.46s
[exit 0]
```

```text
$ cargo test --release -p ergo-web --test evidence_replay
running 4 tests
    Blocking waiting for file lock on build directory
test replay_request_never_fetches_or_broadcasts ... ok
   Compiling ring v0.17.14
   Compiling rustls v0.23.43
   Compiling rustls-webpki v0.103.15
   Compiling ureq v3.4.1
   Compiling ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `release` profile [optimized] target(s) in 37.43s
test http_and_cli_replay_have_identical_semantic_results ... ok
test legacy_preflight_does_not_use_node_claim_badge ... ok
test historical_and_hypothetical_state_labels_are_visible ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 37.77s

[exit 0]
```

```text
$ node --test ui/tests/evidence-replay.test.js
TAP version 13
# Subtest: historical_and_hypothetical_state_labels_are_visible
ok 1 - historical_and_hypothetical_state_labels_are_visible
  ---
  duration_ms: 21659.835334
  type: 'test'
  ...
# Subtest: legacy_preflight_does_not_use_node_claim_badge
ok 2 - legacy_preflight_does_not_use_node_claim_badge
  ---
  duration_ms: 4.209887
  type: 'test'
  ...
# Subtest: saved verdicts are discarded and errors clear earlier claims
ok 3 - saved verdicts are discarded and errors clear earlier claims
  ---
  duration_ms: 4.422853
  type: 'test'
  ...
1..3
# tests 3
# suites 0
# pass 3
# fail 0
# cancelled 0
# skipped 0
# todo 0
# duration_ms 21869.941122
[exit 0]
```

```text
$ python3 scripts/roadmap_gate.py --require P08
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: passed: required tests executed
P05: passed: required tests executed
P06: passed: required tests executed
P07: passed: required tests executed
P08: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/P08.json
[exit 0]
```

```text
$ python3 scripts/roadmap_gate.py --through-completed
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: passed: required tests executed
P05: passed: required tests executed
P06: passed: required tests executed
P07: passed: required tests executed
P08: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/through-completed.json
[exit 0]
```

```text
$ python3 scripts/roadmap_gate.py --all-goals
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: passed: required tests executed
P05: passed: required tests executed
P06: passed: required tests executed
P07: passed: required tests executed
P08: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/all-goals.json
[exit 0]
```

`cargo test --workspace` exits 0: **459 passed, 0 failed**, versus the P07
baseline of 455 passed. One pre-existing optional external-node measurement is
ignored; P08 introduces no ignored test. Actual per-target summaries follow.

```text
$ cargo test --workspace
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-76bc11a3ccec096d)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-3884ae02c3af3345)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-195b124d96f08bf4)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-9e780f962f2cc987)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-867a538af62c8c59)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
     Running tests/claim_replay.rs (target-p00/debug/deps/claim_replay-1eeca24b641ba3d1)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-96f38a1b20e781ff)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s
     Running tests/cli_json.rs (target-p00/debug/deps/cli_json-78217134dfb35b8f)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli_match.rs (target-p00/debug/deps/cli_match-99b415b5dbad97d4)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compile_params.rs (target-p00/debug/deps/compile_params-04d6d589f9b5181b)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/compose.rs (target-p00/debug/deps/compose-0c1763dbe7a1c00b)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/decompile_ast.rs (target-p00/debug/deps/decompile_ast-b97b3cbe689ccef2)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_corpus.rs (target-p00/debug/deps/decompile_corpus-fd4b9d025a16633b)
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.32s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-b90d7ecfb604ed9d)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-05385b0f9ce21643)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-0049cecfe2605568)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/drain.rs (target-p00/debug/deps/drain-a587fae7e43b0e13)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 56.82s
     Running tests/drain_promotion.rs (target-p00/debug/deps/drain_promotion-1a4eafd2d804d913)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.10s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-6279ded19698364d)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
     Running tests/evidence_wire.rs (target-p00/debug/deps/evidence_wire-f9ca542c54b2ac21)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/hot_spots.rs (target-p00/debug/deps/hot_spots-f639f4dc7fb50cb7)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/hunt.rs (target-p00/debug/deps/hunt-8038c0be4d44b697)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/identity.rs (target-p00/debug/deps/identity-d7eaadb79846ffcd)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
     Running tests/ingest.rs (target-p00/debug/deps/ingest-9a59f52ffb9032b7)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
     Running tests/map.rs (target-p00/debug/deps/map-c3de162f75160169)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-91836e4d1ac72d18)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-daff6d8031bc25ad)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-d249f4aea675b5ca)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 873.30s
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
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-22e8472d542ae703)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/transaction_proofs.rs (target-p00/debug/deps/transaction_proofs-b597196e4304bbf9)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/tree.rs (target-p00/debug/deps/tree-29fee86d46fc2c00)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/triage.rs (target-p00/debug/deps/triage-fcd4913da558a07b)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-4136b3ac20645e9f)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-00f71cf05665f353)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_web-58a61400bbf06183)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
     Running unittests src/main.rs (target-p00/debug/deps/ergo_web-5f03d3a11e4efa70)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/evidence_replay.rs (target-p00/debug/deps/evidence_replay-1f84eb8193967d89)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.59s
     Running tests/http.rs (target-p00/debug/deps/http-de5d928124428075)
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
     Running tests/map.rs (target-p00/debug/deps/map-3720d2a9f2f3f122)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
   Doc-tests ergo_sandbox
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
   Doc-tests ergo_web
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
[exit 0]
```

```text
$ cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings
   Compiling ring v0.17.14
   Compiling rustls v0.23.43
    Checking rustls-webpki v0.103.15
    Checking ureq v3.4.1
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.49s
[exit 0]
```

The explicit cost-trace suite also exits 0: **388 passed, 0 failed**, with the same pre-existing ignored external measurement.

```text
$ cargo test -p ergo-sandbox --features cost-trace
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-b8deb5d2dc4f01d8)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-300623d0c0afdfe6)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-d133cca64c44808b)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-058b63216fb7f65f)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-bb4bbaa8e2e794ef)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
     Running tests/claim_replay.rs (target-p00/debug/deps/claim_replay-0118b7fa1985ae35)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-fa151cfe99805052)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
     Running tests/cli_json.rs (target-p00/debug/deps/cli_json-365a240149b18506)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli_match.rs (target-p00/debug/deps/cli_match-055cd2e1d34343e5)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compile_params.rs (target-p00/debug/deps/compile_params-ce8afeefc29a4c74)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/compose.rs (target-p00/debug/deps/compose-e046ced8e4bfb969)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_ast.rs (target-p00/debug/deps/decompile_ast-2c84dee672ea33d8)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_corpus.rs (target-p00/debug/deps/decompile_corpus-ac411beffd0db75d)
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.06s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-ceb7f1b226f0970f)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-7ae82f97ca6e16f8)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-3f8f56eb04c9f479)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/drain.rs (target-p00/debug/deps/drain-e7a59f90bc3cbecc)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 48.53s
     Running tests/drain_promotion.rs (target-p00/debug/deps/drain_promotion-9f31f24da9351875)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-85e71ecb74d87f95)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
     Running tests/evidence_wire.rs (target-p00/debug/deps/evidence_wire-c69a5670dc1b4a4d)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hot_spots.rs (target-p00/debug/deps/hot_spots-dbddc138469fb98a)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hunt.rs (target-p00/debug/deps/hunt-6e10479a46d99e70)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/identity.rs (target-p00/debug/deps/identity-ddc813331c6bc61c)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/ingest.rs (target-p00/debug/deps/ingest-5784c8e63feacfc0)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/map.rs (target-p00/debug/deps/map-bd6dad821f7e6893)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-4490cc9ef6d4aecb)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-6bf9fcb826123f12)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-6d9c5b862c2bdb0c)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 823.10s
     Running tests/node_validation.rs (target-p00/debug/deps/node_validation-5505a2f7ecdb1408)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/obligation_reports.rs (target-p00/debug/deps/obligation_reports-b623640b5dc59713)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-052151606c965359)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/play.rs (target-p00/debug/deps/play-757e3f1c59a6eece)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-491b80850096b172)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-e76e152b6bdfc861)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-d9bd071b6fa929e1)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-945e8d355d7e0a4e)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-fa6444531f6214f9)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/transaction_proofs.rs (target-p00/debug/deps/transaction_proofs-7781a003e7eb6794)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/tree.rs (target-p00/debug/deps/tree-b06c25c34df1ba95)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-91c30f193f7c666c)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-c1780c8de143a28c)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-1f02e6af081086ec)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
   Doc-tests ergo_sandbox
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
[exit 0]
```

Full command stdout/stderr and SHA-256 identities are preserved in
[target-p00/p08-verification/verification.json](../target-p00/p08-verification/verification.json)
and the adjacent logs. The gate's machine reports are
[P08.json](../target-p00/roadmap-gates/P08.json),
[through-completed.json](../target-p00/roadmap-gates/through-completed.json), and
[all-goals.json](../target-p00/roadmap-gates/all-goals.json).

Nothing in P08 remains incomplete. No stop rule fired and no stop record was
changed. Historical-state authentication and the other explicitly excluded
capabilities remain outside this completed unit. No new work is authorized by
completion of the finite plan.
