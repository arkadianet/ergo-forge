# Batch 8 — I03 Watch

Implemented in the `v2/batch-8` worktree based on `1d53b2f`. The I03 roadmap
gate passed, including W00/I01/I02 dependencies and the Watch DOM tests.
All required final checks passed with observed exit 0. The full workspace run
finished in 904.39 seconds; the mutation target passed 5/5 tests in
835.29 seconds. All 38 DOM tests, 30 roadmap tests and four lockfile-action
tests passed. Formatting, workspace Clippy and I03 passed.

Git staging and commits were blocked by read-only worktree metadata. No commit
or push was created; the implementation and three prepared commit messages
remained in this worktree.

## Implementation

- `ergo-sandbox/src/watch.rs`: per-lockfile NFT lists and optional R4–R9 register
  references produce one `WatchReport` per NFT. `lockfile::compare_live` retains
  all singleton/holder/byte decisions. A read adapter captures the exact holder
  response for register observations, with no second box fetch or new matcher.
- Each report records source kind/URL, source height immediately before lookup,
  holder ID through `live.boxId`, lockfile SHA-256 fingerprint, full identity
  limitation, and the explicit statement that it observes the source response,
  without independent chain membership or currentness verification. Script
  statuses are `bytes_match`, `bytes_differ`, `unverified`; register statuses are
  `unchanged`, `changed`, `unverified`. Missing source/height/holder/register data
  remains unverified. Lookups use one holder page, offset 0, limit 2. A batch is
  capped at 64 lockfile/NFT pairs; invalid or excess inputs fail before reads.
- `ergo-es watch`: multiple lockfiles, repeated NFTs, optional registers,
  supplied baseline box, explicit/environment explorer configuration, text/JSON
  output. Each CLI NFT applies to each CLI lockfile. Exits are 0 for all matching
  bytes and unchanged registers, 3 for change, 4 for any unverified observation
  (precedence over 3), and 1 for input errors. Usage documents these exits.
- `POST /api/v1/watch`: inline `watches` with per-lockfile inputs and optional
  `baselineBoxes` keyed by NFT. It uses only `state.cfg.explorer_url`, shares the
  body cap/rate limiter/engine budget, rejects request URLs/fixture overrides,
  and returns HTTP 200 unverified reports when no explorer is configured.
- `ui/watch.js` and Read controls: pasted/uploaded lockfile, NFT IDs, optional
  register names and baseline box, per-NFT observations with reasons and
  provenance. First complete register observations become per-NFT baselines in
  page memory; later clicks reuse them. Input edits/reset/reload clear them.
  Uploads clear the old lockfile and disable Watch until the file is read.
  Aborted, failed, incomplete and stale responses cannot show old observations.
  The control is disabled/explained without explorer configuration. No CDN or
  server-side persistence was added. Nothing is signed or broadcast.
- `docs/immutability.md` documents the API, fingerprint, baseline lifecycle,
  source limits and CLI exits. The lockfile schema was unchanged.

Shared edits were local: two lines in `ui/app.js`, one Read section and one
script include in `ui/index.html`, one I03 entry in `extras()`, one new gate
unit test and only I03 added to its implemented-set assertion. Both v2 records
set I03 implemented. No batch 6/7 feature surface was changed.

## Validation and command ledger

Every validation and Git command was captured by `run-command.py` with its
observed exit, duration and portable log in `commands.json`. The build directory
was this batch's `$CARGO_TARGET_DIR` throughout. The gate's ten internal commands,
exit codes, full output and output hashes are also retained in `I03.json`.

<!-- command-table -->

| Command | Observed exit | Log |
|---|---:|---|
| `cargo fmt --all` | 0 | [fmt-step1](logs/fmt-step1.log) |
| `cargo test -p ergo-sandbox --test watch -p ergo-web --test watch` | 0 | [watch-rust](logs/watch-rust.log) |
| `git add Cargo.lock ergo-sandbox/src/watch.rs ergo-sandbox/src/lib.rs ergo-sandbox/src/bin/ergo-es.rs ergo-sandbox/src/bin/watching/mod.rs ergo-sandbox/tests/watch.rs ergo-sandbox/tests/fixtures/watch/holder.json ergo-web/Cargo.toml ergo-web/src/routes/watch.rs ergo-web/src/routes/mod.rs ergo-web/src/app.rs ergo-web/tests/watch.rs docs/immutability.md` | 128 | [git-add-step1](logs/git-add-step1.log) |
| `git commit -F docs/reports/batch-8/commit-1-message.txt` | 128 | [git-commit-step1](logs/git-commit-step1.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy](logs/clippy.log) |
| `node --test ui/tests/watch.test.js` | 0 | [watch-dom](logs/watch-dom.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 101 | [workspace](logs/workspace.log) |
| `git add ui/watch.js ui/tests/watch.test.js ui/app.js ui/index.html` | 128 | [git-add-step2](logs/git-add-step2.log) |
| `git commit -F docs/reports/batch-8/commit-2-message.txt` | 128 | [git-commit-step2](logs/git-commit-step2.log) |
| `python3 scripts/test_lockfile_action.py` | 0 | [lock-action-tests](logs/lock-action-tests.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | [roadmap-tests](logs/roadmap-tests.log) |
| `cargo test -p ergo-sandbox --test decompile_corpus` | 0 | [corpus-fixture-boundary](logs/corpus-fixture-boundary.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/evidence-replay.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js ui/tests/watch.test.js` | 0 | [dom-all](logs/dom-all.log) |
| `python3 scripts/roadmap_gate.py --require I03 --report docs/reports/batch-8/I03.json` | 0 | [gate-i03](logs/gate-i03.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy-final](logs/clippy-final.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-check](logs/fmt-check.log) |
| `python3 docs/reports/batch-8/check-integrity.py` | 0 | [integrity](logs/integrity.log) |
| `git diff --check` | 0 | [whitespace](logs/whitespace.log) |
| `cargo test -p ergo-sandbox --no-default-features --test watch` | 0 | [watch-no-default-features](logs/watch-no-default-features.log) |
| `cargo build --workspace --bins` | 0 | [restore-workspace-binaries](logs/restore-workspace-binaries.log) |
| `python3 docs/reports/batch-8/check-integrity.py` | 0 | [integrity-report](logs/integrity-report.log) |
| `node --test ui/tests/attack.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/evidence-replay.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js ui/tests/watch.test.js` | 0 | [dom-all-final](logs/dom-all-final.log) |
| `python3 scripts/roadmap_gate.py --require I03 --report docs/reports/batch-8/I03.json` | 0 | [gate-i03-final](logs/gate-i03-final.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 101 | [workspace-final](logs/workspace-final.log) |
| `cargo fmt --all` | 0 | [fmt-http-memory](logs/fmt-http-memory.log) |
| `cargo test --locked -p ergo-sandbox --test property_inventory -p ergo-web --test watch` | 0 | [property-and-watch](logs/property-and-watch.log) |
| `cargo fmt --all -- --check` | 0 | [fmt-check-final](logs/fmt-check-final.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy-memory-http](logs/clippy-memory-http.log) |
| `python3 scripts/roadmap_gate.py --require I03 --report docs/reports/batch-8/I03.json` | 0 | [gate-i03-locked-dependencies](logs/gate-i03-locked-dependencies.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 | [workspace-complete](logs/workspace-complete.log) |
| `python3 docs/reports/batch-8/check-integrity.py` | 0 | [integrity-final](logs/integrity-final.log) |
| `git diff --check` | 0 | [whitespace-final](logs/whitespace-final.log) |
| `git add docs/ROADMAP.md docs/superpowers/specs/2026-09-13-forge-roadmap-v2.md scripts/roadmap_gate.py scripts/test_roadmap_gate.py docs/reports/batch-8` | 128 | [git-add-step3](logs/git-add-step3.log) |
| `git add -f docs/reports/batch-8/logs` | 128 | [git-add-logs](logs/git-add-logs.log) |
| `git commit -F docs/reports/batch-8/commit-3-message.txt` | 128 | [git-commit-step3](logs/git-commit-step3.log) |
| `git status --short` | 0 | [git-status-final](logs/git-status-final.log) |

<!-- /command-table -->

The first workspace test run exited 101 before the mutation measurement: the
new `holder.json` recording was automatically included in the frozen decompiler
corpus inventory. The new recording was renamed to `holder.fixture`, retaining
JSON content and using the existing USE recording convention. The unchanged
corpus test then exited 0. No corpus expectation or baseline digest was edited.

The next workspace run completed the mutation measurement (5/5 tests in
804.33 seconds), then exited 101 at `property_inventory`: the test-only `tower`
dependency had changed the frozen `Cargo.lock` hash. The dependency was removed
and `Cargo.lock` restored byte-for-byte. HTTP tests then used Tokio's existing
in-memory duplex streams through the real Axum router, with no sockets or new
dependency; requests reused the same application state when checking that no
baseline persisted. The property inventory and Watch targets passed with
`--locked`. Both package manifests and `Cargo.lock` remained unchanged in the
final implementation; no pinned baseline hash was edited.

The final required workspace command exited 0 in 904.39 seconds.
It included the complete mutation measurement (5/5 passed in 835.29 seconds),
the restored property inventory, both Watch targets, all later targets and
doctests. The requested seed-corpus skip and pre-existing ignored tests were
left unchanged. No command was interrupted; every command in the ledger has an
observed exit.

Watch coverage used this batch's synthetic holder recording, the pinned
compiler's matching P2PK bytes, tree/register mutations, zero/multiple holders,
inconsistent singleton/token metadata, missing/invalid data, height failure,
input validation, lock fingerprints, batch exit precedence and CLI errors.
The recording source asserted the exact `height`, `token_info`, and bounded
`boxes_by_token_id` calls; unexpected read categories failed the test. Source
checks covered the library, route and CLI for signing/broadcast references.
The two HTTP tests used in-memory HTTP against the real router with no sockets
or network.
Five Watch DOM tests covered rendering, explorer gating, baseline retention,
uploads, request failure/staleness and actual app configuration wiring.

## Scope and session limitations

Git could not create `index.lock` because this worktree's Git metadata was
mounted read-only. Staging and commit attempts exited 128. The three messages
with the required co-author/session trailers were retained as
`commit-1-message.txt`, `commit-2-message.txt`, and `commit-3-message.txt`.
`commit-files.json` assigns each final file to one of those steps. The report
logs matched the repository ignore rule, so their staging command used `-f`.
Step 1 groups the library, CLI, route, Rust tests, fixture and documentation;
step 2 groups the UI and DOM tests; step 3 groups registration and reports.
No alternate Git directory, branch, worktree or push was used.

The requested seed-corpus test was skipped for the known local checkout
mismatch; its implementation was unchanged. No live explorer was contacted or
claimed to have been verified. Watch's fixture and runtime paths used the same
I02 comparison function. Browser behavior was checked with linkedom; a full
browser session was not run. Supplied baseline boxes remained caller references,
and a first observation established no earlier history. Register omission could
not be distinguished from absence by ChainBox and therefore stayed unverified.

The read-only `check-integrity.py` confirmed unchanged protected source files,
`Cargo.lock`, package manifests, lockfile schema, mapping/properties manifests,
decompiler corpus expectations, CI workflow and batch-1..7 reports. `roadmap-policy:v1` remained byte-identical,
SHA-256 `45c68423f26d61f290b8b36b44a747015381fd840e86368c9b9a26b4acb2c491`.
No digest was re-blessed. I03 met its gate; no stop record was required.

## Post-session verification

No post-session validation was performed during this implementation session.
`commands.json` retains an empty `postSession` array for later handoff evidence.

## Post-session verification

After the implementation session ended, the same tree was re-verified outside
its sandbox with this batch's build directory (`$CARGO_TARGET_DIR`), before the
commits were created from the three prepared messages. Observed exits:

| Command | Exit |
|---|---:|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `python3 scripts/test_roadmap_gate.py` | 0 |
| `python3 scripts/test_lockfile_action.py` | 0 |
| `node --test ui/tests/*.test.js` (38 passed) | 0 |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 |
| `python3 scripts/roadmap_gate.py --require I03 --report docs/reports/batch-8/I03.json` | 0 |

The filtered output is committed as
[`logs/workspace-post-session.log`](logs/workspace-post-session.log) and
recorded as the `postSession` entry of `commands.json`. After the branch was
stacked on batch 7 (batch 6 merged into main), clippy, the workspace suite and
the gate were run again on that tree and exited 0; the committed
`I03.json` written by that run hashes as `14e74c49b188a5f8b8a7e480e5bc3e49a9852e18ea1f82f5bb1787fce5c1becc`.
