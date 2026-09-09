# ergo-forge

ergo-forge is an ErgoScript workbench for building contracts and inspecting deployed code through reproducible experiments on the Rust Ergo node's pinned compiler and execution engine. Its authoring and audit views share contracts, scenarios, and evidence; every result states whether it is a static observation, a synthetic experiment, or a transaction checked against an explicitly supplied state. It helps people check specific contract claims and investigate counterexamples. It is not a general security certification service, an autonomous vulnerability scanner, a wallet, or a protocol-design generator. New work must improve the fidelity, reproducibility, or interpretation of those experiments; more recipes, detectors, graph features, and search breadth do not qualify by themselves.

Four ways in, one engine:

- **Build** — for people who don't write code: pick a recipe, answer its
  questions in plain terms (addresses, dates, amounts), get an address and a
  plain-language summary of what you made.
- **Write** — for developers: an editor with an ErgoScript grammar, compile-time
  parameters as a form, the decompiled round-trip of what consensus will
  run, findings underlined in your source, spendability, scenarios, and test
  suites you can run in CI.
- **Read** — for anyone: paste an address, see the contract in plain words
  and as ErgoScript, static observations, sampled spending scenarios, and unsigned preflight
  checks of selected transaction conditions; full node validation has not run.
- **Play** — a sandbox chain in the browser: fund boxes under any
  contract, build transactions that spend them (secrets, data inputs,
  tokens, registers), watch scenario reduction and balance checks accept or refuse, advance the
  height, and keep going with the boxes that came out.

Layout:

- [`ergo-sandbox/`](ergo-sandbox/) — the engine crate + `ergo-es` CLI:
  compile (with parameters and EIP-5 templates), decompile, round-trip,
  audit, spend hunt, scenario eval, test suites, unsigned transaction preflight,
  cost hot-spots.
- [`ergo-web/`](ergo-web/) — the HTTP service and the playground UI
  (`ui/`, vanilla JS, nothing loaded from a CDN). Optional explorer lookups
  and per-client rate limiting for a public instance; a container image.
- [`examples/`](examples/) — 101 contracts (16 recipes, 6 protocol contracts and 7 basics written
  here, 79 real deployed contracts vendored from the node's corpus) and test
  suites.
- [`docs/workbench-PLAN.md`](docs/workbench-PLAN.md) — the historical plan and the
  measured record of every phase; design records under
  `docs/superpowers/specs/`.

New to Ergo? Build mode has three-minute walkthroughs that drive the
real tool: lock savings and spend them in Play, a shared account signed by
two of three, and a spend gated by an oracle box.

## Quick start

```text
cargo run -p ergo-sandbox --bin ergo-es -- compile contract.es
cargo run -p ergo-sandbox --bin ergo-es -- eval scenario.json
cargo run -p ergo-sandbox --bin ergo-es -- decompile 100104c801d191a37300
# decompile → recompile → byte-compare (add -v for every failure reason)
cargo run -p ergo-sandbox --bin ergo-es -- roundtrip 100104c801d191a37300
cargo run -p ergo-sandbox --bin ergo-es -- audit 1001040ad191e4c6a704047300
cargo run -p ergo-sandbox --bin ergo-es -- hunt 1001040ad191e4c6a704047300
cargo run -p ergo-sandbox --bin ergo-es -- drain request.json          # {inputs:[{role,box…}], outputs:[{payee,box…}], protocolNfts, height, objective:{terms:[]}}
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
that box), Build can turn dates into heights, and unsigned preflight can
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
| Can a keyless transaction **drain this contract set** over the shapes an attacker can build? | `ergo-es drain request.json` — labelled roles (`protected`/`companion`/`attacker`/`external`), a fixed shape, bounded enumeration of input permutations and a generic decoy family; the incident replay is the acceptance test (`examples/incidents/`) |
| Does my contract pass in *this* spending context, and what does it cost? | `ergo-es eval`, `POST /api/v1/eval`, the reader's Scenario panel |
| Can I work with files? | Open `.es`, `params.json`, `contract.test.json`; save a project zip the CLI runs unchanged; raw `.es` at `/api/v1/examples/{id}.es` |
| I want to try a contract's whole life: fund it, spend it, spend what came out | **Play**: a sandbox chain in the browser over `POST /api/v1/play` — every input's script runs in the transaction's context, ERG and tokens must balance, outputs get deterministic simulation IDs; scenario proofs use a supplied/default message; "Play with it" from Build funds a box under the contract you just made |
| Does this unsigned transaction pass selected preflight checks? | `ergo-es validate-tx`, `POST /api/v1/validate-tx`, the Validate section in Read |
| Do all my contract's paths still behave after a change? | `ergo-es test contract.test.json` (CI), `POST /api/v1/test`, the Tests panel |
| Where does the cost go? | `ergo-es eval --hot-spots` (cost-trace build) |

Compilation and scenario reduction use the pinned node engine. Static lints,
recognition and structural matching do not consult the reducer. Preflight adds
selected balance checks; it does not establish full node acceptance or future
inclusion. Current result envelopes disclose their method, provenance limits,
and `nodeValidated: false`. Static severity is review priority.

The [governing roadmap](docs/ROADMAP.md) and [record-only scoreboard](docs/roadmap-metrics.json)
separate recovery coverage, sampled preflight detection, and node-validated claims.

Example scenario (`sigmaProp(HEIGHT > 100)` failing at height 99):

```json
{ "source": "sigmaProp(HEIGHT > 100)", "height": 99 }
```

See [`ergo-sandbox/README.md`](ergo-sandbox/README.md) for the full scenario
schema, verdicts, and the Rust API.

For third-party sources with free named constants, `ergo-es ingest <directory>`
compiles and lifts `.ergo`/`.es` files using reported synthetic bindings, with
type/value overrides and a row for every failure. See [static source ingestion](docs/ingestion.md)
and the [Lithos before/after measurement](docs/ingestion-lithos-report.md).

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
  byte-exact recompilation. Historical node-corpus measurements were 270 of
  279 mainnet trees and 332 of 344 trees from a sampled set of blocks. The
  current committed fixture inventory records 238/329 exact obtained trees
  (339 rows including 10 initial compile failures); see the
  [versioned scoreboard](docs/roadmap-metrics.json). Misses degrade to
  honest `<…>` placeholders and an audit over a partial tree says so.
- **Real contracts, not toys.** 61 of the 79 deployed contracts in the
  gallery compile with auto-filled parameters; the rest are EIP-5 templates
  with non-literal defaults or files the reference parser also rejects.
- **The spend hunt is a sample, not a proof.** A hit is a passing sampled scenario; canonical transaction
  validation has not run; a miss says "not under these probes" and names the reason
  (synthetic SELF, missing data inputs).
- **Positions are carets, not ranges.** Findings point at the start of the
  cited expression; the reader selects the whole expression by matching the
  snippet.
- **Storage rent applies to every contract.** After about four years a
  miner may take a size-based fee from any box regardless of its script, and
  a box holding less than the fee is swept, tokens included. The inspect,
  compile and hunt answers say so with the estimated fee; a "burn" address
  is not an exception.
- **Unsigned preflight checks selected script and balance conditions.** An
  input that reduces to a sigma proposition is reported as a signature needed.
  Signatures and the full transaction pipeline are not validated. `preflightPassed`
  is the result; `valid` remains its deprecated alias and never means node acceptance.

CI enforces the verification bars on every PR, including the whole-corpus
round-trip floors against the pinned node checkout. A `v*` tag publishes the
container image to GHCR and attaches prebuilt `ergo-es` binaries to the
release.

## Status

The only active build queue is [docs/ROADMAP.md](docs/ROADMAP.md). Earlier phase
records are historical; their remaining-work lists authorize no new work.

Engine crates are consumed from `arkadianet/ergo` via pinned git
revisions (`Cargo.toml`) — bump deliberately, the node is the oracle.

Drain requests require an explicit authorization policy; see [drain accounting](docs/drain-accounting.md) for the formula, key declarations and release terms.

### Ingest an address or deployed box

```sh
ergo-es tree 4MQyML64GnzMxZgm --json
ergo-es tree 10010101d17300
ergo-es tree <boxId> --explorer https://api.ergoplatform.com --json
ergo-es tree <boxId> --source recorded-chain.json --json
```

The library entry point is `ergo_sandbox::tree::ingest_tree(input, network,
source)`, accepting an optional `&dyn ChainSource` (including `RecordingSource`).
P2S, P2PK and P2SH addresses decode offline with checksum and prefix validation.
Address networks are detected automatically; `--network mainnet|testnet` enforces
an expected network. Trees and box IDs carry no network: their address forms use
mainnet unless `--network testnet` is supplied. Select an explorer for that network.

Box lookup requires explicit `--explorer URL` or `EXPLORER_URL`, using the same
`ExplorerSource` as `map`, and the `explorer` build feature. `--source` replays a
recorded `Fixture` without network support. Address/tree inputs never query a
source, even with explorer configuration present. Exactly 64 hex characters mean
a box ID; prefix a 32-byte tree with `tree:` to disambiguate. Explicit `address:`
and `box:` prefixes are also accepted.

JSON format version 1 has fixed fields `formatVersion`, `treeHex`, `network`,
`addresses` (`p2s`, `p2pk`, `p2sh`), `boxData`, `source`, `audit`, and `notes`.
Unavailable address forms and absent box data are `null`. Address forms decode to
the exact reported tree; `p2sh` is present only for a standard P2SH wrapper.
`boxData` preserves `boxId`, `ergoTree`, `value` (nanoERG), ordered `tokens`
(`id`, `amount`), raw `additionalRegisters`, `creationHeight`, and `inclusionHeight`.
`source` is decompiled text. `audit` contains the existing lint findings plus
`complete`, `rawPlaceholders`, and `truncated`; a partial lift is never reported
as a complete audit. For P2SH, only the hash-check spending wrapper is recoverable;
the underlying script preimage is unavailable and is **not audited**.

Feed the exact tree into a scenario or the existing audit command:

```sh
ergo-es tree 4MQyML64GnzMxZgm --json | jq '{tree: .treeHex, height: 100}' | ergo-es eval - --json
ergo-es audit "$(ergo-es tree 4MQyML64GnzMxZgm --json | jq -r .treeHex)"
```
### Contract identity under constant substitution

`ergo-es match` compares compiled source with a deployed tree, or two tree hex
strings, using the library's `ergo_sandbox::identity::match_trees` API:

```sh
# params.json uses the existing name -> {type, value} compile parameter format.
CARGO_TARGET_DIR=./target-match cargo run --release -p ergo-sandbox --bin ergo-es -- \
  match examples/contracts/dexy/lp/pool/swap.es "$DEPLOYED_TREE_HEX" \
  --params params.json --json
# Or, with the binary installed:
ergo-es match "$SOURCE_TREE_HEX" "$DEPLOYED_TREE_HEX"
```

For the bundled `examples/incidents/use-lp-drain.deployed-swap.test.json` tree,
compile the swap source with:

```json
{
  "feeNumLp": {"type": "Long", "value": 3},
  "feeDenomLp": {"type": "Int", "value": 1000}
}
```

The verdict is **same program with differing constants**. Parameter types matter:
using `Int` for the numerator causes the compiler to insert two extra casts and
produces **different program** under this conservative structural comparison.
Casts are retained, even when their operand is a constant. Source inputs (`.es`
or `.ergo`) compile with the existing parameter compiler, mainnet address parsing,
and requested tree version 3 (ordinary contracts retain version 0). Missing
parameters are errors; values are never guessed. Both tree inputs are hexadecimal
ErgoTree bytes, not addresses. The command does not fetch chain data.

The three verdicts (JSON: `same_program`,
`same_program_with_differing_constants`, `different_program`) describe parsed
program structure and resolved constant occurrences. `byte_identical` separately
reports exact serialization equality. A successful comparison exits 0 even for
`different_program`; malformed input, unparsed/unsupported trees, unresolved
constant references, and compilation errors exit nonzero without a verdict.

The matcher compares the full parsed opcode IR before decompiler lifting. It
preserves operation tags, ordered children and arities, binding IDs, register and
context-variable selectors, method IDs and type arguments, numeric casts, optional
children, and other non-constant metadata. Inline constants and constant-table
references become untyped holes; references resolve independently in each tree,
so table reordering and sharing do not affect identity. Boolean literal opcodes
and packed Boolean collections are also constant leaves. A collection stored as
one constant is one hole, regardless of its length. Other compiler rewrites and
binding renumbering are deliberately not normalized. Tree versions must match;
size/segregation flags and unused constant-table entries are ignored.

`constant_differences` lists every differing occurrence by ordered child `path`
(`[]` is the root, `[0]` its first child), with each side's `table_index` when
present, type, and full IR value representation. Reused table entries can appear
at multiple paths. A null side denotes an absent node or an operation at that
path. For different programs, these are positional observations, not a proposed
substitution map. Values and types are compared as parsed data, never as strings.

Similarity is the number of equal node labels at identical child paths divided
by the union of paths, including one additional label for tree version. Hole
labels ignore values and types; operation labels retain their metadata and child
layout. The report includes the numerator, denominator, and definition. Only a
score of 1 establishes structural identity under this definition. Partial scores
are structural overlap, not confidence percentages or evidence of equivalent
behaviour.

**Matching structure while constants differ does not prove behavioural
equivalence.** A constant can change which branch is reachable, which key is
authorized, or which asset is accepted. Even `sigmaProp(true)` and
`sigmaProp(false)` match under constant substitution. This tool neither certifies
safety nor establishes source provenance or that a supplied tree is deployed at
an address. Review all constant differences and independently obtain the deployed
tree. Existing audit detectors and mutation-corpus expectations are unchanged.

Offline regression fixtures cover the Dexy swap, the HodlCOMET10 bank and burn
proxy, and near-miss Phoenix hodlcoin siblings. The Phoenix trees and explorer
transaction/box provenance are pinned in
[`identity_phoenix.json`](ergo-sandbox/tests/fixtures/identity_phoenix.json).
Library callers compile sources through `compile::compile_with_params`, then pass
`CompileOutput::tree_bytes` and deployed bytes to `identity::match_trees`. As with
other deep-tree parser consumers, use `decompile::with_large_stack` on hosts with
small thread stacks.

### Authority rules

Use the exact pinned compiler/reducer; no second acceptance engine. A property
claim requires full pinned node validation **and** a versioned violated property;
today there are zero such producers. Premises must survive export before any
promotion; unsupported semantics fail closed for promotion. A miss never means
safe, truncation stays visible, and third-party audit material stays private.

Source inference uses synthetic constants ([ingestion](docs/ingestion.md)).
A complete lift measures recovery coverage, not audit completeness. Conditional
[contract-set discharge](docs/audit-context.md) depends on the supplied co-execution.
`same_program_with_differing_constants` is [structural matching](ergo-sandbox/src/identity.rs#L18),
not behavioral equivalence or deployment identity.

Triage records use `formatVersion: 2`: `reproduced-in-scenario` means the sampled
objective passed unsigned preflight replay, not that its associated lint caused
a flaw. `consensusReducerConsulted` records a method call, not node validation.
Saved v1 `confirmed` records must be replayed via their embedded request; no
verified-record import exists. See [scenario reproduction](examples/incidents/README.md).
