# P04 — local implementation and verification record

P04 is implemented in the working tree on `p00/claim-contract`, starting from
P03 commit `8858731`. No Git write was attempted. Nothing was pushed or opened
as a PR; the user will review and commit. P05 has not started.

The [transaction proof API](transaction-proofs.md) adds one explicitly selected
standard P2PK funding proof. The node's canonical signing bytes feed the existing
wallet prover; P03's full raw transaction validator alone produces acceptance.
An owned key is a separate non-serializable argument with redacted Debug and
zeroized owned storage. This does not promise erasure of caller copies. No key
is placed in a replay request, signing annotation, accepted report or error.
Arbitrary caller metadata is preserved, not sanitized.

Signing checks the selected UTXO's exact standard P2PK script against the owned
public key. It does not use caller roles as authority. It preserves all other
proofs/extensions, refuses to overwrite existing funding proofs, checks canonical
wire identity, and retains the original P02 attachment plus parent fingerprint
when attaching signed bytes. Imported signed requests replay without any key;
no accepted-report flag bypasses fresh validation. Source/box origins and unknown
deployment identity remain unchanged. Acceptance proves no property or history.

The four required tests pass at the unit target. Output, extension and input-order
mutations reject the old proof with the exact node `ProofFailed` diagnostic;
fresh proofs accept those otherwise valid changed transactions. Missing context
and future-output controls fail signing's full-validation requirement. A declared
public key without proof fails; wrong keys, invalid scalars, nonstandard selected
inputs and out-of-range indices fail. Replay uses a pinned signed artifact before
constructing any key. Two compile-fail doctests additionally prohibit key serde.

The one public authored experiment has unsigned/signed requests, exact file
hashes, node revision, source/family, publication eligibility and explicit
no-property-claim metadata. Tests authenticate the entire manifest before use.
The producer was run once successfully; its node-accepted result and transaction
ID are retained. The public test scalar is deliberately hypothetical and must
never receive real funds. Gates never regenerate proofs or rewrite expected data.

ROADMAP amendment: P04's policy entry names `proof-manifest.json` and its required
experiment ID. A separate manifest is necessary because P03's completed harness
binds its exact node-vector inventory; appending P04 rows would change that frozen
inventory and require rewriting the predecessor gate. P04 enforces the same
revision/hash/provenance requirements without changing a P03 row. No numeric
threshold or named acceptance was weakened. P04 is registered as implemented;
`completedThrough` advances to P04 after fresh P00–P04 passes, and CI's current
unit moves to P04. P05–P08 remain unimplemented.

No existing corpus row, cap, objective, detector, search axis, numeric baseline,
dependency or engine revision changed. Legacy prover behavior and endpoint
semantics remain intact. No browser key storage, new sigma protocol, keyed
victim/companion search or next-unit implementation was added. No stop record is
warranted within P04's three-day ceiling.

The first unit run failed a test assertion that expected a diagnostic containing
`Script`; the actual node result was `ProofFailed { index: 1 }`. The corrected
assertion now requires that exact node error/index and retains the fresh-signature
positive controls. The failed transcript is retained as `own-initial.log`; it is
not counted as a passing gate. No engine result or fixture was changed to repair it.

## Verification

All required final commands completed with `CARGO_TARGET_DIR=./target-p00`.
[Raw transcripts and machine records](../target-p00/p04-verification/manifest.json)
remain locally under `target-p00/p04-verification/` (ignored build artifacts).
The manifest binds commands, exit codes, log hashes and changed source hashes.
The excerpts below are actual command output. Suite totals explicitly sum Cargo's
per-target summaries rather than presenting a fabricated single summary line.

Both new key-serde compile-fail doctests passed. The only ignored test is the
existing external-checkout compiled-fixture measurement; no P04 test was ignored.
The all-goals failure is exactly P05–P08 unimplemented, with no missing-gate.
No P04 acceptance remains incomplete, and no stop rule was triggered.

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
    Blocking waiting for file lock on build directory
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 44.96s
```

### `cargo test --release -p ergo-sandbox --test transaction_proofs -- --nocapture` — exit 0

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
   Compiling ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `release` profile [optimized] target(s) in 2.42s
     Running tests/transaction_proofs.rs (target-p00/release/deps/transaction_proofs-42ec8b2769fb1184)

running 4 tests
test declared_public_key_without_proof_is_not_acceptance ... ok
test replay_bundle_contains_no_secret ... ok
test owned_p2pk_funding_signs_actual_transaction ... ok
test changed_transaction_invalidates_proof ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
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
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-ab1a86b974931b3d)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-2acd6a5b1b4e97fc)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
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
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.15s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-7d278555b1f8a3a0)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-cc281e409e1cbf34)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-2c76cae4915aac68)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
     Running tests/drain.rs (target-p00/debug/deps/drain-87587731972da5b1)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 57.79s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-02061ea86cd219be)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
     Running tests/evidence_wire.rs (target-p00/debug/deps/evidence_wire-f03fd92c3aa92a8f)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hot_spots.rs (target-p00/debug/deps/hot_spots-e025a9e9fcf717b9)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hunt.rs (target-p00/debug/deps/hunt-8c8d8cdae4f96ccc)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/identity.rs (target-p00/debug/deps/identity-f18f6c0d3be85482)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/ingest.rs (target-p00/debug/deps/ingest-d6286c26e857d91d)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/map.rs (target-p00/debug/deps/map-64598df9aef4bcf1)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-0eddf5098399727c)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-8245cdbda7778ca7)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-a2fc17dcfaa7b014)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 812.62s
     Running tests/node_validation.rs (target-p00/debug/deps/node_validation-ac99913601a8669b)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-f024620180484279)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/play.rs (target-p00/debug/deps/play-f49907fa9c995ca5)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-a9e0937080a4cc5e)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-ee34412f2cb9652f)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-7910b2d870be8e85)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-8ead2a2c6eb9440d)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-7b2356c2d782814c)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/transaction_proofs.rs (target-p00/debug/deps/transaction_proofs-41d6df6b9eed8c3b)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/tree.rs (target-p00/debug/deps/tree-3b1e73cb016ab6a0)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-846eadcc086abf9e)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-14a82d01974bc854)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-815d5edc6095a173)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_web-58a61400bbf06183)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
     Running unittests src/main.rs (target-p00/debug/deps/ergo_web-5f03d3a11e4efa70)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/http.rs (target-p00/debug/deps/http-de5d928124428075)
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
     Running tests/map.rs (target-p00/debug/deps/map-3720d2a9f2f3f122)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
   Doc-tests ergo_sandbox
test ergo-sandbox/src/evidence/validate.rs - evidence::validate::AcceptedExecution (line 172) - compile fail ... ok
test ergo-sandbox/src/evidence/validate.rs - evidence::validate::AcceptedExecution (line 168) - compile fail ... ok
test ergo-sandbox/src/prove.rs - prove::OwnedDlogSecret (line 275) - compile fail ... ok
test ergo-sandbox/src/prove.rs - prove::OwnedDlogSecret (line 280) - compile fail ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
   Doc-tests ergo_web
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Sum of per-target summaries: **441 passed / 0 failed / 1 ignored**.

### `python3 scripts/roadmap_gate.py --require P04` — exit 0

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/P04.json
```

### `python3 scripts/roadmap_gate.py --all-goals` — exit 1

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: passed: required tests executed
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
P04: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/through-completed.json
```

### `cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` — exit 0

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 44.66s
```

### `cargo test -p ergo-sandbox --features cost-trace` — exit 0

```text
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-12321df51caca61c)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-06eb44d00497866d)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-93b44b6eabd268af)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-7e4beea168872c1d)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-180ef2ed16cf04c1)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-9b56f675a9fb6219)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
     Running tests/cli_json.rs (target-p00/debug/deps/cli_json-7b04261d2f996176)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli_match.rs (target-p00/debug/deps/cli_match-324ffab81a74109f)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compile_params.rs (target-p00/debug/deps/compile_params-f96e526beeaa3a7a)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/compose.rs (target-p00/debug/deps/compose-e2673e654d330564)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/decompile_ast.rs (target-p00/debug/deps/decompile_ast-14af58a36917c4f3)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_corpus.rs (target-p00/debug/deps/decompile_corpus-3ff751f7c03a848d)
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.49s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-6e6763e311dc83f2)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-02ee190d832ff3be)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.67s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-5740574630d7a07f)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/drain.rs (target-p00/debug/deps/drain-2ef5444af97273fd)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 51.20s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-5987a4a0711a45a7)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
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
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-324256cc787ba3cb)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-477a3352439a7479)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 812.11s
     Running tests/node_validation.rs (target-p00/debug/deps/node_validation-39b51296776ceae4)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-bf2bff6417bb5fef)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/play.rs (target-p00/debug/deps/play-d51859b6326a6fdd)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-427e0cfb39de769b)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-3f186a8ee8d3f969)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-242b6e1bed5f8526)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-8460eb14795041c7)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-057909eb3359a3b0)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/transaction_proofs.rs (target-p00/debug/deps/transaction_proofs-3dc4212901df5ab3)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/tree.rs (target-p00/debug/deps/tree-00b6e82906bbfa11)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-84d6e167718dddaf)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-fc24d9e7d4b4ee0a)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-19848dfddffc6646)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests ergo_sandbox
test ergo-sandbox/src/evidence/validate.rs - evidence::validate::AcceptedExecution (line 172) - compile fail ... ok
test ergo-sandbox/src/prove.rs - prove::OwnedDlogSecret (line 275) - compile fail ... ok
test ergo-sandbox/src/prove.rs - prove::OwnedDlogSecret (line 280) - compile fail ... ok
test ergo-sandbox/src/evidence/validate.rs - evidence::validate::AcceptedExecution (line 168) - compile fail ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
```

Sum of per-target summaries: **374 passed / 0 failed / 1 ignored**.

All work remains uncommitted for user review. No Git write was attempted.
