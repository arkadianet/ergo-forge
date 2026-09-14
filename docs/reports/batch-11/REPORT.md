# Batch 11 — S03: the two-instance family in the drain hunt

Implemented on `v2/batch-11`, based on `fc26a95` (batches 0–10, engine pin
`0165331`, version 0.5.0 prepared), on 2026-09-14.

**Who implemented it.** Codex's cybersecurity classifier refused the unit
brief twice (the original and a rephrased one that kept only file paths and
the fixed test name), before any work started. Per the batch process, the
unit was implemented directly by the Claude session instead. It is
authorised defensive work: a bounded search family over authored test
contracts, gated by a counterexample/control pair, with the same caps,
objective and promotion path as the existing families.

## What landed

- `ergo-sandbox/src/drain.rs`: an opt-in degree `synthesis.multiInstance`.
  When on, the hunt runs the unchanged single-request hunt on the base
  request and on one derived request per declared `protected` input, each
  inside an equal slice of the request's probe cap (remainder to the earliest
  runs), and merges the reports. `second_instance` derives the copy: same
  script, value, tokens and registers; the next creation height; no declared
  id; and, when `tokens(0)` is a protocol NFT, its own hypothetical singleton
  in its place (two instances of one contract each carry their own NFT; a
  second copy of a singleton is not a shape the chain can produce, so the
  family never asks the oracle about one). The derived request declares that
  singleton in `protocolNfts`.
- The family sits outside the synthesis axes: each derived request is hunted
  with the unchanged pinned order. Same caps (`maxProbes` split, not
  widened; `maxPermutations` and the synthesis caps unchanged), same
  objective (`recognized-attacker-receipts-v1`), same accounting.
- The report gains `synthesis.multiInstance` **only when the family ran**
  (`skip_serializing_if`), carrying the protected-input count, the cap per
  run, the family's position in the axis order, and one record per run
  (label, derived-from index, the exact derived box, the derived singleton,
  probes total/run, oracle calls, capped, hits, verdict). Derived runs' shape
  tallies are appended under their label; a hit's `shape` names the run it
  came from. With the degree off (absent or explicit `false`) the report is
  byte-identical to before the family existed (`tests/drain.rs::absent_two_instance_degree_is_byte_identical`).
- Promotion is unchanged. The family reports the derived box; a caller who
  wants node-validated evidence declares it (with a hypothetical id) as one
  more `protected` input and promotes that explicit request through P06.
  This keeps the P06 rule that every candidate input is a declared input.

## The pair and the measurement

`examples/mutants/search.json` registers `S03-01-v1` (a new, append-only
namespace: the frozen M1–M8 corpus, `staticLintPairs` and `recipeMutants`
are untouched). Counterexample and control are the S00 class-11 vector pair
`examples/contracts/vectors/shared-payment-across-instances/{vulnerable,fixed}.es`:
each instance requires `OUTPUTS(0)` to pay at least its own value to a fixed
key; the fixed contract additionally requires exactly one input carrying its
script. The request declares one protected instance (10 ERG, a singleton at
`tokens(0)`), an attacker funding input, a fixed payment output that carries
the singleton, and a free output. The payment carries the singleton because
the vector contract never binds its token: with a bare payment the existing
families already report the NFT leaving, and the pair would not isolate the
two-instance mechanism.

| Run | Counterexample | Control |
|---|---|---|
| Family off (one declared instance) | `notUnderProbes` | `notUnderProbes` |
| Family on | **`drainable`**, hit from `two-instance(protected=0)`, 10 ERG extracted | `notUnderProbes`, derived run ran with 0 hits |
| Declared two-instance request through P06 | `confirmed-violation`, `nodeValidated: true`, preflight record unchanged | — |

Caps: `maxProbes` 50000 (split 25000/25000 across the base and derived
runs), `maxPermutations` 120. Truncation is recorded per run
(`caps_and_truncation_are_recorded` drives a three-probe cap and checks every
run's slice, the sum, and each run's `capped` flag). No stop was needed: the
counterexample is found well inside the family's slice.

The vector catalogue row `shared-payment-across-instances` names the
instrument `drain` alongside `scenario`; `manual` stays because the reviewer
checks remain. `docs/roadmap-metrics.json` gains an `S03` scoreboard entry;
no historical field changed.

## Honesty boundaries

- A hit is an unsigned preflight candidate; the family never promotes and
  never rewrites the preflight record. A miss on every run is "not under
  these probes".
- The derived second instance is hypothetical material. Node validation is
  transaction-local; whether a protocol can actually have two instances of a
  script on chain is a premise the promoting caller supplies, and the claim
  is conditional on it.
- The family derives at most one extra instance per protected input. Three
  or more instances, and instances derived from `companion` inputs, are
  outside this family and would be a new axis, which the plan does not
  authorise.

## Gate

- `ergo-sandbox/tests/drain_promotion.rs`: `two_instance_mutant_is_found_and_control_is_not`,
  `caps_and_truncation_are_recorded`, plus `two_instance_candidate_promotes_when_declared`.
- `ergo-sandbox/tests/drain.rs`: `absent_two_instance_degree_is_byte_identical`.
- S03 is `implemented: true` in the `roadmap-policy:v2` block and the spec's
  `newUnits` example; the implemented-set assertion adds `S03`. The v1 block,
  the mapping/properties manifests, the answer-key history and the frozen
  mutation measurement are unchanged.

## Verification

Every command below was run outside any sandbox in this worktree with
`CARGO_TARGET_DIR=$CARGO_TARGET_DIR`; exits are as observed. The full
workspace suite ran with no skips.

| Command | Exit |
|---|---:|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace -- -D warnings` | 0 |
| `python3 scripts/test_roadmap_gate.py` | 0 |
| `python3 scripts/test_release.py` | 0 |
| `cargo test -p ergo-sandbox --test drain_promotion` | 0 |
| `cargo test -p ergo-sandbox --test drain` | 0 |
| `cargo test -p ergo-sandbox --test vector_catalogue` | 0 |
| `cargo test --workspace` (no test filter, no `--skip`) | 0 |
| `cargo test -p ergo-sandbox -p ergo-web --features cost-trace -- --skip the_corpus_is_measured_and_does_not_regress` | 0 |
| `cargo test -p ergo-web --no-default-features` | 0 |
| `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory --test mutation_corpus` | 0 |
| `python3 scripts/roadmap_gate.py --require S03 --report docs/reports/batch-11/S03.json` | 0 |

The filtered output of the workspace suite, the feature-variant suites, the
guards and the gate run is committed as [`logs/verification.log`](logs/verification.log);
the ledger is `commands.json`, including three earlier failing attempts and
what each taught. The committed `S03.json` written by the gate run hashes as
`3e0a6b3e94b0c9585fe998269ef74951107ad1d2cd6903e4cdd2c39b3f7fda24`.

After that run, one comment-only edit was made (the module doc bullet in
`drain.rs` describing the family); `cargo fmt --all -- --check` and `cargo check`
were re-run on the final tree and exited 0.

## Review follow-up

CodeRabbit's first review found that the per-run floor of one probe let the
slices sum to more than the requested cap when `maxProbes` was smaller than
the number of runs. The slices now always sum to the cap; a run left with no
budget is recorded as truncated and never run (`caps_and_truncation_are_recorded`
covers `maxProbes = 1`). The ledger now records the executable workspace
command with the no-filter condition as a note. After the fix,
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test -p ergo-sandbox --test drain_promotion` and
`cargo test -p ergo-sandbox --test drain` were re-run and exited 0.
