# Batch 2 / S02 implementation report

S02 was implemented in the `v2/batch-2` worktree. **The implementation session
could not create commits:** its sandbox mounted the worktree's Git metadata
directory (`$WORKTREE_GIT_DIR`) read-only, so `git add` and `git commit` both
exited **128** when Git could not create `index.lock`. Nothing was pushed and
`main` was not modified during that session. The three commit messages it
prepared, with the requested trailers, are retained as `commit-1-message.txt`,
`commit-2-message.txt`, and `commit-3-message.txt`; the commits were created
afterwards from those messages once the tree had been verified.

## Implementation and evidence

- Four registered, documented AST lints: `unconstrained-outputs`,
  `successor-field-drift`, `trivial-sigma-branch`, and
  `unauthenticated-code-execution`. Findings remain static review priorities;
  none asserts an accepted transaction, vulnerability or exploitable deployment.
- Shared positional helpers, reused successor/reserve recognition and immutable
  identity anchors. `unbound_box_reserves` has only a scope-note change; its
  behaviour is identical. Existing delegated/trust recognition is unchanged.
- Nine versioned mechanical mutant/control pairs in `examples/mutants/s02/v1`,
  registered in separate `staticLintPairs` sections of both JSON documents.
  Every pair compiles and lifts in both rendering modes, catches the mutant
  exactly once for its lint and leaves that lint clean on its control.
- The existing `more_lints` precision suite now has 18 tests covering all four
  new lints, including wrong fields/indices/contexts, aliases, real authority
  conjuncts, sum/universal bounds, byte authentication and explicit AST-only
  deserialisation shapes. Compiler/lift coverage is not claimed for raw forms.
- Both exact gate names are in `mutation_corpus.rs`; the sweep gate checks the
  real corpus file set, all source/tree hashes, compilation outcomes and every
  current finding against the retained review and Markdown reasons.
- Full release sweep: **124 files, 114 complete lifts, 0 partial, 10 compile
  failures; 87 reviewed sites = 8 true-positive static observations + 79 benign
  patterns/recogniser limitations**. See [the complete per-site review](../../audit-sweep.md)
  and [raw sweep](audit-sweep.json). This is the actual bundled population,
  including 16 vector files, authored examples and LSP fixtures, not the spec's
  historical 79-contract denominator. No deployment identity was verified.
- Shipping priorities: LOW for output tails, successor fields and trivial
  sigma; MEDIUM for unauthenticated code. Benign fee/change, delegated
  accounting, resets and token delivery remain explicit observations rather
  than false vulnerability labels.
- S02 is implemented in `docs/ROADMAP.md`'s v2 block and the spec's `newUnits`
  example, with package `ergo-sandbox`, target `mutation_corpus`. The executable
  implemented set is exactly `{'W00', 'S00', 'S02'}` on this base; W01 is still
  unimplemented here. Vector classes 2/3/5/9 name the new instruments and
  `VECTORS.md` was regenerated.

## Existing checks reconciled

The first targeted audit run failed six tests. The standalone context-variable
fixture was fixed by narrowing `trivial_sigma_branch` to writable-data direct
result disjuncts. It retains its original expected result. Constant-true entire
results remain covered because compilation can erase their original OR.
Conjunctions containing real signature authority are also explicitly negative
controls.

Five existing tests now assert the exact additional `unconstrained-outputs`
observation instead of asserting that *all* lints are absent:

- `get_or_else_is_never_flagged`: the fallback handles absence, but its register
  comparison constrains only output 0.
- `an_nft_bound_box_is_not_flagged`: NFT identities bind the input/output boxes;
  they do not constrain additional outputs.
- `a_self_successor_pool_is_not_flagged`: script/value preservation constrains
  the named successor, while external funding/fee/change outputs remain open.
- `binding_by_the_whole_token_pair_counts`: the pair binds identity, with no
  total-output or tail constraint.
- `binding_by_the_whole_tokens_collection_counts`: full token preservation
  binds the successor, with no constraint on additional outputs.

These assertions still require every old lint to remain absent. No assertion
of static-only provenance, review-priority authority or non-exploitability was
removed.

The new answer-key section triggered the roadmap scoreboard and the M00/D00
whole-document baseline checks. The roadmap scoreboard now excludes only the separate `staticLintPairs`
namespace from the frozen hunt projection; a regression test rejects historical
cap/verdict/membership changes and unknown extra namespaces. The exact pre-S02
answer-key bytes are archived in `examples/mutants/answer-key.pre-s02.json` under
M00 and D00's unchanged SHA-256 `e7f31518f021dca2d74ce4267126313f1624f091b6cedda0246e8115ead3c3d5`.
Both inventory gates additionally compare every historical live field against
that archive through a shared test helper. The pinned manifests, all historic measurements/caps/methodology and the entire
`roadmap-policy:v1` block are unchanged. The release gate measured the same
historical **4/5** attributable result; no static pair enters that denominator.

## Verification

Every build used
`CARGO_TARGET_DIR=$CARGO_TARGET_DIR`, the shared local build directory.
Commands below are recorded verbatim with observed exit codes and logs.
An interrupted initial sweep has **no observed exit**, not an invented pass.
The named-filter `lint-step` run executed only the mutant gate; its other
selected targets ran zero tests and are not counted as suite verification.
`existing-lint-tests.log` and `lint-precision.log` contain the real suite runs.

The required `cargo test --workspace` exited **101** on
`seed_corpus_holds_the_exact_floor_when_checkout_present`: the local newer node
checkout supplies **92** seed entries versus the pinned **87**. The test and
pinned floor were left unchanged. A subsequent workspace run with only that
exact environmental test skipped first exposed the M00 file-hash issue above;
that issue was fixed and the run repeated. The repeated run passed the debug
historical mutation measurement, then exposed the same answer-key pin in D00.
Concretely: the workspace run recorded below with only the environmental test
skipped exited **101** because `property_inventory_pins_24_cases_and_independent_answers`
failed on the answer-key hash pin. That pin was then shared through the archive
check, and both inventory suites passed (`inventory-baselines.log`). No complete
workspace run passed inside the implementation session. After the branch was
rebased onto main and the review fixes were applied, the full workspace command
with only the environmental skip was run again outside the session and exited
**0**; its filtered output is retained as `workspace-rebased.log` and recorded in
the table below.

The S02 report is [S02.json](S02.json); scoreboard, W00, S00 and S02 all passed.
Its command stream includes instrument existence, compiled suites, both S02
gates and the unchanged historical mutation corpus, with no skipped tests.

| Command | Exit | Evidence |
| --- | ---: | --- |
| `cargo test -p ergo-sandbox --test audit --test more_lints --test delegated_reserves` | 101 | [initial-lint-tests.log](initial-lint-tests.log) |
| `cargo test -p ergo-sandbox --test mutation_corpus new_lint_mutants_are_caught_and_controls_are_clean -- --exact` | 0 | [mutant-pairs.log](mutant-pairs.log) |
| `cargo run --release -p ergo-sandbox --example audit_sweep` | unobserved (interrupted) | [sweep-initial.log](sweep-initial.log) |
| `cargo test -p ergo-sandbox --test audit --test more_lints --test delegated_reserves` | 0 | [existing-lint-tests.log](existing-lint-tests.log) |
| `cargo test -p ergo-sandbox --test more_lints` | 0 | [lint-precision.log](lint-precision.log) |
| `cargo test -p ergo-sandbox --test audit --test more_lints --test delegated_reserves --test mutation_corpus new_lint_mutants_are_caught_and_controls_are_clean` | 0 | [lint-step.log](lint-step.log) |
| `bash -c 'cargo run --release -p ergo-sandbox --example audit_sweep > docs/reports/batch-2/audit-sweep.json'` | 0 | [sweep-final.log](sweep-final.log) |
| `cargo test -p ergo-sandbox --test mutation_corpus deployed_corpus_sweep_is_recorded -- --exact` | 0 | [sweep-gate.log](sweep-gate.log) |
| `python3 scripts/render_vectors.py` | 0 | [render-vectors.log](render-vectors.log) |
| `python3 scripts/render_vectors.py` | 0 | [render-vectors-final.log](render-vectors-final.log) |
| `cargo fmt --all -- --check` | 0 | [fmt.log](fmt.log) |
| `python3 scripts/test_roadmap_gate.py` | 1 | [roadmap-tests.log](roadmap-tests.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy.log](clippy.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | [roadmap-tests-final.log](roadmap-tests-final.log) |
| `git add ergo-sandbox/src/audit ergo-sandbox/tests/audit.rs ergo-sandbox/tests/more_lints.rs ergo-sandbox/tests/mutation_corpus.rs examples/mutants docs/audit-sweep.md docs/reports/batch-2/audit-sweep.json docs/reports/batch-2/site-review.json docs/reports/batch-2/sweep-final.log` | 128 | [stage-lints.log](stage-lints.log) |
| `git commit -F /tmp/s02-commit-1.txt` | 128 | [commit-lints.log](commit-lints.log) |
| `cargo test --workspace` | 101 | [workspace-tests.log](workspace-tests.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 101 | [workspace-with-known-skip.log](workspace-with-known-skip.log) |
| `git diff --check` | 0 | [final-diff-check.log](final-diff-check.log) |
| `cargo test -p ergo-sandbox --test mapping_inventory legacy_metrics_are_measured_without_baseline_changes -- --exact` | 0 | [mapping-baseline.log](mapping-baseline.log) |
| `python3 scripts/roadmap_gate.py --require S02 --report docs/reports/batch-2/S02.json` | 0 | [roadmap-S02.log](roadmap-S02.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-final.log](fmt-final.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy-final.log](clippy-final.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | [roadmap-tests-complete.log](roadmap-tests-complete.log) |
| `git diff --check` | 0 | [post-review-diff-check.log](post-review-diff-check.log) |
| `cargo test -p ergo-sandbox --test more_lints sigma_disjuncts_preserve_conjunctive_authority_and_result_scope -- --exact` | 0 | [sigma-result-precision.log](sigma-result-precision.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-complete.log](fmt-complete.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy-complete.log](clippy-complete.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 101 | [workspace-with-known-skip-final.log](workspace-with-known-skip-final.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-baselines.log](fmt-baselines.log) |
| `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory` | 0 | [inventory-baselines.log](inventory-baselines.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` (after rebase and review fixes) | 0 | [workspace-rebased.log](workspace-rebased.log) |

`cargo fmt --all` was also run during editing and exited 0; the final check
commands above verify the resulting formatting without edits. Read-only
inspection and edits are not test results. Final `git diff --check` passed.

## Remaining limitation

During the implementation session, commit creation was blocked by that
session's read-only Git worktree metadata, not by missing authorization. No
alternate Git metadata/index path was used to bypass the restriction; the
commits were made afterwards from the prepared messages. Source-only analysis cannot decide reachability,
singleton supply, intentional state transitions or deployment exploitability;
all recogniser limits and the ten unanalysed corpus files are recorded.
