# Mutation corpus — the scoreboard for the drain hunt

> Design record, 2026-09-08. Implements phase 2.5 of the roadmap
> (`2026-09-08-drain-hunt-roadmap.md`, #66). Phase 1 found exactly one bug: the
> one it was built from. This corpus answers the question the roadmap asks —
> **does the hunt find bugs nobody showed it?** — with a number, and classifies
> every miss so the number has a shape.

## The methodological crux

A seeded defect only counts if it is **actually exploitable**. A mutation that
weakens a check without creating a reachable keyless drain is not a miss when
the hunt does not find it — it is a non-bug. Every mutant in this corpus is
therefore one of:

- **proven** — a hand-built witness exists showing a keyless drain of the
  mutant: a transaction that `txcheck::check` accepts, in which value leaves
  a protected box without any signature the attacker does not hold. These
  are constructed exactly the way the USE rediscovery is — by hand, from the
  protocol's honest shape, before the hunt runs. They form **the denominator
  of the detection rate**.
- **unproven** — believed exploitable, no witness constructed. Reported
  separately, **never in the denominator**.
- **negative control** — the unmutated original. Must come back
  `notUnderProbes`. If an original reports `drainable`, that is either a real
  finding in a contract we ship as an example (escalate; do not silently fix
  the fixture) or a harness bug.

Detection rate = proven mutants found / proven mutants. Both counts are
published, with the unproven list. A rate over an unknown denominator is
worse than no rate.

## The anti-fixture rules

Three review rounds on the phase-2 implementation found the same failure: a
property verified in a fixture configuration where it holds, failing on real
input. The analogue here inflates the headline number, so the corpus has
three hard rules:

1. **The transaction template is the protocol's HONEST shape.** What a
   legitimate fill looks like — successors rebuilt verbatim, payments to the
   right parties, a free payee that exists in the honest transaction. The
   hunt must find the drain *from there*. A template that already contains
   the attack shape is a fixture bug.
2. **Caps are part of the measurement.** One cap configuration for the whole
   corpus, recorded per mutant (`probes_run`, `capped`). A mutant found only
   at 200k probes is a *cost* result, recorded as such — never a reason to
   raise the cap for that mutant.
3. **Role labelling is decided once, from the protocol's structure, before
   the hunt runs**, and recorded in the mutant file. Labeling is a judgement
   call that decides what is even askable; iterating it until something is
   found is how a fixture starts lying.

Both configurations run per mutant: synthesis off, and synthesis on (the
phase-2 block, fully enabled). #67 records why the two are not the same
under a binding cap; the corpus reports both so the record shows what the
synthesis degrees buy.

## What the corpus covers

Seven mutants across six contracts, all drawn from `examples/tests/` (the
workbench's own contract suites — sources compiled with `compile_with_params`,
never hand-edited trees):

| # | contract | operator | defect | proven? | attacker model |
|---|----------|----------|--------|---------|----------------|
| M1 | `vesting` | 5 — weaken comparison | `remainder.value >= total - vested` → `>= 0L`: the beneficiary-path residual no longer binds the remainder's value | **no — keyed-insider** | the residual is `beneficiary && …`: needsProof on the protected input; only the beneficiary can build the spend |
| M2 | `subscription` | 5 | `OUTPUTS(1).value >= SELF.value - $amount` → `>= 0L`: the receiver takes a payment while the "pot kept" guard passes at dust | **no — keyed-insider** | the residual is `funder \|\| (receiver && …)`; the receiver-keyed path |
| M3 | `subscription` | 2 — flip index | `OUTPUTS(1).value >= …` → `OUTPUTS(0).value >= …`: the pot guard reads the payment output | **no — keyed-insider** | as M2 |
| M4 | `token-sale` | 5 | `payment.value >= sold * $pricePerToken` → `>= 0L`: the buyer takes the whole stock for dust | **yes** — the buyer path is `sigmaProp(…)`, keyless | keyless |
| M5 | `bank` (AgeUSD shape) | 1 — delete NFT identity check | `successor.tokens(0) == SELF.tokens(0)` deleted: the bank NFT (box identity) leaves with the attacker while a junk token sits at `tokens(0)` | **yes** — the bank's residual is a pure bool; every input passes | keyless |
| M6 | `amm-pool` | 4 — drop `propositionBytes ==` | the pool successor may be re-treed to the attacker's script: the full reserves end the transaction in an attacker-spendable box | **yes** | keyless |
| M7 | `use-lp-swap` (the FIXED incident contract) | 4 | `swapSucc.propositionBytes == SELF.propositionBytes` dropped: the swap successor can be re-treed, stealing the locked order value | **yes** | keyless |

M1–M3 taught the first corpus lesson, and it is exactly the one the
methodology warns about: their witnesses *validated under `txcheck`* — the
oracle is unsigned by design, so a residual needing a key shows up as
`needsProof`, not invalid — and they were initially misproven. Reclassified:
a mutant is **proven** only if its witness needs no signature the attacker
does not hold. The keyed-insider class (M1–M3) is real defects the drain
hunt structurally cannot see (it refuses `needsProof` on protected inputs by
contract); it goes to the phase-3+ attacker-model backlog, not the rate.

Excluded by the methodology (recorded, not silently dropped):

- **savings-cap cap-weakening** (`rest.value >= SELF.value - $cap` → `>= 0L`)
  — the exploit still requires `owner` (`needsProof` on a protected input);
  the drain hunt models *keyless* attackers by contract, so this mutant has
  no keyless witness. Not a non-bug — a different attacker model.
- **htlc comparison weakening** (`HEIGHT < $deadline` → `<=`) — a one-block
  off-by-one with no value consequence; no witness exists because no drain
  exists.
- **use-lp-swap index flip** (`INPUTS(0)` → `INPUTS(1)`) — the maths move to
  the swap box, which cannot carry the LP NFT; the mutant fails closed. Not
  exploitable.

Excluded classes (out of scope for phase 2.5, per the roadmap): register
value variation and data-input/oracle manipulation (phase 3's axes),
multi-transaction defects (phase 5), arithmetic mutants needing computed
constants (phase 4's exit criterion — M1–M7's witnesses all use amounts the
honest template or plain arithmetic can produce, deliberately: no corpus
mutant should *require* the phase-4 solver to be provable).

## The detection-rate experiment (recorded)

Methodology, applied: every mutant is compiled from source with
`compile_with_params`; every witness was hand-built from the honest template
and validated with `txcheck::check` **before** the hunt ran; every original
runs as a negative control (all seven originals return `notUnderProbes` in
both configurations); every mutant runs twice (synthesis off/on); one cap
policy for all (`maxProbes` 50,000, `maxPermutations` 120, synthesis
fully on in the second configuration).

**The rate: 0 attributable / 4 proven = 0.00**, with one raw drainable
verdict, confounded.

A drainable verdict counts **only when the mutant's own unmutated control is
clean in the same configuration** — that is the entire reason the control
exists, and a hit that also fires on the original says nothing about the
mutation. M4 is the case: the hunt reports `drainable` on the mutant *and*
on the unmutated token-sale (see the escalation below), so its verdict is
not attributable. It is reported separately, never in the rate.

The harness **derives** attribution from the two runs rather than taking a
recorded flag: the last hand-held guarantee here (the keyless proof) had
already been promoted to a mechanical check for the same reason, and a
number this document leans on should not depend on someone remembering to
set a boolean.

Recorded in `examples/mutants/answer-key.json` and asserted by
`ergo-sandbox/tests/mutation_corpus.rs` as **no regression** — the recorded
per-mutant verdicts must not degrade, the confounded set is pinned exactly
(a mutant becoming confounded, or ceasing to be, changes what the number
means), and never an absolute floor (a hard threshold turns into a fixture
that gets tuned).

### The operator matrix — read the rate next to it

| operator | proven | attributable | raw drainable |
|----------|--------|--------------|---------------|
| delete-nft-check (M5) | 1 | 0 | 0 |
| drop-successor-script (M6, M7) | 2 | 0 | 0 |
| weaken-comparison (M4) | 1 | 0 | 1 (confounded) |
| **flip-index** | **0** | — | — |
| **positional ↔ unbound-search** | **0** | — | — |

0.00 is a rate over **three of the five operators**. flip-index's only
candidate (M3) turned out keyed-insider, and positional ↔ unbound-search —
the USE incident's own class — has no mutant at all yet. Keyless mutants
for the two uncovered operators are the next increment and are deliberately
**not** in this PR (candidate sketches: an amm-pool index flip whose
successor reads land on an attacker-controlled output; a positional →
unbound-search mutant on the fixed swap's pool binding). If they push the
rate down, that is also the finding.

### The must-find control (harness validity, not a mutant)

`knownDetectableControl` in `mutants.json`: the `delete-nft-check` operator
applied to the incident's own FIXED contract reconstructs the **deployed
vulnerable `useLpSwap`** — the contract that drained 284,695 ERG on mainnet.
Ground truth is certain; phase 1 rediscovers it today from the honest shape.
The harness asserts it as a **separate pass/fail on the apparatus**, outside
the rate: a found control would move the rate without the hunt changing,
which would read as improvement. Landing it caught a fixture bug in its own
template (the swap successor initially lacked the swap NFT, failing
`swapSucc.tokens == SELF.tokens` on every probe) — the control doing its job.

### The negative control that fired

M4's unmutated original reports `drainable` with synthesis on — recorded in
`escalatedFindings`, not softened: the winning probe has the attacker
**overpay the seller** (20M of the attacker's own nanoERG into the seller's
P2PK box) so `paid` passes legitimately and the stock leaves with the
payment. The seller is fully paid; the attacker loses money. No exploit —
**the leak objective has no attacker-cost accounting**: protected value that
empties into a protocol-designated payee scores as a drain. That objective
limitation is the roadmap's parked "pricing / worth-weighted objectives;
economic probes — separate spec", and this firing is its concrete evidence.
Consequence recorded honestly: M4's `found` is **confounded** — the mutant
IS really exploitable (the 1-nanoERG purchase is a genuine keyless theft in
family), but the verdict alone cannot discriminate real drains from
overpayment noise until the objective knows what the attacker spent.

**This is structural, not a missing cost term.** The leak objective assumes a
**custody-shaped** protocol — value must stay in the box — and `sanctioned`
is a *declared static amount* on a free payee, so it cannot express
"sanctioned **iff** the payment condition holds". For **exchange-shaped**
protocols (sales, orders, swaps where assets legitimately leave against
payment) the honest transaction is indistinguishable from a drain: the goods
leave the protected box (counted as leak) and the payment lands in a box that
is not script-matched to it (counted as nothing).

That is exactly why the token-sale original fires and
`the_gallery_amm_pair_is_not_drainable` does not: the pool keeps its reserves
in a script-matched successor, so `protectedOut` stays high. Change the shape
from custody to exchange and the objective inverts.

The consequence reaches past this corpus: **`drainable` verdicts on order- or
sale-shaped sets are not trustworthy today**, and the mapped USE/Dexy sets
carry swap and order boxes. They return `notUnderProbes` at present, so
nothing in CI is wrong — but a continuous sweep (roadmap phase 6) over
exchange-shaped protocols would produce this noise at scale, and a
disclosure pipeline fed by it would be reporting non-findings. Tracked
separately; until a conditional-sanction model exists, an exchange-shaped set
deserves a flag rather than a silent score.

What the run actually showed, beyond the number:

- **M4 is found, and only with synthesis on** (4,512 probes vs 28): the
  drain needs a NEW output — the attacker's payment box at the sale's price
  position — while the stock returns to the sale box. Phase 1's fixed
  output set cannot express it. This is the phase-2 degrees earning their
  place, measured.
- **M5, M6, M7 are missed, and all three miss the same way**: the witnesses
  re-pad or re-tree a **fixed (template) output** — the bank successor's
  token layout, the pool successor's script, the swap successor's script.
  Phase 2's synthesis adds sinks and companion re-creations but never
  modifies a template output's tokens or tree; phase 1's output shape is
  fixed by definition. **The missing probe axis is "re-pad / re-tree a
  fixed output"** — phase-3 backlog entry #1, and the likely conversion of
  all three misses. The axis, not the budget: M7 spends 29,448 probes in
  family and misses.
- **The keyed-insider class (M1–M3) is invisible by construction.** The
  hunt refuses `needsProof` on protected inputs — correctly, under its
  keyless attacker model — so a whole defect family (insider drains,
  beneficiary overdraws) is dark to it. Extending the attacker model is
  phase-3+ work with a different oracle story (signed witnesses).

A rate of 0.00 on the first pass is the finding, and it ships as the
finding. The one raw hit is confounded rather than promising: it is the
corpus's own control telling us the objective, not the mutation, produced
the verdict. The corpus is not tuned until it looks good; the miss table below
is the hand-off to phase 3.

## What this corpus does NOT cover

Stated explicitly, because the rate is only as honest as its boundaries:

- **Register-driven defects.** No mutant turns on a register value
  (`R4` contents, oracle readings). The decoy family clears registers but
  never varies their values; `usePayout`-shaped bugs are unreachable by
  construction. Arrives with phase 3's register axis.
- **Data-input / oracle defects.** Data inputs are pinned `external` and
  passed verbatim; oracle-manipulation mutants (price stuffing, oracle
  substitution) are not in the corpus. Arrives with phase 3's data-input
  axis.
- **Multi-transaction defects.** One transaction per hunt; a mutant
  exploitable only after a setup spend (moving an NFT into position,
  seeding a box) is unprovable here. Phase 5.
- **Arithmetic-only drains.** No proven mutant here requires a computed
  constant (an exact CPMM amount, an exact fee boundary). That is
  deliberate: phase 4's exit criterion is *one such mutant found*; seeding
  it now would make the phase-4 claim unfalsifiable.
- **Signature/malleability, memory safety, fee/UTXO-set economics** — the
  oracle models none of these; mutants of that class would be unverifiable.

## The miss table (the hand-off to phase 3)

| class | mutants | what it implies |
|-------|---------|-----------------|
| **out-of-family** | M5, M6, M7 | one missing probe axis: **re-pad / re-tree a fixed (template) output**. Phase 2 pads companion re-creations only; the successors of protected boxes are untouchable. This is the single highest-yield phase-3 entry — it plausibly converts all three misses. |
| **keyed-insider (attacker model)** | M1, M2, M3 | the hunt's keyless model cannot see insider drains (beneficiary overdraws, receiver pot drains). Extending the attacker model needs signed-witness machinery — phase-3+ design work, not a coverage knob. |
| **cap-truncation** | none | no proven mutant was cut off by the budget at 50k probes; M7's 29,448 in-family probes were exhaustive-in-family, which is what makes its miss an *axis* miss and not a cost miss. |
| **role-labelling** | none | the recorded labellings (incl. the synthetic singletons on M1–M3's token-less boxes and M5's de-attacked template) did not hide any route — with one caveat: the M5 template originally carried the attack shape (junk at `tokens(0)`, NFT pre-declared leaving) and was corrected *before* the run; that iteration is recorded here, in the open. |
| **arithmetic** | none | deliberately: every witness uses amounts the honest template or plain arithmetic produces. The arithmetic mutant is phase 4's exit criterion and must not be pre-seeded. |

## What the corpus does NOT cover

Stated explicitly, because the rate is only as honest as its boundaries:

- **Register-driven defects.** No mutant turns on a register value (`R4`
  contents, oracle readings). The decoy family clears registers but never
  varies their values; `usePayout`-shaped bugs are unreachable by
  construction. Arrives with phase 3's register axis.
- **Data-input / oracle defects.** Data inputs are pinned `external` and
  passed verbatim (the bank mutant's oracle is pinned and the witness uses
  it honestly); oracle-manipulation mutants (price stuffing, oracle
  substitution) are not in the corpus. Arrives with phase 3's data-input
  axis.
- **Multi-transaction defects.** One transaction per hunt; a mutant
  exploitable only after a setup spend is unprovable here. Phase 5.
- **Arithmetic-only drains.** No proven mutant requires a computed
  constant. Phase 4's exit criterion; must not be pre-seeded.
- **Keyed-insider drains** as *findable* targets: M1–M3 are in the corpus
  as classified exclusions, not as rate entries.
- **Signature/malleability, memory safety, fee/UTXO-set economics** — the
  oracle models none of these.
