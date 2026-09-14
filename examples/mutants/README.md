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
