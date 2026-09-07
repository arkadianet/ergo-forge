# The scenario format (v1)

This is the contract between ergo-forge and any other reducer that wants
to answer the same question: *given this script and this spending
context, what does the chain say?* A differential runner feeds one
scenario to both and compares the answers. Everything here is stable
within `formatVersion` 1: fields are only ever **added**, never renamed
or re-typed, and a field a reader does not know may be ignored.

Pinned reference: the `ergo-es` binary of the release that ships this
document (`v0.4.1` onward). `ergo-es eval <scenario> --json` and
`ergo-es test <suite> --json` print the shapes in "Output" below.

## Scenario

A JSON object. Keys are camelCase. Unknown keys are ignored.

| key | type | meaning |
|---|---|---|
| `tree` | hex string | ErgoTree wire bytes to evaluate. Exclusive with `source`. |
| `source` | string | ErgoScript to compile with `treeVersion`, `network`, `params`. |
| `params` | object name → typed value | Values for `$name` constants in `source`. |
| `treeVersion` | 0–3, default 3 | Header version for `source`. With 3 every method is visible; the header is stamped only when the script uses a v6 method. |
| `network` | `"mainnet"` (default) or `"testnet"` | Address encodings only. |
| `height` | integer | `CONTEXT.HEIGHT`. |
| `selfBox` | box | The box being spent. Default: a synthetic box carrying the evaluated tree, value 0. |
| `selfIndex` | integer | Alternative to `selfBox`: SELF is `inputs[selfIndex]`, and `inputs` is the whole input list in transaction order. That input may omit `ergoTree`. |
| `inputs` | array of box | `CONTEXT.INPUTS`. Under `selfBox`, SELF is prepended at index 0. |
| `outputs` | array of box | `CONTEXT.OUTPUTS`. |
| `dataInputs` | array of box | `CONTEXT.dataInputs`. |
| `contextVars` | object `"id"` → typed value | SELF's context extension (`getVar`). Keys are decimal var ids as strings. |
| `minerPubkey` | 33-byte hex | Default all-zero. |
| `preHeader` | object | Pre-header fields; every one optional, default zero. |
| `headers` | array of header | The last block headers, newest first, at most 10. Every field optional, default zero. |
| `costLimit` | integer | Budget for the evaluation (default the chain's per-transaction budget). |
| `activatedScriptVersion` | integer, default 3 | `blockHeaderVersion - 1`. |
| `message` | hex string | The bytes a proof commits to (a real spend signs the transaction bytes). Default empty. |
| `proof` | hex string | A spending proof to verify against `message`. |
| `secrets` | array of secret | Secrets the spender holds; when the script reduces to a sigma proposition and no `proof` is given, a proof is **made** with them, then verified. |
| `parties` | array of party | Signers who never pool secrets (multi-party flow). |
| `avl` | object name → AVL spec | AVL+ trees built by a real prover; typed values refer to them (below). |

### Box

| key | type | meaning |
|---|---|---|
| `value` | integer | nanoERG. |
| `ergoTree` | hex string | Wire bytes. May be omitted on the SELF box. |
| `tokens` | array of `{id, amount}` | 32-byte hex id, integer amount. |
| `creationHeight` | integer | |
| `registers` | object `"R4"`…`"R9"` → typed value | Dense from R4 upward. |
| `boxId` | 32-byte hex | Default: computed from the box's bytes, as the chain would (transaction id all-zero, index 0). |
| `extension` | object `"id"` → typed value | This input's own context extension, read by `getVarFromInput`. For SELF, `contextVars` takes precedence. |

### Typed value

`{"type": T, "value": V}`. `T` is an ErgoScript type name (`Int`, `Long`,
`Coll[Byte]`, `GroupElement`, `SigmaProp`, `AvlTree`, `(Int, Long)`,
`Option[Int]`, `Coll[GroupElement]`, …). `V` is the JSON rendering:
numbers as JSON numbers (or strings for full 64-bit range), byte
collections as hex, points as 33-byte hex, collections as arrays, tuples
as arrays, options as the value or `null`.

Two special forms:

- `{"type": "raw", "value": "<hex>"}` — a serialized constant exactly as
  a node reports a register or extension (type descriptor first).
- `"@avl.name"`, `"@avl.name.after"`, `"@avl.name.proof"`,
  `"@avl.name.digest"`, `"@avl.name.digestAfter"` as `V` for an
  `AvlTree` or `Coll[Byte]` — the tree before its operations, after
  them, and the proof covering them.

An `AvlTree` value may also be spelled out:
`{"digest": hex, "keyLength": n, "valueLength": n|null, "insertAllowed":
b, "updateAllowed": b, "removeAllowed": b}`.

### AVL spec

`{"keyLength": 32, "valueLength": 8|null, "entries": [[keyHex, valueHex]…],
"operations": [{"insert": {"key", "value"}} | {"update": …} |
{"insertOrUpdate": …} | {"remove": {"key"}} | {"lookup": {"key"}}…],
"insertAllowed": true, "updateAllowed": true, "removeAllowed": true}`.
The tree is built from `entries` in order, then `operations` are applied
in order and one proof covers them all. Flags default to `true`.

### Secret

`{"dlog": "<32-byte hex>"}` or
`{"dht": {"g": pointHex, "h": pointHex, "x": "<32-byte hex>"}}`.

### Header

`{"id", "parentId", "version", "height", "timestamp", "nBits",
"transactionsRoot", "extensionRoot", "stateRoot", "adProofsRoot",
"minerPk", "powOnetimePk", "powNonce", "powDistance", "votes"}`, every
one optional.

## Suite

`{"source" | "tree", "params", "network", "treeVersion", "scenarios": [case…]}`.
A case is a scenario plus `name`, `expect`, and optionally
`expectResidual` / `expectResidualExcludes` (substrings the residual
proposition must contain / must not contain, for `needsProof` cases).

## Verdicts

| verdict | meaning |
|---|---|
| `pass` | reduced to `true` |
| `fail` | reduced to `false` |
| `error` | the script threw (see `error`) |
| `needsProof` | reduced to a sigma proposition; no proof was given or could be made (`reducedTo` shows it) |
| `proofAccepted` | reduced to a sigma proposition and the proof (given, or made from `secrets`/`parties`) verified against `message` |
| `proofRejected` | a proof was given and did not verify |
| `invalid` | (suites only) the scenario could not be marshalled |

`cost` is the JIT cost in block units, as the node accounts them, charged
against `costLimit`.

## Output

`ergo-es eval <scenario> --json`:

```json
{ "formatVersion": 1, "verdict": "pass", "error": null, "reducedTo": "true",
  "cost": 123, "costLimit": 1000000, "proof": null,
  "treeHex": "…", "p2sAddress": "…" }
```

`ergo-es test <suite> --json`:

```json
{ "formatVersion": 1, "treeHex": "…", "address": "…",
  "cases": [ { "name": "…", "expected": "pass", "actual": "pass", "passed": true,
               "error": null, "reducedTo": "true", "cost": 123 } … ],
  "passed": 2, "failed": 0 }
```

A differential runner should compare, per case: `actual` (must be
equal), `cost` (must be equal; a difference is a finding about one of the
evaluators), and `reducedTo` when both print one. `error` text is
informational and may differ in wording.

## What a second reducer needs to reproduce

1. Compile `source` with `params` (constant segregation on; header per
   `treeVersion`) or take `tree` as given.
2. Build the boxes with real bytes: `boxId` is `blake2b256` of the box
   serialized with a zero transaction id and index 0 unless given.
3. Put SELF at `selfIndex` (or at 0 with `selfBox`), give every input its
   `extension`, SELF its `contextVars` on top.
4. Build the AVL+ trees with the standard `ergo-avltree` prover (key
   length, value length, entries in order, operations in order), and
   substitute `@avl.…` references.
5. Reduce with the given height, headers, pre-header, miner key, cost
   limit and activated version; report the verdict vocabulary above.
6. If a sigma proposition remains: verify `proof` against `message`, or
   make a proof from `secrets` (single prover) / `parties` (distributed
   signing) and verify that.
