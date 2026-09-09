# MAP report

Branch: `feat/map`. Implementation commit: `4e77a5d`. All Cargo commands used `CARGO_TARGET_DIR=./target-map`.

## Implementation

Added `audit_with_contracts(&Lifted, &ContractSet)` alongside the unchanged local
`audit()`. Every original finding remains, including its severity and message;
context adds an active/discharged status and serializable evidence. Discharges
name the companion contract and its execution location, literal transaction slot,
binding kind and identity, covered dimension, AST/IR binding anchors, and (for
NFTs) singleton issuance evidence.

The set explicitly declares spending inputs and required context extensions in
one transaction. A discovered protocol map alone is not evidence of co-execution.
Names can be map box IDs or source paths. Missing/partial contexts, incomplete
lifts, duplicate names or execution locations, missing input positions, orphaned
extensions, unrecognized bindings, and per-input context-variable indices cannot
produce discharges. NFT discharges require explicit singleton issuance evidence.
Transaction membership, extension resolution and issuance evidence are caller
premises; these results are conditional on the declared transaction set, not
universal claims about every spend of these protocols.

The map module's new `required_bindings` reader is stricter than its existing
syntactic reference inventory: it follows aliases, unions conjunctions and
intersects alternatives. Optional, unused, negated or unsupported checks supply
no proof. It matches only the exact literal slot; it never equates a companion's
SELF with an unbound input. Existing local detectors and mutation answer keys
were not changed. Only reserve-identity and data-provider-provenance findings
can be discharged; other findings retain active status.

Details: `docs/audit-context.md`.

## Measured before/after sweep

Both runs used:

```sh
CARGO_TARGET_DIR=./target-map cargo run --release -p ergo-sandbox --example audit_sweep
```

The parsed JSON documents are identical, including all per-contract findings,
parameters, source/tree hashes and compilation errors. The sweep has no
transaction-set evidence and correctly makes no contextual discharges.

| Measurement | Before | After |
|---|---:|---:|
| Contracts | 108 | 108 |
| Complete | 98 | 98 |
| Failed compilation/ingestion | 10 | 10 |
| Findings | 526 | 526 |
| HIGH | 379 | 379 |
| MED | 49 | 49 |
| LOW | 98 | 98 |

| Lint | Before | After |
|---|---:|---:|
| delegated-reserves | 15 | 15 |
| height-guards | 6 | 6 |
| trust-assumptions | 4 | 4 |
| unbound-box-reserves | 74 | 74 |
| unchecked-get | 427 | 427 |

## Measured contextual regression cases

- Phoenix HodlToken proxy/bank: **8 original proxy findings retained; 1 discharged**.
  The discharged reserve finding is `OUTPUTS(0)`, with evidence naming
  `phoenix_v1_hodltoken_bank.es`, input 0, binding `self-successor`, and
  `OUTPUTS(0).propositionBytes == SELF.propositionBytes`. The remaining findings
  stay active. This applies when the bank is declared as an executing companion.
- Lithos: **2 config-provenance findings discharged**, one each from
  `Collateral_Enforcer.ergo` and `LIT_Emissions.ergo`. Both name
  `Emission_Guard.ergo` and its required
  `CONST_EMCONFIG_NFT == CONTEXT.dataInputs(0).tokens(0)._1` check. These are
  `trust-assumptions` findings, not reserve-math findings. Extensions are explicitly
  declared at input 0, context variables 65 and 64 respectively. Singleton
  evidence in these structural tests is a labeled synthetic test premise.
- Dexy LP swap/main: **the unbound `INPUTS(0)` finding remains active HIGH** in
  both the intended order and a decoy-first order where the actual pool is at
  input 2. The pool's successor checks and action-NFT alternatives do not bind
  that input. Original findings remain intact.

Eight context tests additionally cover optional/negated/unused/exists checks,
wrong slots and collections, incomplete or mismatched contexts, missing singleton
proof, context-variable indices, code identity versus data provenance, deterministic
serialization, and preservation of every original finding.

## Lithos prerequisite and reproducibility

Lithos is absent from `examples/contracts` in this checkout. The three unmodified
CC0 fixtures are pinned to `Lithos-Protocol/Lithos-Client` revision
`e7a9994b9560e0c6d057e7cc0a59c0cb796e4d44` and their hashes were checked against
`docs/ingestion-lithos-results.json`. License and provenance accompany the fixtures.

Actual initial ingestion here produced 2 raw placeholders each for the enforcer
and guard (the emissions script had 0), despite the older ingestion report's
zero-raw claim. The missing lifted forms were typed `CONTEXT.getVarFromInput[T]`
and `executeFromVar[T]`. Added faithful rendering of these operations, preserving
explicit types and variable IDs. Compile/lift/recompile tests verify exact tree
round trips for Byte, GroupElement, SigmaProp, Int and Boolean cases. The old
raw-placeholder expectation for DeserializeContext moved to this supported-form
coverage; other unsupported-operation assertions remain. Complete lifts are still
required for contextual discharge.

## Verification

All required commands passed on the final Rust changes:

```sh
CARGO_TARGET_DIR=./target-map cargo fmt --all -- --check
CARGO_TARGET_DIR=./target-map cargo clippy --workspace --all-targets -- -D warnings
CARGO_TARGET_DIR=./target-map cargo test --release
```

The full release suite reports **383 passed, 0 failed, 1 ignored** across 35 test/doc-test groups.
All three mutation-corpus tests passed, including
`the_corpus_is_measured_and_does_not_regress`. The mutation harness, seeded
mutants, and answer key are unchanged. `git diff --check` also passed.

The isolated `target-map` directory and temporary measurement logs were removed
after verification. This report is intentionally not committed. No push or PR.
