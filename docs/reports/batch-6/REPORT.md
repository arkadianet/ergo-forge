# Batch 6 — W04: cost and explain in Write

Implemented in the `v2/batch-6` worktree based on `1d53b2f`. The required W04
release gate passed, including W00, all three named Rust gates, and both DOM
suites. The final gate's six subprocesses exited 0. All 40 DOM tests, 30
roadmap Python tests, 4 lockfile-action tests, formatting and Clippy passed.
The complete web suite also passed with cost tracing compiled out.

Workspace command observed exit: **0**. Supplementary sandbox
cost-trace command with the documented seed-test exclusion: **0**.
The exact required `cargo test -p ergo-sandbox --features cost-trace` command
exited **101** at the known local seed-corpus mismatch (92 entries versus the
frozen expectation of 87); that test and its floor were unchanged.

Both final mutation measurements completed with 5/5 tests passing: 804.93
seconds in the workspace run and 803.35 seconds in the standalone feature run.

## What landed in the worktree

- `ergo-sandbox/src/cost_spans.rs`: library folding the existing diagnostic
  cost trace into exact, ambiguous and unattributed JIT rows. Every row carries
  its rule, reason, share and candidate IR ids with nullable source starts.
  Unlabelled increments in cumulative trace totals are retained. The totals
  reconcile, and `exactShare` states the precisely attributed fraction.
- `ergo-sandbox/src/source_positions.rs`:
  source metadata is checked against the evaluated bytes, canonical preorder
  walk and compiler `aligns_with`. Possible string substitutions cannot supply
  editor citations. Only compiler starts are returned; there is no invented
  end. Templates, unmapped nodes, rewritten trees and map failures stay explicit.
- `ergo-web/src/routes/eval.rs`, `dto.rs`, `app.rs`, `routes/lookup.rs` and
  `Cargo.toml`: `POST /api/v1/eval` returns the same run's `costSpans` and
  `mapStatus`. `POST /api/v1/cost-spans` shares that handler. The default-on
  web `cost-trace` feature forwards to sandbox, and the route is registered
  only under the compiled feature and runtime `AppConfig.cost_trace` flag.
  Disabled routes return 404; `/api/v1/config` exposes effective `costTrace`.
- `ergo-web/src/routes/explain.rs` and `routes/mod.rs`: feature-independent
  `POST /api/v1/explain` takes `{scenario,selection}` and uses the same full
  scenario reduction helper. A normal selection chooses the first source start,
  then its smallest IR subtree (innermost), then preorder id. Whole-source
  selection denotes root 0. Recorded values retain their full-run trace indices;
  values, residuals and costs never come from a second extracted-expression run.
- `ui/cost-spans.js`, `ui/explain.js`, `app.js`, `index.html`: Write's Run
  renders **Cost by source** from its eval response, with start-token links and
  explicit ambiguous/unattributed costs. **Explain selection** sends the editor
  selection and full scenario; its panel shows opcode, IR id, source start,
  original recorded values, residuals and missing/truncated evidence. Edit and
  scenario changes invalidate diagnostics; generation checks reject stale
  results. Both buttons preserve explicit scenario parameter/network overrides
  and fill absent fields from Write. Server/source text is rendered as text,
  never HTML.
- `ergo-web/tests/write_cost_explain.rs`, `ergo-sandbox/tests/cost_spans.rs`,
  and both standalone DOM suites provide the new coverage. CI cost-trace
  commands now include ergo-web and CI also runs web without default features.
  W04 is implemented in both complete v2 records. `extras()` registers each
  DOM suite separately; the Python test rejects failed, absent, empty or skipped
  suites. The frozen v1 policy was unchanged.
- [Interface and attribution rules](../../write-cost-explain.md), this report,
  [command ledger](commands.json), [W04 gate evidence](W04.json), integrity
  checker and four prepared commit messages.

Everything here is a synthetic sandbox reduction (`nodeValidated: false`).
Costs and values do not assert a vulnerability or safety. Static findings
retain review-priority meaning. No engine revision, consensus path, evidence
claim semantics, baseline digest, protected report folder or mutant changed.

## Attribution and explanation evidence

The HTTP test used a contract combining `OUTPUTS.exists`, two box value reads,
and a height comparison. It demonstrated nonzero exact and ambiguous costs,
verified every cited start against the compiler map, and checked shares and
all totals against the real cost trace. Repeated amount opcodes remained
ambiguous even though their values differed. Library tests also covered
repeated collection opcodes with `n=8`, missing/misaligned maps, unlabelled
cost increments, substitutions, errors, proofs and deserialisation.

No evaluation-order ownership rule was introduced. All opcode nodes count,
including uncited and unevaluated nodes. Runtime collection detail does not
identify a node. Known method helper aliases and dynamically deserialised
code remain conservative exclusions. The diagnostic JIT trace is distinct
from block-unit cost and excludes the existing optional proof-verification
pass. Error traces can stop before the final attempted charge.

The Explain gate compared whole-source residuals to the same response's
`reducedTo`, and sub-expression values to the same response's indexed full-run
value record and IR id. It additionally compared against a whole-contract run
using identical inputs, including different heights. Tests covered inlined
names, whitespace, sigma values whose result differs from the full root,
Unicode/CRLF positions, malformed ranges, budget exhaustion and partial traces.
The feature-disabled HTTP run exercised Explain as well as the absent cost route.

## Commands and observed exits

Every build used this batch's `$CARGO_TARGET_DIR`. The ledger records validation
and Git subprocess commands, actual exits, elapsed times and full logs,
including failed development attempts. Gate subprocess commands and their
outputs/hashes are retained separately inside `W04.json`. Initial read-only
file inspection and editing used the session tool transcript. No command was
interrupted; no unobserved command was claimed to pass.

| Command | Observed exit | Log |
|---|---:|---|
| `cargo fmt --all` | 0 | [fmt-step1.log](logs/fmt-step1.log) |
| `cargo test -p ergo-web --test http --no-run` | 0 | [build-step1.log](logs/build-step1.log) |
| `cargo fmt --all` | 0 | [fmt-step1-tests.log](logs/fmt-step1-tests.log) |
| `cargo test -p ergo-web --test write_cost_explain` | 101 | [tests-step1.log](logs/tests-step1.log) |
| `cargo test -p ergo-sandbox --features cost-trace --test cost_spans` | 101 | [library-step1.log](logs/library-step1.log) |
| `cargo fmt --all` | 0 | [fmt-step1-final.log](logs/fmt-step1-final.log) |
| `cargo test -p ergo-web --test write_cost_explain` | 0 | [tests-step1-fixed.log](logs/tests-step1-fixed.log) |
| `cargo test -p ergo-sandbox --features cost-trace --test cost_spans` | 0 | [library-step1-fixed.log](logs/library-step1-fixed.log) |
| `cargo test -p ergo-web --no-default-features --test write_cost_explain` | 0 | [feature-off-step1.log](logs/feature-off-step1.log) |
| `cargo fmt --all` | 0 | [fmt-before-commit1.log](logs/fmt-before-commit1.log) |
| `git add ergo-sandbox/src/compile.rs ergo-sandbox/src/lib.rs ergo-sandbox/src/source_positions.rs ergo-sandbox/src/cost_spans.rs ergo-sandbox/tests/cost_spans.rs ergo-web/Cargo.toml ergo-web/src/app.rs ergo-web/src/dto.rs ergo-web/src/routes/eval.rs ergo-web/src/routes/lookup.rs ergo-web/tests/http.rs ergo-web/tests/write_cost_explain.rs .github/workflows/ci.yml` | 128 | [git-add-step1.log](logs/git-add-step1.log) |
| `git commit -F docs/reports/batch-6/commit-1-message.txt` | 128 | [git-commit-step1.log](logs/git-commit-step1.log) |
| `cargo fmt --all` | 0 | [fmt-step2.log](logs/fmt-step2.log) |
| `cargo test -p ergo-web --test write_cost_explain` | 101 | [tests-step2.log](logs/tests-step2.log) |
| `cargo fmt --all` | 0 | [fmt-step2-rule.log](logs/fmt-step2-rule.log) |
| `cargo test -p ergo-web --test write_cost_explain` | 0 | [tests-step2-rule.log](logs/tests-step2-rule.log) |
| `cargo fmt --all` | 0 | [fmt-step2-final.log](logs/fmt-step2-final.log) |
| `cargo test -p ergo-web --test write_cost_explain` | 0 | [tests-step2-final.log](logs/tests-step2-final.log) |
| `cargo test -p ergo-web --lib routes::explain` | 0 | [tests-step2-residual.log](logs/tests-step2-residual.log) |
| `git add ergo-web/src/routes/explain.rs ergo-web/src/routes/mod.rs ergo-web/src/app.rs ergo-web/tests/write_cost_explain.rs` | 128 | [git-add-step2.log](logs/git-add-step2.log) |
| `git commit -F docs/reports/batch-6/commit-2-message.txt` | 128 | [git-commit-step2.log](logs/git-commit-step2.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy-workspace.log](logs/clippy-workspace.log) |
| `node --test ui/tests/cost-spans.test.js ui/tests/explain.test.js` | 0 | [dom-new.log](logs/dom-new.log) |
| `git add ui/app.js ui/index.html ui/cost-spans.js ui/explain.js ui/tests/cost-spans.test.js ui/tests/explain.test.js` | 128 | [git-add-step3.log](logs/git-add-step3.log) |
| `git commit -F docs/reports/batch-6/commit-3-message.txt` | 128 | [git-commit-step3.log](logs/git-commit-step3.log) |
| `cargo test -p ergo-sandbox --features cost-trace` | 101 | [sandbox-cost-trace.log](logs/sandbox-cost-trace.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | [roadmap-tests.log](logs/roadmap-tests.log) |
| `python3 scripts/test_lockfile_action.py` | 0 | [lockfile-action-tests.log](logs/lockfile-action-tests.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/cost-spans.test.js ui/tests/evidence-replay.test.js ui/tests/explain.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 0 | [dom-all.log](logs/dom-all.log) |
| `cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` | 0 | [clippy-cost-trace.log](logs/clippy-cost-trace.log) |
| `python3 scripts/roadmap_gate.py --require W04 --report docs/reports/batch-6/W04.json` | 0 | [gate-w04.log](logs/gate-w04.log) |
| `cargo clippy -p ergo-web --all-targets --features cost-trace -- -D warnings` | 0 | [clippy-web-cost-trace.log](logs/clippy-web-cost-trace.log) |
| `cargo test -p ergo-web --no-default-features` | 0 | [web-feature-off.log](logs/web-feature-off.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-check.log](logs/fmt-check.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/cost-spans.test.js ui/tests/evidence-replay.test.js ui/tests/explain.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 0 | [dom-final.log](logs/dom-final.log) |
| `python3 scripts/roadmap_gate.py --require W04 --report docs/reports/batch-6/W04.json` | 0 | [gate-w04-final.log](logs/gate-w04-final.log) |
| `git diff --check` | 0 | [whitespace.log](logs/whitespace.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/cost-spans.test.js ui/tests/evidence-replay.test.js ui/tests/explain.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 0 | [dom-context-precedence.log](logs/dom-context-precedence.log) |
| `python3 scripts/roadmap_gate.py --require W04 --report docs/reports/batch-6/W04.json` | 0 | [gate-w04-context-final.log](logs/gate-w04-context-final.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 101 | [workspace.log](logs/workspace.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/cost-spans.test.js ui/tests/evidence-replay.test.js ui/tests/explain.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 0 | [dom-null-scenario.log](logs/dom-null-scenario.log) |
| `python3 scripts/roadmap_gate.py --require W04 --report docs/reports/batch-6/W04.json` | 0 | [gate-w04-complete.log](logs/gate-w04-complete.log) |
| `cargo fmt --all` | 0 | [fmt-frozen-compile.log](logs/fmt-frozen-compile.log) |
| `git diff 1d53b2f -- ergo-sandbox/src/compile.rs` | 0 | [pinned-compile-restored.log](logs/pinned-compile-restored.log) |
| `cargo test -p ergo-sandbox --test property_inventory` | 0 | [property-inventory-final.log](logs/property-inventory-final.log) |
| `cargo test -p ergo-sandbox --features cost-trace --test cost_spans` | 0 | [cost-library-final.log](logs/cost-library-final.log) |
| `cargo test -p ergo-sandbox --features cost-trace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 | [sandbox-cost-trace-known-skip.log](logs/sandbox-cost-trace-known-skip.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy-workspace-final.log](logs/clippy-workspace-final.log) |
| `cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` | 0 | [clippy-cost-trace-final.log](logs/clippy-cost-trace-final.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-check-final.log](logs/fmt-check-final.log) |
| `git status --short --branch` | 0 | [final-status.log](logs/final-status.log) |
| `git diff --check` | 0 | [whitespace-final.log](logs/whitespace-final.log) |
| `cargo test -p ergo-web --no-default-features` | 0 | [web-feature-off-final.log](logs/web-feature-off-final.log) |
| `python3 scripts/roadmap_gate.py --require W04 --report docs/reports/batch-6/W04.json` | 0 | [gate-w04-pinned-final.log](logs/gate-w04-pinned-final.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 | [workspace-final.log](logs/workspace-final.log) |
| `cargo test -p ergo-sandbox --features cost-trace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 | [sandbox-cost-trace-final.log](logs/sandbox-cost-trace-final.log) |
| `python3 docs/reports/batch-6/check-integrity.py` | 0 | [integrity.log](logs/integrity.log) |
| `git add docs/ROADMAP.md docs/superpowers/specs/2026-09-13-forge-roadmap-v2.md scripts/roadmap_gate.py scripts/test_roadmap_gate.py docs/write-cost-explain.md docs/reports/batch-6` | 128 | [git-add-step4.log](logs/git-add-step4.log) |
| `git add -f docs/reports/batch-6/logs` | 128 | [git-add-logs.log](logs/git-add-logs.log) |
| `git commit -F docs/reports/batch-6/commit-4-message.txt` | 128 | [git-commit-step4.log](logs/git-commit-step4.log) |
| `python3 docs/reports/batch-6/check-integrity.py` | 0 | [integrity-final.log](logs/integrity-final.log) |

Development failures were retained: the first disabled-route check observed
405 from the static fallback; the general API 404 fallback corrected it.
A library fixture initially used the wrong deserialisation syntax; changing
it to the pinned compiler's `executeFromVar` exercised the intended path.
Initial Explain assertions exposed shared starts between a comparison and
its child; the documented innermost tie-break resolved that selection case.
The first full workspace run completed the mutation measurement but then
failed the property inventory because an added helper changed byte-pinned
`compile.rs`. That helper was removed, restoring the exact original file.
A conservative quote/parameter check lives in the new source-position module;
no manifest digest changed. The affected checks and full workspace command
were rerun. The required W04 release gate subsequently passed in all recorded invocations.
Final diff review also caught and removed an unintended stale-check edit in
`runTests`; the complete DOM suite and W04 gate passed again afterwards.

## Session limitations and commit preparation

Git staging and commit attempts exited 128 because the worktree Git metadata
was mounted read-only and could not create `index.lock`. No commits or push
were created. Work remained in this worktree, with the required co-author and
session trailers in `commit-1-message.txt` through `commit-4-message.txt`.
No alternate Git directory, main branch or other worktree was used.

The messages group (1) attribution/library/route/feature, (2) Explain route,
(3) UI and DOM tests, and (4) registration/docs/evidence. `lib.rs` belongs to
step 1; `routes/mod.rs` to step 2; `index.html` and `app.js` to step 3. The
router and HTTP target necessarily gain their Explain entries in step 2 after
the independent cost step. Each intended intermediate step compiled.

The unfiltered sandbox feature run could not pass the existing seed-corpus
floor in this checkout. The workspace command used the requested exclusion;
the supplementary feature run used the identical exclusion to test the rest.
No frozen test or digest was re-blessed to make the environment appear green.

The pinned compiler supplied only source starts and supplied no template map.
Parameter string substitution could change positions, so those citations
were withheld. The pinned value recorder truncated long values at 150 bytes;
complete sub-expression residuals could not be recovered from truncated
strings. Such values retained their original rendering and an explicit
`residualNote`, rather than an invented proposition. Root residuals remained
the full run's `reducedTo`. Evaluator-made node copies could lack value records
for the original IR ids; missing values were not inferred from neighbours.
The engine and node revision were not patched to remove those limitations.

UI behavior was verified with linkedom and production-handler DOM tests; no
full browser session was run. No live node or explorer validation was performed
or claimed. The protected-scope check compared against `1d53b2f`, including
reports for batches 1–5, all mutants, mapping/properties manifests, and the
engine pin/lockfile. The v1 policy SHA-256 remained
`45c68423f26d61f290b8b36b44a747015381fd840e86368c9b9a26b4acb2c491`.

## Post-session verification

No post-session verification had occurred when this report was written.
All results above were observed in this implementation session; prepared
commit messages were not evidence that commits had been created.

## Post-session verification

After the implementation session ended (it could not create commits: its
sandbox mounted this worktree's Git metadata read-only, exit 128 on staging
and commit; the commits were created afterwards from its prepared messages,
with the first two folded into one because both register routes in
`app.rs`), the same tree was re-verified outside the sandbox with this batch's
build directory (`$CARGO_TARGET_DIR`). Observed exits:

| Command | Exit |
|---|---:|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo clippy -p ergo-sandbox --all-targets --features cost-trace -- -D warnings` | 0 |
| `python3 scripts/test_roadmap_gate.py` | 0 |
| `python3 scripts/test_lockfile_action.py` | 0 |
| `node --test ui/tests/*.test.js` (40 passed) | 0 |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 |
| `cargo test -p ergo-sandbox --features cost-trace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present --skip the_corpus_is_measured_and_does_not_regress` | 0 |
| `python3 scripts/roadmap_gate.py --require W04 --report docs/reports/batch-6/W04.json` | 0 |

The filtered output of those runs is committed as
[`logs/workspace-post-session.log`](logs/workspace-post-session.log) and
recorded as the `postSession` entries of `commands.json`. The committed
`W04.json` written by that run hashes as `96d8fc8d4a084809870677f1f6db455150895e9571a6c26b3c71a47e0e3a98f8`.
