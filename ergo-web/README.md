# ergo-web

HTTP service for the ErgoScript workbench: **paste a contract address or
ErgoTree hex, get the contract back as readable ErgoScript, plus static audit
findings** about the code that will actually execute.

Built on `ergo-sandbox` (decompiler + audit lints). Stateless — no database,
nothing shared between requests, **no outbound network calls** (addresses decode
locally; no node or explorer is contacted).

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
| `RUST_LOG` | `info`           | `tracing` filter (method, path, status, duration are logged; never request bodies) |

Shutdown is graceful on SIGTERM/Ctrl-C so deploys do not cut live requests.

## API

Versioned under `/api/v1/`. All responses and errors are JSON.

### `GET /api/v1/health`

```json
{"status":"ok","version":"0.1.0"}
```

### `POST /api/v1/inspect`

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
  "tree_hex": "1001040ad191e4c6a704047300",
  "address": "8NJuqcG7SdhX7cFKGBmfAkXn",
  "source": "SELF.R4[Int].get > 5",
  "completeness": "complete",
  "raw_placeholders": 0,
  "truncated": false,
  "findings": [
    {
      "lint": "unchecked-get",
      "severity": "high",
      "node_id": 2,
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
near the mainnet tip) and `selfBox` (the spent box in the scenario-JSON box
shape: `value`, `tokens`, `registers`, `creationHeight`). Six probes — three
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

`verdict` is one of `spendableByAnyone` (an attacker probe passed — a hit is
a transaction anyone can build), `movableByAnyone` (only preserve probes
passed: anyone can re-spend the box back into the same contract),
`requiresProof` (`residuals` lists the distinct sigma propositions — who can
spend), or `notUnderProbes` (every probe failed or errored; **not** a safety
claim). Without `selfBox`, `selfSynthetic` is true and any register read
errors out — supply the real box before concluding anything.

### `POST /api/v1/eval`

Run a scenario — contract plus spending context — on the consensus reducer.
The body is the scenario JSON itself (the `ergo-es eval` schema in the sandbox
README): `source` or `tree`, `height`, optional `selfBox` / `inputs` /
`outputs` / `dataInputs` / `contextVars` / `proof` / `costLimit` / `network`.

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

Errors: `{"error":{"code":"invalid_input","message":"…"}}` — `invalid_input`
(400, including malformed JSON and unknown `network`), `too_large` (413),
`internal` (500). Every error, including the extractor's own rejections, uses
this envelope. Panics never reach the client. An `invalid_input` message for a
bad tree carries the parser's reason (offset, opcode): it describes the
caller's own bytes, not server state, and is the useful part of the reply.

Limits: request bodies capped at 64 KiB; at most 64 engine requests (inspect,
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
