# Drain hunt phase 3 — objective gate, output axes, then steering

> Design record, 2026-09-09. Follows phase 1 (#61, implemented #64) and
> phase 2 (#65, implemented #67, corrected #68). **Design doc — no
> implementation.** Historical corpus observations are distinguished from
> proposed acceptance criteria and budget estimates. This revision does not
> claim a new corpus run or change the separately maintained baseline.

## Objective decision (phase-3 gating)

**Choose C: unauthorized, victim-funded receipts in recognized
attacker-spendable outputs.** Measure assets the attacker can take beyond
their own funding and the releases the protocol's declared terms authorize.
Victim holdings include both `Protected` and `Companion` inputs. A companion
role describes how a box authorizes the transaction; it does not mean that
the box's owner donated its assets to the attacker.

This combines B's destination requirement with A's explicit asset scope and
#71's conditional sanctions. It replaces `leak()`'s custody deficit as the
`drainable` objective. Custody deficits and `nftDetached` remain separate
diagnostics; neither alone establishes attacker extraction.

**Why this choice.** A custody deficit, even with conditional sanctions,
can still call an underpaid transfer of all stock back to the seller a drain.
That measures a violated release condition, not a receipt by the attacker.
Conversely, B alone calls an honest, fully paid purchase a drain: the buyer
really can spend the purchased tokens. It can also count the attacker's own
change. Requiring a recognized destination, accounting for funding, and
excluding authorized releases addresses all three cases without assigning
market prices to assets. Destruction, gifts to unrelated recipients, and
assets parked in scripts whose spendability is unknown are outside this
extraction objective; report any custody loss separately.

**Accounting contract, per asset.** For each valid realized transaction:

- `V[a]`: holdings of asset `a` in all declared `Protected` and `Companion`
  spending inputs, counted once. Pin this scope before probing, independently
  of input permutation and of which input carries the mutated script. Data
  inputs contribute nothing. Do not relabel M7 to make it score.
- `A[a]`: holdings in realized outputs recognized as attacker-spendable,
  counted once regardless of output position, template payee label, or
  whether the output was synthesized. A fixed successor re-treed by #72 is
  eligible; a free-payee label by itself proves no spendability.
- `N[a]`: all non-victim input holdings plus newly minted supply of `a`.
  Use realized attacker funding after decoy substitution, not the template
  amounts. Counting all outside funding conservatively prevents attribution
  of another source's assets to the victim. Data inputs are not funding.
- `S[a]`: an upper bound on authorized victim-funded receipts in those
  attacker outputs, obtained from explicit static release allowances or
  satisfied conditional terms (#71). Bind allowances to victim sources,
  assets, destinations and realized quantities; cap them by available
  holdings and deduplicate overlapping claims. A template's declared free
  amount is no longer an implicit blanket sanction.

The score is the conservative per-asset lower bound:

```text
leak[a] = min(V[a], max(0, A[a] - N[a] - S[a]))
```

Attribute ambiguous fungible funding to non-victim sources and authorized
receipts first. This can undercount theft when sources share an asset; it
must not invent provenance. Report `V`, `A`, `N`, `S`, the recognized output
indices, and the terms used alongside `extracted`. Keep assets separate:
spending ERG cannot cancel a stolen NFT or stock token without an explicit
exchange term. This is an authorization measurement, not profit or market
value. One positive asset is sufficient, subject to the existing transaction
and missing-key gates.

**Spendability is a bounded recognition rule, not a general oracle claim.**
Phase 3 recognizes the canonical unconditional `sigmaProp(true)` sink and
exact standard P2PK scripts for keys explicitly declared held by the
attacker. Such key ownership is an attacker-model assumption, recorded with
the request, as for the attacker's funding inputs; no private key or signing
is needed to compare the public-key script. The unsigned reducer's
`needsProof` does not establish possession. Matching an arbitrary supplied
`attackerTree` is insufficient: reject unsupported trees for scored sinks,
or record their spendability as unknown and exclude them from `A`.
Arbitrary script spendability depends on a future spending context and is
not decided by the oracle validating the transaction that creates the box.
Protected and companion inputs must still reduce to `pass`; accepting a
declared attacker P2PK sink does not permit beneficiary keys on those inputs.

The consequences below are acceptance obligations, **not measured new
detections**:

| Case | Consequence of the decision | Required detection evidence |
|---|---|---|
| M4 — price guard deleted | The recorded seller-only winning witness scores zero on mutant and original: the attacker receives nothing. A fully paid purchase into the buyer's sink is sanctioned; an underpaid purchase into that sink can score token extraction. | Re-prove M4 with an actual attacker receipt and rediscover it from an honest template. Require the original clean under identical terms and caps. The old seller-only witness and 710 raw hits do not establish this. |
| M5 — delete NFT check | The bank NFT reaching the attacker scores one unsanctioned token. A same-script successor with displaced identity is only a diagnostic unless assets actually reach the attacker. | Express the token-slot replacement and NFT transfer; sanction any legitimate redemption separately. Find the mutant with its original clean. |
| M6 — drop successor script | Full pool reserves in a recognized attacker sink score after funding and authorized releases are excluded. | #72 must re-tree the declared successor while preserving its required holdings; the original must reject the theft. |
| M7 — drop swap successor script | The companion's 1,000,000,000 nanoErg and swap NFT enter `V`; its re-treed successor enters `A`. The preserved pool contributes no attacker receipt. | Both companion accounting and a constructible swap successor are necessary. Re-treeing alone under today's objective still cannot score the stolen companion assets. Require mutant found and original clean. |
| Phase-1 USE rediscovery / C-incident | Pool reserves reach the attacker beyond their funding, while the tiny pool successor and preserved swap companion remain outside `A`. Extraction stays positive. | Mandatory `drainable` anchor with synthesis off and the new objective, plus its fixed-contract negative control. Keep C-incident outside the corpus denominator. |

**Gate:** implement and validate this objective contract (3b) before claiming
conversions from 3a. Widen and re-prove the corpus as specified below before
accepting any phase-3 sub-phase. Objective changes require a versioned
baseline; a changed definition must not be presented as generator progress.

## Why this order, and why it changed

The roadmap led with coverage-guided steering. The corpus instead exposed
an incomplete family and an inadequate objective. Historically M7 ran
29,448 probes and missed; its output shape was unavailable, and its stolen
companion assets were also outside the accounting scope. More budget fixes
neither. M4's raw `drainable` also fired on the original, with identical
custody-deficit extraction and no attacker receipt.

The order is therefore **corpus and source-fidelity prerequisites, objective
(3b), output expressiveness (3a), further axes (3c), steering only if
justified (3d)**. Source preservation is specified in 3c but must land before
any earlier acceptance that depends on mapped registers. The labels retain
their roadmap correspondence.
Each change gets a separate measurement against
`examples/mutants/answer-key.json`: first rescore witnesses under the new
objective, then run the unchanged family, then add each axis. Distinguish
an objective miss, a source-fidelity failure, an out-of-family witness, and
cap truncation. Do not credit a whole batch to #72.

## 3a — the missing output axis: re-pad / re-tree a declared output

**The gap** (#72). `materialize_synthesized` adds boxes with `new_boxes.push`;
it does not provide general tree or token-layout edits of fixed-payee
template outputs. Their trees and token-id layouts stay fixed, although
drain mode can shrink successor values and token amounts. This restriction
does **not** apply to the first free payee: `realize_outputs` rebuilds its
tokens from the remainder and replaces its tree in drain mode.

M6 motivates fixed-successor re-treeing. M5 motivates changing the NFT slot
and sending the displaced NFT to the attacker. M7 additionally needs the
objective gate and an honest swap continuation that the family can modify;
withdraw the earlier claim that one output axis alone converts all three.
`request_from_map` currently declares successors only for `Protected`
inputs. For phase 3, also declare verbatim companion continuations as fixed
outputs, with their identities and state preserved. These become ordinary
targets of the edit axis; include their additional construction cost in the
mapped measurement. This is a prerequisite to a mapped M7 conversion, not
proof of one: a repaired hand-built template cannot demonstrate that the
map path supplies a valid honest shape or the required output ordering.

**The family.** Target each fixed-payee declared output with one of:

- `verbatim` — baseline, no edit;
- `re-treed` — replace its script with a recognized attacker sink;
- `re-padded(f)` — insert `f ∈ 1..=3` sourced filler tokens before its
  first token, retaining the existing token sequence;
- `first-token-replaced` — replace the first token with one sourced filler,
  preserve the remaining positions, and return the displaced token to the
  conservation remainder for an attacker payout.

The last variant is necessary for an M5 claim: its witness replaces the
NFT at `tokens(0)` while retaining a three-token successor. Prefix insertion
alone increases the token count and shifts the treasury positions, so it
does not reproduce that witness. Both operations are generic token-layout
choices; neither reads the target script. Reject unsourced or unbalanced
materializations and report their construction limits. Fillers must come
from attacker inputs under the existing conservation rules, never be
invented or counted twice. The displaced NFT must actually leave the
successor to reproduce M5; `nftDetached` alone earns no detection credit.

**Budget decision: at most one declared output is edited per probe.** This
limits the multiplicative family and keeps per-shape slices useful. It does
not follow from the single-defect hypothesis. The verified counterexample
has one deleted `propositionBytes` check and needs two outputs re-treed:
if a retained check equates two output trees and the deleted check anchored
one to `SELF`, changing either output alone still fails. This known class
is excluded by the initial budget, recorded as out-of-family, and retained
in the widened corpus. It is not deferred on the premise that no such
witness exists. Payout balancing of the free sink is separate from this
one-fixed-output edit budget.

**Accounting changes are required.** The phase-2 NFT ride-along rule can
still report identity loss, but a re-treed box scores only if the new
objective recognizes its destination and attributes unsanctioned receipts.
M6 meets that condition through its protected pool; M7 meets it only after
the companion enters `V`. The generator remains blind to target script
content; accounting declarations do not become generation hints.

**Placement in the pinned axis order:**

```text
none → declared-output modifications → companion re-creations → sinks
```

This is a truncation policy motivated by M5/M6, not three promised
conversions. Mapped M7 uses the newly declared companion continuation as an
edit target; this additive order does not implicitly compose a synthesized
companion recreation with another shape's edit.

### Budget arithmetic (historical baseline; new family estimated)

The prior mapped USE measurement (`request_from_map` over `use-lp.json`)
had **12 inputs, 4 outputs**, **19 shapes / 67 tally buckets**, and a
**slice floor of 16**. The mapper produces one free sink, so that output
count gives three fixed edit targets. The recorded release timing was
**8.31 ms/probe** (2,000 probes in 16.6 s); it is not a phase-3 timing run.

For that unchanged template, five non-verbatim variants per fixed output
give at most `3 × 5 = 15` extra shapes, or **34 shapes** before filtering
and deduplication. At 50,000 probes this is roughly 1,470 probes per shape;
using the old timing gives roughly seven minutes, an estimate only. New
companion continuations, source preservation, and objective work can change
both counts and cost. Re-measure the actual mapped requests and their
`ShapeTally`, `capacity` and `sliceFloor` before acceptance. Retire the old
35-shape and powerset comparison as arithmetic for a different family.

**Acceptance.** With 3b pinned, measure the unchanged family and then the
new variants at common caps. Require actual attacker NFT receipt for M5,
reserve receipt for M6, and clean originals. Claim M7 only after both scope
and mapped construction prerequisites pass. Preserve the USE anchor and
record the two-output counterexample as a budget exclusion.

## 3b — implement the objective and conditional release terms

**The evidence** (#71). Today's `leak()` subtracts script-matched output
holdings and static declared free-payee amounts from `Protected` input
holdings. It neither checks the recipient's spendability nor includes
companions. In the verified M4 winning shape, **all ten stock tokens and the
payment go to the seller's P2PK box; the attacker's output is empty**.

| Historical run | Seller's output | Attacker's output | Old reported extraction |
|---|---|---|---|
| Unmutated original | 21,000,000 nanoErg and 10 stock tokens | Empty | 10 tokens + 1,000,000 nanoErg; `drainable`, 6 hits |
| Price-guard mutant | 3,000,000 nanoErg and 10 stock tokens | Empty | 10 tokens + 1,000,000 nanoErg; `drainable`, 710 hits |

These figures are custody deficits, not assets extracted by the attacker.
The control is an honest overpayment; the mutant row also gives the
attacker nothing. The claim that the attacker spends ERG and extracts the
stock in these rows is withdrawn. Subtracting costs from that mislabeled
stock deficit cannot repair destination attribution. Under the chosen
objective both seller-only rows score zero, irrespective of the price
predicate. They cannot prove an attributable M4 detection.

**Conditional sanctions remain necessary.** In an actual purchase the
attacker is the buyer and receives tokens. Recognizing the buyer's sink
alone cannot distinguish payment at the agreed rate from theft. The caller
must declare exchange terms; the hunt never derives them from the hunted
tree. A declarative sale term needs at least:

- the stable victim input reference and stock asset;
- the seller's exact recipient script, bound independently of an output's
  mutable index or payee label;
- the released quantity, measured from realized stock leaving legitimate
  continuation/owner destinations, with attacker receipts identified;
- the payment asset and rate (here 1,000,000 nanoErg per stock token),
  checked against realized payment to that seller;
- explicit treatment of released box value, refunds and fees, and caps
  preventing one payment or release from satisfying multiple claims.

For ten tokens delivered to the buyer, payment of at least 10,000,000
nanoErg satisfies the sale term and sanctions that token receipt. Payment
of 3,000,000 does not. Stock delivered back to the seller is not an
attacker receipt at either payment amount. Evaluate terms after all
permutations and re-treeing: moving a payment slot or replacing its script
must not make an attacker payment count as seller payment. Bind source
references through permutations as well. The former illustrative JSON
using only `paidTo.output = 0` is insufficient and is withdrawn.

Custody sets explicitly declare no authorized attacker release, or a bounded
static allowance where appropriate. Exchange sets need terms for their
legitimate releases, including the bank's normal redemption in M5; never
sanction its identity NFT as an incidental fee. Terms are caller/map claims,
versioned and recorded like singleton claims. The request must declare its
objective policy; the hunt cannot infer an undeclared exchange by inspecting
the script. Missing or unsupported terms make extraction scoring incomplete,
not a silent `drainable` or an ordinary `notUnderProbes`. Report that
limitation explicitly and keep such sets out of attribution measurements.

**Acceptance.** First rescore the witnesses with destinations, funding and
terms exposed in the report. Require zero for both seller-only M4 witnesses,
zero for a fully paid buyer receipt and self-funded change, and positive
stock extraction for a keyless underpaid buyer receipt on M4. Validate that
receipt against the original too: it must not pass without the seller's key.
Then require discovery from the honest template with a clean original at
the common cap before marking M4 found. If the family cannot generate the
receipt, record an out-of-family miss; do not restore the seller-only score.
An old `proven` flag is insufficient for the new objective: M4 requires a
replacement extraction witness before entering its denominator.

Verify M5/M6/M7 as specified in the decision table, including a preserved
companion scoring zero and a stolen companion scoring positively. The
phase-1 USE anchor must remain `drainable` with synthesis off. Version the
objective independently of synthesis; retain historical baseline results
under their old definition rather than requiring byte-identical extraction
across this deliberate semantic change.

## 3c — source fidelity, registers and data inputs

**Register preservation is a prerequisite, not an existing data source.**
`map::source::ChainBox` has no register field and
`scenario_box_from_chain` initializes empty registers. Consequently mapped
requests currently supply no observed register values to this family, and
scripts reading their own or other inputs' registers can lose required
context before probing begins.

Preserve registers end to end: archived source response → source adapters
and `ChainBox` → map nodes/serialization → scenario conversion → request
inputs, data inputs and honest successors. Preserve types and serialized
values; distinguish genuinely absent registers from a source that omitted
them. Required but unavailable data is a source-fidelity limitation, not
evidence that a register family was exhausted. Acceptance must exercise
`request_from_map` on register-bearing source fixtures, including round-trip
fidelity and the immutable companion's actual value, before measuring gains.

**Register family.** After that prerequisite, enumerate type-compatible
values observed on declared boxes and mapped nodes, plus a pinned finite set
of offsets relative to the request height (with checked conversions). Record
the offsets; never extract constants from the hunted script. Apply variations
only to attacker-controlled inputs and constructible outputs. Preserve
protected and companion input contents, including registers, byte for byte.

The motivating `usePayout` condition is
`SELF.R4[Int].get + 5040 <= HEIGHT`, not equality to a height offset. Its R4
belongs to an immutable companion input. Varying an attacker decoy's R4
cannot change that companion's last-payment height or make an immature box
mature. Faithful register preservation makes the real predicate evaluable;
it does not itself add height search or permit rewriting the companion.
Use a distinct proven mutant whose exploit depends on an attacker-controlled
register to measure this axis. Any future height or alternative companion
selection family needs its own declared scope and measurement.

**Data-input family.** Today data inputs are passed through verbatim and
mapped external nodes become pinned data inputs. The attacker may choose
which available oracle box to reference, not invent its contents. Enumerate
substitution among source-preserved boxes the map actually found, with their
identities, trees, values, tokens and registers intact. Pin the candidate
snapshot and availability at the request height; never substitute spent or
future boxes merely because an archive contains them.

**Acceptance.** Add keyless proven register and data-input mutant classes
with clean originals before evaluating these axes. Measure preservation
alone first, then register variation and data-input substitution separately.
Require a mapped request to expose the necessary data and a valid attacker
receipt under 3b; invented oracle values and changed companion R4 values
cannot count as detections.

## 3d — steering, last

`hot_spots()` aggregates cost by opcode/detail label, recording total cost
and count for each label. It has **no branch identity**: distinct program
locations can collapse into the same row. It can support a cost-based
experiment, not a claim that "this branch was reached." Coverage-guided
selection requires new execution instrumentation with stable opaque
location/edge identities, and its own measured overhead and validation.

**Separate construction from selection.** Candidate construction remains a
fixed, declared, script-independent family. An adaptive scheduler may choose
which existing candidate to try next using an explicitly permitted execution
signal. It may not manufacture new constants, registers or output shapes
from the script or trace. Adaptively choosing candidates makes the traversal
depend on target execution; do not call that traversal wholly blind.

Enforce the boundary with separate interfaces: the constructor receives only
request data and family parameters; the scheduler receives candidate IDs
and whitelisted feedback (cost totals, or instrumented opaque coverage IDs),
not trees, decoded operations, constants or raw traces. Module docs explain
the policy but cannot enforce it. Require checks that changing target trees
with all generator-visible metadata fixed leaves the candidate family
unchanged, and that recorded feedback plus a seed reproduces selection.

**Reporting.** For fully specified objectives, `notUnderProbes` still means
no hit under the recorded probes, never safety. An adaptive seed and schedule
describe a traversal, not exhaustive coverage. Record the family version,
objective version, selection policy, permitted signal and instrumentation
version, seed, caps, actual probes and replay information. Report enumerated
and steered runs separately at the same budget, including runtime overhead.

**Gate.** Do 3d only if, after 3a–3c, the widened corpus has witnessed
in-family misses caused by cap truncation. The historical cap-truncation
column is empty. Cost ranking is not evidence that this gate has passed.

## Corpus prerequisites — block phase-3 acceptance

**Decision: widening the corpus is a blocking dependency, not debt to clear
alongside a claimed phase-3 success.** Design and implementation preparation
can proceed, but no sub-phase is accepted or credited before its measurement
gate is satisfied. The existing seven mutants cover only three of five
operators with proven keyless witnesses; `flip-index` has no proven case and
`positional ↔ unbound-search` has no mutant. Every acceptance above depends
on this scoreboard, so the existing denominator cannot validate the plan.

Before accepting 3b or 3a, require the corpus's original **at least 20
mutants** target, with at least one independently validated keyless extraction
witness for each of the five operators. Count keyed-insider and unproven
cases separately; they cannot fill a proven-operator coverage cell. Include
the demonstrated single-defect/two-output witness as an explicit initial
family exclusion. Before accepting 3c, also require proven register and
data-input cases. Preserve C-incident as a separate apparatus check.

Revalidate all proven witnesses under the new objective, not merely
`txcheck.valid`: non-attacker spending inputs must pass without unavailable
keys, and at least one victim-funded unauthorized attacker receipt must
exist. Replace M4's non-extraction witness through this independent proof
step or report it as unproven under the new definition, never quietly count
it as found or drop it to improve the rate. Widening does not require every
proven mutant to be found; out-of-family misses remain in the denominator.

Freeze honest templates, role/accounting declarations, sanctions, source
snapshots, attacker capabilities and common caps before the hunt. Run each
mutant and its original in the same configurations; derive attribution from
a positive mutant and a clean original. Rebaseline explicitly when objective
or corpus membership changes, report both denominators and operator tables,
and separate rescoring from search improvements. A mapped construction or
source failure cannot be hidden by a hand-built replacement request.

## Non-goals, recorded

- **Solver-assisted values** — phase 4. Neither enumeration nor steering
  invents a required computed constant.
- **Multi-transaction chains and arbitrary future-script spendability** —
  outside this single-transaction extraction objective.
- **The keyed-insider attacker model.** M1–M3 require beneficiary/receiver
  keys on victim inputs; recognizing the attacker's own P2PK outputs does
  not bring these cases into scope.
- **Pricing / worth-weighted objectives.** Conditional terms compare
  declared obligations to realized transfers; they do not value tokens in
  ERG or measure profitability. Pure destruction and third-party diversion
  remain custody-loss diagnostics outside `drainable`.
- **Continuous sweep and role inference** — phase 6, gated on complete
  objective declarations, source fidelity and clean exchange controls.
  Unsupported policies must not feed an ordinary extraction verdict into
  a sweep's findings.

## The standing rule this document inherits

**If an artifact states a policy, the harness must read it.** Every objective
version, victim scope, attacker capability, sanction, source-fidelity claim,
corpus coverage gate, cap and steering parameter introduced here must be
validated and pinned as part of the measurement. Module documentation and
hand-set `proven`/`found` flags are not enforcement.

The phase-2 lesson also stands: every claim about mapped operation must be
assertable against `request_from_map` output. Hand-built witnesses establish
ground truth; they do not establish that the mapped request preserves the
same registers, companion continuations, terms or constructible family.
