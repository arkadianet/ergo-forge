# ergo-forge

ErgoScript workbench — author, test, and read Ergo contracts on the same
interpreter and compiler that run the Rust Ergo node
([arkadianet/ergo](https://github.com/arkadianet/ergo)). No second
interpreter, no second compiler: a verdict here is the consensus verdict.

- [`ergo-sandbox/`](ergo-sandbox/) — the engine crate + `ergo-es` CLI
  (scenario evaluation, cost reporting, compile, decompile, round-trip, audit)
- [`ergo-web/`](ergo-web/) — HTTP service + one-page reader: paste an address
  or ErgoTree hex, get readable ErgoScript plus audit findings
- [`docs/workbench-PLAN.md`](docs/workbench-PLAN.md) — the plan: audiences,
  phases, verification bar (decompiler v1 = byte-exact recompilation)

## Quick start

```text
cargo run -p ergo-sandbox --bin ergo-es -- compile contract.es
cargo run -p ergo-sandbox --bin ergo-es -- eval scenario.json
cargo run -p ergo-sandbox --bin ergo-es -- decompile 100104c801d191a37300
# decompile → recompile → byte-compare (add -v for every failure reason)
cargo run -p ergo-sandbox --bin ergo-es -- roundtrip 100104c801d191a37300
cargo run -p ergo-sandbox --features cost-trace --bin ergo-es -- eval scenario.json
```

Example scenario (`sigmaProp(HEIGHT > 100)` failing at height 99):

```json
{ "source": "sigmaProp(HEIGHT > 100)", "height": 99 }
```

See [`ergo-sandbox/README.md`](ergo-sandbox/README.md) for the full scenario
schema, verdicts, and the Rust API.

## Status

Done: P0 (decompiler recon), P1 (sandbox engine + CLI), P2 (decompiler v1),
P2.5 (public lifted AST), P3a (static audit lints), P4a (HTTP service + reader
UI). Next in the plan: P3b (scenario fuzz — "spendable by anyone?"), more
lints, P3c (cost hot-spots). CI enforces the verification bars on every PR,
including the whole-corpus round-trip floors against the pinned node checkout.

Engine crates are consumed from `arkadianet/ergo` via pinned git
revisions (`Cargo.toml`) — bump deliberately, the node is the oracle.
