# Batch 9 — W05 and S05

Implementation on `v2/batch-9`, based on `a69af13`, 2026-09-14. Git staging
and commits were refused by the sandbox's read-only worktree index (exit 128).
The work remained uncommitted. No push, branch switch, other-worktree edit,
explorer request or broadcast occurred. All builds used this batch's isolated
`$CARGO_TARGET_DIR`.

All required validation commands finished with exit 0. The final workspace run
completed in 873.65 seconds, including the full mutation-corpus measurement and
the requested seed-corpus skip. Earlier failures are retained in the ledger.

## Implemented

- W05: `pool-bound-swap.es` and `successor-locked-vault.es`, using the existing
  EIP-5 recipe format discovered in the checkout. Both compile at the default
  tree version 3, appear in Build, and carry an authored walkthrough in their
  doc header. Build's production edit was two title entries; index.html was
  unchanged. The DOM test drives the existing picker/question flow.
- Independent source+params suites beside the recipes: 5 swap scenarios and
  8 vault scenarios. Honest cases pass; decoy pool/NFT, falling reserve product,
  successor script/value/register changes and extra outputs fail as specified.
  Verdicts were authored independently, never populated from evaluator output.
- Two versioned source mutants under `examples/mutants/w05/v1/`, with their
  recipes as controls. `examples/mutants/recipes.json` registers them and a
  separate in-memory output-count mutation. The three defects individually
  flip their binding case from required fail to actual pass; the suite catches
  each. Their respective static observations are unbound-box-reserves,
  successor-field-drift and unconstrained-outputs. Controls are clean under
  every current lint in both lift modes. The separate manifest avoided changing
  either historical mutant JSON file or parallel staticLintPairs appends.
- S05: `ergo_sandbox::incident::scaffold` and `ergo-es incident <txid>
  --explorer <url> [--out <dir>] [--network mainnet|testnet]`. A new directory
  receives README.md, empty triage.json and one `input-N/contract.test.json`
  per distinct spent script, with one case per matching SELF position.
- TxBoxes now carries optional transaction inclusion height and ordered data
  inputs. The explorer transaction adapter hydrates incomplete references via
  read-only box lookups and preserves hex case. Fixtures and CLI share the
  scaffold; offline adapter tests also exercise hydration and exact raw hex.
- Both units were marked implemented in v2 policy and spec; the implemented-set
  assertion adds only W05/S05. W05's DOM test is required by extras(), including
  failure/empty/ignored-run checks in the gate runner unit test.

## Exact box reproduction and evidence boundary

The test constructs and JSON-round-trips a Fixture archive from the committed
USE boxes, then passes it through the production scaffold. It matches the swap
and pool suites by script, preserving selfIndex 1/2 and inclusion height 1868204.
All projected input/output JSON bytes match after common JSON serialization:
fields, array/token order, integer precision and exact hex characters.

The committed suites lack every boxId and every input creationHeight. Their
outputs carry creationHeight 1868202, which is retained and compared. The test
supplies conspicuous synthetic values only for the missing metadata and removes
precisely those additions for equality. It never changes the original suites.
Data inputs and mixed-case hex are covered separately. There was no recorded
live transaction fixture added: the committed USE boxes supply the Fixture.

Every generated expectation is REPLACE-ME, rejected by the real CLI test parser.
Missing inclusion height is another flagged placeholder; the chain tip and
box creation heights never replace it. Triage selection, objective and bounds
remain empty for the author. The scaffold makes no exploitability claim.
Recipe results are synthetic with nodeValidated: false; static findings mean
review priority. See [incident-scaffold.md](../../incident-scaffold.md).

## Integration and retained limitations

The existing every-recipe recognition test exposed unsupported guarded branches,
reserve-product comparisons, non-SELF script comparisons and empty collection
literals. Small additions to Read's literal description renderer cover those
shapes while retaining its incomplete/quoted fallback. The Build combinator DSL,
compiler, evaluator, lints and existing recipe tests were unchanged.

The old decompiler corpus inventory is frozen. A separate
`w05_decompile_expectations.json` adds four measured records, with duplicate
baseline keys rejected. The two recipe trees lift completely and recompile,
but their decompiled source did not round-trip to identical tree bytes on the
pinned engine. Those outcomes were retained as recompiles-but-differs, not
reclassified as exact. The two embedded context-tree records are byte-identical.
[decompile-w05.json](decompile-w05.json) records the new measurements.
The immutable S02 sweep delegates only these two new recipes to W05's all-lint
checks; every historical source, finding and record remains pinned.

The live explorer path was unverified in this session, as required by the
explorer-dependency boundary. Fixtures supplied the required box projection,
so no capability stop was recorded and no unit was widened to obtain live data.
The teaching recipes constrain exactly two/one outputs; they did not implement
fee/change assembly, wallet signing or node-validated transaction acceptance.

Initial development runs exposed aliased synthetic input/output objects in
fixture authoring, an unfinished import, an overbroad creationHeight projection,
missing corpus registration and unsupported descriptions. An intermediate
vault simplification also let compilation eliminate the SELF R4 read from its
mutant; the explicit accounting-unit condition was restored. These failures
and the intentional measurements before registering the new decompiler rows
are retained below. Required unit gate runs passed; no interrupted command was
reported as a pass. The first workspace attempt exited at the corpus inventory
check; the final run follows the separate registration.

## Required command results

The latest observed exit is shown below. The known local seed-corpus mismatch
was skipped exactly as requested; existing ignored tests were left unchanged.

| Command | Exit | Log |
|---|---:|---|
| `cargo fmt --all -- --check` | 0 | [fmt-check-final.log](logs/fmt-check-final.log) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | [clippy-final.log](logs/clippy-final.log) |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 | [workspace-final.log](logs/workspace-final.log) |
| `python3 scripts/test_roadmap_gate.py` | 0 | [roadmap-tests.log](logs/roadmap-tests.log) |
| `python3 scripts/test_lockfile_action.py` | 0 | [lockfile-action.log](logs/lockfile-action.log) |
| `node --test (all 10 UI test files, passed explicitly)` | 0 | [dom-all.log](logs/dom-all.log) |
| `python3 scripts/roadmap_gate.py --require W05 --report docs/reports/batch-9/W05.json` | 0 | [gate-w05-final.log](logs/gate-w05-final.log) |
| `python3 scripts/roadmap_gate.py --require S05 --report docs/reports/batch-9/S05.json` | 0 | [gate-s05-final.log](logs/gate-s05-final.log) |
| `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory` | 0 | [inventories.log](logs/inventories.log) |

## Complete command ledger

Every validation and commit attempt is recorded in [commands.json](commands.json)
with exit status, elapsed time and its log. Earlier failures remain visible.
The adapter-only filter run executed one library test and zero integration
tests; it was not counted as integration or gate evidence.

| # | Command | Exit | Seconds | Log | Log SHA-256 |
|---:|---|---:|---:|---|---|
| 1 | `cargo test -p ergo-sandbox --test compose_recipes` | 101 | 16.84 | [recipes-first.log](logs/recipes-first.log) | `4b8c50ff20982c75925af82b5f1229d24c4295bc45ab70897e561a35026dc966` |
| 2 | `cargo test -p ergo-sandbox --test compose_recipes` | 101 | 0.82 | [recipes-corrected.log](logs/recipes-corrected.log) | `b2a0d371e3a9800005e2e24f4fdeb9efc562503bb1492d377033ca6f8e6c2c1b` |
| 3 | `cargo test -p ergo-sandbox --test compose_recipes --test incident_scaffold` | 101 | 4.99 | [focused.log](logs/focused.log) | `d0b8480d50788fd4597b9670fda14e2c2b15e082ae531341014a13249d2c20d6` |
| 4 | `cargo test -p ergo-sandbox --test compose_recipes --test incident_scaffold --lib incident_transaction_adapter` | 0 | 4.96 | [focused-corrected.log](logs/focused-corrected.log) | `88742231fadd98b0e53c963d2b58c6af1af3a2275d11115ca2c271b89873918c` |
| 5 | `cargo fmt --all` | 0 | 0.43 | [fmt-implementation.log](logs/fmt-implementation.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 6 | `cargo test -p ergo-sandbox --test compose_recipes --test incident_scaffold` | 0 | 2.88 | [recipe-incident-tests.log](logs/recipe-incident-tests.log) | `77a0b85af604234e65e3ecc99a4368ba0d652dfe289b4a90f7e548363dfbd764` |
| 7 | `node --test ui/tests/binding-recipes.test.js` | 0 | 0.26 | [binding-dom.log](logs/binding-dom.log) | `33858d113ab2bdcb581560bb077b883389032b351a085287df39b0e69d8163a0` |
| 8 | `cargo fmt --all` | 0 | 0.41 | [fmt-finalize.log](logs/fmt-finalize.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 9 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 12.98 | [clippy.log](logs/clippy.log) | `f52ff47a235af3e1622a48d116161964c33c9a3c8563099c72c32141c5ec4dea` |
| 10 | `cargo test -p ergo-sandbox --test compose_recipes` | 0 | 2.41 | [recipe-final.log](logs/recipe-final.log) | `04807396ff71d05222819c6a9ec28f8e8bd1d7c8e91d45eca54d10622e67c414` |
| 11 | `git add examples/contracts/recipes/pool-bound-swap.es examples/contracts/recipes/pool-bound-swap.test.json examples/contracts/recipes/successor-locked-vault.es examples/contracts/recipes/successor-locked-vault.test.json examples/mutants/recipes.json examples/mutants/w05/v1 ergo-sandbox/tests/compose_recipes.rs ergo-sandbox/tests/mutation_corpus.rs` | 128 | 0.0 | [git-add-step1.log](logs/git-add-step1.log) | `2551ea43ccf3532ffa340043e8ae43fefd9b839639345abd6918ab77656b62ab` |
| 12 | `git commit -F docs/reports/batch-9/commit-1-message.txt` | 128 | 0.0 | [git-commit-step1.log](logs/git-commit-step1.log) | `2551ea43ccf3532ffa340043e8ae43fefd9b839639345abd6918ab77656b62ab` |
| 13 | `python3 scripts/test_roadmap_gate.py` | 0 | 0.71 | [roadmap-tests.log](logs/roadmap-tests.log) | `e4caed7dbf6265b8e0075962d66a20c457da807ada36819c346736092a3a0e89` |
| 14 | `python3 scripts/test_lockfile_action.py` | 0 | 0.05 | [lockfile-action.log](logs/lockfile-action.log) | `d094b1bd2c8318c96097f80c50da282add5b2972970a77046de8db6c8935f6ca` |
| 15 | `node --test ui/tests/attack.test.js ui/tests/binding-recipes.test.js ui/tests/checklist.test.js ui/tests/claim-labels.test.js ui/tests/cost-spans.test.js ui/tests/evidence-replay.test.js ui/tests/explain.test.js ui/tests/play-export.test.js ui/tests/share.test.js ui/tests/verify.test.js` | 0 | 30.27 | [dom-all.log](logs/dom-all.log) | `50fab24e588db71070f42642d6391d0cc69959970d466bbb314d65df2942b7ba` |
| 16 | `cargo fmt --all -- --check` | 0 | 1.07 | [fmt-check.log](logs/fmt-check.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 17 | `cargo test -p ergo-sandbox --test incident_scaffold` | 0 | 15.55 | [incident-final.log](logs/incident-final.log) | `4f0d2e54c0321a2d795bbf1731cd8dd5cc2294327fa0cf069898318391b8bc4f` |
| 18 | `git add ergo-sandbox/src/incident.rs ergo-sandbox/src/bin/incident/mod.rs ergo-sandbox/src/bin/ergo-es.rs ergo-sandbox/src/lib.rs ergo-sandbox/src/map/source.rs ergo-sandbox/src/map/explorer.rs ergo-sandbox/tests/incident_scaffold.rs docs/incident-scaffold.md` | 128 | 0.0 | [git-add-step2.log](logs/git-add-step2.log) | `2551ea43ccf3532ffa340043e8ae43fefd9b839639345abd6918ab77656b62ab` |
| 19 | `git commit -F docs/reports/batch-9/commit-2-message.txt` | 128 | 0.0 | [git-commit-step2.log](logs/git-commit-step2.log) | `2551ea43ccf3532ffa340043e8ae43fefd9b839639345abd6918ab77656b62ab` |
| 20 | `git add ui/app.js ui/tests/binding-recipes.test.js` | 128 | 0.0 | [git-add-step3.log](logs/git-add-step3.log) | `2551ea43ccf3532ffa340043e8ae43fefd9b839639345abd6918ab77656b62ab` |
| 21 | `git commit -F docs/reports/batch-9/commit-3-message.txt` | 128 | 0.0 | [git-commit-step3.log](logs/git-commit-step3.log) | `2551ea43ccf3532ffa340043e8ae43fefd9b839639345abd6918ab77656b62ab` |
| 22 | `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 101 | 35.6 | [workspace.log](logs/workspace.log) | `5e637d48426d85f557a7d500f9170ddadb24be71a8563497bee2974c504f5ea5` |
| 23 | `cargo test -p ergo-sandbox --test recognize every_recipe_is_put_into_words_completely` | 101 | 0.45 | [recipe-recognition.log](logs/recipe-recognition.log) | `8e7c08ef4ba4f486632cd934bb06a24b8ce143e7d3683bebb724dce247078c78` |
| 24 | `cargo test -p ergo-sandbox --test recognize --test compose_recipes` | 101 | 2.18 | [recognition-and-recipes.log](logs/recognition-and-recipes.log) | `55c42ac3c3f84446eaee7a253f97abcf428eee87bb1bbf5da2d0061d9350faa4` |
| 25 | `DC_REPORT="$PWD/docs/reports/batch-9/decompile-measurement.json" cargo test -p ergo-sandbox --test decompile_corpus bundled_contracts_and_compiled_fixtures_round_trip` | 101 | 1.6 | [decompile-measurement.log](logs/decompile-measurement.log) | `936e24bf08f69de0e1332ed88418861d50d10a13ecaecbd4f2716c3efce829ea` |
| 26 | `cargo test -p ergo-sandbox --test recognize` | 101 | 0.11 | [recognition-final.log](logs/recognition-final.log) | `77144e34673ef26e55ce27efd20309c69727611e410dbb56bcd7f5439d624b25` |
| 27 | `python3 scripts/roadmap_gate.py --require W05 --report docs/reports/batch-9/W05.json` | 0 | 179.87 | [gate-w05.log](logs/gate-w05.log) | `66a3dd61cd9c0bca4fca7c6e27f4a21056c7247fde72ef28a9e714848f17ea9e` |
| 28 | `python3 scripts/roadmap_gate.py --require S05 --report docs/reports/batch-9/S05.json` | 0 | 2.36 | [gate-s05.log](logs/gate-s05.log) | `2bee30247915006690c245d5d89af8db344e4f913fe5bc0f44302514076cba93` |
| 29 | `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory` | 0 | 1.23 | [inventories.log](logs/inventories.log) | `ff256642dd6dc8d6328710d9c7086dc4e74eec4e2fc2652d273ac7afd356bd09` |
| 30 | `cargo test -p ergo-sandbox --test recognize --test compose_recipes` | 0 | 2.08 | [recognition-recipes-complete.log](logs/recognition-recipes-complete.log) | `75f95fc90c2805f3f3761c8d43405dc4121a1931f783f7e75cee2a93c2e9ad59` |
| 31 | `cargo fmt --all` | 0 | 0.4 | [fmt-integration.log](logs/fmt-integration.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 32 | `DC_REPORT="$PWD/docs/reports/batch-9/decompile-measurement.json" cargo test -p ergo-sandbox --test decompile_corpus bundled_contracts_and_compiled_fixtures_round_trip` | 101 | 1.52 | [decompile-final-measurement.log](logs/decompile-final-measurement.log) | `7658e8fca8bb28c6b9026f61a2d4758775940c9ffe8f7dba533d06c0e5df727c` |
| 33 | `cargo test -p ergo-sandbox --test decompile_corpus bundled_contracts_and_compiled_fixtures_round_trip` | 0 | 1.49 | [decompile-inventory-final.log](logs/decompile-inventory-final.log) | `5f757def9bbbae6b863c380da064a93c971d0d0c1faeb21cc5bb2d6ab9a7c965` |
| 34 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 11.19 | [clippy-final.log](logs/clippy-final.log) | `0c5bee21dd709deb0ee5e5187e27ea3c941dd39fc78f6136f8ffdcb46fec3210` |
| 35 | `cargo fmt --all -- --check` | 0 | 0.42 | [fmt-check-final.log](logs/fmt-check-final.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 36 | `python3 docs/reports/batch-9/check-integrity.py` | 0 | 0.67 | [integrity.log](logs/integrity.log) | `8625a5242b5ffe29509ff2a392cb18a654103f5e08eabad3e228617eed72f191` |
| 37 | `git diff --check` | 0 | 0.05 | [whitespace.log](logs/whitespace.log) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 38 | `git add docs/ROADMAP.md docs/superpowers/specs/2026-09-13-forge-roadmap-v2.md scripts/roadmap_gate.py scripts/test_roadmap_gate.py docs/reports/batch-9` | 128 | 0.0 | [git-add-step4.log](logs/git-add-step4.log) | `2551ea43ccf3532ffa340043e8ae43fefd9b839639345abd6918ab77656b62ab` |
| 39 | `git add -f docs/reports/batch-9/logs` | 128 | 0.0 | [git-add-logs.log](logs/git-add-logs.log) | `2551ea43ccf3532ffa340043e8ae43fefd9b839639345abd6918ab77656b62ab` |
| 40 | `git commit -F docs/reports/batch-9/commit-4-message.txt` | 128 | 0.0 | [git-commit-step4.log](logs/git-commit-step4.log) | `2551ea43ccf3532ffa340043e8ae43fefd9b839639345abd6918ab77656b62ab` |
| 41 | `python3 scripts/roadmap_gate.py --require W05 --report docs/reports/batch-9/W05.json` | 0 | 180.4 | [gate-w05-final.log](logs/gate-w05-final.log) | `1497eb22b6ef0468faabee7b1fddae8b29038918e2e1b409a8e956c2a7168d14` |
| 42 | `python3 scripts/roadmap_gate.py --require S05 --report docs/reports/batch-9/S05.json` | 0 | 2.4 | [gate-s05-final.log](logs/gate-s05-final.log) | `03492121febfe7ba836502eba3397ac5cdc3fe641f5c8916c4272b611ac390d1` |
| 43 | `python3 docs/reports/batch-9/check-integrity.py` | 0 | 0.71 | [integrity-final.log](logs/integrity-final.log) | `8625a5242b5ffe29509ff2a392cb18a654103f5e08eabad3e228617eed72f191` |
| 44 | `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 | 873.65 | [workspace-final.log](logs/workspace-final.log) | `1b77e5efeb97d109f1076ef7a74b6ab3e2c288ff19b3c7d35022fa7b5f66d31e` |
| 45 | `python3 docs/reports/batch-9/check-integrity.py` | 0 | 0.68 | [integrity-completed.log](logs/integrity-completed.log) | `8625a5242b5ffe29509ff2a392cb18a654103f5e08eabad3e228617eed72f191` |

## Integrity and prepared commits

[check-integrity.py](check-integrity.py) checks all forbidden source files,
all historical fixtures/recipes/suites/mutants, batch 1–8 reports, v1 policy,
the active branch and portable report paths against a69af13. Both inventory
guards additionally verify their unchanged baseline and policy hashes.

[commit-plan.json](commit-plan.json) assigns the final files to four logical
steps. [commit-1-message.txt](commit-1-message.txt),
[commit-2-message.txt](commit-2-message.txt),
[commit-3-message.txt](commit-3-message.txt) and
[commit-4-message.txt](commit-4-message.txt) retain the requested co-author and
session trailers. No commit was created because Git could not write index.lock.

| Artifact | SHA-256 |
|---|---|
| [W05.json](W05.json) | `10d88691deb528e544c28cd70da77fc5221914129b60b6adabddd2e444bfe6d5` |
| [S05.json](S05.json) | `6b716df294d4a73ed2594a80078d54f49d28adb884b53356ed1dc78d81099b83` |
| [decompile-w05.json](decompile-w05.json) | `1c93693e81553b2c03dbb3c7923193136da2513fc179a3f65d0fec55089df1de` |

## Post-session

No post-session checks or externally supplied passes were claimed by the
implementation session itself. All observed results above came from that
session. The live explorer path remained unverified for the reasons recorded
above; the commits were created afterwards from the prepared messages.

Post-session ledger entry: `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` exited **0** ([log](logs/workspace-post-session.log)). Post-session re-verification outside the implementation sandbox; the same log records the W05 and S05 gate runs (exit 0) that wrote the committed W05.json and S05.json.

## Post-session re-verification

After the implementation session ended, the same tree was re-verified outside
its sandbox with this batch's build directory (`$CARGO_TARGET_DIR`), before the
commits were created from the four prepared messages. Observed exits:

| Command | Exit |
|---|---:|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `python3 scripts/test_roadmap_gate.py` | 0 |
| `python3 scripts/test_lockfile_action.py` | 0 |
| `node --test ui/tests/*.test.js` (41 passed) | 0 |
| `cargo test -p ergo-sandbox --test mapping_inventory --test property_inventory` | 0 |
| `cargo test --workspace -- --skip seed_corpus_holds_the_exact_floor_when_checkout_present` | 0 |
| `python3 scripts/roadmap_gate.py --require W05 --report docs/reports/batch-9/W05.json` | 0 |
| `python3 scripts/roadmap_gate.py --require S05 --report docs/reports/batch-9/S05.json` | 0 |

The filtered output is committed as
[`logs/workspace-post-session.log`](logs/workspace-post-session.log) and
recorded as the `postSession` entry of `commands.json`. After the branch was
stacked on batches 7 and 8, clippy, the workspace suite and both gates were run
again on that tree and exited 0; the committed gate reports are from that run
and hash as:

| Report | SHA-256 |
|---|---|
| `W05.json` | `10d88691deb528e544c28cd70da77fc5221914129b60b6adabddd2e444bfe6d5` |
| `S05.json` | `6b716df294d4a73ed2594a80078d54f49d28adb884b53356ed1dc78d81099b83` |
