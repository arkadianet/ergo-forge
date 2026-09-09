# ergo-web

HTTP service for the ErgoScript workbench: **paste a contract address or
ErgoTree hex, get the contract back as readable ErgoScript, plus static audit
findings** about the code that will actually execute.

Built on `ergo-sandbox` (decompiler + audit lints). Stateless — no database,
nothing shared between requests, and **no outbound network calls unless you
set `EXPLORER_URL`** (addresses decode locally; only chain lookups you ask for
contact the explorer).

## Running

```bash
cargo run -p ergo-web --bin ergo-web
# → listening on 127.0.0.1:8080; open http://127.0.0.1:8080/
```

Configuration is environment-only, no config file:

| Variable   | Default          | Meaning                                             |
|------------|------------------|-----------------------------------------------------|
| `BIND_ADDR`| `127.0.0.1:8080` | Socket the server binds                             |
| `UI_DIR`   | `ui`             | Static folder served for non-API paths (repo root's `ui/` when run from the workspace root) |
| `EXAMPLES_DIR` | `examples/contracts` | Example `.es` files for the gallery |
| `RATE_LIMIT_PER_MINUTE` | unset | Per-client budget for the engine routes (burst = the same number). Unset = no rate limiting; set it on a public instance. Over budget → `429 rate_limited` with `Retry-After` |
| `TRUST_PROXY` | unset | `1` to take the client address from the last `X-Forwarded-For` entry (only behind a proxy you control) |
| `EXPLORER_NETWORK` | `mainnet` | Which network `EXPLORER_URL` serves; the UI converts dates to heights only for that network |
| `EXPLORER_URL` | unset | Base URL of an Ergo explorer API (e.g. `https://api.ergoplatform.com`). **The one outbound dependency.** Unset = no outbound calls, `/api/v1/lookup` answers 501 |
| `RUST_LOG` | `info`           | `tracing` filter (method, path, status, duration are logged; never request bodies) |

Shutdown is graceful on SIGTERM/Ctrl-C so deploys do not cut live requests.

### Container

```bash
docker build -t ergo-web .            # from the repo root; multi-stage, --locked
docker run --rm -p 127.0.0.1:8080:8080 ergo-web
```

The image runs the release binary as an unprivileged user, binds `0.0.0.0:8080`
inside the container, and carries a health check against `/api/v1/health`.
Put TLS and per-IP rate limiting in front of it. Roughly 86 MB.

Pushing a `v*` tag builds and publishes the image to
`ghcr.io/arkadianet/ergo-web:<version>` (`.github/workflows/release.yml`).

## API

Versioned under `/api/v1/`. All responses and errors are JSON; every field is camelCase.

### `GET /api/v1/health`

```json
{"status":"ok","version":"0.1.0"}
```

### `POST /api/v1/inspect`

Both `inspect` and `compile` answer with `plain` — the contract in words,
one sentence per way to spend ("the key 9fSgJ7…HjAV, if from block
900000") — and `plainComplete`, false when a clause the recognizer does
not know was quoted as code instead of paraphrased.

`input` accepts a P2S address or raw ErgoTree hex (hex is tried first; the two
cannot collide). `network` is optional: absent means `mainnet`; the only
accepted values are exactly `mainnet` and `testnet`, anything else is a 400
(a silently defaulted network would produce a wrong address, not an error).

Real example against a known fixture — an unguarded register read:

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/inspect \
  -H 'content-type: application/json' \
  -d '{"input":"1001040ad191e4c6a704047300"}'
```

```json
{
  "treeHex": "1001040ad191e4c6a704047300",
  "address": "8NJuqcG7SdhX7cFKGBmfAkXn",
  "source": "SELF.R4[Int].get > 5",
  "completeness": "complete",
  "rawPlaceholders": 0,
  "truncated": false,
  "findings": [
    {
      "lint": "unchecked-get",
      "severity": "high",
      "nodeId": 2,
      "message": "Option.get with no isDefined guard — …",
      "snippet": "SELF.R4[Int].get"
    }
  ]
}
```

The guarded variant (`1001040ad801d601c6a70404d1ede6720191e472017300`) returns
`"findings": []`.

### `POST /api/v1/hunt`

The spend hunt: **can someone with no key spend this box?** Same `input` /
`network` as inspect, plus optional `height` (base spending height, default
near the mainnet tip), `selfBox` (the spent box in the scenario-JSON box
shape: `value`, `tokens`, `registers`, `creationHeight`) and `dataInputs`
(an array of boxes, each with an `ergoTree` — the oracle boxes a contract
reads; on-chain facts, so supplying them keeps the "anyone" question honest). Six probes — three
heights × an attacker output that takes the funds / a preserve output that
copies SELF — each a full consensus reduction with no proof and no context
variables.

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/hunt \
  -H 'content-type: application/json' \
  -d '{"input":"1001040ad191e4c6a704047300",
       "selfBox":{"value":1000000,"registers":{"R4":{"type":"Int","value":9}}}}'
```

```json
{
  "rent": { "boxBytes": 66, "feeNanoerg": 82500000, "periodBlocks": 1051200, "feeFactor": 1250000 },
  "treeHex": "1001040ad191e4c6a704047300",
  "address": "8NJuqcG7SdhX7cFKGBmfAkXn",
  "verdict": "spendableByAnyone",
  "residuals": [],
  "selfSynthetic": false,
  "probes": [
    {"height":1500000,"output":"attacker","verdict":"pass","reducedTo":"true","error":null,"cost":…},
    …
  ]
}
```

Legacy wire verdicts are retained: `spendableByAnyone` means an attacker-output
sample passed without a proof; `movableByAnyone` means a preserving-output sample
passed. Neither establishes a valid canonical transaction. `requiresProof`
records distinct residual propositions observed under the probes, not every
possible spender; `notUnderProbes` is not a safety claim. `selfSynthetic` applies
to positive and negative results alike. Every response labels method/provenance
and `nodeValidated: false`; the browser keeps the synthetic warning visible.

### `POST /api/v1/compile`

The write side of the playground: source (+ compile-time parameters) →
tree, addresses, the tree decompiled back to source, and findings.

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/compile \
  -H 'content-type: application/json' \
  -d '{"source":"sigmaProp(HEIGHT > $minHeight)",
       "params":{"minHeight":{"type":"Int","value":100}}}'
```

`params` map a name to a typed value (the scenario typed-value shape). A
`SigmaProp` value may be a 33-byte pubkey hex or a P2PK address (mainnet or
testnet); a script address is refused, since it is not a key. An
EIP-5 `@contract def` template source takes the template path: it is
compiled and instantiated through the compiler's `ContractTemplate::apply`,
declared defaults filling any parameter not given (`template: true` in the
response; `params[].default` carries the declared default).

Findings on the compile route carry `offset`, `line` and `col` into the
submitted source when the compiler's source map (P5-B) aligned with the
tree (`positioned: true`). Templates are not positioned yet. Offsets inside
a string literal that was substituted may drift by the substitution's
length difference; everything before the first substituted literal is
exact.
`$name` and bare identifiers resolve through the compiler's environment;
`"$name"` and all-caps tokens inside string literals are substituted
textually (`String` params as given, `Coll[Byte]` as hex). The response
lists every parameter the source uses with `supplied: true/false`.

Errors: `missing_params` (400, with `missingParams: [{name, typeHint}]` so
a form can be built), `compile_error` (400):

```json
{"error":{"code":"compile_error","message":"compile failed: …","phase":"type","offset":10,"offsetSource":"source"}}
```

`phase` is `parse`, `bind`, `type`, `root`, `emit`, `serializer`, or `write`.
`offset` preserves the compiler's 0-based UTF-8 byte offset; it is a start,
not a range. `offsetSource` tells clients whether it can locate that start:

- `source`: an offset in the submitted source, including a genuine zero or EOF.
- `unavailable`: a post-typecheck phase; its zero is a sentinel, not a location.
- `substitutedSource`: quoted parameters may have been replaced. The public
  sandbox API has no origin map, so the editor conservatively withholds a
  marker even when a particular replacement might not shift the position.

The Write editor places a point marker and collapsed caret for `source`,
with the phase and line/column shown. Editing source, parameters or network
clears the diagnostic; stale compile responses cannot mark changed source.
`POST /api/v1/test` compile errors use the same metadata.

### `GET /api/v1/examples`, `GET /api/v1/examples/{id}`

The gallery: `[{id, group, name}]`, then `{id, group, name, source, params,
template, doc}` for one — `params[]` carry `typeHint`, `default` and, for
templates, the `@param` `description` (the question a form asks); `doc` is
the template's name and description. `GET /api/v1/examples/{id}.es` returns
the raw source as `text/plain`. The `recipes/` group is the Build mode's
library: EIP-5 templates whose docs are written for non-technical users. Files under `EXAMPLES_DIR` (default `examples/contracts`;
86 files — 7 basics plus the node corpus's 79 real deployed contracts, see
`examples/contracts/ORIGIN.md`). `template` marks EIP-5 `@contract def`
sources, which cannot be parameterised yet.

### `GET /api/v1/config`, `POST /api/v1/lookup`

`config` → `{"explorer": true|false, "height"?: n, "network"?: "mainnet"|"testnet"}`
(`height` is the chain tip when an explorer is configured, so a form can
turn a date into a height — for that `network` only). When an explorer is configured,
`lookup` `{input, limit?}` fetches a box by id (64 hex) or an address's
unspent boxes, and the current height, in the scenario box shape — registers
are passed through as `{"type": "raw", "value": "<serialized constant hex>"}`
and re-parsed by the engine, so a real box can be the hunt's `selfBox` or a
`dataInputs` entry without retyping anything. Without `EXPLORER_URL` the
route answers `501 not_configured`; the UI hides the feature.

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/lookup -H 'content-type: application/json' \
  -d '{"input":"<address or box id>"}'
```

Explorer failures are `502 upstream`; an unknown box or address is `404`.

### `POST /api/v1/validate-tx`

Does this unsigned transaction pass selected preflight checks? Full node validation has not run. `{tx, boxes?, height?, network?}`
where `tx` is the node-format transaction (`inputs` with optional
`extension`, `dataInputs`, `outputs`) and `boxes` the input and data-input
boxes in node/explorer shape. Boxes not supplied are fetched from the
explorer when one is configured (and `height` defaults to the chain tip);
otherwise they are reported `missing`. Every input's script runs with SELF
at its real index, all inputs in order, all outputs, the data inputs and
that input's extension; ERG and token conservation are checked (one new
token may be minted with the first input's id). Signatures are not checked:
an input reducing to a sigma proposition counts as `needsProof` and does not
invalidate. Response: `{method, provenance, nodeValidated: false, preflightPassed, valid, signaturesNeeded, inputs[], problems[],
ergIn, ergOut, height}`.

### `POST /api/v1/compose`

`{spec, params?, run?}` → `{source, params, suite?, results?}`. A `spec` is a
list of spending paths, each `who` (`{anyOne: true}`, `{anyOf: [names]}`,
`{allOf: [names]}`, `{kOf: k, keys: [names]}`) plus `conditions`; names are
parameter names. Paths are OR-ed, a path's conditions AND-ed. The
conditions cover what a script can see of the spending transaction:

| Condition | Meaning |
|---|---|
| `{after: h}` / `{before: h}` | `HEIGHT >= $h` / `HEIGHT < $h` |
| `{afterTime: t}` / `{beforeTime: t}` | the block's timestamp (Unix ms) at or after / before `$t` |
| `{boxAge: n}` | the box has sat here at least `$n` blocks (`HEIGHT - SELF.creationInfo._1`) |
| `{inputCount: n}` / `{outputCount: n}` | `INPUTS.size == $n` / `OUTPUTS.size == $n` |
| `{payTo: {key, amount}}` | the next output slot pays `$key` at least `$amount` |
| `{keepHere: {atLeast}}` | the next output slot stays under this contract with at least `$atLeast` |
| `{sumPaidTo: {key, atLeast}}` | outputs to `$key` add up to at least `$atLeast` (a fold) |
| `{oracleAbove: {nft, floor}}` | data input 0 carries `$nft` and its `R4[Long] >= $floor` |
| `{tokenGated: {tokenId}}` | some input carries `$tokenId` (a membership token) |
| `{varEquals: {index, type, value}}` | the spender attaches context variable `index` equal to `$value` |
| `{hashPreimage: {var, hash, algo?}}` | `blake2b256`/`sha256` of context variable `var` equals `$hash` |
| `{minerIs: m}` | `CONTEXT.minerPubKey == $m` |
| `{tokenConserved: {id}}` | the outputs carry exactly as much of `$id` as this box does (none burned, none conjured) |
| `{box: rule}` | a rule on one box, below |

A **box rule** names a box — `which`: `output`, `input`, `dataInput` or
`self`; `index`: a number, `"any"` (exists) or `"all"` (forall); an output
with no index takes the next free slot — and requires any of: `script`
(`"self"` for this same contract, or `{key}`), `valueAtLeast`,
`valueAtLeastShare: {percent}` (of `SELF.value`), `token: {id, atLeast?}`,
`noTokens`, `keepsSelfTokens` (`tokens == SELF.tokens`),
`valueAtLeastSelfMinus` (`value >= SELF.value - $x`), `mints: {atLeast?}`
(the box's first token is named after the first input's id, as EIP-4
requires), and `registers`
(`[{reg: "R4".."R9", type, op, value?}]` with `op` one of `eq`, `ne`,
`gte`, `lte`, `eqHeight`, `eqSelf`). Registers are read with
`.isDefined` guards, so a missing register refuses rather than errors.

The source is readable ErgoScript with `$name` params. With `params`
values, a test suite is generated whose expected verdicts come from the
composer's own model of the rules — running it (`run: true`, or `ergo-es
test`) checks that the assembled ErgoScript means what the spec says: one
case per path with everything met, one per condition violated (the most
specific requirement of a box rule), and a baseline. `spec.witness`
(`{"0": {type, value}}`) supplies context variables the checks need but the
contract must not contain, such as the secret behind a `hashPreimage`. A
path that anyone may take with no conditions, an empty box rule, and a
path whose conditions contradict each other (no single transaction can
meet them) are refused with a message.

### `POST /api/v1/play`

`{height, boxes, tx: {inputs: [{boxId, contextVars?, secrets?, parties?}],
dataInputs?: [boxId], outputs: [box]}, network?}` →
`{ok, txId, inputs: [{boxId, verdict, reducedTo?, error?, cost}], outputs,
problems, ergIn, ergOut}`. The one operation of a sandbox chain, stateless:
`boxes` are the unspent boxes (each with its `boxId`); every input's script
is evaluated with `selfIndex`, all inputs, the outputs, the data inputs and
that input's own context variables and secrets; ERG and tokens must balance
(one new token named after the first input may be minted); `outputs` come
back with `boxId` and `creationHeight` set, ready to be the next `boxes`.
`ok` is true only when every input passed (or its proof was accepted) and
nothing is unbalanced; nothing is applied server-side.

### `POST /api/v1/point`

`{secret, base?}` → `{point, generator, address?, testnetAddress?}`: the public point of a 32-byte
hex secret — `g^x`, or `base^x` when a compressed base point is given — so
a scenario's `secrets` and a script's constants agree. For test keys.

### `POST /api/v1/test`

Run a contract test suite: the contract (`source` + `params`, or `tree`) and
named `scenarios`, each a scenario with a `name` and the verdict it must
produce (`expect`: `pass` / `fail` / `error` / `needsProof` / `proofAccepted`
/ `proofRejected`). The contract compiles once; every case runs against it. A case may also assert on
WHO may spend with `expectResidual` / `expectResidualExcludes` (substrings of
the reduced sigma proposition, e.g. a key's hex prefix or `OR(`) — the way
to tell "the receiver may claim" from "only the funder may cancel" when both
are `needsProof`. In any scenario
box, `"ergoTree": "$self"` stands for the contract under test (its tree is
not known when the suite is written) — the way to assert "the rest stays
under this contract".
The response lists each case with expected and actual verdicts, `passed`,
the error text when the script threw, the residual proposition, and cost,
plus `passed`/`failed` totals. Same file shape as `ergo-es test` — see
`examples/tests/*.test.json`.

### `POST /api/v1/eval`

Run a scenario — contract plus spending context — on the consensus reducer.
The body is the scenario JSON itself (the `ergo-es eval` schema in the sandbox
README): `source` or `tree`, `height`, optional `selfBox` / `inputs` /
`outputs` / `dataInputs` / `contextVars` / `proof` / `costLimit` / `network`,
plus `headers` (`CONTEXT.headers`, newest first), `values` in the answer
(every evaluated node's value with its preorder `irId`, positioned by
`offset`/`line`/`col` when the scenario came from `source` and the source
map aligned — the workbench marks them over the editor), `secrets` (the sandbox makes and verifies the spending proof), `parties` (the multi-party flow, no pooled secrets) and
`avl` (trees built by a real prover, referenced as `@avl.name…`); see the
engine README. Both also work per case in `POST /api/v1/test`.

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/eval \
  -H 'content-type: application/json' \
  -d '{"source":"sigmaProp(HEIGHT > 100)","height":200}'
```

```json
{
  "verdict": "pass",
  "error": null,
  "cost": 4,
  "costLimit": 8001091,
  "reducedTo": "true",
  "trace": [],
  "treeHex": "100104c801d191a37300",
  "address": "…"
}
```

`verdict` is `pass` / `fail` / `error` (the script threw; `error` says why) /
`needsProof` (`reducedTo` is the residual proposition) / `proofAccepted` /
`proofRejected`. A script that ran and failed is a 200 with that verdict;
only marshalling and compile errors are 400s, with the compiler's message.

The response also includes `hotSpots: [{label, jit, count, share}]`, ranked
by descending JIT units (ties by label), using the public
`ergo_sandbox::hot_spots::hot_spots` fold. `count` is the number of cost
charging steps and `share` is a fraction of the recorded trace (0–1).
The web dependency enables the sandbox's existing `cost-trace` feature.

These rows describe the diagnostic **reduction** only. They exclude the
later proof-verification pass, whose cost can replace the top-level `cost`
(in block units). They are operation labels, without source positions.
Errors can have partial traces; an empty array means no ranking is available.
The Scenario result displays the ranking in both Write and Read, retaining
the existing semantic Trace below it.

### Storage rent in every answer

`inspect`, `compile` and `hunt` responses carry `rent`: `{boxBytes,
feeNanoerg, periodBlocks, feeFactor, nextCollectionHeight?}` — the fee a
miner may take from a box under this contract once per storage period
(mainnet: 1,051,200 blocks ≈ 4 years, 1,250,000 nanoERG per byte), computed
from the box's serialized size (the hunt's `selfBox` when given, else a
minimal box). A box holding less than the fee is taken entirely, tokens
included; otherwise it is recreated minus the fee with the same script,
tokens and registers. This applies regardless of the script, which is why
the UI says it for every contract.

Errors: `{"error":{"code":"invalid_input","message":"…"}}` — `invalid_input`
(400, including malformed JSON and unknown `network`), `too_large` (413),
`internal` (500). Every error, including the extractor's own rejections, uses
this envelope. Panics never reach the client. An `invalid_input` message for a
bad tree carries the parser's reason (offset, opcode): it describes the
caller's own bytes, not server state, and is the useful part of the reply.

Limits: request bodies capped at 1 MiB (a model-swept suite is ~100 KB); at most 64 engine requests (inspect,
hunt and eval together, one shared semaphore) in flight, with the rest queued. The
limit is scoped to the engine routes so the health check and the static UI
stay answerable while the engine is saturated; it also bounds the number of
large-stack threads alive at once. Per-IP rate
limiting is deliberately left to the reverse proxy.

## The stack budget — why every decompile runs on a big stack

The decompile path (`parse_tree → lift_tree → print → audit`) recurses, and
**stack frames per level are much wider for real contracts than for synthetic
ones**: an arithmetic-only shape lifts through a cheap path (~1 KiB/level),
while real contracts drive `lift_op_inner` — a large function whose debug frame
is far wider. Measured on the deepest tree in the mainnet corpus (46 levels,
2.7 KB of wire bytes):

- **overflows a 2 MiB thread** — tokio's default for blocking threads;
- **fits in 3 MiB**.

An overflow is not a failed request: it aborts the whole process, killing every
in-flight request. So the handler runs the decompile inside
`tokio::task::spawn_blocking` wrapping
`ergo_sandbox::decompile::with_large_stack` (16 MiB). This is load-bearing, not
belt-and-braces — `ergo-web/tests/http.rs` pins it with a test over that real
tree, verified to abort when `with_large_stack` is removed.

Two properties keep this safe rather than fragile:

- The engine's depth caps (`MAX_EXPR_DEPTH` = 110 at parse, `MAX_LIFT_DEPTH` =
  128 at lift) bound **node count**, not frame width — they are not the stack
  guarantee. The big stack is.
- `LARGE_STACK_BYTES` is 16 MiB against a measured requirement somewhere
  between 2 and 3 MiB — generous by design. Leave it alone rather than tuning
  toward a cliff.

## Tests

```bash
cargo test -p ergo-web
```

Integration tests start a real server on an ephemeral port: both fixtures,
garbage-input / malformed-JSON / unknown-network 400s (all JSON), an
oversized-body 413 (JSON), the testnet address prefix, the deep-contract
test above, the hunt endpoint (spendable-by-anyone, selfBox + height
accepted, bad selfBox → 400), and the eval endpoint (pass / fail / error /
needsProof verdicts, compile error → 400, empty scenario → 400).
`cargo test -p ergo-sandbox` must stay at its baseline (39 passed) — the engine
is not modified by this crate.

### Reader workflow and protocol maps

The browser opens in **Read**. Paste an address or ErgoTree hex for local
source recovery, or a box ID for lookup through the configured explorer.
Auto detection treats 64 hex characters as a box ID; select **ErgoTree hex**
explicitly for a script of exactly 32 bytes. Chain availability and the
explorer's network are shown before a request. **Choose a box from chain**
lets an address's returned boxes be selected individually; selecting a box
reads its own script and replaces the spending context.

**Test this contract** opens the reader's test tools. In Read, scenarios
without `source`/`tree` use the recovered tree and, unless overridden, the
selected box as SELF. Suites and exported suites use the recovered tree.
Scenario heights remain explicit; the shortcut seeds the selected box's
lookup height (or 1500000 without a lookup). In Write, the same tools use the
editor's source and parameters. Changing reader input invalidates the old
context, and late responses cannot replace a newer contract's results.

`POST /api/v1/map` follows references using the engine's existing protocol
map implementation. It is a reading endpoint; it does not run drain hunts.

```json
{
  "input": "<64-character box ID>",
  "kind": "boxId",
  "network": "mainnet",
  "maxNodes": 24,
  "maxDepth": 2
}
```

`kind` is required: `boxId`, `address`, `tokenId` or `transactionId`.
`network` defaults to `mainnet`. Live traversal requires `EXPLORER_URL` and
a matching `EXPLORER_NETWORK`; unconfigured is 501, network mismatch is 400.
`maxNodes` accepts 1–64; `maxDepth` accepts 0–4 (0 maps only the seed
frontier). Invalid caps and malformed IDs return the standard JSON 400.
A missing seed is 404; explorer failures are 502.

An optional `fixture` accepts the engine's format-version-1 **recorded
chain fixture**. This always runs offline, even if an explorer is configured
or the recording contains a URL. Missing recorded answers are errors, not
empty results. `ui/examples/reader-map.json` is a small, explicitly synthetic
oracle-reading example used by the browser's **Try an offline example**.

The response is a web-owned DTO:

- `seed`: input and kind; `network`; `source`: kind, height, recorded flag.
- `caps`: requested node/depth bounds; `truncated`: omitted nodes, depth
  boundaries, omitted frontier boxes and omitted holders by token/script
  hash, or null when no cap was hit. Other traversal bounds use the engine's
  `MapOptions::default()` (frontier 16, holders 2 per token/script hash,
  page size 100, fetch cap 32 per query).
- `nodes`: box ID, tree hex, nanoERG `value` **as a decimal string**, depth,
  completeness and optional tree error. The tree can be sent to `/inspect`.
- `edges`: source box ID, target box ID or `unresolved` reason, `binding`,
  identity dimensions in `covers`, code `site`, optional `singleton`.

The UI renders box cards and a typed reference table, retains unresolved
edges and truncation notices, opens each node's source, and downloads this
response as `protocol-map.json`. That download is a **map result**, not a
chain fixture and not the CLI's canonical map schema. The file picker accepts
chain fixtures under 900 KB to leave room inside the 1 MiB request envelope.

Map work uses the shared engine budget and large stack. The engine's live
explorer client runs synchronously inside that blocking job and can retry
slow requests; a large live map can take minutes. Browser cancellation
suppresses obsolete results but does not cancel an engine job. Its permit
remains held until completion. A map is the set reached at the reported
height, not a protocol-completeness claim. Map nodes contain scripts and token
metadata, not registers or a complete spending context; source navigation
therefore clears any previous SELF. Fetch the box separately for testing.

### Browser regression checks

No npm installation or browser bundle is needed. The runner drives
`/usr/bin/chromium-browser --headless` through CDP using Node's built-in
WebSocket and saves screenshots under `ui/ux-review/screenshots/`.
Use separate terminals for the two test instances and synthetic explorer:

```bash
CARGO_TARGET_DIR=/home/rkadias/.cache/cargo-target cargo build --release -p ergo-web
BIND_ADDR=127.0.0.1:8099 /home/rkadias/.cache/cargo-target/release/ergo-web
python3 ergo-web/tests/reader-explorer.py 8101
BIND_ADDR=127.0.0.1:8100 EXPLORER_URL=http://127.0.0.1:8101 /home/rkadias/.cache/cargo-target/release/ergo-web
node ergo-web/tests/browser.mjs http://127.0.0.1:8099 http://127.0.0.1:8100
```

Run from the repository root with Node 22 or newer. Ports 8099, 8100 and
8101 are review instances only; the service on 8090 is not involved. Browser
checks cover reader/editor context isolation, stale responses, source
navigation, typed edges and caps, offline/live lookup, box selection,
network mismatch, keyboard tabs, mobile layout, and the existing Build,
Write, Play, eval, tests, examples, point and validation surfaces. Live
explorer paths use synthetic data served on loopback.

Compiler-position and cost-view browser checks (system Chromium, no npm
packages) and before/after captures are documented in
[`ui/positions-review/README.md`](../ui/positions-review/README.md):

```bash
CARGO_TARGET_DIR=./target-sh cargo build --release -p ergo-web
BIND_ADDR=127.0.0.1:8099 EXPLORER_URL= UI_DIR=ui ./target-sh/release/ergo-web
# In another terminal:
node ergo-web/tests/positions-browser.mjs http://127.0.0.1:8099
```

`valid` is a deprecated alias for `preflightPassed`, never node acceptance.
All analysis/execution responses carry method/provenance limits and
`nodeValidated: false`. Static severity means review priority; triage format v2
uses `reproduced-in-scenario`, with no verified-record import. Play uses
deterministic simulation IDs and supplied/default proof messages.
