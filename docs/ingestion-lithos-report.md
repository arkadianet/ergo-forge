# Lithos ingestion measurement

Measured 2026-09-09 on the unchanged local corpus at
`.lithos-src/lithos-lib/src/main/resources`, revision
`e7a9994b9560e0c6d057e7cc0a59c0cb796e4d44`. The compiler remains pinned to
`9468043396e5daa2828211bcff4234bc70fae4f0`.

The 28 `.ergo` files contain 4948 lines. [Machine-readable evidence](ingestion-lithos-results.json)
contains per-file SHA-256 hashes, all baseline/prototype/final rows, complete
errors, inferred values and types, binding provenance and source lines.

| Ingestion path | Compiled | Not compiled | Rate |
|---|---:|---:|---:|
| No inferred parameters (reproduced) | 3/28 | 25 | 10.7% |
| Prototype name heuristic (reproduced, compilation only) | 18/28 | 10 | 64.3% |
| Library AST inference, no overrides | 26/28 | 2 | 92.9% |

Every final compiled result also parses and lifts with zero raw nodes and no
truncation. No source edits or protocol-specific rules were used. The final
path also handles free constants whose names do not start with `CONST_`.

## All contracts

`Missing` means an unbound parameter on the no-inference path; `type error`
is the prototype binding the wrong type. `v0 gate` is the serialization failure
described below. Full errors are in the JSON evidence.

| Contract | No inference | Prototype | AST inference |
|---|---|---|---|
| `collateral/Collateral_Enforcer.ergo` | Missing | Compiled | Compiled |
| `collateral/Collateral_Mainnet.ergo` | Missing | Compiled | Compiled |
| `collateral/Collateral_Testnet.ergo` | Missing | Missing | Compiled |
| `collateral/Emission_Config.ergo` | Missing | type error | Compiled |
| `collateral/Emission_Gate.ergo` | Missing | Compiled | Compiled |
| `collateral/Emission_Guard.ergo` | Missing | Compiled | Compiled |
| `collateral/LIT_Emissions.ergo` | Missing | type error | Compiled |
| `dictionaries/MinerData_Guard.ergo` | Missing | Compiled | Compiled |
| `dictionaries/MinerData_Logic.ergo` | Missing | Compiled | Compiled |
| `dictionaries/MinerDictionary.ergo` | Missing | type error | Compiled |
| `fraudproofs/FP_IncorrectN.ergo` | Compiled | Compiled | Compiled |
| `fraudproofs/FP_InvalidDiff.ergo` | v0 gate | v0 gate | v0 gate |
| `fraudproofs/FP_InvalidFormat.ergo` | Missing | Compiled | Compiled |
| `fraudproofs/FP_MalformedGE.ergo` | v0 gate | v0 gate | v0 gate |
| `fraudproofs/FP_MalformedGenesis.ergo` | Missing | Compiled | Compiled |
| `fraudproofs/FP_NonMatchingCommitment.ergo` | Missing | type error | Compiled |
| `fraudproofs/FP_NonUniqueHeaders.ergo` | Compiled | Compiled | Compiled |
| `fraudproofs/FP_NotInWindow.ergo` | Missing | Compiled | Compiled |
| `fraudproofs/FP_TransactionNotIncluded.ergo` | Compiled | Compiled | Compiled |
| `lithosdex/LD_FeeVault.ergo` | Missing | Compiled | Compiled |
| `lithosdex/LD_LiquidityPool.ergo` | Missing | Compiled | Compiled |
| `lithosdex/LD_Provision.ergo` | Missing | Compiled | Compiled |
| `lithosdex/LD_Provision_Guard.ergo` | Missing | Compiled | Compiled |
| `rollups/Evaluation.ergo` | Missing | type error | Compiled |
| `rollups/FP_Control_Testnet.ergo` | Missing | type error | Compiled |
| `rollups/Holding_Guard.ergo` | Missing | Compiled | Compiled |
| `rollups/Holding_Logic.ergo` | Missing | type error | Compiled |
| `rollups/Payout.ergo` | Missing | Compiled | Compiled |

## Remaining failures

| Contract | Cause |
|---|---|
| `fraudproofs/FP_InvalidDiff.ergo` | `tree not serializable under a v0 header: UnsignedBigInt constant data` |
| `fraudproofs/FP_MalformedGE.ergo` | `tree not serializable under a v0 header: UnsignedBigInt constant data` |

Both fail even with `tree_version=3`. The argument selects language/emission
semantics, while the pinned compiler uses a fixed v0 serialization header.
The public `compile` and `compile_with_source_map` entry points both enter that
pipeline; `build_tree`/graph assembly are private. The existing wrapper stamps
a newer header only after compilation has already serialized successfully, so
it cannot repair these errors. This change preserves the diagnostic and explains
the limitation instead of reimplementing compiler assembly or changing source.

## Rules and assumptions

The [ingestion documentation](ingestion.md#inference-rules) lists the rules:
scoped aliases, declared types, tuple fields, comparisons, numeric arithmetic,
Boolean/SigmaProp operators, collection indexing, and the compiler’s own
versioned method/predef signatures. The seven small source fixtures exercise
these rules independently of Lithos. Additional tests cover overrides, ambiguous
and unsupported types, conflicts, binder scope collisions, serialization errors,
mixed batch failures, traversal errors and CLI behavior.

All inferred bindings use synthetic values, not the Scala deployment values.
Numeric widths choose the widest observed context. Byte collections have a
synthetic 32-byte length; Long collections contain four values; SigmaProp
placeholders are single ProveDlog propositions. Numeric values and points are
synthetic too. Exact values are in the JSON report. Compiler folding can change
the resulting tree, including removal of branches. Successful ingestion proves
compilation/parsing/lifting of that reported instantiation, not deployment
equivalence or runtime validity. No analysis was run to obtain these numbers.

The prototype measurement recreated only the supplied `infer_params` logic
and passed its environment through the existing compile command. Its analysis
calls were not used. The baseline uses `ingest --no-infer`; the final uses
`ingest` with default options and an empty override map.

## Reproduction

```sh
CARGO_TARGET_DIR=./target-ing cargo build --release -p ergo-sandbox --bin ergo-es
./target-ing/release/ergo-es ingest .lithos-src/lithos-lib/src/main/resources --no-infer --json > baseline.json
./target-ing/release/ergo-es ingest .lithos-src/lithos-lib/src/main/resources --json > after.json
```

Both report commands exit 1 because the corpus has failures; their JSON reports
remain complete. The corpus and throwaway prototype remain untracked inputs.
The project’s own example corpus is not used for this acceptance measurement.

## Verification

All required checks passed with `CARGO_TARGET_DIR=./target-ing`:

- `cargo test --release`
- `cargo clippy --workspace --all-targets --release -- -D warnings`
- `cargo fmt --all -- --check`

The worktree remains on `feat/ingest-named-constants`, based on merged main
`e1e7218`. No commit or push was made.
