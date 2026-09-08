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
