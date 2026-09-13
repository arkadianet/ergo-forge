# Roadmap v1 queue — historical record

Sections 2–3 below are preserved verbatim from `docs/ROADMAP.md` at
`a57df087df85813c7f90a6f6f9ad6aaf5b4597cd`. Relative links in the preserved
text retain their original `docs/ROADMAP.md` context. This queue is superseded
by the [v2 plan](../superpowers/specs/2026-09-13-forge-roadmap-v2.md).
The later M-series registration and completion text is copied below as well;
it also remains in the roadmap’s retained implementation history.

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

**Change:** add `ergo-es replay <bundle.json> --json`, which validates a canonical execution bundle, evaluates the existing versioned extraction property, and emits a claim bound to the complete evidence bundle.

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

M00–M05 registration (author-authorized following PR #97): the [mapping design](superpowers/specs/2026-09-10-comprehensive-mapping-design.md), sections 5–7, supplies the bounded post-P08 queue and replaces the parked mapping question. Only M00 implementation is authorized in this change. Section 2 freezes remain in force; registration grants no discovery edge proof authority. The six units are appended unchanged, initially unimplemented. Completion requires their own executable gates. See [M00 record](mapping/M00.md).

M00 gate completion: P00–P08 and all three registered M00 tests pass under
`--require M00`. Only M00 is implemented among the appended mapping units;
`completedThrough` is M00. The [M00 report](mapping/M00.md) records the fixed
32-case inventory, legacy measurements and full verification output. M01–M05
remain unimplemented. Registration does not authorize their implementation here.

M01 author-authorized implementation: the separate discovery and unresolved-relation
artifacts and all three registered gates are implemented. `--require M01` passed
through P00–P08 and M00 before advancing `completedThrough` to M01. See the
[M01 report](mapping/M01.md) for the import authority boundary and real gate output.
M02–M05 remain unimplemented. All section 2 freezes and pinned M00 data remain unchanged.

M02 author-authorized implementation: bounded literal and existential discovery
over supplied canonical material passes its three registered gates. Only M02's
implementation flag and the completed prefix advance; see the [M02 report](mapping/M02.md)
for exact supported/all-case recall, false hints, unresolved boundaries and real
verification output. M03–M05 remain unimplemented. M00/M01 and all section 2
freezes remain unchanged.

### M03 implementation record — 2026-09-10

The author authorized M03 after PR #97. The direct exact-tree checker and its
four original `mapping_necessity` tests retain the registered M02 dependency,
`ergo-sandbox` package and four-day ceiling. M03 alone advances the completed
prefix. The [M03 report](mapping/M03.md) records checked guarded co-spends,
authentication, distinctness, canonical accepted/omission controls and actual
gates. A narrow design amendment represents the already pinned `HEIGHT >= 100`
guard. A versioned supplemental metadata correction retains its original fixture
and results; all M00–M02 data, thresholds and execution verdicts are unchanged. M04–M05 remain
unimplemented. Discovery, execution acceptance and proof authority stay separate;
no discharge, drain, request generation, detector or UI integration is included.


### M04 implementation record — 2026-09-10

The author authorized M04 after PR #97. Explicit action alternatives and supplied
omission replay are implemented under the unchanged M03 dependency, three-day
ceiling, `ergo-sandbox` package, `mapping_actions` target and three registered test
names. Only M04's implementation flag and the completed prefix advance. See the
[M04 report](mapping/M04.md) for exact inventory accounting, the supplemental
metadata clarification and real gates. M05 remains unimplemented; its four
positive Execute action expectations remain visible and deferred. No pinned
membership, expected facts, established claims, numeric baselines or freezes change.

M05 author-authorized attempt stopped on 2026-09-10 under the mapping design's
`coverage-gate` rule. The registered `context_scope-positive` code requires
local execution but does not authenticate a fixed code digest; full validation
accepts two different extension programs under identical premises. See the
[M05 report](mapping/M05.md), [design amendment](superpowers/specs/2026-09-10-comprehensive-mapping-design.md#m05-attempted-implementation-amendment--2026-09-10-stopped)
and [stop record](roadmap-stops.json). M05 remains unimplemented and
`completedThrough` remains M04. No policy registration, pinned answer, historical
claim, action disposition, numeric threshold or frozen capability changes.
The separate counterexample diagnostic grants no M05 capability authority.
