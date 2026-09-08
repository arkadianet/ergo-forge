# Drain hunt — phase 1 ("can value leave these boxes?")

> Design record, 2026-09-08. Companion to the P3b spend-hunt spec; the same
> audit layer and the same instrument (bounded scenario sampling on the
> consensus reducer), turned to a different question. This is a **design doc**
> — no implementation. Scope is strictly phase 1; later phases are named where
> they are deferred so they are not rediscovered.

## The question

The spend hunt asks, of one box, **"can anyone spend this box with no key?"**
The USE / DexyGold LP drain (2026-09-08, `examples/incidents/`) was invisible
to that question: every input's script was individually satisfiable *by
design*, and the pool was *meant* to be spent by whoever runs a swap. Nothing
was "spendable by anyone" in the P3b sense. The loss came from the
**composition** — a swap that validated its maths against a box chosen by
position, and a pool that never required the swap to have validated against
*itself*.

So the drain hunt inverts the spend hunt's subject from a box to a **protocol**:

> Given a protected contract set — the boxes carrying a protocol's NFTs — and
> one transaction shape, can any attacker-controlled arrangement of that
> transaction make value **leave** the protected boxes?

"Spendable by anyone" becomes "**drainable**": not *who* may spend, but
*whether value conserved by the protocol can be extracted* by someone who
supplies their own boxes and chooses the ordering.

## Inputs

The hunt takes what an auditor can read off chain without a private key:

- **Protected boxes.** The boxes that hold the protocol's value, each with its
  full contents (value, tokens, registers, ergoTree) — e.g. the LP pool box.
  Supplied as `ScenarioBox`es, exactly as `hunt`/`txcheck` already accept them.
- **Protocol NFTs.** The set of singleton token ids that *define* "protected":
  a box is protected iff its `tokens(0)._1` (by the singleton convention) is in
  this set. For USE LP that is `{ 4ecaa1aa… }` (the LP NFT); the swap NFT
  `ef461517…` names a cooperating contract but holds no reserve, so whether it
  is listed changes only the accounting, not the drain.
- **A transaction template.** The fixed skeleton the protocol expects: which
  protected boxes are inputs, which cooperating contract boxes must be present
  (the swap box), and the recreated successors that keep each protocol script
  happy. This is the "one transaction shape" the question is scoped to.

Data inputs, height and network are carried verbatim as in the spend hunt —
on-chain facts, not spender secrets.

## Probe space — phase 1 only

A probe is one candidate transaction built from the template by applying the
moves an attacker genuinely controls, and nothing more:

| Move | Phase 1 | Why it is enough for the LP class |
|---|---|---|
| **Add decoy inputs** | Attacker supplies extra inputs with chosen value, tokens, registers and script (their own P2PK, or `sigmaProp(true)`). | The drain needs one decoy carrying `tokens(1)`/`tokens(2)` so a positional read of it type-checks. |
| **Permute order** | Permute the input list, and independently the output list, around the template's fixed roles. | The whole exploit is "put the decoy at `INPUTS(0)`, the real pool at `INPUTS(2)`". Order *is* the attack surface. |
| **Choose recipients** | Each template output's recipient script is attacker-chosen (their payout address) where the protocol script does not pin it. | The drained reserves must be allowed to land on an attacker box. |

Everything else is **out of scope for phase 1**, and deferred explicitly:

- **Free-form output synthesis** — inventing outputs beyond the template's
  slots (new boxes, split payouts, arbitrary token layouts). Phase 1 only
  *reorders* and *re-addresses* the template's outputs and *adds inputs*.
- **Coverage-guided fuzzing** — using cost-trace / `--hot-spots` hot spots to
  steer which permutations to try (phase 2, see Oracle).
- **Multi-step / multi-tx chains** — drains that need a setup transaction, a
  poisoned oracle round, or a sequence of spends. Phase 1 is one transaction.

The probe count is the product of the enabled moves, bounded: decoy templates
drawn from a small fixed catalogue (empty box; box mirroring each protected
box's token shape with attacker-chosen amounts), input/output permutations
capped (e.g. all orderings for ≤ N boxes, otherwise a sampled subset with the
protected boxes and the cooperating contract boxes at each position). The cap
is a knob, recorded per run, exactly as the spend hunt records its six probes.

## Objective function — measuring a leak

A probe is scored by how much value **left the protected set to non-protocol
outputs**. Define, over one candidate transaction:

- `protectedIn` = the multiset of (nanoErg, and per-token amount) summed over
  every **input** box whose `tokens(0)._1` is a protocol NFT.
- `protectedOut` = the same sum over every **output** box whose `tokens(0)._1`
  is a protocol NFT (the recreated successors — value that stayed inside the
  protocol).
- **Leak**, per asset (nanoErg, and each token id):
  `leak(asset) = protectedIn(asset) − protectedOut(asset)`.

  A positive `leak` is value that was under protocol control before the
  transaction and is not after — it went to outputs that are *not* protected,
  i.e. to the attacker.

- **Objective** = `leak(nanoErg) + Σ_token leak(token)`, i.e. the total value
  extracted from the protected boxes across the transaction. (Token amounts and
  nanoErg are summed in their raw units; a later phase may weight tokens by a
  price. Phase 1 does not price — a positive leak in *any* asset is a hit.)

For the USE LP drain this is unmistakable: `protectedIn` is 284,695.585 ERG +
the full `useErgLp` and `USE` reserves (the pool at `INPUTS(2)`); `protectedOut`
is 0.002 ERG + one of each token (the drained successor at `OUTPUTS(0)`); the
objective is essentially the entire reserve. The decoy at `INPUTS(0)` is **not**
protected (its `tokens(0)._1` is junk), so it never enters the sum — the leak
is measured on identity, not on position, which is the very property the
deployed swap lacked.

## Oracle — the reducer is ground truth

A probe is only a finding if the transaction it describes would be **accepted
by consensus**. The existing reducer is the oracle, reused with no new
evaluator entry (the one-primitive rule the spend hunt already holds):

A probe **succeeds** when **both**:

1. **Every input script passes.** Each protected box and each cooperating
   contract box, evaluated by `eval_scenario` in the candidate context, returns
   `pass` (reduced to `true`) or `needsProof` (reduces to a sigma proposition —
   the attacker signs their own decoy, which is not an obstacle). A `fail` or
   `error` on any protocol/contract input disqualifies the probe. This is
   exactly the per-input verdict `txcheck` already computes.
2. **The objective is positive.** `leak > 0` by the definition above.

Condition 1 is what makes a hit *real*: the context that produced it is a
transaction the attacker can actually build and get mined. Condition 2 is what
makes it a *drain* rather than a benign reordering. A legitimate swap satisfies
1 but has `leak ≈ 0` (reserves move by the trade amount, conserved into the
successor), so it is correctly not a hit.

Feedback is **milliseconds per probe** — each condition-1 check is a consensus
reduction, sub-millisecond on real trees (measured for the spend hunt on the
mainnet corpus). The whole bounded probe set runs interactively.

`--hot-spots` / cost-trace can later rank the operations a script spends its
cost on, which is a signal for *which* permutations are worth trying — that is
**phase 2** coverage guidance, not phase 1. Phase 1 enumerates its bounded
space exhaustively and needs no guidance.

## Minimisation — delta-debug to a witness

A raw hit may carry more than the exploit needs (extra decoys, an incidental
ordering). On a hit, **delta-debug the probe JSON** to a minimal witness: greedily
drop decoy inputs, collapse permutations toward the template order, and revert
attacker choices, re-running the oracle after each removal and keeping a change
only if the probe still succeeds (both oracle conditions still hold). The fixed
point is the **minimal witness transaction** — the smallest box set that still
drains. For the USE LP drain that reduces to exactly the three inputs / two
relevant outputs the incident corpus already records: one decoy at `INPUTS(0)`,
the swap at `INPUTS(1)`, the pool at `INPUTS(2)`, drained successor at
`OUTPUTS(0)`, payout at `OUTPUTS(2)`.

The minimised witness is emitted **as a scenario/suite** (`ergo-es test`-runnable),
so a discovered drain drops straight into `examples/incidents/` — the hunt's
output is the next regression suite.

## Reuse — what exists, what is new

Phase 1 is assembly over modules that already exist; it adds no evaluator and
no new consensus surface.

**Builds on:**

- `eval` (`eval_scenario`, `EvalOutcome`, `Verdict`) — the oracle for
  condition 1, unchanged.
- `scenario` (`Scenario`, `ScenarioBox`, `TypedValue`) — the probe *is* a
  `Scenario`; protected boxes and decoys are `ScenarioBox`es.
- `box_build` (`build_eval_box`, `build_eval_box_in`) — realising each box with
  real bytes so ids and token layouts are chain-faithful.
- `txcheck` (`check`, `TxCheck`, `InputCheck`) — already computes the per-input
  verdicts *and* `ergIn`/`ergOut` for a whole transaction. Condition 1 is its
  per-input pass/needsProof check; the objective extends its conservation
  accounting from "ERG in vs out" to "protected-in vs protected-out, per asset,
  keyed by NFT". This is the single most-reused module.
- `hunt` (`hunt`, `HuntOptions`, `Probe`) — the bounded-sampling harness and
  its verdict-aggregation shape are the pattern to follow; the drain hunt is a
  second harness beside it, not a change to it.
- `compose` — the composer's **model** (`compose.rs`: `Spec`, `Path`, `BoxRule`,
  the model types around line 807 onward, `Composed`) already reasons about a
  transaction as fixed roles plus per-box rules and derives expected verdicts.
  The transaction **template** borrows that model directly: the template is a
  `compose`-style description of the roles and successors, and the probe
  generator permutes and augments *its* box set.

**New surface (phase 1):**

- A `drainhunt` module: `drain_hunt(protected: &[ScenarioBox], nfts: &[TokenId],
  template: &Template, opts: &DrainOptions) -> DrainReport`. It generates the
  bounded probe set (decoy catalogue × capped permutations × recipient
  choices), runs each through the oracle, scores the objective, and returns the
  hits with their minimised witnesses.
- The **leak accounting** helper (protected-in minus protected-out, per asset,
  keyed by NFT) — a small extension of `txcheck`'s balance logic.
- The **decoy catalogue** and **permutation enumerator** — pure generators over
  `ScenarioBox`, no consensus code.
- A CLI entry `ergo-es drain-hunt <request.json>` mirroring `hunt` /
  `validate-tx`, and (later) a `POST /api/v1/drain-hunt` route reusing the
  inspect route's blocking-task + concurrency machinery.

## Validation — the acceptance test

**The spec is accepted when the phase-1 hunt rediscovers the USE LP drain from
the deployed trees and the protocol NFTs alone.** Concretely, given only:

- the two protected/cooperating boxes as they stood before the drain — the LP
  pool box (LP NFT `4ecaa1aa…`, full reserves) and the `useLpSwap` box (swap
  NFT `ef461517…`), with their **deployed** ergoTrees;
- the protocol NFT set `{ 4ecaa1aa… }`;
- the swap template (pool recreated as its successor, swap preserved);

the hunt must, with **no knowledge of the exploit**, generate a probe that
places an attacker decoy at `INPUTS(0)` and the pool at `INPUTS(2)`, have the
oracle confirm every protocol input passes, score a positive leak of the pool's
reserves, and minimise to the three-input witness. That witness must match the
box set in `examples/incidents/use-lp-drain.deployed-swap.test.json` (the
verdict there — deployed swap `pass` — is condition 1 of the same oracle). The
fixed swap (`fixed/use-lp-swap.es`) under the same hunt must yield **no hit**:
its `INPUTS(0).tokens(0)._1 == $lpNft` makes every decoy-at-0 probe `fail`
condition 1.

Rediscovering a known drain from trees + NFTs is the bar; the incident corpus
is both the fixture and the answer key.

## Honest limits

Phase 1 is a *finder of one class*, not a proof of safety. It **cannot** find:

- **Economic / oracle bugs** — a drain that is value-conserving at the token
  level but wrong in price (a manipulated oracle round, an under-collateralised
  mint). The objective measures token/ERG movement, not worth.
- **Computed-index reads** — a script that selects "the pool" by an index
  computed at runtime, or by an `exists`/`forall` search over the inputs,
  rather than by a fixed position. The template permutes fixed roles; it does
  not model index arithmetic. (The `unbound-box-reserves` lint records the same
  gap.)
- **Anything needing output synthesis** — drains that require inventing outputs
  beyond the template's slots. That is a deferred phase, by construction.
- **Multi-transaction drains** — setup-then-strike sequences. Phase 1 is one tx.

A miss is "not drainable under these phase-1 probes", never "safe" — the same
honesty the spend hunt's `notUnderProbes` verdict carries.

### Responsible disclosure

The hunt is run against **deployed third-party contracts**. A hit is a working
exploit for a live protocol. Therefore:

- Findings on contracts the user **does not own** route **privately** —
  never to a public artifact, a public issue, or a shared channel. The tool
  must make the private path the default and the public path a deliberate,
  owner-only action.
- The incident corpus is published only **after** the fact, for a drain that
  already happened on chain and is public record. A *newly discovered*,
  not-yet-exploited drain is a disclosure obligation to the affected protocol
  first, not a regression suite to commit.
- No timeline or attribution claims travel with a finding. (The "2 hours"
  figure sometimes attached to the USE drain is unverified; the same LP
  contract had legitimate swaps roughly 200k blocks earlier. The tool reports
  the mechanism and the witness, not a story.)

## Out of scope, recorded

- Coverage-guided search over cost hot spots — **phase 2**.
- Free-form output synthesis — **phase 3**.
- Multi-step / multi-tx chains — **phase 3+**.
- Pricing tokens for the objective (worth-weighted leak) — later; phase 1
  treats any positive-leak asset as a hit.
- No capability named "GraphSQL" or any query engine beyond the modules named
  above is assumed or required; the hunt is scenario sampling on the reducer,
  nothing more.
