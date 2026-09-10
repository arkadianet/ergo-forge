# P01 — local implementation and verification record

P01 adds versioned, provenance-bound inputs and static inspection. Changes are
uncommitted on `p00/claim-contract`, following the user-committed P00 changes
`8630ddc` and `5ecb33b`. No Git write was attempted. Main, remotes, detectors,
generation, drain families, caps, objectives, corpus rows and numeric baselines
are unchanged. P02 has not started.

The implementation is in `ergo-sandbox/src/evidence/{mod,case}.rs`, with adapters
in ingestion, structural identity comparison and map source recording. See
[evidence-cases.md](evidence-cases.md) for the library API and format boundaries.

| Acceptance | Implemented behavior |
|---|---|
| `synthetic_binding_survives_export_and_import` | Ingestion retains exact target bytes, source identity/text, binding values/mechanisms/origins and compiler options in the artifact and report. Success and partial failure cases round-trip. Every final archived source row retains its recorded bindings and source identity; missing origins or inventory members are rejected. |
| `structural_match_does_not_establish_deployment` | Structural comparison retains both cases and reports deployment identity unknown, including identical bytes. Case import rejects forged acceptance claims and unsupported versions. |
| `missing_registers_are_not_empty_registers` | Source JSON is captured before legacy box defaults. Missing, null and empty registers retain distinct records and cache identities. Context defaults and hypothetical values survive export/import. |
| `changing_any_premise_invalidates_cached_evidence` | Cache lookup binds every premise, source identity, origin, context and policy assumption, plus comparison dependencies. Changed inputs return no cached result; imported results cannot become trusted cache entries. |

The archived compilation inventory is read without rerunning ingestion: 28 rows,
26 recorded compilations, two failures. Binding values and origins exist in
`after.contracts`; source text and target bytes do not. Cases retain that absence
and cannot analyze invented historical artifacts. Deployment equivalence remains
unknown; node-validated claim count remains zero. A recomputed source digest is
labelled as a check of attached source text only, never of UTXO membership or
source-to-deployment equivalence.

The existing low-level byte/IR functions and legacy box defaulting remain
available. Evidence-bearing callers use `artifact.analyze()`, `case.analyze()`
or `identity::match_cases()`, and capture box JSON with `map::source::record_box`
before defaulting. P01 does not recover provenance from a previously defaulted
box, validate transactions, or migrate legacy browser state. These are the P01
boundaries, not skipped acceptance items.

No acceptance gate or threshold was amended. ROADMAP changes only register P01
as implemented, advance `completedThrough` after P00/P01 reran green, and link
this record and the API documentation. CI's required implementation unit changes
from P00 to P01. P02–P08 remain registered and unimplemented.

An initial implementation test failed because the archive adapter selected
`prototype_compile_only` (18 compilations) rather than the final `after.contracts`
(26). The adapter was corrected; the archive and policy floor were not changed.
An initial clippy failure required moving the new map adapter before the existing
test module. Neither failure required a scope or gate change.

All required checks completed. Workspace totals, summed from Cargo's target
summaries, are **426 passed / 0 failed / 1 ignored**: P00's 422 passed tests plus
P01's four. Cost-trace totals are **359 passed / 0 failed / 1 ignored**. Both
retain the same existing ignored `external_compiled_fixtures_measurement`; no
P01 or registered predecessor test was skipped. The full-goal command exits 1
only because P02–P08 are unimplemented, with zero `missing-gate` results. No P01
acceptance item is incomplete. The three-working-day ceiling has not been
reached; no stop record is warranted.

## Actual command output

Every command below used `CARGO_TARGET_DIR=./target-p00`. Outputs below are
verbatim, with full-suite output limited to each target's Cargo summary and gate
runner output limited to its final status summary. Complete stdout/stderr and
machine reports are retained in [target-p00/p01-verification](../target-p00/p01-verification/),
with exit codes and transcript hashes in
[manifest.json](../target-p00/p01-verification/manifest.json). These local build
artifacts are not proposed for commit. Empty formatter output is stated explicitly;
aggregate counts above are derived from the printed target summaries, not a
fabricated Cargo summary line.

`cargo fmt --all -- --check` — exit 0:

No stdout or stderr.

`cargo clippy --workspace --all-targets -- -D warnings` — exit 0:

```text
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Checking ergo-web v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-web)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.90s
```

`cargo test --release -p ergo-sandbox --test evidence_provenance -- --nocapture` — exit 0:

```text
    Finished `release` profile [optimized] target(s) in 0.09s
     Running tests/evidence_provenance.rs (target-p00/release/deps/evidence_provenance-3a25201ae178db74)

running 4 tests
test missing_registers_are_not_empty_registers ... ok
test structural_match_does_not_establish_deployment ... ok
all case premises and comparison dependencies are bound; stale cache entries return no result
test changing_any_premise_invalidates_cached_evidence ... ok
recorded inventory: 28 rows retained; recorded binding values/origins retained; missing source text/target bytes not invented
test synthetic_binding_survives_export_and_import ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
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
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-deb2e97de8714ee2)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
     Running tests/cli_json.rs (target-p00/debug/deps/cli_json-5c547a3a4ad3a795)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli_match.rs (target-p00/debug/deps/cli_match-44be0c76bab79e58)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compile_params.rs (target-p00/debug/deps/compile_params-1e5233e157a9930e)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compose.rs (target-p00/debug/deps/compose-5fd275a0c4c23b16)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/decompile_ast.rs (target-p00/debug/deps/decompile_ast-8066b117f3b62c9b)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_corpus.rs (target-p00/debug/deps/decompile_corpus-880f0c597bf209ab)
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.15s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-dde4103a643c0ac0)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-9d1c40bead3df276)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-8f305a199ddfe690)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/drain.rs (target-p00/debug/deps/drain-30197e429d33d343)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 56.66s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-16dde7768f9e92c4)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
     Running tests/hot_spots.rs (target-p00/debug/deps/hot_spots-9f8c238fefe552c8)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hunt.rs (target-p00/debug/deps/hunt-fd72a3c71910d1b1)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/identity.rs (target-p00/debug/deps/identity-421e8bc148a52a91)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/ingest.rs (target-p00/debug/deps/ingest-d33025538526a023)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/map.rs (target-p00/debug/deps/map-cd74a2344054ccc7)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-d141a4973864a8c5)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-b0b3c031e2767c47)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-cbebec634dd22547)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 819.50s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-af3dc425ec952c9f)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/play.rs (target-p00/debug/deps/play-ee5a70cf2bf116bd)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/proofs.rs (target-p00/debug/deps/proofs-bb8ef4f1515e1bc1)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/recognize.rs (target-p00/debug/deps/recognize-64aa7b22738288d0)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rent.rs (target-p00/debug/deps/rent-bed930132c2aec9e)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/sandbox_eval.rs (target-p00/debug/deps/sandbox_eval-8a6e39c9b60d3ea8)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/testsuite.rs (target-p00/debug/deps/testsuite-7e173d1bea8c8cb8)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tree.rs (target-p00/debug/deps/tree-a2486e44d7aa9f6c)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/triage.rs (target-p00/debug/deps/triage-3caa74bbff0cd590)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.67s
     Running tests/txcheck.rs (target-p00/debug/deps/txcheck-acb60f0cbb787413)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/vault_corpus.rs (target-p00/debug/deps/vault_corpus-054ebdfe1e346cf8)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_web-51850fcd77aa36da)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
     Running unittests src/main.rs (target-p00/debug/deps/ergo_web-d768bb3e1ed7feb3)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/http.rs (target-p00/debug/deps/http-88c48044569cbed9)
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
     Running tests/map.rs (target-p00/debug/deps/map-6a998bc887fcc568)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
   Doc-tests ergo_sandbox
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
   Doc-tests ergo_web
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`python3 scripts/roadmap_gate.py --require P01` — exit 0:

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/P01.json
```

`python3 scripts/roadmap_gate.py --all-goals` — exit 1:

```text
scoreboard: passed
P00: passed: required tests executed
P01: passed: required tests executed
P02: unimplemented: registered product unit is not implemented
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
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/through-completed.json
```

`cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` — exit 0:

```text
    Blocking waiting for file lock on build directory
    Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.64s
```

`cargo test -p ergo-sandbox --features cost-trace` — exit 0:

```text
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_sandbox-5fc8fbf662817bd0)
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/bin/ergo-es.rs (target-p00/debug/deps/ergo_es-85f25a33f1ea1256)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/audit.rs (target-p00/debug/deps/audit-621482f83609feab)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/audit_context.rs (target-p00/debug/deps/audit_context-31b27395f8018e45)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-91ef981139bf9d10)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-c73a01b48c76daeb)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
     Running tests/cli_json.rs (target-p00/debug/deps/cli_json-0d69d85162a7c516)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli_match.rs (target-p00/debug/deps/cli_match-4fb2fc05ca8c36f9)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/compile_params.rs (target-p00/debug/deps/compile_params-bd1b2c64c75005f2)
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/compose.rs (target-p00/debug/deps/compose-e878ea059a3750dd)
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/decompile_ast.rs (target-p00/debug/deps/decompile_ast-0cf5d6c6277ed619)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/decompile_corpus.rs (target-p00/debug/deps/decompile_corpus-dc7fd85430ed4f30)
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.42s
     Running tests/decompile_robustness.rs (target-p00/debug/deps/decompile_robustness-98b01c0e256dd87c)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/decompile_roundtrip.rs (target-p00/debug/deps/decompile_roundtrip-0673b523ab1220a6)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
     Running tests/delegated_reserves.rs (target-p00/debug/deps/delegated_reserves-d19945292cb79974)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/drain.rs (target-p00/debug/deps/drain-5c62702eb16ba635)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 52.44s
     Running tests/evidence_provenance.rs (target-p00/debug/deps/evidence_provenance-de137a89a4f7e2d4)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
     Running tests/hot_spots.rs (target-p00/debug/deps/hot_spots-88e33ff02884b4a2)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/hunt.rs (target-p00/debug/deps/hunt-122c673391a73f3e)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/identity.rs (target-p00/debug/deps/identity-2b6ca13b42a493f9)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/ingest.rs (target-p00/debug/deps/ingest-ddc83fafbab3d6a0)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/map.rs (target-p00/debug/deps/map-dbdccb920aae9848)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
     Running tests/method_sweep.rs (target-p00/debug/deps/method_sweep-fc3b033098db68cd)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
     Running tests/more_lints.rs (target-p00/debug/deps/more_lints-ef34c57548473c94)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-12927b8728022c25)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 820.45s
     Running tests/payment_reserves.rs (target-p00/debug/deps/payment_reserves-3b97e24407e8e163)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/play.rs (target-p00/debug/deps/play-1a16441f75ee667f)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
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
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
```
