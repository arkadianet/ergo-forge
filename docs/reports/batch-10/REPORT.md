# Batch 10 — X01, X02, X03

Prepared on `v2/batch-10`, based on `a18999a`, on 2026-09-14. The work remained
uncommitted: staging and all four commit attempts exited 128 because the
sandbox made the worktree index read-only. Four commit messages with the exact
requested co-author and session trailers are retained in this directory.
Nothing was pushed, tagged or published; no container was pushed. No other
branch or worktree was modified.

Every required validation command completed with exit 0.

## Prepared changes

- X01: all eight Cargo dependency declarations and all nine node packages in
  Cargo.lock advance from `9468043396e5daa2828211bcff4234bc70fae4f0` to
  `016533194f94ad95b1a87df70bb9bfce493922e2`. Both the read-only node checkout's
  HEAD and origin/main matched the latter; origin identified arkadianet/ergo.
  The node packages changed from 0.6.0 to 0.7.0. No unrelated dependency or
  dependency edge changed in the final lockfile.
- The new `node_corpus_expectations.json` pins all 92 eligible seed rows and
  279 mainnet trees individually, including non-exact outcomes. The two floor
  tests now require 79 and 270 exact rows, and compare every original tree
  hash and exact disposition. The historical 73-row compiled subset, the
  339-row v1 decompiler expectations and W05 expectations remain byte-identical.
- `node_vectors_pass_on_new_rev` verifies the measured new revision, agreement
  across the Cargo declarations and recorded engine revision, and all original
  P03 expected outcomes. `engine_revision()` already read Cargo.toml and stayed
  unchanged. Hunt's rule-basis metadata and the wire-vector generator now use
  that same source instead of a repeated live revision literal.
- X02: the inherited workspace version is 0.5.0. CHANGELOG.md names every
  implemented v2 unit and every batch 0–9, plus batch 10, with historical tag
  sections grounded in local git history. README/action tag examples use
  v0.5.0. Existing v0.4.0/v0.4.1 trees still declared crate version 0.3.0.
- The existing release workflow and action download logic required no code
  change: assets are uploaded for the tag, and the action selects a named tag
  or latest release. The current CLI help contains `verify-lock`; its existing
  compatibility probe will pass once v0.5.0 is the latest release. The new
  Python release gates parse the workflow/action YAML offline. Cutting the
  tag after merge remained the maintainer's action.
- X03: both Rust verification jobs cache the sibling `ergo/` checkout using
  the pinned revision and only check it out on a cache miss. The cache action
  uses verified v4.3.0 SHA `0057852bfaa89a56745cba8c7296529d2fc39830`.
  Cost-trace clippy/tests run in their own job with no dependency on the
  default verification job. Both install Node/DOM dependencies and cache Rust
  builds. Fmt, default clippy/workspace tests, web without cost-trace, the
  lockfile action runner, completed-prefix/P08/all-goal gates remain. Parsed
  YAML tests enforce this topology and the retained checks. PyYAML 6.0.3 is an
  explicit CI test dependency.
- X01/X02/X03 are implemented in both complete v2 policy/spec records. The
  policy test's implemented set includes all three. The v1 block is unchanged.

## Measurement and stop decision

| Measurement | Before: 9468043 | After: 0165331 | Previously exact lost |
|---|---:|---:|---:|
| Eligible seed corpus | 74/87 | 79/92 | 0 |
| Mainnet corpus | 270/279 | 270/279 | 0 |
| P03 vectors, including accepting/rejecting cases | 9/9 | 9/9 | 0 |

The old committed floors (73 and 250) were lower than the freshly measured
before counts. The stop decision used the observed 74 and 270, with individual
identity checks, rather than treating the floors as measurements. Seed vector
indices moved when upstream inserted entries, so the comparison joined source
SHA-256 plus tree SHA-256 with multiplicity. Mainnet joined tree SHA-256; its
corpus file hash was identical across revisions. All 74 old exact seed rows
and 270 old exact mainnet trees survived; five added seed rows were exact.
No engine-regression stop was triggered and the new pin was retained.

[engine-before.json](engine-before.json), [engine-after.json](engine-after.json)
and [engine-comparison.json](engine-comparison.json) retain the observations,
file identities and comparison. The old node corpus was extracted with
`git show` into a temporary directory; the read-only sibling was never reset.
The retained `engine_measurement.rs` diagnostic used the same eligibility and
recompile rules as the floor tests. `compare-engine.py` reproduces the identity
join from those observations and the node's two committed corpus revisions.
The initial diagnostic compilation failed on the changed upstream API, then
passed with only the adapter changes listed below.

The unchanged bundled corpus test passed at **264/357 obtained trees**:
367 entries, 264 byte-identical, 93 recompiling differently, 10 initial compile
failures. The separately preserved historical scoreboard remains 238/329 from
339 entries. The difference between those inventories includes S00/W05
additions; it is not attributed to the engine bump. `docs/roadmap-metrics.json`
adds an X01 scoreboard with the rev pair, before/after counts, vector results
and measurement hashes; every historical scoreboard field remains unchanged.

## Evidence integration and exact API edits

Original P03 requests, expected statuses, stages, transaction IDs, costs,
diagnostics and all their file hashes were unchanged. The retained diagnostic
`node_validation_measurement.rs` first authenticated those bytes and executed
fresh requests with only their engine premise changed. Its three tests passed,
including the full nine-vector loop and the existing controls.

The permanent test-only `engine_support/request.rs` makes this boundary
explicit: it authenticates the historical revision, proves production
validation rejects it before invoking the pipeline, clones the premises with
the current Cargo pin, and requires a different request fingerprint. The
lockfile test uses the actual historical pin as both independent compiler/node
drift controls. HTTP/CLI/DOM replay tests also create fresh test requests;
the DOM test first proves the original bundle is rejected. Shipped historical
bundles were not silently accepted or rewritten by production code.

Tests for historical mapping/property results recompute only revision-derived
identities from the original JSON using `engine_support::RevisionMap`.
They compare complete expected reports, with unchanged verdicts, costs, state,
transaction bytes and independent answers. Immutable source-artifact SHA-256
fields remain original. The old metadata identities are not copied onto new
executions. The D00 measurement example imports the shared test helper too.

The only production API adaptations forced by upstream were:

1. `NetworkRules::node` in `ergo-sandbox/src/evidence/validate.rs` takes an
   activation version and passes it to the node's now-two-argument
   `check_tree_version_supported`.
2. `validate` passes the already supplied transaction context's activation
   version to that method.
3. `ergo-sandbox/src/map/check_relation.rs` passes its already supplied
   state-domain activation version to the same method.

The only other production edit was Hunt's rule-basis revision metadata, now
computed by `node_rule_basis()` from `engine_revision()`. No decompiler, lint,
claim, replay or promotion algorithm changed.

The mapping/properties manifests and every baseline digest remained unchanged.
`baseline_support` authenticates exact historical copies of the changed Cargo
files and validator, permitting only the explicit revision/version/API edits,
and authenticates the old metrics with only the X01 scoreboard appended.
Both inventory tests passed. The M05 stop source had to gain the same explicit
remeasurement setup: its stop-record evidence locator now names an exact
archived original under `baseline/`, with its original SHA-256. The reason,
gate exits, capabilities and all other stop-record content remain unchanged.
The initial gate attempts detected that stale locator and failed; those exits
are retained. M05 and D03 remain stopped; S03 remains unimplemented.

## Validation and session limitations

All builds used this batch's isolated `$CARGO_TARGET_DIR`. The runner set the
same optimized dev profile used by CI (`opt-level=3`, debug assertions and
overflow checks enabled). No `--skip` flag was used. The actual sibling seed
and mainnet floor tests ran and printed 79/92 and 270/279; the existing ignored
optional external-corpus diagnostic was not counted as an executed gate.
The full workspace mutation-corpus measurement was allowed to finish.

Hosted GitHub cache hits, parallel CI elapsed time, three-platform release
builds, container builds and live latest-release downloads were not exercised
in this session. Workflow/action structure was tested offline and the local
CLI compatibility was observed. Tagging/publication was deliberately not
attempted. Git staging/commits were attempted and refused by the read-only
worktree index; no commit was created. Prepared messages and
[commit-plan.json](commit-plan.json) retain the four intended logical steps.

Early failures were integration diagnostics: the upstream API argument,
historical evidence revision/fingerprint checks, frozen artifact projections,
a stale M05 source locator, and shared-module imports found by clippy. They
were retained in the ledger. No interrupted command or unobserved exit was
reported as a pass.

## Required command results

Latest observed exits are shown here; report flags direct gate artifacts to
batch 10 instead of the runner's default report directory.

| Command | Exit | Seconds | Log |
|---|---:|---:|---|
| `cargo fmt --all -- --check` | 0 | 0.46 | [fmt-complete.log](logs/fmt-complete.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 0.36 | [clippy-default-complete.log](logs/clippy-default-complete.log) |
| `cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace -- -D warnings` | 0 | 0.19 | [clippy-cost-trace-complete.log](logs/clippy-cost-trace-complete.log) |
| `cargo test --workspace` | 0 | 281.62 | [workspace-final.log](logs/workspace-final.log) |
| `cargo test -p ergo-sandbox -p ergo-web --features cost-trace` | 0 | 285.85 | [cost-trace-final.log](logs/cost-trace-final.log) |
| `cargo test -p ergo-web --no-default-features` | 0 | 57.51 | [web-no-default-features.log](logs/web-no-default-features.log) |
| `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory` | 0 | 12.06 | [inventories-final.log](logs/inventories-final.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | 0.88 | [roadmap-tests-final.log](logs/roadmap-tests-final.log) |
| `python3 scripts/test_lockfile_action.py` | 0 | 0.08 | [lockfile-action.log](logs/lockfile-action.log) |
| `python3 scripts/test_release.py` | 0 | 0.07 | [release-tests-final.log](logs/release-tests-final.log) |
| `python3 scripts/test_ci_workflow.py` | 0 | 0.08 | [ci-workflow-tests-final.log](logs/ci-workflow-tests-final.log) |
| `node --test (all 10 UI files passed explicitly; exact argv below)` | 0 | 4.39 | [ui-final.log](logs/ui-final.log) |
| `python3 scripts/roadmap_gate.py --require X01 --report docs/reports/batch-10/X01.json` | 0 | 3.22 | [gate-x01-final.log](logs/gate-x01-final.log) |
| `python3 scripts/roadmap_gate.py --require X02 --report docs/reports/batch-10/X02.json` | 0 | 1.34 | [gate-x02-final.log](logs/gate-x02-final.log) |
| `python3 scripts/roadmap_gate.py --require X03 --report docs/reports/batch-10/X03.json` | 0 | 1.21 | [gate-x03-final.log](logs/gate-x03-final.log) |
| `python3 scripts/roadmap_gate.py --through-completed --report docs/reports/batch-10/through-completed.json` | 0 | 147.63 | [gate-through-completed.log](logs/gate-through-completed.log) |
| `python3 scripts/roadmap_gate.py --require P08 --report docs/reports/batch-10/P08.json` | 0 | 88.73 | [gate-p08.log](logs/gate-p08.log) |
| `python3 scripts/roadmap_gate.py --ci --report docs/reports/batch-10/ci.json` | 0 | 495.52 | [gate-ci.log](logs/gate-ci.log) |

## Complete validation and Git-write ledger

Every observed build, test, gate, integrity check and Git write attempt is
retained in [commands.json](commands.json). Failed development commands are
included. Logs contain portable locations; hashes cover the retained log bytes.

| # | Command | Exit | Seconds | Log | Log SHA-256 |
|---:|---|---:|---:|---|---|
| 1 | `git -C $NODE_CHECKOUT rev-parse origin/main HEAD` | 0 | 0.0 | [node-revision.log](logs/node-revision.log) | `093f7391dfc1f9b78396d2428717f5a97d0785a56cf66f5df7071aab99982ccb` |
| 2 | `DC_NODE_CHECKOUT=$BEFORE_CORPUS DC_REPORT=$REPO/docs/reports/batch-10/engine-before.json cargo test -p ergo-sandbox --test engine_measurement --test node_validation -- --nocapture` | 0 | 42.46 | [engine-before.log](logs/engine-before.log) | `389af98e3a58d8332c8046a20da81382574866122a4a9b8cf642cda075687186` |
| 3 | `cargo update -p ergo-compiler --precise 016533194f94ad95b1a87df70bb9bfce493922e2` | 0 | 4.76 | [cargo-update.log](logs/cargo-update.log) | `0aedc4dbb7503cd8b8d6969582a421594bc971f9cea93ddf32dee47d11f3a3a5` |
| 4 | `DC_NODE_CHECKOUT=$NODE_CHECKOUT DC_REPORT=$REPO/docs/reports/batch-10/engine-after.json cargo test -p ergo-sandbox --test engine_measurement -- --nocapture` | 101 | 13.73 | [engine-after.log](logs/engine-after.log) | `95cd0e9d50b601982abd58356ed9f439d27830ebd05e8dd8e00415f1f5d81b79` |
| 5 | `python3 scripts/test_ci_workflow.py` | 0 | 0.08 | [ci-workflow-tests.log](logs/ci-workflow-tests.log) | `d1276a2d9d79b9ed0a61446aa751886a8b573a45c52c4fe838159bc8be061837` |
| 6 | `DC_NODE_CHECKOUT=$NODE_CHECKOUT DC_REPORT=$REPO/docs/reports/batch-10/engine-after.json cargo test -p ergo-sandbox --test engine_measurement -- --nocapture` | 0 | 23.46 | [engine-after-api.log](logs/engine-after-api.log) | `1c1b703921faacd3514e8e3491621ea9c1d801752478ade3aaa000e40d0a4eaf` |
| 7 | `cargo test -p ergo-sandbox --test node_validation_measurement -- --nocapture` | 0 | 3.71 | [p03-new-revision.log](logs/p03-new-revision.log) | `e03515343cf1ecb21dbbea697f2bed444a53a2172c9268fb85d857def0e3d492` |
| 8 | `DC_REPORT=$REPO/docs/reports/batch-10/bundled-after.json cargo test -p ergo-sandbox --test decompile_corpus bundled_contracts_and_compiled_fixtures_round_trip -- --nocapture` | 0 | 1.35 | [bundled-after.log](logs/bundled-after.log) | `20c5d2a907f48e7fa28efdf2e07f6a8e2890f60c5b5a4fb68cbbd504aa510d30` |
| 9 | `cargo fmt --all` | 0 | 0.47 | [format-development.log](logs/format-development.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 10 | `git add ergo-sandbox/tests/node_validation.rs` | 128 | 0.01 | [git-stage-probe.log](logs/git-stage-probe.log) | `5ddaf5438b625b195cd8abcf295d756270a6ec2ccc47ce1eecf33b3fc93ae618` |
| 11 | `python3 scripts/test_release.py` | 0 | 0.2 | [release-tests.log](logs/release-tests.log) | `fefd166249373c22bb2d282e0ce97b29629bb483ebf37ff0769f08b9370f8b7d` |
| 12 | `cargo fmt --all` | 0 | 0.57 | [format-evidence.log](logs/format-evidence.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 13 | `python3 scripts/test_roadmap_gate.py` | 0 | 0.85 | [roadmap-tests.log](logs/roadmap-tests.log) | `6997025e47f7844419f03f86807ef5621e1228870f038371df94d45098b71171` |
| 14 | `cargo test -p ergo-sandbox --test node_validation --test lockfile --test evidence_wire --test transaction_proofs --test claim_replay --test obligation_reports --test mapping_inventory --test property_inventory --test property_evaluation --test mapping_necessity --test mapping_actions --no-fail-fast` | 101 | 37.54 | [evidence-integration.log](logs/evidence-integration.log) | `31c112bd77fb5b62e6efe6dfba18b52a2edfb6e198e26e466dc80d7708fe199b` |
| 15 | `python3 scripts/test_lockfile_action.py` | 0 | 0.08 | [lockfile-action.log](logs/lockfile-action.log) | `d094b1bd2c8318c96097f80c50da282add5b2972970a77046de8db6c8935f6ca` |
| 16 | `git log --format=%h:%cs:%s v0.3.0..v0.4.1` | 0 | 0.0 | [release-history.log](logs/release-history.log) | `f6e889157ced7b180dd4ccf944e540730f631212a6ba1951425703405e742411` |
| 17 | `git ls-remote https://github.com/actions/cache.git refs/tags/v4.3.0` | 0 | 0.62 | [cache-action-pin.log](logs/cache-action-pin.log) | `db3fab89f51ab2267d997e83d8f7f76d66b0d4411a714154a9a586d21ae2d14c` |
| 18 | `node --test ui/tests/attack.test.js ui/tests/binding-recipes.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/cost-spans.test.js ui/tests/evidence-replay.test.js ui/tests/explain.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 1 | 25.43 | [ui-tests.log](logs/ui-tests.log) | `e440f4e160c2b94d1385f3ab82f236cb5ea1ba07d4e500a7bec8177b5071f752` |
| 19 | `cargo test --workspace --no-fail-fast` | 101 | 352.22 | [workspace-diagnostic.log](logs/workspace-diagnostic.log) | `4ee08ff618e4863e6ecca93febcac4c998090fab4887c6c5446757b5ef249f8a` |
| 20 | `cargo fmt --all` | 0 | 0.57 | [format-inventories.log](logs/format-inventories.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 21 | `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory --test mapping_actions --no-fail-fast` | 101 | 14.49 | [inventories-and-actions.log](logs/inventories-and-actions.log) | `44c8236333b449ff5bdf408628a0b264f989a2bc5c5f2a8ba5e867a661efeba4` |
| 22 | `cargo fmt --all` | 0 | 0.49 | [format-replay.log](logs/format-replay.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 23 | `cargo test -p ergo-sandbox --test mapping_actions --test mapping_context_boundary --test property_replay_boundary --no-fail-fast` | 101 | 7.0 | [replay-integration.log](logs/replay-integration.log) | `859ba6cad3d1a5ae5b630589199e934c0dab864e6b38cc37bc1e9ff70322e7b9` |
| 24 | `cargo fmt --all` | 0 | 0.65 | [format-metadata.log](logs/format-metadata.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 25 | `cargo test -p ergo-sandbox --test mapping_actions` | 0 | 7.54 | [mapping-actions-final.log](logs/mapping-actions-final.log) | `103512ca533a53de9e0fbfc16678a2c522961ee03a0b89ae429bdd96855d1cd8` |
| 26 | `cargo test -p ergo-web --test checklist --test evidence_replay --no-fail-fast` | 0 | 44.85 | [web-replay-integration.log](logs/web-replay-integration.log) | `760eaefbffebabd4bea8821c04bb436573b3c8858c331cff40e580267285d92e` |
| 27 | `cargo fmt --all -- --check` | 0 | 0.59 | [fmt-final.log](logs/fmt-final.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 28 | `cargo clippy --workspace --all-targets -- -D warnings` | 101 | 15.55 | [clippy-default.log](logs/clippy-default.log) | `597a2ee0e9b5a9a54baca0bbcdd46c8b50d9b856bd162848e482b26cbc0012f0` |
| 29 | `cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace -- -D warnings` | 101 | 0.92 | [clippy-cost-trace.log](logs/clippy-cost-trace.log) | `9fc94f2973dc726eec626536ac2088ee0c39f14ab1b684ebab92b13584b5b92c` |
| 30 | `cargo fmt --all` | 0 | 1.39 | [format-modules.log](logs/format-modules.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 31 | `cargo clippy --workspace --all-targets -- -D warnings` | 101 | 7.57 | [clippy-default-final.log](logs/clippy-default-final.log) | `76f08ecfce022952c94e34b6118108a6beecc9d3b49f48357fabcabc338393ca` |
| 32 | `cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace -- -D warnings` | 101 | 0.26 | [clippy-cost-trace-final.log](logs/clippy-cost-trace-final.log) | `bee4b399aaefa56276ee68473f9ed04cb282c67ab15633f123bd3a70ea94837d` |
| 33 | `python3 scripts/roadmap_gate.py --require X01 --report docs/reports/batch-10/X01.json` | 1 | 0.31 | [gate-x01.log](logs/gate-x01.log) | `4b658855f96f090c34a19859942356c08d2b943c2e443b61ab7210f000b7adb6` |
| 34 | `python3 scripts/roadmap_gate.py --require X02 --report docs/reports/batch-10/X02.json` | 1 | 0.22 | [gate-x02.log](logs/gate-x02.log) | `8d4ded19f0aba4a860651bed500c7cd940a491d99ead8bfcd3b0a8d77ce6b494` |
| 35 | `python3 scripts/roadmap_gate.py --require X03 --report docs/reports/batch-10/X03.json` | 1 | 0.22 | [gate-x03.log](logs/gate-x03.log) | `4315dbd304e823b0718e832bb2c091e8092ff31203a1adec09357f1c1249f28d` |
| 36 | `cargo fmt --all` | 0 | 0.57 | [format-example.log](logs/format-example.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 37 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 0.36 | [clippy-default-complete.log](logs/clippy-default-complete.log) | `e2e756fc4c6f6e2546aec61daaaea759a209f22e8f11483b00f4a397a5eef485` |
| 38 | `cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace -- -D warnings` | 0 | 0.19 | [clippy-cost-trace-complete.log](logs/clippy-cost-trace-complete.log) | `dd756d73eac8ebae32dd594b314d24b4fbe7ac4630b91ce602a9c1aad6fe445b` |
| 39 | `python3 scripts/roadmap_gate.py --require X01 --report docs/reports/batch-10/X01.json` | 0 | 3.22 | [gate-x01-final.log](logs/gate-x01-final.log) | `2ccfd4fbb6000da3b55e51bffb73d80019ba493c61ce8db461ccb333c8f2588d` |
| 40 | `python3 scripts/roadmap_gate.py --require X02 --report docs/reports/batch-10/X02.json` | 0 | 1.34 | [gate-x02-final.log](logs/gate-x02-final.log) | `c727bdcea17293f6214ee4ac9b990a041b99882c9d96e51e2f2484f5fefdcd81` |
| 41 | `python3 scripts/roadmap_gate.py --require X03 --report docs/reports/batch-10/X03.json` | 0 | 1.21 | [gate-x03-final.log](logs/gate-x03-final.log) | `af9f76cd213086c25ae6c3c1f18ab9756d78308f27ab78977ef6fd62b62053b8` |
| 42 | `python3 docs/reports/batch-10/compare-engine.py` | 0 | 0.05 | [engine-comparison.log](logs/engine-comparison.log) | `faa373aaa661d304228e3a513c32e7a6542016e8a17ca846790835b7f6c2ba14` |
| 43 | `git -C $NODE_CHECKOUT remote get-url origin` | 0 | 0.01 | [node-origin.log](logs/node-origin.log) | `da3d78e30791de5d0cba26e63c6e59967bf6294d8a5fc76a64d57c4aee06d40d` |
| 44 | `cargo test --workspace` | 0 | 281.62 | [workspace-final.log](logs/workspace-final.log) | `0e0bad75e831a414845116116d5e107db37f1830964f4fbe1bbe12865ae09033` |
| 45 | `cargo test -p ergo-sandbox --test decompile_roundtrip -- --nocapture` | 0 | 19.75 | [corpus-floors.log](logs/corpus-floors.log) | `f46bb045760ea0b0b7441c512827e6f569eab2efa529d42db4bcd2835bbe5c67` |
| 46 | `node --test ui/tests/attack.test.js ui/tests/binding-recipes.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/cost-spans.test.js ui/tests/evidence-replay.test.js ui/tests/explain.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 0 | 4.39 | [ui-final.log](logs/ui-final.log) | `2aebcc2a042a2cf13b6d38ef4a22893c0435fe31a677d09d34c287a85d72dc27` |
| 47 | `python3 scripts/roadmap_gate.py --through-completed --report docs/reports/batch-10/through-completed.json` | 0 | 147.63 | [gate-through-completed.log](logs/gate-through-completed.log) | `1708672183d010ba2247f38104dfb37883e9222d0825e9eb2457682ab40a43a5` |
| 48 | `python3 docs/reports/batch-10/check-integrity.py` | 0 | 0.4 | [integrity-first.log](logs/integrity-first.log) | `e39caa92a63c45058f081969a389aeed7d25a9701df9c4bd58df5564fdd3b988` |
| 49 | `cargo run --locked -p ergo-sandbox --bin ergo-es -- --help` | 0 | 0.16 | [cli-compatibility.log](logs/cli-compatibility.log) | `ee3a0b56d30f767853e6fe1ef6299882bfc7fd89b666e5073dde04ce57b00c52` |
| 50 | `python3 scripts/test_roadmap_gate.py` | 0 | 0.88 | [roadmap-tests-final.log](logs/roadmap-tests-final.log) | `31ee4ec40ab320df351bc51921037e22a35e311ef44bb65fe71cfdcd3a34312e` |
| 51 | `python3 scripts/test_release.py` | 0 | 0.07 | [release-tests-final.log](logs/release-tests-final.log) | `455a234a903f02c7d36882c9b036acc1ee7cca1176404082cfe9897ae40211f7` |
| 52 | `python3 scripts/test_ci_workflow.py` | 0 | 0.08 | [ci-workflow-tests-final.log](logs/ci-workflow-tests-final.log) | `53b77ed73b881186e0f76e3a51bce55b2d9b1423f4fc18759eb12e7a5b519b65` |
| 53 | `python3 scripts/roadmap_gate.py --require P08 --report docs/reports/batch-10/P08.json` | 0 | 88.73 | [gate-p08.log](logs/gate-p08.log) | `b482c349da4bd3c30a81a118c537e6fe64ec0f9471afe2a32577419e4e448844` |
| 54 | `git add Cargo.toml Cargo.lock ergo-sandbox ergo-web/tests ui/tests/evidence-replay.test.js docs/roadmap-metrics.json docs/roadmap-stops.json docs/reports/batch-10/baseline` | 128 | 0.01 | [git-add-step1.log](logs/git-add-step1.log) | `2ba5e1c383f642ab66ff4b8166307a09b4f9324516a7bd8c7c25509f76b84c59` |
| 55 | `git commit -F docs/reports/batch-10/commit-1-message.txt` | 128 | 0.01 | [git-commit-step1.log](logs/git-commit-step1.log) | `2ba5e1c383f642ab66ff4b8166307a09b4f9324516a7bd8c7c25509f76b84c59` |
| 56 | `git add Cargo.toml Cargo.lock README.md CHANGELOG.md .github/actions/test/action.yml scripts/test_release.py scripts/requirements-test.txt` | 128 | 0.01 | [git-add-step2.log](logs/git-add-step2.log) | `2ba5e1c383f642ab66ff4b8166307a09b4f9324516a7bd8c7c25509f76b84c59` |
| 57 | `git commit -F docs/reports/batch-10/commit-2-message.txt` | 128 | 0.01 | [git-commit-step2.log](logs/git-commit-step2.log) | `2ba5e1c383f642ab66ff4b8166307a09b4f9324516a7bd8c7c25509f76b84c59` |
| 58 | `git add .github/workflows/ci.yml scripts/test_ci_workflow.py` | 128 | 0.01 | [git-add-step3.log](logs/git-add-step3.log) | `2ba5e1c383f642ab66ff4b8166307a09b4f9324516a7bd8c7c25509f76b84c59` |
| 59 | `git commit -F docs/reports/batch-10/commit-3-message.txt` | 128 | 0.01 | [git-commit-step3.log](logs/git-commit-step3.log) | `2ba5e1c383f642ab66ff4b8166307a09b4f9324516a7bd8c7c25509f76b84c59` |
| 60 | `git add docs/ROADMAP.md docs/superpowers/specs/2026-09-13-forge-roadmap-v2.md scripts/test_roadmap_gate.py docs/reports/batch-10` | 128 | 0.01 | [git-add-step4.log](logs/git-add-step4.log) | `2ba5e1c383f642ab66ff4b8166307a09b4f9324516a7bd8c7c25509f76b84c59` |
| 61 | `git add -f docs/reports/batch-10/logs` | 128 | 0.0 | [git-add-logs.log](logs/git-add-logs.log) | `2ba5e1c383f642ab66ff4b8166307a09b4f9324516a7bd8c7c25509f76b84c59` |
| 62 | `git commit -F docs/reports/batch-10/commit-4-message.txt` | 128 | 0.01 | [git-commit-step4.log](logs/git-commit-step4.log) | `2ba5e1c383f642ab66ff4b8166307a09b4f9324516a7bd8c7c25509f76b84c59` |
| 63 | `cargo test -p ergo-sandbox -p ergo-web --features cost-trace` | 0 | 285.85 | [cost-trace-final.log](logs/cost-trace-final.log) | `265722bdee41bc0783029bd4be1dcbb65c42365f49f0f17187372d9058832921` |
| 64 | `cargo test -p ergo-web --no-default-features` | 0 | 57.51 | [web-no-default-features.log](logs/web-no-default-features.log) | `d36b40ad17d199210609cc007ab598c3d01e47e60688856c64f09b8923b2eee4` |
| 65 | `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory` | 0 | 12.06 | [inventories-final.log](logs/inventories-final.log) | `fcdb802b729eb590759491230a4e8d303b964b8012f3ecc1a27bd0e7a7a8b06b` |
| 66 | `python3 docs/reports/batch-10/check-integrity.py` | 0 | 0.16 | [integrity-report.log](logs/integrity-report.log) | `e39caa92a63c45058f081969a389aeed7d25a9701df9c4bd58df5564fdd3b988` |
| 67 | `git diff --check` | 2 | 0.06 | [whitespace-first.log](logs/whitespace-first.log) | `91ba9018dd091aea7b7e67b753f5d9467fbfc32d6083cbd452be32219773b4ff` |
| 68 | `git diff --check` | 0 | 0.06 | [whitespace-final.log](logs/whitespace-final.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 69 | `cargo fmt --all -- --check` | 0 | 0.46 | [fmt-complete.log](logs/fmt-complete.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 70 | `python3 scripts/roadmap_gate.py --ci --report docs/reports/batch-10/ci.json` | 0 | 495.52 | [gate-ci.log](logs/gate-ci.log) | `50835d474a52ca95ddb43ad40870b7b27bcc2dce88d4b8dcccd93ead9348c114` |
| 71 | `python3 docs/reports/batch-10/check-integrity.py` | 0 | 0.16 | [integrity-complete.log](logs/integrity-complete.log) | `e39caa92a63c45058f081969a389aeed7d25a9701df9c4bd58df5564fdd3b988` |
| 72 | `git diff --check` | 0 | 0.06 | [whitespace-complete.log](logs/whitespace-complete.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Artifact identities

| Artifact | SHA-256 |
|---|---|
| [X01.json](X01.json) | `974aaae190e69912a71a6de14067852481d5129bfea621066e46434f3eac0a5c` |
| [X02.json](X02.json) | `2e6ce6079004ee306b8fed302a1a9fc5e9996414ac317b13f091af5d7352345d` |
| [X03.json](X03.json) | `2df652b288d1ad62f5b6d6755a09df32d9219b05fe0a139fb7f49dee7c5bde98` |
| [through-completed.json](through-completed.json) | `2bf873fadcbd0cdd7cddf92be41ea3e1287ea68d726c5fdd0eb194447f0e371c` |
| [P08.json](P08.json) | `a9bfb5aa37318410a872f68edfaa31530d4dbcce9f9ab193903c2c986cbd823f` |
| [ci.json](ci.json) | `1bda39a24468439d8916c60df46fc02c295ef374a290f5bdc134307b41d04788` |
| [engine-before.json](engine-before.json) | `603aaa89ea3d5cc3857a7ea6308d4e2d14485006e1dfa96d4a2e16a90fbd0922` |
| [engine-after.json](engine-after.json) | `e9f2f18eb000a35796ae4b90a5434b6dd8f6e100d4906cd17705cedeffd9944d` |
| [engine-comparison.json](engine-comparison.json) | `faa373aaa661d304228e3a513c32e7a6542016e8a17ca846790835b7f6c2ba14` |
| [bundled-after.json](bundled-after.json) | `861fd9c5185fbfbd541bd140e5f7c38e3a2488eb145330e3e0503f5d3101060d` |

## Post-session

No post-session results were claimed. All recorded exits came from this
implementation session. The maintainer still needed to create the prepared
commits outside the read-only index restriction, merge, and then decide when
to cut v0.5.0. Nothing was pushed or tagged by this session.

## Post-session re-verification

After the implementation session ended, the same tree was re-verified outside
its sandbox with this batch's build directory (`$CARGO_TARGET_DIR`), before the
commits were created from the four prepared messages. Observed exits:

| Command | Exit |
|---|---:|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo clippy -p ergo-sandbox -p ergo-web --all-targets --features cost-trace -- -D warnings` | 0 |
| `python3 scripts/test_roadmap_gate.py`, `test_lockfile_action.py`, `test_release.py`, `test_ci_workflow.py` | 0 |
| `node --test ui/tests/*.test.js` (47 passed) | 0 |
| `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory` | 0 |
| `cargo test --workspace` (no skips; the seed-corpus floor passes on the new pin) | 0 |
| `cargo test -p ergo-sandbox -p ergo-web --features cost-trace -- --skip the_corpus_is_measured_and_does_not_regress` | 0 |
| `cargo test -p ergo-web --no-default-features` | 0 |
| `python3 scripts/roadmap_gate.py --require X01 / X02 / X03` | 0 |

The filtered output is committed as
[`logs/workspace-post-session.log`](logs/workspace-post-session.log) and
recorded as the `postSession` entries of `commands.json`. The committed gate
reports written by that run hash as:

| Report | SHA-256 |
|---|---|
| `X01.json` | `5022b7d6eb108ae550d1dc5af6c61fc6b31541c1143812813e7cb689db7b979a` |
| `X02.json` | `fd8cebec33cf3083effe19fc3426e524c5b01bfdc99a877c1d19171ac142207d` |
| `X03.json` | `59f2a4b39ffaa011739ba81d5dc2e34d3f44604aae7504b74c888971568db772` |
