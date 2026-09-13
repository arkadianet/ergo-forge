# Batch 4 — I01 and I02

Implemented in the `v2/batch-4` worktree based on `a235eed`. The
implementation session could not commit: its sandbox mounted this worktree's
Git metadata read-only and Git could not create `index.lock` (observed exit
128). No push, branch switch or other-worktree modification was attempted. The
four commit messages it prepared are retained in this folder; the commits were
created afterwards from them, as three commits, because the first two share
`lib.rs`, the CLI dispatcher and the router and would not build separately
(see the final section).

The implementation adds:

- `ergo-sandbox/src/verify.rs`, CLI `ergo-es verify`, and
  `POST /api/v1/verify` for offline source/params comparison with address or
  raw tree bytes. Exact/template/no-match have an enum and distinct exits
  (0/3/4; errors 1). Every template constant difference is the unchanged
  `identity::ConstantDifference`; left is compiled source, right is target.
- `ergo-sandbox/src/lockfile.rs`, CLI `ergo-es lock` and `verify-lock`, and
  `POST /api/v1/lock`. Locks record exact-source SHA-256, typed parameters,
  existing pinned compiler/node revision, workbench version, tree bytes,
  P2S address, emitted version, sorted static lint projection SHA-256 and the
  identity limitation. Verification compares all 11 recorded fields
  independently, with distinct drift kinds and expected/recorded values.
- `ui/verify.js` and a Write control, with complete typed differences,
  escaped text, input invalidation and stale-response suppression. Project
  downloads from Write and Build await the lock route and include
  `contract.lock.json`; the existing STORE-only zip writer is unchanged.
- Optional `lockfile` input in `.github/actions/test/action.yml`, backed by
  `verify-lockfiles.py` and CI tests. The action verifies every selected
  project lock against sibling source, params and independent build options.
  Unmatched patterns and nonzero verification fail the job. `version: latest`
  still selects the latest GitHub release; this batch does not publish one.
- I01/I02 implemented registrations in the v2 policy and spec, I01 DOM
  registration in the gate runner, and the complete implemented-set assertion.
  The v1 policy and protected fixture digests are not re-blessed.

See [the complete interface and digest definition](../../immutability.md).
Network and compiler options for verification come from independent inputs
(default mainnet/version 3), so an altered lock address or tree version cannot
conceal its own drift. Project README and CI pass those options explicitly.

All normal verify and lock outputs carry `identity::LIMITATION`. Structural
identity does not establish behavioural equivalence, deployment provenance or
safety. Static findings set review priority; none asserts a vulnerability.
No synthetic result is node-validated. Optional live status remains
`unverified` without an explorer, with missing data, or when a holder is
ambiguous. The real recorded USE box is passed through the same comparison
used for explorer responses; a freshly generated lock with the recorded public key as a typed
SigmaProp parameter matches its P2PK bytes exactly, without replacing a lock
field to force a match. This does not assert historical source provenance.
No explorer access is used in tests, and no live-chain check was performed.
The fixture and unverified path satisfy `explorer-dependency`; no scope
expansion or new roadmap stop is needed.

All required commands have observed exit **0**: formatting, Clippy with warnings
denied, the full workspace test command with only the requested known skip,
the roadmap Python tests, all 21 DOM tests, and both I01/I02 gate commands.
The mutation-corpus measurement completed: all 5 tests passed in 831.63 seconds.
Its long measurement was not interrupted or skipped.

After the workspace build, the recorded-box positive control was strengthened
to require freshly compiled bytes. The final I02 release gate and a targeted
debug run both passed that stronger test; final formatting and Clippy checks
also passed. No production Rust changed after the workspace build.

The first lockfile test run exited 101 because its intended empty-findings
control (`sigmaProp(true)`) actually raises an existing lint. The control was
changed to a public-key proposition; the corrected run passed. The first new
DOM run exited 1 because this fresh worktree lacked linkedom. `npm ci --prefix
ui` installed the existing locked dependency, after which all DOM tests passed.
No lint, matcher or fixture baseline was changed to fix either result.

The read-only boundary check passed for **283 protected files**, including
unchanged engine pins and the hash-frozen v1 policy. Its complete hashes are
in [the boundary log](logs/protected-boundaries.log). [files.json](files.json)
records SHA-256 for each changed/new implementation, test and interface document.

One combined edit/check tool call was rejected before execution because it
included `rm -rf` for generated Python bytecode. It has **no observed exit**;
only the generated file was subsequently removed with a bounded pathlib cleanup,
and the edit/check sequence was run. No test pass is attributed to that rejected
call. All Git staging/commit attempts exited 128 due to read-only Git metadata.

The command ledger below records build, setup, validation and Git commands,
including unsuccessful attempts. Read-only source inspection and file-writing
scaffolding are not test evidence. Commands executed independently during the
long workspace run are marked in [commands.json](commands.json). All builds
and test subprocesses used this batch's `$CARGO_TARGET_DIR`; no other batch's
target directory was used.

<!-- command-ledger -->

| Command | Exit | Log |
|---|---:|---|
| `cargo fmt --all` | 0 | [fmt-step1.log](logs/fmt-step1.log) |
| `cargo test -p ergo-sandbox --test verify_deployment` | 0 | [verify-library.log](logs/verify-library.log) |
| `cargo test -p ergo-web --test verify` | 0 | [verify-http.log](logs/verify-http.log) |
| `git add ergo-sandbox/src/verify.rs ergo-sandbox/src/lib.rs ergo-sandbox/src/bin/ergo-es.rs ergo-sandbox/src/bin/deployment/mod.rs ergo-sandbox/tests/verify_deployment.rs ergo-web/src/routes/verify.rs ergo-web/src/routes/mod.rs ergo-web/src/app.rs ergo-web/tests/verify.rs` | 128 | [git-add-step1.log](logs/git-add-step1.log) |
| `git commit -F docs/reports/batch-4/commit-1-message.txt` | 128 | [git-commit-step1.log](logs/git-commit-step1.log) |
| `cargo fmt --all` | 0 | [fmt-step2.log](logs/fmt-step2.log) |
| `cargo test -p ergo-sandbox --test lockfile` | 101 | [lockfile-tests.log](logs/lockfile-tests.log) |
| `"$CARGO_TARGET_DIR/debug/ergo-es" compile $TMPDIR/b4-pk.es` | 0 | [pk-fixture-compile.log](logs/pk-fixture-compile.log) |
| `cargo fmt --all` | 0 | [fmt-step2-fixed.log](logs/fmt-step2-fixed.log) |
| `cargo test -p ergo-sandbox --test lockfile` | 0 | [lockfile-tests-fixed.log](logs/lockfile-tests-fixed.log) |
| `git add ergo-sandbox/src/lockfile.rs ergo-sandbox/src/lib.rs ergo-sandbox/src/bin/deployment/mod.rs ergo-sandbox/src/bin/ergo-es.rs ergo-sandbox/tests/lockfile.rs` | 128 | [git-add-step2.log](logs/git-add-step2.log) |
| `git commit -F docs/reports/batch-4/commit-2-message.txt` | 128 | [git-commit-step2.log](logs/git-commit-step2.log) |
| `cargo fmt --all` | 0 | [fmt-step3.log](logs/fmt-step3.log) |
| `cargo test -p ergo-web --test lock` | 0 | [lock-http.log](logs/lock-http.log) |
| `node --test ui/tests/verify.test.js` | 1 | [verify-dom.log](logs/verify-dom.log) |
| `python3 scripts/test_lockfile_action.py` | 0 | [lock-ci-tests.log](logs/lock-ci-tests.log) |
| `npm ci --prefix ui --cache $TMPDIR/b4-npm-cache` | 0 | [ui-dependencies.log](logs/ui-dependencies.log) |
| `node --test ui/tests/verify.test.js` | 0 | [verify-dom-installed.log](logs/verify-dom-installed.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | [roadmap-tests.log](logs/roadmap-tests.log) |
| `node --test ui/tests/*.test.js` | 0 | [all-dom.log](logs/all-dom.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy.log](logs/clippy.log) |
| `git add ergo-web/src/routes/lock.rs ergo-web/src/routes/mod.rs ergo-web/src/app.rs ergo-web/tests/lock.rs ui/verify.js ui/tests/verify.test.js ui/index.html ui/app.js .github/actions/test/action.yml .github/actions/test/verify-lockfiles.py .github/workflows/ci.yml scripts/test_lockfile_action.py docs/immutability.md` | 128 | [git-add-step3.log](logs/git-add-step3.log) |
| `git commit -F docs/reports/batch-4/commit-3-message.txt` | 128 | [git-commit-step3.log](logs/git-commit-step3.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-check.log](logs/fmt-check.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 | [workspace-tests.log](logs/workspace-tests.log) |
| `python3 scripts/roadmap_gate.py --require I01 --report docs/reports/batch-4/I01.json` | 0 | [gate-i01.log](logs/gate-i01.log) |
| `python3 scripts/roadmap_gate.py --require I02 --report docs/reports/batch-4/I02.json` | 0 | [gate-i02.log](logs/gate-i02.log) |
| `python3 docs/reports/batch-4/check-boundaries.py` | 0 | [protected-boundaries.log](logs/protected-boundaries.log) |
| `python3 scripts/test_lockfile_action.py` | 0 | [lock-ci-tests-final.log](logs/lock-ci-tests-final.log) |
| `"$CARGO_TARGET_DIR/release/ergo-es" lock "$TMPDIR/b4-recorded-lock/contract.es" --params "$TMPDIR/b4-recorded-lock/params.json" --out "$TMPDIR/b4-recorded-lock/contract.lock.json"` | 0 | [recorded-key-compile.log](logs/recorded-key-compile.log) |
| `cargo fmt --all` | 0 | [fmt-recorded-fixture.log](logs/fmt-recorded-fixture.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-check-final.log](logs/fmt-check-final.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy-final.log](logs/clippy-final.log) |
| `cargo test -p ergo-sandbox --test lockfile` | 0 | [lockfile-final-debug.log](logs/lockfile-final-debug.log) |
| `git diff --check` | 0 | [whitespace.log](logs/whitespace.log) |
| `git add docs/ROADMAP.md docs/superpowers/specs/2026-09-13-forge-roadmap-v2.md scripts/roadmap_gate.py scripts/test_roadmap_gate.py docs/reports/batch-4` | 128 | [git-add-step4.log](logs/git-add-step4.log) |
| `git add -f docs/reports/batch-4/logs` | 128 | [git-add-report-logs.log](logs/git-add-report-logs.log) |
| `git commit -F docs/reports/batch-4/commit-4-message.txt` | 128 | [git-commit-step4.log](logs/git-commit-step4.log) |

## Post-session verification

After the implementation session ended, the same tree was re-verified outside
its sandbox with this batch's build directory (`$CARGO_TARGET_DIR`), before the
commits were created. Observed exits:

| Command | Exit |
|---|---:|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `python3 scripts/test_roadmap_gate.py` | 0 |
| `python3 scripts/test_lockfile_action.py` | 0 |
| `node --test ui/tests/*.test.js` (21 passed) | 0 |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 |
| `python3 scripts/roadmap_gate.py --require I01 --report docs/reports/batch-4/I01.json` | 0 |
| `python3 scripts/roadmap_gate.py --require I02 --report docs/reports/batch-4/I02.json` | 0 |

The filtered output of the workspace run and both gate runs is committed as
[`logs/workspace-post-session.log`](logs/workspace-post-session.log) and
recorded as the `postSession` entry of `commands.json`. The committed gate
reports written by that run hash as:

| Report | SHA-256 |
|---|---|
| `I01.json` | `190901bd27379efb1b431bcb0eb079878b9d937738b67c17c7fd6696bb876225` |
| `I02.json` | `344bec19a0e53927524235110ceb80dbe468b42f8910a387372937344e8e5e08` |
