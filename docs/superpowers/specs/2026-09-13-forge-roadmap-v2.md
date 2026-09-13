# ergo-forge roadmap v2 — the playground first, with the evidence boundary underneath it

Status: **proposed governing plan.** Inspected at `a57df08` (main, clean) on 2026-09-13. Once merged, this document replaces `docs/ROADMAP.md` section 2 (the freeze ledger) and section 3 (the P00–P08 queue) as the executable queue. Sections 1, 4 and 6 of the old roadmap survive as stated below. Nothing here reruns a benchmark or reopens a recorded stop.

## 0. Why a new plan

The old roadmap did its job. P00–P08 landed: every legacy result is labelled static, synthetic or preflight; canonical boxes and transactions use node wire types; the pinned node's full validator produces non-forgeable acceptance; funding proofs are real; a property violation replays offline from a bundle in the CLI and in the browser. M00–M04 landed and M05 stopped on its own checker. The discovery arc landed D00–D02 in PR #100 (open, green, mergeable at the time of writing) and D03 stopped there on its own pre-declared utility gate: the property machinery works on its 24-row corpus, and nothing shows it helps an author yet.

What the old plan also did was freeze every surface a user touches. Build, Compose, Play, the decompiler, the CI runner and the UI are all FREEZE; a sixth lint and any recipe growth are forbidden. The last user-facing commit is the evidence replay panel on 2026-09-10. Since 2026-09-09 nothing has made the forge a better place to write, read or break a contract.

The mission has not changed: **the best ErgoScript playground**, with security and immutability as the emphasis after the USE/DexyGold LP drain. This plan puts the playground back as the primary axis and uses the evidence boundary as the thing that makes its security claims trustworthy, rather than as a separate product.

### What survives unchanged from the old roadmap

| Principle | Status |
|---|---|
| One pinned compiler/reducer/validator from the node; no hand-ported consensus rules | **Survives.** Every unit below runs on the pinned engine. |
| A miss is never "safe"; caps, truncation, objective and provenance are recorded | **Survives.** Applies to every new probe, lint and attacker-mode result. |
| Static severity is review priority, not vulnerability authority | **Survives.** The checklist (S01) renders it that way. |
| Only a fresh accepted execution plus a declared property can say `confirmed-violation` | **Survives.** Attacker mode in Play reports acceptance, never "exploit". |
| One implementation branch, one open PR, finish in order, record stops | **Survives.** The batch table in section 4 is the order. |
| Confidential third-party cases stay private | **Survives.** |
| Mutation/control pairs decide whether a detector helped | **Survives.** Every new lint ships with mutant pairs in the answer key. |
| M05 stopped; D03 stopped | **Survives.** D00–D02 are maintained like the P-series (E00). E01 below states the precondition for D04 and for capabilities 2 and 3. |

### What changes

| Old disposition | New disposition |
|---|---|
| Play FREEZE | **BUILD.** Play becomes the attacker's workbench (W01) and the source of test cases (W03). |
| Five lints, no sixth | **BUILD, bounded.** Four named lints (S02), each gated by mutant pairs and a precision sweep over the deployed corpus. |
| Compose/Build FREEZE, no recipe growth | **BUILD, bounded.** Two recipes that teach the binding discipline (W05). No wallet step, no protocol DSL. |
| Web/UI REWORK, no editor work | **BUILD.** Checklist, verify-deployment, cost-per-span, Play attacker controls, share for Play and suites. No redesign. |
| Spend hunt DEMOTE, no new probes | **BUILD, bounded.** Immobilisation probes only (S04). |
| Drain hunt: no new search axes | **One axis** (S03, multi-instance), last in the queue, gated by a mutant pair. |
| Autonomous sweeps DELETE | **Stays deleted.** |

## 1. Tracks

Five tracks. Units carry a track letter, a number, a ceiling in working days, and a gate. A gate is a named set of tests the runner discovers and runs; the runner rejects zero-test and ignored runs as it does today.

- **W — Workbench.** The playground: Play, Write, Read, Build, sharing, files.
- **S — Security.** Detectors, checklist, probes, incident tooling.
- **I — Immutability.** Deployment verification, lockfiles, on-chain change monitoring.
- **E — Evidence and discovery.** The P-series boundary as a maintained capability; discovery gated by a real-user precondition.
- **X — Platform.** Engine pinning, releases, CI, repository hygiene.

### Track W — Workbench

**W00 — Adopt this plan and clean the root (1 day).**
Replace `docs/ROADMAP.md` sections 2–3 with a pointer to this file and the new policy block. Move `GATE-REPORT.md`, `MAP-REPORT.md`, `MATCH-REPORT.md` under `docs/reports/`. Leave `target-p00/` and `logs/` ignored and untouched (65 GB local build cache and explorer logs; not repository content). Gate: `scripts/test_roadmap_gate.py` passes on the new block; `roadmap_gate.py --scoreboard-only` still reads every old artifact hash.

**W01 — Attacker mode in Play (5 days).**
Play already runs every input's script in the transaction's context over `POST /api/v1/play`. Add adversary operations on a drafted transaction, each producing a new draft and a diff of which scripts accepted:
- reorder inputs and data inputs (permute, or drag one box to a slot);
- insert a decoy box at a chosen input slot, auto-populated with junk tokens and registers sufficient to satisfy every `tokens(i)` and `R{n}` access the target script makes (derived from the lifted tree, the same walk `unbound_box_reserves` does);
- swap a data input for a look-alike (same registers, no NFT);
- shift token indices by prepending a junk token;
- tamper one register or one output field and rerun.
Every result is labelled `synthetic` and `nodeValidated: false`, exactly as Play is today. Ship a guided walkthrough, "the USE drain in four moves", built from `examples/incidents/` boxes, that ends with the fixed swap refusing the same draft.
Gate: `play_attacker_reorder_reproduces_use_drain` (deployed pair accepts, fixed swap refuses, through the HTTP route), `decoy_box_satisfies_every_positional_access`, `attacker_results_are_synthetic_and_unvalidated`, and a DOM test for the walkthrough.

**W02 — Play to test suite (2 days).**
"Save as test" turns a Play transaction into a `contract.test.json` case for a chosen input's contract, with the current verdict as the expectation. "Save as scenario" writes a scenario file. Both must run unchanged under `ergo-es test` / `ergo-es eval`.
Gate: `play_export_roundtrips_through_cli_test_runner` (verdict identical), `exported_case_names_only_the_suite_contract`.

**W03 — Share Play chains and suites (1 day).**
The `#s=` share link today carries a contract. Extend it to a Play chain (boxes, drafts, height) and to a test suite, size-capped, with the same "nothing loaded from a CDN, nothing fetched" rule.
Gate: `share_link_roundtrips_play_state`, `share_link_over_cap_is_refused_with_size`.

**W04 — Cost and explain in Write (3 days).**
`ergo-es eval --hot-spots` (cost-trace build) exists only on the CLI. Surface it in Write as cost per source span, and add "explain this expression": reduce a selected sub-expression in the current scenario and show its value or the residual sigma proposition. Both use the existing engine budget.
Gate: `hot_spots_map_to_source_spans`, `explain_subexpression_matches_full_reduction`, `cost_trace_route_is_behind_feature_flag`.

**W05 — Two recipes that teach binding (2 days).**
"Pool-bound swap" (an order that names the pool NFT and checks it on the input it reads) and "successor-locked vault" (script, NFT, value floor and named registers all carried). Each ships with independent expectations, a Build walkthrough, and a mutant in `examples/mutants/` that the suite catches. No other recipe growth.
Gate: `new_recipes_have_independent_expectations`, `new_recipe_mutants_are_caught`.

**W06 — Read: what the contract does not check (2 days).**
Read renders the contract in plain words. Add the negative space under it, derived from S00/S01: which boxes it reads by position, which outputs it leaves unconstrained, which registers it trusts, which data inputs it never identifies. Every line is a static observation with an anchor.
Gate: `negative_space_lines_have_anchors`, `negative_space_is_labelled_static`.

### Track S — Security

**S00 — Attack-vector catalogue as data (2 days).**
`docs/security/vectors.json` and a rendered `docs/security/VECTORS.md`. One row per vector: id, class, description, the mechanism, which forge instrument answers it (lint id, hunt probe, drain family, scenario, node-validated claim, or `manual`), and at least one positive and one negative example contract in `examples/`. Initial rows, twelve classes:
1. positional binding without identity (inputs, outputs, data inputs);
2. unbound successor and reserves (script bound, value/tokens/registers not);
3. unconstrained outputs (only `OUTPUTS(0)` checked; fee and change as extraction);
4. oracle and data-input trust (NFT, freshness, epoch);
5. attacker-controlled context (`getVar`, extension, `executeFromVar`, deserialised code);
6. token index games;
7. arithmetic (truncation direction, one-sided invariants, precision, overflow throws);
8. immobilisation (unguarded `get`, cost limit, min box value, size, storage rent);
9. sigma-proposition mistakes (trivial branch, `atLeast` k=0, key from writable register);
10. height and ordering (bounds, miner choice, front-running);
11. multi-instance composition;
12. deployment drift and upgrade hooks.
Gate: `every_vector_has_examples_or_is_marked_manual`, `every_named_instrument_exists`.

**S01 — Checklist surface (3 days).**
`POST /api/v1/checklist` and `ergo-es checklist`: for one contract (and optional scenario or evidence bundle), every S00 vector with its answer and its answer's provenance: `static`, `scenario`, `preflight`, `node-validated`, or `unchecked`. Rendered in Write and Read as a list, never as a score. `unchecked` is the default and is visible.
Gate: `checklist_defaults_to_unchecked`, `checklist_answer_carries_provenance`, `checklist_never_emits_a_score`, DOM test.

**S02 — Four lints (4 days).**
Each with mutant/control pairs in `examples/mutants/answer-key.json` and a precision sweep over the 79 deployed contracts with every reported site reviewed and recorded in `docs/audit-sweep.md`:
- `unconstrained_outputs`: the tree constrains specific output indices and nothing bounds `OUTPUTS.size`, total value, or the remaining outputs;
- `successor_field_drift`: an output is bound to `SELF.propositionBytes` (or NFT) but a value floor, a token amount, or a register the script reads on SELF is not carried;
- `trivial_sigma_branch`: a disjunct of the result reduces to a constant or to a proposition over attacker-writable data (a register on a non-SELF box, a `getVar`);
- `unauthenticated_code_execution`: `executeFromVar`, `deserialize`, or `substConstants` where the bytes are not compared to a literal or to a SELF-rooted value (the M05 class).
`unbound_box_reserves` already covers vector 1 for reserves; extend its scope note, do not fork it.
Gate: `new_lint_mutants_are_caught_and_controls_are_clean`, `deployed_corpus_sweep_is_recorded`, and the existing lint precision fixture with positive and negative controls per lint.

**S03 — Multi-instance axis in the drain hunt (3 days, last in queue).**
One new candidate family: two instances of the protected box in one transaction. Same caps, same objective, same promotion path (P06). Gate: `two_instance_mutant_is_found_and_control_is_not`, `caps_and_truncation_are_recorded`. If the mutant is not found within the family's cap, record a stop; do not widen.

**S04 — Immobilisation probes and storage rent in Read (2 days).**
Hunt gets probes that answer "can this box be spent at all": every register read absent, minimum box value on every output, and a cost-limit probe. Read shows the storage-rent line ("after four years, anyone may claim this box for the rent") using the P03 storage-rent case as its basis.
Gate: `immobilisation_probes_report_unspendable_not_safe`, `storage_rent_line_is_static`.

**S05 — Incident scaffold (2 days).**
`ergo-es incident <txid>` (explorer required) writes the three-suite skeleton for a mainnet transaction: one suite per script under test, boxes verbatim, expectations left for the author. Regenerating USE must reproduce the committed box sets byte-for-byte.
Gate: `incident_scaffold_reproduces_use_boxes`, `incident_scaffold_never_fills_expectations`.

### Track I — Immutability

**I01 — Verify deployment in Write (2 days).**
A button and `ergo-es verify <address> --source f.es --params p.json`: exact bytes match, template match with differing constants (listed), or no match. Uses `identity.rs` and offline address decoding; no behavioural equivalence.
Gate: `verify_exact_template_and_mismatch_are_distinct`, `verify_lists_differing_constants`.

**I02 — Lockfile (3 days).**
`ergo-es lock` writes `contract.lock.json`: source hash, parameter values, compiler and node revision, ErgoTree bytes, address, tree version, lint sweep digest. `ergo-es verify-lock` recompiles and compares; with an explorer, also compares to the live box under an NFT. Project zips include it; the CI action can require it.
Gate: `lockfile_detects_source_param_and_engine_drift`, `lockfile_matches_live_box_bytes` (fixture-backed).

**I03 — Watch (3 days).**
`ergo-es watch <lockfile>` and a Read panel: given protocol NFTs, fetch current boxes and report whether the script under each NFT still matches the locked bytes, and whether any upgrade hook register changed. Fixture-driven tests; explorer optional at runtime.
Gate: `watch_reports_script_change_under_nft`, `watch_never_broadcasts`.

**I04 — Upgrade-hook audit (1 day).**
A checklist row and a `trust_assumptions` extension: script hash stored in a register with a spend path that may rewrite it, and who may do so. Gate: mutant pair.

### Track E — Evidence and discovery

**E00 — Maintain the boundary (ongoing, no unit).** P01–P08 tests stay in the gate runner's completed prefix. Any unit above that touches `evidence/` must leave `claim.rs`, `replay.rs`, `promotion.rs` semantics unchanged.

**E01 — Discovery precondition.** D04, lifecycle construction (capability 2) and numeric solving (capability 3) may be scheduled only when an author (you, or a named external team) has two live protocols and has written D01 property declarations for both by hand first, giving D03 the transfer denominator it stopped on. Until then D00–D02 are a maintained capability with no further units. This is the spec's own utility argument, adopted as policy.

### Track X — Platform

**X01 — Engine bump (3 days).** Move the pinned node rev forward to the current arkadianet/ergo main; rerun the decompile corpus and the P03 vectors; record the scoreboard. A regression in exact round-trips is a stop, not a reason to patch the decompiler.
Gate: existing corpus expectations, `node_vectors_pass_on_new_rev`.

**X02 — Release 0.4.0 (1 day).** Binaries, container, CI action `version: latest` pointing at it, changelog from the batch table.

**X03 — CI time (1 day).** Cache the node sibling checkout and split cost-trace into a separate job.

## 2. Sequencing — twelve batches, one PR each

Order is by user value first, with each security unit landing before the surface that displays it. Ceilings sum to 52 working days; that is the scope of this plan, not a schedule promise.

| Batch | Units | Days | What a user gets |
|---|---|---|---|
| 0 | W00, S00 | 3 | The plan adopted, a clean root, the vector catalogue with examples |
| 1 | W01 | 5 | Attacker mode in Play and the USE walkthrough |
| 2 | S02 | 4 | Four new lints with mutants and a recorded sweep |
| 3 | S01, W06 | 5 | The checklist in Write and Read, and "what it does not check" |
| 4 | I01, I02 | 5 | Verify deployment, lockfiles in projects and CI |
| 5 | W02, W03 | 3 | Play exports tests; share links for chains and suites |
| 6 | W04 | 3 | Cost per span and explain-expression in Write |
| 7 | S04, I04 | 3 | Immobilisation probes, storage rent, upgrade-hook row |
| 8 | I03 | 3 | Watch for on-chain script changes |
| 9 | W05, S05 | 4 | Two binding recipes; incident scaffold |
| 10 | X01, X02, X03 | 5 | Engine bump, 0.4.0 release, faster CI |
| 11 | S03 | 3 | Multi-instance drain axis, then a checkpoint |
| 12 | Checkpoint | 1 | Scoreboard review; decide E01 and the next plan |

Batch 10 can move earlier if the engine bump is needed for a v6 feature a batch depends on.

## 3. Stop rules

Same mechanism as the old roadmap section 6: reaching a ceiling, failing a gate twice, or finding the capability refuted records a stop in `docs/roadmap-stops.json` with reason code, commit, gate output and retained capability. New reason codes for this plan:

- `precision-regression`: a new lint reports a site on the deployed corpus that review classes as a false positive and the lint cannot be narrowed within ceiling. Ship the lint as `observation` severity only, or drop it.
- `synthetic-drift`: an attacker-mode result or a Play export disagrees with `ergo-es test` on the same input. Stop the unit; this is a correctness bug in the shared engine path, not a UI issue.
- `engine-regression`: X01 loses exact round-trips or P03 vectors. Stay on the old rev.
- `explorer-dependency`: a unit needs live chain data its fixtures cannot supply. Ship the fixture path, mark the live path `unverified`.

## 4. Scoreboard

Record-only, extending `docs/roadmap-metrics.json`. No unit is complete because a number moved; the gate is.

| Metric | Now | Source |
|---|---|---|
| Vectors with a forge instrument / catalogued | n/a | S00 |
| Lints with mutant pairs and recorded sweep | 5/5 | S02 |
| Deployed-corpus false positives per lint (reviewed) | recorded in `audit-sweep.md` | S02 |
| Incidents in the replay corpus | 1 (USE) | S05 |
| Recipes with a caught mutant | 0/16 | W05 |
| Exact decompile round-trips | 238/329 committed | X01 |
| Node-validated claims (P05 families) | as recorded | E00 |
| Play exports that round-trip through the CLI | n/a | W02 |

## 5. Policy block for the gate runner

To be installed by W00 as a **separate `roadmap-policy:v2` block** in `docs/ROADMAP.md`. The existing `roadmap-policy:v1` block stays byte-for-byte frozen: `ergo-sandbox/tests/mapping_inventory.rs::legacy_metrics_are_measured_without_baseline_changes` hashes it against a pinned `protectedPolicySha256`, so mutating it (even appending a unit) is a tamper failure by design. The v1 block remains the record of the P- and M-series; the v2 block is the live queue. `scripts/roadmap_gate.py` gains a v2 parser and unions the two unit sets for selection and scoreboard, with v2's `depends` allowed to reference v1 unit ids. Ceilings and test names match the units above; the runner discovers them by name and rejects a unit whose tests are absent. Each v2 unit carries the runner's required fields (`schemaVersion`, `implemented`, `days`, `package`, `target`, `tests`) and the v2 block carries its own `completedThrough` (initially `null`), `baselineRev` (full 40-hex sha of the base commit), and the `thresholds`/`frozenPreflight`/`scoreboard`/`stopRecords` keys the parser requires, pointing at the same artifacts as v1.

```json
{
  "schemaVersion": 1,
  "baselineRev": "a57df08",
  "completedThrough": "M04",
  "maxActiveImplementationBranches": 1,
  "maxOpenImplementationPrs": 1,
  "newUnits": [
    {"id": "W00", "depends": [], "days": 1, "package": "scripts", "target": "test_roadmap_gate", "tests": ["policy_v2_block_parses", "scoreboard_reads_every_baseline_artifact"]},
    {"id": "S00", "depends": ["W00"], "days": 2, "package": "ergo-sandbox", "target": "vector_catalogue", "tests": ["every_vector_has_examples_or_is_marked_manual", "every_named_instrument_exists"]},
    {"id": "W01", "depends": ["W00"], "days": 5, "package": "ergo-web", "target": "play_attacker", "tests": ["play_attacker_reorder_reproduces_use_drain", "decoy_box_satisfies_every_positional_access", "attacker_results_are_synthetic_and_unvalidated"]},
    {"id": "S02", "depends": ["S00"], "days": 4, "package": "ergo-sandbox", "target": "mutation_corpus", "tests": ["new_lint_mutants_are_caught_and_controls_are_clean", "deployed_corpus_sweep_is_recorded"]},
    {"id": "S01", "depends": ["S00", "S02"], "days": 3, "package": "ergo-web", "target": "checklist", "tests": ["checklist_defaults_to_unchecked", "checklist_answer_carries_provenance", "checklist_never_emits_a_score"]},
    {"id": "W06", "depends": ["S01"], "days": 2, "package": "ergo-web", "target": "negative_space", "tests": ["negative_space_lines_have_anchors", "negative_space_is_labelled_static"]},
    {"id": "I01", "depends": ["W00"], "days": 2, "package": "ergo-sandbox", "target": "verify_deployment", "tests": ["verify_exact_template_and_mismatch_are_distinct", "verify_lists_differing_constants"]},
    {"id": "I02", "depends": ["I01"], "days": 3, "package": "ergo-sandbox", "target": "lockfile", "tests": ["lockfile_detects_source_param_and_engine_drift", "lockfile_matches_live_box_bytes"]},
    {"id": "W02", "depends": ["W01"], "days": 2, "package": "ergo-web", "target": "play_export", "tests": ["play_export_roundtrips_through_cli_test_runner", "exported_case_names_only_the_suite_contract"]},
    {"id": "W03", "depends": ["W01"], "days": 1, "package": "ergo-web", "target": "share_links", "tests": ["share_link_roundtrips_play_state", "share_link_over_cap_is_refused_with_size"]},
    {"id": "W04", "depends": ["W00"], "days": 3, "package": "ergo-web", "target": "write_cost_explain", "tests": ["hot_spots_map_to_source_spans", "explain_subexpression_matches_full_reduction", "cost_trace_route_is_behind_feature_flag"]},
    {"id": "S04", "depends": ["S00"], "days": 2, "package": "ergo-sandbox", "target": "immobilisation", "tests": ["immobilisation_probes_report_unspendable_not_safe", "storage_rent_line_is_static"]},
    {"id": "I04", "depends": ["S02"], "days": 1, "package": "ergo-sandbox", "target": "mutation_corpus", "tests": ["upgrade_hook_mutant_is_caught"]},
    {"id": "I03", "depends": ["I02"], "days": 3, "package": "ergo-sandbox", "target": "watch", "tests": ["watch_reports_script_change_under_nft", "watch_never_broadcasts"]},
    {"id": "W05", "depends": ["S02"], "days": 2, "package": "ergo-sandbox", "target": "compose_recipes", "tests": ["new_recipes_have_independent_expectations", "new_recipe_mutants_are_caught"]},
    {"id": "S05", "depends": ["W00"], "days": 2, "package": "ergo-sandbox", "target": "incident_scaffold", "tests": ["incident_scaffold_reproduces_use_boxes", "incident_scaffold_never_fills_expectations"]},
    {"id": "X01", "depends": ["W00"], "days": 3, "package": "ergo-sandbox", "target": "node_validation", "tests": ["node_vectors_pass_on_new_rev"]},
    {"id": "S03", "depends": ["S02", "X01"], "days": 3, "package": "ergo-sandbox", "target": "drain_promotion", "tests": ["two_instance_mutant_is_found_and_control_is_not", "caps_and_truncation_are_recorded"]}
  ]
}
```

## 6. Out of scope for this plan

- A wallet, signing with user keys in the browser, or broadcasting from the forge.
- Behavioural-equivalence matching of contracts.
- Automatic role inference, global safety discharge, or a protocol-specification DSL.
- Any hosted sweep, disclosure, or alerting service beyond `ergo-es watch` run by the user.
- D04 and discovery capabilities 2 and 3 until E01's precondition is met.
- Editor redesign. The four-mode layout stays.

## 7. First action

PR #100 should merge before batch 0 so that the roadmap edit rebases cleanly on the consolidated discovery entry. Batch 0 is a docs-and-data PR: adopt this plan (W00) and write the vector catalogue with its example contracts (S00). Batch 1 is the first feature: attacker mode in Play with the USE walkthrough.
