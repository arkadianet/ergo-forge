# Batch 3 — S01 and W06

Implemented in the `v2/batch-3` working tree based on `137fc8b`. The
implementation session could not commit: its sandbox marked this worktree's Git
metadata read-only, and staging and committing each returned 128 while creating
`index.lock`. No push, branch switch, or other-worktree modification was
attempted. The four logical commit messages it prepared, including the requested
trailers, are retained here as `commit-1-message.txt` through
`commit-4-message.txt`; the commits were created afterwards from those messages
once the tree had been re-verified (see the final section).

The checklist library (`ergo-sandbox/src/checklist.rs`),
`POST /api/v1/checklist` route, and `ergo-es checklist` CLI list all 15 S00 vectors.
Every answer carries provenance, static observations keep their lint/node/IR
anchors and snippets, and unchecked rows remain visible. There is no score.
The HTTP route uses inspect's network and address/hex handling, adds an explicit
source field, and runs compilation and analysis under the shared engine permit,
large stack, rate limiter and input/body limits.

Supplied artifact envelopes associate explicit vector IDs with scenarios,
preflights or P05 replay bundles. Scenarios and preflights run through the
existing engines and remain `nodeValidated: false`. Fresh P05 replay plus the
exact contract on a property-bound spending input is required for a
node-validated row. Bundle results retain their premise and property scope;
caller vector associations do not establish causation. Failed/incomplete
bundles remain unchecked evidence. Raw scenario/bundle CLI files also work,
with no vector attribution unless an association is supplied. The response's
top-level claim always remains static and unvalidated. See
[the interface and artifact format](../../security/CHECKLIST.md).

`ui/checklist.js` renders the list in Write and Read from the new route, with
visible provenance and anchors, escaped text and stale-response handling. It is
plain local JavaScript. `ui/tests/checklist.test.js` exercises the module and the
shipped Read/Write handlers using linkedom. No CDN dependency was added; linkedom
was already installed, so `npm ci` was unnecessary.

`ergo-sandbox/src/negative_space.rs` projects existing catalogue-named lint
findings without new analysis. Inspect exposes the lines as `negativeSpace` so
plain words and negative space share the same audit/recovery. Read renders them
immediately below plain words, each with a visible static label, node/IR anchor
and snippet. No observations never means all constraints are checked.

The exact HTTP gate tests are in `ergo-web/tests/checklist.rs` and
`ergo-web/tests/negative_space.rs`. Additional CLI/HTTP/DOM checks cover source
and address input, the S00 zero-threshold positive/control and stale-oracle
scenario, the existing P05 USE bundle, preflight, malformed or unrelated input,
limits, and stale UI responses. S01's DOM test is registered in gate `extras()`
and tested by the runner's Python suite. S01 and W06 are marked implemented in
both complete v2 policy records. The gate normalizer also renders an absolute
build directory as `$CARGO_TARGET_DIR` before hashing reports.

The integrity check compares protected paths with `137fc8b`: no changes to
`roadmap-policy:v1`, either frozen manifest, `claim.rs`, evidence code, lint code,
the S00 catalogue, `examples/mutants/`, or batch-2 reports. The catalogue needed
no new fields, so no regeneration or fixture re-blessing was necessary.
[Integrity transcript](logs/protected-integrity.log);
[reproducible check](check-boundaries.py).

The final [S01 gate](S01.json) and [W06 gate](W06.json) both exited 0, including
all required predecessor tests and S01's DOM test, with no ignored gate tests.
The initial interrupted W06 attempt is retained separately; no stop record was
needed after the unchanged retry passed. [File inventory and hashes](files.json)
identify every added or modified implementation/documentation file.

All Cargo and gate commands used this batch's assigned build directory,
rendered in these artifacts as `$CARGO_TARGET_DIR`. No target-p00 build was used.
`commands.json` records observed exit statuses and links to complete logs.
Gate JSON files additionally include each nested command, exit status, normalized
output and its SHA-256. Read-only discovery commands are summarized above;
the table records build, validation, formatting and Git-write attempts.

The literal required `node --test ui/tests/` invocation exited 1: Node v22.22.2
tried to load the directory as a module. Running all four explicit test files
executed all 17 tests and exited 0. No source/test shim or runtime change was
introduced to disguise that environment incompatibility.

Two early development checks failed and were corrected: the first HTTP run
assumed the P05 case target was present, while that fixture supplies its actual
spending inputs instead; the checklist now handles absent optional case targets
and checks the property-bound spending inputs. The first DOM run used a select
value setter that linkedom does not implement; selecting the actual option
fixed the test setup. Their logs and exits are retained. The first W06 gate attempt exited 1 because its S02 Cargo subprocess ended
with SIGTERM (`-15`), despite printing a passing five-test summary. That child
has **no observed exit** (SIGTERM), so no gate pass is claimed for the attempt.
Its original report is retained as `W06-initial-interrupted.json` and its complete
output as `logs/gate-w06.log`. The unchanged gate was retried once; the table
records that outcome separately.

The required full workspace command also ended with SIGTERM after the debug
historical mutation measurement printed `5 passed; 0 failed` (951.35 seconds).
Its Cargo parent has **no observed exit**; the wrapper's OS marker is `-15`
(the wrapper shell exposed 241). This is not a passing workspace command.
The supplemental workspace run omits that already-executed measurement as well
as the user-specified seed mismatch. It exited 0, including the remaining HTTP and documentation tests; no tests or fixtures were changed to omit the measurement.


| Command | Observed exit | Log |
|---|---:|---|
| `cargo fmt --all` | 0 | [fmt-apply.log](logs/fmt-apply.log) |
| `cargo test -p ergo-web --test http --no-run` | 0 | [checklist-build.log](logs/checklist-build.log) |
| `cargo test -p ergo-web --test checklist` | 101 | [checklist-http.log](logs/checklist-http.log) |
| `cargo test -p ergo-web --test checklist` | 0 | [checklist-http-fixed.log](logs/checklist-http-fixed.log) |
| `cargo fmt --all` | 0 | [fmt-step1.log](logs/fmt-step1.log) |
| `cargo test -p ergo-sandbox --test checklist_cli` | 0 | [checklist-cli.log](logs/checklist-cli.log) |
| `cargo fmt --all` | 0 | [fmt-step1-final.log](logs/fmt-step1-final.log) |
| `git add ergo-sandbox/src/checklist.rs ergo-sandbox/src/lib.rs ergo-sandbox/src/bin/ergo-es.rs ergo-sandbox/tests/checklist_cli.rs ergo-web/src/routes/checklist.rs ergo-web/src/routes/mod.rs ergo-web/src/app.rs ergo-web/src/dto.rs ergo-web/tests/checklist.rs docs/security/CHECKLIST.md` | 128 | [git-add-step1.log](logs/git-add-step1.log) |
| `git commit -F docs/reports/batch-3/commit-1-message.txt` | 128 | [git-commit-step1.log](logs/git-commit-step1.log) |
| `node --test ui/tests/checklist.test.js` | 1 | [checklist-dom.log](logs/checklist-dom.log) |
| `cargo fmt --all` | 0 | [fmt-step3.log](logs/fmt-step3.log) |
| `cargo test -p ergo-web --test negative_space` | 0 | [negative-space-http.log](logs/negative-space-http.log) |
| `node --test ui/tests/checklist.test.js` | 0 | [checklist-dom-fixed.log](logs/checklist-dom-fixed.log) |
| `cargo fmt --all` | 0 | [fmt-final-apply.log](logs/fmt-final-apply.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-check.log](logs/fmt-check.log) |
| `node --test ui/tests/` | 1 | [ui-all.log](logs/ui-all.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | [roadmap-python.log](logs/roadmap-python.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy.log](logs/clippy.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | [roadmap-python-final.log](logs/roadmap-python-final.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/evidence-replay.test.js` | 0 | [ui-all-files.log](logs/ui-all-files.log) |
| `python3 $TMPDIR/batch3-integrity.py` | 0 | [protected-integrity.log](logs/protected-integrity.log) |
| `git diff --check` | 0 | [whitespace.log](logs/whitespace.log) |
| `python3 scripts/roadmap_gate.py --require S01 --report docs/reports/batch-3/S01.json` | 0 | [gate-s01.log](logs/gate-s01.log) |
| `python3 scripts/roadmap_gate.py --require W06 --report docs/reports/batch-3/W06.json` | 1 | [gate-w06.log](logs/gate-w06.log) |
| `python3 scripts/roadmap_gate.py --require W06 --report docs/reports/batch-3/W06.json` | 0 | [gate-w06-retry.log](logs/gate-w06-retry.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | no observed exit | [workspace-tests.log](logs/workspace-tests.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present --skip the_corpus_is_measured_and_does_not_regress` | 0 | [workspace-supplement.log](logs/workspace-supplement.log) |
| `python3 docs/reports/batch-3/check-boundaries.py` | 0 | [protected-integrity-final.log](logs/protected-integrity-final.log) |
| `git diff --check` | 0 | [whitespace-final.log](logs/whitespace-final.log) |

## Post-session verification

After the implementation session ended, the same tree was re-verified outside
its sandbox with this batch's build directory (`$CARGO_TARGET_DIR`), before the
commits were created. Observed exits:

| Command | Exit |
|---|---:|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `python3 scripts/test_roadmap_gate.py` | 0 |
| `node --test ui/tests/*.test.js` (17 passed) | 0 |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 |
| `python3 scripts/roadmap_gate.py --require S01 --report docs/reports/batch-3/S01.json` | 0 |
| `python3 scripts/roadmap_gate.py --require W06 --report docs/reports/batch-3/W06.json` | 0 |

The full workspace command that ended with SIGTERM inside the session therefore
has an observed passing run here. The committed `S01.json` and `W06.json` are
the reports from this run; the session's interrupted first W06 attempt is kept
as `W06-initial-interrupted.json`.
