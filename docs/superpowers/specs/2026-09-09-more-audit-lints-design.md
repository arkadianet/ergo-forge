# Height guards and data-input trust review

Date: 2026-09-09
Scope: two additional static lints over the existing lifted AST.

The P3a design record defines the framework and `unchecked-get`, including the
High/Medium/Low vocabulary, but explicitly leaves additional lints out of scope.
Item 4 of `workbench-PLAN.md` names height guards, anyOf shadowing and trust
assumptions without specifying their semantics. This record supplies deliberately
narrow rules for two of those names. Findings describe review obligations, as
`delegated-reserves` does; none establishes exploitability.

## `height-guards`

Two rules:

1. **Low:** the result, or a direct disjunct of the result, consists only of
   `HEIGHT > h` or `HEIGHT >= h` (including reversed comparisons), where `h` is
   a nonnegative integer literal below `Int.MaxValue`. Follow aliases, block
   results, `sigmaProp`, binary `||`, and literal `anyOf` collections. The
   alternative adds no authorisation or expiry requirement after that height.
   A permissionless release can be intentional; this is informational.
2. **Medium:** an `if` condition contains a comparison with a direct HEIGHT
   operand, and the two branch expressions render identically after resolving
   their outer aliases. The height does not select different requirements.
   This does not remove the condition: evaluating it may still throw.

The first rule only follows result alternatives; it does not flag every lower
bound nested under `&&`. In particular, an owner-authorised timelock does not
need an upper bound. The second rule scans the whole tree and is local to each
conditional; an enclosing signature or another time check can still constrain
the spend. Expressions hidden inside function bodies are not beta-reduced.

Positive corpus cases:

- `basics/height-lock.es`: an intentional permissionless release after
  `$unlockHeight`, one Low finding with the fixture's height 1000.
- `crystalpool/deposit.es`: both sides of the height conditional call
  `getSellerPk(SELF)`. The pool-key requirement in the early branch is commented
  out in the source. One Medium review obligation; not a claim about intent.

Negative corpus cases:

- `recipes/time-lock.es`: the owner signature remains required after unlocking.
- `recipes/htlc.es`: distinct receiver/secret and sender/refund paths.
- `crystalpool/sell-token-for-erg.es`: the pre-expiry branch retains the pool
  signature and the seller-or-filled-order condition; outcomes are distinct.

Limits: no inference of expected deadlines from names or application intent;
no arithmetic solving, dynamic-bound release analysis, conjunction distribution,
general tautology proof, or semantic branch equivalence. Negation is not analysed
by this lint, although compiler normalization can expose a supported comparison.
Alias/wrapper reasoning is bounded by the shared `OPERAND_DEPTH` (8). Branches
with raw placeholders, unresolved binding cycles or excessive operand depth
are undecided. A block initializer or condition can still fail, so even a
height-only result is not a successful-spend proof.

## `trust-assumptions`

**Medium:** a positional data input supplies a register through `get` or
`getOrElse`, and nowhere in the tree is there a recognized identity comparison
for that box. Report once per canonical box key, anchored at its first register
extraction. This is specifically a provenance review, not a claim that no
constraint on the register value exists.

Recognized identity evidence:

- A box-id equality, or a literal-index token-id equality (any token slot),
  with a literal or a value rooted in SELF. Aliases, register extraction and
  literal indexing on the anchor are supported.
- `box.tokens.exists(t => t._1 == expected)`, or the reversed equality, with
  the same anchor rule. Both the tuple lambda and the lift's two-parameter
  tuple-unwrapping form are recognized. This matters for Rosen's contracts.

A context variable or another positional box is not an identity anchor. Script
equality alone does not identify a particular box or token lineage. Presence
guards establish a register's existence, not its provider. Value comparisons
and key checks may nevertheless be the intended authentication mechanism;
the finding explicitly asks the reader to review that possibility.

Positive corpus case:

- `chaincash-basis/chaincash/onchain/note.es`: reads the reserve's R4 public key
  from `CONTEXT.dataInputs(0)` and compares it to the note holder. Its token id
  becomes part of the history entry; the note does not compare that id with a
  fixed expected reserve id. One finding asking whether this family of reserves
  authenticated by owner key is intended. The signature and history checks are
  not claimed to be absent or broken. Test parameters instantiate the two
  external contract hashes with distinct fixed values; the source is unchanged.

Negative corpus cases:

- `basics/oracle-price-gate.es`: oracle register R4 is accompanied by the
  expected oracle NFT equality, using the existing gallery fixture parameters.
- `chaincash-basis/chaincash/onchain/reserve.es`: gold oracle R4 is accompanied
  by its literal NFT equality.
- `rosen-bridge/Lock.es`: guard registers R4/R5 are accompanied by a token
  `exists` predicate. The test substitutes a fixed 32-byte base64 NFT for
  `GUARD_NFT`, leaving the predicate and the rest of the contract intact.

Limits: whole-tree absence analysis, not branch-sensitive enforcement. An
identity equality in just one branch, or under negation, can suppress a finding.
Tests explicitly record that limitation. Token singleton supply, freshness,
register validity, signatures, AVL proofs and transitive authentication are
undecided. Computed indices, searched boxes, dynamic register access, compound
`exists` predicates, transitive identity equalities and unsupported wrappers
can be missed or reported despite alternative authentication. Box keys use the
shared positional recognizer, including its literal/context-variable forms.
Index expressions outside that recognizer are skipped. Only extracted data-input
registers are covered; an existence check alone does not trigger the lint.

## Deferred catalogue scope

**anyOf shadowing:** not shipped. Syntactic absorption such as `a || (a && b)`
is implementable, but inspection of the corpus's executable disjunctions did
not establish an unmodified real positive. The apparent candidates in
Crystalpool require different seller/payment conditions while sharing the pool
signature. Recipe inheritance/refunds use different keys, and Dexy action
disjunctions select distinct transaction checks. A key-authorised escape path
is not itself subsumption. Shipping only synthetic absorption tests would fail
the requested acceptance criterion. Arbitrary logical implication over mixed
Boolean/Sigma propositions, partial expressions and cryptographic predicates
needs a separate design and corpus evidence.

**General unconstrained registers/context variables:** deferred. These routinely
carry witnesses, action selectors, keys and authenticated data. A comparison
count or a missing range check cannot decide whether a value is taken on faith;
signature/AVL/data-flow reasoning would be needed. The implemented trust rule
states only the specific missing identity evidence it can establish.

## Corpus measurement and review

The sibling `ergo` checkout's 110 accepted compile-seed vectors and 279 unique
mainnet trees were audited through `target-lints/release/ergo-es`. Indices below
are zero-based in the accepted-vector list (not all ACCEPT/REJECT records).

| Corpus | Height trees / findings | Trust trees / findings | Partial trees | Newly flagged trees |
|---|---:|---:|---:|---:|
| Seed, 110 | 9 / 9 | 2 / 3 | 1 | 8 |
| Mainnet, 279 | 0 / 0 | 0 / 0 | 2 | 0 |

No new mainnet findings exist to sample; both new lints are below the original
20% mainnet noise gate. All 12 new seed findings on 11 trees were reviewed:

| Seed index | New finding | Review |
|---:|---|---|
| 0 | HEIGHT > 100 | Accurate informational release policy. |
| 1 | HEIGHT > 5 via alias | Accurate informational release policy. |
| 5 | HEIGHT > 5 | Accurate informational release policy. |
| 11 | Singleton anyOf, HEIGHT > 5 | Accurate release policy; no sibling to shadow. |
| 31 | Data input 0, R4 key | Chaincash old note uses holder-key authentication, without fixed reserve identity; review that intended trust model. |
| 32 | Data input 0, R4 key | Chaincash old redemption compares reserve ids with context-provided ids, coupled to signature/history proofs; fixed identity is absent, authentication remains undecided. |
| 32 | Data input 1, R4 key | Same review obligation for the alternate/holder reserve; no claim that its proofs fail to authenticate it. |
| 38 | Identical height branches | Crystalpool deposit, same seller key before and after the height. |
| 43 | HEIGHT > 100 | Simple corpus height-lock example; accurate informational release policy. |
| 44 | HEIGHT > 100 | Simple corpus height example; accurate informational release policy. |
| 93 | HEIGHT >= 5 | Compiler normalizes `!(HEIGHT < 5)`; accurate informational release policy. |
| 96 | HEIGHT > 5 via alias chain | Accurate informational release policy. |

The three Chaincash findings describe missing fixed-identity evidence, not
unvalidated keys or failed cryptographic authentication. Treating them as
vulnerability verdicts would be false positives; the message deliberately
requires a review of the alternative authentication.

The eight previously clean seed records are informational height-only results.
No existing test's clean assertion was relaxed. The deployed USE extract,
bank and fixed-swap clean assertions remain about `unbound-box-reserves`;
neither new lint introduces findings there. The bank's reserve delegation is
already covered by the pre-existing `delegated-reserves` lint.

The original lint files and shared traversal/box-reference behavior are unchanged.
Corpus findings for existing lints remain: seed unchecked-get 236 and delegated
reserves 1; mainnet unchecked-get 79, unbound-box-reserves 1 and delegated
reserves 2.

Verification, all using `CARGO_TARGET_DIR=./target-lints`:

- `cargo test -p ergo-sandbox --release`: 276 passed, none failed or ignored.
- `cargo clippy --all-targets --release -- -D warnings`: workspace clean.
- `cargo fmt --all` and `cargo fmt --all -- --check`: clean.
