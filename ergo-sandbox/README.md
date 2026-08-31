# ergo-sandbox

ErgoScript workbench engine: scenario-driven evaluation, cost reporting,
compile, and structural decompile — on the **consensus interpreter**
(`ergo-sigma`) and the **oracle-parity compiler** (`ergo-compiler`). No second
interpreter, no second compiler.

Not a consensus surface: a bug here produces a wrong answer in a playground,
never a fork.

## API sketch

```rust
use ergo_sandbox::{eval_scenario, Scenario, Verdict};

let sc: Scenario = serde_json::from_str(r#"{
    "source": "sigmaProp(HEIGHT > 100)",
    "height": 200
}"#).unwrap();
let outcome = eval_scenario(&sc).unwrap();
assert_eq!(outcome.verdict, Verdict::Pass);
```

- `scenario` — JSON model + typed-value parser
- `eval` — scenario → `ReductionContext` (same shape block validation
  assembles) → bounded-cost reduce → `EvalOutcome { verdict, cost, trace }`
- `compile` — thin wrapper over `ergo_compiler::compile`
- `inspect` — structural view of ErgoTree bytes (P0 spike, productized)

## CLI

```text
cargo run -p ergo-sandbox --bin ergo-es -- compile src/timelock.es
cargo run -p ergo-sandbox --bin ergo-es -- eval scenario.json
cargo run -p ergo-sandbox --bin ergo-es -- decompile 100104c801d191a37300
cargo run -p ergo-sandbox --features cost-trace --bin ergo-es -- eval scenario.json
```

## Scenario schema (v1)

```jsonc
{
  "tree": "<ergo-tree hex>",          // OR "source": ErgoScript (exclusive)
  "source": "sigmaProp(...)",
  "treeVersion": 0,                   // for source; 0 (v5) or 3 (v6)
  "network": "mainnet",               // address encoding; "testnet" too
  "height": 900000,                   // CONTEXT.HEIGHT (required)
  "selfBox": {                        // defaults: value 0, tree = evaluated tree
    "value": 1000000,
    "ergoTree": "<hex>",              // optional on the self box
    "creationHeight": 800000,
    "registers": { "R4": { "type": "Int", "value": 7 } },
    "tokens": [ { "id": "<32-byte hex>", "amount": 10 } ],
    "boxId": "<32-byte hex>"
  },
  "inputs": [], "outputs": [], "dataInputs": [],
  "contextVars": { "5": { "type": "Long", "value": 42 } },
  "minerPubkey": "<33-byte hex>",
  "preHeader": { "timestamp": 0, "version": 0, "parentId": "<hex>",
                 "nBits": 0, "votes": [0,0,0] },
  "costLimit": 8001091,
  "activatedScriptVersion": 3,
  "proof": "<hex>",                   // optional: verify through the full path
  "message": "<hex>"                  // bytes the proof commits to
}
```

Value types: `Boolean`, `Byte`, `Short`, `Int`, `Long`, `BigInt` (decimal
string), `GroupElement` (33-byte hex), `SigmaProp` (`true`/`false`/pubkey hex),
`Coll[T]` for any of those (`Coll[Byte]` also accepts a hex string).

Verdicts: `PASS` (trivially true), `FAIL` (evaluated to false), `ERROR`
(runtime exception / budget exhausted — see `error`), `NEEDS-PROOF`
(reduced to a sigma proposition — sign it with a real wallet), `PROOF-ACCEPTED`,
`PROOF-REJECTED`.

Registers must be dense from R4 (an `EvalBox` invariant). Box ids and
transaction ids default to zero — synthetic boxes only; nothing here touches
chain state.
