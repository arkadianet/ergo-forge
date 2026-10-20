# `parse_tree` refuses a size-less tree whose root is not a SigmaProp

Prepared on `fix/parse-tree-rule-1001` from `chore/node-pin-55229e00` (`b1d8820`) on
2026-10-21. It closes the follow-up that [node-pin-55229e00.md](node-pin-55229e00.md)
section 3 left open (issue #118): "Forge's own `inspect::parse_tree` and `parse_tree_complete`
still accept the tree, more leniently than either node." That report is not edited. This
one corrects it.

## The rule

Validation rule 1001, `CheckDeserializedScriptIsSigmaProp`: a deserialized script must be a
SigmaProp. What a violation does depends on the size bit:

- **No size bit.** There is no declared-size region to keep, so the tree cannot be wrapped as
  unparsed. Scala's `ErgoTreeSerializer.deserializeErgoTree` raises "Cannot handle
  ValidationException, ErgoTree serialized without size bit", and a box holding the tree
  cannot be parsed. The node refuses a transaction creating such an output at
  deserialization (`ergo_ser::ergo_tree::check_sigma_prop_root`, called by the box readers).
- **Size bit.** Both wrap the tree as an unparsed soft-fork tree, which parses and
  re-serializes. Spending it is refused ("Script has not been recognized due to
  ValidationException, and it cannot be accepted as soft-fork").

## What changes

- `ergo-sandbox/src/inspect.rs`: one private reader, `read_checked`, runs the node's
  `read_ergo_tree` and then the node's own `check_sigma_prop_root`. Forge reimplements
  nothing. `parse_tree`, `parse_tree_complete` and `tree_report` (the `ergo-es decompile
  <hex>` renderer) use it.
- `ergo-sandbox/src/tree.rs`: `ergo-es tree` reads the tree itself, so it calls
  `check_sigma_prop_root` after its own `read_ergo_tree`. The message keeps its
  `invalid ErgoTree:` prefix.
- A sized tree is not refused at parse, as before.

Other readers of tree bytes are left as they are, on purpose. `box_build.rs` keeps its
synthetic shape for a script that does not parse. `evidence/wire.rs` builds a candidate for a
validation request, and the node's own pipeline then refuses it. `identity.rs` only compares
trees.

## Reference behaviour

The probe [`scala/BoolRoot.scala`](parse-tree-rule-1001/scala/BoolRoot.scala) runs sigma-state
6.0.7 (`sigma-state_2.12-6.0.7.jar`, scala-cli 1.17.1, JDK 17.0.20.1). It deserializes each tree
as a box script is deserialized (`ErgoTreeSerializer.DefaultSerializer`, activated script version 3), and
spends a box protected by the result with an empty proof at height 100
([`boolroot.out`](parse-tree-rule-1001/scala/boolroot.out)). The node column is
`ergo_ser::ergo_box::read_ergo_box_candidate`, asserted for each vector by
`sandbox_eval.rs` `a_sizeless_tree_with_a_non_sigma_prop_root_is_refused_at_parse`.

| Tree | Shape | Scala 6.0.7 | Node 55229e00 box reader | Forge before | Forge after |
|---|---|---|---|---|---|
| `100101d17300` | size-less, Boolean root | refused at deserialization | refused | parsed | refused |
| `10810001d17300` | size-less, Boolean root, overlong count | refused at deserialization | refused | parsed | refused |
| `18050101d17300` | sized, Boolean root | unparsed tree; spend refused | accepted (unparsed) | parsed (unparsed) | parsed (unparsed) |
| `10010101d17300` | sigmaProp(true) | parsed; spends | accepted | parsed | parsed |
| `1081000101d17300` | sigmaProp(true), overlong count | parsed; spends | accepted | parsed | parsed |

The second row's Scala result is also in
[node-pin-55229e00/scala/validation](node-pin-55229e00/scala/validation/probe3.out).

## Changed expectation

`sandbox_eval.rs` `standalone_tree_input_must_consume_all_bytes` used `10810001d17300` to show
that a tree with an overlong constant count parses. That tree is not a tree either reference
accepts. The test now uses `1081000101d17300`, the same overlong count with a SigmaProp root,
which both references accept: it parses, completes and evaluates to `pass`. The trailing-byte
assertion is unchanged.

The refused tree has its own test. Its sandbox evaluation used to end in an `error` verdict
("expected SigmaProp script result, got Bool(true)"). It is now an input error from
`eval_scenario`, like the trailing-byte case in the same test, with the node's rule 1001
message. This is the only behaviour change a caller sees beyond the refusal itself.

## Callers

Every caller propagates the error. Each was run on `10810001d17300` where it has an entry
point.

| Entry point | Reports the refusal as |
|---|---|
| `ergo-es tree <hex>` | `error: invalid ErgoTree: invalid data: sizeless ErgoTree root has type SBoolean, expected SigmaProp (CheckDeserializedScriptIsSigmaProp, rule 1001)`, exit 1 |
| `ergo-es decompile <hex>`, `eval`, `audit <hex>`, `hunt`, `checklist` | the same message with the `invalid ergo tree:` prefix, exit 1 |
| `ergo-es roundtrip <hex>` | an `ERR` row with that message, as for any unparseable tree |
| `ergo-es audit --seed / --mainnet / --trees` | counted in `parse-errors: N`, as for any unparseable tree |
| `ergo-es decompile --seed / --mainnet` | stop at the first refusal; their corpora hold compiled and on-chain trees only |
| `POST /api/v1/inspect` | 400 `invalid_input` with the message (`ergo-web/tests/http.rs`) |
| `POST /api/v1/compile` | unchanged path (the tree comes from the compiler); not exercised |
| `claim`, `triage`, `necessity`, `check_relation`, evidence case analysis | `Err(String)` with the message |
| `map` | a box node records `tree_error`; `discover_refs` records `parse-failure: ...`, which blocks the walk |
| `ingest` (sweeps) | a failed record, `compiled tree failed to parse: ...` |
| `attack::slot_shape` | unchanged: any unparseable tree gives the default shape |

## Not changed

`parse_tree` still does not apply the node's other box-script gates
(`ergo_ser::ergo_tree::gates`). Observed: a version-1 tree without a size bit, which the nodes
refuse under rule 1012 (`check_header_size_bit`), parses and decompiles in forge
(`0108cd034a53f17d249721c647c13477bb16982c8b2b16daa923d2a49dee8a88593c8356`). The same holds in
code for `check_resolvable_methods` and `check_tree_version_supported`, which this change does
not exercise. They are outside issue #118.

## Records

No historical or frozen record was edited. `docs/discovery/consolidation-verification/preservation.json`
lists a SHA-256 for `ergo-sandbox/src/tree.rs`, taken at its 2026-09 baseline, which this change
makes stale. The file is left as recorded; no test or gate reads it, and it already disagrees
with `inspect.rs` and `validate.rs` after earlier changes.

## Checks

Every command forge's CI runs (the 14 checks of [node-pin-55229e00.md](node-pin-55229e00.md)), and
the engine and external measurements compared byte for byte with PR #116's, exited 0, each on its
own, under CI's build settings (`RUSTFLAGS=-D warnings`, the dev profile at `opt-level=3` with
debug assertions and overflow checks), Rust 1.99.0, Node 22, `npm ci --prefix ui`,
`scripts/requirements-test.txt`, the node checked out beside forge at the pin, and `--locked` on
every cargo command. They ran on this change before it was rebased onto `main` at `d0e7f10`
(#120, which changes only the lock commands). The logs are not committed; CI runs on the pull
request.

The workspace and `cost-trace` suites each ran 665 tests, none failed, one ignored (the external
measurement), four more than PR #116's 661: the new rule 1001 tests in `sandbox_eval.rs` (one
test split into two), `tree.rs` (two) and `ergo-web/tests/http.rs` (one). `tests-web-no-default`
ran 98. `contract-suites` ran all 22 example suites. The decompiler measurements are byte for byte
PR #116's (371 rows, seed 79/92 and mainnet 270/279 exact; 401 external rows), so no
compiled tree and no round trip changed.
