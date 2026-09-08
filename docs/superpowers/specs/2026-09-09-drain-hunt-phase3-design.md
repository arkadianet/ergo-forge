# Drain hunt phase 3 — the axes, then the objective, then steering

> Design record, 2026-09-09. Follows phase 1 (#61, implemented #64) and
> phase 2 (#65, implemented #67, corrected #68), and is ordered by what the
> mutation corpus (#70) measured rather than by what the roadmap (#66)
> originally guessed. **Design doc — no implementation.** Every number below
> is measured against merged `main` and says which set it was measured on;
> anything unmeasured is marked as an estimate.

## Why this order, and why it changed

The roadmap led phase 3 with coverage-guided steering: synthesis multiplies
the space, guidance makes it tractable. The corpus falsified that as a
*first* priority, specifically and cheaply:

- **M7 ran 29,448 probes inside its family and missed.** The witness needs a
  shape the generator cannot construct. No amount of steering reaches a
  point that is not in the space.
- **The one raw `drainable` on a proven mutant was confounded** — the same
  verdict fires on the unmutated original, because the objective scores an
  honest purchase as a drain.

Steering makes a search of a *complete* family faster. Phase 3's evidence
says the family is incomplete and the objective is wrong on a whole protocol
shape. So: axes first, objective second, steering last, and steering only if
the corpus then shows probe budget — rather than expressiveness — as the
binding constraint.

The scoreboard is the arbiter throughout. **Each sub-phase is measured
separately** against `examples/mutants/answer-key.json`, so credit attaches
to a change rather than to a batch.

## 3a — the missing output axis: re-pad / re-tree a declared output

**The gap** (#72). Phase-2 synthesis *adds* outputs; it never modifies one
the caller declared. In `drain.rs`, padding always constructs a new box
(`new_boxes.push` in `materialize_synthesized`), and the only mutations
applied to template outputs are the drain-mode successor shrink (value →
`DRAIN_KEEP_VALUE`, token amounts → 1) and the free-payee re-tree. A
declared output's **token layout** and **ergoTree** are therefore fixed for
the entire probe space. A real attacker has no such restriction.

Three of the corpus's four proven misses want exactly this: M5 re-pads a
successor's token layout so junk takes `tokens(0)`; M6 and M7 re-tree a
successor to the attacker's tree.

**The family.** For each declared output, one of:

- `verbatim` — today's behavior;
- `re-treed` — the output's script replaced by the attacker tree;
- `re-padded(f)` — `f ∈ 1..=3` sourced filler tokens inserted before the
  output's first token, shifting its layout right.

**Decision: at most one declared output is modified per probe.** Not the
powerset. Three reasons, in order of weight: it matches the single-defect
hypothesis the corpus itself is built on (one mutation per mutant, one
witness per mutant); it keeps slices thick enough to mean something (below);
and every miss this axis is meant to convert needs exactly one modified
output. If a two-output witness ever turns up, that is a finding and a
follow-up, not a reason to buy the powerset up front.

**Filler sourcing carries over unchanged.** Conservation
(`txcheck.rs:296`) rejects any output token id the inputs do not carry
except the single mint id, so a re-padded declared output sources its
fillers from attacker inputs exactly as companion re-creations do. An
unsourced probe must not be generated — a `notUnderProbes` that means
"conservation blocked us" is the failure phase 2 spent two review rounds
closing.

**Script-independence is preserved.** Re-treeing to the attacker tree and
inserting sourced fillers are both blind operations. The generator still
never branches on the content of the tree under hunt.

**Accounting needs no change.** Decision 2's NFT ride-along rule (#67)
already says a script-matched output shields only when the protected input's
NFT rides along. A re-treed successor is no longer script-matched, so it
stops shielding and the value scores as leak — which is precisely why M6 and
M7 should convert. A re-padded successor that loses its NFT from `tokens(0)`
is already reported as `nftDetached`.

**Placement in the pinned axis order.** Axis (1) is synthesized-output
shapes. Declared-output modifications join it, **before** companion
re-creations:

```
none → declared-output modifications → companion re-creations → sinks
```

Ordering is a truncation decision, and the evidence is one-sided: this axis
has three misses waiting on it, while companion re-creations have converted
nothing measured so far. If that changes, so does the order.

### Budget arithmetic (measured on merged `main`)

The mapped USE set (`request_from_map` over `use-lp.json`) is **12 inputs,
4 outputs**, and today produces **19 shapes / 67 tally buckets** with the
phase-2 degrees on, against a **slice floor of 16**. Probe cost on that set
is **8.31 ms/probe** (release; 2,000 probes in 16.6 s).

3a adds `4 outputs × 4 variants = 16` shapes → **35 shapes** (+84%). At the
50,000-probe default that is ~1,428 probes per slice against a floor of 16 —
comfortable — and ~7 minutes single-threaded on that set.

The powerset alternative would be ~600 shapes: still above the floor at ~83
probes each, and the wall clock is unchanged because the run is cap-bound,
but per-shape coverage collapses by 17×. The corpus's own contracts are
2–3 outputs, where the axis costs 8–12 extra shapes and nothing else.

Report additions: the new shape labels are already carried by
`ShapeTally`; `capacity`/`sliceFloor` (#68) state the cost honestly without
new machinery.

## 3b — the objective's custody assumption

**The gap** (#71). `leak()` computes `protectedIn − protectedOut(script-
matched) − sanctioned(declared free-payee amounts)`. `sanctioned` is a
**declared static amount**, so the objective cannot express *"sanctioned
**iff** the payment condition holds"*. It assumes a **custody-shaped**
protocol: value must stay in the box. For **exchange-shaped** protocols —
sales, orders, swaps where goods legitimately leave against payment — the
honest transaction is structurally indistinguishable from a drain.

**Measured, on the corpus's token-sale (`pricePerToken` = 1,000,000):**

| run | payment to seller | extracted | verdict |
|---|---|---|---|
| unmutated original | 21,000,000 (overpay) | 10 tokens + 1,000,000 nanoErg | `drainable`, 6 hits |
| mutant (price guard deleted) | 3,000,000 (underpay) | 10 tokens + 1,000,000 nanoErg | `drainable`, 710 hits |

**The extraction is identical.** That kills the cheap fix before it is
proposed: *subtracting what the attacker spent does not work*, because the
assets differ — the attacker spends nanoErg and extracts tokens, and netting
across assets requires a price, which is the parked worth-weighting spec.
Per-asset netting would clear the nanoErg component and leave the 10-token
leak untouched, so the control would still fire.

**Decision: conditional sanction, declared by the caller.** A request may
declare exchange terms; sanctioned outflow becomes a predicate over the
realized transaction rather than a fixed number. Sketch:

```json
"sanctions": [{
  "releases":   { "fromInput": 0, "assets": ["cccc…", "nanoErg"] },
  "paidTo":     { "output": 0 },
  "atLeast":    { "asset": "nanoErg", "perUnitOf": "cccc…", "rate": 1000000 }
}]
```

Read: *the protected input's stock and value are sanctioned to leave when
output 0 pays the seller at least 1,000,000 nanoErg per token taken.*
Proportional, because the honest condition is a rate — a fixed minimum
cannot express "pay for what you take".

Two properties this must keep:

- **The rate is the caller's claim, exactly like `protocolNfts`.** It is
  declared or map-fed; the hunt never reads the target script to derive it.
  A wrong declaration produces a wrong verdict, and that is the caller's
  error, recorded as such — the alternative is the generator reading the
  contract, which is the anti-cheat.
- **Undeclared exchange sets are flagged, not silently scored.** Until a
  set carries sanctions, `drainable` on an exchange-shaped set should be
  reported with an explicit caveat. Otherwise a phase-6 sweep feeds
  non-findings into a disclosure pipeline, which is the expensive form of
  this bug.

**Acceptance.** With the term declared: the unmutated token-sale returns
`notUnderProbes` (21,000,000 ≥ 10 × 1,000,000 — sanctioned), the mutant
returns `drainable` (3,000,000 < 10,000,000 — not sanctioned). M4 becomes
**attributable**, and the corpus's first real detection is real.

## 3c — the two axes the incident-derived family left out

Unchanged in substance from the roadmap; both are bug classes, not knobs.

1. **Register variation.** Today the only variant is `registers cleared`
   (`drain.rs:760`); values are never varied. The vault's own `usePayout`
   turns on one (`HEIGHT == SELF.R4 + 5040`). The declared finite family:
   values observed on the declared boxes and on the map's nodes, plus
   `HEIGHT`-relative offsets drawn from the request's own height. Never a
   value read out of the target script.
2. **Data inputs.** Passed through verbatim (`drain.rs:895`) and pinned as
   `external` by role, which puts oracle manipulation out of scope by
   construction. The honest bound: an attacker chooses **which** oracle box
   to reference, not what it says. So the family is *substitution across the
   boxes the map actually found*, never fabrication of oracle contents.

Both need mutant classes added to the corpus in this sub-phase, or their
effect is unmeasurable — the same rule that put 2.5 before 3.

## 3d — steering, last

`hot_spots.rs` already folds the per-step cost trace into a ranked view, so
the feedback signal exists.

**What steering may see.** The anti-cheat is that the *generator* never
branches on the content of the tree under hunt, and it is currently enforced
by function signatures. Coverage feedback is derived from *executing* the
tree, which is a post-hoc signal over cost/coverage, not a read of script
content: the generator still cannot ask "what does this script check?" — it
can only be told "that probe was cheap/expensive, this branch was reached".
That distinction is the whole justification, and it must be stated in the
implementation's module docs, not just here.

**Verdict vocabulary: unchanged, with the parameters recorded.**
`notUnderProbes` means "the declared space did not reach it". Under steering
the space is still declared — a seed, a schedule, a budget — it is merely
*traversed* adaptively. So no new verdict; instead the report must record
that the space was steered, with the seed and schedule, because a steered
miss and an enumerated miss are different claims and a reader cannot tell
them apart from the verdict alone.

**Gate.** Do 3d only if, after 3a–3c, the corpus shows misses classified as
*cap-truncation* rather than *out-of-family*. Today that column is empty.

## Corpus debt to clear alongside

The scoreboard is thinner than its own exit criteria and this spec depends
on it: **7 mutants where ≥20 was asked**, three of five operators
represented, `flip-index` with no proven mutant, and
`positional ↔ unbound-search` — the USE incident's own class — with no
mutant at all. Plus the register and data-input classes 3c needs. Widening
the corpus is not optional decoration here; every acceptance in this
document is a corpus measurement.

## Non-goals, recorded

- **Solver-assisted values** — roadmap phase 4. Enumeration and steering
  cannot invent a computed constant; a drain that needs one is unreachable
  at any cap, and this document does not pretend otherwise.
- **Multi-transaction chains** — phase 5.
- **The keyed-insider attacker model.** Three corpus mutants need the
  beneficiary's or receiver's key. That is a different oracle story (signed
  witnesses) and its own project, not a coverage fix.
- **Pricing / worth-weighted objectives.** 3b's conditional sanction is
  deliberately *not* pricing: it compares declared terms against the
  realized transaction. If it proves insufficient, worth-weighting is the
  next lever — but it is not needed for the measured confound.
- **Continuous sweep, role inference** — phase 6, and gated on 3b, since a
  sweep over exchange-shaped sets without conditional sanction produces
  noise at scale.

## The standing rule this document inherits

From #70's own defects — five recorded policies that nothing enforced
(a control loop that no-opped, assertions comparing a mutant to itself, caps
read from a null field, an unchecked synthesis policy, an unpinned headline):
**if an artifact states a policy, the harness must read it.** Every cap,
every declared sanction and every steering parameter this spec introduces is
part of the measurement, and must be validated and pinned to the answer key
rather than recorded for a reader.

And the phase-2 lesson that produced three rounds of review: **every
property claimed here must be assertable against `request_from_map` output,
not a hand-built fixture.** If a property can only be demonstrated on a
fixture the author constructs, the property is about the fixture.
