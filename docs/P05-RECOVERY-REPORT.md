# P05 recovery and implementation

**Decision: B, recover the real source snapshot and keep P05's original gate.
The strongest argument against it was spending the unit on incident-specific
recovery that might fail while buildable replay machinery stayed shelved.** The
recovery succeeded, so no switch to A or reduction in acceptance was needed.
The rationale and authorization are in [P05-DECISION.md](P05-DECISION.md).

The uncommitted change adds `evidence/claim.rs`, `evidence/replay.rs` and
`ergo-es replay <case.json> --json`. A claim requires fresh native acceptance and
violation of the existing versioned extraction property. Its fingerprint binds
the entire execution request and declared property; imported report flags carry
no authority. The CLI is tested from an unrelated working directory, with dead
proxy settings and a local source-URL trap that receives no connection. It does
not fetch, sign, search or broadcast. No P06–P08 implementation was started.

The drain accounting moved into `drain/accounting.rs`; its function bodies match
HEAD exactly after accounting for visibility qualifiers. The new path projects
accepted inputs/output candidates into those unchanged functions, never through
a scenario reducer or synthetic-box serializer. No detector, family, objective,
cap, corpus row, verdict, or numerical baseline changed. The workspace lockfile
only adds two already pinned native crates as test dependencies: chain
specification and crypto commitment checks use the same node revision.

The [recovery directory](p05-recovery/README.md) preserves every raw public request
and maps the derivations to native APIs. Header IDs and links bind the incident to
its epoch; native Merkle checks bind the epoch extension and transaction/proof
commitment. Complete archived inputs and explicit proof extensions reproduce the
transaction IDs. The five preceding transactions validate with cumulative costs
14059, 100833, 123759, 136478 and 194317. USE validates with prior cost 194317 and
cumulative cost 216345. Epoch input/output costs are 2407/298; these replace
borrowed values only in the new USE fixture, not in any old artifact or baseline.

The new acceptance set has three distinct accepted transaction bundles, two
positive claims in two named families, one accepted nonviolating control and zero
negative-control claims. The public family is source-backed USE. The authored
fixed-rate sale has a positive unpaid mutant, an accepted paid fixed control and
a rejected unpaid fixed counterfactual with newly serialized hypothetical box
identities. Its existing in-flight files were retained; no corpus mutant was
rewritten to obtain these counts. The public/hypothetical cases are reported
separately. Extra preceding transactions are derivation evidence, not padding for
the three-bundle or two-family floors.

The P05 target repeats source recovery rather than importing generated request
files. Its public-source comparison rejects the previously withdrawn candidate;
substituting the old context, parameters, rules, headers or prior cost and merely
changing the origin label to `source-recorded` also fails. This closes the precise
harness gap that invalidated the first attempt's green results. The raw source
material remains source-recorded: this is not a genesis-to-tip chain validation,
authenticated historical UTXO membership, or inferred protocol policy.

ROADMAP amendments are explicit and non-numerical: register the separate P05
fixture manifest and case identities; permit reopening the exact historical stop
through a policy-pinned governing decision; mark P05 implemented; advance
`completedThrough` only after predecessor gates reran green. `roadmap-stops.json`
retains every original field and evidence hash, appending only a resolution. The
runner validates exact record/decision hashes; missing records, altered decisions
and subsequent stops fail closed. No threshold changed, including
`publicIncidentClaimMin: 1` and `claimFamiliesMin: 2`. The governing stop rule now
names evidence recovery as a reopening route. P06–P08 remain unimplemented, and
CI's required gate advances to P05.

The first recovery adapter builds exposed field/API mismatches (`SpendingProof`
construction and the node's hex-encoded votes, among them). Those were corrected
in the adapter; no source value or expected verdict was changed to hide an engine
rejection. Once the adapter consumed the recorded schemas, all six transactions
passed. The original stop report remains intact as history. No Git write, commit,
push or PR was attempted; all changes await user verification and commit.

## Verification

All required commands completed with `CARGO_TARGET_DIR=./target-p00`.
Full transcripts and machine records are preserved locally under
[target-p00/p05-recovery-verification](../target-p00/p05-recovery-verification/manifest.json).
The output below is copied from those actual runs. Aggregate test counts are
explicit sums of Cargo's per-target summaries, not fabricated summary lines.
The sole ignored test remains the existing external-checkout compiled-fixture
measurement. No P05 test was ignored; both private-claim compile-fail doctests ran.

`--require P05` and `--through-completed` are green; `--all-goals` exits 1 for
P06–P08 unimplemented, with no `missing-gate`. P00's runner self-tests (15) and
rendered UI claim-label tests (7) also execute through the predecessor gates.
Every original numerical floor is unchanged. The original stop audit still exits
1 for its unchanged rejected artifact; resolution rests on new evidence, not on
rewriting that failure. There is no outstanding P05 acceptance requirement.

`cargo fmt --all -- --check` — exit **0**

```text
(no output)
```

`cargo clippy --workspace --all-targets -- -D warnings` — exit **0**

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.48s
```

`cargo test --release -p ergo-sandbox --test claim_replay -- --nocapture` — exit **0**

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `release` profile [optimized] target(s) in 0.49s
     Running tests/claim_replay.rs (target-p00/release/deps/claim_replay-d79525742826db59)

running 4 tests
epoch parameters: {"dataInputCost":100,"inputCost":2407,"maxBlockCost":8001091,"maxBlockSize":1271009,"maxBoxSize":4096,"maxTokensPerBox":122,"minValuePerByte":360,"outputCost":298,"storageFeeFactor":1250000,"storagePeriod":1051200,"tokenAccessCost":100}
index 0: accepted 10ff5c82991eb44853b7aaf6da95372d9c006e8a18c18809ca9a9a5405356b68, total cost 14059
index 1: accepted ce0f75d6d59d5e350bd5841a038ff9f22136127af9f69ecf05d777533dcf7ec1, total cost 100833
index 2: accepted 2b5a56dc2714ebcd1eceaef086edfcc12d4d63eb30ca8a42257ad7df04b57b60, total cost 123759
index 3: accepted 6a4d184c397f1b4159d1ca0065d987e41efc23a563c5f0800003dcc50eedb641, total cost 136478
index 4: accepted 30c1cacc7f4e30f1b52f1f1da35ae93750bf9187461eb374ecf9d9f9ade3d719, total cost 194317
index 5: accepted 5371373d346aade57f684ead23f386e3441ffb2cb7a860babe96e3c5048b2725, total cost 216345
accepted bundles: 3; positive claims: 2; families: 2; accepted nonviolating controls: 1; negative-control claims: 0; recorded-input cases: 1; hypothetical-input cases: 3
test unknown_policy_cannot_confirm ... ok
test claim_requires_accepted_execution_and_violated_property ... ok
test wrong_companion_or_policy_invalidates_claim ... ok
test offline_replay_reproduces_claim ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
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
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-867a538af62c8c59)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
     Running tests/claim_replay.rs (target-p00/debug/deps/claim_replay-1eeca24b641ba3d1)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-96f38a1b20e781ff)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
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
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-0049cecfe2605568)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/drain.rs (target-p00/debug/deps/drain-a587fae7e43b0e13)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 53.16s
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
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-daff6d8031bc25ad)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-d249f4aea675b5ca)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 850.32s
     Running tests/node_validation.rs (target-p00/debug/deps/node_validation-2db5a0b67406d036)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-3626131eca09286f)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/play.rs (target-p00/debug/deps/play-432bab585fd5d216)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-a556daeceeb5f2b7)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-f4287d780e542da1)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-65e9a0bc6b1d0069)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-75ae46c51f516316)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-22e8472d542ae703)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/transaction_proofs.rs (target-p00/debug/deps/transaction_proofs-b597196e4304bbf9)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/tree.rs (target-p00/debug/deps/tree-29fee86d46fc2c00)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-fcd4913da558a07b)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
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
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
   Doc-tests ergo_sandbox
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
   Doc-tests ergo_web
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Sum of actual per-target summaries: **447 passed / 0 failed / 1 ignored**.

`python3 scripts/roadmap_gate.py --require P05` — exit **0**

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: passed: required tests executed
P05: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/P05.json
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
P06: unimplemented: registered product unit is not implemented
P07: unimplemented: registered product unit is not implemented
P08: unimplemented: registered product unit is not implemented
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/all-goals.json
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
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/through-completed.json
```

`cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` — exit **0**

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.01s
```

`cargo test -p ergo-sandbox --features cost-trace` — exit **0**

```text
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-b8deb5d2dc4f01d8)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-300623d0c0afdfe6)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-d133cca64c44808b)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-058b63216fb7f65f)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-bb4bbaa8e2e794ef)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
     Running tests/claim_replay.rs (target-p00/debug/deps/claim_replay-0118b7fa1985ae35)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-fa151cfe99805052)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
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
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.17s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-ceb7f1b226f0970f)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-7ae82f97ca6e16f8)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-3f8f56eb04c9f479)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/drain.rs (target-p00/debug/deps/drain-e7a59f90bc3cbecc)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 62.87s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-85e71ecb74d87f95)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
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
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-4490cc9ef6d4aecb)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-6bf9fcb826123f12)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-6d9c5b862c2bdb0c)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 850.68s
     Running tests/node_validation.rs (target-p00/debug/deps/node_validation-5505a2f7ecdb1408)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-052151606c965359)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/play.rs (target-p00/debug/deps/play-757e3f1c59a6eece)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-491b80850096b172)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-e76e152b6bdfc861)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/rent.rs (target-p00/debug/deps/rent-d9bd071b6fa929e1)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-945e8d355d7e0a4e)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-fa6444531f6214f9)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/transaction_proofs.rs (target-p00/debug/deps/transaction_proofs-7781a003e7eb6794)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/tree.rs (target-p00/debug/deps/tree-b06c25c34df1ba95)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-91c30f193f7c666c)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.67s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-c1780c8de143a28c)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-1f02e6af081086ec)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests ergo_sandbox
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
```

Sum of actual per-target summaries: **380 passed / 0 failed / 1 ignored**.

`cargo clippy --locked --manifest-path docs/p05-recovery/derive/Cargo.toml --all-targets -- -D warnings` — exit **0**

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
   Compiling zerocopy v0.8.57
   Compiling generic-array v0.14.7
   Compiling crossbeam-utils v0.8.23
   Compiling find-msvc-tools v0.1.12
   Compiling crossbeam-epoch v0.9.21
   Compiling crossbeam-deque v0.8.8
   Compiling syn v3.0.5
    Checking indexmap v2.14.2
    Checking tinyvec v1.13.2
   Compiling rustls v0.23.44
   Compiling cc v1.4.5
    Checking unicode-normalization v0.1.25
    Checking crypto-common v0.1.7
    Checking block-buffer v0.10.4
    Checking sec1 v0.7.3
    Checking crypto-bigint v0.5.5
    Checking block-buffer v0.9.0
    Checking digest v0.9.0
    Checking inout v0.1.4
    Checking universal-hash v0.5.1
    Checking aead v0.5.2
    Checking cipher v0.4.4
    Checking sha2 v0.9.9
    Checking crossbeam-channel v0.5.17
    Checking digest v0.10.7
    Checking polyval v0.6.2
    Checking hmac v0.12.1
    Checking blake2 v0.10.6
    Checking signature v2.2.0
    Checking sha2 v0.10.9
    Checking ctr v0.9.2
    Checking aes v0.8.4
    Checking ghash v0.5.1
    Checking rfc6979 v0.4.0
    Checking pbkdf2 v0.12.2
    Checking rayon-core v1.13.0
   Compiling ring v0.17.14
    Checking aes-gcm v0.10.3
    Checking rayon v1.12.0
    Checking elliptic-curve v0.13.8
    Checking ecdsa v0.16.9
    Checking k256 v0.13.4
   Compiling thiserror-impl v2.0.20
   Compiling serde_derive v1.0.229
   Compiling clap_derive v4.6.4
    Checking thiserror v2.0.20
    Checking ergo-primitives v0.6.0 (https://github.com/arkadianet/ergo?rev=9468043396e5daa2828211bcff4234bc70fae4f0#94680433)
    Checking ergo-ser v0.6.0 (https://github.com/arkadianet/ergo?rev=9468043396e5daa2828211bcff4234bc70fae4f0#94680433)
    Checking clap v4.6.6
    Checking ppv-lite86 v0.2.21
    Checking rand_chacha v0.3.1
    Checking rand v0.8.8
    Checking ergo-chain-spec v0.6.0 (https://github.com/arkadianet/ergo?rev=9468043396e5daa2828211bcff4234bc70fae4f0#94680433)
    Checking ergo-crypto v0.6.0 (https://github.com/arkadianet/ergo?rev=9468043396e5daa2828211bcff4234bc70fae4f0#94680433)
    Checking serde v1.0.229
    Checking ergo-compiler v0.6.0 (https://github.com/arkadianet/ergo?rev=9468043396e5daa2828211bcff4234bc70fae4f0#94680433)
    Checking gf2_192 v0.28.0
    Checking ergo_avltree_rust v0.1.1
    Checking ergo-sigma v0.6.0 (https://github.com/arkadianet/ergo?rev=9468043396e5daa2828211bcff4234bc70fae4f0#94680433)
    Checking bincode v1.3.3
    Checking bip39 v2.2.2
    Checking rustls-webpki v0.103.15
    Checking ergo-validation v0.6.0 (https://github.com/arkadianet/ergo?rev=9468043396e5daa2828211bcff4234bc70fae4f0#94680433)
    Checking ergo-state v0.6.0 (https://github.com/arkadianet/ergo?rev=9468043396e5daa2828211bcff4234bc70fae4f0#94680433)
    Checking ureq v3.4.1
    Checking ergo-wallet v0.6.0 (https://github.com/arkadianet/ergo?rev=9468043396e5daa2828211bcff4234bc70fae4f0#94680433)
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Checking p05-recovery v0.1.0 (/home/rkadias/coding/ergo-forge/docs/p05-recovery/derive)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.08s
```
