# Finding confirmation gate

Implemented on `feat/gate`; offline analysis only.

## Behavior

- Every finding starts `unconfirmed`, with explicit static-only evidence and
  `consensusReducerConsulted: false`.
- `ergo-es triage <request.json>` re-audits the selected input's contract,
  verifies the lint/node anchor, runs the declared drain request, and emits
  the finding with recorded evidence.
- `confirmed` requires the hunt to reach its objective and the winning
  witness to pass an additional local txcheck replay. Evidence includes the
  exact contract set, objective, bounds, winning shape, full transaction,
  accounting, and reducer verdict.
- `not-reproduced` records the hunt's budget, cap status, probe counts,
  oracle calls, and shape tallies. Its warning is explicit:
  **Absence of a result under a bound is not evidence of absence.
  Not-reproduced does not mean safe.**
- Invalid shapes and incomplete objectives remain unconfirmed with hunt
  evidence. Stale finding anchors are rejected.
- CLI audit/map output, web finding DTOs, map JSON, and browser finding
  displays expose triage. State fields are private and cannot be deserialized
  from user-supplied claims of confirmation.

## Reproduction and measured incident results

Committed inputs:
`examples/incidents/use-lp.triage.json` and
`examples/incidents/fixed/use-lp.triage.json`.

Both select the pool's `delegated-reserves` finding, node 31, input 0.
Only the companion swap's compiled tree changes. The finding remains on the
pool; the fixed companion closes the substitution. This confirms the
contract-set objective, not unique causation by the selected lint.

| Contract set | State | Probes / oracle calls | Hits | Cap reached |
|---|---|---:|---:|---|
| Deployed | confirmed | 84 / 84 | 2 | no |
| Fixed companion | not-reproduced | 84 / 84 | 0 | no |

Both declare 50,000 maximum probes and 120 maximum input permutations,
with synthesis disabled and no authorized release terms. The explored
template family is named `none`: 96 generated probes, 84 executed after
deduplication. These are actual regression and CLI run measurements.

The winning deployed witness used input permutation `[2, 1, 0]`,
`input 2: filler tokens 0..=2`, and `drain` payout. Its replayed reducer
verdict was `valid: true`; recognized net receipts included
284,695,581,089,453 nanoERG. The evidence retains the complete token
accounting and transaction as well.

The generator `triage_incident_requests` rebuilds these committed requests
from the existing incident boxes and fixed source. Tests verify fixed
contract compilation, witness replay, CLI JSON, default evidence,
invalid/incomplete runs, stale anchors, and a one-probe truncated miss.

## Supporting accounting change

Drain reports now count actual txcheck calls separately from constructed
probes. Template-only hunts now populate shape generation/execution tallies,
which previously stayed zero when synthesis was disabled. Probe generation,
scoring, and detector logic are unchanged.

The decompiler corpus automatically includes new JSON fixtures. Its existing
328 expectations were checked unchanged; nine measured entries were added:
three byte-identical round trips and six recompiling with different bytes.
Existing divergences remain visible. The mutation corpus harness and mutation
answer key were not edited.

## Final validation

All Cargo commands used `CARGO_TARGET_DIR=./target-gate`.

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --release -- --nocapture`: passed (exit 0); 379 tests
  passed, zero failed, one existing external-node fixture test ignored,
  across 35 reported test/doc-test suites.
- Mutation corpus: passed without changing its harness or answer key.
  Actual attributable detection rate: **4/5 = 0.80**; four raw drainable
  proven mutants; zero confounded detections.
- All five new triage tests passed, including the one-probe truncated miss.
- Both committed requests ran successfully through the CLI.
- `node --check ui/app.js` and `git diff --cached --check`: passed.

The first full release run stopped at the decompiler's strict membership
check because the new JSON requests added nine entries. Those entries were
measured and added without changing any existing expectation; the final
full release run passed.

The isolated target directory and temporary measurement logs were removed
after validation. Code and committed fixtures are committed on `feat/gate`;
this report remains uncommitted. Nothing was pushed and no PR was opened.
