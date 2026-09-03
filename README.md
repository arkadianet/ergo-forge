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
cargo run -p ergo-sandbox --bin ergo-es -- audit 1001040ad191e4c6a704047300
cargo run -p ergo-sandbox --bin ergo-es -- hunt 1001040ad191e4c6a704047300
cargo run -p ergo-sandbox --bin ergo-es -- test examples/tests/height-lock.test.json
cargo run -p ergo-sandbox --features cost-trace --bin ergo-es -- eval scenario.json --hot-spots

cargo run -p ergo-web --bin ergo-web        # http://127.0.0.1:8080 — the reader
docker build -t ergo-web . && docker run --rm -p 127.0.0.1:8080:8080 ergo-web
```

## What it answers

| Question | Where |
|---|---|
| I want a contract that does X, and I don't write code | **Build** mode: pick a recipe (time lock, inheritance, 2-of-3, refundable payment, price gate, burn), answer its questions in plain terms (addresses, dates, amounts), get an address |
| What does this on-chain contract say? | `ergo-es decompile`, `POST /api/v1/inspect`, the reader |
| Is the code fragile? (unguarded `Option.get`, tiered by who controls the value) | `ergo-es audit`, inspect findings |
| Can someone with **no key** spend this box? | `ergo-es hunt`, `POST /api/v1/hunt`, the reader's Spendability section |
| Does my contract pass in *this* spending context, and what does it cost? | `ergo-es eval`, `POST /api/v1/eval`, the reader's Scenario panel |
| Can I work with files? | Open `.es`, `params.json`, `contract.test.json`; save a project zip the CLI runs unchanged; raw `.es` at `/api/v1/examples/{id}.es` |
| Do all my contract's paths still behave after a change? | `ergo-es test contract.test.json` (CI), `POST /api/v1/test`, the Tests panel |
| Where does the cost go? | `ergo-es eval --hot-spots` (cost-trace build) |

Every answer comes from the node's own compiler and reducer. Verification
bars are measured on real corpora and pinned in CI (byte-exact round-trip
floors, lint flag rates, the hunt tally, the stack budget).

Example scenario (`sigmaProp(HEIGHT > 100)` failing at height 99):

```json
{ "source": "sigmaProp(HEIGHT > 100)", "height": 99 }
```

See [`ergo-sandbox/README.md`](ergo-sandbox/README.md) for the full scenario
schema, verdicts, and the Rust API.

## Status

Done: P0 (decompiler recon), P1 (sandbox engine + CLI), P2 (decompiler v1),
P2.5 (public lifted AST), P3a (static audit lints, severity-tiered, val/`||`
guards followed), P3b (spend hunt with self-box and data-input probes), P3c
(cost hot-spots), P4a (HTTP service: inspect / hunt / eval, reader UI,
container image). Remaining: register fuzzing for the hunt, cross-branch
reasoning in the lint, and P5-B (node-side source positions) so findings and
cost can cite source spans. CI enforces the verification bars on every PR,
including the whole-corpus round-trip floors against the pinned node checkout;
a `v*` tag publishes the image to GHCR.

Engine crates are consumed from `arkadianet/ergo` via pinned git
revisions (`Cargo.toml`) — bump deliberately, the node is the oracle.
