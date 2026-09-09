# ergo-forge — governing roadmap

**This document replaces `docs/superpowers/specs/2026-09-08-drain-hunt-roadmap.md` as the governing plan.** That document and the per-phase specifications are historical design and measurement records. They do not authorize additional work. Where they conflict with this plan, this plan wins. The architecture rationale is in [ARCHITECTURE-REVIEW-CODEX.md, sections 1–7](ARCHITECTURE-REVIEW-CODEX.md); this document defines the executable queue, not another review.

Baseline: main `ee4ac6a874531872c27828d94e21e4fe7a1d7f7c`, inspected 2026-09-09. No benchmarks were rerun to write this plan. The committed artifacts, not remembered headlines or uncommitted reports, supply the scoreboard below.

**The landing situation has changed:** local main already contains #95, #94, #93, #92, #90, and #91. Six sibling branches are patch-equivalent to changes on main; `git cherry main HEAD` reports `-` for their unique commits. The GitHub API and web lookup were unavailable, so the current remote PR state is unverified. This plan does not pretend #95 is still pending: its implementation is present locally, and the next change is an amendment on top of it.

| Earlier conclusion | Governing decision now |
|---|---|
| One pinned compiler/reducer; no alternative acceptance engine | **Survives.** Extend the dependency boundary to the pinned node's full transaction validator. |
| A miss is never “safe”; record caps, truncation, objective, and provenance | **Survives.** Enforce through the result schema and gates below. |
| Mutation/control pairs decide whether work helped; private treatment of third-party findings | **Survives.** Do not publish confidential worktree contents as acceptance fixtures. |
| A/C before deeper search, B is important | **Narrowed.** Preserve provenance and demote claims now. General automatic composition is not in this delivery queue. Existing conditional discharge remains useful. |
| `txcheck::check` is always the authoritative oracle | **Superseded.** It is unsigned preflight. Only the full pinned validator can produce node-accepted execution evidence. |
| Blind generation is a permanent invariant | **Superseded as a universal rule.** Keep the current blind family frozen and measurable; a future target-guided proposer would need its own declared method and gate. None is authorized here. |
| Expand 3a–3c, then steering, solver, chains, autonomy | **Superseded as a build queue.** No new search axes in this cycle. Existing gains remain. 3d/4/5 are unplanned; phase 6 autonomous discovery is cancelled. |
| Twenty mutants/all operators before accepting broader search coverage | **Retained for any future coverage expansion.** It does not block honest preflight releases or the evidence boundary. Five proven mutants do not satisfy it. |
| One-transaction composition subsumes chains; chains depend on effective steering | **Rejected.** These are distinct representation and scheduling questions. Neither becomes a task by implication. |
| “Complete lift” or successful constant substitution supports a complete audit | **Rejected.** Recovery coverage, exact identity, property coverage, and execution validity are separate measurements. |

## 1. What ergo-forge is

**ergo-forge is an ErgoScript workbench for building contracts and inspecting deployed code through reproducible experiments on the Rust Ergo node's pinned compiler and execution engine.** Its authoring and audit views share contracts, scenarios, and evidence; every result states whether it is a static observation, a synthetic experiment, or a transaction checked against an explicitly supplied state. It helps people check specific contract claims and investigate counterexamples. It is not a general security certification service, an autonomous vulnerability scanner, a wallet, or a protocol-design generator. New work must improve the fidelity, reproducibility, or interpretation of those experiments; more recipes, detectors, graph features, and search breadth do not qualify by themselves.

P03 now provides node-accepted execution against supplied premises through the library API. Node-validated property claims still have zero producers until P05; transaction acceptance alone is not a property claim.

## 2. Freeze / finish / rework / demote ledger

**One developer, one implementation branch, one PR awaiting review.** Finish the numbered queue in order; open no speculative worktrees. Corrections needed to keep existing capabilities working are allowed. A new capability requires replacing a queued unit, updating its machine gate and stop rule, and explicitly recording the displaced work. Finishing early is not permission to start an appendix item.

Each row has exactly one disposition. FINISH means a bounded remaining landing/closure task; FREEZE means retain the working capability without expansion; REWORK means keep the capability but replace the specified boundary; DELETE/DEMOTE means remove authority or obsolete development work, not indiscriminately delete user data.

| Subsystem | Disposition | Allowed work / what stops |
|---|---|---|
| Decompiler | **FREEZE** | Preserve per-entry round-trip expectations and visible partial recovery. No general IR migration or pursuit of 100% printable source this cycle. Site: [decompile/mod.rs:136](../ergo-sandbox/src/decompile/mod.rs#L136). |
| Five audit lints | **DELETE/DEMOTE** | Retain detectors as observations; remove vulnerability authority from their default severity and from unrelated drain confirmation. P00/P07 only; no sixth lint. Sites: [audit/mod.rs:24](../ergo-sandbox/src/audit/mod.rs#L24), [finding.rs:38](../ergo-sandbox/src/audit/finding.rs#L38). |
| Single-box spend hunt | **DELETE/DEMOTE** | Retain six-probe preview; remove unconditional “stealable”/consensus authority. No register expansion or replacement search. Sites: [hunt.rs:168](../ergo-sandbox/src/hunt.rs#L168), [hunt.rs:228](../ergo-sandbox/src/hunt.rs#L228). |
| Drain hunt | **REWORK** | Keep the exact candidate family, caps, and objective. Separate candidate preflight from promotion through canonical execution evidence; P06. No token-replacement, membership, register, or oracle-search expansion. Sites: [drain.rs:282](../ergo-sandbox/src/drain.rs#L282), [drain.rs:1070](../ergo-sandbox/src/drain.rs#L1070). |
| Protocol map and conditional composition | **FREEZE** | Keep discovery and the existing explicitly conditional discharges. No automatic role inference expansion, global safety discharge, or new graph. Sites: [map/mod.rs:936](../ergo-sandbox/src/map/mod.rs#L936), [audit/context.rs:35](../ergo-sandbox/src/audit/context.rs#L35). |
| Mutation corpus | **FINISH** | Reconcile the existing machine baseline with stale prose; register it with the gate runner. Preserve all eight cases and separate preflight from new node evidence. No target of improving 4/5 by adding an axis. Sites: [answer-key.json:1](../examples/mutants/answer-key.json#L1), [mutation_corpus.rs:697](../ergo-sandbox/tests/mutation_corpus.rs#L697). |
| Compose / Build | **FREEZE** | Maintain the current recipes, combinator DSL, and independent expectations. No recipe catalogue growth, wallet step, or general protocol-specification DSL. Site: [compose.rs:747](../ergo-sandbox/src/compose.rs#L747). |
| Play | **FREEZE** | Keep it explicitly synthetic, including its current simulation IDs. P00 corrects claims. Canonical state transitions and chain search are deferred; do not silently migrate existing browser chains. Sites: [play.rs:130](../ergo-sandbox/src/play.rs#L130), [play.rs:168](../ergo-sandbox/src/play.rs#L168). |
| Static ingestion | **REWORK** | Preserve binding provenance with analysis artifacts; P01. Do not chase the remaining compiler failures with source substitutions. Sites: [ingest/mod.rs:74](../ergo-sandbox/src/ingest/mod.rs#L74), [ingest/mod.rs:87](../ergo-sandbox/src/ingest/mod.rs#L87). |
| Web / UI | **REWORK** | P00 truthful labels, P08 one evidence-replay panel using the existing engine budget. No dashboard, sweep UI, editor redesign, new jobs service, or graph polish. Sites: [ui/app.js:279](../ui/app.js#L279), [app.rs:93](../ergo-web/src/app.rs#L93). |
| `txcheck`, box marshalling, and execution adapters | **REWORK** | Preserve legacy preflight/simulation; introduce a separate canonical path through the node validator in P01–P05. Never accumulate a second list of hand-ported consensus rules. Sites: [txcheck.rs:90](../ergo-sandbox/src/txcheck.rs#L90), [box_build.rs:50](../ergo-sandbox/src/box_build.rs#L50). |
| Structural matcher / offline address ingestion | **FREEZE** | Already landed. Preserve exact address bytes and explicit structural-match limitations. No behavioral-equivalence engine. Sites: [identity.rs:18](../ergo-sandbox/src/identity.rs#L18), [tree.rs:1](../ergo-sandbox/src/tree.rs#L1). |
| CI action and test-suite runner | **FREEZE** | Keep existing scenario verdict assertions. Add roadmap gates to repository CI, not new hosted workflows or distribution products. Sites: [.github/workflows/ci.yml:1](../.github/workflows/ci.yml#L1), [.github/actions/test/action.yml:1](../.github/actions/test/action.yml#L1). |
| Autonomous sweeps / disclosure automation | **DELETE/DEMOTE** | Remove from the committed build queue. Preserve private handling of manual audit cases; no automated discovery/disclosure service. |

### Worktree closure and landing order

**Do not merge any of the six already-integrated branches again.** Their squash commits on main are the landing record. The actual order was alias → severity → address → map → match → gate. That resolved the shared lint/finding/DTO/CLI/corpus surfaces already. Replaying a stale branch can restore old files or duplicate fixtures; it does not finish work.

| Order | Worktree / item | Disposition | Landing instruction |
|---|---|---|---|
| 0 | Main worktree | **FINISH** | Use `ee4ac6a` as this plan's baseline. Preserve the uncommitted architecture review and this roadmap; neither is a reason to reset the workspace. Start P00 from current main. |
| 1 | `../ergo-forge-alias` | **DELETE/DEMOTE** | **Abandon the duplicate landing branch.** `d0e66a5` is patch-equivalent to `9439db1` (#91). Its only untracked artifact is `ALIAS-REPORT.md`; retain it as historical evidence before retiring the worktree. No rebase/merge. |
| 2 | `../ergo-forge-sev` | **DELETE/DEMOTE** | **Abandon the duplicate landing branch.** `47cc5ff` is represented by `854efcc` (#90). Preserve `SEV-REPORT.md`; do not reclassify further lint cases as cleanup. |
| 3 | `../ergo-forge-addr` | **DELETE/DEMOTE** | **Abandon the duplicate landing branch.** `effacd4` is represented by `f7f9a70` (#92). Preserve `ADDR-REPORT.md`. The capability stays frozen on main. |
| 4 | `../ergo-forge-map` | **DELETE/DEMOTE** | **Abandon the duplicate landing branch.** `95b3cae` is represented by `a158ad7` (#93). No pending working changes; its report is already tracked on main. |
| 5 | `../ergo-forge-match` | **DELETE/DEMOTE** | **Abandon the duplicate landing branch.** `d2a723a` is represented by `d71cc6a` (#94). No pending working changes; its report is already tracked on main. |
| 6 | `../ergo-forge-gate` | **DELETE/DEMOTE** | **Abandon the stale landing branch.** `b7380ab` is represented by `ee4ac6a` (#95). Put the amendment in a fresh P00 branch from main; do not rebase and replay the old implementation. |
| 7 | PR #95 capability | **REWORK** | **Land amended, not as-is; do not wait for the new validator.** Since it is already on local main, land a follow-up P00 immediately. Preserve replay/request/budget evidence; rename scenario reproduction and remove `confirmed` authority. True claim promotion waits for P05/P06. |
| 8 | `../ergo-forge-dexy` | **DELETE/DEMOTE** | **Abandon as an active workstream.** HEAD `07ce532` is an ancestor of main and the worktree is clean, with no unique commits. No live hunting campaign is queued. Retain already-committed public incident fixtures. |
| 9 | `../ergo-forge-lithos` | **DELETE/DEMOTE** | **Abandon as a public landing branch; retain the confidential case privately.** HEAD is an ancestor of main, but `.gitignore`, the local source clone, review artifacts, and a review example are uncommitted. Do not merge, publish, clean, or force-remove these. Generic future fixtures must be authored independently or explicitly cleared for publication. |

“Abandon” here closes a development queue entry. Physical removal is a separate housekeeping action after preserving untracked/private artifacts. This roadmap does not authorize deleting them. If remote #95 proves still open despite the local squash record, amend that PR with P00 before landing instead of creating a duplicate follow-up. The substantive decision is unchanged.

The closure check is read-only: `git worktree list`, `git -C <path> status --short`, and `git -C <path> cherry main HEAD`. A `+` or a newly modified file invalidates this inventory; inspect that delta before retirement. It does not authorize another feature branch.

## 3. Sequenced plan — nine reviewable PRs, then stop

The queue is **P00 → P01 → P02 → P03 → P04 → P05 → P06 → P07 → P08**. P01–P05 deliberately split the execution-evidence boundary into five PRs. These are scope ceilings for one developer with AI assistance: 2, 3, 4, 6, 3, 4, 4, 4, and 3 working days respectively, totaling 33 working days plus integration/review buffer within twelve weeks. Reaching a ceiling triggers the stop rules; it does not authorize silently widening the PR. There is no tenth feature waiting behind this list.

### P00 — Amend #95 and make all current claims truthful

**Change:** preserve today's capabilities and evidence while making every legacy result explicitly static, synthetic, or unsigned preflight, and install the roadmap gate runner.

**Touch:** [audit/triage.rs:114](../ergo-sandbox/src/audit/triage.rs#L114), [tests/triage.rs:12](../ergo-sandbox/tests/triage.rs#L12), [txcheck.rs:75](../ergo-sandbox/src/txcheck.rs#L75), hunt/Play result envelopes, [dto.rs:17](../ergo-web/src/dto.rs#L17), [bin/ergo-es.rs:83](../ergo-sandbox/src/bin/ergo-es.rs#L83), the checklist in section 7, and new `scripts/roadmap_gate.py`, `scripts/test_roadmap_gate.py`, `ergo-sandbox/tests/claim_contract.rs`, `ui/tests/claim-labels.test.js`.

**Buys:** no current positive gains authority just because a reducer ran twice. **Dependencies:** none; do this before refactoring.

**Acceptance P00:** `scenario_reproduction_is_not_confirmation` replaces the existing test that expects `confirmed`; legacy triage writes `formatVersion: 2` and `state: reproduced-in-scenario`. `legacy_results_are_explicitly_preflight_or_simulation` requires method/provenance fields and `nodeValidated: false` on all existing result envelopes. Keep `TxCheck.valid` as a deprecated alias for preflight success; never change it to mean node acceptance. `legacy_record_cannot_import_as_verified` rejects or explicitly downgrades old stored `confirmed` records. UI tests render positive and negative synthetic hunts and assert the warning is visible in both; they assert “Preflight passed” instead of “Would validate.” Gate-runner self-tests must reject an unknown unit, missing evidence, a skipped test, and a zero-test filter. No detector, generator, corpus verdict, or objective change belongs here.

**Shippable state:** the current instrument, honestly labelled; node-validated claim count remains zero. Future schemas are not required to land this correction.

### P01 — Bind provenance to analysis and execution-case inputs

**Change:** introduce a versioned evidence-case schema that preserves target bytes, constant origins, source identity, state/context provenance, and missing premises through every analysis result.

**Touch:** new `ergo-sandbox/src/evidence/{mod,case}.rs`, [lib.rs:44](../ergo-sandbox/src/lib.rs#L44), [ingest/mod.rs:87](../ergo-sandbox/src/ingest/mod.rs#L87), [identity.rs:59](../ergo-sandbox/src/identity.rs#L59), [map/source.rs:45](../ergo-sandbox/src/map/source.rs#L45), and new `tests/evidence_provenance.rs`. Preserve existing byte-only low-level functions, but they cannot construct an evidence-bearing deployment claim without resolving provenance again.

**Buys:** synthetic compilation and structural matching cannot accidentally become assertions about deployed code. **Dependencies:** P00.

**Acceptance P01:** `synthetic_binding_survives_export_and_import`, `structural_match_does_not_establish_deployment`, `missing_registers_are_not_empty_registers`, and `changing_any_premise_invalidates_cached_evidence`. Every ingested artifact in the 28-row recorded corpus has explicit binding-origin status; all 28 source rows remain, including failures. An artifact with no deployment comparison is `deploymentIdentity: unknown`, not false or verified. No hypothetical/default field disappears during case round-trip.

**Shippable state:** cases can be saved and inspected, but cannot claim transaction acceptance. Provenance has orthogonal states: hypothetical, caller-supplied, source-recorded, and independently checked where supported. A source-recorded box is not automatically proven unspent at a historical height.

### P02 — Add canonical boxes and transaction construction

**Change:** build a strict codec using node wire types, full box creation references, and canonical transaction signing bytes, separately from permissive `ScenarioBox` marshalling.

**Touch:** new `ergo-sandbox/src/evidence/wire.rs`, [box_build.rs:50](../ergo-sandbox/src/box_build.rs#L50) as the legacy boundary, [scenario.rs:204](../ergo-sandbox/src/scenario.rs#L204) as the conversion boundary, node `ergo-ser` transaction/box APIs, and new `tests/evidence_wire.rs` plus `tests/fixtures/evidence/`.

**Buys:** a witness has the actual bytes/IDs/message it asks the node to evaluate. **Dependencies:** P01.

**Acceptance P02:** `wire_box_id_matches_full_serialization`, `transaction_id_and_message_use_node_bytes_to_sign`, `legacy_box_missing_reference_cannot_be_promoted`, and `invalid_tree_never_becomes_empty_bytes`. Pin complete bytes and expected IDs for at least two boxes with different transaction references/output indices and one transaction; changing the reference must change the ID. A synthetic constructor creates a new explicitly hypothetical object; it never repairs an allegedly observed box's ID behind the caller's back.

**Shippable state:** canonical cases are constructible, but unvalidated. Play and scenario eval continue to use their explicitly synthetic path. No existing localStorage state is rewritten.

### P03 — Invoke the pinned node's full validator

**Change:** validate canonical transaction bytes through `ergo_validation::tx::validate_transaction` with explicit UTXOs, protocol parameters, network rules, and block context, and return a non-forgeable accepted-execution value.

**Touch:** [Cargo.toml:14](../Cargo.toml#L14), [ergo-sandbox/Cargo.toml:10](../ergo-sandbox/Cargo.toml#L10), new `evidence/validate.rs`, and `tests/node_validation.rs`. The available upstream entry is [pinned tx/mod.rs:122](/home/rkadias/.cargo/git/checkouts/ergo-99b30a0efa25f868/9468043/ergo-validation/src/tx/mod.rs:122); its checked-transaction type begins at line 83. Do not implement the missing rules in `txcheck`.

**Buys:** transaction acceptance comes from the node pipeline rather than Forge's balance/reduction approximation. **Dependencies:** P02.

**Acceptance P03:** `full_pipeline_matches_pinned_node_vectors` consumes a pinned manifest of at least eight named cases: accepted keyless spend, duplicate-input rejection, future-output rejection, canonical-encoding rejection, output monetary/size constraint rejection, aggregate-cost rejection, missing-UTXO rejection, and storage-rent acceptance. Expected outcomes/stages come from upstream vectors or the full node path, not `txcheck`. `incomplete_context_is_not_node_acceptance` covers missing network rules/activation/parameters. `accepted_execution_cannot_be_deserialized_or_fabricated` requires a fresh validation call even when importing an old accepted report. The fixture harness verifies node revision and every case hash before execution; all eight IDs are required and none may be ignored.

**Shippable state:** the library can report `node-accepted` **against the supplied state**. This does not establish that the state is real, that a property is violated, or that a transaction will be mined. No legacy endpoint changes the meaning of `valid`.

### P04 — Produce real transaction-bound proofs

**Change:** connect the existing prover to canonical signing bytes for explicitly owned funding keys and require full validation of the signed transaction.

**Touch:** new `evidence/sign.rs`, [prove.rs:1](../ergo-sandbox/src/prove.rs#L1), the node wallet dependency already present in [ergo-sandbox/Cargo.toml:16](../ergo-sandbox/Cargo.toml#L16), and new `tests/transaction_proofs.rs`.

**Buys:** attacker-labelled funding inputs no longer substitute for possession of the proof their script requires. **Dependencies:** P03.

**Acceptance P04:** `owned_p2pk_funding_signs_actual_transaction`, `changed_transaction_invalidates_proof`, `declared_public_key_without_proof_is_not_acceptance`, and `replay_bundle_contains_no_secret`. Scope is one standard DLog funding input plus keyless protocol inputs; imported valid proofs remain usable. Do not expand drain to keyed victim/companion attacks, implement new sigma protocols, or add browser key storage.

**Shippable state:** accepted signed executions exist; property confirmation still does not. Hypothetical key fixtures remain hypothetical, regardless of successful proof verification.

### P05 — Create property claims and an offline replay command

**Change:** add `ergo-es replay <case.json> --json`, which validates a canonical case, evaluates the existing versioned extraction property, and emits a claim bound to the complete evidence bundle.

**Touch:** new `evidence/{claim,replay}.rs`, extract the existing accounting into `drain/accounting.rs` without semantic expansion, [drain.rs:2267](../ergo-sandbox/src/drain.rs#L2267), [bin/ergo-es.rs:21](../ergo-sandbox/src/bin/ergo-es.rs#L21), `tests/claim_replay.rs`, and the public/owned evidence fixture manifest.

**Buys:** the user can reproduce a specific accepted property violation rather than trust a lint badge. **Dependencies:** P04.

**Acceptance P05:** `claim_requires_accepted_execution_and_violated_property`, `unknown_policy_cannot_confirm`, `offline_replay_reproduces_claim`, and `wrong_companion_or_policy_invalidates_claim`. Require at least three distinct accepted transaction bundles, at least two positive extraction claims across two named families, and at least one accepted nonviolating control. One family is the existing public USE incident; one is an authored fixed-rate sale/mutant pair. The USE case must preserve actual archived input bytes and record context provenance; missing source evidence blocks that fixture rather than being replaced by invented fields. A fixed-contract counterfactual gets newly serialized hypothetical boxes and IDs, not a changed script under a real box ID. Count observed and hypothetical cases separately. All negative controls have zero confirmed violations.

**Shippable state:** `confirmed-violation` belongs to a property/case, never a `Finding`. Its scope is “node-accepted under these premises and violates this declared property.” A hypothetical-state claim cannot render as a deployed exploit. The command is offline and broadcasts nothing.

### P06 — Promote drain candidates through the evidence boundary

**Change:** keep today's search unchanged but offer an explicit promotion step that constructs, signs where supported, validates, and scores a candidate through P02–P05.

**Touch:** [drain.rs:415](../ergo-sandbox/src/drain.rs#L415), [drain.rs:1070](../ergo-sandbox/src/drain.rs#L1070), [drain.rs:2875](../ergo-sandbox/src/drain.rs#L2875), [audit/triage.rs:100](../ergo-sandbox/src/audit/triage.rs#L100), and new `tests/drain_promotion.rs`. Promotion consumes an explicit canonical case/material binding; it does not infer live availability from a `ScenarioBox`.

**Buys:** search results can earn transaction/property authority, and failed promotion identifies the gap. **Dependencies:** P05.

**Acceptance P06:** `generated_public_incident_candidate_promotes`, `unbacked_synthetic_material_stays_preflight`, `missing_funding_proof_blocks_promotion`, and `associated_lint_is_not_the_confirmed_claim`. At least one generated public-incident-family candidate must promote under the existing caps and family. Archive candidate-to-canonical mapping and any hypothetical funding premises. A label-derived ID, invented filler-token supply, unavailable data box, or incomplete context returns a named promotion failure; it does not become a node claim. Run the existing `the_corpus_is_measured_and_does_not_regress` gate with its original oracle/version and retain all eight rows. New node validation results use a separate ledger; never overwrite the 4/5 baseline or relabel failures to preserve that percentage.

**Shippable state:** every drain report has a preflight result and optional, separately tagged claim references. Only the latter can say `confirmed-violation`. The old #95 reproduction record remains useful and never changes meaning again.

### P07 — Make the report an obligation queue, not a severity count

**Change:** group repeated static observations by a stable obligation key, attach explicit conditional premises/discharge reasons, and keep property claims separate from static review priorities.

**Touch:** [finding.rs:38](../ergo-sandbox/src/audit/finding.rs#L38), [audit/context.rs:78](../ergo-sandbox/src/audit/context.rs#L78), report assembly in `evidence/claim.rs`, [dto.rs:17](../ergo-web/src/dto.rs#L17), new `tests/obligation_reports.rs`, and `tests/fixtures/evidence/precision.json`. Do not expand detector semantics or automatic delegation inference.

**Buys:** the user sees one unresolved obligation with supporting anchors rather than many implied vulnerabilities, and a suppression cannot outlive its premises. **Dependencies:** P06.

**Acceptance P07:** `duplicate_observations_keep_all_anchors_in_one_obligation`, `suppression_invalidates_on_premise_change`, `conditional_discharge_exports_the_full_premise_set`, and `curated_precision_has_positive_and_negative_controls`. Pin exactly six initial benign-case IDs: malformed optional witness rejection, missing candidate-output register rejection, a well-formed SELF register case, fixed-recipient paid output, required companion identity binding, and intended permissionless height release. Use authored/public cleared fixtures only. Pin two positive claim cases from P05. Require zero confirmed violations on all six benign cases and two retained positive claims; dropping every finding cannot pass. A duplicate fixture with twelve anchors for the same obligation must produce one group and preserve all twelve anchors. Unrecognized obligations remain separate and unconfirmed. This is a curated report-precision gate, not a claim of live-protocol precision.

**Shippable state:** syntax observations, contextual discharges, preflight reproductions, and node-validated property claims coexist with distinct names. Legacy `severity` is documented as review priority; new claim impact is contextual, and unknown impact remains unknown.

### P08 — Let the browser replay the same saved evidence

**Change:** add one evidence-file replay operation in Read, using the same P05 runner and result schema as the CLI.

**Touch:** [app.rs:93](../ergo-web/src/app.rs#L93), new `ergo-web/src/routes/replay.rs`, existing [engine.rs:38](../ergo-web/src/engine.rs#L38), [ui/app.js:1749](../ui/app.js#L1749), `ergo-web/tests/evidence_replay.rs`, and `ui/tests/evidence-replay.test.js`.

**Buys:** both skins can inspect the actual evidence rather than reserve the stronger instrument for CLI users. **Dependencies:** P07.

**Acceptance P08:** `http_and_cli_replay_have_identical_semantic_results`, `historical_and_hypothetical_state_labels_are_visible`, `legacy_preflight_does_not_use_node_claim_badge`, and `replay_request_never_fetches_or_broadcasts`. The UI fixture imports one positive bundle, one accepted nonviolating bundle, and one incomplete bundle; all must render the precise claim scope and missing-premise list. Use a versioned `/api/v2/replay` response. No search endpoint, server job storage, private-key entry, or chain lookup is added.

**Shippable state:** a finite workbench with truthful legacy experiments and one shared strict replay path. Stop here; review the scoreboard before authorizing another plan.

## 4. Gates a harness can read

The following JSON block is the **policy source**, not illustrative pseudocode. P00 adds a small standard-library Python runner that extracts the uniquely marked block from this file. Do not duplicate its thresholds in test code. Tests load the same block and the fixture manifests; the runner validates schema, verifies required test names are discovered, rejects ignored/zero-test runs, runs dependencies, and exits nonzero on any unmet requirement. New paths and test names in this section are planned deliverables, not files claimed to exist today.

<!-- roadmap-policy:v1 -->
```json
{
  "schemaVersion": 1,
  "baselineRev": "ee4ac6a874531872c27828d94e21e4fe7a1d7f7c",
  "completedThrough": "P06",
  "maxActiveImplementationBranches": 1,
  "maxOpenImplementationPrs": 1,
  "units": [
    {
      "id": "P00",
      "depends": [],
      "days": 2,
      "package": "ergo-sandbox",
      "target": "claim_contract",
      "tests": [
        "scenario_reproduction_is_not_confirmation",
        "legacy_results_are_explicitly_preflight_or_simulation",
        "legacy_record_cannot_import_as_verified"
      ],
      "implemented": true
    },
    {
      "id": "P01",
      "depends": [
        "P00"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "evidence_provenance",
      "tests": [
        "synthetic_binding_survives_export_and_import",
        "structural_match_does_not_establish_deployment",
        "missing_registers_are_not_empty_registers",
        "changing_any_premise_invalidates_cached_evidence"
      ],
      "implemented": true
    },
    {
      "id": "P02",
      "depends": [
        "P01"
      ],
      "days": 4,
      "package": "ergo-sandbox",
      "target": "evidence_wire",
      "tests": [
        "wire_box_id_matches_full_serialization",
        "transaction_id_and_message_use_node_bytes_to_sign",
        "legacy_box_missing_reference_cannot_be_promoted",
        "invalid_tree_never_becomes_empty_bytes"
      ],
      "implemented": true
    },
    {
      "id": "P03",
      "depends": [
        "P02"
      ],
      "days": 6,
      "package": "ergo-sandbox",
      "target": "node_validation",
      "tests": [
        "full_pipeline_matches_pinned_node_vectors",
        "incomplete_context_is_not_node_acceptance",
        "accepted_execution_cannot_be_deserialized_or_fabricated"
      ],
      "caseIds": [
        "accepted-keyless-spend",
        "duplicate-input-rejection",
        "future-output-rejection",
        "canonical-encoding-rejection",
        "output-constraint-rejection",
        "aggregate-cost-rejection",
        "missing-utxo-rejection",
        "storage-rent-acceptance",
        "reemission-rule-rejection"
      ],
      "implemented": true
    },
    {
      "id": "P04",
      "depends": [
        "P03"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "transaction_proofs",
      "tests": [
        "owned_p2pk_funding_signs_actual_transaction",
        "changed_transaction_invalidates_proof",
        "declared_public_key_without_proof_is_not_acceptance",
        "replay_bundle_contains_no_secret"
      ],
      "fixtureManifest": "ergo-sandbox/tests/fixtures/evidence/proof-manifest.json",
      "caseIds": [
        "owned-p2pk-with-keyless-companion"
      ],
      "implemented": true
    },
    {
      "id": "P05",
      "depends": [
        "P04"
      ],
      "days": 4,
      "package": "ergo-sandbox",
      "target": "claim_replay",
      "tests": [
        "claim_requires_accepted_execution_and_violated_property",
        "unknown_policy_cannot_confirm",
        "offline_replay_reproduces_claim",
        "wrong_companion_or_policy_invalidates_claim"
      ],
      "implemented": true,
      "fixtureManifest": "ergo-sandbox/tests/fixtures/evidence/claim-manifest.json",
      "caseIds": [
        "use-incident",
        "sale-mutant-unpaid",
        "sale-fixed-paid",
        "sale-fixed-unpaid"
      ],
      "positiveCaseIds": [
        "use-incident",
        "sale-mutant-unpaid"
      ],
      "negativeCaseIds": [
        "sale-fixed-paid",
        "sale-fixed-unpaid"
      ]
    },
    {
      "id": "P06",
      "depends": [
        "P05"
      ],
      "days": 4,
      "package": "ergo-sandbox",
      "target": "drain_promotion",
      "tests": [
        "generated_public_incident_candidate_promotes",
        "unbacked_synthetic_material_stays_preflight",
        "missing_funding_proof_blocks_promotion",
        "associated_lint_is_not_the_confirmed_claim"
      ],
      "implemented": true
    },
    {
      "id": "P07",
      "depends": [
        "P06"
      ],
      "days": 4,
      "package": "ergo-sandbox",
      "target": "obligation_reports",
      "tests": [
        "duplicate_observations_keep_all_anchors_in_one_obligation",
        "suppression_invalidates_on_premise_change",
        "conditional_discharge_exports_the_full_premise_set",
        "curated_precision_has_positive_and_negative_controls"
      ],
      "implemented": false
    },
    {
      "id": "P08",
      "depends": [
        "P07"
      ],
      "days": 3,
      "package": "ergo-web",
      "target": "evidence_replay",
      "tests": [
        "http_and_cli_replay_have_identical_semantic_results",
        "historical_and_hypothetical_state_labels_are_visible",
        "legacy_preflight_does_not_use_node_claim_badge",
        "replay_request_never_fetches_or_broadcasts"
      ],
      "implemented": false
    }
  ],
  "thresholds": {
    "ingestRows": 28,
    "ingestCompiledFloor": 26,
    "wireBoxFixturesMin": 2,
    "wireTransactionFixturesMin": 1,
    "nodeVectorCasesMin": 8,
    "acceptedBundlesMin": 3,
    "confirmedPropertyClaimsMin": 2,
    "claimFamiliesMin": 2,
    "publicIncidentClaimMin": 1,
    "acceptedNonviolatingControlMin": 1,
    "promotedGeneratedCandidatesMin": 1,
    "benignPrecisionCases": 6,
    "benignConfirmedViolationsMax": 0,
    "positivePrecisionCases": 2,
    "positivePrecisionClaimsMin": 2,
    "duplicateInputAnchors": 12,
    "duplicateOutputGroups": 1,
    "duplicateOutputAnchors": 12
  },
  "frozenPreflight": {
    "artifact": "examples/mutants/answer-key.json",
    "cases": 8,
    "proven": 5,
    "attributable": 4,
    "confounded": 0,
    "maxProbes": 50000,
    "maxPermutations": 120
  },
  "stopRecords": "docs/roadmap-stops.json",
  "scoreboard": {
    "path": "docs/roadmap-metrics.json",
    "ingestion": "docs/ingestion-lithos-results.json",
    "decompiler": "ergo-sandbox/tests/fixtures/decompile_corpus_expectations.json",
    "history": "examples/mutants/answer-key.custody-v1.json"
  },
  "resolvedStopRecords": [
    {
      "unit": "P05",
      "recordSha256": "0639e75daaab01d5a67ebb25a797b0cdbbba197474c0b0fec29815706ed67986",
      "decision": "docs/P05-DECISION.md",
      "decisionSha256": "cbc1096d4d208a900c2be4c7091ba49feb2bf1722e8dca77f2d3567ad4d02aa2"
    }
  ]
}
```
<!-- /roadmap-policy:v1 -->

P00 implementation amendment: result provenance is descriptive legacy metadata, not P01's case schema. The UI gate uses the production renderer against a DOM, with the exact dev dependency locked in `ui/package-lock.json`. The record-only command is `python3 scripts/roadmap_gate.py --scoreboard-only`; reports default to `target-p00/roadmap-gates/` and include command output and artifact hashes. The explicit `implemented` flag separates planned product units from missing required gates; it never passes a planned unit. These clarify the original gate specification without reducing any threshold.

The runner's gate command is `python3 scripts/roadmap_gate.py --require P05` (substitute the unit ID). It runs each registered target via `cargo test --release -p <package> --test <target> -- --show-output`, after checking the required test names through `--list`. Whole targets run, not filters that might silently select nothing. Install the pinned DOM-test dependency with `npm ci --prefix ui`. P00 additionally runs `python3 -m unittest discover -s scripts -p 'test_roadmap_gate.py'` and `node --test ui/tests/claim-labels.test.js`; P08 runs `node --test ui/tests/evidence-replay.test.js`. Those extra commands are fixed runner requirements with self-tests, not optional notes.

`python3 scripts/roadmap_gate.py --all-goals` runs the full goal set and returns nonzero while anything is missing or failing. **It is expected to fail today**, first because the runner/new tests do not exist, then because the product obligations are not met. An infrastructure failure is reported as `missing-gate`, never as experimental evidence that a contract is unsafe. The runner writes a machine report distinguishing `missing-gate` (required infrastructure/evidence absent), `failed` (executed product gate failed), `passed`, `stopped`, and `unimplemented` (a registered unit explicitly marked `implemented: false`); none of missing-gate/failed/stopped/unimplemented counts as completion. P00 registers P01–P08 as unimplemented, so the full-goal check fails for unfinished product work rather than misclassifying planned files as broken infrastructure.

To keep every release shippable, repository CI runs `--through-completed` plus the next PR's `--require` gate. `completedThrough` advances only in the PR whose gates pass, with all predecessors rerun. The status report remains separate from the green CI prefix: unimplemented goals cannot disappear behind a green release. A stopped unit blocks downstream dependent units until a governing-plan amendment removes them or changes the dependency with a new acceptance gate, or a policy-pinned governing decision resolves that exact stop with recovered evidence and reopens the original gate. Reopening does not award a pass: the original unit and predecessor tests must execute successfully. The stop record and its original evidence remain history.

P00 also adds a record-only scoreboard check that reads committed JSON, verifies corpus membership/thresholds and links, and exits nonzero if this document's baseline claims disagree. P01 adds origin validation of the 28 ingestion rows. P03–P08 register `tests/fixtures/evidence/manifest.json` (P04 uses the separately policy-registered `proof-manifest.json`, as amended below) with case ID, family, source kind, exact file hashes, node revision, property version, expected acceptance/claim status, and publication eligibility. P07's `precision.json` supplies the eight frozen labels and references those case IDs. No harness may accept a hand-set `confirmed: true` as evidence: replay derives that result.

Existing release checks remain required: `cargo fmt --all -- --check`, workspace clippy with warnings denied, workspace tests, and the existing cost-trace CI variants ([ci.yml:51](../.github/workflows/ci.yml#L51)). P06 additionally runs `cargo test --release -p ergo-sandbox --test mutation_corpus -- --nocapture`. The per-entry decompiler test remains `bundled_contracts_and_compiled_fixtures_round_trip` ([decompile_corpus.rs:160](../ergo-sandbox/tests/decompile_corpus.rs#L160)); do not loosen its existing exact entries to make an evidence fixture fit.

| Prose bar being retired | Executable replacement | Status: baseline expectation → P00 result |
|---|---|---|
| “The reducer was consulted, so the finding is confirmed” | P00 `scenario_reproduction_is_not_confirmation`; P05 claim-constructor/replay tests | **PASS in P00:** the named test now requires v2 scenario reproduction and `nodeValidated: false`; baseline emitted `confirmed`. |
| “Synthetic SELF is disclosed” | P00 browser test renders both positive and negative synthetic results | **PASS in P00:** production Read/Write renderers keep the warning visible on positive and negative results; baseline hid the positive banner. |
| “The transaction would validate” | P02 wire fixtures + P03 eight node-vector cases + P04 signing tests | **FAIL / unavailable:** current path is unsigned preflight with incomplete references/context. |
| “We read the protocol” | P01 all 28 rows retained, origin required, unverified deployment identity remains unknown | **Incomplete:** row/binding data exists; the artifact boundary can discard it. |
| “This lint was confirmed by a drain” | P06 `associated_lint_is_not_the_confirmed_claim` | **Baseline failed; P00 removes the confirmation label.** P06 promotion/causation gate remains unimplemented. |
| “Precision improved” | P07 six benign cases, zero false confirmations, two retained positives, grouping/anchor counts | **UNKNOWN / gate absent:** no equivalent committed precision corpus today. |
| “The browser uses the stronger instrument” | P08 HTTP/CLI equality and visible provenance tests | **Unavailable:** there is no strict evidence-replay route. |

These are specified required outcomes, not claims that these new tests were run while writing the roadmap. If a current expected failure unexpectedly passes when its harness lands, record that result and proceed; do not manufacture a failure to match the plan.

## 5. Scoreboard — report this, not feature counts

Values are from `git show HEAD:<artifact>` at the baseline revision. Summing committed JSON buckets is record inspection, not a benchmark rerun. P00 writes these identities and denominators into `docs/roadmap-metrics.json`; subsequent gate runs update measured values with their source commit, harness version, and fixture hashes. Unknown fields remain JSON `null` with a named producer, never zero.

| Metric | Honest value today | Target by P08 / producing gate |
|---|---|---|
| Static source ingest | **26/28 (92.9%)** after inference; baseline without inference **3/28**. Two explicit failures. Source: [ingestion-lithos-results.json:1](ingestion-lithos-results.json#L1), `after`/`baseline`. | Preserve **28 rows and ≥26 compilations**; do not call this deployment coverage. P01. No 28/28 compiler promise this cycle. |
| Source inventory provenance | **28/28 source hashes recorded**, with compiler and corpus revisions in the same ingestion artifact. | Preserve **28/28** and attach binding provenance to every successful analysis artifact. P01. |
| Exact deployed-source equivalence rate for that corpus | **UNKNOWN.** The ingestion artifact supplies synthetic bindings, not a per-contract deployment-byte comparison. Structural matcher results are not equivalent evidence. | Remain explicitly UNKNOWN until a deployment-comparison manifest exists; P01 must prevent substitution of the ingest percentage. No live deployment-recovery campaign is queued. |
| Current attributable drain detection | **4/5**, eight total mutants, three excluded as keyed-insider; zero confounded detections under `recognized-attacker-receipts-v1`. Source: [answer-key.json:1](../examples/mutants/answer-key.json#L1). | Preserve this **versioned preflight result**, including all rows, objective and caps. P06. Target is not “80% of vulnerabilities” and not forced 5/5. |
| Validity of search measurements | Proven cases have **0 invalid synthesis-on probes** in their recorded rows; excluded M1 has **1,976 invalid probes**. M7 reaches the **50,000 cap**. Same answer key. | Preserve rejection buckets and exclusions; no M1 key-refusal claim derived from malformed probes. Node-promotion failures get separate counts. P06. A cap flag alone does not justify steering. |
| Node-validated property-claim count | **0.** There is no full-node validator producer in the baseline dependency/call path. | **≥2 distinct positive claims, ≥2 families**, with ≥1 public-incident case; source kinds and residual premises visible. P05. Not two claims attached to different lints on the same transaction. |
| Canonical accepted transaction bundles | **0 produced by the proposed boundary**; existing scenario/witness JSON is not counted. | **≥3**, including a signed funding case and ≥1 accepted nonviolating control; at least eight acceptance/rejection vector cases in total. P03–P05. |
| Search-generated candidates promoted through the node | **0.** Existing hits only replay txcheck. | **≥1** under the frozen candidate family/caps, with explicit material/state premises. P06. |
| Actionable precision on real audits | **UNKNOWN.** The historical 145 findings / 0 real observations in [architecture review §5](ARCHITECTURE-REVIEW-CODEX.md#5-the-precision-problem) are evidence of a failed run, not a current versioned precision corpus. | Keep real-audit precision UNKNOWN until a permissioned, hashed, adjudicated case set exists. P07 delivers a separate curated figure: **2/2 positive claims, 0/6 benign cases falsely confirmed**, with all raw observations retained. Do not relabel it real-world precision. |
| Review duplication | **UNKNOWN** under an obligation-level definition; current artifacts count lint occurrences. | On the pinned duplicate case: **12 anchors → 1 obligation, all 12 retained**. P07. Subsequent real-audit effort needs an adjudicated case set, not an invented time-saved number. |
| Decompiler byte recovery | Current expectation file has **339 entries: 238 byte-identical, 91 recompiling with different bytes, 10 initial compilation failures**. Thus **238/329** byte-identical among obtained trees. Source: [decompile_corpus_expectations.json:1](../ergo-sandbox/tests/fixtures/decompile_corpus_expectations.json#L1). | Preserve every previously exact entry and explicit divergence/failure. New evidence fixtures add separately recorded rows; do not use old 328-entry totals as today's denominator. Existing decompiler gate. |

The newer 4/5 and 26/28 figures supersede the historical 0/4 and 3/28 as current recorded results; they do not erase the lessons. Some prose fields inside `answer-key.json` still describe the old confounded M4 baseline. P00 must reconcile those descriptions against the current row data while preserving `answer-key.custody-v1.json` as history; do not rerun or alter the numeric baseline merely to clean its documentation.

Three-month success means P08's gates pass and the table distinguishes node evidence from preflight, with the finite counts above. No increase in detector count, contract gallery size, or search budget counts toward success. If only P00–P03 ship before a stop rule fires, report that completed prefix and keep all stronger metrics unfulfilled.

## 6. Stop rules — record a boundary instead of extending the project

P00's runner validates `docs/roadmap-stops.json` when present. Each stop record must contain `unitOrProposal`, `reasonCode`, `attemptCommit`, `daysSpent`, `gateResults` with command/exit code/evidence hashes, `retainedCapability`, `unsupportedClass`, and `reopenEvidenceRequired`. A record without these fields fails the policy check. Stopping is a successful scope decision but **not** a passing feature gate; dependent work pauses. Neither a TODO nor another worktree is a stop record.

| Trigger | Required decision | Honest retained boundary / reopening evidence |
|---|---|---|
| Any PR reaches its stated working-day ceiling without its gate | Stop implementation and record `timebox`. Land only an independently coherent prefix whose existing gate passes; otherwise retain the last shippable main. | Name the unmet test and unsupported case. Another attempt requires a smaller replacement unit with its own gate; no automatic extension. |
| P03 cannot reach the pinned full validator without porting rules into Forge | Stop at canonical unvalidated cases. One minimal upstream API-exposure patch may replace P03 within its timebox; no parallel adapter rewrite. | “Unsigned preflight only; node acceptance unavailable.” Reopen with an upstream API and an executable vector proving the missing path. |
| A public incident lacks full box/context evidence for P05 | Stop that claim's promotion, retain its existing preflight replay, and mark `missing-provenance`. | “Reproduced under supplied scenario; historical state not established.” Do not invent values or use a confidential finding to satisfy the public gate. Reopen with a complete, pinned, publishable source artifact. |
| P06 cannot promote a generated candidate without a new search axis or invented material | Stop P06 rather than widen generation. Preserve P05's manual replay capability. | “Search returns preflight candidates; canonical construction unavailable for these candidates.” Reopen with a material-preserving adapter or independently evidenced candidate from the existing family. |
| The node rejects a previously counted preflight hit | Keep the disagreement. Do not bypass its validation stage or increase a box floor until it happens to pass. | Classify wire/context/funding/property mismatch; separate node and preflight denominators. A repaired witness must be a new version with its changed premises visible. |
| A benign precision case is confirmed or either positive control disappears | Stop report/promotion changes. Do not lower severity or suppress the case to pass. | Keep the active obligation unconfirmed. Reopen after the property/context error is fixed and the frozen eight-case gate passes. |
| General composition needs proofs outside literal slots/current authenticated binding vocabulary | Do not expand P07 into a theorem prover. | “Conditional discharge under supplied co-execution; transitive/dynamic obligation unresolved.” Reopen only with one independently proved missed binding and one adversarial omission counterexample. |
| A proposed new axis has no independent witness and clean original under a pinned property | Refuse the work proposal. | Add an unsupported-class entry, not a detector. Future search work must first meet the twenty-mutant/all-operator gate and preserve the current denominator separately. |
| A future solver experiment reaches ten working days without finding a pre-registered arithmetic witness unaided | Abandon the solver experiment; do not create a second interpreter or weaken replay. | “Computed-value search unsupported; supplied arithmetic scenarios can still be replayed.” Reopen only with a smaller typed fragment and a new executable benchmark. This is a ceiling rule, **not authorization to start the experiment**. |
| A future steering comparison finds no additional held-out attributable detection at equal oracle-call and wall-time budgets, or has no witnessed in-family cap miss | Abandon steering. No optimization project follows a null result. | “Deterministic bounded enumeration; cap/traversal reported.” Reopen with a witnessed in-family miss and a preregistered equal-budget benchmark. Both budgets and seed set must be pinned before comparison. |
| Someone proposes phase 6 autonomous discovery or a live sweep before target/property/provenance and actionable precision are established | Reject the proposal. | Manual nominated cases and offline replay remain the product. New automation requires a separate governing decision; an appendix entry is not that decision. |

Do not make an unmet fixture-count gate disappear by declaring a narrower success after the fact. A deliberate scope reduction updates this roadmap's manifest, retains the previous target/result in a stop record, and changes the public claim. The current release may remain useful while the proposed milestone is not achieved.

## 7. Doc and claim hygiene — P00 checklist

- [x] Replace [README.md:3](../README.md#L3) with section 1's product paragraph; delete the unqualified “a verdict here is the consensus verdict.”
- [x] Rewrite [README.md:17](../README.md#L17) and [README.md:146](../README.md#L146) to say unsigned preflight checks selected conditions, not full transaction acceptance or future inclusion.
- [x] Replace the “Every answer comes from…” claim at [README.md:91](../README.md#L91) with method-specific descriptions; static lints and recognition do not consult the reducer.
- [x] Replace hunt's “a hit is a transaction anyone can build” promise in [README.md:135](../README.md#L135) and [hunt.rs:8](../ergo-sandbox/src/hunt.rs#L8) with “a passing sampled scenario; canonical transaction validation has not run.”
- [x] Show the synthetic SELF warning for **every** `selfSynthetic` result at [ui/app.js:279](../ui/app.js#L279), and do the same for Write's result at [ui/app.js:719](../ui/app.js#L719).
- [x] Rename positive hunt labels at [ui/app.js:212](../ui/app.js#L212) to “Sample passed without a proof” / “Preserving-output sample passed”; label residual keys as observed under the probes, not an exhaustive list of who can spend.
- [x] Replace [ui/app.js:1765](../ui/app.js#L1765) with “Preflight passed — full node validation has not run”; label failure “Preflight failed,” preserving signature counts and problem details.
- [x] Replace Play's “real ids”/“ids the chain would give” descriptions at [README.md:86](../README.md#L86) and [play.rs:1](../ergo-sandbox/src/play.rs#L1) with “deterministic simulation IDs”; state that scenario proofs use a supplied/default message.
- [x] Rename #95's `confirmed` state at [audit/triage.rs:114](../ergo-sandbox/src/audit/triage.rs#L114) to `reproduced-in-scenario`, version the record, and update [tests/triage.rs:12](../ergo-sandbox/tests/triage.rs#L12) plus CLI/UI documentation together.
- [x] Label static severity as review priority in [dto.rs:17](../ergo-web/src/dto.rs#L17) and browser finding displays; `consensusReducerConsulted` must never imply node validation or causation.
- [x] Remove mineability claims from [drain.rs:13](../ergo-sandbox/src/drain.rs#L13) and [txcheck.rs:1](../ergo-sandbox/src/txcheck.rs#L1); document preflight as a candidate filter.
- [x] Mark the old roadmap historical at its top, link here from [README.md:155](../README.md#L155) and [workbench-PLAN.md:1](workbench-PLAN.md#L1), and remove their independent active build lists.
- [x] Replace the old cross-cutting oracle invariant at [drain-hunt-roadmap.md:370](superpowers/specs/2026-09-08-drain-hunt-roadmap.md#L370) with a link to this plan's authority rules; keep the old text only as explicitly superseded history.
- [x] State the surviving invariants in the active docs: exact pinned engine; claims require full node validation plus a versioned property; premises survive export; no miss means safe; truncation visible; private handling of third-party cases; unsupported semantics fail closed for promotion.
- [x] State that source inference is synthetic, a complete lift is recovery coverage, and `same_program_with_differing_constants` is structural matching; link [ingestion.md:22](ingestion.md#L22), [audit-context.md:10](audit-context.md#L10), and [identity.rs:18](../ergo-sandbox/src/identity.rs#L18) rather than duplicating stronger promises.
- [x] Reconcile stale mutation prose with current committed rows and publish section 5's denominators; retain historical artifacts and label their revisions.

These changes are one atomic claim-contract release, not optional documentation after the architecture lands. Automated gates exercise rendered output and result envelopes; a string-search-only test is insufficient to establish visible disclosure.

## Not yet planned — questions required before another queue exists

No item below has a slot, estimate in the active schedule, or permission to spawn a worktree. Promotion into the plan requires a new PR-sized unit, machine-readable gate, explicit displaced work, and a stop rule.

| Capability | Question that must be answered first |
|---|---|
| Remaining v0/UnsignedBigInt compilation support | Can the pinned compiler expose serialization-version selection without source rewrites? Supply the upstream failing/passing fixture and API design; do not turn static ingest into another compiler. |
| First-token replacement, multiple output edits, register/data-input expansion | Which independently validated missed witness requires this exact axis, and has the broader corpus gate been satisfied? M5's existing miss stays visible meanwhile. |
| General B / input-set selection / dynamic delegation | What narrow obligation vocabulary can express a real required co-spend and reject its omission without treating map adjacency as execution? |
| Keyed insiders / collusion | Which property limits an authorized principal, and what signed positive/control pair proves the new attacker model? P04 owns funding proofs only. |
| Canonical Play transitions / depth-two chains | What saved sequence requires real references, and what invariant distinguishes reachable bad state from arbitrary funding? Do not migrate browser state without a versioned compatibility decision. |
| Solver (old phase 4) | Which typed arithmetic fragment and held-out exact-value witness justify a ten-day experiment? A trace string is not a constraint language. |
| Steering (old 3d) | Which witnessed in-family miss is caused by budget, and what stable feedback distinguishes execution locations? A high opcode-cost row is not branch coverage. |
| Real-protocol actionable precision | Which permissioned case set can be hashed, adjudicated, and replayed without publishing confidential findings? What are the claim and abstention denominators? |
| Automatic historical state verification | Which source can supply and prove the required snapshot/issuance/availability facts? Archived explorer responses establish observations, not consensus membership. |
| New recipes, wallet integration, graph dashboard, autonomy | No current product justification. Identify a blocked reproducible experiment that cannot be served by the shipped workbench before proposing any of them. |

P00 verification/landing record: see [P00-REPORT.md](P00-REPORT.md). All sixteen
hygiene items above are implemented. `completedThrough` records the passing
product-gate prefix, not a claim that a Git commit or remote landing occurred.
The sandbox mounted `.git` read-only, blocking the requested docs-first commit.

P01 execution record: see [P01-REPORT.md](P01-REPORT.md) and the
[evidence-case API boundary](evidence-cases.md). P01 is registered as implemented;
`completedThrough` advances to P01 after rerunning P00 and P01 green. Acceptance
names, numeric thresholds and frozen measurements are unchanged. At P01
completion, P02–P08 were unimplemented.

P02 execution record: see [P02-REPORT.md](P02-REPORT.md) and the
[node wire API boundary](evidence-wire.md). P02 is registered as implemented;
`completedThrough` advances to P02 after P00, P01 and P02 reran green. Acceptance
names, numeric thresholds and frozen measurements are unchanged. At P02
completion, P03–P08 were unimplemented. Codec vectors are not accepted transaction bundles.

P03 execution record: see [P03-REPORT.md](P03-REPORT.md) and the
[full validator API boundary](node-validation.md). P03 is registered as implemented;
`completedThrough` advances to P03 after P00–P03 reran green. The policy now names
the eight acceptance cases explicitly in `caseIds`, plus an enabled re-emission
rejection to test rule forwarding. This makes the existing prose requirements
machine-readable and adds a control; the minimum of eight and all other numeric
thresholds remain unchanged. At P03 completion, P04–P08 were unimplemented. These authored
hypothetical vectors do not change any frozen measurement or prove a property.

P03 gate-command amendment: the runner uses `--show-output` instead of
`--nocapture`. Actual P00/P02/P03 runs demonstrated that uncaptured diagnostics
can split Rust test-status lines, causing false `missing-gate` results even
with one test thread. Capturing and then showing diagnostics preserves complete
status lines and all output. Discovery, required names, whole-target execution,
ignored/zero-test rejection and every threshold are unchanged. The separately
requested P03 `--nocapture` command is also run directly.

P04 manifest amendment: P04 registers `proof-manifest.json` beside P03's
`manifest.json`, with its location and required experiment ID in the policy block.
The P03 harness binds its exact node-vector inventory; appending P04 rows there
would force a change to that completed gate and its corpus membership. P04's
separate manifest preserves those rows and adds the same required revision,
file-hash, family, source, property-status and publication metadata. No threshold,
acceptance name, old fixture or numeric baseline is changed.

P04 execution record: see [P04-REPORT.md](P04-REPORT.md) and the
[transaction proof API boundary](transaction-proofs.md). P04 is registered as
implemented. Its completed prefix advances only after P00–P04 rerun green.
The signed execution is hypothetical supplied-state evidence, not a property
claim. P05–P08 remain unimplemented; legacy endpoints and frozen search stay unchanged.

P05 attempt stopped on 2026-09-10 (local date), under the existing section 6
`missing-provenance` rule. See [P05-REPORT.md](P05-REPORT.md) and the
machine-readable [stop record](roadmap-stops.json). The required public USE
case lacked a fully backed validation context; a prototype's hypothetical-context
green results are not accepted. At that stop, P05 remained unimplemented and `completedThrough`
remained P04. No acceptance requirement or policy threshold was amended.

P05 governing decision after `e251f22`: **B, recover the historical source
material; preserve every original threshold.** See [P05-DECISION.md](P05-DECISION.md).
The original stop is retained with a policy-pinned resolution. This only reopens
the gate; it cannot grant completion. P05 uses a separate `claim-manifest.json`
so the completed P03/P04 fixture inventories remain unchanged. The recovery
harness re-derives the USE request from raw recorded artifacts, including actual
prior transaction costs, before counting it as the public-incident case.
P06–P08 remain unimplemented. No lowered-bar alternative is authorized.

P05 completion: P00–P05 and `--through-completed` through P04 reran green before
advancement. `completedThrough` is now P05. The final through-P05 and full-suite gates
passed; actual output is recorded in [P05-RECOVERY-REPORT.md](P05-RECOVERY-REPORT.md).
The boundary is source-recorded context plus declared-property validation, not
authenticated historical UTXO membership or canonical-chain certification.
