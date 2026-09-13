# Contract identity matching — measured report

Worktree: `/home/rkadias/coding/ergo-forge-match`, branch `feat/match`.
Date: 2026-09-09. Every Cargo invocation used `CARGO_TARGET_DIR=./target-match`.
This report is intentionally uncommitted.

## Method

The library compares parsed ErgoTree opcode IR, retaining non-constant metadata
and replacing constant leaves with holes. Typed constants resolve independently
from each table; inline constants work too. It compares ordered child paths, not
source strings, printed ASTs, serialized substrings, or a longest-common-substring
threshold. Similarity counts equal labels at the same paths over the union of
paths, plus one tree-version label. Exact structure is required for a match.

## Actual fixture measurements

Measured by `cargo test --release -p ergo-sandbox --test identity --test cli_match -- --nocapture`
with the isolated target directory. All nine tests passed (eight library, one CLI).

| Comparison | Verdict | Equal / total labels | Similarity | Differing constant occurrences |
|---|---|---:|---:|---:|
| Dexy | same_program_with_differing_constants | 121/121 | 1.000000000 | 6 |
| Phoenix bank | same_program_with_differing_constants | 295/295 | 1.000000000 | 1 |
| Phoenix bank sibling | different_program | 42/423 | 0.099290780 | 24 |
| Phoenix proxy | same_program_with_differing_constants | 277/277 | 1.000000000 | 2 |
| Phoenix proxy sibling | different_program | 12/556 | 0.021582734 | 58 |

All three positive fixtures were byte-different from their source compilations.
For Dexy, `feeNumLp: Long = 3`, `feeDenomLp: Int = 1000`: table indices
9, 11, 12, and 14 differ as Long(3) versus Long(997); indices 10 and 13 differ
as Int(1000) versus Long(1000). Thus the numeric-type difference occurs at **two
table positions** in this compiler's output, rather than one position.

An additional actual CLI run with both Dexy parameters typed Int returned
`different_program`: 119/123 labels, similarity 0.967479674796748. Two extra cast
nodes are present. With both typed Long it returned a match at 121/121, with four
value differences and no type differences. The README makes the required types
explicit. No cast was erased to force a positive result. The prompt's 137-byte
substring figure is not used as a matching criterion or reported as a measurement.

Phoenix source parameters: `phoenixFeeContractBytes: Coll[Byte] = 00010203`,
`minTxOperatorFee: Long = 2000000`; the hodlcoin bank sibling additionally uses
`phoenixFeeContractBytesHash: Coll[Byte] = 32 zero bytes`. The deployed bank differs
at table index 16 (fee contract bytes); the proxy differs in its fee constant
occurrences. An exploratory run using minTxOperatorFee=1000000 produced
`same_program`, 277/277 with zero constant differences for the proxy: those
constants already matched deployment. The final positive test deliberately uses
a distinct fee value, rather than changing the deployed fixture.

## Deployment provenance

Dexy uses the existing incident test's `tree` unchanged. Phoenix fixtures were
retrieved from the explorer API and pinned in
`ergo-sandbox/tests/fixtures/identity_phoenix.json` with provenance:

- Transaction: https://api.ergoplatform.com/api/v1/transactions/a56eeb590442bba3cbbb055f353d0bbbdb9c454c703563e8d513a1b83e2a92d1
- Bank input: f7798e12e1324150b764891a7bf3977005f5ff8363a439ae3045e6e15edfb9d0 (1,225 bytes).
- Burn proxy input: cc7b4730a96774fe13f03fad72c4d1d8d983c9809a1855012c8c5ab18631c2c6 (585 bytes).
- Bank output: 9214b56aec284b3356e7901c8d8608b89d3122639bc8eef08f9d4c63f3558820.

The transaction spends a bank with 20,664,138,144 hodlCOMET10 and a proxy with
10,900,000 hodlCOMET10, returning 20,675,038,144 hodlCOMET10 to the bank and
paying COMET out. Tests use only pinned trees and never require explorer access.

## Limits and review

Structural matching does not prove behavioural equivalence: constants may change
branch reachability, authorization, or token identity. It does not prove safety,
source provenance, or deployment at an address. Even true and false propositions
match under this relation. Versions must match; compiler casts, binding IDs,
non-constant types, register selectors, method metadata, arities, and ordered
children remain structural. Serialization flags and unused constants are ignored.
The matcher rejects unsupported/unparsed bodies, trailing data, and unresolved
constant references. Partial overlap scores do not relax the verdict threshold.

No audit detector, mutation-corpus test, or answer key was edited.

## Decompiler fixture registration

The first full release run stopped because the decompiler corpus automatically
found the two new fixture trees and rejected unregistered corpus membership.
An actual DC_REPORT measurement showed both new trees are `byte-identical` on
decompile/recompile, with zero raw placeholders and no truncation. Only those
two entries were added to `decompile_corpus_expectations.json`; all 328 existing
entries were programmatically compared with HEAD and remained identical. No
mutation-corpus expectation or answer-key entry was changed.

## Mutation guard rail: actual run

`CARGO_TARGET_DIR=./target-match cargo test --release -p ergo-sandbox --test mutation_corpus -- --nocapture`
passed all three tests in 163.04 seconds. The unchanged harness reported:

- Attributable detections: **4/5 proven mutants (0.80)**.
- Raw drainable verdicts among proven mutants: **4**.
- Confounded detections: **none**.
- Eight seeded mutants measured; the existing answer-key assertions passed.

| Mutant | Proven | Synthesis off | Synthesis on | On probes | Attributable |
|---|---|---|---|---:|---|
| M1 | False | notunderprobes | notunderprobes | 1976 | False |
| M2 | False | notunderprobes | notunderprobes | 7152 | False |
| M3 | False | notunderprobes | notunderprobes | 7152 | False |
| M4 | True | notunderprobes | drainable | 5352 | True |
| M5 | True | notunderprobes | notunderprobes | 10368 | False |
| M6 | True | notunderprobes | drainable | 4008 | True |
| M7 | True | notunderprobes | drainable | 50000 | True |
| M8 | True | notunderprobes | drainable | 10720 | True |

## Final validation

All required commands ran with `CARGO_TARGET_DIR=./target-match` and exited 0:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --release`: **383 passed, 0 failed, 1 ignored** across 36 test-result
  summaries (including doc tests). The ignored test is the existing optional
  external-node decompiler fixture measurement. The bundled decompiler corpus,
  new identity and CLI tests, and all three mutation-corpus tests passed.

`git diff --check` passed. The isolated `target-match` directory and temporary
measurement/download files were removed after validation. This report is left
uncommitted; the implementation is committed on `feat/match` without a push or PR.
