# ergo-forge — governing roadmap

**Date convention:** Historical dates below are local (Australia/Brisbane, UTC+10).
The discovery date 2026-09-11 corresponds to authoring on 2026-09-10 UTC.
See the [repository record date convention](DATE-CONVENTION.md); new records use UTC.

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

P03 now provides node-accepted execution against supplied premises through the library API. P05 replay now produces node-validated property claims from accepted bundles, and P06 promotes search candidates through that path. Claims remain conditional on supplied premises and supported property semantics; transaction acceptance alone is not a property claim.

## 2. Governing queue — roadmap v2

The [roadmap v2 spec](superpowers/specs/2026-09-13-forge-roadmap-v2.md) now
supplies the governing queue and bounded changes to the old freeze decisions.
Its section 2 batch order governs implementation; section 5 supplies the new
registrations in the separate v2 policy below. The [v1 queue archive](reports/ROADMAP-v1-queue.md)
preserves the old ledger and P/M text. Later completion records and baseline
scoreboard prose in this file remain historical records, not a second queue.

W00 adopts the plan at `11d9a1c0986c5943d5b421e42271e1277d23528c`.
The scoreboard re-anchors artifact identities at that commit without rerunning
measurements; its original measurement revision and hashes remain recorded.
The completed prefix stays M04 and M05's recorded stop stays in force.

The v1 policy block remains byte-identical to main.

Runner amendment: `scripts` targets use standard-library unittest discovery
and verbose execution, with required names checked in both. In `--ci`, future
unimplemented v2 units are reported as `not-started` and do not fail CI; they never
count as completed. Explicit `--require` and `--all-goals` remain strict.
Validated stops still take precedence, and missing evidence or failed gates
still fail CI. This supersedes section 4's earlier CI treatment of all future
registrations as failures. The policy schema remains version 1.

## 4. Gates a harness can read

D00 scheduling decision (2026-09-11): register only D00, retaining the complete
proposed D00–D04 queue in discovery section 5.3. CI cannot enforce gates for
unregistered units; this is the accepted scheduling cost. Its acceptance rules
are unchanged. The [governing decision](discovery/D00-DECISION.md) resolves the
historical D00 registration stop while preserving both blockers and evidence.
M00 retains its original manifest bytes and digest. Its anchor projects onto
original P-era units **and original resolution entries**; a separately pinned
original policy authenticates every original P/M registration and P05 resolution.
Only the exact D00 registration, corresponding completion advancement and exact
D00 resolution are permitted additions. All other protected changes are rejected
by executable mutation regressions. Later registrations require a new decision.


The following JSON block is the **policy source**, not illustrative pseudocode. P00 adds a small standard-library Python runner that extracts the uniquely marked block from this file. Do not duplicate its thresholds in test code. Tests load the same block and the fixture manifests; the runner validates schema, verifies required test names are discovered, rejects ignored/zero-test runs, runs dependencies, and exits nonzero on any unmet requirement. New paths and test names in this section are planned deliverables, not files claimed to exist today.

<!-- roadmap-policy:v1 -->
```json
{
  "schemaVersion": 1,
  "baselineRev": "ee4ac6a874531872c27828d94e21e4fe7a1d7f7c",
  "completedThrough": "D02",
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
      "implemented": true
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
      "implemented": true
    },
    {
      "id": "M00",
      "depends": [
        "P08"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "mapping_inventory",
      "tests": [
        "inventory_has_32_pinned_cases_and_independent_answers",
        "legacy_metrics_are_measured_without_baseline_changes",
        "fixture_constructs_execute_and_unsupported_members_remain"
      ],
      "implemented": true
    },
    {
      "id": "M01",
      "depends": [
        "M00"
      ],
      "days": 2,
      "package": "ergo-sandbox",
      "target": "mapping_artifacts",
      "tests": [
        "discovery_cannot_import_as_required_execution",
        "premise_changes_invalidate_imported_proofs",
        "raw_provenance_and_legacy_map_bytes_survive"
      ],
      "implemented": true
    },
    {
      "id": "M02",
      "depends": [
        "M01"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "mapping_discovery",
      "tests": [
        "supported_reference_recall_and_all_site_accounting",
        "false_hints_never_become_required_relations",
        "caps_missing_pages_and_computed_identities_stay_unresolved"
      ],
      "implemented": true
    },
    {
      "id": "M03",
      "depends": [
        "M02"
      ],
      "days": 4,
      "package": "ergo-sandbox",
      "target": "mapping_necessity",
      "tests": [
        "exact_guarded_necessity_matches_pinned_answers",
        "alternatives_dead_checks_and_self_do_not_prove_cospend",
        "satisfying_omission_and_relaxed_controls_use_full_validator",
        "unsupported_anchors_and_cyclic_proofs_are_rejected"
      ],
      "implemented": true
    },
    {
      "id": "M04",
      "depends": [
        "M03"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "mapping_actions",
      "tests": [
        "alternative_action_sets_are_checked_separately",
        "data_outputs_and_unrelated_inputs_cannot_satisfy_spend",
        "accepted_omission_refutes_only_matching_claim_and_premises"
      ],
      "implemented": true
    },
    {
      "id": "M05",
      "depends": [
        "M04"
      ],
      "days": 4,
      "package": "ergo-sandbox",
      "target": "mapping_context_code",
      "tests": [
        "authenticated_config_code_requires_same_input_execution",
        "optional_wrong_scope_and_mutable_config_do_not_promote",
        "all_four_omission_families_replay_and_refute",
        "final_metrics_preserve_all_members_and_unsupported_ceiling"
      ],
      "implemented": false
    },
    {
      "id": "D00",
      "depends": [
        "M04"
      ],
      "days": 2,
      "package": "ergo-sandbox",
      "target": "property_inventory",
      "tests": [
        "property_inventory_pins_24_cases_and_independent_answers",
        "reference_executions_and_legacy_results_are_reproduced",
        "transfer_registration_and_exposure_are_accounted"
      ],
      "implemented": true
    },
    {
      "id": "D01",
      "depends": [
        "D00"
      ],
      "days": 2,
      "package": "ergo-sandbox",
      "target": "property_schema",
      "tests": [
        "property_versions_units_and_limits_fail_closed",
        "bindings_never_infer_missing_roles_or_authority",
        "declaration_identity_binds_all_semantic_premises"
      ],
      "implemented": true
    },
    {
      "id": "D02",
      "depends": [
        "D01"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "property_evaluation",
      "tests": [
        "four_property_families_match_independent_operands",
        "guards_missing_fields_and_overflow_preserve_unknowns",
        "bounded_response_requires_a_complete_accepted_linked_trace",
        "property_evaluation_requires_fresh_node_acceptance"
      ],
      "implemented": true
    },
    {
      "id": "D03",
      "depends": [
        "D02"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "property_replay",
      "tests": [
        "all_registered_property_dispositions_match",
        "property_replay_rejects_tampered_claims_and_premises",
        "legacy_replay_bytes_and_semantics_remain_unchanged",
        "registered_author_cases_show_usefulness_beyond_extraction"
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
    },
    {
      "unit": "D00",
      "recordSha256": "026d54998eb717de27922d8b10b4268fa07a24e49aa2b79466cd4106945280f3",
      "decision": "docs/discovery/D00-DECISION.md",
      "decisionSha256": "9293e9a6f50a11fa9d5b07c89dc2b2b8bbba9a027e8657830882ec4bd4609f8e"
    }
  ]
}
```
<!-- /roadmap-policy:v1 -->

P00 implementation amendment: result provenance is descriptive legacy metadata, not P01's case schema. The UI gate uses the production renderer against a DOM, with the exact dev dependency locked in `ui/package-lock.json`. The record-only command is `python3 scripts/roadmap_gate.py --scoreboard-only`; reports default to `target-p00/roadmap-gates/` and include command output and artifact hashes. The explicit `implemented` flag separates planned product units from missing required gates; it never passes a planned unit. These clarify the original gate specification without reducing any threshold.

The runner's gate command is `python3 scripts/roadmap_gate.py --require P05` (substitute the unit ID). It runs each registered target via `cargo test --release -p <package> --test <target> -- --show-output`, after checking the required test names through `--list`. Whole targets run, not filters that might silently select nothing. Install the pinned DOM-test dependency with `npm ci --prefix ui`. P00 additionally runs `python3 -m unittest discover -s scripts -p 'test_roadmap_gate.py'` and `node --test ui/tests/claim-labels.test.js`; P08 runs `node --test ui/tests/evidence-replay.test.js`. Those extra commands are fixed runner requirements with self-tests, not optional notes.

`python3 scripts/roadmap_gate.py --all-goals` is the strict completion report: it prints every goal and exits nonzero for anything other than `passed`, including a recorded stop. With M05 stopped, its exit code is expected to be 1. Reports distinguish `missing-gate` (required infrastructure/evidence absent), `failed` (executed gate failed), `passed`, `stopped` (a validated open stop record), `blocked` (a dependency did not pass), and `unimplemented` (unfinished work without a recorded stop). Only `passed` counts as completion.

**CI governance decision (PR #98):** CI runs the completed prefix and required landing gate, plus a separate, always-visible `--ci` full-goal evaluation. This mode runs the same complete selection, evidence validation and tests as `--all-goals`, but accepts only `passed` and validated `stopped` results. It records `ciAccepted` separately from `passed`; a stopped goal is never a completed goal. Any `unimplemented`, `missing-gate`, `failed`, or `blocked` result makes CI red. A valid recorded stop takes precedence over an unimplemented flag: explicitly governed stopped work is distinct from silently unfinished work. Removing M05's stop would expose its unchanged unimplemented flag and fail CI, so deleting a stop cannot buy a green build. Malformed or missing stop evidence fails closed. The full status list remains in CI output even if an earlier step fails; no blanket `continue-on-error` or exit-code suppression is used.

`completedThrough` advances only in the PR whose gates pass, with all predecessors rerun. A stopped unit blocks downstream dependent units until a governing-plan amendment removes them or changes the dependency with a new acceptance gate, or a policy-pinned governing decision resolves that exact stop with recovered evidence and reopens the original gate. Reopening does not award a pass: the original unit and predecessor tests must execute successfully. The stop record and its original evidence remain history.

Gate command output is normalized to `<repo>`, `<home>`, and `<tmp>` before printing, recording and computing transcript hashes. For standalone Cargo transcripts, use `python3 scripts/capture_gate.py --output PATH -- COMMAND ...`, which applies the same normalization and preserves the command's exit code. Historical transcript sanitization changes transcript hashes only; fixture and measurement hashes remain unchanged.


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

P07 manifest clarification: `tests/fixtures/evidence/precision.json` is the
separate P07 fixture manifest and frozen label registry. Like P04/P05, it does
not append rows to P03's exact node-vector inventory in `manifest.json`. Its
eight entries retain case IDs, families, source kinds, file hashes, node and
property revisions, expected acceptance/claim status, and publication eligibility;
the duplicate fixture is separately hash-pinned there. The P07 tests derive
results from execution/replay and compare the frozen labels. This changes no
acceptance case, threshold, baseline row or required test.


P08 completion record: see [P08-REPORT.md](P08-REPORT.md). Read now replays saved
P05 evidence through `/api/v2/replay`, returning the unchanged CLI report inside
an API-version-2 envelope. Its tests reuse the policy-pinned P05 `use-incident`
and `sale-fixed-paid` bundles and derive an explicitly incomplete copy by removing
required premises; no existing fixture or manifest row changes. The policy changes
only P08's implementation flag and `completedThrough`. Earlier “today” and
“unimplemented” statements above describe their recorded implementation stages.
The finite queue ends at P08; further work requires a new governing decision.


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

D00 author-authorized registration attempt stopped on 2026-09-11. The exact
proposed D00–D04 append makes the current all-goal CI reject unfinished D units
and fails M00's frozen-policy projection. See the [D00 report](discovery/D00.md),
[design amendment](superpowers/specs/2026-09-11-discovery-capability-design.md#d00-attempted-registration-amendment--2026-09-11-stopped)
and appended [stop record](roadmap-stops.json). The temporary diagnostic append
was restored; no D units are registered or implemented in the governing policy.
The completed prefix remains M04, all P/M flags and baseline fields remain
unchanged, and M05's stop remains. Reopening requires an explicit registration/CI
scheduling decision and M00 projection integration preserving the original
digest, followed by the original D00 inventory and gates. This records a
registration boundary, not a property measurement or a passing D00 gate.


D00 author-authorized incremental implementation record — 2026-09-11:
D00 alone is registered and implemented following its three direct inventory
gates. `completedThrough` advances to D00, whose dependency is M04; M05 remains
stopped and is not a D00 dependency. The [D00 record](discovery/D00.md) reports
final prefix and CI verification. D01–D04 remain proposed and unregistered, so
CI cannot enforce their gates. No later capability is implemented. The exact
policy-pinned D00 resolution retains both historical registration blockers and
all evidence; the [historical report](discovery/D00-STOP.md) remains unchanged.
M00's original policy digest and manifest bytes survive the narrowly authorized
projection extension. All original P/M registrations and P05 resolution are
separately authenticated, and real mutation checks reject other protected changes.
The synthetic inventory is complete; absent independent semantic review and a
transfer pair mean incomplete semantic measurement and no utility claim.


D01 author-authorized implementation record — 2026-09-11:
D01 alone is added after the committed D00 inventory under section 4's
incremental registration rule. Its exact proposed dependency, two-day ceiling,
package, target and three test names are retained. The strict opt-in declaration
schema, normalized immutable identity and explicit role binding checks confer no
execution or property-result authority. Only D01's flag and `completedThrough`
advance, after its direct tests and all predecessors passed. See the
[D01 report](discovery/D01.md) for final gates and preservation evidence.
D00's 24 rows, hashes and null transfer registration are unchanged. M00's original
anchor remains unchanged; exact permitted-additions authentication now covers
D01 while rejecting prior-field mutations and later-unit appends. M05 remains
stopped. D02–D04 are unregistered; capability 1 can remain the endpoint and no
capability 2/3 work is planned by this change.


D02 author-authorized implementation record — 2026-09-11:
D02 alone is registered with the unchanged D01 dependency, three-day ceiling,
package, target and four test names. Fresh per-step node validation now supports
checked supplied-trace linkage and the four bounded property families, including
response horizons 1–4. Only D02's implementation flag and `completedThrough`
advance. See the [D02 report](discovery/D02.md) for authority limits, adversarial
checks and real gate output. The D00/D01 pins, schema, M00 anchor, frozen data
and M05 stop remain unchanged. Exact permitted-additions authentication covers
D02 and retains earlier mutation/append rejections. D03/D04 remain unregistered;
capability 1 may remain the endpoint, with no capability 2/3 work planned here.


D03 author-authorized attempted implementation — 2026-09-11:
D03 alone is registered with the exact proposed D02 dependency, three-day ceiling,
package, target and four tests. The first required run measured 24/24 synthetic
dispositions and 16/16 applicable supported guards, but failed author usefulness:
the frozen transfer denominator is null. D03 is stopped and unimplemented;
`completedThrough` remains D02. The full failed target and command report are
retained, with a separate passing replay correctness prefix. See
[the D03 report](discovery/D03.md). This does not waive or pass its utility gate.
D04 remains unregistered and unattempted. Capabilities 2 and 3 are not scheduled.


## Roadmap v2 policy

The v1 block above is frozen. This separate block registers the twenty new
units; dependencies resolve against v1 followed by v2. Each block retains its
own completed prefix. Only v2 units can be reported as `not-started` by CI.

<!-- roadmap-policy:v2 -->
```json
{
  "schemaVersion": 1,
  "baselineRev": "11d9a1c0986c5943d5b421e42271e1277d23528c",
  "completedThrough": null,
  "maxActiveImplementationBranches": 1,
  "maxOpenImplementationPrs": 1,
  "units": [
    {
      "id": "W00",
      "depends": [],
      "days": 1,
      "package": "scripts",
      "target": "test_roadmap_gate",
      "tests": [
        "policy_v2_block_parses",
        "scoreboard_reads_every_baseline_artifact"
      ],
      "implemented": true
    },
    {
      "id": "S00",
      "depends": [
        "W00"
      ],
      "days": 2,
      "package": "ergo-sandbox",
      "target": "vector_catalogue",
      "tests": [
        "every_vector_has_examples_or_is_marked_manual",
        "every_named_instrument_exists"
      ],
      "implemented": true
    },
    {
      "id": "W01",
      "depends": [
        "W00"
      ],
      "days": 5,
      "package": "ergo-sandbox",
      "target": "attack",
      "tests": [
        "play_attacker_reorder_reproduces_use_drain",
        "decoy_box_satisfies_every_positional_access",
        "attacker_results_are_synthetic_and_unvalidated"
      ],
      "implemented": true
    },
    {
      "id": "S02",
      "depends": [
        "S00"
      ],
      "days": 4,
      "package": "ergo-sandbox",
      "target": "mutation_corpus",
      "tests": [
        "new_lint_mutants_are_caught_and_controls_are_clean",
        "deployed_corpus_sweep_is_recorded"
      ],
      "implemented": false
    },
    {
      "id": "S01",
      "depends": [
        "S00",
        "S02"
      ],
      "days": 3,
      "package": "ergo-web",
      "target": "checklist",
      "tests": [
        "checklist_defaults_to_unchecked",
        "checklist_answer_carries_provenance",
        "checklist_never_emits_a_score"
      ],
      "implemented": false
    },
    {
      "id": "W06",
      "depends": [
        "S01"
      ],
      "days": 2,
      "package": "ergo-web",
      "target": "negative_space",
      "tests": [
        "negative_space_lines_have_anchors",
        "negative_space_is_labelled_static"
      ],
      "implemented": false
    },
    {
      "id": "I01",
      "depends": [
        "W00"
      ],
      "days": 2,
      "package": "ergo-sandbox",
      "target": "verify_deployment",
      "tests": [
        "verify_exact_template_and_mismatch_are_distinct",
        "verify_lists_differing_constants"
      ],
      "implemented": false
    },
    {
      "id": "I02",
      "depends": [
        "I01"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "lockfile",
      "tests": [
        "lockfile_detects_source_param_and_engine_drift",
        "lockfile_matches_live_box_bytes"
      ],
      "implemented": false
    },
    {
      "id": "W02",
      "depends": [
        "W01"
      ],
      "days": 2,
      "package": "ergo-web",
      "target": "play_export",
      "tests": [
        "play_export_roundtrips_through_cli_test_runner",
        "exported_case_names_only_the_suite_contract"
      ],
      "implemented": false
    },
    {
      "id": "W03",
      "depends": [
        "W01"
      ],
      "days": 1,
      "package": "ergo-web",
      "target": "share_links",
      "tests": [
        "share_link_roundtrips_play_state",
        "share_link_over_cap_is_refused_with_size"
      ],
      "implemented": false
    },
    {
      "id": "W04",
      "depends": [
        "W00"
      ],
      "days": 3,
      "package": "ergo-web",
      "target": "write_cost_explain",
      "tests": [
        "hot_spots_map_to_source_spans",
        "explain_subexpression_matches_full_reduction",
        "cost_trace_route_is_behind_feature_flag"
      ],
      "implemented": false
    },
    {
      "id": "S04",
      "depends": [
        "S00"
      ],
      "days": 2,
      "package": "ergo-sandbox",
      "target": "immobilisation",
      "tests": [
        "immobilisation_probes_report_unspendable_not_safe",
        "storage_rent_line_is_static"
      ],
      "implemented": false
    },
    {
      "id": "I04",
      "depends": [
        "S02"
      ],
      "days": 1,
      "package": "ergo-sandbox",
      "target": "mutation_corpus",
      "tests": [
        "upgrade_hook_mutant_is_caught"
      ],
      "implemented": false
    },
    {
      "id": "I03",
      "depends": [
        "I02"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "watch",
      "tests": [
        "watch_reports_script_change_under_nft",
        "watch_never_broadcasts"
      ],
      "implemented": false
    },
    {
      "id": "W05",
      "depends": [
        "S02"
      ],
      "days": 2,
      "package": "ergo-sandbox",
      "target": "compose_recipes",
      "tests": [
        "new_recipes_have_independent_expectations",
        "new_recipe_mutants_are_caught"
      ],
      "implemented": false
    },
    {
      "id": "S05",
      "depends": [
        "W00"
      ],
      "days": 2,
      "package": "ergo-sandbox",
      "target": "incident_scaffold",
      "tests": [
        "incident_scaffold_reproduces_use_boxes",
        "incident_scaffold_never_fills_expectations"
      ],
      "implemented": false
    },
    {
      "id": "X01",
      "depends": [
        "W00"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "node_validation",
      "tests": [
        "node_vectors_pass_on_new_rev"
      ],
      "implemented": false
    },
    {
      "id": "S03",
      "depends": [
        "S02",
        "X01"
      ],
      "days": 3,
      "package": "ergo-sandbox",
      "target": "drain_promotion",
      "tests": [
        "two_instance_mutant_is_found_and_control_is_not",
        "caps_and_truncation_are_recorded"
      ],
      "implemented": false
    },
    {
      "id": "X02",
      "depends": [
        "X01"
      ],
      "days": 1,
      "package": "scripts",
      "target": "test_release",
      "tests": [
        "release_action_points_at_tagged_binary",
        "changelog_lists_every_batch"
      ],
      "implemented": false
    },
    {
      "id": "X03",
      "depends": [
        "W00"
      ],
      "days": 1,
      "package": "scripts",
      "target": "test_ci_workflow",
      "tests": [
        "node_checkout_is_cached",
        "cost_trace_runs_in_its_own_job"
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
<!-- /roadmap-policy:v2 -->
