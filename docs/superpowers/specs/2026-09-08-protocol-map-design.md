# Protocol map — discovering a contract set from chain

> Design record, 2026-09-08. The layer beneath the P3a lints and the phase-1
> drain hunt: both reason about a contract *set*, and neither has a way to
> find one. This is a **design doc** — no implementation.

## Context

Auditing the 2026-09-08 USE/Dexy LP drain took one mechanical loop, run by
hand with `ergo-es decompile` and explorer queries:

1. start from a known artifact (the exploit transaction);
2. decompile the tree of a box it spent;
3. read the 32-byte `Coll[Byte]` constants out of that tree;
4. recognise them as *other contracts' NFTs*;
5. fetch the box holding each one, and recurse.

The LP pool's tree yielded the swap, mint, redeem, extract and intervention
NFTs. Arbitrage-mint's yielded the bank, LP, buyback, oracle and tracking
NFTs. The bank's yielded free-mint, arbitrage-mint, intervention, payout and
the update NFT. Thirteen contracts recovered from one seed, purely by
following constants.

That loop is the **map**, and the insight it rests on is worth stating
plainly:

> The NFT constants compiled into an ErgoTree *are* the protocol's dependency
> graph. A contract that must recognise another box has that box's identity
> baked into its own bytes.

The map is what made the drain visible. A single tree cannot show a
composition bug, because the bug is in an **edge**, not a node: the pool
references the swap by NFT, while the swap references the pool by *position*.
Neither tree is wrong alone. The asymmetry is the vulnerability, and it is
only visible holding both nodes and comparing the two directions.

The forge today can fetch one box (`POST /api/v1/lookup`) and decompile one
tree. It cannot follow a reference. Everything above was manual.

## The question

> **Given one artifact of a deployed protocol — an NFT, an address, or a
> transaction — what is the whole contract set, and how does each contract
> identify the others?**

The answer is a graph. Nodes are boxes; edges are "contract A recognises box
B", labelled with *how*. The audit then runs on the edges, not only the nodes.

## Seeds

Any of:

- **A token id** — an NFT the protocol uses. Resolve to the box holding it.
- **An address** — resolve to its unspent boxes, fetched **paginated and
  bounded** by the frontier cap below. A high-volume address can hold more
  boxes than any traversal should admit, so the initial frontier is truncated
  deterministically (canonical order, below) and the truncation is reported
  like any other.
- **A transaction id** — take its inputs and outputs as the initial frontier.

A seed is a starting point, never a trust anchor: everything the chain source
returns is data, re-parsed by the engine, exactly as `lookup` already treats
explorer responses.

## Traversal

Breadth-first from the seed frontier. For each box:

1. **Decompile** its ergoTree to the lifted AST (existing decompiler).
2. **Extract every constant** and classify it (below).
3. For each constant classified as a **token id**, resolve it against the
   chain. A token may have **many holders**, so selection is defined before
   the cap applies: fetch holders paginated, order them canonically by
   `(inclusionHeight, boxId)` ascending, then take the first `max-boxes-per-token`.
   The order is a property of the map, never of the chain source's response
   order, so two runs against the same chain state select the same boxes.
   Each selected box is a node and an **edge** from this contract to it is
   recorded. Truncation is reported **per token id**, not only in the map's
   overall result — omitting a holder can omit the box actually at risk.
4. For each constant classified as a **script hash**, resolve it two ways:
   - against trees already in the map (closed on a second pass); and
   - where the chain source supports a script/template-hash query, by a
     **bounded lookup** subject to the same canonical ordering and per-hash cap.

   A script hash that neither resolves is recorded as an edge to an
   **`unresolved`** target. Such an edge is kept — the reference is real and
   worth reporting — but the map is **not** complete, and unresolved edges are
   excluded from any completeness claim. A collaborator referenced only by
   script hash must never silently vanish from the set.
5. Recurse until no new nodes, or until the caps below bite.

Caps, recorded per run and never silent: maximum nodes, maximum depth,
maximum boxes per token id, maximum boxes per script hash, and the initial
frontier size. A truncated map is reported as truncated, in the same spirit as
the audit's `Completeness::Partial`.

## Constant classification

A 32-byte `Coll[Byte]` constant is one of:

- **A token id** — a token with that id exists on chain. (Singleton NFTs,
  amount 1, are the strong case; the emission amount is recorded so a
  non-singleton reference is visible as such.)
- **A script hash** — it equals `blake2b256(propositionBytes)` of some tree in
  the map. Contracts that pin a collaborator by script rather than by NFT use
  this form; the Aegis-USE sidechain drafts do exactly that
  (`RECEIPT_SCRIPT_HASH`, `FEE_POT_SCRIPT_HASH`).
- **Opaque** — neither resolves. Recorded, not followed.

Classification is *evidence-based*, never by shape alone: a 32-byte constant
is only a token id if the chain says a token with that id exists. The chain
source and the response are archived so the classification is re-derivable.

### What counts as a *protocol NFT*

The role rules below depend on this term, and boxes routinely carry unrelated
tokens — the drained USE pool's own holders carry RSN, DORT and airdrop dust,
and the attacker's decoy carried three junk tokens precisely to satisfy
positional `tokens(i)` reads. So the term is defined by evidence, with an
explicit precedence:

A token id is a **protocol NFT** iff both hold:

1. it appears as a **constant in at least one mapped tree** (the protocol
   itself names it); and
2. its **emission amount is 1** — a singleton, so holding it identifies one box.

A token id that is referenced but not a singleton is recorded as a
**referenced token**, not a protocol NFT; it may still carry value and appear
in edges, but it cannot establish identity.

A *box's* protocol NFT is: the token at `tokens(0)` if that is a protocol NFT
(the singleton convention every deployed set here follows), otherwise the
lowest-indexed token that is one. Unrelated tokens at any index are ignored
for identity and reported separately. A box with no protocol NFT has none —
the map does not guess.

## Edge typing — the point of the exercise

For each edge "contract A recognises box B", record **how A establishes B's
identity**, by inspecting A's tree at the site of the reference:

| Binding | Shape in A | What it actually pins |
|---|---|---|
| `nft` | `B.tokens(0)._1 == <NFT constant>` | A **token id**. It pins a unique *box* only with singleton evidence (emission amount 1, per the definition above); against a non-singleton token any holder qualifies. Recorded as `nft(singleton)` or `nft(fungible)`. |
| `script-hash` | `blake2b256(B.propositionBytes) == <hash constant>` | B's **code**. Any box with that script qualifies. |
| `self-successor` | `B.propositionBytes == SELF.propositionBytes` | B's **script bytes only** — *not* its value, tokens or registers. Another box with the same script satisfies it. |
| `positional` | A reads `INPUTS(n)` / `OUTPUTS(n)` with no identity check | **Nothing.** Whoever builds the transaction chooses what sits there. |
| `data-input` | B is read via `CONTEXT.dataInputs(n)` | Typed by the same rules; a data input pinned only by position is equally unpinned. |

No binding is treated as unique box identity by default. The map records
**which identity dimensions a binding covers** — script, token id, value,
registers — and leaves the rest open, because the uncovered dimensions are
where this bug class lives: the USE pool checked its successor's script and
token *ids* and never its token *amounts*, which is exactly how the reserves
walked out of a box that looked correctly pinned.

An edge may carry more than one binding; the strongest one wins for scoring,
and the covered dimensions are the union.

**The set-level finding** is an edge typed `positional` where A does value
arithmetic on B — the graph form of the `unbound-box-reserves` lint, and the
exact shape of the USE drain. The single-tree lint sees "this tree reads an
unpinned box"; the map additionally answers "and that box is the pool holding
284,695 ERG", which is what turns a lint into a severity.

**Asymmetry** is worth reporting on its own: A pins B by NFT while B pins A
only positionally means the protocol's authors intended a pairing that one
side does not enforce. Every deployed Dexy LP action pair has this shape.

## Role inference — what the drain hunt needs

The phase-1 drain hunt requires the caller to label every input
`protected` / `companion` / `attacker` / `external`, and refuses to guess.
The map is what stops that being hand work. From the graph:

- **`protected`** — a node whose **protocol NFT** (as defined above) is
  present **and** which holds value — ERG, or referenced tokens other than
  that NFT — that other contracts do arithmetic on.
- **`companion`** — a node with a protocol NFT, holding only dust, whose
  script is the authorisation; typically also `movableByAnyone` under the
  spend hunt.
- **`external`** — an oracle or tracker: a node read for its *data* rather
  than spent. Evidence is incoming `data-input` edges, which may coexist with
  `nft` edges — the USE oracle is both pinned by NFT and read as a data input,
  so a data-input edge is sufficient evidence, not a requirement that no other
  binding exists.
- **`unknown`** — anything the rules do not settle. Emitted as `unknown` and
  never silently defaulted; the caller resolves it.

Roles are a *proposal*, always overridable. The hunt still refuses to run on
an `unknown`.

## Output

One JSON document, **byte-identical** for a given chain state and caps. Two
rules make that true rather than aspirational:

- **No run metadata inside the canonical artifact.** `chainSource` carries only
  what the map depends on (`kind`, `url`, `height`). The wall-clock
  `fetchedAt`, durations and any request ids live in a separate `run` block
  that is explicitly **non-canonical** and excluded from comparison.
- **Every array has a canonical sort key**: `nodes` by `boxId`; `edges` by
  `(from.boxId, to.boxId | targetHash, binding, site)`; `findings` by
  `(severity, lint, from.boxId, to.boxId)`; `tokens` within a node by token id.

```text
{ "seed": …,
  "chainSource": { "kind": …, "url": …, "height": … },
  "run": { "fetchedAt": …, "durationMs": … },          // non-canonical
  "nodes": [ { "boxId", "nft", "treeHash", "value", "tokens",
               "role": "protected|companion|external|unknown", "complete": bool } ],
  "edges": [ { "from": <node>, "to": <node> | { "unresolved": <hash> },
               "binding": "nft|script-hash|self-successor|positional|data-input",
               "covers": ["script","tokenId","value","registers"],
               "valueMath": bool, "site": <snippet> } ],
  "findings": [ … set-level lints … ],
  "truncated": { "nodes": n, "depth": d, "frontier": n,
                 "perToken": { <tokenId>: n }, "perScriptHash": { <hash>: n } } | null }
```

Rendering (CLI table, and later a graph in the Read pane) is a view over this;
the JSON is the artifact, consumable by the drain hunt and the sentinel.

## Chain source

Pinned, versioned, and archiving, per the drain-hunt spec's sentinel
precondition. The source is declared in the output (`chainSource`), raw
responses are archived alongside a run, and the height is recorded so a map is
reproducible. Reuses the existing `EXPLORER_URL` configuration and the
`lookup` route's treat-everything-as-data discipline. A node with the
blockchain indexer is an equally valid source; the GraphQL service used
manually during incident response is a candidate, but it is a service the
forge *calls*, never a capability it assumes.

## Acceptance criteria

1. **Rebuild the USE set from one seed.** Given only the LP NFT
   `4ecaa1aa…` (or the exploit transaction id), the map must recover the
   contract set found by hand: pool, swap, mint, redeem, extract,
   intervention, bank, free-mint, arbitrage-mint, payout, buyback, oracle,
   tracking, and the update NFT — with each edge typed.
2. **Find the asymmetry unaided.** The pool→swap edge must type `nft`, and
   the swap→pool edge must type `positional` with `valueMath: true`, raising
   the set-level finding. Mint and redeem must raise it too; extract must
   not (it pins the LP by NFT). This is the same discrimination the
   `unbound-box-reserves` lint makes per tree, now attributed to the box
   actually at risk.
3. **DexyGold.** Seeded with its LP NFT `905ecdef…`, the map must recover
   that set and raise the same finding — the twin deployment.
4. **Negative control.** The gallery pair
   (`examples/contracts/protocols/amm/`) maps with every edge `nft`-bound and
   raises no set-level finding.
5. **Roles feed the hunt.** The map's proposed roles for the USE set must be
   exactly the labelling the drain-hunt phase-1 fixture uses by hand: pool
   `protected`, swap `companion`, oracle/tracking `external`.
6. **Determinism and honesty.** Same chain state and caps → **byte-identical**
   canonical JSON (the non-canonical `run` block excluded), independent of the
   chain source's response ordering. Truncation is reported — overall, per
   token id, and per script hash — and never silent. Unresolved script-hash
   edges are present in the output and excluded from completeness claims.

## Integration

- `ergo-sandbox/src/map.rs`, reusing the decompiler, the constant table, the
  audit layer for set-level findings, and a chain-source trait so the explorer,
  a node indexer, or a recorded fixture are interchangeable (fixtures are what
  make the acceptance criteria run in CI without network).
- CLI: `ergo-es map <seed> [--depth N] [--max-nodes N]`, JSON via the `--json`
  convention.
- Web: `POST /api/v1/map` later, and a graph view in the Read pane. Not first
  cut.
- The drain hunt consumes the map's roles; the sentinel is "map from every
  seed, nightly, then hunt". Both are downstream and neither is in scope here.

## Honest limits

- **Dynamic references are invisible.** A contract that finds a collaborator
  by scanning (`INPUTS.exists { … }`) or by an index computed at runtime has
  no constant to follow. The map records the site as an unresolved reference
  rather than pretending the edge does not exist.
- **A map is of *now*.** It reflects one chain height. A protocol that
  rotates boxes needs re-mapping; the height is in the output for that reason.
- **Reachability is not completeness.** Contracts a protocol never references
  from a mapped tree — an off-chain-assembled collaborator, a not-yet-deployed
  action — are not found. Absence from a map is not absence from the protocol.
- **Classification can be wrong.** A 32-byte constant that coincidentally
  matches a token id is recorded as an edge that is not one. Evidence is
  archived so a human can overrule it.
- A map is a description, not a verdict. It finds no bug by itself; it makes
  the set-level lints and the drain hunt possible.

## Responsible disclosure

Mapping is run against **third-party deployments** by design — that is the
point of a sentinel. The disclosure rules in the drain-hunt spec apply
unchanged and are inherited here: findings on contracts the operator does not
own route privately to the affected team; the default output of a sweep is a
private report, never a feed; no timeline or attribution claims travel with a
finding. A map alone is public chain data, but a map *plus a set-level
finding* is a vulnerability report, and is treated as one.

## Out of scope, recorded

- The sentinel sweep itself (scheduling, scoring, alerting) — its own spec.
- Graph rendering in the UI — after the JSON is stable.
- Following dynamic/computed references — needs symbolic reasoning, not
  traversal.
- Cross-protocol maps (one graph spanning several protocols) — the seed
  defines the boundary for now.
