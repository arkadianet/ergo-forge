# Comprehensive protocol mapping — design record, 2026-09-10

**Answer to the parked question:** use one guarded necessity relation, `Requires(A, guard, Spend(selector))`, backed by exact code and a checked derivation. It means that every node-accepted transaction spending A, within the recorded premises and satisfying the guard, also spends a *distinct* box matching the selector. A selector names a predicate, not whichever discovered box looks related. An accepted transaction satisfying the same premises and guard while omitting every matching companion refutes the relation. Two supporting relations, `Authenticates(subject, field, expected)` and `Requires(A, guard, Execute(extension))`, permit narrowly checked code delegation. Discovery edges imply none of these relations. “Every spend” requires guard `true`; a branch-specific requirement must retain its guard.

**The strongest argument against this approach:** the derivation checker can become a second, unsound interpretation of ErgoScript. A finite corpus cannot prove that its inference rules cover every acceptance path, and making it broad enough for computed identities and delegation could consume the project without producing a trustworthy protocol model. The proposed response is a small typed fragment, explicit refusal outside it, independent accepted omission witnesses, and a hard ceiling. If exact semantic correspondence cannot be established within that ceiling, ship the discovery inventory and supplied-transaction checks, leaving universal co-spend unresolved.

Status: **proposal only; no implementation authorized by this record.** Inspected local HEAD `168687a92a16a7ca28683caa07c798ba10396b26` (PR #96 merged per author). P00–P08 are implemented in the governing policy; the author reports `python3 scripts/roadmap_gate.py --all-goals` exits 0. This pass does not rerun product benchmarks or change their measurements. The only new file is this record, left uncommitted.

## 1. Decision and scope

“Absolutely comprehensive” means every member of a declared inventory receives an accountable disposition, with separately measured recovery and supported reasoning. It does **not** mean a complete model of all deployed protocols. There is no independently known denominator for the latter today. A report must distinguish accounted-for sites, recovered relationships, necessary co-spends, validated executions, and unknown protocol membership.

The governing [ROADMAP](../../ROADMAP.md), especially sections 2 and 6 and its unplanned appendix, remains authoritative. This record answers its parked question and proposes a replacement queue for a later governing decision. The [architecture review, section 4](../../ARCHITECTURE-REVIEW-CODEX.md#4-coverage-what-is-the-tool-structurally-blind-to), distinguishes discovery, membership, identity and execution. The historical [Phase B](2026-09-08-drain-hunt-roadmap.md) is not authorization to import its automatic private-case discharge or mutation-corpus targets.

| Gap | Decision for this proposed queue | Deliberate boundary |
|---|---|---|
| Dynamic references | Recover literal-identity `INPUTS.exists` predicates and resolve them within a declared finite snapshot; record other sites | General computed indices, arbitrary searches and data-flow remain unresolved, even when replay executes them |
| Input-set membership | Check explicitly supplied alternative action sets against guarded requirements | No membership search, action inference, map-to-transaction construction or new drain axis |
| Required co-spend | Check direct necessity, distinct companion identity and exact companion code under explicit premises | No adjacency-to-execution conversion, universal safety or unrestricted transitive closure |
| Script from context | One authenticated config-register-to-extension-hash idiom with mandatory execution | No arbitrary dispatch, nested extensions, external code fetching or authentication-chain engine |
| Broader identity | Represent exact box IDs, exact script bytes/hashes, token predicates at literal positions and existential token predicates; allow tokenless boxes | Fungible tokens identify a set, not a unique actor. General register/AVL/computed identities are inventoried but not proved |
| Completeness | Versioned corpus, negative edges, denominator and snapshot/source/cap disclosures | No verified historical UTXO service, whole-chain coverage, future-state or lifecycle guarantee |

This is a library/artifact proposal. It buys protocol relationship experiments without a graph dashboard, UI migration, detector changes or automatic finding suppression.

## 2. Current implementation and compatibility boundary

`ergo-sandbox/src/map/mod.rs` has box nodes, proposed roles, discovery edges, edge-attributed `SetFinding`, `protocol_nfts`, traversal caps and unresolved references. `refs.rs` contains both broad `tree_refs` observations and the narrower `required_bindings` reader. The latter unions conjunction requirements, intersects alternatives and returns no facts for unknown forms/budget exhaustion. That is a useful starting discipline, not an exact-code proof API.

The implementation is slightly broader and narrower than the shorthand description: it follows script-hash references where the source supports them; its public `Seed` enum has address, token ID and transaction ID, while `ChainSource::box_by_id` is a lookup, not a box-ID seed variant. Neither detail warrants an implementation change in this pass.

`source.rs` is a query interface. Its height and recorded responses do not establish simultaneous unspentness. Legacy `ChainBox` defaults can collapse missing registers into empty registers; new evidence must enter through raw `RecordedBox`/canonical evidence before that loss. Do not manufacture creation references from map JSON.

`audit/context.rs` accepts `ContractSet.complete` and `Execution::ContextExtension` as caller premises. Its literal-slot discharges remain conditional. `audit/obligation.rs` groups observations for presentation; its `Obligation` is **not** the new necessity relation. Keep distinct types and version namespaces. `drain::request_from_map` includes every proposed protected/companion node and defaults successors only for protected nodes. It remains a frozen legacy proposal helper, unsuitable as evidence of a protocol action.

## 3. Vocabulary and semantics

### 3.1 Subjects, domains and premises

A spending subject A is either a canonical box ID or an exact serialized script with explicitly fixed SELF fields. A script-family claim with varying SELF fields is not supported initially. A selector is a typed predicate over a box: exact ID; exact proposition bytes; hash of proposition bytes; token ID/amount at a literal token position; or existential token membership. Conjunctions of these predicates are allowed. Value-at-risk, role and attacker capability are separate caller fields; none is inferred from identity or `protocol_nfts`.

Premise bundle P records exact root bytes, SELF bytes/reference, pinned node/compiler revision, network and activation rules, supplied state constraints, guard, authentication roots, provenance and analysis caps. Source-recorded, caller-supplied, hypothetical and independently checked remain distinct. No caller-supplied “B must execute” premise is admissible as a derivation of the same co-spend claim. Store the complete normalized P and its digest, not only a label.

Let `Accepted(P,T)` mean full pinned node validation accepts T under a state/context allowed by P. Then:

`Requires(A,g,Spend(s)) := forall T: Accepted(P,T) and A in T.inputs and g(P,T) => exists b in T.inputs: b.id != A.id and s(P,T,b)`.

A concrete box selector means that box, not any later holder of its NFT. A script selector means any matching instance, not uniqueness. A token selector alone establishes no code identity; singleton issuance evidence can establish uniqueness under its stated history premise but cannot establish future holder code or lifecycle. Data inputs and outputs never satisfy `Spend`. Multiple selectors may match the same companion unless explicit distinctness is supported; this first fragment only requires companion-versus-A distinctness and does not express cardinality constraints.

Guards initially comprise typed literal equalities and their Boolean combinations over explicitly named context/SELF fields. Preserve variable scopes and field types. If a guard cannot be represented and checked, retain an unresolved branch; do not erase it or upgrade branch necessity to `true`.

### 3.2 Three relations, no general protocol DSL

1. **`Requires(A,g,Spend(s))`**: mandatory distinct spending input as defined above. There is no `Related => Requires` rule.
2. **`Authenticates(subject,field,expected)`**: under the same guard/P, successful root evaluation forces exact equality of a selected identity/code field with the expected bytes or supported hash. Records the enforcing code/site and trusted source of `expected`. It proves that field only, not reserves, freshness or policy correctness.
3. **`Requires(A,g,Execute(input,var,codeDigest))`**: successful root evaluation necessarily executes the exact authenticated extension bytes at the named variable of the named spending input and requires its success. It is not a co-spend; extensions share that input's SELF. Reading bytes or checking their hash without mandatory execution is insufficient.

These form a directed, guarded obligation artifact whose proof dependencies must be acyclic. A cycle is not a proof of its own premises. No automatic transitive co-spend closure is in this queue. If a future consumer combines two necessities, it must prove intermediate selector equality, code identity, compatible guards and premises; graph reachability cannot do this.

No relation expresses general value conservation policy, authorized insider behavior, fairness, liveness, reachable-state invariants, arbitrary authentication chains, arbitrary arithmetic, all possible actions, or safety. Existing property claims retain their versioned semantics and full validator boundary.

### 3.3 Establishment, omission and refutation

The extractor proposes a derivation; a separately callable checker checks exact tree anchors, types, scopes, rule applications and premises. Do not certify recovered source merely because it recompiles. The checker must use pinned typed tree semantics or a verified exact mapping to those nodes. Unsupported lift nodes, ambiguous aliases, truncation or unmapped anchors block the affected proof. This is a structural checker, not a replacement evaluator or a solver.

Initial rules cover mandatory positive equality checks in conjunctions, common requirements across alternatives, literal guards and a literal-identity existential over INPUTS. Unused definitions, optional/negated checks, caught/defaulted lookups and unknown branches contribute no unconditional necessity. One cannot prove that a branch is impossible from unsuccessful sampling. Distinctness must be explicit in the predicate or follow from fixed SELF failing the selector; an existential satisfied by SELF is not a companion requirement.

An action-set checker may report `violates-declared-requirement` when a supplied set omits a matching input under a checked true guard. That rejects the *set as satisfying the claim*, not the transaction through consensus. A full node rejection is separate evidence and must identify the failing stage; a balance or missing-proof failure is not evidence of required co-spend.

For refutation, accept a canonical bundle with A spent, identical P, guard true, full node acceptance and no distinct matching spending input. Bind the result to the claim digest. This yields `refuted`, even if the checker previously said `established-under-premises`; such a conflict blocks the unit and preserves both artifacts. It is not automatically a vulnerability. The witness may be supplied manually; no new search is promised.

Testing a rejection does not prove universality. Every supported positive rule needs (a) an accepted satisfying spend to avoid a merely vacuous demonstration, (b) a well-formed omission case rejected at the relevant script, and (c) a relaxed/alternative code control with a fully accepted omission. The control refutes a separately stated claim about its own bytes, never the original by silently changing P. Arbitrary external claims can remain `unresolved`; tested examples never become universal certificates by enumeration.

### 3.4 Bounded context-code resolution

The sole proposed dispatch pattern is: A necessarily authenticates a literal config slot by exact ID or script; a typed config register supplies a code hash; A necessarily checks that hash against the extension bytes for its own input/variable; A necessarily executes those same bytes with success required. Record each equality, register type, execution site and guard. A data-input config supplies authenticated data but its script does not execute.

Exact config script identity alone does not make every register trustworthy. Either P fixes a canonical config box and its register bytes (a supplied-state conditional claim), or a separate established invariant would be needed. The latter is outside scope. Mutable configuration therefore cannot yield an all-future-states assertion. Missing bytes, wrong types, hash mismatch, unauthenticated config, another input's variable, optional execution and recursive dispatch remain unresolved/refused. Reuse the pinned parser and validator; no source rewriting or second interpreter.

## 4. Separate artifacts and authority

| Artifact | Contents | Authority/status |
|---|---|---|
| `discovery-map:v2` | Seeds, canonical box references where available, observed code, typed reference sites, proposed targets/roles, source queries, snapshot scope, caps and unresolved reasons | Per edge: `hint`, `observed-match`, `rejected-match`, `unresolved`; no proof state |
| `required-relations:v1` | Claims, P, exact anchors, typed selectors/guards, checked derivations, authenticated bytes and dependency IDs | `unresolved`, `established-under-premises`, `refuted`; method is static derivation, never node-validated by implication |
| `action-check:v1` | Explicit caller-named spending inputs, data inputs, extensions and guard values; relation IDs; missing/ambiguous bindings | `satisfies-declared-requirements`, `violates-declared-requirement`, `unresolved`; satisfaction is not transaction acceptance |
| Existing canonical execution evidence, referenced by digest | Full transaction/context and accepted/rejected validation; optional omission assessment tied to claim | Acceptance comes only from the existing pinned validator; observed execution is not universal necessity |

Separate serialized envelopes and Rust types; no `From<DiscoveryMap>` into a proof or complete action set. An explicit nomination API accepts discovery IDs only as candidates, then resolves independent evidence. Import rechecks proof material rather than trusting serialized status. Unknown versions fail closed. Missing code/state prevents promotion while keeping observations readable. Premise/bytes/guard/revision changes invalidate proof caches and witness comparisons.

Confidence is categorical and orthogonal: discovery match confidence, code identity, state provenance, proof status and execution status. No shared percentage or inherited green badge. A correctly hashed code blob may have unknown provenance; a source-recorded box may have unknown historical availability. Record both. Keep legacy map JSON and existing conditional discharge APIs intact. No new relation automatically suppresses a lint in this queue.

Alternative actions are explicit lists supplied by the experiment author. Check each separately; do not union their companions. Unknown guards produce unresolved checks, not inferred membership. No success implies the action list is exhaustive or minimal. Tokenless boxes can participate in these experiments without a fake NFT; the frozen drain objective still has its existing NFT restrictions.

## 5. Measurable comprehensiveness

### 5.1 Denominator registration before extractor work

M00 must register `ergo-sandbox/tests/fixtures/mapping/manifest.json` and independent `expected.json`. Use independently authored, publishable synthetic fixtures, plus separately labelled public observations where sufficient material already exists. Do not import confidential worktrees. Pin source/tree/canonical fixture hashes, node revision, source provenance, publication status, case IDs, reference sites, true/false relationship tuples, permitted premises, action sets and expected unsupported reasons. Two existing map fixtures are regression material, not known-complete protocols.

Register **16 paired families (32 cases)**, positive and adversarial/unsupported member in each. IDs and intended distinctions are fixed here:

| Family ID | Positive / paired distinction |
|---|---|
| `literal_spend` | Mandatory companion / optional branch accepts omission |
| `self_alias` | Distinct companion / SELF alone satisfies existential |
| `action_alternatives` | Guarded B action / C action legitimately omits B |
| `data_only` | Actual co-spend / same identity only in data inputs |
| `output_only` | Actual co-spend / matching output is not an input |
| `dead_check` | Root-required equality / unused or negated equality |
| `script_identity` | Exact authenticated code / structurally similar different constants |
| `token_identity` | Literal nonzero-slot identity / fungible duplicate is not unique |
| `tokenless` | Exact tokenless escrow identity / same-value unrelated escrow |
| `exists_literal` | Distinct literal-identity search / optional search accepts omission |
| `computed_index` | Supplied index resolves in an explicit transaction / general index remains unresolved |
| `register_identity` | Fixed canonical config register / unconstrained mutable register |
| `context_code` | Hash-authenticated mandatory extension / hash checked but execution optional |
| `context_scope` | Correct input-local variable / same variable ID on another input |
| `avl_identity` | Supplied replay material / static credential relation unsupported |
| `snapshot_collision` | Closed synthetic inventory / coincidental constant plus capped or inconsistent source inventory |

Each family has explicit sub-vectors for its listed variants; adding variants cannot replace either member. Fixtures test claims about their own exact code. Unsupported members are part of the denominator, not failures to be deleted. M00 must demonstrate each intended construct is expressible through the pinned engine or record a stop; do not manufacture a green baseline by changing the family. Canonical accepted omission controls are mandatory for `literal_spend`, `action_alternatives`, `exists_literal` and `context_code` before their corresponding capability gate passes.

Assign `-positive` and `-control` case IDs deterministically. Hand-author the answer key from exact code semantics and explicit synthetic universe construction before running the new extractor. The finite universe includes all intentionally unrelated boxes; expected true/false edges must not be derived from the mapper's output. Hash-pin it at M00 landing. Any later correction is a versioned denominator change retaining old results, not a quiet answer-key edit. This is independently reproducible fixture coverage, not independent human adjudication of real protocols.

### 5.2 Metrics and honest baseline

| Metric | Checkable numerator / denominator | Current value at this design |
|---|---|---|
| Site accountability | Expected reference sites with resolved or explicit unresolved disposition / all independently inventoried sites | `null`; manifest not built. Target 100% across all 32 cases |
| Closed-fixture reachability recall | Correctly recovered expected box members from registered seeds / known members reachable by expected relations in finite synthetic universes | `null`; no registered denominator yet. M02 target 100% for declared supported cases only; publish all-case recall too |
| Dynamic relation recovery | Correct recovered dynamic relationship tuples / all expected dynamic tuples, split by family and supported/unsupported | `null`; legacy dynamic sites are unresolved, but a numeric corpus rate has not been measured |
| False proposal rate | Adjudicated false candidate edges / all emitted candidate edges; also publish unadjudicated count | `null`; do not claim zero from existing tests. No universal zero target for hints |
| Promoted false relation rate | Incorrect established claims / all established claims on the corpus | `null`; target 0 with a nonzero positive count; any accepted contradictory omission blocks release |
| Necessity recall and abstention | Correct established requirements / expected supported necessities; unresolved claims / all registered claims | `null`; M03/M05 require every registered supported positive and every mandated abstention, with all-family totals retained |
| Action-set accuracy | Correct satisfy/violate/unresolved dispositions / all registered action sets, including alternatives and omissions | `null`; target 100% on fixed inventory, never all real protocol actions |
| Omission refutation | Accepted, premise-matching omissions correctly refuting claims / all registered accepted omission witnesses | `null`; target 100%, at least the four named families by M05 |
| Whole deployed-protocol completeness | All relevant roles/actions/states recovered / independently known complete deployed inventory | `null`, denominator unavailable; no target or completion claim |

Counts are exact integers with member IDs; rates are `null` for zero/unknown denominators. Never score unresolved as recovered. Report caps, unsupported-source queries, missing pages, parse failures and unknown inventories alongside all metrics. “32/32 accounted for” could coexist with poor recovery and must never be titled “100% mapped.” No claim of real-world precision follows from synthetic fixtures.

M00 records the legacy baseline, without changing existing baselines. Subsequent unit tests recompute metrics and check equality to emitted `mapping-results.json`, with revision and input hashes. Required suites load the pinned manifest, reject duplicate/missing members, exercise real product APIs and compare independent expected results. Printed counts or existence-only tests do not pass. The runner does not interpret arbitrary new metric keys: Rust gate targets own these checks. Include result paths and hashes in test diagnostics, which the existing runner captures; do not pretend its existing `evidenceHashes` automatically indexes new artifacts.

## 6. Proposed PR queue and gates

One developer, one implementation branch and one review PR at a time. Proposed order M00 → M01 → M02 → M03 → M04 → M05, **19 working days maximum**, then stop. Each unit includes its manifest/result checks and a brief report under `docs/mapping/`; all paths below are proposed future touches, not files created by this design pass.

| Unit / ceiling | Dependencies | Proposed files touched | What it buys and acceptance meaning |
|---|---|---|---|
| M00 / 3 days | P08 | New `tests/mapping_inventory.rs`, `tests/fixtures/mapping/{manifest,expected,legacy-results}.json` and fixture files; `docs/mapping/M00.md`; governing roadmap additions | Pins 32 cases and hashes before extraction; rejects missing/duplicate/altered members; records actual legacy metrics, proves required engine constructs can execute and preserves unsupported cases |
| M01 / 2 days | M00 | New `src/map/{discovery,relations}.rs`, exports in `map/mod.rs`, `tests/mapping_artifacts.rs`; schema examples in the new fixture directory | Separate serializable artifacts; untrusted imported proof status rejected/rechecked; raw provenance survives; discovery cannot construct proof/action authority; old map snapshots remain byte-stable |
| M02 / 3 days | M01 | New `src/map/discover_refs.rs`, `discovery.rs`, `tests/mapping_discovery.rs`; new mapping result/report files | Literal and bounded existential discovery over finite supplied material, tokenless/script/token identities; computed/AVL cases explicit unresolved; exact supported recall and site accounting, false hints visible, cap/source inconsistency tests |
| M03 / 4 days | M02 | New `src/map/{necessity,check_relation}.rs`, `relations.rs`, `tests/mapping_necessity.rs`; independently authored canonical vectors in new fixture directory | Checks direct guarded co-spend and authentication rules, distinctness and exact anchors; all supported positives plus adversarial alternatives; accepted satisfying and omission/control executions; no discharge integration |
| M04 / 3 days | M03 | New `src/map/action.rs`, exports, `tests/mapping_actions.rs`; result/report files | Explicit action alternatives checked independently; data/output/SELF cannot satisfy companion need; existing validator validates supplied omission witnesses and digest-matched refutations; no request generation |
| M05 / 4 days | M04 | New `src/map/context_code.rs`, checker/relations additions, `tests/mapping_context_code.rs`; public-independent extension fixtures and final `docs/mapping/M05.md` | Single bounded config/hash/extension idiom, required execution and input-local scope; all corpus metrics emitted; mandatory omission controls replayed; unsupported ceiling published |

All Rust paths in the table are relative to `ergo-sandbox/`. Each unit may edit its own new report, manifest execution metadata and results; it may not silently edit pinned case membership/expected facts. Gate thresholds are fixed by section 5 and asserted by the named Rust tests. Registration in the governing policy happens only in a later authorized change. Existing `scripts/roadmap_gate.py`, old tests, old metrics and detector behavior need no edits.

### Proposed `roadmap-policy:v1` unit additions

The following JSON is an **append fragment for the existing `units` array**, not a replacement policy and not an active policy block. Preserve `baselineRev`, `thresholds`, `frozenPreflight`, `scoreboard`, stop resolutions, P00–P08 and `completedThrough: "P08"`. Initially every added unit is false; set each true only with its implementation and passing evidence, then advance the completed prefix. The unchanged runner will correctly report unimplemented goals until then.

```json
[
  {
    "id": "M00", "depends": ["P08"], "days": 3,
    "package": "ergo-sandbox", "target": "mapping_inventory",
    "tests": ["inventory_has_32_pinned_cases_and_independent_answers", "legacy_metrics_are_measured_without_baseline_changes", "fixture_constructs_execute_and_unsupported_members_remain"],
    "implemented": false
  },
  {
    "id": "M01", "depends": ["M00"], "days": 2,
    "package": "ergo-sandbox", "target": "mapping_artifacts",
    "tests": ["discovery_cannot_import_as_required_execution", "premise_changes_invalidate_imported_proofs", "raw_provenance_and_legacy_map_bytes_survive"],
    "implemented": false
  },
  {
    "id": "M02", "depends": ["M01"], "days": 3,
    "package": "ergo-sandbox", "target": "mapping_discovery",
    "tests": ["supported_reference_recall_and_all_site_accounting", "false_hints_never_become_required_relations", "caps_missing_pages_and_computed_identities_stay_unresolved"],
    "implemented": false
  },
  {
    "id": "M03", "depends": ["M02"], "days": 4,
    "package": "ergo-sandbox", "target": "mapping_necessity",
    "tests": ["exact_guarded_necessity_matches_pinned_answers", "alternatives_dead_checks_and_self_do_not_prove_cospend", "satisfying_omission_and_relaxed_controls_use_full_validator", "unsupported_anchors_and_cyclic_proofs_are_rejected"],
    "implemented": false
  },
  {
    "id": "M04", "depends": ["M03"], "days": 3,
    "package": "ergo-sandbox", "target": "mapping_actions",
    "tests": ["alternative_action_sets_are_checked_separately", "data_outputs_and_unrelated_inputs_cannot_satisfy_spend", "accepted_omission_refutes_only_matching_claim_and_premises"],
    "implemented": false
  },
  {
    "id": "M05", "depends": ["M04"], "days": 4,
    "package": "ergo-sandbox", "target": "mapping_context_code",
    "tests": ["authenticated_config_code_requires_same_input_execution", "optional_wrong_scope_and_mutable_config_do_not_promote", "all_four_omission_families_replay_and_refute", "final_metrics_preserve_all_members_and_unsupported_ceiling"],
    "implemented": false
  }
]
```

After adoption, per-unit commands are `python3 scripts/roadmap_gate.py --require M00` through `--require M05` (substitute the corresponding ID). `--require Mxx` includes its entire dependency prefix. Each target is a normal Rust integration test at `ergo-sandbox/tests/<target>.rs`; the runner discovers required names, runs the whole target and rejects absent, zero, failed or ignored tests. No new command adapter, feature flag, network service or custom Python gate is required. `--all-goals` must stay nonzero while any registered unit is unimplemented/stopped; `--through-completed` verifies only the completed prefix. No stub tests may stand in for absent behavior.

## 7. Stop rules — retain the useful prefix

Use the existing section 6 stop-record schema in `docs/roadmap-stops.json`: `unitOrProposal`, `reasonCode`, actual 40-character `attemptCommit`, `daysSpent`, `gateResults` (command array, exit code, evidence path/hash), `retainedCapability`, `unsupportedClass`, `reopenEvidenceRequired`. This design creates no stop record for work not attempted. A future stop preserves failed evidence and blocks dependents; it is not a passing capability gate.

| Trigger | Required decision | Honest retained boundary and reopening evidence |
|---|---|---|
| Any unit reaches its days ceiling or total reaches 19 days without all gates | Record `timebox`; stop at last independently passing prefix | Name unmet tests and cases; reopen only with a smaller replacement unit and explicit displaced work |
| M00 cannot supply independent, publishable fixture material or required engine vectors | Record `missing-provenance` or `missing-gate`; do not start extraction | Existing map only, comprehensiveness unmeasured; reopen with pinned independent inventory and executable vectors |
| Typed exact-tree correspondence requires a new IR/compiler or semantic evaluator | Stop M03 with `semantic-boundary` | Discovery plus explicit premises, universal necessity unresolved; reopen with a small pinned upstream API and positive/adversarial semantic vectors |
| An accepted same-premise omission contradicts an established relation | Record `unsound-relation`; stop affected checker and dependent promotions | Retain contradictory artifacts and discovery; reopen with corrected rule and regression witness, never suppress the witness |
| Supported denominator/false-relation threshold fails | Record `coverage-gate`; do not exclude hard cases or relabel them supported after the run | Publish actual counts and abstentions; narrower scope requires a governing revision preserving prior target/results |
| Resolving context code needs arbitrary register chains, recursive dispatch, signature/AVL reasoning or new search | Stop M05 with `unsupported-authentication` | Exact supplied extension execution remains possible through existing evidence; mandatory authenticated delegation unresolved; reopen with one bounded fragment and independent omission control |
| Source cannot establish coherent snapshot or complete query results | Record unresolved source reason per artifact; stop any milestone requiring that evidence | “Observed responses at reported height; simultaneous availability and protocol completeness unknown.” Reopen with pinned complete material; never invent historical state |
| No accepted satisfying witness can be supplied for a purported positive necessity case | Stop that capability gate with `vacuous-positive` | Static claim unestablished for release; reopen with full accepted witness under the same P |
| Action checking would require automatically finding inputs, proofs, outputs or values | Refuse expansion; record `search-out-of-scope` if it blocks the unit | Explicit action checking and supplied replay only; a search proposal needs its own independent witness/control and governing gate |
| Whole-protocol completeness needs unknown off-chain code, arbitrary computation, future config or lifecycle reachability | Record `completeness-ceiling` for the refused proposal; no endless follow-on queue | “Complete accounting of registered sites at this supplied snapshot; unresolved classes listed; no claim about all actions, states or deployments.” Reopen only with an independently closed domain and a gated smaller question |

Per-artifact abstention on a preregistered unsupported case is expected passing behavior; failure on a preregistered supported case is a milestone stop. This distinction cannot be changed after seeing results without the documented scope-reduction process. A ceiling report is a useful outcome, but must not relabel the proposed comprehensive capability as delivered.

## 8. Frozen work and displacement

This pass changes only this design file. Proposed adoption would narrowly unfreeze mapping's new discovery/necessity artifacts and explicit action checking. It would not thaw the legacy map-to-drain helper, `protocol_nfts` victim model, detector rules or severity, conditional audit discharges, mutation/search family, caps/objective, decompiler, structural matcher, ingestion recovery, Compose recipes, Play, UI, canonical evidence semantics, pinned node revision, or P00–P08 baselines/gates. Necessary correctness fixes require their own scoped decision; they are not hidden inside fixture updates.

There is **no unfinished numbered P-unit to displace**. The proposal replaces the post-P08 stop with one bounded 19-day mapping allocation; it consumes the next implementation/review capacity that would otherwise remain unallocated or go to maintenance. It explicitly postpones every competing appendix proposal: compiler/version support, search axes, solver/steering, keyed insiders, chains, historical-state verification, real-protocol precision campaign, new recipes and UI work. Each M-unit consumes its stated share of that allocation; no second workstream is opened. If maintenance must consume it, reduce or stop this queue through the same governing process.

The historical Phase B promises of automatic private false-positive discharge and a new proven mutation operator are displaced by public-independent relationship and omission experiments. Automatic finding discharge and mutation-corpus expansion remain frozen. M05 is the ceiling, not a bridge to unrestricted composition. The next decision after its report may be to retain explicit unresolved boundaries permanently.
