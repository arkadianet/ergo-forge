# ergo-forge

ErgoScript workbench — author, test, and read Ergo contracts on the same
interpreter and compiler that run the Rust Ergo node
([arkadianet/ergo](https://github.com/arkadianet/ergo)). No second
interpreter, no second compiler: a verdict here is the consensus verdict.

- [`ergo-sandbox/`](ergo-sandbox/) — the engine crate + `ergo-es` CLI
  (scenario evaluation, cost reporting, compile, structural decompile)
- [`docs/workbench-PLAN.md`](docs/workbench-PLAN.md) — the plan: audiences,
  phases, verification bar (decompiler v1 = byte-exact recompilation)

## Quick start

```text
cargo run -p ergo-sandbox --bin ergo-es -- compile contract.es
cargo run -p ergo-sandbox --bin ergo-es -- eval scenario.json
cargo run -p ergo-sandbox --bin ergo-es -- decompile 100104c801d191a37300
cargo run -p ergo-sandbox --features cost-trace --bin ergo-es -- eval scenario.json
```

Example scenario (`sigmaProp(HEIGHT > 100)` failing at height 99):

```json
{ "source": "sigmaProp(HEIGHT > 100)", "height": 99 }
```

See [`ergo-sandbox/README.md`](ergo-sandbox/README.md) for the full scenario
schema, verdicts, and the Rust API.

## Status

P0 (decompiler recon) and P1 (sandbox engine + CLI) done; see the plan for
P2 (decompiler v1), P3 (audit layer), and the browser shell.

Engine crates are consumed from `arkadianet/ergo` via pinned git
revisions (`Cargo.toml`) — bump deliberately, the node is the oracle.
