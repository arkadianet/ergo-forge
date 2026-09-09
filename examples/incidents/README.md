# Incident-replay corpus

> Every mainnet exploit becomes a regression suite **the same day** — the
> 24-hour rule. Reconstruct the drain from on-chain data, prove the deployed
> contract is exploitable, prove a fixed shape is not, and keep both in CI so
> the vulnerable shape can never ship again.

Each suite here is a `*.test.json` runnable with `ergo-es test`. The scenarios
are built from real mainnet boxes — values, tokens, ergoTrees and registers
taken verbatim from the exploit transaction — and each asserts the exact
verdict the consensus reducer gives.

## The invariant a corpus entry encodes

For one exploit, three assertions, each a `pass`/`fail` the reducer confirms:

1. **The deployed contract is exploitable** — the deployed tree, in the real
   exploit context, reduces to the verdict that let value leave.
2. **The fixed contract is not** — the patched source, in the *same* context,
   rejects it.
3. **Ship only the fixed shape** — the fix is the minimal binding that closes
   the substitution, and the forge's `unbound-box-reserves` lint flags the
   deployed tree and clears the fixed one (linked from the audit layer, see
   below).

## USE / DexyGold LP drain — 2026-09-08 (height 1868204)

Exploit tx `5371373d346aade57f684ead23f386e3441ffb2cb7a860babe96e3c5048b2725`.

Root cause (full write-up in `USE-LP-incident-report.md`): the `useLpSwap`
box enforces the constant-product maths against **the box at `INPUTS(0)`**,
by position, and never checks that box carries the LP singleton NFT. The
`useLp` pool box, for its part, only checks that *some* input carries a swap
NFT — never that the swap validated against *itself*. So the attacker put a
decoy box (0.002 ERG, three junk tokens so `tokens(1)`/`tokens(2)` exist) at
`INPUTS(0)`, the real pool at `INPUTS(2)`, and the swap at `INPUTS(1)`. The
swap did its maths on the decoy's fabricated "before" and the drained pool's
real "after"; the pool waved it through. 284,695.59 ERG plus the reserves left
to the attacker's payout at `OUTPUTS(2)`.

| index | INPUTS | OUTPUTS |
|---|---|---|
| 0 | attacker decoy (0.002 ERG, 3 junk tokens) | drained LP successor (0.002 ERG, 1 of each token) |
| 1 | `useLpSwap` box (swap NFT `ef461517…`) | swap box, preserved |
| 2 | real LP pool (284,695.585 ERG + reserves, LP NFT `4ecaa1aa…`) | attacker payout (284,695.583 ERG + reserves) |
| 3 | — | miner fee |

### Suites

The suite loader compiles **one** contract per suite (a scenario may not name
its own tree/source — the suite's contract is what is under test), so this one
incident is three suite files, one per script under test, sharing the same
reconstructed box set:

| file | contract | asserts |
|---|---|---|
| `use-lp-drain.deployed-swap.test.json` | deployed swap tree (`ef461517…`) | **pass** — the swap script accepts the drain. This is the proof the deployed contract is exploitable. |
| `use-lp-drain.deployed-pool.test.json` | deployed pool tree (LP NFT `4ecaa1aa…`) | **pass** — the pool script accepts it too; its only cross-box check is "`INPUTS(1)` carries a swap NFT". |
| `use-lp-drain.fixed-swap.test.json` | `fixed/use-lp-swap.es` compiled with `$lpNft = 4ecaa1aa…` | **fail** — same context, but `INPUTS(0).tokens(0)._1 == $lpNft` rejects the decoy. |

### The fix

`fixed/use-lp-swap.es` is the deployed swap logic (decompiled from the mainnet
tree) with the one missing line restored:

```
poolIn.tokens(0)._1  == $lpNft &&
poolOut.tokens(0)._1 == $lpNft
```

Because the LP NFT is minted in quantity one, only the genuine pool can satisfy
it, and the positional substitution is closed. This is the same principle the
USE **bank** already follows (it binds every box it touches by unique NFT), and
why the bank was assessed as not vulnerable to this technique.

## USE bank vault — funded target, corrected decode (2026-09-08)

`use-bank-vault.json` is not a replay suite; it is the **checkable decode** of
the still-funded vault (box `e6162e2aff23f7c88968cc958541bacfe3ad80d6541befb7231ac3106e966f8b`,
292,615.109709675 ERG + the USE treasury, untouched since height 1,867,725).
Landing it was the phase-2 spec's own gate — and it caught an error: the
design doc's hand-decode of the whitelist (`OUTPUTS(0).tokens(2)._1`) was
wrong on collection and index. The deployed tree asserts, mechanically
(`ergo-sandbox/tests/vault_corpus.rs`):

- the whitelist is **input-side**: `INPUTS(0).tokens(0)._1` ∈ {useFreeMint,
  useArbitrageMint, usePayout, useUpdateNft};
- the never-minted `dbf655…` dead branch reads `INPUTS(2).tokens(0)._1`;
- a faithful continuation is required at `OUTPUTS(1)` (same script bytes,
  `tokens(0)` id **and amount** pinned, `tokens(1)` id only — the treasury
  amount is *not* pinned by the vault script itself);
- the `useUpdateNft` route stands **alone**, outside the continuation guards
  — which is exactly why the admin's P2PK box (needsProof) is the route the
  hunt refuses by construction.
- **The lint record.** `unbound-box-reserves` on the deployed tree says
  **clean** — the vault has the NFT-binding shape the lint rewards — but
  clean is not safe: the vault pins its successor's *identity*, not its
  value (the treasury token is compared by id only, the successor's ERG
  value not at all). A dust successor is permitted by the vault script
  alone; every reserve guarantee is delegated to the three script
  authorizers. That collapses the flagship question to a single sharper
  one: **do `useFreeMint`/`useArbitrageMint`/`usePayout` constrain the
  bank's value?** The empirical run needs those three companions; the admin
  box contributes nothing.

The tree re-serializes byte-identically and the fixture's `mirrorSource`
compiles back to the same constants and proposition, so the decode is a fact
about the wire bytes, not a reading. The flagship empirical run (are the
three script authorizers satisfiable without a key?) is deliberately not
pinned here yet — see the phase-2 spec's disclosure-first rule.

## How to run

```sh
cargo run -p ergo-sandbox --bin ergo-es -- test examples/incidents/use-lp-drain.deployed-swap.test.json
cargo run -p ergo-sandbox --bin ergo-es -- test examples/incidents/use-lp-drain.deployed-pool.test.json
cargo run -p ergo-sandbox --bin ergo-es -- test examples/incidents/use-lp-drain.fixed-swap.test.json
```

Add `--json` for the machine shape (`docs/scenario-format.md`). Every suite
here is run in CI by the `contract-tests` workflow.

## Linkage to the audit lint

The static lint that catches this class before deployment is
`unbound-box-reserves` (`ergo-sandbox/src/audit/lints/unbound_box_reserves.rs`).
Its regression coverage lives in `ergo-sandbox/tests/audit.rs`:

- `the_deployed_use_lp_swap_contract_is_flagged` — the deployed swap tree flags.
- `the_deployed_use_lp_extract_contract_is_clean` /
  `the_deployed_use_bank_contract_is_clean` — the NFT-binding siblings are clean.
- `the_incident_corpus_fixed_swap_is_clean` — compiles **this directory's**
  `fixed/use-lp-swap.es` and asserts the lint clears it, tying the replay
  corpus to the audit layer.

## Fidelity — what is exact, what is approximated

- **Exact.** All three inputs and all four outputs are taken verbatim from the
  exploit transaction: values, token ids and amounts, ergoTrees, and the LP
  pool's `R4` register (`Coll[Byte]` "New", carried as a `raw` constant). The
  swap and pool are evaluated with `selfIndex` 1 and 2, so `SELF` is the real
  box at that position with its real script. Both reduce to a boolean directly
  (`pass`/`fail`), so no proof, secret, or signed message is involved — the
  reducer's verdict is the whole story, and it matches the chain.
- **Not modelled, because no script reads it.** `preHeader`, `headers`,
  `minerPubkey` are left at their zero defaults; `CONTEXT.HEIGHT` is set to the
  inclusion height 1868204. Neither script reads any of these. Box ids are
  computed from the box bytes (the chain-standard way) rather than pinned to
  the on-chain ids; no script compares a box id to a literal, so this is
  invisible to the verdict.
- **The attacker's own decoy input (`INPUTS(0)`) is a P2PK box.** In the real
  spend it needed the attacker's signature; here it is present only as context
  for the swap and pool scripts, which is all the exploit proof requires. We do
  not evaluate its P2PK script (it is the attacker's box, not the protocol's).
- **No timeline claims.** This corpus documents the *mechanism*, not how fast
  it was found. The same LP contract had legitimate swaps long before the drain.

## Recorded finding triage

From the repository root (use your worktree's isolated target directory):

```sh
CARGO_TARGET_DIR=./target-gate cargo run --release -p ergo-sandbox --bin ergo-es -- triage examples/incidents/use-lp.triage.json > deployed-triage.json
CARGO_TARGET_DIR=./target-gate cargo run --release -p ergo-sandbox --bin ergo-es -- triage examples/incidents/fixed/use-lp.triage.json > fixed-triage.json
```

These committed requests select the pool's `delegated-reserves` finding at
node 31. They differ only in the companion swap's compiled contract. The
finding persists on the pool in both sets; the fixed swap supplies the missing
binding. The deployed set reaches `confirmed`; the fixed set reaches
`not-reproduced`. **Absence of a result under a bound is not evidence of
absence. Not-reproduced does not mean safe.** Confirmation means the declared
contract set reached the declared objective, not that the selected lint alone
caused it.

A triage request is a JSON object with `inputIndex` (declared spending input),
`lint`, `nodeId` (from `ergo-es audit <tree-hex>`), and `drain` (a complete
drain-hunt request). Commit that object to make the check reproducible.
The selected input must be protected or companion and have explicit
`ergoTree` bytes. Triage re-audits those bytes and rejects a stale anchor.
Choose roles, authorization policy, and search bounds for the actual protocol:
the incident's empty `objective.terms` declares no authorized releases and
must not be copied blindly to contracts allowing payments or withdrawals.

Each output is a finding with recorded triage evidence: the full request,
hunt report, resolved synthesis caps, shape tallies, probe and oracle-call
counts, and (for confirmation) the winning transaction plus its independently
replayed reducer verdict. Extract `triage.evidence.request` to rerun triage,
`triage.evidence.request.drain` to run `ergo-es drain`, or
`triage.evidence.hunt.best.witness.txRequest` to run `ergo-es validate-tx`.
Malformed shapes and incomplete objectives stay unconfirmed with their hunt
notes; they cannot become bounded negative results. Ordinary audit findings
start unconfirmed and explicitly disclose that the reducer was not consulted.

To regenerate the two request files mechanically from the incident boxes and
fixed source:

```sh
CARGO_TARGET_DIR=./target-gate cargo run --release -p ergo-sandbox --example triage_incident_requests
```

The regression tests also verify that the fixed request's bytes match the
committed fixed source and parameters.
