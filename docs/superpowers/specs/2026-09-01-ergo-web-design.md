# ergo-web — design

Date: 2026-09-01
Status: proposed
Depends on: P2.5 (public lifted AST), P3a (audit lints) — both on `main`
Scope: the HTTP service and one UI loop. Not the whole workbench.

## The product loop this ships

**Paste a contract address or ErgoTree hex → read the contract in ErgoScript →
see audit findings.**

That is the entire v1. It is the capability nothing else in the Ergo ecosystem
has, it needs no scenario builder, and it is a complete useful product alone.
The compile page and the scenario runner are later work on the same API.

## Why a server, and why not WebAssembly yet

A browser-only build was tested on 2026-09-01 and is blocked: `ergo-sigma` has a
`compile_error!` requiring `panic = "unwind"`, because its AVL verifier relies on
`catch_unwind` to fail closed on malformed proofs. `wasm32-unknown-unknown` is
abort-only on stable Rust. That guard protects consensus and must not be weakened
for a playground.

A WASM build of the *reading* half remains viable later — only `eval.rs` and
`box_build.rs` touch `ergo_sigma` — and would need `getrandom`'s `js` feature
(verified working; `getrandom` arrives transitively via `k256`, which
`ergo-crypto` needs for `PK("…")` rendering).

Nothing here forecloses that. The API is the stable boundary; a future WASM build
becomes a second implementation behind the same UI.

## v1 makes no outbound network calls

`ergo_ser::address::decode_address_to_tree_bytes` recovers the tree from a P2S
address locally, so the whole loop runs offline. Box-id and transaction lookups
would require an explorer or node dependency — deferred.

This is a deliberate production property, not a limitation to apologise for: no
outbound calls means no third-party outage, no API key, no rate limit but our
own, and no data leaving the host.

## Architecture

```
ergo-forge/
  ergo-sandbox/   engine (exists) — compile, lift, print, audit, eval
  ergo-web/       NEW: HTTP service; JSON API + serves the UI
  ui/             NEW: static files, plain HTML/CSS/JS, no bundler
```

`ergo-web` depends on `ergo-sandbox` exactly as the CLI does. **Nothing is added
to the node.** `ergo-sandbox` keeps its "no tokio" rule — the async runtime lives
in `ergo-web` only.

### One API, many clients

The UI is the first client of the API, not a special one. Everything the page can
do is reachable over HTTP, so a VS Code extension, a script, or someone else's
front-end needs no new server work. This is the reason to build an API rather
than server-rendered pages.

Do **not** build two UIs. One UI, sectioned as the product grows.

## HTTP surface

Versioned under `/api/v1/`. All responses JSON, all errors JSON.

### `POST /api/v1/inspect`

The whole v1 loop in one call.

```jsonc
// request
{
  "input": "9hY16vzHmmfyVBwKeFGHvb2bMFsG94A1u7To1QWtUokACyFVENQ",
  "network": "mainnet"          // optional, default "mainnet"
}
```

`input` accepts either a P2S/P2PK address or raw ErgoTree hex. The server
distinguishes by trying hex decode first, then address decode — the two are
unambiguous in practice (addresses are base58, trees are hex).

```jsonc
// 200 response
{
  "tree_hex": "1001040ad191e4c6a704047300",
  "address":  "9hY16…",          // canonical P2S for the tree, on `network`
  "source":   "SELF.R4[Int].get > 5",
  "completeness": "complete",    // or "partial"
  "raw_placeholders": 0,
  "truncated": false,
  "findings": [
    {
      "lint": "unchecked-get",
      "severity": "high",
      "node_id": 4,
      "message": "Option.get with no isDefined guard — …",
      "snippet": "SELF.R4[Int].get"
    }
  ]
}
```

`completeness` is not decoration. When `partial`, the lift did not understand the
whole contract, and the UI must say so prominently — absence of findings proves
nothing. This carries the discipline the decompiler and audit layer already
apply.

### `GET /api/v1/health`

`{"status":"ok","version":"<crate version>"}`. For uptime checks and deploys.

### Errors

```jsonc
{ "error": { "code": "invalid_input", "message": "not a valid address or ErgoTree hex" } }
```

Codes: `invalid_input` (400), `too_large` (413), `rate_limited` (429),
`internal` (500). Never leak a Rust panic message or backtrace to the client;
log it server-side and return `internal`.

### Wire types are DTOs, defined in `ergo-web`

Do **not** add `Serialize` to `ergo_sandbox`'s `Finding`/`Audit`/`Completeness`.
Define the JSON shapes as separate structs in `ergo-web` and convert. The API is
a versioned public contract; the engine's types must stay free to change without
silently altering it.

## The stack budget is a correctness requirement, not a tuning knob

The lift and printer recurse. A 46-level contract needs roughly 3 MiB, and
**tokio worker threads default to 2 MiB** — so calling the decompiler directly
from a request handler will abort the process on a deep contract. This is not
theoretical; it is the bug that `decompile::with_large_stack` and
`MAX_LIFT_DEPTH` exist for.

Every decompile must run on a large stack. Use `tokio::task::spawn_blocking`
wrapping `decompile::with_large_stack` (16 MiB), so the recursion never runs on
a runtime worker.

An integration test must cover a deep contract through the HTTP layer, not just
through the library.

## Production properties

- **Stateless.** No database, no session, nothing shared between requests.
  Restarts cleanly, scales by running more copies.
- **One binary plus a static folder.** `ergo-web` serves `ui/` itself; no nginx
  required to run it.
- **Limits.** Request body capped at 64 KiB (an ErgoTree hex is far smaller);
  per-IP rate limit; the engine's existing cost limit stays authoritative for
  any future eval endpoint.
- **Config by environment**: `PORT`, `BIND_ADDR`, `RUST_LOG`. No config file.
- **Structured logging** of method, path, status, and duration. No request
  bodies in logs.
- **Graceful shutdown** on SIGTERM so deploys don't cut live requests.

## UI

Plain HTML, CSS, and JavaScript. **No bundler, no framework, no build step** —
this follows the constraint already recorded in the compiler-UI design doc, and
it means the UI still works unchanged in five years.

One page:

- a single input box (address or hex) and a Read button;
- the decompiled source in a `<pre>`, monospace;
- findings below it, most severe first, each showing severity, message, and
  snippet;
- a prominent banner when `completeness` is `partial`;
- the canonical address and tree hex shown for copying.

Server-rendered HTML is explicitly rejected: it would make the API a
second-class citizen, and the API is the product boundary.

## Testing

- **Unit**: input dispatch (address vs hex vs neither) and DTO conversion.
- **Integration over HTTP**, using a real server on an ephemeral port:
  a known unguarded-`get` tree returns exactly one `high` finding; a guarded
  tree returns none; a malformed input returns 400 `invalid_input`; an oversized
  body returns 413; `/health` returns 200.
- **The deep-contract test described above**, exercising the stack path through
  a request.
- The two known trees from the P3a work are the fixtures:
  `1001040ad191e4c6a704047300` (one finding) and
  `1001040ad801d601c6a70404d1ede6720191e472017300` (none).

## Out of scope

Compile-from-source page, scenario builder and evaluation endpoint, box-id and
transaction lookup, WASM build, authentication, persistence, and any
multi-contract or project concept. Each is separate work on the same API.
