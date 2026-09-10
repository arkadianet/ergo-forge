# P02 — local implementation and verification record

P02 adds a separate node-wire construction path in
`ergo-sandbox/src/evidence/wire.rs`. Work is uncommitted on `p00/claim-contract`,
starting from P01 commit `9f0d4fa`. No Git write was attempted. P03 has not started.
See [evidence-wire.md](evidence-wire.md) for the API and its limits.

| Acceptance | Implemented behavior |
|---|---|
| `wire_box_id_matches_full_serialization` | Two pinned full boxes carry different creation transaction references/output indices. Node serialization and full-byte hashes match their pinned IDs. Independent reference/index changes change ID. Imported claimed-ID mismatches are errors. |
| `transaction_id_and_message_use_node_bytes_to_sign` | Full bytes, signing bytes and transaction ID match pinned node vectors. Signing bytes differ from unsigned serialization. Proof changes preserve signing bytes/ID; extension changes alter them. Output boxes use the actual transaction ID/index. Parsed/raw cache disagreement is rejected. |
| `legacy_box_missing_reference_cannot_be_promoted` | `ScenarioBox::canonical_box()` refuses promotion even with a supplied box ID; missing creation-reference fields are rejected. Hypothetical construction requires explicit material/reference and produces a new hypothetical record. |
| `invalid_tree_never_becomes_empty_bytes` | Missing, invalid, truncated and trailing tree/register material fail without fallback in candidate, box and transaction construction. Complete recorded boxes are required; trailing/truncated full serialization is rejected. |

The fixture count gates read `wireBoxFixturesMin` and
`wireTransactionFixturesMin` directly from ROADMAP's policy block. Expected bytes
and IDs were produced by the pinned node APIs directly, separately from the new
Forge wrapper. The retained generator was executed and reproduced the fixture's
values exactly. These authored vectors contain dummy proof bytes, a nonempty
context extension, a token and R4. They are codec evidence, not transaction
acceptance or historical-state evidence.

P01 cases can retain canonical transaction bytes and resolved spending/data-input
box records through `bind_case`. It checks reference association, preserves other
premises and the previous missing-state premise, and refuses to overwrite
occupied material. Complete proof bytes participate in case identity even when
the transaction ID is unchanged. Export/import retains provenance; typed wire
imports recompute consistency and IDs. `nodeValidated` stays false and deployment
identity unknown. Predicted output boxes are explicitly hypothetical.

No detector, search axis, drain family, cap, objective, existing corpus row,
numeric baseline, engine pin or dependency changed. Legacy marshalling, Play,
txcheck and browser storage retain their simulation/preflight semantics.
`box_build.rs` only gains boundary documentation; `scenario.rs` gains a refusal
API and documentation. No transaction validator was invoked and no consensus rules were ported.

No acceptance or policy threshold was amended. ROADMAP registers P02 as
implemented, advances `completedThrough` after P00/P01/P02 gates reran green,
links the implementation record, and makes the older P01 status paragraph
explicitly historical. CI's current implementation gate changes to P02.
P03–P08 remain unimplemented. No stop record is warranted within the four-day
ceiling.

The initial workspace and cost-trace runs failed the existing decompiler corpus
membership gate: recursive `.json` discovery enrolled exactly one new row,
`ergo-sandbox/tests/fixtures/evidence/wire-v1.json#/candidate/ergoTree`, with no
original rows missing. The new codec vector now uses the `.fixture` suffix while
retaining the same JSON values; P02's explicit loader and fixture-count gate
still execute. The decompiler harness, expectation file and original denominator
were not changed. The failed transcripts are retained alongside final outputs.
This is a fixture-layout correction, not a weaker acceptance gate or a changed
corpus baseline.

All required final runs completed. Summing Cargo's per-target summaries gives
**430 passed / 0 failed / 1 ignored** for the workspace and
**363 passed / 0 failed / 1 ignored** for cost-trace. The workspace retains P00's
422 passing tests, adds P01's four and P02's four, and keeps the same existing
ignored `external_compiled_fixtures_measurement`. No registered target test was
ignored or filtered out. No P02 acceptance item remains incomplete. The full-goal
exit 1 is expected: only P03–P08 are unimplemented, with no `missing-gate`.

## Actual command output

Every command used `CARGO_TARGET_DIR=./target-p00`. Below are verbatim outputs,
limited to per-target summaries for full suites and final status summaries for
the gate runner. Aggregate totals above are sums, not fabricated Cargo output.
Complete stdout/stderr, initial failure logs and machine reports are retained in
[target-p00/p02-verification](../target-p00/p02-verification/); the
[manifest](../target-p00/p02-verification/manifest.json) records exit codes,
transcript hashes and the tested source snapshot. Local build artifacts are not
proposed for commit.

`cargo fmt --all -- --check` — exit 0:

No stdout or stderr.

`cargo clippy --workspace --all-targets -- -D warnings` — exit 0:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.43s
```

`cargo test --release -p ergo-sandbox --test evidence_wire -- --nocapture` — exit 0:

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
   Compiling ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `release` profile [optimized] target(s) in 1.77s
     Running tests/evidence_wire.rs (target-p00/release/deps/evidence_wire-647054ae0880d189)

running 4 tests
test legacy_box_missing_reference_cannot_be_promoted ... ok
test invalid_tree_never_becomes_empty_bytes ... ok
2 pinned full boxes retain bytes/IDs/references; claimed ID mismatches are rejected
test wire_box_id_matches_full_serialization ... ok
1 pinned transactions use node bytes_to_sign; proof/extension changes and canonical output references checked
test transaction_id_and_message_use_node_bytes_to_sign ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test --workspace` — exit 0:

```text
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-75a1761c109c62b3)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-ceeab01652511ab9)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-e77aa7a351a668b8)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-0a1a7009b1d119b7)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-7718b7d41a3e748e)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-deb2e97de8714ee2)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s
     Running tests/cli_json.rs (target-p00/debug/deps/cli_json-5c547a3a4ad3a795)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli_match.rs (target-p00/debug/deps/cli_match-44be0c76bab79e58)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compile_params.rs (target-p00/debug/deps/compile_params-1e5233e157a9930e)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/compose.rs (target-p00/debug/deps/compose-5fd275a0c4c23b16)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_ast.rs (target-p00/debug/deps/decompile_ast-8066b117f3b62c9b)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_corpus.rs (target-p00/debug/deps/decompile_corpus-880f0c597bf209ab)
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.07s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-dde4103a643c0ac0)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-9d1c40bead3df276)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-8f305a199ddfe690)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/drain.rs (target-p00/debug/deps/drain-30197e429d33d343)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 54.35s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-16dde7768f9e92c4)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
     Running tests/evidence_wire.rs (target-p00/debug/deps/evidence_wire-1d5cd0b85b05d1ec)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hot_spots.rs (target-p00/debug/deps/hot_spots-9f8c238fefe552c8)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hunt.rs (target-p00/debug/deps/hunt-fd72a3c71910d1b1)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/identity.rs (target-p00/debug/deps/identity-421e8bc148a52a91)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/ingest.rs (target-p00/debug/deps/ingest-d33025538526a023)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/map.rs (target-p00/debug/deps/map-cd74a2344054ccc7)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-d141a4973864a8c5)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-b0b3c031e2767c47)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-cbebec634dd22547)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 816.92s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-af3dc425ec952c9f)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/play.rs (target-p00/debug/deps/play-ee5a70cf2bf116bd)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-bb8ef4f1515e1bc1)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-64aa7b22738288d0)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-bed930132c2aec9e)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-8a6e39c9b60d3ea8)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-7e173d1bea8c8cb8)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tree.rs (target-p00/debug/deps/tree-a2486e44d7aa9f6c)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-3caa74bbff0cd590)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-acb60f0cbb787413)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-054ebdfe1e346cf8)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_web-51850fcd77aa36da)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
     Running unittests src/main.rs (target-p00/debug/deps/ergo_web-d768bb3e1ed7feb3)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/http.rs (target-p00/debug/deps/http-88c48044569cbed9)
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/map.rs (target-p00/debug/deps/map-6a998bc887fcc568)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
   Doc-tests ergo_sandbox
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
   Doc-tests ergo_web
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`python3 scripts/roadmap_gate.py --require P02` — exit 0:

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/P02.json
```

`python3 scripts/roadmap_gate.py --all-goals` — exit 1:

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
P03: unimplemented: registered product unit is not implemented
P04: unimplemented: registered product unit is not implemented
P05: unimplemented: registered product unit is not implemented
P06: unimplemented: registered product unit is not implemented
P07: unimplemented: registered product unit is not implemented
P08: unimplemented: registered product unit is not implemented
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/all-goals.json
```

`python3 scripts/roadmap_gate.py --through-completed` — exit 0:

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/through-completed.json
```

`cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` — exit 0:

```text
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.22s
```

`cargo test -p ergo-sandbox --features cost-trace` — exit 0:

```text
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-5fc8fbf662817bd0)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-85f25a33f1ea1256)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-621482f83609feab)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-31b27395f8018e45)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-91ef981139bf9d10)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-c73a01b48c76daeb)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
     Running tests/cli_json.rs (target-p00/debug/deps/cli_json-0d69d85162a7c516)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli_match.rs (target-p00/debug/deps/cli_match-4fb2fc05ca8c36f9)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compile_params.rs (target-p00/debug/deps/compile_params-bd1b2c64c75005f2)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/compose.rs (target-p00/debug/deps/compose-e878ea059a3750dd)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_ast.rs (target-p00/debug/deps/decompile_ast-0cf5d6c6277ed619)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_corpus.rs (target-p00/debug/deps/decompile_corpus-dc7fd85430ed4f30)
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.38s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-98b01c0e256dd87c)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-0673b523ab1220a6)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-d19945292cb79974)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/drain.rs (target-p00/debug/deps/drain-5c62702eb16ba635)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 53.49s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-de137a89a4f7e2d4)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
     Running tests/evidence_wire.rs (target-p00/debug/deps/evidence_wire-b8d314689418dbd4)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hot_spots.rs (target-p00/debug/deps/hot_spots-88e33ff02884b4a2)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hunt.rs (target-p00/debug/deps/hunt-122c673391a73f3e)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/identity.rs (target-p00/debug/deps/identity-2b6ca13b42a493f9)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/ingest.rs (target-p00/debug/deps/ingest-ddc83fafbab3d6a0)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/map.rs (target-p00/debug/deps/map-dbdccb920aae9848)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-fc3b033098db68cd)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-ef34c57548473c94)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-12927b8728022c25)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 814.51s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-3b97e24407e8e163)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/play.rs (target-p00/debug/deps/play-1a16441f75ee667f)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-679dc3e2f122527d)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-7d343fffdc9e4655)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-dfdcad691a8db4df)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-9f0e26478372594f)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-7100f7cc327292ab)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tree.rs (target-p00/debug/deps/tree-04c099c875780864)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-d54526283e108323)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-417acde0237fa283)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-7ee82b51918af87e)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests ergo_sandbox
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
```
