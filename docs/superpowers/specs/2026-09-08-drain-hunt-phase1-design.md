# Drain hunt phase 1 — decoy substitution and input permutation

> Design record, 2026-09-08. Companion to the P3b spend-hunt spec; same audit
> layer, same reducer, a bigger question. Extends the P3a lint work
> (`unbound-box-reserves`, #59) from static to dynamic. This is a **design
> doc** — no implementation. Scope is strictly phase 1; later phases are named
> where they are deferred so they are not rediscovered.

## Context

The 2026-09-08 USE/Dexy LP drain emptied two pools (~287,900 ERG) through
contracts that were each, internally, correct. The swap-order script computed
the constant-product invariant faithfully — against `INPUTS(0)`, a position
the transaction builder chooses. The pool script released everything to any
transaction carrying an order NFT at `INPUTS(1)`. Nothing in either tree said
which box had to sit at which index, so the attacker placed a decoy wallet box
at position 0 and the real pool at position 2. The maths was right; it was
applied to the wrong box.

The static lint (#59) catches this shape in one tree. It cannot catch it
across a contract *set*, and it cannot catch classes nobody has written a lint
for yet. The spend hunt already runs the real reducer over attacker-controlled
contexts — but its probe space varies only height and outputs, with
`INPUTS = [SELF]`. Against the deployed order box it correctly answers
`requiresProof` (the trader's key): true, and irrelevant. The drain never
spends the order box's 1 ERG; it launders the order box's *script
satisfaction* into authorization for the pool box. The spend hunt's unit of
analysis — one box, its own value — cannot see a composition bug.

Timeline note (corrected): the flaw was live for **months**, not hours. Pool
states under this template churn on mainnet from at least h≈1.66M, and the
incident-corpus work reports a legitimate swap on the same contract at
h≈1,658,380. A sentinel (future phase) would have had months of runway. That
figure must be verified box-by-box before it is quoted anywhere, and no
timeline travels with a finding (see Disclosure).

## The question

> **Can a transaction that holds no key extract value from these protected
> boxes, over the transaction shapes an attacker can build?**

The unit of analysis changes from *a box* — the spend hunt's "can anyone spend
this box with no key?" — to *a contract set plus a fixed transaction shape*.
Phase 1 keeps the shape fixed and varies only what the attacker controls about
composition: the order of inputs and the contents of attacker-owned slots.
"Spendable by anyone" becomes **drainable**: not *who* may spend, but *whether
value the protocol conserves can be extracted* by someone who supplies their
own boxes and chooses the ordering.

## Roles

The caller declares a scenario (existing `Scenario`/`ScenarioBox` types) and
labels each input with a role. Roles are the whole interface: they say what an
attacker controls, and the hunt refuses to guess.

| Role | Meaning | Attacker may… |
|---|---|---|
| `protected` | Boxes whose value must not leave — the pool, the vault. Identity is their singleton NFT (`tokens(0)._1`). | include at any input index; never alter contents |
| `companion` | Keyless boxes whose script *is* the authorization (`useLpSwap`, trackers, any box whose spend-hunt verdict was `movableByAnyone`). Present by design. | include, reorder; never alter contents |
| `attacker` | The attacker's own funds — decoy material. | reorder, substitute contents from the decoy family, or omit |
| `external` | Data inputs and pinned boxes (oracles). Fixed on-chain facts. | nothing |

Roles apply to the declared **`inputs` list only**. `data_inputs` are
`external` by definition — pinned by box id, carried verbatim, never permuted
and never re-addressed (a probe that swaps in a different oracle box is a
different protocol instance, which is out of scope and recorded as a limit
below). Unlabelled inputs are rejected: the hunt will not analyse a set it
cannot attribute. Data inputs, height and network are carried verbatim as in
the spend hunt — on-chain facts, not spender secrets.

Outputs keep the caller's shape (this is what makes it *phase 1*), except that
any output the caller marks `payee: free` may have its recipient tree
substituted for the attacker's — in the USE replay the attacker simply played
the trader and took the payout, and honest shapes usually have at least one
free payee.

## Probe space (bounded enumeration, no search yet)

A probe is one candidate transaction built from the template by applying only
the moves an attacker genuinely controls. Two axes, multiplied:

1. **Input permutation** — every ordering of the declared `protected`,
   `companion` and `attacker` inputs. The whole exploit is "put the decoy at
   `INPUTS(0)`, the real pool at `INPUTS(2)`": order *is* the attack surface.
   `external` entries keep their absolute positions and contents, so the hunt
   only ever evaluates shapes the attacker can actually build. Cap: inputs
   ≤ 5 → at most 120 permutations; the cap is a parameter, and overflow
   samples deterministically (seeded, ordered), never silently.
2. **Decoy substitution** — for each `attacker` slot, the decoy family:
   - the caller's declared decoys, plus
   - the generic family, a **finite, script-independent** enumeration:
     `{declared reserves} × k` for `k ∈ {0, 1, 10}`; one fixed dummy token id
     (32 zero bytes) placed as amount 1 at every token index `0..3`; token
     reordering (declared order, reversed); registers absent or zeroed.

The generic family never reads the target script, its trees, or its state —
a script-adaptive decoy (one "matched to the script's implied minimum") would
make the family non-reproducible and void the anti-cheat. The generic family
is deliberately boring. **It must include, without any
special-casing, the exact shape that drained USE** — a 2-ERG wallet box
carrying three amount-1 filler tokens. The incident replay (below) is the test
that the boring family is sufficient; if it ever is not, the family grows and
the incident corpus records why. This is the anti-cheat: the acceptance test
is worthless if the winning decoy was hand-fed, so the family is fixed and
generic and the fixture must fall out of it.

Phase 1 **enumerates; it does not search** — no coverage guidance, no solver,
no output synthesis. Enumeration with caps keeps the implementation honest,
deterministic and debuggable, and — per the incident — it is enough: the USE
witness sits in the smallest corner of the space. The probe count is the
product of the enabled axes and is recorded per run, exactly as the spend hunt
records its six probes.

## Objective — measuring a leak by identity, not position

A probe is scored by how much value **left the protected set to outputs that do
not faithfully recreate it**. Roles — not token ids — define the protected set;
the protocol-NFT set is used to *validate identity*, never to classify:

- `protectedIn(asset)` = the sum of `asset` (nanoErg, or a token amount) over
  every **input labelled `protected`**. `companion` boxes never enter the sum,
  regardless of what their `tokens(0)._1` happens to be — the USE order box
  carries the swap NFT at index 0, and that NFT is an authorization flag, not
  reserve.
- Identity check (separate step): every `protected` box must carry its
  declared singleton protocol NFT at `tokens(0)._1`; a mismatch is
  `invalidShape`, not a leak.
- `protectedOut(asset)` = the same sum over every **output** box that
  **faithfully recreates** a `protected` input: same `propositionBytes`, same
  token ids and amounts, value no smaller — the successor test the static lint
  and the P3b hunt already use.

```
extracted = Σ_asset max(0, protectedIn(asset) − protectedOut(asset))
          over nanoErg and every token id
```

Value moving between protected boxes, or into a faithful recreation, does not
count. Fees are not subtracted — the attacker's problem; the report is gross.

The measurement is keyed on **identity, not position**: the decoy at
`INPUTS(0)` is labelled `attacker`, so it never enters `protectedIn`, and the
leak is scored against the pool wherever it actually sits. That is precisely
the property the deployed swap lacked, which is why measuring it this way finds
the bug the swap could not see. For the USE drain, `protectedIn` is
284,695.585 ERG plus the full `useErgLp` and `USE` reserves (the pool box
alone — the order box is `companion` and contributes nothing);
`protectedOut` is the 0.002-ERG drained successor; `extracted` is essentially
the whole reserve.

Phase 1 does not price tokens: a positive `extracted` in **any** asset is a
hit. A worth-weighted objective is a later refinement.

## Oracle — the reducer is ground truth

A probe is a finding only if consensus would accept its transaction. The
existing reducer is the oracle, reused with no new evaluator entry (the
one-primitive rule the spend hunt already holds). A probe **drains** when both:

1. **The full transaction validates.** `txcheck::check` runs over the whole
   probe — per-input script verdicts, ERG/token conservation, output rules:
   the same surface `validate-tx` uses, not script behaviour alone. On every
   `protected` and `companion` input the verdict must be **`pass`** —
   `needsProof` is *never* accepted for these roles, because a residual sigma
   proposition is a key the attacker does not hold (only `attacker` inputs may
   reduce to a proof: their own key, no obstacle). A `fail`, `error` or
   `needsProof` on any protocol/companion input disqualifies the probe, and
   the residual is reported with the miss. The USE replay is unaffected by
   this rule: in the drain layout the deployed pool and order box both reduce
   to `true`.
2. **`extracted` is positive** under the objective above.

Millisecond reducer feedback makes exhaustive enumeration over the capped space
practical. Using cost-trace / `--hot-spots` hot spots to *steer* which probes to
try is phase 3, explicitly not phase 1.

## Verdict

Carried over from the hunt vocabulary, priority order:

1. **`drainable`** — some probe extracted value. The finding names the
   extraction (per token, per ERG), the winning permutation, the decoys used,
   and carries the witness.
2. **`notUnderProbes`** — nothing extracted. The report states the full probe
   space (roles, caps, decoy family) so a miss is never mistaken for a clean
   bill. Same honesty as P3b: a miss is "not under these probes", never "safe".
3. **`invalidShape`** — caller error: incomplete role coverage, caps exceeded,
   or a template that does not reduce.

## Witness

The witness is a **bundle**, because the two CLI validators consume different
representations: `ergo-es eval` reads a `Scenario`, `ergo-es validate-tx`
reads a `TxRequest` (`tx`, `boxes`, optional `height`). The bundle carries all
three, plus the role labels:

- `scenario` — the probe as a `Scenario`, with roles recorded alongside;
- `txRequest` — derived mechanically from the scenario: the boxes are
  realised with real bytes via `box_build` (so box ids are chain-faithful),
  and the `TxRequest` references those ids;
- the conversion is a pure function, part of the `drainhunt` module, so
  `scenario` and `txRequest` cannot drift.

Anyone can reproduce the verdict with either CLI and diff it against the
mainnet transaction it mirrors. For the incident corpus the witness is
byte-comparable in *effect* (same box ids where inputs are pinned, same
extraction) to the on-chain drain, and must match
`examples/incidents/use-lp-drain.deployed-swap.test.json`.

Delta-debug minimisation to a smallest witness is included only if it falls out
of the report plumbing for free; it is **not** required for acceptance (phase 3
concolic minimisation is the real treatment).

## Acceptance criteria

1. **Incident replay (positive control).** Vendored trees: the deployed USE
   pool and swap-order (`ErgoTree` hex from mainnet, round-trip per the P2
   floor). Roles: pool `protected`, order box `companion`, attacker wallet
   `attacker`; the honest builder shape with a free payee. With **no knowledge
   of the exploit**, the hunt must find a witness extracting ≥ **74,519,918 USE
   base units** and **284,695,583,089,453 nanoERG**, via permutation + the
   generic decoy family alone. The same fixture for the DexyGold pair must
   extract its pool. The witness must match the box set in
   `examples/incidents/use-lp-drain.deployed-swap.test.json`.
2. **Negative control.** The gallery's correct pair
   (`examples/contracts/protocols/amm/`: `pool.es` + `swap-order.es`, NFT-bound)
   must yield `notUnderProbes` — no permutation or generic decoy extracts. So
   must the incident corpus's `fixed/use-lp-swap.es`. If either ever leaks,
   the pair is broken or the objective is wrong, and we **stop until we know
   which**. An independently-correct pair is a stronger control than a patch of
   the broken one, so both are used.
3. **Boundedness and determinism.** Caps enforced; same input → same probes →
   same verdict, seeded sampling included.
4. **Witness portability.** Every reported witness bundle re-evaluates to
   `pass` with the same extraction via `ergo-es eval`, and its derived
   `TxRequest` — not a hand-written copy — validates via `validate-tx`, with
   the role labels preserved in the bundle.

Rediscovering a known drain from trees + NFTs alone is the bar; the incident
corpus is both the fixture and the answer key.

## Integration

- `ergo-sandbox/src/drain.rs` next to `hunt.rs`; reuses `eval_scenario`,
  `Scenario`, `ScenarioBox`, the attacker-tree compiler, and the report shapes.
  The "faithful recreation" logic moves to a shared helper rather than being
  duplicated across the lint, the P3b hunt and this.
- CLI: `ergo-es drain scenario.json [--roles inline | --roles roles.json]`,
  JSON report via the `--json` convention shipped in #58.
- Web: `POST /api/v1/drain` later; the Read pane can offer it once role
  labelling has an interface. Not phase 1.
- The incident-corpus fixtures live in `examples/incidents/` (landed in #60);
  the drain tests reference them, so the corpus and the hunt keep each other
  honest.

## Honest limits

Phase 1 is a *finder of one class*, not a proof of safety. It **cannot** find:

- **Economic / oracle bugs** — a drain that is value-conserving at the token
  level but wrong in price (a manipulated oracle round, an under-collateralised
  mint, the price-coupled tracker interventions from the USE incident). The
  objective measures movement, not worth.
- **Computed-index reads** — a script that selects "the pool" by an index
  computed at runtime, or by an `exists`/`forall` search over the inputs,
  rather than a fixed position. The template permutes fixed roles; it does not
  model index arithmetic. (`unbound-box-reserves` records the same gap.)
- **Anything needing output synthesis** — drains that require inventing outputs
  beyond the template's slots. Deferred by construction.
- **Multi-transaction drains** — setup-then-strike sequences.
- **Swapped oracles** — `external`/data inputs are pinned by box id and held
  fixed; a drain that requires substituting a *different* on-chain oracle box
  is a different protocol instance and is out of scope here.

A miss is "not drainable under these phase-1 probes", never "safe" — the same
honesty the spend hunt's `notUnderProbes` verdict carries.

## Responsible disclosure

The hunt is run against **deployed third-party contracts**. A `drainable`
verdict is a working exploit for live money. Therefore:

- Findings on contracts the operator **does not own** route **privately** — to
  the affected team and to any bug-bounty the project runs — never to a public
  artifact, a public issue, a shared channel, or a feed. The tool makes the
  private path the default and public reporting a deliberate, owner-only action
  taken only after a fix or an agreed disclosure window.
- The incident corpus is published only **after** the fact, for a drain already
  on chain and in the public record. A *newly discovered*, not-yet-exploited
  drain is a disclosure obligation to the affected protocol first, **not** a
  regression suite to commit.
- No timeline or attribution claims travel with a finding. The tool reports the
  mechanism and the witness, not a story. (The "2 hours" figure sometimes
  attached to the USE drain is unverified; the same LP contract had legitimate
  swaps months earlier.)

The drain hunt makes finding these cheap; this clause is what keeps that
legitimate, and it is a **precondition of the sentinel**, not an afterthought.

## Sentinel preconditions (recorded now, built later)

The sentinel that would run this over every live deployment needs two things
designed in from the start:

1. **A reproducible chain source.** During incident response, contract-set
   enumeration ran manually against the public explorer GraphQL service
   (`gql.ergoplatform.com`) — template-hash queries, recorded in the session,
   re-runnable today. This is a service the forge *used*, not a capability it
   *has*; the sentinel must pin its data source (explorer REST, that GraphQL,
   or a node indexer), version it, and archive raw responses alongside every
   sweep so any published finding is re-derivable. No query engine beyond a
   named, pinned source is assumed.
2. **Responsible disclosure by default**, as above — the default output of a
   sweep is a private report.

## Out of scope, recorded

- Free-form output synthesis (attacker invents outputs) — **phase 2**.
- Coverage-guided search over cost hot spots, and concolic minimisation —
  **phase 3**. Delta-debug on witnesses only if it is free; not required for
  acceptance.
- Multi-step / multi-tx chains — **phase 3+**.
- Economic probes (front-running at height+1, oracle staleness) — a **separate
  spec**; the price-coupled tracker interventions in the USE incident are the
  motivating case.
- Worth-weighted (priced) objective — later; phase 1 treats any positive-leak
  asset as a hit.
- Any claim about *all* attacker strategies. The verdict vocabulary exists
  because phase 1 is a sample — a bigger, smarter sample than the spend hunt,
  and one that would have caught USE months ago, but a sample.

## Estimate

Days, not weeks: probe generation is permutation × a small decoy family over
existing scenario types; the reducer and report plumbing exist; the fixtures
are two decompiled trees and a shape JSON. The schedule risk is the
attribution logic (faithful-recreation edge cases), covered by acceptance
criterion 2's negative control.
