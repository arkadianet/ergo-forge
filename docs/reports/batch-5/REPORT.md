# Batch 5 — W02 and W03

Implemented in the `v2/batch-5` worktree based on `126567f`. The required
W02 and W03 roadmap gates passed. Formatting, Clippy with warnings denied,
29 roadmap Python tests, 4 lockfile-action tests and all 33 DOM tests passed.
The workspace test command passed with observed exit 0.
The unchanged mutation-corpus measurement completed: 5/5 tests passed in 842.89 seconds.

## What landed in the worktree

- `ergo-sandbox/src/play_export.rs`: `export(request, input_index, verdict)`
  returns one `Suite`/`Case` and an equivalent standalone `Scenario`.
  `play.rs` shares context preparation with the exporter: transaction input
  order, `selfIndex`, resolved data inputs, selected context variables, network,
  height, output simulation IDs and default creation heights are preserved.
  The chosen input's exact ErgoTree is the only suite contract. No source or
  params are exported. Signing secrets/parties are removed and an unsigned
  expectation is explicitly named. Unsigned disagreement is refused as
  `synthetic-drift`.
- `POST /api/v1/play/export` in `ergo-web/src/routes/play_export.rs`, registered
  with the existing Play engine budget, rate limiter and 1 MiB body limit.
  It accepts the Play request, zero-based `inputIndex`, and `kind` (`test` or
  `scenario`), then returns the JSON document directly.
- `ui/play-export.js`, `ui/app.js` and `ui/index.html`: per-input **Save as
  test** / **Save as scenario** downloads, retaining the exact request snapshot
  after a successful spend. Results remain visible when the transaction form
  closes. Filenames are `contract.test.json` and `scenario.json`.
- `ui/share.js`: offline base64url JSON codecs for unchanged Write fragments,
  `{k:"play",v:1,height,network,boxes,history?}` and `{k:"suite",v:1,suite}`.
  The 7168-byte cap includes `#s=` and reports actual/cap bytes on creation and
  loading. Rust's `ergo-web/tests/share_links.rs` executes this exact codec with
  Node; no server share endpoint, storage, CDN or fetching was introduced.
- Play share controls and explicit replacement confirmation; cancellation and
  initial loading do not write the shared chain to localStorage. Imported
  chains render offline and display their network. Shared suites land intact
  in Write's Tests area, where the existing `POST /api/v1/test` runner uses the
  document's own contract. Existing scenarios arrays still use Read/Write.
- Exact W02/W03 test targets in `ergo-web/tests/play_export.rs` and
  `ergo-web/tests/share_links.rs`; standalone DOM tests in
  `ui/tests/play-export.test.js` and `ui/tests/share.test.js`; `extras()` gate
  registration and a Python test rejecting missing, failed and skipped DOM
  runs. W02/W03 are `implemented: true` in both complete v2 records, and the
  implemented-set assertion contains all ten requested units.
- [Interface documentation](../../play-sharing.md), this ledger, gate reports,
  CLI replay artifacts and four prepared commit messages.

Exports and shared experiments are labelled synthetic, with
`nodeValidated: false`. Verdicts are sandbox verdicts. Static findings set
review priority and do not establish vulnerabilities. Existing CLI commands
were used unchanged; no new CLI command or runner semantics were introduced.

## Evidence and synthetic drift

The HTTP tests reproduced pass, fail, error and needs-proof verdicts at input
index 1 despite another input refusing. They compared costs, resolved input
order, context variables, data inputs and the generated outputs. Signing tests
covered both secrets and parties and ran the exported unsigned expectations.
Bad indices, missing boxes, unknown kinds and oversized requests were refused.

The actual binaries also consumed ten HTTP export documents unchanged (five
cases, two commands each). Every `ergo-es test` and `ergo-es eval` subprocess
exited 0 and matched the expected sandbox verdict. The signed case changed
from Play's `proofAccepted` to the documented signature-less `needsProof`;
this was the requested removal of signing material. No synthetic drift was
observed, and no new stop record was needed.

The retained [CLI reproducer](cli-roundtrip.py), [documents](cli-artifacts/)
and [subprocess ledger](cli-commands.json) make these checks repeatable using
`$CARGO_TARGET_DIR/debug/ergo-es` and `$CARGO_TARGET_DIR/debug/ergo-web`.

Both required release gates include W00/W01 dependencies, exact test discovery,
execution and DOM extras. Each gate's seven subprocesses exited 0; full commands,
outputs and output hashes are embedded in [W02.json](W02.json) and
[W03.json](W03.json).

## Commands and observed exits

All builds used this batch's `$CARGO_TARGET_DIR`. The command wrapper recorded
stdout/stderr and the actual process exit, including failed attempts. The
[machine-readable ledger](commands.json) records elapsed time and log paths.
The CLI subprocesses are listed separately below; gate subprocesses are
retained verbatim in the two gate reports. No command was interrupted.

| Command | Observed exit | Log |
|---|---:|---|
| `cargo fmt --all` | 0 | [fmt-step1.log](logs/fmt-step1.log) |
| `cargo test -p ergo-web --test play_export` | 0 | [export-http.log](logs/export-http.log) |
| `git add ergo-sandbox/src/lib.rs ergo-sandbox/src/play.rs ergo-sandbox/src/play_export.rs ergo-web/src/app.rs ergo-web/src/routes/mod.rs ergo-web/src/routes/play_export.rs ergo-web/tests/play_export.rs` | 128 | [git-add-step1.log](logs/git-add-step1.log) |
| `git commit -F docs/reports/batch-5/commit-1-message.txt` | 128 | [git-commit-step1.log](logs/git-commit-step1.log) |
| `cargo fmt --all` | 0 | [fmt-step2.log](logs/fmt-step2.log) |
| `cargo test -p ergo-web --test share_links` | 101 | [share-rust.log](logs/share-rust.log) |
| `cargo test -p ergo-web --test share_links` | 0 | [share-rust-boundary.log](logs/share-rust-boundary.log) |
| `git add ui/share.js ergo-web/tests/share_links.rs` | 128 | [git-add-step2.log](logs/git-add-step2.log) |
| `git commit -F docs/reports/batch-5/commit-2-message.txt` | 128 | [git-commit-step2.log](logs/git-commit-step2.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 | [workspace.log](logs/workspace.log) |
| `node --test ui/tests/play-export.test.js ui/tests/share.test.js` | 0 | [dom-new.log](logs/dom-new.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy.log](logs/clippy.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | [roadmap-tests.log](logs/roadmap-tests.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/evidence-replay.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 0 | [dom-all.log](logs/dom-all.log) |
| `python3 scripts/test_lockfile_action.py` | 0 | [lock-action-tests.log](logs/lock-action-tests.log) |
| `python3 scripts/roadmap_gate.py --require W02 --report docs/reports/batch-5/W02.json` | 0 | [gate-w02.log](logs/gate-w02.log) |
| `cargo build -p ergo-sandbox -p ergo-web --bins` | 0 | [build-cli-http.log](logs/build-cli-http.log) |
| `python3 docs/reports/batch-5/cli-roundtrip.py` | 0 | [cli-roundtrip.log](logs/cli-roundtrip.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-check.log](logs/fmt-check.log) |
| `python3 scripts/roadmap_gate.py --require W03 --report docs/reports/batch-5/W03.json` | 0 | [gate-w03.log](logs/gate-w03.log) |
| `git add ui/app.js ui/index.html ui/play-export.js ui/tests/play-export.test.js ui/tests/share.test.js docs/play-sharing.md` | 128 | [git-add-step3.log](logs/git-add-step3.log) |
| `git commit -F docs/reports/batch-5/commit-3-message.txt` | 128 | [git-commit-step3.log](logs/git-commit-step3.log) |
| `node --test ui/tests/share.test.js` | 0 | [dom-share-final.log](logs/dom-share-final.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/evidence-replay.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 0 | [dom-all-final.log](logs/dom-all-final.log) |
| `python3 scripts/roadmap_gate.py --require W03 --report docs/reports/batch-5/W03.json` | 0 | [gate-w03-final.log](logs/gate-w03-final.log) |
| `git diff --check` | 0 | [whitespace.log](logs/whitespace.log) |
| `python3 docs/reports/batch-5/check-integrity.py` | 0 | [integrity.log](logs/integrity.log) |
| `git diff --stat` | 0 | [final-diff.log](logs/final-diff.log) |
| `git add docs/ROADMAP.md docs/superpowers/specs/2026-09-13-forge-roadmap-v2.md scripts/roadmap_gate.py scripts/test_roadmap_gate.py docs/reports/batch-5` | 128 | [git-add-step4.log](logs/git-add-step4.log) |
| `git add -f docs/reports/batch-5/logs` | 128 | [git-add-logs.log](logs/git-add-logs.log) |
| `git commit -F docs/reports/batch-5/commit-4-message.txt` | 128 | [git-commit-step4.log](logs/git-commit-step4.log) |
| `python3 docs/reports/batch-5/check-integrity.py` | 0 | [integrity-final.log](logs/integrity-final.log) |

| CLI / fixture server command | Observed exit | Result |
|---|---:|---|
| `$CARGO_TARGET_DIR/debug/ergo-es test docs/reports/batch-5/cli-artifacts/pass/contract.test.json --json` | 0 | pass |
| `$CARGO_TARGET_DIR/debug/ergo-es eval docs/reports/batch-5/cli-artifacts/pass/scenario.json --json` | 0 | pass |
| `$CARGO_TARGET_DIR/debug/ergo-es test docs/reports/batch-5/cli-artifacts/fail/contract.test.json --json` | 0 | fail |
| `$CARGO_TARGET_DIR/debug/ergo-es eval docs/reports/batch-5/cli-artifacts/fail/scenario.json --json` | 0 | fail |
| `$CARGO_TARGET_DIR/debug/ergo-es test docs/reports/batch-5/cli-artifacts/error/contract.test.json --json` | 0 | error |
| `$CARGO_TARGET_DIR/debug/ergo-es eval docs/reports/batch-5/cli-artifacts/error/scenario.json --json` | 0 | error |
| `$CARGO_TARGET_DIR/debug/ergo-es test docs/reports/batch-5/cli-artifacts/needsProof/contract.test.json --json` | 0 | needsProof |
| `$CARGO_TARGET_DIR/debug/ergo-es eval docs/reports/batch-5/cli-artifacts/needsProof/scenario.json --json` | 0 | needsProof |
| `$CARGO_TARGET_DIR/debug/ergo-es test docs/reports/batch-5/cli-artifacts/signed/contract.test.json --json` | 0 | needsProof |
| `$CARGO_TARGET_DIR/debug/ergo-es eval docs/reports/batch-5/cli-artifacts/signed/scenario.json --json` | 0 | needsProof |
| `$CARGO_TARGET_DIR/debug/ergo-web` | 0 | fixture server; graceful SIGTERM after HTTP checks |

The first share Rust target run exited 101: its boundary fixture assumed
base64url could occupy every byte length. Some lengths are unrepresentable.
The fixture was corrected to choose the largest encodable payload within the
unchanged cap, then check that the next payload is refused. The target and the
required W03 gate subsequently passed. This was not a Play/CLI verdict drift.

## Scope and session limitations

Git could not create `index.lock` because the worktree Git metadata was mounted
read-only. Staging and commit attempts exited 128. No commit or push was
created; all changes remained in this worktree and all four commit messages
with the required co-author/session trailers were retained. The messages group
export library/routes, share codec/gates, UI/DOM work, and registration/reports.
The shared Rust module/router files belong to step 1 and app/index to step 3.
The final versions of each step's files were retained for staging after the
session; no alternate Git directory or other worktree was used.

The requested seed-corpus test was skipped for the known local checkout
mismatch. Its implementation was unchanged. Browser behavior was verified with
linkedom DOM tests, including production handlers and offline rendering; a
full Chromium session was not run. Sharing large documents remained bounded
by the documented fragment cap; Play history retained the existing text log.
No live node or explorer validation was performed or claimed by these units.

The read-only [integrity check](check-integrity.py) confirmed the branch/head,
unchanged protected report folders and mutants, unchanged mapping/properties
manifests, and byte-identical `roadmap-policy:v1`. Its SHA-256 remained
`45c68423f26d61f290b8b36b44a747015381fd840e86368c9b9a26b4acb2c491`.
No baseline digest was re-blessed. Report paths use portable placeholders.

## Post-session verification

After the implementation session ended (it could not create commits: its
sandbox mounted this worktree's Git metadata read-only, exit 128 on staging
and commit; the commits were created afterwards from its four prepared
messages), the same tree was re-verified outside the sandbox with this batch's
build directory (`$CARGO_TARGET_DIR`). Observed exits:

| Command | Exit |
|---|---:|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `python3 scripts/test_roadmap_gate.py` | 0 |
| `python3 scripts/test_lockfile_action.py` | 0 |
| `node --test ui/tests/*.test.js` (33 passed) | 0 |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 |
| `python3 scripts/roadmap_gate.py --require W02 --report docs/reports/batch-5/W02.json` | 0 |
| `python3 scripts/roadmap_gate.py --require W03 --report docs/reports/batch-5/W03.json` | 0 |

The filtered output of the workspace run and both gate runs is committed as
[`logs/workspace-post-session.log`](logs/workspace-post-session.log) and
recorded as the `postSession` entry of `commands.json`. The committed gate
reports written by that run hash as:

| Report | SHA-256 |
|---|---|
| `W02.json` | `d874c601f9769e92cf2968a95bafc1d804830ea0e564e0ac42847afd2ebef815` |
| `W03.json` | `4abc1e4b9a8948655bda2157be549f082806a198c2006c13561311f46cf68abe` |
