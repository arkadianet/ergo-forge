An author can declare versioned protocol guarantees, supply a finite initialization and action model, and run an offline bounded search that returns independently replayable, node-accepted counterexample traces with explicit property, reachability, provenance and search limits.

The strongest argument against building this arc is that authoring trustworthy properties, initialization rules and action templates may cost more than writing the regression witnesses directly: the tool could become a second, incomplete protocol implementation whose impressive search results mostly expose mistakes in its own model. A supplied-witness property workbench may be the useful endpoint. The first measured result must be allowed to cancel lifecycle construction and solving, rather than merely postpone them.

Status at initial design: **design only**, inspected at `dd8dab6af8c494b86e626ea29a274b08372854c1`. The subsequent [D00 governing decision](../../discovery/D00-DECISION.md) authorizes only incremental D00 registration and its inventory; all later units and capabilities remain proposed. Proposed filenames, tests, budgets and numeric targets below are future deliverables, not measured results. [ROADMAP](../../ROADMAP.md), its frozen measurements and its section 6 stops remain authoritative. M05 remains stopped. No product benchmark was rerun for this record.

## 1. End-state and the dependency between capabilities

The concrete workflow is:

1. The author pins exact contract bytes, compiler/node revisions, context rules, property definitions, role bindings and local fixture or public historical inputs. They provide good and bad examples of what each guarantee means, including authorization assumptions. The tool refuses unsupported or ambiguous definitions.
2. The author declares an initialization transaction from explicit external premises, a finite inventory of action templates, owned test credentials, environmental schedules and bounds. The tool constructs canonical transactions, obtains only supported proofs and validates every transition through the existing full node validator. Nothing is fetched or broadcast by the experiment.
3. The author runs deterministic bounded action exploration. Within a selected arithmetic branch, an optional bounded numeric proposer fills declared numeric holes. Every proposal goes through canonical construction, fresh proofs where necessary, full node validation and independent property evaluation.
4. The report contains either a replayable violating trace or an accountable ledger of tested cases, rejections, unsupported operations and exhausted bounds. A clean run says “no counterexample found within this registered model and budget.” The author replays the exported evidence without the searcher or solver, inspects the first failed guarantee and corrects their contract or model. A changed property, model or contract creates a new experiment identity.

This is a **bounded counterexample searcher for author-declared protocol models, with concrete execution evidence**. It is not an exploit oracle, deployment certification, whole-chain protocol inference or proof that the author's model captures intended economics. It cannot establish unrestricted safety, historical availability from box bytes, arbitrary signature capabilities, fair scheduling, price assumptions, unbounded liveness or coverage outside declared actions and arithmetic. Whether all three capabilities are worth shipping is deliberately unsettled.

| Capability | Useful independently | What it cannot buy without the others |
|---|---|---|
| 1. Author properties | Turns supplied accepted executions into refutations of guarantees beyond extraction; supports regression review | Without 2, arbitrary supplied states do not demonstrate an occurrence from initialization. Without 3 or another proposal source, a property cannot discover missing numeric material. A correct evaluator with no applicable author guarantee has no discovery utility. |
| 2. Lifecycle construction | Produces canonical, linked, legally executable states relative to a declared root; can remove spurious supplied-state hits | Without 1, legal traces do not identify protocol failures. Without 3, fixed numeric domains can miss a reachable branch; construction does not solve it. It has no value for a case whose initialization or action inventory cannot be grounded. |
| 3. Numeric solving | Finds previously unsupplied assignments in a deliberately small branch fragment | Without 1, satisfiability is not a flaw. Without 2, a violating assignment may only work in an assumed state. Without fresh node replay, solver output has no claim authority at all. |

The shared boundary is an immutable experiment manifest and an execution trace. Properties consume validated observations; lifecycle construction supplies trace prefixes; the numeric proposer supplies assignments for one action at a prefix. No producer may supply its own accepted/violating/reachable flag. Mapping may nominate boxes or requirements for author review, but neither graph reachability nor M04 action-set satisfaction supplies transition legality or lifecycle reachability.

## 2. Capability 1: property vocabulary and authority

### 2.1 Versioned declarations

Add a separate `author-property:v1` vocabulary and `property-replay:v1` envelope. Keep `recognized-attacker-receipts-v1`, existing `ReplayBundle` format 1, CLI/HTTP/UI behavior and old reports byte-compatible. Do not broaden `evidence::claim::Property` in place or reinterpret its extraction numerator. The new library entry point is explicitly opt-in; a new CLI, server endpoint and UI are outside this arc.

A declaration contains schema version, author property ID/revision, exact contract identities, scope (`transition` or `bounded-response`), typed role selectors, guard, assertion, units, authorization premises and source/provenance references. Its digest binds all these fields. An experiment also binds exact canonical transactions, input/data-input bytes, contexts and their origins, node/compiler and evaluator revisions, finite trace rules and budgets. Unknown versions, fields, operators or ambiguous bindings are refused, never defaulted. Import re-evaluates declarations and execution material; it does not deserialize claim authority.

Role selectors use explicit input/output positions, exact box IDs, exact script bytes or exact token-ID predicates over a bounded collection. Each binding declares `exactly-one` or `all-matches`; a count assertion can require zero, one or another literal cardinality. No implicit first match, uniqueness from fungible membership, actor identity from a role name, or carry-over of roles to successors. An empty `all-matches` sum is zero; `exactly-one` failure is unresolved. Assertions needing existence must say so. Missing fields/type mismatches are unresolved rather than zero or false.

The expression fragment is finite typed JSON, not callbacks or embedded ErgoScript: Boolean constants/and/or/not; exact byte equality; integer equality/order; literal count; sums of selected ERG/token quantities; named integer register reads; addition/subtraction and multiplication by an integer literal. Amounts, counts and heights carry distinct declared units; explicit compatible arithmetic is required. Accounting uses checked mathematical integer arithmetic with a signed 128-bit result ceiling; overflow is unresolved, never wrapping. Contract arithmetic is not inferred from this arithmetic. No division, variable multiplication, floats, price feeds, arbitrary collection programs, recursion, hash inversion, AVL reasoning or dynamic extension execution.

Per declaration: at most 32 expression nodes, eight role bindings, 16 boxes per collection, eight token entries per box inspected, and eight transactions per supplied trace. Exceeding a bound is an explicit unsupported/capped disposition. No expression cap is silently raised to admit a fixture.

### 2.2 Four guarantees and concrete refutations

For a transition property `(P, G, A)`, the author's claim is: every accepted transition in scope P for which guard G is true satisfies assertion A. The tool **does not establish this universal claim**. It refutes it with one fully accepted, same-premise transition for which G is true and A is false. G false means `not-applicable`; missing G means `unresolved`. Track guard coverage so a vacuous set of examples cannot pass utility gates.

| Family | What v1 expresses | Refuting evidence and deliberate limit |
|---|---|---|
| Reserve accounting | For declared reserve/liability roles, e.g. `reserveAfter - reserveBefore >= liabilityAfter - liabilityBefore`, with units and fixed scale; explicit fee/external-flow adjustments when needed | An accepted transition with bound roles and false inequality. Partial reserves are labelled partial; no market solvency, inferred exchange rate or inferred complete reserve inventory. |
| Authorized issuance | A delta in a named token supply across the declared transaction collections requires spending an exact authority-script input, or respects an author-declared quota | Accepted excess issuance or positive delta with the required input absent. Full node validation establishes acceptance of that input's exact script, not a generic statement that a named human consented. No parsing arbitrary proof bytes into signer identity. Unauthorized mint and authorized over-issuance are distinct declarations. |
| Continuation preservation | If an exact state-role input is spent, require a specified number of successor outputs preserving explicit script/token/register fields | An accepted spend with missing, duplicate or changed successor under the declared rule. This does not prove which NFT is globally unique or impose continuation on a branch whose guard permits termination. |
| Bounded progress/response | A trigger on an accepted transition must be followed by a stated goal within K accepted transitions, where `1 <= K <= 4`, under an explicitly recorded action/environment schedule | An accepted linked trace with a true trigger, K subsequent transitions and no goal at any of the K successor states. Early termination or incomplete horizon is unresolved. This refutes the finite response promise only; no claim about eventual progress, fairness, elapsed real time, transaction inclusion or an indefinitely idle chain. |

`bounded-response` describes one supplied trace only; do not stretch it to encode comparisons between alternative traces. Such comparisons are outside this record’s scope.

For bounded response, the trigger and goal use the same typed observation fragment, with trace index and canonical prefix identity bound. Environmental restrictions must be independently checkable on every supplied step. No “eventually” operator, unbounded temporal logic, absence-of-enabled-action proof or universal deadlock verdict is supported. Exhausting action proposals without reaching a goal does not refute liveness. A protocol whose meaningful liveness guarantee requires those features receives `unsupported-property`, even if that leaves the entire progress family practically unusable.

Example: an author's “request is serviced within two subsequent accepted protocol actions” can be refuted by a request plus two accepted actions that preserve the pending request. It says nothing about whether an adversary can prevent all subsequent transactions. Do not substitute this finite promise for the author's actual eventual-service requirement without recording their different claim.

The output distinguishes `violated`, `holds-on-execution`, `not-applicable`, `unresolved` and invalid declaration; execution rejection is separate. Only a private, non-deserializable result created from fresh accepted execution(s) and the evaluator may carry `violated`. It includes evaluated operands, exact bindings, first failing assertion/index and all premise digests. `holds-on-execution` is never “property proved” or “safe.” A bad author assertion may be faithfully refuted without exposing a contract bug; model review and contract-defect attribution remain separate adjudications.

### 2.3 Supplied trace replay is not lifecycle search

Capability 1 accepts manually authored traces. Existing `ergo-sandbox/src/evidence/replay.rs` provides only one `ValidationRequest` per `ReplayBundle` and one `validate` call; `ergo-sandbox/src/evidence/validate.rs` validates one transaction against its supplied snapshot, not a linked trace. Multi-step checking is a proposed D02 deliverable, not an existing replay capability.

D02 must create a separate request for every supplied step and obtain fresh acceptance through the pinned `evidence::validate::validate` path, including the trigger and all K subsequent transitions for bounded response. Preflight results, stored acceptance flags and acceptance of only the final step cannot authorize `violated`. The proposed checker must independently check canonical predecessor/output references, no double spends, data-input availability, state updates and context schedules across those requests, including cumulative block cost within a block. External boxes are registered at the root; a later unexplained box is refused. This small replay checker is needed to score finite response, but builds no action or initialization transaction and explores no successors. Its report remains `assumed-root` even when every supplied step is accepted.

If this checker cannot pass within D02’s existing ceiling, the four-family D03 promise is not deliverable: stop the dependent units and record the boundary, without dropping bounded-response cases or raising caps. D00 can validate individual reference transactions with the current API, but cannot claim linked-trace replay from those calls alone. Capability 2, if separately authorized, reuses this checker; it must not write a second transition interpreter.

## 3. Capability 2: valid lifecycle construction, provisional boundary

Reachability is the largest semantic gap. A node-accepted spend under a supplied box/context bundle does not show those boxes can coexist or that the protocol can create their register/token values. The proposed constructor starts with an explicit finite external UTXO set and initialization transaction. It validates initialization, materializes its actual output IDs from canonical bytes, then explores declared actions using only still-live prefix outputs or registered external inputs. Every step has a fresh full validation request, supported proofs and a state update derived from accepted bytes. Transaction edits invalidate old signatures and IDs. Script rejection cannot be repaired by changing state premises invisibly.

The action model fixes input/data-input selectors, transaction skeletons, permitted role transitions, finite parameter domains and credential ownership. Node acceptance, not action-set checking, defines a legal spend; satisfying the property must **not** be an action prerequisite or construction would exclude its own counterexamples. Template scope is a caller premise: any omission of a possible contract action limits search coverage. Funding, token issuance history, data-input coexistence, headers, activation, timestamps, height evolution and block-cost assumptions are explicit. Within a modeled block, cumulative cost is carried forward; block changes use a pinned supplied schedule. No PoW/history fabrication or claim of future inclusion follows from that schedule.

Initial experiment ceilings: depth three after initialization, four declared action classes, eight parameter assignments per action/state, 128 distinct explored states, 512 full-node validation calls and 60 seconds per case. All ceilings apply; the first reached wins. State identity includes canonical UTXOs, trace monitor state, environmental position and credential scope; pruning equal box sets alone must not erase a pending response obligation. Depth three need not exercise all capability 1 horizons. Longer required traces remain outside the construction bound.

Reachability is an independent field:

- `assumed-state`: isolated canonical supplied state, no checked prefix.
- `assumed-root`: accepted linked prefix from externally supplied state without validated declared initialization.
- `constructed-from-declared-init`: initialization and every prefix step revalidated, relative to named external funding/context/credential premises and finite model.
- `unresolved`: missing/broken ancestry, availability or schedule evidence. No reachable violation promotion on this path.

Historical provenance remains separate (`source-recorded`, `hypothetical`, `mixed`, with per-premise evidence). `constructed-from-declared-init` may be entirely hypothetical and is never “historically reached.” Even initialization is relative to an assumed external root unless independently authenticated history is supplied. This arc builds no historical membership verifier. Capability 2 buys occurrence in a checked, bounded author model; it cannot remove all assumptions about the world.

Gate contract, subject to the checkpoint: independently authored init-plus-two-action witness/control pairs in at least two property families, including one issuance/continuation interaction; a known bad arbitrary state not constructed within the registered model and bounds; all accepted violating traces replay without the constructor; tampered references, reused spends, missing data inputs, invented funding and invalid initialization are refused. Compare the same frozen cases with supplied-state testing and construction; require at least one valid constructed violation and one evidenced withholding of constructed-reachability credit from that supplied-state candidate, relative only to the registered model and bounds. This is a filtering benefit, not evidence that the state is globally spurious or unreachable: omitted actions and exhausted bounds may hide a construction path. Unreached within bounds is `not-constructed-within-bound`, never proved unreachable. Failure to build the reference trace within the cap is a miss, even if a shorter unrelated trace succeeds.

## 4. Capability 3: bounded numeric solving, provisional experiment

The initial fragment is quantifier-free, bounded linear integer constraints: at most eight signed 64-bit variables, 32 constraints, eight branch alternatives; literal-bounded domains; addition/subtraction, literal multiplication, equality/order and Boolean combinations. Only declared output values/token quantities, integer registers and integer context-extension holes are writable. Inputs already in a reached state, scripts, token IDs, role membership, proofs, heights and context rules are fixed. No general tree-to-SMT compiler is promised. A small typed adapter binds each supported constraint to exact typed-tree nodes, branch and variable locations; unsupported anchors, types or semantics stop that branch. Solver claims about unreachable or ignored code are not contract coverage.

For supported contract arithmetic, constrain every intermediate value to the pinned node's representable range and exclude overflow-dependent paths. Preserve strict inequalities and operand widths. Do not silently use property arithmetic as contract semantics. Division/modulo, nonlinear arithmetic, bitwise operations, collections, hashing, signatures, AVL proofs and arbitrary dispatch remain unsupported. ErgoScript totality does not justify expanding this fragment.

Each proposed assignment is substituted into one declared action skeleton at a recorded prefix. Canonical serialization, fresh supported signing, full node validation and property scoring follow in that order. The proposer can target a supported branch and a false property condition when translatable; an untranslatable property is scored only after proposal and never reinterpreted by the solver. Export all replay inputs so reproduction does not require a solver binary. Solver `sat` means candidate; `unsat` means no assignment for that encoded branch/domain only, never a safe verdict. Timeout/unknown/encoding failure are separate and preserve the case denominator. A mismatch between predicted execution and node execution is retained and blocks the affected encoder until resolved.

Pin backend/version/binary hash, seed, ordering, thread count and resource settings before comparison. Limit each query to one second, 64 model proposals and 512 total node calls per case, and 60 seconds wall time per case including encoding, signing, validation and scoring. Combined lifecycle/solver runs share the lifecycle 512-call and 60-second budget; the solver does not receive an extra allowance per state. Candidate exclusion clauses may enumerate alternate assignments within those limits; no stochastic tuning after seeing held-out answers.

Gate contract: a registered arithmetic witness whose necessary value is absent from supplied enumeration choices, a clean original, differential small-domain exhaustive checks against the pinned node, refusal controls for every excluded operator class, fresh end-to-end replay, and at least one **additional held-out attributable discovery** over deterministic enumeration at equal node-call and wall-time ceilings. Also report domain enumeration as a separate baseline so novelty from a larger domain is not presented as solving efficiency. Baselines receive identical domains, action skeletons and property definitions; the solver gets no answer-key values or completed witness transactions. Already exposed witnesses are regression cases only. Ten person-days is the absolute experiment ceiling, including benchmark registration and reporting.

The current [section 6 new-axis admission rule](../../ROADMAP.md#6-stop-rules--record-a-boundary-instead-of-extending-the-project) still requires the twenty-mutant/all-operator gate before new search work. Capability 1's property corpus does not satisfy it. Lifecycle exploration and numeric search remain ineligible until a separate registered search cohort satisfies that prerequisite, with independently validated witness/clean-original pairs and all proposed mutation operators represented. If that inventory cannot be honestly constructed within the later budget, cancel search. This record does not waive or edit that rule.

## 5. Registration, metrics and PR-sized capability 1 units

### 5.1 D00 inventory before evaluator construction

Follow [M00](../../mapping/M00.md): independently authored answers precede output inspection, and fixture accounting is separate from discovery. D00 registers **24 cases**: four named families (`reserve`, `issuance`, `continuation`, `bounded_response`), each with two accepted violating examples and two accepted nonviolating controls, plus eight boundary cases. The boundary IDs are `missing_role`, `ambiguous_role`, `wrong_register_type`, `arithmetic_overflow`, `false_guard`, `short_response_horizon`, `unbounded_eventuality`, `unsupported_dynamic_expression`. Thus 16 intended supported cases comprise eight violations and eight controls; all eight boundary cases remain in total accounting. Guard-inactive is not a nonviolating control. Cross-family examples must receive one primary case/family ID and do not multiply the denominator.

Pin exact source/tree/fixture bytes and hashes; canonical boxes, transactions, proofs, contexts, role/authorization premises; complete expected bindings/operands/verdicts and independent explanations; case IDs, exclusions, publication status and exposure log; node/compiler/Forge/lockfile hashes, dirty-tree status, budgets and commands. Store independent answers separately from production inputs, with the accepted manifest and answer digests locked outside the files they authenticate, as M00 does. Use `treeHex`/`propositionHex` fixture fields to avoid expanding the frozen recursive decompiler corpus. Pin raw baseline outputs without rewriting existing artifacts.

Synthetic authorship is not separate human adjudication. D00 needs an independent reviewer of property meaning and reference verdicts; absent review yields an incomplete semantic measurement, not a transfer pass. Before output inspection, register two additional public or author-released local contract/action cases from a different code family, chosen for a consequential guarantee beyond extraction. They are a separate two-case transfer denominator, not part of the 24. Pin reference violating/control executions and manual question, with operator exposure recorded. Nomination has a four-person-hour ceiling inside D00; if no defensible pair can be supplied, leave transfer denominator `null`, complete only the synthetic regression inventory and stop the arc at the checkpoint. No historical recovery or private publication project follows.

A corrected fixture after registration becomes a new version outside the accepted score; preserve its original failed row. D00 can catch draft authoring defects before locking, with history retained. No gate may quietly rename an unsupported case into a passing supported case.

| Metric | Honest value at this design | Required evidence / failure |
|---|---|---|
| Implemented built-in property versions | 1, `recognized-attacker-receipts-v1` | Code inventory, not a discovery numerator; old results retain their own denominators. |
| Registered new property cases / planned 24 | 0/24 deliverables; measured-case denominator `null` | D00 must freeze every member and reference answer. Missing inventory is incomplete work, not zero recall. |
| Evaluator agreement | `null` | D03: 24/24 exact expected dispositions, including eight true violations and eight accepted controls; any false violation is a correctness stop. |
| Applicable guard coverage | `null` | All 16 intended supported cases have true guard and valid bindings; inactive/missing guards cannot satisfy this gate. |
| Replay fidelity and claim identity | `null` | Every counted violation freshly replays; every identity/premise/version tamper invalidates imported status. One mismatch stops promotion. |
| Author usefulness beyond extraction | `null`; no registered transfer denominator | Both registered transfer cases express the reference guarantee, with accepted violating/control replay; at least one independently adjudicated consequential decision beyond extraction. Otherwise no utility pass. Report author/reviewer hours per case and unresolved disagreements. |
| New attributable discoveries | 0 credited; eligible discovery denominator `null` | D03 uses supplied witnesses and claims none. Later divide novel node-accepted property violations on cleanly held-out cases by all frozen eligible cases, with misses and caps retained. |
| Constructed reachability coverage | `null` | Later: recovered independently known reference traces / all registered reference traces, plus withholding of constructed-reachability credit within the registered model and bounds separately. Existing mapping reachability is not this metric. |
| Numeric incremental yield | `null` | Later: paired held-out gain at equal budgets must be at least one attributable discovery; zero cancels solver. Report invalid proposal and timeout counts, node calls and wall time. |
| Unrestricted safety, deployed completeness, incident discovery rate | `null` | No denominator or producer in this arc; neither synthetic cases nor replayed historical incidents fill these cells. |

These are finite go/no-go criteria, not statistical population estimates. Every report includes all case IDs, numerator/denominator, provenance/exposure splits, unsupported/missing/capped rows, raw command exits and artifact hashes. Count unique case/property/root violations, with duplicate traces reported separately. An accepted historical replay remains reproduction; there is no incident-count target.

### 5.2 Capability 1 implementation queue

All paths below are relative to the repository root. New production files live under `ergo-sandbox/src/properties/`; they reuse `evidence::{wire,validate,sign}`. Existing `ergo-sandbox/src/evidence/claim.rs`, `ergo-sandbox/src/evidence/replay.rs` and `ergo-sandbox/src/evidence/promotion.rs` remain semantic compatibility boundaries. New tests may call their public interfaces without changing them.

| Unit / ceiling | Concrete files and deliverable | Gate substance |
|---|---|---|
| D00 / 2 person-days | `ergo-sandbox/tests/fixtures/properties/{manifest,expected,legacy-results}.json`; `ergo-sandbox/tests/property_inventory.rs`; `docs/discovery/D00.md` and registration receipt | Freeze 24 rows and separate transfer registration; independently validate each reference transaction using current node API; linked-trace checking awaits D02; record current extraction outcomes and unsupported new-property producer. Authenticate old policy/baselines. |
| D01 / 2 | `ergo-sandbox/src/properties/{mod,schema}.rs`, module export in `ergo-sandbox/src/lib.rs`; `ergo-sandbox/tests/property_schema.rs` | Strict typed declarations, units and limits; immutable normalized claim identity; round-trip inputs only; unsupported/missing/ambiguous inputs fail closed. No evaluator authority yet. |
| D02 / 3 | `ergo-sandbox/src/properties/{evaluate,trace}.rs`; `ergo-sandbox/tests/property_evaluation.rs` | Evaluate four bounded families only from fresh accepted execution observations; new per-step node-validation orchestration, supplied-trace linkage and monitor semantics; guard/empty-set/type/overflow adversarial cases; no constructor or proposer. |
| D03 / 3 | `ergo-sandbox/src/properties/replay.rs`; `ergo-sandbox/tests/property_replay.rs`; `docs/discovery/D03-results.json`, raw logs and author review | Public library replay entry point, output-only violation type and full envelope; 24-case agreement, fresh tamper tests, unchanged legacy semantics; first actual author usefulness measurement with every transfer row retained. A negative result is reportable but does not pass the utility test. |
| D04 / 2 | `ergo-sandbox/tests/discovery_checkpoint.rs`; `docs/discovery/checkpoint.json` and decision memo | Audit D03 raw results and blockers, encode exactly one next allocation or cancellation with evidence; reject continuing search after a failed utility/correctness prerequisite. No capability 2/3 code. |

D04 depends on D02, not D03, intentionally: it must run to record a stop even when D03's first measured result fails utility. Its test loads D03's actual command report and verifies that failure is retained; it cannot pass a nonexistent measurement. A passing checkpoint is a decision gate, not a feature gate and never turns D03 green. After a failed D03 gate, `completedThrough` stays unchanged. D04 remains blocked if D02 or its prerequisites fail; in that case record the stop directly under section 6, without claiming a passing checkpoint.

### 5.3 Runner-compatible proposed registrations

The JSON below contains **proposed entries for the existing policy's `units` array**, not a second policy source. The author-authorized 2026-09-11 decision registers **D00 only**, initially unimplemented, and retains all five entries below as the full proposed queue. D01–D04 remain unregistered; each needs a later governing registration decision. **CI cannot enforce gates for units not yet registered.** This is the explicit cost of incremental scheduling, not a change to CI acceptance semantics. See [the decision](../../discovery/D00-DECISION.md). Preserve the existing `schemaVersion`, `baselineRev`, thresholds, scoreboard, stop records/resolutions and P/M units. The runner reads only `docs/ROADMAP.md`; it has no alternate-spec argument. Do not add one or claim these tests run today. M05 is not a dependency and remains stopped; D00 depends on the passing M04 prefix. D04's deliberate dependency is explained above.

```json
[
  {"id":"D00","depends":["M04"],"days":2,"package":"ergo-sandbox","target":"property_inventory","tests":["property_inventory_pins_24_cases_and_independent_answers","reference_executions_and_legacy_results_are_reproduced","transfer_registration_and_exposure_are_accounted"],"implemented":false},
  {"id":"D01","depends":["D00"],"days":2,"package":"ergo-sandbox","target":"property_schema","tests":["property_versions_units_and_limits_fail_closed","bindings_never_infer_missing_roles_or_authority","declaration_identity_binds_all_semantic_premises"],"implemented":false},
  {"id":"D02","depends":["D01"],"days":3,"package":"ergo-sandbox","target":"property_evaluation","tests":["four_property_families_match_independent_operands","guards_missing_fields_and_overflow_preserve_unknowns","bounded_response_requires_a_complete_accepted_linked_trace","property_evaluation_requires_fresh_node_acceptance"],"implemented":false},
  {"id":"D03","depends":["D02"],"days":3,"package":"ergo-sandbox","target":"property_replay","tests":["all_registered_property_dispositions_match","property_replay_rejects_tampered_claims_and_premises","legacy_replay_bytes_and_semantics_remain_unchanged","registered_author_cases_show_usefulness_beyond_extraction"],"implemented":false},
  {"id":"D04","depends":["D02"],"days":2,"package":"ergo-sandbox","target":"discovery_checkpoint","tests":["checkpoint_accounts_for_first_measurement_and_all_blockers","checkpoint_cannot_continue_after_failed_prerequisites","next_allocation_has_finite_gates_or_explicit_cancellation"],"implemented":false}
]
```

The current runner selects only the requested unit and its transitive dependencies (`scripts/roadmap_gate.py`, `select_units`); it does not run intervening entries. With these proposed registrations, `--require D04` selects D00, D01, D02 and D04 plus their existing prerequisites, but never D03. After separate registration and implementation, the checkpoint sequence must therefore be:

1. Run `python3 scripts/roadmap_gate.py --require D03 --report docs/discovery/D03-command-report.json` first. Preserve the report and actual process exit even on failure; do not condition D04 execution on a zero exit or overwrite this first attempt with a retry. The runner writes its report before returning nonzero for a failed gate.
2. Then run `python3 scripts/roadmap_gate.py --require D04 --report docs/discovery/D04-command-report.json`. D04 loads the preserved D03 report and `docs/discovery/D03-results.json`, binds their hashes, policy/manifest identity, attempt revision and dirty-tree evidence in the checkpoint, and verifies actual execution of the D03 integration target with its exit/output. D03 must write complete per-case results and artifact hashes before its utility assertion can fail; report generation belongs to the existing D03 allocation.
3. D04 rejects missing, stale or mismatched artifacts and a D03 entry that was only unimplemented, blocked or failed to build/discover tests. A command failure alone is not a complete measurement. Preserve incomplete attempts as stop evidence; do not fabricate missing rows or a checkpoint pass. A complete negative measurement can support cancellation while D03 remains failed. D04’s own passing D02 prefix is still required.

Future gate command for each unit is `python3 scripts/roadmap_gate.py --require D00` (substitute the unit ID), with captured reports. The unmodified runner discovers test names and runs the entire declared Rust integration target; it rejects missing, ignored or zero-test gates. Numeric requirements live once in the frozen discovery manifest, whose hash the inventory gate authenticates; tests consume it. The old scoreboard remains frozen because the runner's scoreboard is baseline-specific. Discovery metrics are authenticated by the new integration tests, which print artifact hashes for runner capture. The runner does not enforce person-days or semantic claims by reading prose: checkpoint tests verify recorded effort/decisions, while product tests supply semantic evidence.

Each implementation PR must run its required prefix, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace --release`, with `CARGO_TARGET_DIR=./target-p00`. Preserve and explain existing external-only ignored tests; none may be introduced in required targets. `--all-goals` remains nonzero for M05 and any unimplemented/failed new unit. Do not redefine that as complete. Use explicit required-unit results and, where appropriate, existing `--ci` stop handling.

## 6. Mandatory re-plan and finite later envelopes

**Checkpoint: immediately after D03 produces its first complete measurement attempt, before any lifecycle or solver implementation.** A failing or inconclusive first measurement triggers the same checkpoint. There is no automatic second transfer cohort. D04's two days are reserved for adjudication and decision, not improving the evaluator after seeing answers. The entire capability 1 allocation is at most 12 person-days / 96 person-hours, including reviewers and the checkpoint; per-unit limits also apply.

Classify every blocked registered author case by the earliest evidenced blocker: property expressiveness/meaning, input provenance, reachability, action representation, numeric material, proof capability, operational cap or unknown. Preserve secondary blockers and time spent. Reviewer judgment must distinguish a wrong contract from a wrong declaration. The checkpoint publishes numerator/denominator and raw evidence for each classification and chooses exactly one next allocation:

| Evidence from capability 1 | Binding next decision |
|---|---|
| Any false violation, replay inconsistency or unresolved property meaning | Stop dependent capabilities; retain only correctly gated supplied replay. A smaller separately designed correction must retain the counterexample. |
| Transfer denominator missing, either transfer case cannot express/replay its reference pair, or no consequential decision beyond extraction | Cancel 2 and 3 for this arc. Keep property regression tooling only if its own correctness gates pass. No utility claim from synthetic agreement. |
| At least one independently evidenced spurious supplied-state hit or missing legal predecessor is the limiting issue | Prioritize 2; its benchmark must distinguish constructible from assumed states. Do not start 3 merely to populate arbitrary states faster. |
| A registered missed arithmetic value is limiting, while a complete manually replayed init-prefix already exists and no reachability blocker remains in selected cases | Reorder the bounded solver experiment before automated lifecycle exploration; reuse checked supplied prefixes. Its claims retain the appropriate root label. A supplied prefix is not automatic construction. |
| All useful cases use one transition with already supplied canonical states and numbers | Shrink to capability 1; cancel both expansions for lack of incremental work to do. |
| Reachability is useful but fixed finite values recover all reference traces | Implement 2 only; cancel 3 unless a new separately registered arithmetic miss appears. |
| Main blocker is unavailable provenance, unsupported proofs, unsupported property semantics or unknown classification | No expansion; document the prerequisite evidence, without building a history service, new prover or larger DSL. |

For ties, reachability precedes solving unless every selected solver case already has a complete checked prefix. With only two transfer cases, do not claim general prevalence; choose a bounded evidenced case or stop. No favorable synthetic count overrides transfer or correctness failure.

The later envelopes are **gate contracts, not settled PR queues**:

| Envelope | Maximum allocation including registration/review | Required re-plan output before code | Passing boundary |
|---|---|---|---|
| L: valid lifecycle construction | 6 person-days / 48 hours | Exact cases, action/state representation, construction order, split into PRs of at most three days, per-PR files and runner-compatible Rust test registrations | Section 3 witness/control, bounded construction-filtering and provenance gates, plus new-axis admission. Fail if no useful constructed violation, no evidenced filtering benefit, or any accepted trace cannot replay. |
| N: numeric experiment | 10 person-days / 80 hours | Exact backend, pinned typed fragment/anchors, independent search cohort and held-out split, budgets/baselines, PRs of at most three days and runner-compatible test registrations | Section 4 differential, replay and incremental held-out discovery gates, plus new-axis admission. No extra discovery means stop. |

D04's machine-readable decision must include `decision`, selected case IDs and evidence hashes, retained/cancelled envelopes, dependencies/order, displaced work, person-hour ceilings, and proposed units with `id`, `depends`, `days`, `package`, `target`, `tests`, `implemented:false`. Every continuing envelope must supply executable gate definitions and freeze its denominator before implementation. If it cannot, choose cancellation. Changes to order or shrinkage preserve original proposed targets and reasons; they cannot retroactively lower a failed gate. D04 does not authorize its suggested work or append its units to policy.

There is no borrowing between ceilings, parallel capacity multiplier or automatic extension. Maximum arc exposure is 28 person-days / 224 hours if both later envelopes survive; stopping early is expected to reduce it. One active implementation PR remains the scheduling rule. No second solver backend, larger fragment, deeper lifecycle model or new fixture cohort is an implicit follow-up to failure.

### Stop records and unreachable endpoints

Use [ROADMAP section 6](../../ROADMAP.md#6-stop-rules--record-a-boundary-instead-of-extending-the-project)'s exact stop fields: `unitOrProposal`, `reasonCode`, actual `attemptCommit`, `daysSpent`, `gateResults` with actual commands/exit codes/evidence hashes, `retainedCapability`, `unsupportedClass`, `reopenEvidenceRequired`. A future governing change owns registry additions; this design edits no registry. Unattempted work has no invented exit code. Preserve an unsuccessful measurement and original denominator even if a smaller replacement is later designed.

| Trigger | Stop and documented boundary | Minimum reopening evidence |
|---|---|---|
| Any unit or total ceiling reached without gate | Stop that unit and dependencies; retain last coherent passing prefix | Smaller replacement unit with independent evidence and displaced work, never an automatic extension |
| Vocabulary cannot express the author's real guarantee | “Supplied executions checked only against the listed finite properties; this guarantee unsupported” | Refutable narrower declaration accepted by the author, with independent violating/control evidence |
| Meaningful liveness needs fairness or unbounded time | “Bounded response only; eventual progress and deadlock unresolved” | A finite, checkable promise and complete refuting trace; no timeout-as-proof |
| Initialization/action model cannot produce reference states | “Supplied-state replay retained; occurrence from declared initialization not constructed within bounds” | Canonical valid reference prefix and a smaller model; do not label the state impossible |
| Node/proof interface cannot validate generated transitions | “Candidates only for this class; no new accepted counterexamples” | Existing supported validation/proof path with executable positive/control; no replacement consensus interpreter |
| Solver has no unaided held-out gain by its ceiling, or requires unsupported arithmetic | “Computed-value search unsupported for this class; supplied arithmetic executions replay” | Smaller typed fragment and a new independently registered executable benchmark |
| Answer leakage, input drift or post-result budget tuning | Invalidate affected discovery credit, retain all rows as exposed regression evidence | New independent held-out cohort under a separately scoped decision |
| Unsafe property claim or mislabeled reachability | Stop affected claim production and dependent metrics, regardless of yield | Reviewed correction plus the retained adversarial regression and fresh full replay |

Capability 1 may end at a few useful finite properties. Capability 2 may never earn a stronger label than assumed-root replay. Capability 3 may be abandoned after its bounded experiment. The published boundary must list exact supported property versions, action/root assumptions, unsupported classes, failed gates, consumed budget and evidence needed to reopen; it must not preserve the original end-state promise as a roadmap certainty.

## 7. Displacement, freezes and refusals

If later authorized, this arc takes the next implementation allocation ahead of more mapping breadth, M05 recovery, steering, enumeration-cap increases, new mutation axes, browser Play migration, dashboards and recipes. The independent real-protocol measurement proposal may supply registered evidence if performed under its own scope; it is not silently absorbed into this implementation budget or treated as already completed. At D04, the next allocation must name which of these remains deferred; “continue everything” is not a valid decision.

Frozen: P00–P08 behavior and evidence contracts; M00–M04 results and M05 stop; old extraction property/objective, drain search axes, caps and mutation corpus; baseline revision, scoreboard artifacts/thresholds, source/compiler/node pins, old public-incident counts, mapping relation authority, CLI/HTTP/UI and all confidential material. Any necessary future compatibility change needs a separately versioned design; it is not incidental cleanup. The new arc is local library/artifact work over explicit inputs. No private source, witness or identifying fixture enters public CI without its own publication authorization.

Refuse to build or claim:

- Unsupported “safe,” “secure,” universally live or complete-protocol verdicts, including solver-unsat and search-exhausted variants.
- Hypothetical boxes labelled reachable without a validated initialization/prefix; source-recorded boxes labelled historically available merely because they parse or replay.
- Supplied or leaked witnesses counted as discoveries; answer-key values fed to the proposer; multiple trace variants or renamed incidents padding success counts.
- An unrestricted solver, alternate ErgoScript interpreter, automatic action/property inference, arbitrary key/insider model, history reconstruction service or autonomous scanner justified only by Ergo's totality.
- Property truth as a prerequisite for legal action construction, map adjacency as co-execution, action-set satisfaction as node acceptance, or node acceptance as property satisfaction.
- Extra APIs, UI work, live acquisition, protocol contact, wallet integration, broadcast or automatic issue/report delivery within this arc.

The acceptable deliverable is the measured passing prefix and an explicit decision about the remaining allocation, including cancellation when the evidence does not justify building it.

### D00 attempted registration amendment — 2026-09-11 (stopped)

Following PR #99, the author authorized D00 implementation and exact D00–D04
registration, with only D00 eligible to become implemented, while requiring the
existing `--ci` gate to exit zero. Section 5.3 overlooked the governing PR #98 CI
rule: every registered unimplemented unit without a validated stop makes CI fail.
Consequently even successful D00 implementation cannot meet that combined
landing contract with D01–D04 false. Unattempted future work cannot be made into
completed work or assigned fictitious failure evidence to obtain a green build.

The attempted exact append also exposes M00's policy-authentication projection:
it strips only M entries before checking the frozen P08-policy digest, so D
entries fail that existing gate. The digest and all underlying frozen facts
remain correct and unchanged; registration needs an explicit integration change
to the projection, not a new expected digest.

The [D00 stop report](../../discovery/D00.md) preserves a real run over the exact
proposed registrations and its restored-policy verification. The amendment is
to the registration feasibility claim: the queue above remains proposed, with
its original tests, targets, packages, days, dependencies and false flags; it is
not appended to the governing policy in this stopped attempt. D00 is not
implemented. No inventory, node-reference measurement, independent review,
transfer success or D02 linked-trace replay is claimed.

Before reopening, explicitly reconcile future-unit scheduling with CI's
all-goal acceptance contract and integrate M00's policy projection while
preserving its pinned digest. This amendment grants no threshold waiver, new CI
acceptance state, later-unit implementation or capability 2/3 allocation.


### D00 reopening and inventory record — 2026-09-11

The [governing decision](../../discovery/D00-DECISION.md) supersedes the stopped
registration schedule above. D00 alone is registered; the full proposed queue
in section 5.3 remains visible. Both historical blockers and their raw evidence
remain in the [historical stop report](../../discovery/D00-STOP.md) and registry.
The first was an author instruction error; the second was the latent M00 policy
projection defect. The original digest and manifest bytes are retained, with
separate authentication of the exact permitted additions and rejection tests.

The [D00 execution record](../../discovery/D00.md) freezes 24 synthetic rows,
separate authored answers, a separate null transfer registration and current
extraction measurements. Each reference transaction receives individual current
node validation. No D02 linked-trace replay or new-property evaluator exists.
Independent human review is absent, so semantic measurement is incomplete.
There is no utility or discovery credit. With no defensible transfer pair, the
unchanged checkpoint rule cancels capabilities 2 and 3 for this arc; D04 is not
registered, run or claimed complete here. Capability 1 may remain the endpoint.
