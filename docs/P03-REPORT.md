# P03 — local implementation and verification record

P03 is implemented in the working tree on `p00/claim-contract`, starting from
P02 commit `581acb7`. No Git write was attempted; the user will review and commit.
P04 has not started. The [API boundary](node-validation.md) documents the supplied
premises and the limits of the resulting acceptance claim.

The new `evidence::validate` adapter invokes the pinned node's raw
`ergo_validation::tx::validate_transaction` entry with explicit UTXOs, every
protocol parameter, activation/block context, headers, local policy, network
rules and prior accumulated block cost. The validator dependency uses the same
revision as the compiler/reducer. Its own revision is checked against the case
pin and recorded in reports. No consensus predicates were ported into Forge,
and `txcheck` is not used as an oracle or changed.

Only a successful fresh node call constructs `AcceptedExecution`, which contains
the node's `CheckedTransaction` behind private fields. Reports retain the entire
request and fingerprint and say `node-accepted` against supplied state/rules.
Stored JSON cannot deserialize into the capability. Missing premises, unsupported
provenance assertions, inconsistent context/transaction attachments and malformed
material cannot silently acquire defaults. Two compile-fail doctests exercise
the absent deserializer and inaccessible constructor; the registered import test
also changes UTXOs in an old accepted request and observes a fresh node rejection.

Nine public, authored vectors have exact request hashes and a pinned node revision.
All hashes/revisions are verified before any vector executes. Expected results
were captured by the retained producer calling the full node path directly,
without the Forge validation adapter or preflight. The eight required cases cover
keyless acceptance, duplicate inputs, future outputs, canonical encoding, monetary
constraints, aggregate cost, missing UTXOs and storage rent. The ninth exercises
enabled re-emission rejection. Controls remove the rent extension, reset prior
cost and explicitly disable re-emission, checking that those premises affect the
node result. These are hypothetical supplied snapshots, not historical evidence.

The request fixtures use `.fixture` to keep new validation vectors out of the
frozen recursive JSON decompiler corpus. No existing corpus row, denominator,
cap, objective, verdict, numeric baseline or engine revision changed. Legacy
endpoints remain preflight/simulation; no signing, property-claim producer,
search change or P04 work was added. The historical scoreboard is unchanged.

ROADMAP now enumerates P03's required IDs in its machine-readable policy,
including the additional re-emission vector. This makes the eight existing prose
requirements executable and adds a rule-forwarding control. The minimum remains
eight; no acceptance requirement or threshold was weakened. P03 is registered
as implemented, CI's current gate moves to P03, and `completedThrough` advances
only after fresh P00–P03 gate passes. Older completion statements are marked
historical; P04–P08 remain unimplemented. No stop record is warranted within the
six-day ceiling.

An initial `--through-completed` run returned `missing-gate`: under `--nocapture`,
a new diagnostic print interleaved with a passing Rust test-status line. Removing
P03's diagnostic prints exposed the same pre-existing problem in P02. Serial
execution was tried and also failed: Rust then prints the test-name prefix before
the diagnostic. These failed transcripts are retained, not counted as passes.

The explicit ROADMAP gate-command amendment replaces runner `--nocapture` with
`--show-output`. Captured diagnostics appear after intact status lines. No parser
check, required name, whole-target execution, skipped/zero-test check, or numeric
threshold is weakened. A new runner regression test checks the actual command
selection and retained missing-name rejection. No existing product test changed.
The user-requested direct P03 command still runs with `--nocapture`. This narrow
harness correction is necessary to rerun predecessors reliably; it is not P04
work or a revision of their product behavior. Final successful outputs supersede
the failed attempts below.

## Verification

All required final commands completed. Every command used
`CARGO_TARGET_DIR=./target-p00`; no test-thread override was used in final runs.
The [raw transcripts and machine records](../target-p00/p03-verification/manifest.json)
remain locally under `target-p00/p03-verification/` (ignored build artifacts).
The manifest binds commands, exit codes, log hashes and changed source hashes.
The excerpts below are actual output, not replacement pass assertions.

The single ignored test is the existing external-checkout compiled-fixture measurement; no new test
was ignored. Both new compile-fail doctests ran successfully. The workspace and
cost-trace tests began before removal of P03's diagnostic prints; their assertions
and product code are identical to the final tree. The final release target and
roadmap gates separately exercised the final test source and policy. No product
failure was hidden by the harness correction.

### `cargo fmt --all -- --check` — exit 0

```text
(no output)
```

### `cargo clippy --workspace --all-targets -- -D warnings` — exit 0

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.51s
```

### `cargo test --release -p ergo-sandbox --test node_validation -- --nocapture` — exit 0

```text
    Blocking waiting for file lock on package cache
    Finished `release` profile [optimized] target(s) in 0.24s
     Running tests/node_validation.rs (target-p00/release/deps/node_validation-4f64839c805d180c)

running 3 tests
test accepted_execution_cannot_be_deserialized_or_fabricated ... ok
test incomplete_context_is_not_node_acceptance ... ok
test full_pipeline_matches_pinned_node_vectors ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### `cargo test --workspace` — exit 0

```text
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-e07c3983ea287b14)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-eaba3827398e1719)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-e4459af24fbe59be)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-7e395e280db8c0ab)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-ab1a86b974931b3d)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-2acd6a5b1b4e97fc)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
     Running tests/cli_json.rs (target-p00/debug/deps/cli_json-042667f88da105be)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli_match.rs (target-p00/debug/deps/cli_match-37378f571f127e99)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compile_params.rs (target-p00/debug/deps/compile_params-120630305db0523f)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/compose.rs (target-p00/debug/deps/compose-47797d46f61f692a)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_ast.rs (target-p00/debug/deps/decompile_ast-bd41255a718ec8d3)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_corpus.rs (target-p00/debug/deps/decompile_corpus-e8b224ac4a2dc674)
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.19s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-7d278555b1f8a3a0)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-cc281e409e1cbf34)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-2c76cae4915aac68)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/drain.rs (target-p00/debug/deps/drain-87587731972da5b1)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 58.45s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-02061ea86cd219be)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
     Running tests/evidence_wire.rs (target-p00/debug/deps/evidence_wire-f03fd92c3aa92a8f)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hot_spots.rs (target-p00/debug/deps/hot_spots-e025a9e9fcf717b9)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hunt.rs (target-p00/debug/deps/hunt-8c8d8cdae4f96ccc)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/identity.rs (target-p00/debug/deps/identity-f18f6c0d3be85482)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/ingest.rs (target-p00/debug/deps/ingest-d6286c26e857d91d)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/map.rs (target-p00/debug/deps/map-64598df9aef4bcf1)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-0eddf5098399727c)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-8245cdbda7778ca7)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-a2fc17dcfaa7b014)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 820.93s
     Running tests/node_validation.rs (target-p00/debug/deps/node_validation-ac99913601a8669b)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-f024620180484279)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/play.rs (target-p00/debug/deps/play-f49907fa9c995ca5)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-a9e0937080a4cc5e)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-ee34412f2cb9652f)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-7910b2d870be8e85)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-8ead2a2c6eb9440d)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-7b2356c2d782814c)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tree.rs (target-p00/debug/deps/tree-3b1e73cb016ab6a0)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-846eadcc086abf9e)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.74s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-14a82d01974bc854)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-815d5edc6095a173)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_web-58a61400bbf06183)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
     Running unittests src/main.rs (target-p00/debug/deps/ergo_web-5f03d3a11e4efa70)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/http.rs (target-p00/debug/deps/http-de5d928124428075)
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
     Running tests/map.rs (target-p00/debug/deps/map-3720d2a9f2f3f122)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
   Doc-tests ergo_sandbox
test ergo-sandbox/src/evidence/validate.rs - evidence::validate::AcceptedExecution (line 172) - compile fail ... ok
test ergo-sandbox/src/evidence/validate.rs - evidence::validate::AcceptedExecution (line 168) - compile fail ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
   Doc-tests ergo_web
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Sum of the per-target summaries: **435 passed / 0 failed / 1 ignored**.

### `python3 scripts/roadmap_gate.py --require P03` — exit 0

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/P03.json
```

### `python3 scripts/roadmap_gate.py --all-goals` — exit 1

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: unimplemented: registered product unit is not implemented
P05: unimplemented: registered product unit is not implemented
P06: unimplemented: registered product unit is not implemented
P07: unimplemented: registered product unit is not implemented
P08: unimplemented: registered product unit is not implemented
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/all-goals.json
```

### `python3 scripts/roadmap_gate.py --through-completed` — exit 0

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/through-completed.json
```

### `cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` — exit 0

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.64s
```

### `cargo test -p ergo-sandbox --features cost-trace` — exit 0

```text
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-12321df51caca61c)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-06eb44d00497866d)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-93b44b6eabd268af)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-7e4beea168872c1d)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-180ef2ed16cf04c1)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-9b56f675a9fb6219)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
     Running tests/cli_json.rs (target-p00/debug/deps/cli_json-7b04261d2f996176)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli_match.rs (target-p00/debug/deps/cli_match-324ffab81a74109f)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compile_params.rs (target-p00/debug/deps/compile_params-f96e526beeaa3a7a)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/compose.rs (target-p00/debug/deps/compose-e2673e654d330564)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_ast.rs (target-p00/debug/deps/decompile_ast-14af58a36917c4f3)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_corpus.rs (target-p00/debug/deps/decompile_corpus-3ff751f7c03a848d)
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.07s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-6e6763e311dc83f2)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-02ee190d832ff3be)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-5740574630d7a07f)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/drain.rs (target-p00/debug/deps/drain-2ef5444af97273fd)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 51.26s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-5987a4a0711a45a7)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
     Running tests/evidence_wire.rs (target-p00/debug/deps/evidence_wire-bf64d059a58d7ed0)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hot_spots.rs (target-p00/debug/deps/hot_spots-49189fd995a80fa5)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hunt.rs (target-p00/debug/deps/hunt-986452479882b1a8)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/identity.rs (target-p00/debug/deps/identity-24bf347aee8ba68f)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/ingest.rs (target-p00/debug/deps/ingest-c7260c313ea7bcdb)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/map.rs (target-p00/debug/deps/map-f274b097568e4bcb)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-1d499b93048a9796)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-324256cc787ba3cb)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-477a3352439a7479)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 821.77s
     Running tests/node_validation.rs (target-p00/debug/deps/node_validation-39b51296776ceae4)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-bf2bff6417bb5fef)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/play.rs (target-p00/debug/deps/play-d51859b6326a6fdd)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-427e0cfb39de769b)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-3f186a8ee8d3f969)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-242b6e1bed5f8526)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-8460eb14795041c7)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-057909eb3359a3b0)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tree.rs (target-p00/debug/deps/tree-00b6e82906bbfa11)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-84d6e167718dddaf)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.66s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-fc24d9e7d4b4ee0a)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-19848dfddffc6646)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests ergo_sandbox
test ergo-sandbox/src/evidence/validate.rs - evidence::validate::AcceptedExecution (line 172) - compile fail ... ok
test ergo-sandbox/src/evidence/validate.rs - evidence::validate::AcceptedExecution (line 168) - compile fail ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
```

Sum of the per-target summaries: **368 passed / 0 failed / 1 ignored**.

### `python3 -m unittest discover -s scripts -p 'test_roadmap_gate.py'` — exit 0

```text
..............
----------------------------------------------------------------------
Ran 14 tests in 0.037s

OK
```

P03 is complete in the working tree; nothing was committed or pushed. No checklist
item remains incomplete. P04–P08 were not implemented. The expected all-goals
failure is exclusively those five unimplemented units, with no missing-gate.
