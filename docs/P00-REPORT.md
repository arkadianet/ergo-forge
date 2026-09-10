# P00 claim-contract release — local execution record

P00 changes result authority and presentation only. No lint predicate, drain
candidate family, objective, numeric baseline, or engine dependency changed.
P01 has not started. All sixteen ROADMAP section 7 hygiene items are implemented.

**Landing is blocked, not complete.** The requested first operation was
`git add docs/ARCHITECTURE-REVIEW-CODEX.md docs/ROADMAP.md`, before implementation.
The environment mounts `.git` read-only and returned:

```text
fatal: Unable to create '/home/rkadias/coding/ergo-forge/.git/index.lock': Read-only file system
```

No commit could be created. Work remains on `p00/claim-contract`; main was not
modified, nothing was pushed, and no PR was opened. This is a filesystem
restriction, not a request for renewed authorization. The two planning documents
still need to be committed first when Git writes are available, followed by the
P00 implementation. No two-day timebox has expired; no timebox stop is claimed.

The result contract now has these boundaries:

- Triage writes `formatVersion: 2`, `reproduced-in-scenario`, explicit method and
  provenance limits, and `nodeValidated: false`. Replay still calls precisely the
  same hunt and txcheck paths. Association with a lint does not establish causation.
- `TxCheck.preflightPassed` carries the existing success calculation; `valid`
  remains its documented deprecated compatibility alias. Neither checks signatures
  or full node acceptance. Rust field access remains compatible without adding
  compiler deprecation warnings to frozen callers.
- Descriptive metadata covers sandbox evaluation, spend/drain hunting, preflight,
  Play, suites, findings/triage/context audit, tree/structural reports, ingestion
  reports, and composition; API DTOs preserve it through compile/inspect, hunt,
  eval, compose, map, lookup and key derivation. CLI eval/suite JSON keeps it too.
  These labels are not P01's evidence-case schema. Nested rows inherit their
  enclosing report's scope; request objects and scalar utility values are not
  acceptance claims.
- Saved triage is output-only. The strict replay-request input rejects stored v1
  confirmation records and grafted authority fields. The UI displays legacy or
  forged confirmation labels as unverified, requiring replay.
- The shipped Read and Write handlers are exercised against a DOM parsed from
  `ui/index.html`. Both signs of synthetic hunts show the warning. Preflight
  success/failure preserves signature counts and problem rows. Severity remains
  numerically unchanged and is displayed as review priority.

The mutation artifact's prose now describes its current rows, including the
invalid excluded case and detected capped case. Historical escalation/repair
narratives are labelled historical. Every numeric JSON value, per-mutant row,
negative-control row and cap remains identical to `ee4ac6a`; custody-v1 history
is byte-identical. The record-only scoreboard pins baseline hashes, source/corpus
revisions, membership, denominators and unknown producers. It does not run a
benchmark. The required existing workspace suite does run the corpus harness.

ROADMAP changes are explicit:

- Registered units have an `implemented` flag. Planned P01–P08 fail as
  `unimplemented`; absent required P00 infrastructure/evidence fails distinctly
  as `missing-gate`. Neither can count as completion. This corrects the original
  conflation of planned missing files with broken gates.
- The policy names the record-only scoreboard inputs and command. UI test setup
  now specifies `npm ci --prefix ui`, using a locked, test-only DOM dependency.
  No threshold was reduced or copied into test code.
- `completedThrough` advances to P00's passing product gate, not a Git landing.
  The sixteen hygiene boxes and baseline-expectation table record the correction;
  P06's promotion/causation test remains unimplemented despite P00 removing the old
  confirmation label. No baseline expected-failure row unexpectedly passed before
  correction; the new acceptance tests were first executed after implementation.

The first workspace run stopped on two CLI assertions expecting the retired
“spendable by anyone” text. Those assertions were updated to the specified sample
label, keeping their probe-count assertions. The workspace run was restarted.
Final CLI and web targets were also rerun after the last presentation/DTO edits.

All verification commands below completed. Logs use `CARGO_TARGET_DIR=./target-p00`; full gate reports and raw command
outputs are retained under `target-p00/p00-verification/`.

The workspace test run returned exit 0: **422 passed, 0 failed, 1 ignored**
across its targets. The ignored test is the pre-existing opt-in
`external_compiled_fixtures_measurement` requiring `DC_NODE_CHECKOUT` and
`DC_REPORT`; the bundled decompiler round-trip gate ran and passed. P00's named
target and DOM/Python gates contain no skipped tests. The subsequent focused
web and CLI runs cover the final label/DTO refinements without changing any
engine calculation.

Docs-first recovery copies are in `target-p00/landing/`. The before-P00 roadmap
reconstructs the original policy/content with normalized JSON formatting; the
architecture review is unchanged. These are recovery aids, not commits.

| Command (all Cargo builds use `CARGO_TARGET_DIR=./target-p00`) | Exit | Captured log |
|---|---:|---|
| `cargo fmt --all -- --check` | 0 | [fmt.log](../target-p00/p00-verification/fmt.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy.log](../target-p00/p00-verification/clippy.log) |
| `cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` | 0 | [clippy-cost-trace.log](../target-p00/p00-verification/clippy-cost-trace.log) |
| `cargo test --release -p ergo-sandbox --test claim_contract -- --nocapture` | 0 | [claim.log](../target-p00/p00-verification/claim.log) |
| `python3 -m unittest discover -s scripts -p 'test_roadmap_gate.py'` | 0 | [python.log](../target-p00/p00-verification/python.log) |
| `node --test ui/tests/claim-labels.test.js` | 0 | [ui.log](../target-p00/p00-verification/ui.log) |
| `python3 scripts/roadmap_gate.py --require P00` | 0 | [gate.log](../target-p00/p00-verification/gate.log) |
| `python3 scripts/roadmap_gate.py --all-goals` | 1 | [all-goals.log](../target-p00/p00-verification/all-goals.log) |
| `python3 scripts/roadmap_gate.py --scoreboard-only` | 0 | [scoreboard.log](../target-p00/p00-verification/scoreboard.log) |
| `cargo test --workspace` | 0 | [workspace.log](../target-p00/p00-verification/workspace.log) |
| `cargo test -p ergo-sandbox --features cost-trace` | 0 | [test-cost-trace.log](../target-p00/p00-verification/test-cost-trace.log) |
| `cargo test -p ergo-web` | 0 | [web-final.log](../target-p00/p00-verification/web-final.log) |
| `cargo test -p ergo-sandbox --test cli_hunt --test claim_contract` | 0 | [cli-final.log](../target-p00/p00-verification/cli-final.log) |
| `python3 scripts/roadmap_gate.py --through-completed` | 0 | [prefix.log](../target-p00/p00-verification/prefix.log) |

Actual output excerpts follow; the linked files retain full output, and
[commands.json](../target-p00/p00-verification/commands.json) records exit codes
and log hashes. No checklist item is deferred. Only the requested commits are
blocked. `--all-goals` returning 1 is the required unfinished-product result,
not an infrastructure failure.

`cargo fmt --all -- --check` — exit 0:

```text
(no output)
```

`cargo clippy --workspace --all-targets -- -D warnings` — exit 0:

```text
Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Checking ergo-web v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-web)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.85s
```

`cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` — exit 0:

```text
Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.22s
```

`cargo test --release -p ergo-sandbox --test claim_contract -- --nocapture` — exit 0:

```text
Compiling ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Finished `release` profile [optimized] target(s) in 12.93s
     Running tests/claim_contract.rs (target-p00/release/deps/claim_contract-9c89443942812643)

running 3 tests
stored confirmed records and imported authority fields rejected; replay request required
test legacy_record_cannot_import_as_verified ... ok
positive and negative legacy envelopes: explicit method/provenance, nodeValidated=false
test legacy_results_are_explicitly_preflight_or_simulation ... ok
formatVersion=2 state=reproduced-in-scenario preflightPassed=true nodeValidated=false
test scenario_reproduction_is_not_confirmation ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
```

`python3 -m unittest discover -s scripts -p 'test_roadmap_gate.py'` — exit 0:

```text
.............
----------------------------------------------------------------------
Ran 13 tests in 0.035s

OK
```

`node --test ui/tests/claim-labels.test.js` — exit 0:

```text
1..7
# tests 7
# suites 0
# pass 7
# fail 0
# cancelled 0
# skipped 0
# todo 0
# duration_ms 364.424327
```

`python3 scripts/roadmap_gate.py --require P00` — exit 0:

```text
P00: passed: required tests executed
scoreboard: passed
P00: passed: required tests executed
```

`python3 scripts/roadmap_gate.py --all-goals` — exit 1:

```text
P00: passed: required tests executed
P01: unimplemented: registered product unit is not implemented
P02: unimplemented: registered product unit is not implemented
P03: unimplemented: registered product unit is not implemented
P04: unimplemented: registered product unit is not implemented
P05: unimplemented: registered product unit is not implemented
P06: unimplemented: registered product unit is not implemented
P07: unimplemented: registered product unit is not implemented
P08: unimplemented: registered product unit is not implemented
scoreboard: passed
P00: passed: required tests executed
P01: unimplemented: registered product unit is not implemented
P02: unimplemented: registered product unit is not implemented
P03: unimplemented: registered product unit is not implemented
P04: unimplemented: registered product unit is not implemented
P05: unimplemented: registered product unit is not implemented
P06: unimplemented: registered product unit is not implemented
P07: unimplemented: registered product unit is not implemented
P08: unimplemented: registered product unit is not implemented
```

`python3 scripts/roadmap_gate.py --scoreboard-only` — exit 0:

```text
scoreboard: passed
report: /home/rkadias/coding/ergo-forge/target-p00/roadmap-gates/scoreboard.json
```

`cargo test --workspace` — exit 0:

```text
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-cbebec634dd22547)

running 3 tests
test bank_template_registers_reach_both_consumers ... ok
test m4_recorded_seller_only_receipts_score_zero_on_mutant_and_original ... ok
test the_corpus_is_measured_and_does_not_regress has been running for over 60 seconds
test the_corpus_is_measured_and_does_not_regress ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 850.94s
```

`cargo test -p ergo-sandbox --features cost-trace` — exit 0:

```text
     Running tests/mutation_corpus.rs (target-p00/debug/deps/mutation_corpus-12927b8728022c25)

running 3 tests
test bank_template_registers_reach_both_consumers ... ok
test m4_recorded_seller_only_receipts_score_zero_on_mutant_and_original ... ok
test the_corpus_is_measured_and_does_not_regress has been running for over 60 seconds
test the_corpus_is_measured_and_does_not_regress ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 823.84s
```

`cargo test -p ergo-web` — exit 0:

```text
     Running unittests src/lib.rs (target-p00/debug/deps/ergo_web-d5b5cc9b6d971b0e)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
     Running unittests src/main.rs (target-p00/debug/deps/ergo_web-2230dfdc132befbd)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/http.rs (target-p00/debug/deps/http-531e7f73d719faa7)
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
     Running tests/map.rs (target-p00/debug/deps/map-3abfdb222ff73981)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p ergo-sandbox --test cli_hunt --test claim_contract` — exit 0:

```text
     Running tests/claim_contract.rs (target-p00/debug/deps/claim_contract-2c119230ad732d8c)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
     Running tests/cli_hunt.rs (target-p00/debug/deps/cli_hunt-fee3e947130fdb3f)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s
```

`python3 scripts/roadmap_gate.py --through-completed` — exit 0:

```text
P00: passed: required tests executed
scoreboard: passed
P00: passed: required tests executed
```
