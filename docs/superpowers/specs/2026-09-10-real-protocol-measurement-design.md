Does the existing machinery establish useful, correct obligations about actual protocol actions, and what prevents it from doing so?

Stop further mapping investment if the frozen held-out evaluation produces no useful, correct action obligation attributable to mapping, or if its principal barriers are provenance, ingestion, action representation or review presentation rather than discovery or relation checking. An ungroundable evaluation also stops expansion: it is evidence of an unavailable measurement, not evidence of transfer. Retain the working implementation and use the blocked-case ledger to choose the next bounded task.

# Real-protocol measurement design — 2026-09-10

Historical design record, not an active campaign. The comparison discipline below is corrected for potential reuse; this does not activate the proposal. This document proposes a finite measurement campaign; it does not start it, implement adapters, change behavior, register policy units, resolve stops or edit policy blocks. Baseline inspected: `dd8dab6af8c494b86e626ea29a274b08372854c1` (main after PR #98). Existing M00–M04 capabilities and the M05 stop remain intact. Thresholds below are campaign decisions, not replacements for existing gates.

## 1. Nomination rule, before selecting any new target

An independent protocol reviewer who did not author the mapping implementation or synthetic answers nominates a ranked list of at most four candidates using protocol documentation and operational experience, without seeing Forge output. Record the list, reasons, reviewer identity, conflicts and previous exposure before screening. Select the first eligible public family as the held-out target; do not select by compilation success, recognized identity pattern, finding count or convenience of known fixtures.

Eligibility requires a named version, permission to inspect the material, at least one documented economic action, and an independent route to code/member/action evidence. A different deployment or fork of an already consulted protocol is not a different family: shared contract ancestry or substantially shared action/identity mechanics disqualifies it as held-out. Exclude only for lack of permission, unavailable exact version/material within the screening ceiling, no independently describable action, or prior family exposure. Log every exclusion and its evidence. Compiler failures, dynamic references, missing identity classes and suspected poor fit remain eligible blocked cases. Incomplete member lists disable complete discovery recall, not the target's other measurements.

Screen at most four candidates within four person-hours. Freeze the first eligible family; no replacement after outputs are opened. If none qualifies, stop with `nomination-unavailable`. Do not substitute another synthetic family or relabel a familiar protocol as independent.

SigmaUSD and Lithos are already exposed pilots. Their supplied runs cannot retrospectively become preregistered or held-out. The campaign has at most three targets: these two pilots and one independently nominated public family. Lithos stays a separate confidential source stratum. No additional targets are recruited to repair a result.

The independent reviewer seals the held-out evidence and answer key before any new run. The operator sees inputs only after tools, invocation procedure, budgets, metric definitions and pilot review procedure are frozen. The held-out family is not consulted while building anything. This campaign permits no building at all; if an interface cannot consume its material, record that block. Any later implementation informed by the unsealed family makes it exposed and requires a newly nominated family for a future transfer test.

## 2. Evidence checked for this design

Read-only inspection of saved artifacts confirms the following. These are exploratory observations, not campaign scores, and neither a live query nor a new measurement was run for this design.

| Input | Verified observation | Limit |
|---|---|---|
| Saved SigmaUSD tree output | 722 tree bytes; `complete: true`, 0 raw placeholders, no truncation; recovered `if (HEIGHT > 460000) 800L else 1000000000L`; eight findings | Artifact says `supplied-code; deployment identity not established`, `nodeValidated: false`, no box data. The supplied live Bank v2 attribution still needs independently pinned box/tree identity. |
| Saved SigmaUSD map output | 8 nodes, 26 edges; explorer source at reported height 1869996; per-token truncations of 10 and 3; unresolved `INPUTS(0)` and `OUTPUTS(1)` sites | Caps include two boxes per token and per script hash. Observed responses are not a coherent complete protocol snapshot. The supplied `protocolNfts: 0` summary is not a top-level field in this artifact and is not used as a verified metric. |
| SigmaUSD audit grouping | User reports eight obligations, seven register reads with distinct `unrecognized` keys | Saved tree output contains eight findings but no obligation array. Exact grouping counts/keys remain reported until the audit artifact is recovered or a registered run reproduces them. Code inspection verifies the mechanism below. |
| Saved Lithos ingest output | 28 sources, 26 compiled, two UnsignedBigInt/v0 serialization failures; 66 inferred bindings, 16 raw lift nodes, zero truncated lifts | Top-level `method: source-inference`, `nodeValidated: false`, and `provenance: "source with caller overrides or synthetic bindings; deployment identity not established"`. Inferred values are synthetic premises, not deployment constants. |

Local saved artifact SHA-256 values (locators are temporary, not durable publication):

- `/tmp/sigusd-tree.json`: `3a634eeac1a84a6df5fc9818437d10428825361fe08224931bda8298a5096fb4`.
- `/tmp/sigusd-map.json`: `e7ebdc1602dd08cd14e665b41eb08dc0808c90beec37026a90397e2c9b79eedb`.
- The confidential ingestion artifact's digest is retained locally; its contents and public digest are excluded from this design.

The older [Lithos report](../../ingestion-lithos-report.md) records zero raw nodes for its final run. It must not substitute for the supplied saved run with 16. Register their distinct input/tool provenance or leave the discrepancy unresolved; do not average them. No private source, path list, inferred values or findings are copied here.

[P07's report](../../P07-IMPLEMENTATION.md) and [grouping implementation](../../../ergo-sandbox/src/audit/obligation.rs) agree: only literal typed `GetVar` receivers share witness-schema keys; other expressions use node ID plus occurrence. The twelve-anchor synthetic control does not measure register-schema review burden. [M04](../../mapping/M04.md) checks explicitly supplied actions; it does not infer the action inventory or validate transactions through action satisfaction. These boundaries determine what can be measured.

## 3. Freeze the denominator before running

Follow [M00](../../mapping/M00.md): inputs and independently authored answers precede output inspection. Preserve the exploratory artifacts above separately. The future campaign must produce a timestamped, reviewer-signed registration manifest plus a separately sealed answer key, with SHA-256 over exact bytes and an independently held manifest/answer digest receipt. There is no registration artifact yet and no accepted campaign denominator.

Register and pin:

- Nomination order, exclusions, family definitions, exposure status, disclosure class, reviewer/operator roles, time allocation, thresholds and stop rules.
- Full Forge revision, dirty-tree status, binary and lockfile hashes, compiler/node revisions, network/activation settings, exact existing CLI/API invocations, seeds, all query/lift/traversal caps and ordering. Distinguish legacy `map` from M02 discovery, M03 establishment and M04 action checks; never imply the CLI exercises all four.
- Exact source/tree bytes and versions, binding origins and values, canonical boxes and IDs, script hashes, transactions, headers/context/extensions where available, source URLs/revisions, capture times/heights and raw response hashes. Record coherence and deployment identity as established, conditional or unknown, with supporting evidence. Pin unavailable fields as missing rather than inventing them.
- An independent member list with the unit fixed to exact canonical box IDs at the declared snapshot, seed excluded. Separate role/script lists from box membership. The reviewer derives it from independent deployment records and chain material, never mapper reachability. State the scope and evidence for closure. If only a known subset is defensible, label subset recall and leave protocol-wide recall `null`.
- For each public target, at most three documented action classes, selected in the source document's order before seeing output; retain the full documented action list and all unsampled classes. Each selected action gets exact premises, required participants, property/question, evidence and reference verdict (`necessary`, `not necessary`, `unknown`). Include a documented alternate/optional action when available; lack of a negative case is visible. Lithos registers all 28 ingestion units but at most three source-action cases by the same rule. No claim of exhaustive action coverage.
- Every independently identified reference site/requirement within selected actions, supported and unsupported alike; manual obligation questions and anchor-equivalence judgments; benign/false/unknown labels with reasons. Freeze all output-independent units. The emitted obligation denominator is then every obligation emitted by the frozen run, never a curated subset. Publish its count and unadjudicated remainder.
- Commands, result schema, classification rules, disclosure rules and answer-key access log. Hash raw outputs after each run separately from preregistration.

Failed ingestion, missing provenance, empty output and exhausted caps remain rows in the original inventory. A source correction after freezing is a new version outside the accepted cohort; preserve the original result. Refuse any score whose denominator or reference answer cannot be grounded. No new Rust harness, wrapper, upstream change or reconstructed protocol model is in scope: record `action-representation` when existing entry points cannot accept the registered evidence.

## 4. Measurements and adjudication

An independent reviewer first writes the manual reference questions from the frozen evidence without tool output. Then review all emitted obligations, blind to any claimed success summary. A second reviewer checks all asserted establishments, proposed benign discharges and disputed grouping decisions; unresolved disagreement stays `unknown`. If that review cannot fit the budget, retain unadjudicated rows and withhold the affected conclusion.

A correct observation identifies a real condition under its stated premises. A useful obligation additionally names a concrete action question, evidence needed to settle it, and a consequential review decision. An unresolved register access is not automatically a false positive, a useful question, or a vulnerability. Keep observation correctness, utility and confirmed property claims separate.

| Metric and denominator | Honest current value | Failure or refusal rule |
|---|---|---|
| Obligation precision: correct / adjudicated emitted obligations; also report adjudicated / all emitted | `null`; no independent real-target labels | Below 80% on either public target fails transfer; incomplete adjudication prevents a pass. Unknown is not correct. |
| Obligation usefulness: useful and correct / all emitted obligations, plus distinct useful questions / manual questions | `null` | Below 50% useful emitted obligations, or no useful question on held-out actions, fails practical usefulness. Report missing questions even when output is empty. |
| Mapping contribution: distinct useful, correct action questions settled only with mapping evidence / frozen manual questions | `null` | Zero on the held-out family stops mapping expansion. Compare the same frozen questions using separate reviewers assigned to tree/audit alone and tree/audit plus frozen mapping artifacts. Neither sees the other condition’s decisions before both judgments and evidence citations are locked; record prior exposure and exclude contaminated comparisons from credit while retaining every denominator row. Adjudicate whether the additional cited mapping evidence was necessary to settle each question, not merely whether reviewers disagreed. If separation or adjudication cannot fit the existing budget, keep this metric `null` and withhold a transfer pass. A second reading by the same reviewer supplies no mapping-contribution credit or controlled timing claim. |
| Discovery recall: independently listed non-seed box IDs recovered / complete independent non-seed list | `null`; 8 nodes / 26 edges is not recall | Below 80% on a closed public target fails coverage. Incomplete/empty denominator means `null`, with exact subset hits/misses reported separately. Truncation never shrinks the denominator. |
| Discovery precision: independently confirmed members / adjudicated discovered non-seed IDs | `null` | Below 80% fails useful discovery. Unknown membership remains visible; no complete-precision claim with unadjudicated IDs. |
| Necessity establishment yield: correctly established requirements / independently necessary requirements, all classes included | `null` | Zero on held-out necessary requirements fails establishment transfer; no independently necessary requirements means `null`, not 100%. Report supported-class yield separately without dropping other classes. |
| Establishment precision: correct / all asserted establishments; unsafe assertions separately | `null` | Any false or unjustified establishment is an immediate correctness stop. A sampled failed omission does not prove necessity. |
| Abstention: unresolved / all registered requirements; correct abstentions / independently unknown or unsupported requirements | `null` | Any unsupported requirement promoted without evidence stops; 100% abstention may be safe but cannot pass usefulness. Missing necessary cases count as missed yield. |
| Action representation: grounded selected actions consumable by existing interfaces / all selected actions | `null` | No consumable held-out action refuses action transfer; do not build an adapter to rescue it. Action satisfaction remains conditional, not transaction acceptance. |
| Review grouping: emitted groups / independently distinct questions; split and unsafe-merge counts; decision time per question | `null`; SigmaUSD's reported 8/8 finding-to-group count is diagnostic only | Any merge that hides distinct premises fails safety; multiple groups requiring the same evidenced decision fail presentation efficiency. Report minutes without claiming a controlled speedup. |
| Block accounting: cases with evidenced disposition / all registered cases, counts and person-hours by reason | `null` | Anything below complete accounting fails the campaign report. Every blocked case needs a reason or explicit `unknown`, evidence and next-evidence requirement. |

These small-sample cutoffs are go/no-go rules, not population estimates. Report numerators, denominators and public/held-out/confidential splits; never pool Lithos sources with deployed boxes or synthetic controls. Overall rates cannot hide a held-out failure. Necessary reference labels require a scoped independent argument about exact code/premises; where unavailable, abstain. An already available accepted same-premise omission can refute necessity via existing replay, but no omission-search or historical recovery project is authorized.

### SigmaUSD grouping decision

Provisional diagnosis: the reported result follows the conservative key contract, so it is not yet a demonstrated grouping-key implementation defect. A register-invariant obligation class may be missing; alternatively independent reads correctly remain distinct but need one review presentation. Neither possibility establishes that all seven reads are benign.

For each reported read, recover the exact anchor, receiver/box role, register, type, action branch, state/version scope and evidence that the register is populated. Test the assertion “SigmaUSD always populates R4/R5” against creation and transition constraints within the registered scope, not just observed happy-path boxes. Missing evidence yields unknown. The reviewer partitions anchors by the decision and premises actually shared, before looking at current keys.

Classify the result explicitly: (a) same already-supported typed receiver split contrary to the key contract: key defect; (b) one justified register/state invariant cannot be expressed by an existing obligation category: missing class; (c) distinct semantic obligations share a review decision while requiring distinct anchors/premises: presentation defect; (d) independently distinct decisions: correct separation. Allow mixed outcomes. Measure splits, unsafe proposed merges and review effort even if no remedy is implemented. A seven-to-one presentation must retain seven anchors and cannot transfer a discharge across differing premises.

## 5. Blocked-case taxonomy is the primary deliverable

Produce a row for every case the machinery cannot handle, including cases discovered during review within registered actions. Newly noticed cases are supplemental, never silently added to the frozen score denominator. Each row contains case/action/site ID, attempted stage and command, expected question, observed output or missing artifact, evidence locator/hash, primary blocker, secondary blockers, confidence/disagreement, person-hours spent, retained capability, and the smallest evidence that could reopen it.

| Reason | Distinguishing evidence |
|---|---|
| Ingestion | Exact parse/compile/lift failure, raw placeholder or truncation preventing the question; version/header mismatch distinct from missing source. |
| Identity class | Exact material exists, but token/script/hash/config identity cannot be represented or established by the current vocabulary; fungibility and singleton assumptions explicit. |
| Dynamic reference | Site exists but index, alias, collection search or runtime binding cannot be resolved under available premises. |
| Action representation | Documented action cannot be faithfully supplied through existing action/context interfaces, or required action semantics are absent. |
| Provenance | Source-to-deployment link, exact constants, coherent snapshot, canonical context or independent member closure is missing. |
| Obligation semantics/presentation | Correct anchors do not produce the needed question, or repeated groups impose repeated decisions; split class from presentation as above. |
| Operational cap/access | Recorded query cap, rate limit, inaccessible endpoint or timeout prevents observation; do not diagnose this as an identity defect. |
| Unknown/new class | Preserve the concrete symptom and why existing classes fail. Give an eventual new class a versioned name without rewriting earlier rows. |

Record the earliest evidenced blocker as primary and all known downstream blockers as secondary. Do not guess that lifting or provenance repair would solve the rest. Report both case incidence (with overlaps explained) and time cost. This ledger, including unnamed reasons, is the product that reorders work; a graph-size increase is not a substitute.

## 6. Hard timebox and stops

Maximum **five working days / 40 total person-hours**, including both reviewers, whichever is reached first. Allocate eight hours to nomination/registration, eight to each pilot, eight to the held-out family, and eight to adjudication/reporting. No parallel capacity multiplier, automatic extension or budget borrowing from reporting. The four-hour nomination ceiling is inside registration. Stop with a partial, explicitly incomplete report if independent review is unavailable.

Per target, provenance reconstruction has a hard ceiling of **two person-hours total**, including screening and registration effort, or the first demonstrated need for bespoke historical reconstruction, whichever comes first. This directly prevents the [P05 opportunity-cost failure mode](../../P05-DECISION.md): its eventual recovery does not justify repeating that investment here. Preserve missing identity/context and continue only measurements that do not depend on it. Do not invent constants, recover an epoch, repair a compiler or author hypothetical replacement transactions. One initial run and at most one identical retry for a transient failure; capped live acquisition ends after 30 minutes per target. Offline frozen responses are the measured input.

Use [ROADMAP section 6](../../ROADMAP.md#6-stop-rules--record-a-boundary-instead-of-extending-the-project)'s stop fields in the future campaign report: `unitOrProposal`, `reasonCode`, actual `attemptCommit`, `daysSpent`, `gateResults` with actual commands/exits/evidence hashes, `retainedCapability`, `unsupportedClass`, `reopenEvidenceRequired`. No policy/stop-registry edit is proposed here. Unattempted work has no fabricated command result.

| Trigger | Required action and reopening evidence |
|---|---|
| Nomination, target, provenance or overall ceiling reached | Stop the affected scope, retain all rows and consumed budget. Reopen only through a smaller separately approved measurement with supplied missing evidence and displaced work named. |
| Independent denominator/answer cannot be established | Refuse that score; publish `null` and reason. No replacement denominator derived from Forge output. |
| Accepted same-premise omission contradicts establishment, or unjustified claim/discharge | Stop use of the affected authority and dependent conclusions; preserve contradiction. Reopening needs a separately reviewed correction and retained counterexample. |
| Input drift, answer leakage or changed tool/budget after freeze | Mark affected result invalid for transfer, retain evidence; no in-campaign refreeze to turn it into a pass. |
| Useful output requires building anything | Record the missing interface/class and stop that path. A proposed implementation competes for a later allocation. |
| Held-out mapping contribution is zero, or non-mapping blockers predominate | Deprioritise mapping as specified below; no automatic “one more family” run. |

Define predominant as more than half of blocked selected action cases having an evidenced primary blocker outside discovery/relation checking; report counts and unknowns. If unknown classifications prevent that decision, the outcome is insufficient evidence and mapping expansion remains paused. Any correctness stop overrides favorable utility numbers.

## 7. Confidentiality, deliverables and the next decision

Lithos is a private pre-launch review. Keep source bytes, trees, constants, exact findings, anchors, transactions, paths, answer keys and their identifying hashes in owner-controlled confidential storage, outside committed fixtures and public CI/logs. The confidential reviewer holds the private manifest/digest receipt and can reproduce scores there. No private finding satisfies a public reproducibility or incident-evidence requirement. Existing repository artifacts are not permission to expand publication; this design neither copies nor edits them.

The supplied aggregate ingestion counts may appear in this design as authorized inputs. Future publication of private review numbers requires owner clearance and a disclosure check for small cells or identifying combinations. Publish only approved aggregate counts/rates, denominators, provenance stratum and broad blocker categories, labelled confidential and independently reviewed only if that review occurred. If even a count is sensitive, withhold it publicly rather than calling it zero; retain it in the private report. Permission to publish a number is not permission to publish its corpus. Public reproduction claims cover public artifacts only. Private cases never become committed regression fixtures without separate explicit release authorization; a later independently authored synthetic control must be labelled synthetic and cannot inherit real-case credit.

The finite output is a registration receipt, results/answer comparison, blocked-case ledger and decision memo, with public and confidential storage separated. No dashboards, new benchmark framework or implementation PR are deliverables.

I will accept a result that moves mapping down the roadmap. The decision memo must name the next allocation and what it displaces: if provenance dominates, require supplied evidence or stop deployment claims; if ingestion dominates, nominate a bounded version-support question; if action representation dominates, define the missing action boundary; if review semantics dominates, nominate the evidenced obligation/presentation problem. None automatically authorizes construction. Rank candidates by independently observed blocked action cases and time cost, with uncertainty visible, rather than by ease of extending existing mapping code.

Further mapping is eligible for a separate proposal only if held-out utility and correctness pass and an independently evidenced discovery/relation miss is the limiting problem. Otherwise its next implementation allocation is **zero** pending new evidence. Preserve M00–M04 and the M05 stop; neither an inconclusive campaign nor a useful confidential result reopens M05. The memo must end with one concrete decision—stop, reorder to a named bounded evidence question, or propose a separately gated mapping task—and its reopening evidence. Admitting a negative result without changing the next allocation is not completion.
