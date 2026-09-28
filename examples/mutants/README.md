# Mutation corpus — mutants and answer key

> Phase 2.5 of the drain-hunt roadmap (#66). Design record:
> `docs/superpowers/specs/2026-09-08-mutation-corpus-design.md`. Every mutant
> here is **one mechanically applied defect** to a contract source that
> compiles with `compile_with_params`, plus the honest transaction template,
> the role labelling (decided once, from the protocol's structure), and — for
> proven mutants — a hand-built keyless drain witness validated by
> `txcheck::check` before the hunt ever runs.
>
> The harness (`ergo-sandbox/tests/mutation_corpus.rs`) runs the corpus in
> both configurations (synthesis off / on), reports the detection rate over
> the **proven** denominator only, and asserts no regression against
> `answer-key.json` (the recorded rate). It never asserts an absolute floor.

## Files

- `mutants.json` — the whole corpus: per mutant, the base source, the
  operator, the mechanical diff (find → replace), the params, the honest
  template, the roles, the witness, and the classification fields.
- `answer-key.json` — the recorded run: per-mutant verdicts in both
  configurations, the rate, and the miss classes.

## Conventions

- Mutant ids: `M<number>` (`M1`..`M7` in the design record's table).
- Sources are the `examples/tests/*.test.json` sources, mutated by the
  recorded find/replace — reproducible by string substitution alone.
- `proven` mutants carry a `witness` (a full `txcheck::check` request that
  must validate); `unproven` mutants carry a `suspected` note instead, and
  never enter the rate's denominator.
- Originals are negative controls: expected `notUnderProbes` in both
  configurations. A `drainable` original is escalated, never tuned away.

## S02 static lint pairs (v1)

`mutants.json.staticLintPairs` and `answer-key.json.staticLintPairs` add a
separate syntax-recognition measurement. Nine pairs under `s02/v1/` cover
output tails, successor ERG/token/register fields, constant/writable sigma
alternatives, and context execution/template substitution. Each original and
mutant is compiled and lifted in both rendering modes. The harness checks the
mechanical diff, versioned files, one expected observation on the mutant and
zero for that lint on its control. Other lints can still report on a control.

These are static observations, with no witness or execution claim. They do not
enter the historical hunt's proven denominator, detection rate or caps. The
historical records above remain unchanged. A correction to a committed fixture
requires a new version and a new answer-key row; never silently rewrite v1.
`answer-key.pre-s02.json` retains the exact previous answer-key bytes under
M00 and D00's unchanged pinned digest. Both inventory gates also require every
historical field of the live answer key to equal that archive; only the new static
namespace is outside the frozen hunt measurement.
Direct AST precision tests in `more_lints.rs` additionally cover deserialisation
forms the pinned compiler does not emit, labelled separately from compiled pairs.

## I04 upgrade-hook pair (v1)

`i04/v1/upgrade-hook-{control,mutant}.es` adds `I04-01-v1` only in the
append-only `staticLintPairs` namespace. Removing the same-register equality
from the same-script continuation produces exactly one `upgrade-hook` LOW
observation; the control produces none for that lint. The new sibling leaves
`trust-assumptions` and every existing lint unchanged. Branch authority is a
static observation, with no claim about an executable or exploitable upgrade.
The deployed-corpus measurement and every new site review are in batch-7.

## H01 holdout namespace (v1)

`holdout.json` is a separate cohort outside the frozen hunt measurement, and
`holdout/v1/` is its version directory. Nothing in the harness *enforces*
append-only, so the discipline is explicit: a correction to a committed fixture
is a new version directory with a new manifest, never a silent rewrite of v1.
What the harness does enforce is that every fixture and its compiled tree still
match their recorded digests, that each pair is exactly one recorded
`find` → `replace` applied to its control, and that the declared caps equal the
instrument's own published caps — so editing a v1 fixture fails the run until it
is re-recorded on purpose.

Per row the manifest carries the pair, the role, the digests, the declared SELF
box, the expected verdict **with its stated basis**, and one `authoring` value.
There is no per-row `exposure` field, and there is deliberately no per-row
exposure *claim* either: exposure is **measured** by the harness, which scans the
declared roots and extensions for verbatim copies and for shared clauses (a
conjunct, not only a whole line, split on newlines, `&&`, `||` and `;`), with
the namespace and the manifest itself excluded. The discovery-eligible
denominator is derived from the declared `authoring` and that measurement, so
every current row — `instrument-visible` — yields a denominator of zero and no
rate at all. The harness reproduces the recorded expectations and reports them
as self-consistency; it does not claim novel discovery.

`shadow_model.rs` checks bookkeeping consistency between recorded aggregates and
probe records, and it checks each aggregate class's relation **in both
directions**: the class must have the probe evidence it names, and must not
carry the evidence that names a different class (`movableByAnyone` may not also
report a passing attacker sample, `requiresProof` and `notUnderProbes` may not
report a pass of either shape). It is not a second reducer, cannot produce node
acceptance or a property-violation verdict, and refuses unknown tokens rather
than guessing. It is also write-only: only the recorded input and the policy
deserialise, so a report read back from JSON — including one claiming
`nodeValidated: true` — cannot enter its vocabulary. Every count and text field
it reads is capped, and every finding's detail is at most
`MAX_RESIDUAL_TEXT` characters.
