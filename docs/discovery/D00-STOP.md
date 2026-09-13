# D00 registration attempt — stopped, 2026-09-11

D00 is **not implemented**. The retained coherent prefix is P00–P08 and
M00–M04, with the existing M05 stop. No property inventory is accepted, no
reference transaction was measured, and no evaluator or later unit was started.
The intended 24-case denominator remains a target; its measured denominator and
the separate transfer denominator remain `null`. No discovery or utility credit
is awarded. Independent human semantic review has not been supplied or claimed.

The author authorized D00 on `a57df087df85813c7f90a6f6f9ad6aaf5b4597cd`
but required all five exact proposed registrations, only D00 implemented after
its gate, unchanged CI semantics, and a zero exit from `--ci`. These requirements
cannot hold simultaneously: CI checks the entire units array and rejects every
unimplemented unit without a validated stop. Even a passing D00 would leave
D01–D04 unimplemented and make `--ci` exit 1. They have not been attempted;
inventing failed executions or cancellation records to make CI green would not
resolve the scheduling conflict.

There is a second registration integration defect: M00's protected-policy
projection removes only M units before comparing its pinned P08-policy digest.
Appending D entries changes that projection and fails
`legacy_metrics_are_measured_without_baseline_changes`. Its pinned digest must
remain unchanged. A future explicit projection update could authenticate all
original P units and policy fields while separately authenticating the D append;
this attempt does not edit that completed test or its expected data.

The [reproduction script](d00-stop-evidence/reproduce_registration.py) temporarily
appends the exact five false entries from section 5.3, runs the real unmodified
`--ci` runner and restores the original roadmap bytes in `finally`. The
[proposed roadmap](d00-stop-evidence/proposed-roadmap.fixture),
[receipt](d00-stop-evidence/registration-receipt.json),
[raw log](d00-stop-evidence/proposed-ci.log) and
[command report](d00-stop-evidence/proposed-ci.json) preserve the attempted
registration and actual test results. This is a policy diagnostic, not a D00
product test or a measurement of property semantics. All D flags in that
diagnostic are false; no passing D00 is simulated.

To reproduce that historical experiment, use a disposable workspace at the
recorded attempt commit with the archived script copied into its recorded path,
before adding this stop record. Do not rerun it over these pinned evidence files:
it writes its outputs, and the new D00 stop would change the runner's result.

Section 5.3's proposed queue is retained unchanged in the design. An appended
design amendment records why registration cannot yet land under the requested
gate contract. The final governing units array, completedThrough, baselineRev,
thresholds, frozenPreflight and all P/M flags remain unchanged. The stop registry
only appends this D00 proposal stop; previous records and resolutions remain.

Reopening requires an explicit scheduling decision reconciling registration of
future false units with all-goal CI, without counting them as completed or
silently accepting unfinished goals, plus the M00 policy-projection integration
change retaining its original digest. Then implement the original D00 inventory
and acceptance requirements, including honest reviewer/transfer accounting.
Capabilities 2 and 3 remain contingent; capability 1 may be the endpoint.

Verification output and exits are recorded below. Everything is
uncommitted; no Git mutation, protocol contact or broadcast occurred.

## Verification — actual output

Every command used `CARGO_TARGET_DIR=./target-p00`. The
[verification receipt](d00-stop-evidence/verification.json) records exact commands,
exits and SHA-256 hashes of complete logs. Excerpts below are verbatim; the fmt
command produced no output.

`cargo fmt --all -- --check` — exit **0**. [Full output](d00-stop-evidence/fmt.log).

No stdout or stderr.

`cargo clippy --workspace --all-targets -- -D warnings` — exit **0**. [Full output](d00-stop-evidence/clippy.log).

```text
Checking ergo-sandbox v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-sandbox)
    Checking ergo-web v0.3.0 (/home/rkadias/coding/ergo-forge/ergo-web)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.93s
```

`cargo test -p ergo-sandbox --release --test property_inventory -- --show-output` — exit **101**. [Full output](d00-stop-evidence/property-inventory.log).

```text
error: no test target named `property_inventory` in `ergo-sandbox` package
```

`cargo test --workspace --release` — exit **0**. [Full output](d00-stop-evidence/workspace.log).

Summing the actual target summaries gives **481 passed / 0 failed / 1 ignored**, matching main. The unchanged external-only test is:

```text
test external_compiled_fixtures_measurement ... ignored, external node checkout; set DC_NODE_CHECKOUT and DC_REPORT
```

`python3 -m unittest discover -s scripts -p test_roadmap_gate.py` — exit **0**. [Full output](d00-stop-evidence/python.log).

```text
Ran 20 tests in 0.043s
OK
```

`python3 scripts/roadmap_gate.py --require D00` — exit **1**. [Full output](d00-stop-evidence/require-d00.log).

```text
policy/evidence: missing-gate: unknown unit: D00
report: <repo>/target-p00/roadmap-gates/D00.json
```

`python3 scripts/roadmap_gate.py --through-completed` — exit **0**. [Full output](d00-stop-evidence/through-completed.log).

```text
M04: passed: required tests executed
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
M00: passed: required tests executed
M01: passed: required tests executed
M02: passed: required tests executed
M03: passed: required tests executed
M04: passed: required tests executed
report: <repo>/target-p00/roadmap-gates/through-completed.json
```

`python3 scripts/roadmap_gate.py --ci` — exit **0**. [Full output](d00-stop-evidence/ci.log).

```text
M05: stopped: recorded stop blocks this unit
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
M00: passed: required tests executed
M01: passed: required tests executed
M02: passed: required tests executed
M03: passed: required tests executed
M04: passed: required tests executed
M05: stopped: recorded stop blocks this unit
report: <repo>/target-p00/roadmap-gates/ci.json
```

The final `--ci` pass applies to the unchanged registered P/M queue, with M05
accepted as a validated stop. It does **not** mean D00 passed. The D00 proposal
stop is validated as stop evidence but, because D00 is unregistered, it is not a
selected goal. Its direct required-unit command remains nonzero. No threshold
or CI acceptance rule was relaxed. `--all-goals` was not rerun and remains
expected to reject M05; no all-goals completion is claimed.

The governing policy block was compared byte-for-byte with HEAD and is identical.
The prior stop objects are unchanged, and every new stop evidence hash matches.
No production source, Rust tests, prior fixtures, lockfile or numeric baseline
was edited.
