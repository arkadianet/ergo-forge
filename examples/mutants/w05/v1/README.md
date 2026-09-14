# W05 recipe mutations

`examples/mutants/recipes.json` registers these pairs separately from the
historical hunt answer key and `staticLintPairs`. Neither historical JSON file
was edited, avoiding frozen digest changes and parallel S02/I04 appends.

The controls are the two recipes themselves. Each versioned mutant is exactly
one recorded replacement: omit pool input identity while retaining arithmetic,
or omit the successor's R4 equality while retaining its presence check. Running
the control's independently authored suite with the mutant source must turn
its binding case from the required `fail` into an actual `pass`.

A third manifest row applies an independent output-count mutation in memory
to the pool recipe; it opens the output tail and must violate the same suite's
extra-output expectation. Keeping it separate avoids confounding the identity
mutation. The three relevant observations are `unbound-box-reserves`,
`successor-field-drift` and `unconstrained-outputs`. Controls must be clean under
all current lints in both lift rendering modes. Findings set review priority,
not vulnerability status. Suite results are synthetic, `nodeValidated: false`.

`compose_recipes` runs the recipe tests and all three mutation checks. Existing
recipes, suites and historical measurements stay unchanged. The immutable S02
sweep retains its original corpus; its inventory explicitly delegates only
these two later recipes to this new full-lint gate.
