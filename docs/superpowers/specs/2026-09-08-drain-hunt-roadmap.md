# Drain hunt — roadmap beyond phase 2

> Planning record, 2026-09-08. Sits above the per-phase design records
> (`2026-09-08-drain-hunt-phase1-design.md` #61, implementation #64;
> `2026-09-08-drain-hunt-phase2-design.md` #65, in flight) and the protocol
> map (`2026-09-08-protocol-map-design.md` #62/#63). It does not re-specify
> phase 2. It answers one question the per-phase specs never ask: **what has
> to be true for this to find bugs nobody has shown it, and in what order do
> we make those things true?**

## Where we actually are

- **The map** (#63) turns one seed box into a labelled contract set with
  typed edges — the input side of every hunt.
- **The lints** (`audit/lints/`, two of them) read scripts statically and
  flag known shapes; `unbound-box-reserves` is the USE class.
- **The spend hunt** (`hunt.rs`) asks of one box: can anyone spend it?
- **The drain hunt** (`drain.rs`, #64) asks of a set: can anyone extract
  value? Phase 1 varies input order, attacker-slot contents and free-payee
  recipients. Phase 2 (#65, implemented #67, #68) adds bounded output
  synthesis.
- **The scoreboard** (#70) — the mutation corpus, phase 2.5 below. As of
  2026-09-08 it reads **0 attributable detections of 4 proven mutants**.

That is a real instrument and it rediscovered a real 284,695 ERG drain. The
roadmap's premise was a limitation rather than a complaint — and phase 2.5
has now turned that premise from an argument into a measurement:

**The evidence that the hunt generalizes was one incident — and the first
attempt to widen it returned zero.** The family it
enumerates was drawn from that incident — `filler tokens 0..=3` exists
because the USE attacker used three junk tokens so `tokens(1)`/`tokens(2)`
would exist. The generator is honestly blind to the target (`decoy_variants`
takes only the attacker's own box and tree, `drain.rs:723`), so it is not a
tautology; but blindness to the target is not the same property as coverage
of the bug space. And the negative controls do not correct for it: a hunt
that found nothing anywhere would pass `the_gallery_amm_pair_is_not_drainable`
identically. Every phase below is ordered by that.

## Phase 2.5 — the measurement phase (DONE, #70)

**The problem it solves.** We cannot currently answer "did that change make
the hunt better?" Every phase after this one is a bet placed without a
scoreboard.

**The work.** A mutation corpus. Take contracts we believe sound — the
gallery AMM pair, `examples/incidents/fixed/use-lp-swap.es`, the Dexy bank
family under `examples/contracts/dexy/`, and a sample of the 110 contracts
in `examples/contracts/` — and inject **one** binding defect each,
mechanically and reproducibly:

- delete an `NFT ==` identity check;
- flip an index constant (`INPUTS(0)` → `INPUTS(1)`, `tokens(0)` →
  `tokens(1)`);
- replace a positional reference with an unbound search (`INPUTS(0)` →
  `INPUTS.exists(...)`) and the reverse;
- drop a `propositionBytes ==` successor check;
- weaken a comparison (`>=` → `>`, `==` → `>=`) on a reserve guard.

Each mutant ships with the role labelling and template the hunt needs, and
an expected verdict. The suite reports a **detection rate**, not a pass/fail.

**Why it goes before phase 3.** Coverage-guided steering is an optimisation,
and we currently have no way to observe what it optimises. The mutation
corpus is also the only artefact that distinguishes "finds bugs" from "found
the bug it was shown" — the question a third party will ask first.

**Parallel-safe.** It adds fixtures and a harness; it does not touch
`drain.rs`. It can land while phase 2 is in flight, in a separate worktree,
with no merge surface beyond a new test file and `examples/`.

**Exit criteria.** A published detection rate on ≥20 mutants across ≥5
protocol families, with each miss classified by *why* (out of family, cap
truncation, role labelling, arithmetic). The misses are the phase-3 backlog,
and they are worth more than the hits.

### What it actually returned (#70, merged 2026-09-08)

Seven mutants over six contracts — short of the ≥20 the exit criteria asked
for, so the numbers below are a first pass, not a coverage claim.

**0 attributable detections of 4 proven mutants.** One raw `drainable`
verdict (M4), **confounded**: the same verdict fires on the unmutated
original, so it cannot be attributed to the mutation. Three of the five
mutation operators are represented; `flip-index` has no proven mutant and
`positional ↔ unbound-search` — the USE incident's own class — has none at
all.

Three findings came out of it, and they reorder everything below:

1. **One missing probe axis explains three of the four misses** (#72).
   Phase-2 synthesis adds outputs but never modifies a *declared* one:
   template outputs' token layouts and trees are fixed for the whole probe
   space. M5, M6 and M7 all want a re-padded or re-treed template output.
   M7 ran **29,448 probes in family and missed** — an axis gap, not a
   budget gap, and steering would not have helped.
2. **The leak objective inverts on exchange-shaped protocols** (#71).
   It assumes custody — value must stay in the box — and `sanctioned` is a
   declared static amount, so it cannot say "sanctioned *iff* the payment
   condition holds". For sales, orders and swaps, an honest transaction is
   indistinguishable from a drain. This is why M4's control fires.
3. **The keyless attacker model is a real boundary, not an oversight.**
   Three mutants turned out to need the beneficiary's or receiver's key.
   They are reported outside the denominator, and a keyed-insider hunt is
   its own project with a different oracle story (signed witnesses).

**Methodological note worth carrying forward.** Five separate defects in
that PR were recorded policies that nothing enforced: a negative-control
loop that silently no-opped, control assertions that compared the mutant
against itself, probe caps read from a null field, an unchecked synthesis
policy, and an unpinned headline count. **If an artifact states a policy,
the harness must read it.** That rule is cheap and it has already paid for
itself more than any single axis in this document.

**Secondary yield.** Every mutant the hunt misses but a static reader would
catch is a candidate lint; every mutant the lints catch and the hunt misses
tells us the two layers are complementary rather than redundant, which is
currently an assumption.

## Phase 3 — the axes, then the objective, then steering

**Reordered after phase 2.5 (2026-09-08).** This section originally led with
coverage-guided steering, on the reasoning that synthesis multiplies the
space and guidance makes it tractable. The corpus says otherwise, and the
evidence is specific: M7 ran 29,448 probes **inside its family** and missed,
because the witness needs a shape the generator cannot build. Steering makes
a search of a complete family faster; it does not add a missing axis, and it
does not repair an objective that scores honest transactions as drains.
Search efficiency was not the constraint. It is third here now, and it has to
earn its place against a measurable rate rather than against intuition.

**3a. The missing output axis — re-pad / re-tree a declared output** (#72).
Three of four proven misses want exactly this, and the corpus can measure
whether it converts them. Filler sourcing, script-independence and the
budget accounting from #67/#68 all carry over unchanged. Highest expected
return in this document, and the cheapest to state.

**3b. The objective's custody assumption** (#71). `drainable` verdicts on
sale-, order- and swap-shaped sets are not trustworthy today, and the mapped
USE/Dexy sets carry order boxes. This blocks the corpus from ever scoring a
whole protocol family, and it would feed non-findings into a phase-6 sweep's
disclosure pipeline. Conditional sanction is the real requirement; attacker-
cost accounting alone suppresses the symptom while hiding an attacker who
profits at a bad price.

**3c. Two axes the incident-derived family left out** — both bug classes,
not performance knobs:

1. **Register variation.** Today the only variant is `registers cleared`
   (`drain.rs:760`); register *values* are never varied. Our own flagship
   target turns on one (`usePayout`'s `HEIGHT == SELF.R4 + 5040`). Any
   protocol keyed on R4 contents is currently unexplored by construction.
2. **Data inputs.** Passed through verbatim (`drain.rs:895`) and pinned as
   `external` by role. That makes the whole oracle-manipulation class
   out of scope — defensible for phase 1, indefensible as a permanent
   boundary, because an attacker who can choose *which* oracle box to
   reference often can. Varying data inputs across the boxes the map
   actually found is bounded and honest.

**3d. Steering, last.** `hot_spots.rs` already folds the per-step cost trace
into a ranked view; the feedback signal exists. The steering rule stays
subordinate to the honesty rule: a guided miss is still `notUnderProbes`,
and the report must say the space was steered rather than enumerated,
because those are different claims. Do this after 3a–3c, and only if the
corpus then shows probe budget — rather than expressiveness — as the binding
constraint. Right now nothing in the evidence says it is.

**Corpus debt to clear alongside.** The scoreboard is thinner than its exit
criteria: seven mutants where ≥20 was asked, and three of five operators.
`flip-index` needs a keyless mutant and `positional ↔ unbound-search` needs
any mutant at all — it is the USE incident's own class, and its absence is
the most conspicuous hole in the denominator. Register and data-input mutant
classes arrive with 3c.

**Exit criteria.** The attributable rate on the phase-2.5 corpus improves
from 0/4, measured before and after each of 3a–3d separately so the credit
is attributable to a change rather than to the batch. If steering does not
move the rate, that is a bet we can now prove we lost — say so, and keep the
axes.

## Phase 4 — solver-assisted values (the qualitative jump)

Everything through phase 3 finds bugs reachable by **rearranging declared
material**. A drain that needs a *computed* value — a swap amount that
satisfies constant-product arithmetic to the unit, a fee that lands exactly
on a boundary — is unreachable by enumeration at any cap, and steering does
not invent constants. The USE drain was findable because the attacker's
shape was a permutation of the honest one; the next one may not be.

**The work.** Extract path constraints from the reduction the sandbox
already traces, and solve for the values that satisfy a branch, rather than
sampling them. Concolic minimisation is already parked here by the phase-2
spec; this is the same machinery pointed at generation instead of shrinking.

**This is the phase that changes what the tool is** — from a rearranger to a
finder. It is also the largest single piece of work in this document, and
the most likely to be descoped to "solve for one numeric field at a time",
which would still be worth having.

**Exit criteria.** One arithmetic mutant in the corpus — a weakened
comparison on a reserve guard, where the witness needs a specific amount —
found by the hunt with no hand-fed constant.

**Honest risk.** This may be where the project meets its ceiling. Ergo's
reduction is not trivially SMT-encodable, and a partial encoding that
silently loses constraints would produce witnesses that fail the oracle —
noisy, but not unsound, because `txcheck` remains the arbiter. Prototype
against the corpus before committing to it.

## Phase 5 — multi-transaction chains

Parked by the phase-2 spec as "phase 3+", and correctly out of scope for
the vault question. But most staged drains need a setup spend — moving an
authorizer NFT into position, seeding a box the next transaction reads. A
one-transaction hunt cannot see them, and says `notUnderProbes` with a
straight face.

**The work.** Chain depth 2 first, with the oracle applied per transaction
and the leak objective measured across the chain. The state explosion is
real: the phase-1 caps are already load-bearing at depth 1. Expect this to
depend on phase 3's steering being genuinely effective — if steering did not
move the detection rate, depth 2 is premature.

**Exit criteria.** A depth-2 mutant (a defect only exploitable after a setup
spend) detected, and the report naming the chain.

## Phase 6 — autonomy and scale

Only meaningful once the detection rate is known and non-trivial. Three
pieces, in order:

1. **Role inference with confidence.** Roles are caller-supplied today, and
   `protocolNfts` is the caller's claim — whoever labels the set determines
   what is even askable, and a mislabelled companion is a silently
   unexplored route. The map (#63) has the edge evidence to propose roles;
   what it lacks is a confidence signal and a rule for when to refuse. Keep
   `unknown` refused: the hunt never guesses.
2. **Continuous sweep.** Map from seeds, hunt on a schedule, diff verdicts
   across heights. The interesting signal is a set that *becomes* drainable
   — a deployment or a parameter change that opens a hole.
3. **Disclosure pipeline.** The phase-1 rule (disclosure-first, corpus entry
   only post-fix or on an agreed window) is a paragraph in a spec. At sweep
   scale it needs to be machinery: findings quarantined by default, a
   private report path, and no public artefact until the window opens. Build
   this **before** the sweep, not after the first live hit.

**Exit criteria.** A finding on a set nobody nominated by hand, disclosed
through the pipeline rather than around it.

## Cross-cutting invariants

These hold in every phase; a change that breaks one is a redesign, not an
increment.

- **The oracle is `txcheck::check`, always.** A hit is a mineable
  transaction or it is not a hit. No heuristic verdicts.
- **The generator never reads the tree under hunt.** Copying script bytes
  verbatim (phase 2's companion re-creations) is fine; branching on their
  content is not. This is the anti-cheat, and it is currently enforced by
  the shape of the function signatures — keep it that way.
- **A miss is never "safe".** `notUnderProbes` names the probe space, the
  caps and the truncation order.
- **Truncation is visible.** Once synthesis multiplies the space, the
  normal case is a sampled prefix; a report that hides it is lying by
  omission.
- **Disclosure-first on third-party sets.**

## What this roadmap does not claim

That the phases are independent — 4 depends on 2.5 for its exit criteria,
5 depends on 3 having worked. That the order is fixed — a live incident
re-orders everything, on the 24-hour rule, and phase 2.5's own results
already reordered phase 3 once (steering demoted from first to last). This
document is expected to be rewritten by evidence; that is what having a
scoreboard is for. That the ceiling is known: phase
4 may prove the arithmetic class is out of reach with this architecture, in
which case the honest outcome is a documented boundary, and a tool that is
excellent at binding bugs and explicitly silent on value bugs.
