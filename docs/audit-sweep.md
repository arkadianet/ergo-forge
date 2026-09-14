# Corpus audit sweep

Runs all registered audit lints across every `.es` file in a corpus directory and
emits one JSON record per contract. Its purpose is to measure **the detectors**,
not to publish judgements about the protocols in the corpus.

```sh
cargo run --release -p ergo-sandbox --example audit_sweep > sweep.json
```

An optional positional argument selects another corpus directory. Discovery is
recursive and sorted, with no allowlist. Directory enumeration failure is fatal;
per-contract read, parameter and compilation errors are retained as records so
an unanalysable contract is visible rather than silently absent. An empty corpus
is an error. JSON goes to stdout, diagnostics to stderr.

It is an **example binary** rather than a subcommand so that scanning a corpus
and producing verbose output stays an explicit act.

## What a sweep does and does not establish

A finding is a **syntactic observation about one tree**. It is not a defect, not
a deployment, and not an exploit. Counts are occurrences: duplicated corpus
variants count separately, and one underlying pattern can raise many. Nothing in
a sweep evaluates a transaction — for whether value can actually leave a
contract, that is the drain hunt's job, and its answer comes from the consensus
reducer.

Reporting a rate honestly means saying who the population is. Our corpus is
mostly *other people's deployed contracts*, which is what makes it a fair test
of the detectors and also why per-protocol results are handled under
[disclosure](#disclosure) rather than committed here.

## Baseline (merged main `e1e7218`, 108 files)

| | |
| --- | --- |
| compiled and lifted completely | 96 |
| compiled, lifted partially | 2 |
| could not compile | 10 |

| Lint | Findings | Files | HIGH | MED | LOW |
| --- | --- | --- | --- | --- | --- |
| unchecked-get | 424 | 51 | 302 | 29 | 93 |
| unbound-box-reserves | 74 | 33 | 74 | 0 | 0 |
| delegated-reserves | 14 | 12 | 0 | 14 | 0 |
| height-guards | 6 | 6 | 0 | 1 | 5 |
| trust-assumptions | 4 | 3 | 0 | 4 | 0 |

522 findings across 74 files; 24 analysed files were clean.

## Detector limitations this sweep exposed

The sweep's most useful output is the list of places our own lints are wrong.
No detector was changed while measuring, so these are open:

- **`unchecked-get` does not canonicalise box aliases.** It compares rendered
  receiver expressions and follows Boolean guard aliases, so a contract that
  proves `OUTPUTS(0).R4[Long].isDefined` in a guard and then reads the register
  through an alias (`successor = OUTPUTS(0)`) is reported HIGH even though the
  read is guarded. Confirmed by hand on a corpus contract. This is a substantial
  share of the 302 HIGHs.
- **`unbound-box-reserves` HIGH wording is too broad for payment outputs.** An
  ordinary recipient output pinned to specific proposition bytes and a required
  value does not need a singleton NFT, but is flagged as though it does.
- **`delegated-reserves` cannot see issuance supply,** so a singleton NFT whose
  amount is guaranteed by supply-1 issuance still reads as an unbounded token
  slot. Correct given what the lint can observe; noise to a reader.
- **The lints are whole-tree and branch-insensitive.** A known-patched contract
  and its pre-patch original produce identical counts here. Silence therefore
  does **not** validate a patch, and this sweep must never be cited as though it
  did.

Two contracts lift only partially and four findings come from those partial
trees; a partial lift can both miss and misattribute.

## Disclosure

The corpus contains real, deployed third-party protocols. Findings that name a
protocol, file or line go to that protocol's team privately first. Per-protocol
results are published here only after a fix, or on a window agreed with the
team. A sweep's aggregate numbers and our own detectors' failures — the two
things above — carry no such constraint and are what this document records.


## S02 sweep — 2026-09-13

Command (exit **0**), with `CARGO_TARGET_DIR=$CARGO_TARGET_DIR` (the shared local build directory):

```sh
cargo run --release -p ergo-sandbox --example audit_sweep
```

The [complete output](reports/batch-2/audit-sweep.json), [command log](reports/batch-2/sweep-final.log)
and [machine-readable site review](reports/batch-2/site-review.json) are retained.
The actual bundled population is **124 files: 114 complete lifts, 0 partial,
10 compile failures**. The spec's historical 79-contract population and the
108-file baseline above are not this run's denominator. The bundle includes
vendored protocol variants, authored examples, 16 vector-example files and LSP
fixtures; compilation uses recorded structural placeholder parameters, not
verified deployment constants. Nothing was fetched or executed on chain.

All **87 sites** from the four new lints were reviewed against the source and
recorded below. `true-positive` means the stated static pattern is present and
useful to review; it never means a vulnerability, accepted spend, or exploitable
deployment. `benign` means an intentional pattern or a recogniser limitation
for this observation, not a safety verdict for that contract. The historical
sweep above is not rewritten, and none of its private findings is promoted.

| Lint | Sites | True-positive (static only) | Benign | Shipping review priority |
| --- | ---: | ---: | ---: | --- |
| `unconstrained-outputs` | 57 | 4 | 53 | LOW |
| `successor-field-drift` | 27 | 2 | 25 | LOW |
| `trivial-sigma-branch` | 2 | 1 | 1 | LOW |
| `unauthenticated-code-execution` | 1 | 1 | 0 | MED |

- **unconstrained-outputs:** Ship as observation: ordinary fee/change, composable accounting and inverted branch guards cannot be distinguished from missing policy within scope.
- **successor-field-drift:** Ship as observation: intentional state resets, delegated accounting, token delivery and singleton supply cannot be resolved within scope.
- **trivial-sigma-branch:** Ship as observation: constant-true test/permissionless scripts are intentional; writable data remains limited to direct result disjuncts.
- **unauthenticated-code-execution:** Retain medium review priority: only the authored positive reports; no benign deployed site was observed, which is not a precision guarantee.

The register recogniser deliberately leaves conditional reset quotas, AVL
updates and HEIGHT-based resets for review. The output recogniser does not
mistake a miner-script subtotal for a total-output bound, infer an inverted
`if` guard, or infer transaction-wide accounting from companion scripts.
Those distinctions require policy/path reasoning outside S02, so the benign
sites remain LOW observations. No HIGH finding is emitted by a new lint.

The executable `deployed_corpus_sweep_is_recorded` gate verifies the corpus file
set, source and compiled-tree hashes, recompiles every record using its recorded
parameters, reruns all four lints, checks every site/priority against this review,
and requires each reason to appear in the table. No empty placeholder can pass.

### Every reported site

Paths are relative to `examples/contracts`; node ids are lift-local anchors in
the stored sweep, not source line numbers or deployment identifiers.

| Contract | Lint | Node | Outcome | Review reason |
| --- | --- | ---: | --- | --- |
| `basics/self-preserving.es` | `unconstrained-outputs` | 1 | benign | The teaching example retains SELF.value at output 0; other inputs can fund fee/change. |
| `chaincash-basis/chaincash/layer2-old/note.es` | `unconstrained-outputs` | 156 | benign | Note transfer/change and redemption have action-specific checks; the tail is outside this historical note template. |
| `chaincash-basis/chaincash/layer2-old/redemption.es` | `unconstrained-outputs` | 103 | benign | Dispute paths intentionally release collateral; preservation is checked only on continuing dispute paths. |
| `chaincash-basis/chaincash/layer2-old/redproducer.es` | `unconstrained-outputs` | 1 | benign | The producer retains its reserves and funds one redemption request; extra funding/change is outside that pair. |
| `chaincash-basis/chaincash/onchain/receipt.es` | `successor-field-drift` | 40 | benign | The receipt explicitly delegates new receipt fields to the co-spent reserve; R6/R7 change for re-redemption. |
| `chaincash-basis/chaincash/onchain/reserve.es` | `unconstrained-outputs` | 127 | benign | Redemption bounds and receipt/buyback checks are action-specific; the note recipient and funding tail vary. |
| `dexy/bank/arbmint.es` | `unconstrained-outputs` | 47 | benign | Bank and buyback deltas plus the action successor are checked; minted-token destinations and funding remain composable. |
| `dexy/bank/arbmint.es` | `successor-field-drift` | 166 | benign | R5 equals availableToMint minus minted tokens; availableToMint selects the old R5 or a reset quota through an if expression. |
| `dexy/bank/bank.es` | `unconstrained-outputs` | 1 | benign | The bank explicitly delegates emission and payout accounting to NFT-selected action inputs. |
| `dexy/bank/bank.es` | `successor-field-drift` | 20 | benign | The source explicitly delegates ERG/emission amount restrictions to NFT-selected action contracts. |
| `dexy/bank/buyback-patched.es` | `successor-field-drift` | 60 | benign | The reported slot is the declared buyback NFT, whose amount relies on issuance supply; the script patch does not establish supply. |
| `dexy/bank/buyback.es` | `successor-field-drift` | 59 | benign | The reported slot is the declared buyback NFT; id-only preservation leaves singleton supply outside static analysis. |
| `dexy/bank/freemint.es` | `unconstrained-outputs` | 6 | benign | Bank and buyback payments and the action successor are checked; mint recipients and funding can occupy other outputs. |
| `dexy/bank/freemint.es` | `successor-field-drift` | 116 | benign | R5 is recomputed from an if-selected reset quota or old R5; the arithmetic-only register recogniser stops at that if. |
| `dexy/bank/intervention.es` | `unconstrained-outputs` | 17 | benign | LP/bank deltas and intervention successor are constrained; update and funding outputs are action-specific. |
| `dexy/bank/payout.es` | `unconstrained-outputs` | 1 | benign | Bank payout is matched by buyback gain and a preserved action box; this is not a fixed-output-count policy. |
| `dexy/bank/payout.es` | `successor-field-drift` | 36 | benign | R4 intentionally records the new payment height using HEIGHT while the old R4 gates the payment delay. |
| `dexy/bank/update/update.es` | `unconstrained-outputs` | 6 | benign | Votes authorise a target upgrade while target reserves and update box are preserved; funding outputs remain open. |
| `dexy/hodlcoin/hodlcoin.es` | `unconstrained-outputs` | 1 | benign | Mint/burn reserve deltas and three treasury recipients are checked; the rest of a composed transaction is open. |
| `dexy/hodlcoin/hodlcoin.es` | `successor-field-drift` | 88 | benign | Slot 1 is documented as the bank NFT; token amount is left to singleton issuance rather than a local amount comparison. |
| `dexy/lp/pool/extract.es` | `unconstrained-outputs` | 17 | benign | LP/extraction reserve deltas are coupled; separate funding and upgrade paths do not fix the output count. |
| `dexy/lp/pool/extract.es` | `successor-field-drift` | 85 | benign | The reported slot 0 is the extraction NFT; Dexy amounts are compared separately, while singleton supply is not visible. |
| `dexy/lp/pool/main.es` | `unconstrained-outputs` | 1 | benign | The pool explicitly delegates action accounting to NFT-selected swap/mint/redeem/intervention inputs. |
| `dexy/lp/pool/main.es` | `successor-field-drift` | 31 | benign | The pool delegates ERG and Dexy accounting to its NFT-selected action contracts; local identity alone is not a reserve proof. |
| `dexy/lp/pool/mint.es` | `unconstrained-outputs` | 10 | benign | LP issuance is compared with reserve additions and the action box continues; depositor outputs remain open. |
| `dexy/lp/pool/redeem.es` | `unconstrained-outputs` | 10 | benign | Redemption deltas and oracle bounds are checked; the redeemer and funding outputs are outside the action successor. |
| `dexy/lp/pool/swap.es` | `unconstrained-outputs` | 6 | benign | Swap reserve arithmetic and action-box preservation leave trader and fee/change destinations to the transaction. |
| `dexy/lp/proxy/Deposit.es` | `unconstrained-outputs` | 21 | benign | LP reward/change and a miner-script subtotal cap exist; that subtotal does not constrain every output. |
| `dexy/lp/proxy/Redeem.es` | `unconstrained-outputs` | 21 | benign | Return amounts/recipient and miner-script subtotal are checked; remaining outputs are deliberately composable. |
| `dexy/lp/proxy/SwapBuyV1.es` | `unconstrained-outputs` | 24 | true-positive | Only the positional reward delta/price is compared; no total-output or tail guard is present in this source. |
| `dexy/lp/proxy/SwapBuyV2.es` | `unconstrained-outputs` | 21 | benign | Recipient, quote and miner-script subtotal checks exist; the subtotal is narrower than total-output accounting. |
| `dexy/lp/proxy/SwapSell.es` | `unconstrained-outputs` | 21 | benign | Recipient, quote and miner-script subtotal checks exist; the rest of the composed transaction remains open. |
| `dexy/lp/proxy/SwapSellV1.es` | `unconstrained-outputs` | 19 | true-positive | Only the positional reward price/fee expressions are checked; no total-output or tail guard is present in this source. |
| `dexy/tracking.es` | `unconstrained-outputs` | 1 | benign | The tracker preserves its value/tokens while updating alarm state; external funding covers fee/change. |
| `dexy/tracking.es` | `successor-field-drift` | 53 | benign | R7 intentionally changes between a HEIGHT window and the infinity sentinel when the alarm triggers or resets. |
| `hodlcoin/phoenix/phoenix_v1_hodlcoin_bank.es` | `unconstrained-outputs` | 8 | benign | Mint/burn reserve deltas and the developer-fee output are checked; user redemption outputs are open. |
| `hodlcoin/phoenix/phoenix_v1_hodltoken_bank.es` | `unconstrained-outputs` | 8 | benign | ERG is preserved, token reserve deltas and developer fee are checked; recipient/funding outputs vary. |
| `hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodlerg_bank.es` | `unconstrained-outputs` | 8 | benign | This variant also checks mint/burn reserve deltas and developer fee, leaving user redemption outputs open. |
| `hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodltoken_bank.es` | `unconstrained-outputs` | 8 | benign | This variant preserves ERG and checks token reserve/fee deltas; it has no fixed recipient/output count policy. |
| `lsp/test_simple.es` | `trivial-sigma-branch` | 0 | benign | The bundled LSP smoke-test contract is intentionally sigmaProp(true), not an authorisation policy. |
| `protocols/amm/pool.es` | `unconstrained-outputs` | 11 | benign | Swap/deposit/redeem arithmetic governs pool reserves; traders need other outputs for receipts and change. |
| `protocols/amm/swap-order.es` | `unconstrained-outputs` | 36 | benign | The order pins its minimum token payment and recipient, permitting a trader refund and unrelated transaction outputs. |
| `protocols/bank/bank.es` | `unconstrained-outputs` | 19 | benign | Mint/redeem accounting and circulation registers govern the successor; user receipts and fee/change remain open. |
| `protocols/mixer/half-mix.es` | `unconstrained-outputs` | 13 | benign | The non-owner path is inside the else of OUTPUTS.size != 2; this absence lint does not infer the inverted branch guard. |
| `protocols/registry/registry.es` | `unconstrained-outputs` | 22 | benign | The AVL insertion and registrar payment are checked; additional funding/change outputs are not a registry invariant. |
| `protocols/registry/registry.es` | `successor-field-drift` | 44 | benign | The successor AVL digest equals the result of a proved insertion into SELF.R4, beyond direct register arithmetic. |
| `recipes/auction.es` | `successor-field-drift` | 30 | benign | A new bid intentionally replaces R4 with the new bidder rather than preserving the previous bidder. |
| `recipes/auction.es` | `unconstrained-outputs` | 32 | benign | Bid refund or settlement payments are checked at their slots; bidder/funder change is outside the auction policy. |
| `recipes/auction.es` | `successor-field-drift` | 112 | benign | Whole-token equality identifies the settlement delivery as a successor; it is an item transfer, not a continuing auction state. |
| `recipes/cliff-vesting.es` | `unconstrained-outputs` | 5 | benign | The beneficiary authorises the vested release while the unvested remainder stays; withdrawal destinations are open. |
| `recipes/nft-sale.es` | `unconstrained-outputs` | 22 | benign | Seller/artist payments and token delivery are checked; the buyer chooses the receiving box and funding change. |
| `recipes/nft-sale.es` | `successor-field-drift` | 57 | benign | Whole-token delivery identifies the buyer output as a successor; sale completion does not preserve the sale box ERG there. |
| `recipes/savings-cap.es` | `unconstrained-outputs` | 2 | benign | An owner-authorised capped withdrawal preserves the remainder; a joint-key closure intentionally permits release. |
| `recipes/savings-cap.es` | `successor-field-drift` | 20 | benign | R4 intentionally becomes HEIGHT for the new withdrawal period; the old height gates eligibility. |
| `recipes/subscription.es` | `unconstrained-outputs` | 2 | benign | Periodic payment and remaining-pot bounds exist; authorised final payment/cancellation need no fixed tail. |
| `recipes/subscription.es` | `successor-field-drift` | 41 | benign | R4 intentionally becomes HEIGHT on a periodic claim; old R4 is used only for the delay check. |
| `recipes/token-sale.es` | `unconstrained-outputs` | 28 | benign | Payment is tied to sold stock and the partial-sale successor; buyer receipts and change occupy other outputs. |
| `recipes/token-sale.es` | `successor-field-drift` | 81 | benign | Remaining stock is compared through an if expression in sold/paid accounting; the bounded reserve helper misses that relationship. |
| `recipes/vesting.es` | `unconstrained-outputs` | 5 | benign | The beneficiary authorises withdrawals and the unvested remainder is bounded; fee/change is left to the transaction. |
| `rosen-bridge/Collateral.es` | `unconstrained-outputs` | 1 | benign | Collateral/repository token deltas and watcher identity govern partial return or closure; other flow outputs vary. |
| `rosen-bridge/Collateral.es` | `successor-field-drift` | 69 | benign | Collateral token identity and repository accounting determine the flow; this local id-only slot check does not establish issuance supply. |
| `rosen-bridge/Commitment.es` | `unconstrained-outputs` | 16 | benign | Commitment aggregation/redemption uses filtered permit outputs and event accounting, beyond a fixed tail policy. |
| `rosen-bridge/Commitment.es` | `successor-field-drift` | 378 | benign | Token continuity can lead back to a permit with a different script; commitment R5/R6/R7 are deliberately not continued as commitment state. |
| `rosen-bridge/Emission.es` | `unconstrained-outputs` | 14 | benign | Swap token accounting or guard-signature update controls the flow; output count is not its invariant. |
| `rosen-bridge/EventTrigger.es` | `unconstrained-outputs` | 29 | benign | Filtered reward count, token amounts and WID digest are checked; the universal check covers rewards rather than every output. |
| `rosen-bridge/Fraud.es` | `unconstrained-outputs` | 18 | benign | The fraud flow matches repository RWT gain and requires cleanup/repository inputs; accounting is delegated across scripts. |
| `rosen-bridge/GuardSign.es` | `unconstrained-outputs` | 3 | benign | A SELF-rooted signature threshold authorises a guard update; arbitrary update transaction outputs are intentional. |
| `rosen-bridge/GuardSign.es` | `successor-field-drift` | 39 | benign | SELF-rooted guard signatures authorise replacing the guard set and thresholds; this is an intentional configuration update. |
| `rosen-bridge/Permit.es` | `unconstrained-outputs` | 1 | benign | Permit aggregation/return and commitment token accounting use specific recipients; they do not cap all transaction outputs. |
| `rosen-bridge/Permit.es` | `successor-field-drift` | 101 | benign | Returned permits aggregate token quantities across inputs, so their ERG value need not relate to this one SELF. |
| `rosen-bridge/Permit.es` | `successor-field-drift` | 152 | benign | Permit splitting preserves the permit token lineage while allowing ERG funding to be allocated across the transaction. |
| `rosen-bridge/Permit.es` | `successor-field-drift` | 162 | benign | Token-id continuity selects a newly created commitment, whose new script and state are deliberately different from a permit. |
| `rosen-bridge/RepoConfig.es` | `unconstrained-outputs` | 7 | benign | Guard-NFT-selected signatures authorise reconfiguration while preserving its token; funding/change is outside this policy. |
| `rosen-bridge/RepoConfig.es` | `successor-field-drift` | 27 | benign | Guard-authorised configuration replacement preserves the configuration token; it does not require the old ERG amount locally. |
| `rosen-bridge/RwtRepo.es` | `unconstrained-outputs` | 1 | benign | Repository, collateral and permit transitions constrain their own accounting; unrelated transaction outputs remain open. |
| `vectors/mutable-upgrade-digest/fixed.es` | `unconstrained-outputs` | 1 | true-positive | This authored upgrade fixture checks continuation or the upgrade digest, with no total-output/tail guard; safety of either path is not inferred. |
| `vectors/mutable-upgrade-digest/vulnerable.es` | `unconstrained-outputs` | 1 | true-positive | This authored upgrade fixture checks continuation or the upgrade digest, with no total-output/tail guard; safety of either path is not inferred. |
| `vectors/mutable-upgrade-digest/vulnerable.es` | `successor-field-drift` | 15 | true-positive | The authored continuation omits the R4 upgrade-digest relationship; its fixed counterpart supplies it. |
| `vectors/rounded-payment/fixed.es` | `unconstrained-outputs` | 13 | benign | This authored fixture isolates payment rounding; a payment bound does not declare a closed output tail. |
| `vectors/rounded-payment/vulnerable.es` | `unconstrained-outputs` | 11 | benign | This authored fixture isolates payment rounding; a payment bound does not declare a closed output tail. |
| `vectors/shared-payment-across-instances/fixed.es` | `unconstrained-outputs` | 1 | benign | This authored fixture isolates shared-payment composition; recipient payment is checked while funding/change outputs remain open. |
| `vectors/shared-payment-across-instances/vulnerable.es` | `unconstrained-outputs` | 1 | benign | This authored fixture isolates shared-payment composition; recipient payment is checked while funding/change outputs remain open. |
| `vectors/successor-register-drift/fixed.es` | `unconstrained-outputs` | 4 | benign | This authored fixture isolates register carry-forward and preserves successor reserves; external fee/change remains open. |
| `vectors/successor-register-drift/vulnerable.es` | `unconstrained-outputs` | 1 | benign | This authored fixture isolates register carry-forward and preserves successor reserves; external fee/change remains open. |
| `vectors/successor-register-drift/vulnerable.es` | `successor-field-drift` | 13 | true-positive | The authored successor omits the SELF-read R4 relationship while preserving script/value/tokens; the control restores it. |
| `vectors/unauthenticated-context-code/vulnerable.es` | `unauthenticated-code-execution` | 0 | true-positive | The authored executeFromVar(1) has no byte/digest authentication; its control pins those same variable bytes. |
| `vectors/zero-threshold/vulnerable.es` | `trivial-sigma-branch` | 0 | true-positive | The authored zero-threshold example retains atLeast(0, ...); its control requires a positive threshold. |

### Unanalysed files

All ten are retained as failed records, never counted as clean:

| File | Recorded compile failure |
| --- | --- |
| `chaincash-basis/chaincash/layer2-old/reserve.es` | parse error: syntax error at offset 1126: expected = |
| `curve-trees/CurveTreeVerifier-v6.es` | tree not serializable under a v0 header: UnsignedBigInt constant data |
| `hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodltoken_proxy.es` | parse error: syntax error at offset 7124: expected else |
| `lsp/examples/completion_demo.es` | parse error: Block should contain a list of Val bindings and one expression (offset 768) |
| `lsp/examples/eip5-multi-sig.es` | parse error: syntax error at offset 395: expected literal default value |
| `lsp/examples/eip5-payment-channel.es` | parse error: syntax error at offset 548: expected literal default value |
| `lsp/examples/hover_demo.es` | parse error: Block should contain a list of Val bindings and one expression (offset 1577) |
| `lsp/tests/main.test.es` | parse error: syntax error at offset 0: expected expression |
| `lsp/tests/simple.test.es` | parse error: syntax error at offset 0: expected expression |
| `lsp/tests/template.test.es` | parse error: syntax error at offset 0: expected expression |

## I04 sweep — 2026-09-14

`cargo run --release -p ergo-sandbox --example audit_sweep` exited **0**, using
this batch's `$CARGO_TARGET_DIR`. The final [raw sweep](reports/batch-7/audit-sweep.json)
and [site review](reports/batch-7/site-review.json) cover **124 files: 114 complete,
0 partial, 10 compile failures**. This is the bundled corpus, including authored
vectors and recipes; the spec's historical 79 is not this denominator. The same
ten compile failures and reasons recorded in S02 above remained unanalysed.
No deployment identity or transaction acceptance was established by this sweep.

The sibling `upgrade-hook` leaves all nine existing lints unchanged. It recognises
direct output bytes or blake2b256 comparisons to SELF/input/data-input registers,
then looks for same-script continuations with no same-typed-register equality on
that path. Result aliases, conjunctions, disjunctions, allOf/anyOf and if branches
are bounded to 64 alternatives and depth 128; unsupported/computed boxes and
over-cap expressions remain undecided. Named sigma guards are syntactic authority
requirements, not proof of reachability, signing ability or approval.

Ship upgrade-hook at LOW review priority for same-script continuations only; token transfers and unrecognised continuations remain undecided.

The [initial sweep](reports/batch-7/initial-audit-sweep.json) reported two sites.
[Every initial site was reviewed](reports/batch-7/initial-site-review.json).
Rosen `Commitment.es` node 402 was a benign commitment-to-permit redemption: R7
selects the permit script, and X-RWT moves to the permit. Token identity alone
did not establish a continuing commitment or a rewritten future upgrade hook.
The recogniser was narrowed to same-script continuations, with a negative test
for token transfer to a hook destination. The final sweep reports only the
authored pair below. No precision-regression stop was needed after narrowing.

| Contract | Lint | Node | Outcome | Review reason |
| --- | --- | ---: | --- | --- |
| `vectors/mutable-upgrade-digest/vulnerable.es` | `upgrade-hook` | 8 | true-positive | The authored same-script continuation preserves value but omits R4 equality; the alternative selects code by SELF.R4. No sigma guard is recognised on the continuation. The fixed source carries R4 on that path and does not report. This is a true-positive static pattern, not a validated upgrade. |
