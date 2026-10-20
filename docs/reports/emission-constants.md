# Emission constants for the allocation check

Prepared on `feat/emission-constants` from `chore/node-pin-55229e00` (`b1d8820`) on
2026-10-21. It carries out the follow-up that [node-pin-55229e00.md](node-pin-55229e00.md)
("Emission-box allocation is not checked", maintainer decision 4) and issue #117 left open.
That report is not edited. This one corrects it.

## The gap

Node 55229e00 checks how a transaction that spends the emission box allocates the emission
NFT and the re-emission token (rule 123, `verify_emission_allocation`) only when
`ReemissionRuleInputs::emission` is `Some`. Forge passed `None`, so a `node-accepted` result
did not cover that rule. The re-emission burn checks always ran under `check_rules: true`.

## What changes

- **A new optional premise.** Enabled `networkRules` may carry `emission`
  (`ergo-sandbox/src/evidence/emission.rs`), and forge passes it to the node as
  `ReemissionRuleInputs::emission`:

  | Field | Node input |
  |---|---|
  | `fixedRate`, `fixedRatePeriod`, `epochLength`, `oneEpochReduction`, `foundersInitialReward`, `minerRewardDelay` | `MonetaryParams` (Scala `monetary.*`) |
  | `emissionNftId` (32 bytes) | `emission_nft_id` |
  | `emissionTree` | `emission_tree`, which must pass the gates the pay-to-reemission tree passes |

- **Origin.** The constants sit inside the `networkRules` premise and share its origin, like
  every other network-rule field. Nothing says a named network or a historical height used
  them.
- **No defaults, no named network.** Forge supplies no mainnet constants anywhere else, so
  none is added. Every field is required, and an unknown field is an import error. The
  node's `ChainSpec::mainnet()` holds the real values and the emission trees. Wiring a named
  network to it would be a separate decision about forge's no-defaults rule.
- **`epochLength` must be at least 1.** The node divides by it, so zero would panic. It is
  refused as an invalid premise, before the node runs.
- **Absent means unchecked, as before.** `emission` is not serialized when absent, so every
  request written before this change re-serializes to the same JSON and keeps its
  fingerprint. It is not a required field because that would change every recorded request.
  `docs/node-validation.md` says that without it the allocation check does not run.
  `AcceptedExecution::report()` is unchanged: it holds the full request, which shows
  whether `emission` was present, and nothing more.

## Pinned records

Nothing historical or frozen was edited or re-recorded.

| Item | How it is handled |
|---|---|
| `ergo-sandbox/src/evidence/validate.rs` is byte-pinned against `docs/reports/batch-10/baseline/` | `baseline_support/mod.rs` admits four more spelled-out edits (the import, the variant's field, the destructuring, the node call and its comment), as PR #116 admitted its own. Without them `property_inventory` fails on "unreviewed engine migration edit" (checked). |
| `ergo-sandbox/Cargo.toml` is pinned the same way | `ergo-chain-spec` moves from the dev-dependencies to the dependencies (the new module names `MonetaryParams`, which no non-dev dependency re-exports). One line, same revision. The allow-list admits exactly that move. |
| `Cargo.lock`, pinned by SHA-256 | Unchanged. The lockfile lists dev-dependencies and dependencies together, and `--locked` accepts it. |
| P03 `manifest.json` and the roadmap policy block | Untouched. The policy names the nine P03 case IDs, and `node_validation.rs` requires exactly those. The new vectors have their own `emission-manifest.json`, as P04's `proof-manifest.json` does. |
| `preservation.json`, the batch-10 baseline copies, other recorded requests and bundles | Untouched. They have no `emission`, so they parse and run as before. |

## Vectors

`emission-vectors/*.fixture` and `emission-manifest.json`, written once by
`generate_emission_vectors.rs`, which calls `ergo_validation::tx::validate_transaction`
directly (not forge's adapter). Both requests spend a 100,001 ERG box holding the emission NFT
and the re-emission token at height 100, activation height 99, with the Scala mainnet
monetary defaults. The NFT, token and trees are hypothetical, like the state.

| Vector | Reward-box tokens | Node (direct) | Forge, with `emission` | Forge, `emission` removed |
|---|---:|---|---|---|
| `emission-allocation-rejection` | 13,000,000,000 | `ReemissionRulesViolated("emission token allocation is invalid")` | the same, stage `node-validation` | accepted |
| `emission-allocation-acceptance` | 12,000,000,000 | accepted, cost 13,101, id `5ec299c9...` | the same | |

The schedule gives 12,000,000,000 at that height (`reemission_for_height`). The rejected
vector overpays the reward box by 1,000,000,000 and takes the same amount from the next
emission box, so the token totals still balance and only the allocation rule can refuse it.
The last column is the gap itself, kept in `emission_allocation.rs`: the same transaction is
accepted when the request carries no `emission`.

`emission_allocation.rs` also checks that each malformed constant (zero `epochLength`, a short
or non-hex NFT id, a tree with trailing bytes, an unparsable tree, a size-less tree whose root
is not a SigmaProp) stops before the node runs, that a missing or unknown constant is an
import error, and that `emission` is refused in disabled rules.

## Not covered

- A request without `emission` still runs no allocation check. This change makes the check
  available. It does not make it the default.
- The node's other emission-box branch (an input whose tree equals `emissionTree` and that
  carries no NFT) is reachable through `emissionTree`, but no vector exercises it.
- The activation-height branch (`height == activation_height`, where the NFT comes from input 1)
  has no vector either.
- Forge's recorded incident bundles and replays carry no `emission`, so they are checked as
  before.

## Checks

Every command forge's CI runs (the 14 checks of [node-pin-55229e00.md](node-pin-55229e00.md)), and
the engine and external measurements compared byte for byte with PR #116's, exited 0, each on its
own, under CI's build settings (`RUSTFLAGS=-D warnings`, the dev profile at `opt-level=3` with
debug assertions and overflow checks), Rust 1.99.0, Node 22, `npm ci --prefix ui`,
`scripts/requirements-test.txt`, the node checked out beside forge at the pin, and `--locked` on
every cargo command. They ran on this change before it was rebased onto `main` at `d0e7f10`
(#120, which changes only the lock commands). The logs are not committed; CI runs on the pull
request.

The workspace and `cost-trace` suites each ran 665 tests, none failed, one ignored (the external
measurement), four more than PR #116's 661: the four in `emission_allocation.rs`.
`tests-web-no-default` ran 97. `contract-suites` ran all 22 example suites. The `lockfile` check
passing is the evidence that `Cargo.lock` did not change. The decompiler measurements are byte for
byte PR #116's, as nothing here touches the compiler or the decompiler. The nine P03 vectors
still import to the JSON they were written as (`node_validation.rs`), and the suites that replay
the other recorded requests pass unchanged.
