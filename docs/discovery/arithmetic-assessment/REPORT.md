# Arithmetic entry assessment — 2026-09-10 UTC

**Entry gate not met: 3/20 guarantees across 2/6 families qualify as blocked
solely by the bounded arithmetic proposal. Required: at least 6 across at least
3. Stop; no arithmetic implementation is authorized or performed.** This fails
the proposed operational test of the author's census interpretation. It does
not prove arithmetic has no uses, or turn unknowns into impossibility results.

## Freeze, scope and evidence

[Candidates](candidates.json), authenticated by [their digest](candidates.sha256),
were written before replacing the parser probe or implementing anything for the
assessment. All 20 remain, in their original order, with source hashes and exact
intended guarantees. Six distinct authored template families were selected from
material already in this repository, including arithmetic-heavy AMM and bank
clauses. These are explicitly authored models, not deployed protocol identities
or a new transfer cohort. This selection is not a random census sample.

The proposed operations were frozen as runtime multiplication, signed division
truncating toward zero, minimum and maximum within the **existing checked i128
range**, including dimensional product/quotient rules. No extra reads, numeric
range, conditionals, bindings, solver, lifecycle, mapping or cap changes. Constant
multiplication already exists as `scale`; `data-inputs` roles and integer register
reads already exist. Adding operations is not adding a solver.

[Encoding attempts](attempts.json) contain every submitted guard and assertion.
Unknown op names are deliberately unsupported pseudocode embedded in declaration
sketches to obtain real refusal output from the unchanged parser. The declared
contract bytes are explicitly an **uncompiled parser stand-in**, not the template's
compiled bytes. Source digests identify the actual reviewed text; they are not
zero digests or claims of deployment. Constants and token identities are explicit
illustrative bindings (AMM supply 10^12, fees 997/1000; bank ratio 400 and fee 2%;
vesting heights 100/200). No scenario, baseline or expected fact was edited.

For the arithmetic candidates, the current-vocabulary alternative is `scale`
with a fixed factor plus add/sub and comparisons. Neither a variable reserve,
circulation nor elapsed height is a fixed factor. Replacing one by a fixture's
observed number would encode a different, narrower promise. No exact affine
reduction was established for the product/quotient clauses; cases already using
only fixed fees, fixed raises and accounting differences use existing operators.
The diagnostic sketches retain the missing operations rather than disguise a
constant substitution as a successful encoding.

[Raw parser output](parser-output.json) binds the attempts digest. Reproduce from
the repository root:

```sh
CARGO_TARGET_DIR=./target-p00 cargo run -p ergo-sandbox --release --example real_property_probe -- docs/discovery/arithmetic-assessment/attempts.json
```

The command exits zero when it records all parser outcomes; that is **not** an
entry-gate pass. Six sketches parse, fourteen are refused. Parsing does not
establish meaning, role identity, node acceptance, branch execution, usefulness
or a counterexample. No new transaction validation was attempted in this
assessment. The existing workspace checks are recorded separately.

## Review of all 20

Notation: X/Y are input ERG/token reserves; dX/dY their output-minus-input
deltas; dS is input-minus-output bank-held LP, S the circulating supply. A role
is an explicit positional binding, not inferred adjacency. “Sole” is a model
review of the arithmetic clause's encoding requirements under those bindings,
not independent author adjudication. General absence of transfer/independent
review is retained separately, not counted repeatedly as a DSL defect.

| ID | Intended meaning and attempted encoding | Current result; all identified blockers | Sole arithmetic? |
|---|---|---|---|
| A01 | Positive-X swap: LP unchanged and dX>0 implies Y*dX*997 >= -dY*(X*1000+dX*997). | `mul` refused. 30 expression nodes. Products with fee factors have no established i128 bound over the source's BigInt domain. A safe equivalent within the same cap was not established; scope cannot be narrowed to convenient fixture values after selection. | Uncertain; no credit |
| A02 | Other swap direction: LP unchanged and dX<=0 implies X*dY*997 >= -dX*(Y*1000+dY*997). | `mul` refused. 30 nodes; same unresolved BigInt/intermediate range issue. This includes the dX=0 branch; no case replacement. | Uncertain; no credit |
| A03 | Deposit: positive dS,dX,dY implies dS*X <= dX*S. | `mul` refused. 29 nodes. Two products of Long-bounded amounts fit i128 under the explicit fixed supply binding; no fee-factor third product. Product units agree (LP × nanoERG). | Yes |
| A04 | Deposit: same guard implies dS*Y <= dY*S. | `mul` refused. 29 nodes; two bounded products, same LP × reserve-token units on both sides. | Yes |
| B01 | SC accounting: R4out = R4in + (held SCin - held SCout). | Parses with R4 declared in the actual SC-token unit. Current add/sub suffice. Source also checks nonnegativity and RC tracking; this candidate is only the frozen SC equality, not the entire bank. | No, already expressible |
| B02 | Mint SC: increased circulation implies 100*reserveOut >= 400*scOut*oracleRate. | `mul` refused. 12 nodes. Oracle data-input R4 is readable now. Its named price unit needs a dimensional relationship to SC/nanoERG; proposal permits product rules but this binding is not yet represented by current units. Triple-product i128 range is also unestablished. No blanket oracle-read blocker. | Uncertain; no credit |
| B03 | Mint SC pays capped price plus rounded 2% fee. | `mul`, `min`, `div` missing. Sketch uses min(rate,X/sc), but omits source's sc=0 branch; it is not faithful there. Expanding into guarded Boolean branches needs duplication: the sketch already uses 31/32 nodes. Named price-to-token unit relationship, zero division and source Long intermediate semantics require review. No special nonzero-circulation premise is silently added. | No |
| B04 | RC nominal price is equity/RC when both positive, otherwise default. | Refuses invented `nominal-price` and `if`; also needs mul/min/div, zero-SC price branch and correct units (the diagnostic sketch's scalar zero for RC is not a valid token-unit comparison). Nominal price is an internal value, not an observed register: an independent output guarantee was not established. Replacing it by another payment guarantee would change the frozen candidate. | No |
| V01 | Before start, continuation ERG >= current SELF ERG. | Parses using height and sum-erg. This is current-box accounting, not conservation of an original endowment over many withdrawals. Continuation identity remains an explicit binding; source also constrains script/tokens/register absence, outside this frozen ERG clause. | No, already expressible |
| V02 | During vesting, output ERG >= X - X*(HEIGHT-start)/(end-start). | `mul`/`div` refused. 20 nodes. With fixed valid start/end, elapsed/span have matching height units and the quotient returns nanoERG. Bounded Long × Int-domain elapsed fits i128. This is the source's current-input rule, not a stronger original-deposit promise. Node rejection remains authoritative for invalid/overflowing script execution. | Yes |
| V03 | After end, release still needs beneficiary authorization. | `authorized-by` refused. The property vocabulary has no proof observation; a textual authorization premise does not prove it. Arithmetic changes nothing. | No |
| U01 | Existing bidder's next bid raises input ERG by minRaise before deadline. | Numeric projection parses with add. The sketch lacks the existing-bid predicate: R4 is SigmaProp, with seller fallback; typed SigmaProp/register-byte reads and branch discrimination are unavailable. Seller/no-bids override prevents treating height alone as the branch. | No |
| U02 | New bid refunds previous bidder at least SELF ERG. | Numeric refund comparison is expressible; `register-bytes` refused for dynamic R4 SigmaProp recipient identity. Need fallback seller/no-bids semantics and correct bid-branch discrimination; amount alone is insufficient. | No |
| U03 | Settlement transfers token bundle to recorded winner. | Register-byte and whole-token-bundle observations/equality absent. Dynamic recipient, missing-register fallback and no-bids seller path cannot be replaced by fixed output positions or a single hardcoded token. | No |
| R01 | Public registration preserves registry ERG. | Numeric projection parses. Public versus registrar override is not observed; true guard is only a projection and would make a false universal assertion about the owner path. Explicit branch/proof premise required. | No |
| R02 | Public registration pays fixed fee to registrar. | Amount projection parses. Script equality to an explicitly supplied registrar script is expressible, but the sketch omits it. Same public/owner branch limitation as R01. Fixed-fee arithmetic is already expressible. | No |
| R03 | Insert absent name and commit resulting AVL digest. | `avl-insert-matches` refused. AVL registers/digests, context-variable proof/key/record observations and insertion semantics absent; owner override is another branch limitation. Node proof validation is not a DSL insertion assertion. | No |
| X01 | Partner outputs each equal input ERG. | Both numeric equalities parse. Partner path cannot be inferred from two output positions; owner can spend freely. Need branch/proof premise and full-mix script identities for full interpretation. | No |
| X02 | Partner registers mirrored and distinct. | `mirrored-group-registers` refused. GroupElement register reads/equality absent; partner/owner proof branch and output identities also required. | No |
| X03 | Partner proves DH relation using owner/output keys. | `prove-dh-tuple` refused. Proof observations, GroupElement reads and branch discrimination missing. No new prover or alternative interpreter is authorized. | No |

For A01/A02/B02, “uncertain” is deliberately not “blocked solely by arithmetic.”
The assessment did not prove an i128-safe rearrangement, and did not invent a
nonoverflow premise or enlarge the cap. Even granting A01/A02 after such a proof
would yield only five qualifying guarantees across two families; B02 remains
necessary to reach the frozen three-family threshold. The report leaves that
uncertainty visible instead of treating the first parser error as a sole cause.

The machine-readable [per-row review](review.json) preserves classifications and
expression-node counts. The [accounting gate output](gate-output.json) records
an actual exit **1**, with integrity assertions passing and threshold unmet:

```sh
python3 docs/discovery/arithmetic-assessment/check.py
```

The check authenticates the frozen membership, source hashes, all 20 attempts,
parser digest and review alignment, then applies the threshold. It verifies
accounting; it does not automate semantic adjudication. Its nonzero exit is
separate from the unchanged roadmap CI's successful handling of recorded stops.

## Decision and effort

Qualifying IDs: **A03, A04, V02**. Families: **AMM, vesting**. All other IDs remain
in the denominator, including incorrect sketches and uncertain range cases.
The failed entry gate cancels the conditional three-day implementation allocation
for this unit; no operators, evaluator semantics, tests of new arithmetic, or
registry entries are added. Reopening requires a separate decision and must
retain this failed assessment; no replacements or automatic second cohort.

Assessment ceiling: two person-days. One automated session, within one elapsed
working day; no independent reviewer participated. Exact human person-hours are
unmeasured, not equated with wall time. This is not a claim of ceiling exhaustion.
`discoveryCredit: 0`; independent semantic measurement remains incomplete.
