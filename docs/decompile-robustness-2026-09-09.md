# Decompiler robustness measurement — 2026-09-09

Baseline: `e1e7218` (merged main), branch `feat/decompile-robustness`. Compiler dependency: `9468043396e5daa2828211bcff4234bc70fae4f0`. No commits or pushes were made.

**WRONG OUTPUT FOUND AND FIXED:** Boolean XOR grouping could change the condition; signature operands/types were discarded or guessed; unsupported block statements could disappear. These are correctness defects, not formatting changes. Focused regression tests cover each fix below.

**THE BYTE-EXACT BAR IS NOT YET MET:** 85 bundled entries and 27 external entries still recompile to different bytes. They remain explicit divergences, not successes or claims of semantic equivalence. There were no regressions among previously byte-identical entries. Zero raw placeholders means a structural lift completed; it does not certify byte parity.

## Before → after

| Corpus | Entries | Byte-identical | Recompiles-but-differs | Fails-to-recompile | Fails-to-decompile | Initial compile failed |
|---|---:|---:|---:|---:|---:|---:|
| Bundled contract files | 108 | 29 → 45 | 50 → 53 | 19 → 0 | 0 → 0 | 10 → 10 |
| Committed compiler vectors | 73 | 73 → 73 | 0 → 0 | 0 → 0 | 0 → 0 | 0 → 0 |
| Other bundled compiled fixtures | 147 | 115 → 115 | 28 → 32 | 4 → 0 | 0 → 0 | 0 → 0 |
| **Bundled total** | 328 | 217 → 233 | 78 → 85 | 23 → 0 | 0 → 0 | 10 → 10 |
| External compiler vectors | 110 | 91 → 93 | 14 → 17 | 5 → 0 | 0 → 0 | 0 → 0 |
| External mainnet trees | 279 | 271 → 272 | 5 → 7 | 3 → 0 | 0 → 0 | 0 → 0 |
| External failing-tree fixtures | 5 | 2 → 2 | 1 → 3 | 2 → 0 | 0 → 0 | 0 → 0 |
| **External total** | 394 | 364 → 367 | 20 → 27 | 10 → 0 | 0 → 0 | 0 → 0 |

The bundled run attempts all 108 `.es` files, without a subset. Of these, 98 can be instantiated and compiled; all 98 now decompile and recompile, and 45 are byte-identical. The full bundled measurement takes about **0.24 seconds in release**, excluding compilation. The optional 394-tree external run takes about 0.07 seconds.

## Measurement method and provenance

- The new `ergo-sandbox/tests/decompile_corpus.rs` discovers every contract recursively. It uses `compile_with_params`, then parses/decompiles the resulting bytes, recompiles the emitted source, and compares **the complete bytes**. Synthetic parameters are deterministic: declared defaults are retained; missing numeric values use `100 + parameter index`; bytes use a repeated index byte; public keys use the generator point. Base58 string parameters are encoded as Base58. The harness lists explicit sample values for bare deployment constants such as `PoolNFT`, `SelfX`, and `DexFee`. These are test instantiations, not asserted deployed parameters. Mainnet is tried first; a network-mismatch error selects testnet. Source files are not edited.
- Source syntax cannot carry the wire header. Recompilation uses the same compiler dialect (v3), then reapplies the original header version and size flag, as a wallet would. No body, binding, constant value, constant ordering, or segregation flag is repaired or ignored in the comparison. This same rule is used before and after. EIP-5 templates are instantiated through the existing template API; differences in their emitted constant layout remain failures of the byte-exact bar.
- All 73 committed compiler vectors are counted individually, including repeated tree bytes. Other JSON files under `examples/` and `ergo-sandbox/tests/fixtures/` are searched recursively for hex `tree`, `ergoTree`, and `deployedTree` fields. Identical trees are counted once **per JSON file**, with all JSON pointers retained as aliases in the detailed report. Repeated trees across different files remain separate entries. There are 147 such file/tree entries. Counts describe test entries, not globally distinct contracts.
- The optional external run uses the sibling node checkout: all 110 compiler vectors with `tree_hex` (including environment-dependent vectors, without requiring their original source to compile in an empty environment), 279 distinct mainnet trees, and all five `failing_tree_*.hex` files. Node trees of unknown provenance remain measurements, not the known-provenance gate. Some external vectors overlap the bundled fixture; the totals are deliberately separate.
- A raw placeholder is counted structurally. The harness still attempts recompilation and records its actual error; a parsed tree containing an unsupported construct belongs to “fails-to-recompile,” while a parse/decompile error belongs to “fails-to-decompile.” Initial source compilation failures have their own bucket because there are no bytes to decompile. No panic is swallowed.
- `fixtures/decompile_corpus_expectations.json` pins membership, original-byte SHA-256, bucket, raw count, and truncation **per entry**. Previously exact entries cannot trade places with failures behind an aggregate floor. Known divergences remain visible expectations. The original 73-vector strict byte gate and optional external floor tests remain intact.

## Fixes and their causes

| Fix | Cause and resulting behavior | Regression coverage |
|---|---|---|
| **Boolean grouping** | The printer placed XOR above AND/comparisons, and collapsed distinct comparison precedences. It now matches the pinned parser table, including `>` versus `<`, collection append, and unary/postfix precedence. `(a ^ b) && c` keeps its grouping. | `mixed_boolean_precedence_keeps_the_original_grouping` |
| **Signature operations and types** | `ProveDlog` was retained only for selected AST shapes; computed group-element operands such as `groupGenerator` or conditionals could lose it. Group-element constants were printed as sigma-typed `PK`. `atLeast` guessed child types from text and wrapped already-typed bound signatures in `sigmaProp`. Every computed DLog operand now retains its operation, group elements render as decoded points, and threshold children retain their wire types. | `signature_operands_are_never_discarded_or_converted_to_boolean`, `group_element_constants_keep_their_type` |
| **Lost signature operand** | Opcode `0xCF` printed the bare identifier `isProven`, discarding its operand. It now prints `operand.isProven`. | Signature regression above; `lsp/zed-extension-test.es` in the corpus |
| **Block and scope fidelity** | A two-pass map keyed by recovered names could overwrite reused IDs, bind a RHS to itself, and omit non-binding block items. Statements now lift in order, RHS lookup precedes binding, unsupported items remain visible raw statements, and lookup no longer leaks from completed scopes through a global fallback. | `unsupported_block_items_are_not_silently_dropped`, `reused_binding_ids_keep_statement_order_and_rhs_scope`, `references_do_not_leak_from_a_finished_lambda_scope` |
| Tuple lambda arity | Every unary tuple lambda was treated as a fold. That changed callbacks over tokens and tuples into two-argument functions. A bounded lexical use pass now selects direct fold lambdas and functions used exclusively by folds. Other tuple functions keep their unary signature; ambiguous mixed uses degrade explicitly. | `tuple_parameters_outside_fold_keep_their_arity`, `a_function_used_only_by_folds_keeps_byte_exact_shared_calls`, `a_tuple_function_with_mixed_uses_admits_the_fold_arity_gap` |
| Register zero | `ExtractRegisterAs(R0, T)` became bare `.value`, dropping `Option[T]`. It now remains `.R0[T]`; the distinct amount opcode still prints `.value`. | `register_zero_retains_its_option_type` |
| Collection indexing/default | Some receivers printed as source type application (`obj[index]`), and a default became an option operation on an already indexed element. Indexing now consistently uses `obj(index)`; a default uses `obj.getOrElse(index, default)`. | `collection_index_and_default_keep_the_collection_receiver` |
| Composite constant types | Tuple children were lifted with `Any`, and nested collections guessed element types from values, losing empty-element types. Lifting now propagates the declared tuple fields and collection element type. | `composite_constants_use_declared_types_including_empty_collections`; existing depth-ceiling test |
| Arithmetic source recovery | The compiler re-types already typed operands of `+` and `*`; inline option defaults, folds, applications and conditionals can hit unsupported retyping paths. The printer introduces ordered temporary bindings around affected arithmetic, which the compiler can inline again. The lifted AST, operands and evaluation order are preserved. | `compiler_retyping_requires_temporary_bindings`; bank, recipes and CrystalPool corpus entries |
| Missing readable operations | `ProveDHTuple` and `SubstConstants` had no lift. Both now preserve all operands in their source calls. Byte XOR used an unsupported invented infix `xorBytes`; it now prints `Global.xor`. `MinerPubKey` had the wrong case; the source name is `MinerPubkey`. | `dh_tuple_and_constant_substitution_keep_all_operands`, boolean/XOR regression, `miner_pubkey_uses_the_case_sensitive_source_name` |
| **Honest degradation** | v5 dynamic `getReg` guessed `Int` without a wire type; unsupported deserialization/None/tagged variables and explicit type arguments could look like source despite missing information; raw constants could lack markers. These now produce explicit `<…>` placeholders. Empty/singleton sigma combinators and polymorphic definitions are also retained as raw when their structure cannot be recovered. | `unavailable_information_is_always_a_visible_structural_placeholder` |

All focused regressions are in `ergo-sandbox/tests/decompile_robustness.rs`. No compiler, evaluator, dependency revision, or authored contract was changed.

## Remaining divergences and unliftable constructs

All measured, initially compiled entries now lift without placeholders and recompile. This is **coverage**, not byte-exact correctness. The 85 bundled and 27 external differences remain unresolved decompiler work. Observed differences include explicit numeric casts folding into wider constants (for example a cast of a Long literal becoming a BigInt constant), changes in compiler-generated binding order/sharing, and template constant-table layout/segregation. These observations do not establish semantic or cost equivalence for every differing entry; no such claim is made. Every differing file is identified below.

Outside these measured inputs, the following paths deliberately remain unliftable:

| Construct | Why it remains a placeholder |
|---|---|
| Dynamic Box.getReg v5 | Its wire form lacks the element type; guessing a source type is dishonest. |
| A shared tuple function used both by folds and ordinary calls | Unwrapping its declaration changes other calls. Adding adapters would change the tree; exact shared arity recovery is not implemented. A fold that consumes its synthetic tuple as a whole also remains raw. |
| DeserializeContext / DeserializeRegister, legacy TaggedVar, NoneValue | Verified, type-preserving source reconstruction is not implemented; type/default information must not be silently dropped. |
| Composite SigmaProp constants (including constant DH tuples), unsupported constant types such as Unit/option/string/box/AVL/unsigned BigInt values | These do not yet have a verified constant lift. Dynamic DH-tuple expressions are supported by this change. Typed nested collections and tuples are supported. |
| Unrecognized opcodes/methods or unsupported explicit method type arguments | The lift has no verified source mapping. The existing typed v6 Box.getReg case remains supported. |
| Unsupported/polymorphic block items, dangling references, missing constant indices | Recovering readable bindings would otherwise discard statements or invent missing information. |
| Empty/singleton sigma operation nodes | Printing a bare Boolean or child erases the original operation and can change its type. |
| Unparsed trees or nesting at the lift ceiling (128) | The body is unavailable or the bounded lift stops; depth truncation also sets `truncated`. |

## Source files that cannot initially compile

These ten files are attempted and recorded, not silently skipped. They are outside the decompiler denominator because initial compilation supplies no bytecode.

| File | Initial compiler diagnostic |
|---|---|
| `examples/contracts/chaincash-basis/chaincash/layer2-old/reserve.es` | parse error: syntax error at offset 1126: expected \`=\` |
| `examples/contracts/curve-trees/CurveTreeVerifier-v6.es` | tree not serializable under a v0 header: UnsignedBigInt constant data |
| `examples/contracts/hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodltoken_proxy.es` | parse error: syntax error at offset 7124: expected \`else\` |
| `examples/contracts/lsp/examples/completion_demo.es` | parse error: Block should contain a list of Val bindings and one expression (offset 768) |
| `examples/contracts/lsp/examples/eip5-multi-sig.es` | parse error: syntax error at offset 395: expected literal default value |
| `examples/contracts/lsp/examples/eip5-payment-channel.es` | parse error: syntax error at offset 548: expected literal default value |
| `examples/contracts/lsp/examples/hover_demo.es` | parse error: Block should contain a list of Val bindings and one expression (offset 1577) |
| `examples/contracts/lsp/tests/main.test.es` | parse error: syntax error at offset 0: expected expression |
| `examples/contracts/lsp/tests/simple.test.es` | parse error: syntax error at offset 0: expected expression |
| `examples/contracts/lsp/tests/template.test.es` | parse error: syntax error at offset 0: expected expression |

## Reproduction and verification

```sh
CARGO_TARGET_DIR=./target-dc DC_REPORT=/tmp/dc-after.json cargo test -p ergo-sandbox --release --test decompile_corpus -- --nocapture
CARGO_TARGET_DIR=./target-dc DC_NODE_CHECKOUT=/path/to/ergo DC_REPORT=/tmp/dc-external-after.json cargo test -p ergo-sandbox --release --test decompile_corpus external_compiled -- --ignored --nocapture
CARGO_TARGET_DIR=./target-dc cargo test -p ergo-sandbox --release
CARGO_TARGET_DIR=./target-dc cargo clippy --all-targets --release -- -D warnings
CARGO_TARGET_DIR=./target-dc cargo fmt --all -- --check
```

The baseline used the identical measurement harness with the three changed decompiler implementation files restored to `e1e7218`, then restored the working changes. Detailed `DC_REPORT` JSON includes original bytes, recovered source, recompiled bytes for mismatches, parameters, aliases and errors. The comparison below is the retained before/after measurement; the CI expectation file retains the original input-byte hashes.

Verification: `cargo test -p ergo-sandbox --release` passed; workspace `cargo clippy --all-targets --release -- -D warnings` passed; `cargo fmt --all -- --check` passed. All commands used `CARGO_TARGET_DIR=./target-dc`. The optional external measurement was also run explicitly and passed. The 18 focused robustness tests, the original strict 73-vector test, the full bundled per-entry test, and downstream reader tests all pass. Logs are in `target-dc/test-release.log` and `target-dc/clippy-release.log`.

## Per-entry measurements

Legend: **E** = byte-identical; **D** = recompiles-but-differs; **R** = fails-to-recompile; **L** = fails-to-decompile; **C** = initial compile failed. All entries appear, including unchanged and initially unbuildable files. Fixture suffixes identify zero-based vector indices or JSON pointers; repeated bytes within one non-seed JSON file use the first pointer as their canonical ID.

### Bundled entries

| File / fixture entry | Before | After |
|---|:---:|:---:|
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/0` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/1` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/10` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/11` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/12` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/13` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/14` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/15` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/16` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/17` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/18` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/19` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/2` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/20` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/21` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/22` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/23` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/24` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/25` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/26` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/27` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/28` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/29` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/3` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/30` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/31` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/32` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/33` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/34` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/35` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/36` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/37` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/38` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/39` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/4` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/40` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/41` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/42` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/43` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/44` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/45` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/46` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/47` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/48` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/49` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/5` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/50` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/51` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/52` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/53` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/54` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/55` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/56` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/57` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/58` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/59` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/6` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/60` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/61` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/62` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/63` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/64` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/65` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/66` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/67` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/68` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/69` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/7` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/70` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/71` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/72` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/8` | E | E |
| `ergo-sandbox/tests/fixtures/compile_corpus_subset.json#/9` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/10b755771f7253cff9727a9ca54bb2867e22b1b236657051c47ea9556c517e10/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/1/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/10/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/11/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/12/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/13/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/14/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/15/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/16/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/17/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/18/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/2/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/3/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/4/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/5/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/6/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/7/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/8/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/1ff9d7da132defd21fb421e1a94959a3618e6cac38a23e903770a26f29be8e51/items/9/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/26ef992a598eadfddabfd3c51509fb277b075c943b17199407f68c467b9de1ae/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/3c45f29a5165b030fdb5eaf5d81f8108f9d8f507b31487dd51f4ae08fe07cf4a/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/3fefa1e3fef4e7abbdc074a20bdf751675f058e4bcce5cef0b38bb9460be5c6a/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/4675c1819c3e22add72b73f4b7e83eb743d45013b4ee2d8a63e215de9bc6f57f/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/471057efea32bf406d529902217844a258d3d6bedfcdcd3cfbab01872cc0b74c/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/610735cbf197f9de67b3628129feaa5a52403286859d140be719467c0fb94328/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/610735cbf197f9de67b3628129feaa5a52403286859d140be719467c0fb94328/items/1/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/615be55206b1fea6d7d6828c1874621d5a6eb0e318f98a4e08c94a786f947cec/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/6183680b1c4caaf8ede8c60dc5128e38417bc5b656321388b22baa43a9d150c2/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/6183680b1c4caaf8ede8c60dc5128e38417bc5b656321388b22baa43a9d150c2/items/1/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/6597acef421c21a6468a2b58017df6577b23f00099d9e0772c0608deabdf6d13/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/74f906985e763192fc1d8d461e29406c75b7952da3a89dbc83fe1b889971e455/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/75d7bfbfa6d165bfda1bad3e3fda891e67ccdcfc7b4410c1790923de2ccc9f7f/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/7ba2a85fdb302a181578b1f64cb4a533d89b3f8de4159efece75da41041537f9/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/7ba2a85fdb302a181578b1f64cb4a533d89b3f8de4159efece75da41041537f9/items/2/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/7ba2a85fdb302a181578b1f64cb4a533d89b3f8de4159efece75da41041537f9/items/20/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/7ba2a85fdb302a181578b1f64cb4a533d89b3f8de4159efece75da41041537f9/items/3/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/7ba2a85fdb302a181578b1f64cb4a533d89b3f8de4159efece75da41041537f9/items/5/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/7ba2a85fdb302a181578b1f64cb4a533d89b3f8de4159efece75da41041537f9/items/8/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/905ecdef97381b92c2f0ea9b516f312bfb18082c61b24b40affa6a55555c77c7/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/97ad159235d25d05d7efc5863b5d360f89d7d668409502058be3e7aac177b9cb/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/a40bedb08a32c0258405f950af5a133c397630f89f7d31fb00b3ab9811d29e6a/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/d1c9e20657b4e37de3cd279a994266db34b18e6e786371832ad014fd46583198/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/dexy-gold.json#/boxesByToken/ff7b7eff3c818f9dc573ca03a723a7f6ed1615bf27980ebd4a6c91986b26f801/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/19b7f2e2f11052c020800c8b620660f9f0b5fd5b3f2beacc8b44af960477a694/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/1bfea21924f670ca5f13dd6819ed3bf833ec5a3113d5b6ae87d806db29b94b9a/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/2cf9fb512f487254777ac1d086a55cda9e74a1009fe0d30390a3792f050de58f/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/30e273698c85bbe66319904c499bfefbd76f51487688d674b20623715ff0741b/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/35bc71897cd44d1a624285c54a0be66b69d1c61674603ed89dfe136f32035f0e/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/40db16e1ed50b16077b19102390f36b41ca35c64af87426d04af3b9340859051/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/47472f675d7791462520d78b6c676e65c23b7c11ca54d73d3e031aadb5d56be2/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/6a2b821b5727e85beb5e78b4efb9f0250d59cd48481d2ded2c23e91ba1d07c66/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/74fa4aee3607ceb7bdefd51a856861b5dbfa434a8f6c93bfe967de8ed1a30a78/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/74fa4aee3607ceb7bdefd51a856861b5dbfa434a8f6c93bfe967de8ed1a30a78/items/18/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/78c24bdf41283f45208664cd8eb78e2ffa7fbb29f26ebb43e6b31a46b3b975ae/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/a2482fca4ca774ef9d3896977e3677b031597c6e312b0c10d47157bb0d6ed69f/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/1/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/10/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/11/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/15/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/16/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/17/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/18/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/2/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/20/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/21/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/22/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/23/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/24/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/25/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/28/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/3/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/30/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/31/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/4/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/6/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/8/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ae399fcb751e8e247d0da8179a2bcca2aa5119fff9c85721ffab9cdc9a3cb2dd/items/9/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/bc685d6ad1703ba5775736308fd892807edc04f48ba7a52e802fab241a59962c/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/c79bef6fe21c788546beab08c963999d5ef74151a9b7fd6c1843f626eea0ecf5/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/dbf655f0f6101cb03316e931a689412126fefbfb7c78bd9869ad6a1a58c1b424/items/0/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/dcce07af04ea4f9b7979336476594dc16321547bcc9c6b95a67cb1a94192da4f/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/dcce07af04ea4f9b7979336476594dc16321547bcc9c6b95a67cb1a94192da4f/items/1/ergoTree` | D | D |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/0/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/1/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/10/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/11/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/12/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/13/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/14/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/15/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/16/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/2/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/3/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/4/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/5/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/6/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/7/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/8/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/e1d7fcb9c033970d8485c200efc9febbb17244639c17c57a4c14b76bbd559183/items/9/ergoTree` | E | E |
| `ergo-sandbox/tests/fixtures/map/use-lp.json#/boxesByToken/ef461517a55b8bfcd30356f112928f3333b5b50faf472e8374081307a09110cf/items/0/ergoTree` | D | D |
| `examples/contracts/basics/guarded-register-read.es` | E | E |
| `examples/contracts/basics/height-lock.es` | E | E |
| `examples/contracts/basics/oracle-price-gate.es` | E | E |
| `examples/contracts/basics/pay-to-public-key.es` | E | E |
| `examples/contracts/basics/self-preserving.es` | E | E |
| `examples/contracts/basics/two-of-three.es` | D | E |
| `examples/contracts/basics/unguarded-register-read.es` | E | E |
| `examples/contracts/chaincash-basis/basis-tracker-basis.es` | D | D |
| `examples/contracts/chaincash-basis/chaincash/layer2-old/note.es` | E | E |
| `examples/contracts/chaincash-basis/chaincash/layer2-old/redemption.es` | D | D |
| `examples/contracts/chaincash-basis/chaincash/layer2-old/redproducer.es` | E | E |
| `examples/contracts/chaincash-basis/chaincash/layer2-old/reserve.es` | C | C |
| `examples/contracts/chaincash-basis/chaincash/offchain/basis-token.es` | D | D |
| `examples/contracts/chaincash-basis/chaincash/offchain/basis.es` | D | D |
| `examples/contracts/chaincash-basis/chaincash/onchain/note.es` | E | E |
| `examples/contracts/chaincash-basis/chaincash/onchain/receipt.es` | R | D |
| `examples/contracts/chaincash-basis/chaincash/onchain/reserve.es` | D | D |
| `examples/contracts/crystalpool/buy-token-for-erg.es` | D | D |
| `examples/contracts/crystalpool/deposit.es` | E | E |
| `examples/contracts/crystalpool/sell-token-for-erg.es` | R | D |
| `examples/contracts/crystalpool/swap-tokens-denom.es` | R | D |
| `examples/contracts/crystalpool/swap-tokens.es` | R | E |
| `examples/contracts/curve-trees/CurveTreeVerifier-v6.es` | C | C |
| `examples/contracts/dexy/bank/arbmint.es` | D | D |
| `examples/contracts/dexy/bank/bank.es` | E | E |
| `examples/contracts/dexy/bank/buyback-patched.es` | D | D |
| `examples/contracts/dexy/bank/buyback.es` | D | D |
| `examples/contracts/dexy/bank/freemint.es` | D | D |
| `examples/contracts/dexy/bank/intervention.es` | D | D |
| `examples/contracts/dexy/bank/payout.es` | E | E |
| `examples/contracts/dexy/bank/update/ballot.es` | E | E |
| `examples/contracts/dexy/bank/update/update.es` | D | D |
| `examples/contracts/dexy/gort-dev/emission.es` | E | E |
| `examples/contracts/dexy/hodlcoin/hodlcoin.es` | D | D |
| `examples/contracts/dexy/lp/pool/extract.es` | D | D |
| `examples/contracts/dexy/lp/pool/main.es` | D | D |
| `examples/contracts/dexy/lp/pool/mint.es` | E | E |
| `examples/contracts/dexy/lp/pool/redeem.es` | D | D |
| `examples/contracts/dexy/lp/pool/swap.es` | D | D |
| `examples/contracts/dexy/lp/proxy/Deposit.es` | D | D |
| `examples/contracts/dexy/lp/proxy/Redeem.es` | D | D |
| `examples/contracts/dexy/lp/proxy/SwapBuyV1.es` | D | D |
| `examples/contracts/dexy/lp/proxy/SwapBuyV2.es` | D | D |
| `examples/contracts/dexy/lp/proxy/SwapSell.es` | D | D |
| `examples/contracts/dexy/lp/proxy/SwapSellV1.es` | D | D |
| `examples/contracts/dexy/tracking.es` | E | E |
| `examples/contracts/hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodlerg_bank.es` | D | D |
| `examples/contracts/hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodlerg_fee.es` | D | E |
| `examples/contracts/hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodlerg_proxy.es` | D | D |
| `examples/contracts/hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodltoken_bank.es` | R | E |
| `examples/contracts/hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodltoken_fee.es` | R | E |
| `examples/contracts/hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodltoken_proxy.es` | C | C |
| `examples/contracts/hodlcoin/phoenix/phoenix_v1_hodlcoin_bank.es` | D | D |
| `examples/contracts/hodlcoin/phoenix/phoenix_v1_hodlcoin_fee.es` | D | E |
| `examples/contracts/hodlcoin/phoenix/phoenix_v1_hodlcoin_feeTest.es` | D | E |
| `examples/contracts/hodlcoin/phoenix/phoenix_v1_hodlcoin_feeTest_mainnet.es` | D | E |
| `examples/contracts/hodlcoin/phoenix/phoenix_v1_hodlcoin_proxy.es` | D | D |
| `examples/contracts/hodlcoin/phoenix/phoenix_v1_hodltoken_bank.es` | R | E |
| `examples/contracts/hodlcoin/phoenix/phoenix_v1_hodltoken_fee.es` | R | E |
| `examples/contracts/hodlcoin/phoenix/phoenix_v1_hodltoken_feeTest_mainnet.es` | R | E |
| `examples/contracts/hodlcoin/phoenix/phoenix_v1_hodltoken_feeTest_testnet.es` | R | E |
| `examples/contracts/hodlcoin/phoenix/phoenix_v1_hodltoken_proxy.es` | E | E |
| `examples/contracts/lsp/examples/completion_demo.es` | C | C |
| `examples/contracts/lsp/examples/eip5-multi-sig.es` | C | C |
| `examples/contracts/lsp/examples/eip5-payment-channel.es` | C | C |
| `examples/contracts/lsp/examples/eip5-simple-contract.es` | E | E |
| `examples/contracts/lsp/examples/hover_demo.es` | C | C |
| `examples/contracts/lsp/src/main.es` | E | E |
| `examples/contracts/lsp/src/simple.es` | E | E |
| `examples/contracts/lsp/src/template_test.es` | E | E |
| `examples/contracts/lsp/test_contract.es` | E | E |
| `examples/contracts/lsp/test_simple.es` | E | E |
| `examples/contracts/lsp/tests/main.test.es` | C | C |
| `examples/contracts/lsp/tests/simple.test.es` | C | C |
| `examples/contracts/lsp/tests/template.test.es` | C | C |
| `examples/contracts/lsp/zed-extension-test.es` | R | D |
| `examples/contracts/protocols/amm/pool.es` | D | D |
| `examples/contracts/protocols/amm/swap-order.es` | D | D |
| `examples/contracts/protocols/bank/bank.es` | R | D |
| `examples/contracts/protocols/mixer/full-mix.es` | R | D |
| `examples/contracts/protocols/mixer/half-mix.es` | R | E |
| `examples/contracts/protocols/registry/registry.es` | D | D |
| `examples/contracts/recipes/auction.es` | D | D |
| `examples/contracts/recipes/bounty.es` | D | D |
| `examples/contracts/recipes/burn.es` | D | D |
| `examples/contracts/recipes/cliff-vesting.es` | D | D |
| `examples/contracts/recipes/escrow.es` | D | D |
| `examples/contracts/recipes/htlc.es` | D | D |
| `examples/contracts/recipes/inheritance.es` | D | D |
| `examples/contracts/recipes/nft-sale.es` | D | D |
| `examples/contracts/recipes/price-gate.es` | D | D |
| `examples/contracts/recipes/refundable-payment.es` | D | D |
| `examples/contracts/recipes/savings-cap.es` | R | D |
| `examples/contracts/recipes/subscription.es` | R | D |
| `examples/contracts/recipes/time-lock.es` | E | E |
| `examples/contracts/recipes/token-sale.es` | D | D |
| `examples/contracts/recipes/two-of-three.es` | D | D |
| `examples/contracts/recipes/vesting.es` | D | D |
| `examples/contracts/rosen-bridge/Collateral.es` | E | E |
| `examples/contracts/rosen-bridge/Commitment.es` | D | D |
| `examples/contracts/rosen-bridge/Emission.es` | R | E |
| `examples/contracts/rosen-bridge/EventTrigger.es` | E | E |
| `examples/contracts/rosen-bridge/Fraud.es` | E | E |
| `examples/contracts/rosen-bridge/GuardSign.es` | D | D |
| `examples/contracts/rosen-bridge/Lock.es` | R | E |
| `examples/contracts/rosen-bridge/Permit.es` | E | E |
| `examples/contracts/rosen-bridge/RepoConfig.es` | R | E |
| `examples/contracts/rosen-bridge/RwtRepo.es` | E | E |
| `examples/incidents/use-bank-vault.json#/adminBox/ergoTree` | E | E |
| `examples/incidents/use-bank-vault.json#/authorizers/0/ergoTree` | D | D |
| `examples/incidents/use-bank-vault.json#/authorizers/1/ergoTree` | D | D |
| `examples/incidents/use-bank-vault.json#/authorizers/2/ergoTree` | E | E |
| `examples/incidents/use-bank-vault.json#/deployedTree` | E | E |
| `examples/incidents/use-lp-drain.deployed-pool.test.json#/scenarios/0/inputs/0/ergoTree` | E | E |
| `examples/incidents/use-lp-drain.deployed-pool.test.json#/scenarios/0/inputs/1/ergoTree` | D | D |
| `examples/incidents/use-lp-drain.deployed-pool.test.json#/scenarios/0/inputs/2/ergoTree` | D | D |
| `examples/incidents/use-lp-drain.deployed-pool.test.json#/scenarios/0/outputs/3/ergoTree` | R | D |
| `examples/incidents/use-lp-drain.deployed-swap.test.json#/scenarios/0/inputs/0/ergoTree` | E | E |
| `examples/incidents/use-lp-drain.deployed-swap.test.json#/scenarios/0/inputs/1/ergoTree` | D | D |
| `examples/incidents/use-lp-drain.deployed-swap.test.json#/scenarios/0/inputs/2/ergoTree` | D | D |
| `examples/incidents/use-lp-drain.deployed-swap.test.json#/scenarios/0/outputs/3/ergoTree` | R | D |
| `examples/incidents/use-lp-drain.fixed-swap.test.json#/scenarios/0/inputs/0/ergoTree` | E | E |
| `examples/incidents/use-lp-drain.fixed-swap.test.json#/scenarios/0/inputs/1/ergoTree` | D | D |
| `examples/incidents/use-lp-drain.fixed-swap.test.json#/scenarios/0/inputs/2/ergoTree` | D | D |
| `examples/incidents/use-lp-drain.fixed-swap.test.json#/scenarios/0/outputs/3/ergoTree` | R | D |
| `examples/mutants/mutants.json#/mutants/0/template/inputs/1/ergoTree` | E | E |
| `examples/tests/amm-pool.test.json#/scenarios/63/outputs/0/ergoTree` | E | E |
| `examples/tests/amm-swap-order.test.json#/scenarios/0/inputs/0/ergoTree` | E | E |
| `examples/tests/amm-swap-order.test.json#/scenarios/0/outputs/1/ergoTree` | E | E |
| `examples/tests/auction.test.json#/scenarios/0/outputs/1/ergoTree` | E | E |
| `examples/tests/auction.test.json#/scenarios/3/outputs/1/ergoTree` | E | E |
| `examples/tests/auction.test.json#/scenarios/5/outputs/1/ergoTree` | E | E |
| `examples/tests/bank.test.json#/scenarios/0/dataInputs/0/ergoTree` | E | E |
| `examples/tests/cliff-vesting.test.json#/scenarios/2/outputs/1/ergoTree` | E | E |
| `examples/tests/compose-state-box.test.json#/scenarios/0/outputs/1/ergoTree` | E | E |
| `examples/tests/compose-state-box.test.json#/scenarios/4/dataInputs/0/ergoTree` | E | E |
| `examples/tests/method-sweep.json#/0/scenario/dataInputs/0/ergoTree` | E | E |
| `examples/tests/method-sweep.json#/0/scenario/outputs/1/ergoTree` | E | E |
| `examples/tests/mixer-half.test.json#/scenarios/1/outputs/0/ergoTree` | R | D |
| `examples/tests/mixer-half.test.json#/scenarios/5/outputs/0/ergoTree` | E | E |
| `examples/tests/nft-sale.test.json#/scenarios/0/outputs/0/ergoTree` | E | E |
| `examples/tests/nft-sale.test.json#/scenarios/0/outputs/1/ergoTree` | E | E |
| `examples/tests/nft-sale.test.json#/scenarios/0/outputs/2/ergoTree` | E | E |
| `examples/tests/oracle-price-gate.test.json#/scenarios/1/dataInputs/0/ergoTree` | E | E |
| `examples/tests/registry.test.json#/scenarios/0/outputs/1/ergoTree` | E | E |
| `examples/tests/savings-cap.test.json#/scenarios/3/outputs/0/ergoTree` | E | E |
| `examples/tests/subscription.test.json#/scenarios/0/outputs/0/ergoTree` | E | E |
| `examples/tests/subscription.test.json#/scenarios/5/outputs/0/ergoTree` | E | E |
| `examples/tests/token-sale.test.json#/scenarios/0/outputs/0/ergoTree` | E | E |
| `examples/tests/token-sale.test.json#/scenarios/2/outputs/1/ergoTree` | E | E |
| `examples/tests/vesting.test.json#/scenarios/2/outputs/1/ergoTree` | E | E |
| `examples/tests/vesting.test.json#/scenarios/6/outputs/0/ergoTree` | E | E |

### Optional external entries (relative to the node test-vectors directory)

| File / fixture entry | Before | After |
|---|:---:|:---:|
| `ergoscript/compile/compile_seed.json#/vectors/0` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/1` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/10` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/101` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/129` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/149` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/155` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/172` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/173` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/174` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/175` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/176` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/177` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/178` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/179` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/180` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/181` | R | D |
| `ergoscript/compile/compile_seed.json#/vectors/182` | R | D |
| `ergoscript/compile/compile_seed.json#/vectors/183` | R | E |
| `ergoscript/compile/compile_seed.json#/vectors/184` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/185` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/186` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/187` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/188` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/189` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/228` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/229` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/230` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/231` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/232` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/233` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/234` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/235` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/236` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/237` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/238` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/239` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/240` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/241` | R | E |
| `ergoscript/compile/compile_seed.json#/vectors/243` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/244` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/245` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/246` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/247` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/249` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/250` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/251` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/252` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/253` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/254` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/255` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/256` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/257` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/258` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/259` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/260` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/261` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/265` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/266` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/269` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/270` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/271` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/272` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/273` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/274` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/275` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/276` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/277` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/278` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/279` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/280` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/281` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/282` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/286` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/287` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/288` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/289` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/29` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/290` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/291` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/292` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/293` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/294` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/295` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/296` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/297` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/298` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/299` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/34` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/35` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/37` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/39` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/40` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/41` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/44` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/47` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/48` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/6` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/61` | R | D |
| `ergoscript/compile/compile_seed.json#/vectors/7` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/74` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/82` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/83` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/84` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/85` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/86` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/87` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/88` | E | E |
| `ergoscript/compile/compile_seed.json#/vectors/9` | D | D |
| `ergoscript/compile/compile_seed.json#/vectors/97` | E | E |
| `failing_tree_1645.hex` | D | D |
| `failing_tree_1826.hex` | R | D |
| `failing_tree_219.hex` | E | E |
| `failing_tree_755.hex` | R | D |
| `failing_tree_87.hex` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/0/scalaJson/outputs/0/ergoTree` | R | D |
| `mainnet/scala_tx_json/diff_corpus.json#/0/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/10/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/11/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/12/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/13/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/14/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/15/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/16/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/17/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/18/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/19/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/20/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/21/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/22/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/23/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/24/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/25/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/26/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/27/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/28/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/29/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/30/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/31/ergoTree` | R | D |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/4/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/5/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/6/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/7/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/8/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/1/scalaJson/outputs/9/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/10/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/100/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/100/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/103/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/103/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/103/scalaJson/outputs/4/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/107/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/11/scalaJson/outputs/0/ergoTree` | D | D |
| `mainnet/scala_tx_json/diff_corpus.json#/11/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/11/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/127/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/127/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/127/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/127/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/127/scalaJson/outputs/4/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/127/scalaJson/outputs/5/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/127/scalaJson/outputs/6/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/127/scalaJson/outputs/7/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/127/scalaJson/outputs/8/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/13/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/14/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/10/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/11/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/12/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/13/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/14/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/15/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/16/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/17/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/18/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/19/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/20/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/21/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/22/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/23/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/24/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/25/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/26/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/27/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/28/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/29/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/30/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/31/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/32/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/35/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/4/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/5/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/6/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/7/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/8/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/143/scalaJson/outputs/9/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/146/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/10/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/11/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/12/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/13/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/14/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/15/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/16/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/17/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/18/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/19/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/20/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/21/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/22/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/23/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/24/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/25/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/26/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/27/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/28/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/29/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/30/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/31/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/32/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/35/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/4/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/5/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/6/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/7/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/8/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/15/scalaJson/outputs/9/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/151/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/151/scalaJson/outputs/1/ergoTree` | D | D |
| `mainnet/scala_tx_json/diff_corpus.json#/16/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/161/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/162/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/17/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/186/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/187/scalaJson/outputs/0/ergoTree` | R | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/10/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/11/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/12/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/13/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/14/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/15/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/16/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/17/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/18/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/19/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/20/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/21/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/22/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/23/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/24/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/25/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/26/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/27/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/28/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/29/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/30/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/31/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/32/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/33/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/34/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/35/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/36/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/37/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/38/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/39/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/4/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/40/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/41/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/42/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/43/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/44/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/45/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/46/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/47/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/48/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/49/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/5/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/50/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/51/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/52/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/53/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/54/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/55/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/56/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/57/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/58/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/59/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/6/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/60/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/61/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/62/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/63/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/7/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/8/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/190/scalaJson/outputs/9/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/10/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/11/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/12/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/13/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/14/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/5/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/6/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/7/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/8/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/191/scalaJson/outputs/9/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/22/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/22/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/23/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/25/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/26/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/27/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/28/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/29/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/3/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/31/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/32/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/33/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/34/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/35/scalaJson/outputs/1/ergoTree` | D | D |
| `mainnet/scala_tx_json/diff_corpus.json#/35/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/36/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/36/scalaJson/outputs/1/ergoTree` | D | D |
| `mainnet/scala_tx_json/diff_corpus.json#/36/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/36/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/36/scalaJson/outputs/7/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/4/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/4/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/40/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/40/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/41/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/43/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/44/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/45/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/46/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/48/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/49/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/52/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/53/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/53/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/53/scalaJson/outputs/4/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/56/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/57/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/58/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/59/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/62/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/62/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/62/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/62/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/62/scalaJson/outputs/4/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/62/scalaJson/outputs/5/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/62/scalaJson/outputs/6/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/62/scalaJson/outputs/7/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/62/scalaJson/outputs/8/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/62/scalaJson/outputs/9/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/66/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/66/scalaJson/outputs/1/ergoTree` | D | D |
| `mainnet/scala_tx_json/diff_corpus.json#/7/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/8/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/9/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/9/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/9/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/9/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/9/scalaJson/outputs/5/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/9/scalaJson/outputs/6/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/97/scalaJson/outputs/0/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/97/scalaJson/outputs/1/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/97/scalaJson/outputs/2/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/97/scalaJson/outputs/3/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/97/scalaJson/outputs/4/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/97/scalaJson/outputs/5/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/97/scalaJson/outputs/6/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/97/scalaJson/outputs/7/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/97/scalaJson/outputs/8/ergoTree` | E | E |
| `mainnet/scala_tx_json/diff_corpus.json#/97/scalaJson/outputs/9/ergoTree` | E | E |
