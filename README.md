# ergo-forge

The ErgoScript playground — build, write, read, audit, and test Ergo
contracts on the same compiler and interpreter that run the Rust Ergo node
([arkadianet/ergo](https://github.com/arkadianet/ergo)). No second
interpreter, no second compiler: a verdict here is the consensus verdict.

Four ways in, one engine:

- **Build** — for people who don't write code: pick a recipe, answer its
  questions in plain terms (addresses, dates, amounts), get an address and a
  plain-language summary of what you made.
- **Write** — for developers: an editor with an ErgoScript grammar, compile-time
  parameters as a form, the decompiled round-trip of what consensus will
  run, findings underlined in your source, spendability, scenarios, and test
  suites you can run in CI.
- **Read** — for anyone: paste an address, see the contract in plain words
  and as ErgoScript, what's fragile, who can spend it, and whether a
  transaction you are about to sign would validate.
- **Play** — a sandbox chain in the browser: fund boxes under any
  contract, build transactions that spend them (secrets, data inputs,
  tokens, registers), watch the real rules accept or refuse, advance the
  height, and keep going with the boxes that came out.

Layout:

- [`ergo-sandbox/`](ergo-sandbox/) — the engine crate + `ergo-es` CLI:
  compile (with parameters and EIP-5 templates), decompile, round-trip,
  audit, spend hunt, scenario eval, test suites, transaction validation,
  cost hot-spots.
- [`ergo-web/`](ergo-web/) — the HTTP service and the playground UI
  (`ui/`, vanilla JS, nothing loaded from a CDN). Optional explorer lookups
  and per-client rate limiting for a public instance; a container image.
- [`examples/`](examples/) — 101 contracts (16 recipes, 6 protocol contracts and 7 basics written
  here, 79 real deployed contracts vendored from the node's corpus) and test
  suites.
- [`docs/workbench-PLAN.md`](docs/workbench-PLAN.md) — the plan and the
  measured record of every phase; design records under
  `docs/superpowers/specs/`.

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
cargo run -p ergo-sandbox --bin ergo-es -- validate-tx request.json   # {tx, boxes, height}
cargo run -p ergo-sandbox --features cost-trace --bin ergo-es -- eval scenario.json --hot-spots

cargo run -p ergo-web --bin ergo-web        # http://127.0.0.1:8080 — the playground
EXPLORER_URL=https://api.ergoplatform.com RATE_LIMIT_PER_MINUTE=60 \
  cargo run -p ergo-web --bin ergo-web      # with chain lookups, for a public instance
docker run --rm -p 127.0.0.1:8080:8080 ghcr.io/arkadianet/ergo-web:0.3.0
```

Without `EXPLORER_URL` the service makes no outbound calls at all. With it,
Read can fetch the real box behind an address (so the spend hunt answers for
that box), Build can turn dates into heights, and transaction validation can
fetch the boxes it needs. See [`ergo-web/README.md`](ergo-web/README.md) for
every endpoint and setting.

## What it answers

| Question | Where |
|---|---|
| I want a contract that does X, and I don't write code | **Build** mode: pick a recipe (time lock, inheritance, 2-of-3, escrow, refundable payment, savings with a spending limit, subscription, vesting with or without a cliff, token sale, NFT sale with a creator royalty, auction to the highest bidder, bounty for a secret, cross-chain swap with a hashed time lock, price gate, burn), answer its questions in plain terms (addresses, dates, amounts), get an address |
| I want to combine rules: who may spend, under what conditions | **Build → Combine rules yourself**: ways to spend (a key, any/all/k-of-n keys, anyone) with conditions grouped as when (dates, block clock, how long the funds have sat), payments (to someone, in total, kept here, a percentage kept), tokens (spender must hold one, send one, keep this box's, none leave), outside information (oracle floor/ceiling, a data input carrying a token), records (this box's registers, stamping the height, carrying a register over), secrets and attached values and the miner, and the transaction's shape (input/output counts, or a rule on any box); the tool assembles readable ErgoScript and runs generated checks. Also `ergo-es compose spec.json`, `POST /api/v1/compose` |
| My script needs a proof: a signature, a DH tuple, an AVL+ tree | A scenario's `secrets` make a real spending proof with the node's wallet prover (`proofAccepted`), and its `avl` trees come from a real AVL+ prover (`@avl.name.proof`); `ergo-es point` derives keys. Protocols: a name registry (ErgoNames shape), a mixer (ZeroJoin shape) |
| I want a whole system, not one box: an AMM, a stablecoin bank | **Write → Protocols** in the gallery: a constant-product pool with a swap order, a reserve-backed bank with an oracle; each with a property-style suite swept from an independent model (`examples/contracts/protocols/`) |
| What does this on-chain contract say? | **Read**: the contract in plain words (who may spend, under what conditions), then the source; `ergo-es decompile`, `POST /api/v1/inspect` |
| Is the code fragile? (unguarded `Option.get`, tiered by who controls the value) | `ergo-es audit`, inspect findings |
| Can someone with **no key** spend this box? | `ergo-es hunt`, `POST /api/v1/hunt`, the reader's Spendability section |
| Does my contract pass in *this* spending context, and what does it cost? | `ergo-es eval`, `POST /api/v1/eval`, the reader's Scenario panel |
| Can I work with files? | Open `.es`, `params.json`, `contract.test.json`; save a project zip the CLI runs unchanged; raw `.es` at `/api/v1/examples/{id}.es` |
| I want to try a contract's whole life: fund it, spend it, spend what came out | **Play**: a sandbox chain in the browser over `POST /api/v1/play` — every input's script runs in the transaction's context, ERG and tokens must balance, outputs get real ids; "Play with it" from Build funds a box under the contract you just made |
| Will this transaction validate, before I sign it? | `ergo-es validate-tx`, `POST /api/v1/validate-tx`, the Validate section in Read |
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

## In CI

Run your contract suites on every pull request with the composite action —
it downloads the prebuilt `ergo-es` from the matching release and posts a
table to the job summary:

```yaml
- uses: actions/checkout@v4
- uses: arkadianet/ergo-forge/.github/actions/test@main
  with:
    suites: "contracts/**/contract.test.json"   # default: **/contract.test.json
    version: latest                             # or a tag, e.g. v0.3.0; 'source' builds from an ergo-forge checkout
```

Prebuilt `ergo-es` binaries (Linux x86_64/aarch64, macOS arm64) are attached
to every release alongside the container image.

## How much to trust it

- **Compiler and reducer are the node's own.** The decompiler is graded by
  byte-exact recompilation: 270 of 279 mainnet trees in the node's corpus,
  332 of 344 trees from a live sample of recent blocks. Misses degrade to
  honest `<…>` placeholders and an audit over a partial tree says so.
- **Real contracts, not toys.** 61 of the 79 deployed contracts in the
  gallery compile with auto-filled parameters; the rest are EIP-5 templates
  with non-literal defaults or files the reference parser also rejects.
- **The spend hunt is a sample, not a proof.** A hit is a transaction anyone
  can build; a miss says "not under these probes" and names the reason
  (synthetic SELF, missing data inputs).
- **Positions are carets, not ranges.** Findings point at the start of the
  cited expression; the reader selects the whole expression by matching the
  snippet.
- **Storage rent applies to every contract.** After about four years a
  miner may take a size-based fee from any box regardless of its script, and
  a box holding less than the fee is swept, tokens included. The inspect,
  compile and hunt answers say so with the estimated fee; a "burn" address
  is not an exception.
- **Transaction validation checks scripts and balances, not signatures.** An
  input that reduces to a sigma proposition is reported as a signature
  needed.

CI enforces the verification bars on every PR, including the whole-corpus
round-trip floors against the pinned node checkout. A `v*` tag publishes the
container image to GHCR and attaches prebuilt `ergo-es` binaries to the
release.

## Status

Phases P0–P4f are done (recon, engine, decompiler, public AST, audit lints,
spend hunt, cost hot-spots, HTTP service and UI, playground with parameters
and templates, test suites, chain lookups, Build mode, transaction
validation); the node-side P5-A/P5-B (source positions and the source map)
landed and are consumed. See the plan for the measured record of each.
Remaining: register fuzzing for the hunt, cross-branch reasoning in the
lint, ranges instead of carets, a wallet step in Build.

Engine crates are consumed from `arkadianet/ergo` via pinned git
revisions (`Cargo.toml`) — bump deliberately, the node is the oracle.
