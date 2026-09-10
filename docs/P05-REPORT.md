# P05 — stopped: missing public-incident context provenance

P05 is **not implemented**. The existing section 6 `missing-provenance` stop rule
applies to its mandatory public USE fixture. The retained product tree is exactly
P04 at `05dc9ee`; no claim API, accounting refactor, replay command, new product
tests, or fixture manifest is left in the release. Work remains uncommitted. No
Git write, push, PR, baseline change, corpus change, or P06 work was attempted.
The machine-readable decision is in [roadmap-stops.json](roadmap-stops.json).
One working day was touched; the stop is an evidence prerequisite failure, not a
claim that the four-day time ceiling elapsed.

The prototype reconstructed USE's input identities and archived funding proof,
validated the transaction under supplied premises, and evaluated the existing
extraction property. Its tests reported three accepted bundles, two positive
claims in two families, one accepted nonviolating control and zero negative-control
claims. **Those green results are withdrawn as P05 completion evidence.** The
harness checked source-recorded inputs but let borrowed hypothetical context count
toward the mandatory public-incident floor. That did not enforce the full contract
in sections 3 and 6. Passing these tests was not sufficient.

Public transaction and block records were successfully retrieved from the Ergo
explorer. They establish a concrete mismatch in the rejected candidate: its
preheader version was 1 rather than 4; timestamp was 1700000000000 rather than
1788813125484; parent ID was a repeated test value. Its parameters, network rules,
header window and prior block cost were also borrowed hypothetical values. The
retrieved block does not supply preceding transaction execution costs or the full
active validation snapshot. This is a limit of the evidence assembled in this
attempt, not a claim that historical context is globally unrecoverable. Recovering
and substantiating the missing context is a prerequisite to reopening this unit;
substituting defaults or relabeling them is not an admissible shortcut.

The [source manifest](p05-stop-evidence/sources.json) pins the raw public responses
and rejected candidate, separately from every accepted fixture/corpus. The
[provenance audit](p05-stop-evidence/check_provenance.py) checks their hashes and
prints the mismatch. It exits 1; it is diagnostic stop evidence, not an alternate
P05 gate. The raw candidate is retained specifically to expose the borrowed values,
not to publish a confirmed incident claim. Public responses are explorer source
records; this record does not authenticate canonical-chain inclusion or historical
UTXO membership.

The attempt source, patch and preliminary test logs are preserved locally under
`target-p00/p05-attempt/`. They are ignored, non-shipping artifacts. No incomplete
product prefix was kept: retaining a replay release with a selectively weakened
public-fixture test would leave two meanings of “P05 passed.” The coherent prefix
is the already completed P04 capability, including USE's existing preflight replay.

ROADMAP changes are limited to a status note linking this stop. Its policy block,
numeric thresholds, acceptance paragraph, and `completedThrough: P04` are unchanged
from HEAD. P05 stays `implemented: false`; the stop record makes its runner result
`stopped`. P06–P08 remain `unimplemented`. The requested P05 exit 0 and advancement
to P05 are intentionally not claimed. Reopen with a complete pinned publishable
USE validation snapshot and a harness that verifies its source values, then rerun
all unchanged P05 and predecessor gates.

## Verification of the retained tree

Every requested command ran again after withdrawing the prototype, with
`CARGO_TARGET_DIR=./target-p00`. The actual output excerpts below correspond to
that retained tree. Full logs and machine records remain locally in
[target-p00/p05-verification](../target-p00/p05-verification/manifest.json).
The workspace and cost-trace counts sum Cargo's actual per-target summaries;
they are not fabricated single-suite output. The existing external-checkout
compiled-fixture measurement remains ignored, exactly as in P04.

The direct P05 target exits 101 because the incomplete target was withdrawn.
`--require P05` exits 1 with `stopped`; `--all-goals` exits 1 with P05 `stopped`
and P06–P08 `unimplemented`. Neither roadmap run has `missing-gate`.
`--through-completed` passes through P04; no advancement to P05 occurred.

`cargo fmt --all -- --check` — exit **0**

```text
(no output)
```

`cargo clippy --workspace --all-targets -- -D warnings` — exit **0**

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Checking ergo-web v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-web)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.67s
```

`cargo test --release -p ergo-sandbox --test claim_replay -- --nocapture` — exit **101**

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
error: no test target named `claim_replay` in `ergo-sandbox` package
help: available test targets:
    audit
    audit_context
    claim_contract
    cli_hunt
    cli_json
    cli_match
    compile_params
    compose
    decompile_ast
    decompile_corpus
    decompile_robustness
    decompile_roundtrip
    delegated_reserves
    drain
    evidence_provenance
    evidence_wire
    hot_spots
    hunt
    identity
    ingest
    map
    method_sweep
    more_lints
    mutation_corpus
    node_validation
    payment_reserves
    play
    proofs
    recognize
    rent
    sandbox_eval
    testsuite
    transaction_proofs
    tree
    triage
    txcheck
    vault_corpus
```

`cargo test --workspace` — exit **0**

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
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.41s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-7d278555b1f8a3a0)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-cc281e409e1cbf34)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-2c76cae4915aac68)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/drain.rs (target-p00/debug/deps/drain-87587731972da5b1)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 54.83s
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
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-0eddf5098399727c)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-8245cdbda7778ca7)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-a2fc17dcfaa7b014)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 813.42s
     Running tests/node_validation.rs (target-p00/debug/deps/node_validation-ac99913601a8669b)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-f024620180484279)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/play.rs (target-p00/debug/deps/play-f49907fa9c995ca5)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-a9e0937080a4cc5e)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-ee34412f2cb9652f)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-7910b2d870be8e85)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-8ead2a2c6eb9440d)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-7b2356c2d782814c)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/transaction_proofs.rs (target-p00/debug/deps/transaction_proofs-41d6df6b9eed8c3b)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/tree.rs (target-p00/debug/deps/tree-3b1e73cb016ab6a0)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-846eadcc086abf9e)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-14a82d01974bc854)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-815d5edc6095a173)
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
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
   Doc-tests ergo_web
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Sum of actual per-target summaries: **441 passed / 0 failed / 1 ignored**.

`python3 scripts/roadmap_gate.py --require P05` — exit **1**

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: passed: required tests executed
P04: passed: required tests executed
P05: stopped: recorded stop blocks this unit
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
P05: stopped: recorded stop blocks this unit
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
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/through-completed.json
```

`cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` — exit **0**

```text
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.65s
```

`cargo test -p ergo-sandbox --features cost-trace` — exit **0**

```text
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-12321df51caca61c)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-06eb44d00497866d)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-93b44b6eabd268af)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-7e4beea168872c1d)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-180ef2ed16cf04c1)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-9b56f675a9fb6219)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
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
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.14s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-6e6763e311dc83f2)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-02ee190d832ff3be)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-5740574630d7a07f)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/drain.rs (target-p00/debug/deps/drain-2ef5444af97273fd)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 50.36s
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
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-324256cc787ba3cb)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-477a3352439a7479)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 811.60s
     Running tests/node_validation.rs (target-p00/debug/deps/node_validation-39b51296776ceae4)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-bf2bff6417bb5fef)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/play.rs (target-p00/debug/deps/play-d51859b6326a6fdd)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
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
     Running tests/transaction_proofs.rs (target-p00/debug/deps/transaction_proofs-3dc4212901df5ab3)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/tree.rs (target-p00/debug/deps/tree-00b6e82906bbfa11)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-84d6e167718dddaf)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-fc24d9e7d4b4ee0a)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-19848dfddffc6646)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests ergo_sandbox
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
```

Sum of actual per-target summaries: **374 passed / 0 failed / 1 ignored**.

`python3 docs/p05-stop-evidence/check_provenance.py` — exit **1**

```text
incident: 5371373d346aade57f684ead23f386e3441ffb2cb7a860babe96e3c5048b2725; block height: 1868204; index: 5
blockContext: present; origin=hypothetical
parameters: present; origin=hypothetical
networkRules: present; origin=hypothetical
headers: present; origin=hypothetical
priorBlockCost: present; origin=hypothetical
preHeaderVersion: supplied=1; retrieved=4; equal=False
preHeaderTimestamp: supplied=1700000000000; retrieved=1788813125484; equal=False
preHeaderParentId: supplied=4444444444444444444444444444444444444444444444444444444444444444; retrieved=71b29999204aee33af5dda49eb8dde4ac7a7ccd035b9199f6dab77552d3ca3e3; equal=False
Retrieved transaction records expose no preceding transaction execution costs: True
This audit does not establish historical UTXO membership or authenticate the explorer.
missing-provenance: blockContext, parameters, networkRules, headers, priorBlockCost
```
