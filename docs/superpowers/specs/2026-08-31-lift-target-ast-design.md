# Lift-target AST — design

Date: 2026-08-31
Status: approved, not yet implemented
Scope: the AST the audit layer lints against, and the module split that exposes it.

## Problem

`decompile.rs` fuses three jobs in 1301 lines: the lifted node types (`enum L`,
`enum Stmt`), the wire-IR→AST lift, and the AST→source printer. All three are
private. The only public outputs are `String` and
`Decompiled { source, raw_placeholders, truncated }`.

P3 is the audit layer — static lints for height guards, `anyOf` shadowing,
unchecked `get()`, trust assumptions. Lints operate on a tree. Today there is no
tree to operate on, so P3 cannot start without either exposing one or
re-parsing generated text. Re-parsing text is not an option: it is fragile and
throws away the structure the lift already recovered.

## Decision 1 — lints run on the lifted tree, not on parsed source

Both audit inputs reach the lint suite as an ErgoTree first:

```
authored source ──compile──> ErgoTree ──lift──> Node ──lints──┐
on-chain address ──────────> ErgoTree ──lift──> Node ──lints──┴──> findings
```

Rationale:

- **Consistent with the project's existing rule.** The engine evaluates on the
  real consensus verify function rather than a re-implementation. The tree is
  what consensus executes; source is an artifact of how it was written. An
  auditor that lints the tree audits what will actually run.
- **It points at the differentiator.** Reading on-chain contracts is the
  capability nothing else in the ecosystem has. Tree-first makes that the
  primary audit path rather than the degraded one.
- **One lint suite, not two.** A lint written once fires on both inputs.
- **It catches compiler-introduced surprises.** Constant folding and CSE change
  the emitted tree. A source-level lint cannot see that; a tree-level lint can.
- **It leaves `ergo_compiler::ast::Expr` alone.** That type is oracle-pinned
  node-for-node to the Scala parser. Adding a `Raw` variant for unliftable wire
  shapes — which the parser can never produce — would weaken the parity artifact
  that makes it trustworthy.

### Rejected alternatives

**Lift into `ergo_compiler::ast::Expr`.** The two types sit at different levels:
`ast::Expr` is pre-binder (carries `Ident`, `Select`, `Apply`, `ApplyTypes`
frontend nodes the typer eliminates) and has `pos: Pos` on every node. The
lifted AST is post-everything, with names already resolved. Unifying means
either polluting the parity artifact with a `Raw` escape hatch or losing the
honest-placeholder property. Rejected.

**A third crate holding a neutral lint IR both sides lower into.** Buys spans on
authored source and structure on decompiled trees, for the price of a third type
and two lowerings to keep in sync. The same outcome is reachable later via a
source map (below) without the extra type. YAGNI. Rejected.

### Accepted cost, and how it is repaid

Tree-first findings cannot cite line numbers in the user's own editor: compiling
discards names, comments, and formatting.

This is **not** repaid by changing the AST choice. It is repaid by a source map —
`ergo_compiler` recording tree-node ↔ source-range during emit, letting tree-level
findings project back onto authored source. That work is node-side
(`arkadianet/ergo`) and is required for LSP diagnostics regardless of which AST
the lints use. Tree-first therefore defers editor squiggles; it does not
foreclose them.

Related and already known: `CompileError::pos()` returns real offsets only for
`Parse` and `Bind`. `Type`, `Root`, `Emit`, `Write` return `0` because
`TypedExpr` carries no positions (typecheck.rs:101, documented as E12). Type
errors are the most common class a playground user hits and currently cannot be
underlined. The `pos() == 0` behaviour is deliberately oracle-matched to Scala's
grading, so spans must be added as an **additive tooling channel** alongside the
graded semantics, never by changing them.

## Decision 2 — split in place; no new crate yet

`ergo-sandbox/src/decompile.rs` splits into a `decompile/` module directory:

| File | Owns |
|---|---|
| `ast.rs` | the node types, public |
| `lift.rs` | wire IR → `Node`, depth-bounded |
| `print.rs` | `Node` → source text |
| `mod.rs` | the existing public entry points, unchanged |

The `ergo-decompile` crate the plan originally proposed is deferred to the point
the WASM shell actually needs a dependency split (letting a browser build pull
decompile without `eval`/`scenario`). Once `ast`/`lift`/`print` are separate
modules, extracting the crate is mechanical.

### Type changes made during the split

- `enum L` is renamed. `L` is not a public API name; it becomes `Node`.
- **Precedence leaves the AST.** `Infix(&'static str, u8, ..)` carries a `u8`
  precedence today — a printer concern sitting in the data model. `print.rs`
  derives it from the operator instead.
- `Raw(String)` **stays, and becomes a first-class audit signal.** A raw
  placeholder means the lift could not see through a wire shape. Any audit run
  over a tree containing one must degrade its verdict honestly rather than
  reporting clean — the same discipline the decompiler already applies to its
  rendered output.
- `Stmt` becomes public alongside `Node`.

### Public API after the split

```rust
pub fn lift(tree: &ErgoTree, testnet: bool) -> Lifted;
pub fn print(node: &Node) -> String;

pub struct Lifted {
    pub node: Node,
    pub raw_placeholders: usize,
    pub truncated: bool,
}
```

Existing entry points (`decompile_bytes`, `decompile_bytes_net`,
`decompile_report`, `render`, `render_net`, `render_report_net`,
`with_large_stack`, `MAX_LIFT_DEPTH`) keep their signatures and become thin
compositions of `lift` + `print`. `Decompiled` is retained as the
source-plus-counters return; `Lifted` is the tree-plus-counters return.

## Data flow

```
bytes ──parse──> ErgoTree ──lift──> Lifted { node, raw_placeholders, truncated }
                                       │
                                       ├──print──> source text   (today's output)
                                       └──audit──> findings      (P3, later)
```

`compile_source` gains no new responsibility. The authored-source audit path is
`compile_source` followed by `lift`, composed by the caller.

## Error handling

Unchanged. The lift is total: it degrades to `Raw` at `MAX_LIFT_DEPTH` rather
than overflowing, and `SandboxError::Tree` covers unparseable bytes at the
boundary. Splitting the module moves no error into a new class.

The stack budget carries over intact — the lift and the printer both recurse,
46 levels need roughly 3 MiB in debug builds, and shells must call through
`with_large_stack`. Splitting `lift` and `print` into separate recursions does
not reduce the per-call depth; it means the wrapper must cover whichever of the
two a caller invokes, not just the fused entry point.

## Testing

The existing bar is the regression net and must not move: seed 73 byte-exact /
11 diff / 0 raw / 3 err, mainnet 270 / 6 / 2 / 1, pinned in
`tests/decompile_roundtrip.rs` off the committed 73-vector fixture, with
whole-corpus floors when the node checkout is a sibling.

The split is behaviour-preserving, so the acceptance criterion is that every
existing test passes unchanged, with no edits to expected values. Any diff in
those numbers means the refactor changed behaviour and is a defect.

New tests added with the split:

- `lift` then `print` reproduces the exact string the fused path produced, over
  the whole fixture — pins that the split introduced no drift.
- `Lifted::raw_placeholders` equals the count `Decompiled` reports, so the two
  return shapes cannot disagree.
- A tree that hits `MAX_LIFT_DEPTH` yields a `Node` tree containing `Raw` and
  `truncated == true`, asserted on the tree rather than by string-scanning.

## Amendment (2026-09-01) — reconciled with the node's source-map design

`arkadianet/ergo` landed P5-A (positions on `TypedExpr`, commit `8479a58`) and a
design pass for P5-B at `docs/ergoscript-compiler-source-map-design.md`. That doc
proposes a `SourceMap(BTreeMap<u64, Pos>)` keyed by **preorder index of the IR
tree**, and states the consumer contract as: the forge lift "computes the
identical index while lifting."

Two changes follow for this spec.

### 1. `Node` carries `ir_id`, added in P2.5

```rust
pub struct Node { pub id: u64, pub kind: NodeKind }   // id = IR preorder index
```

The lifted node keeps the **IR node id**, not a `Pos`. Rationale: an id is
meaningful with no source map at all — a contract lifted from a mainnet address
has no source — whereas a `Pos` is only meaningful when we compiled the source
ourselves. Source citation becomes a lookup (`map.get(node.id)`) layered on top,
and lints get stable node identity for free, which they need anyway to dedupe
findings and reference nodes across a report.

Adding the slot during P2.5 avoids re-opening the AST later, which is the whole
point of doing P2.5 before P3.

### 2. Independently-computed preorder indices will silently misalign — reject that contract

The node-side doc's consumer contract assumes emit and the lift can each count
preorder position and agree. They cannot, and it fails silently rather than
loudly. Demonstrably, in today's lift:

- **`MAX_LIFT_DEPTH` truncation skips subtrees.** `lift()` returns
  `L::Raw` at the ceiling *without descending* (decompile.rs, `fn lift`). Every
  IR node in the skipped subtree is never visited, so a naive counter is off by
  that subtree's size for the entire remainder of the walk — every citation
  after the first deep contract points at the wrong node.
- **`lift_const` recurses through collection elements** that are values inside a
  single `Expr::Const`, not separate IR nodes. The lift's recursion shape is not
  the IR's node shape.
- Any future early-return or child-reordering in `lift_op` breaks alignment the
  same way, with no test that would catch it.

**Required instead:** one canonical walk, in `ergo-ser`, consumed by both sides.

```rust
// ergo-ser
pub fn preorder(root: &Expr) -> impl Iterator<Item = (u64, &Expr)>;
```

Emit keys its origin tree from this iterator; the lift takes ids **from** it
rather than counting its own. Truncation and restructuring then cannot desync
anything, because the id travels with the node instead of being re-derived.

**Plus a cheap alignment assert.** `SourceMap` should carry the total node count
(ideally a per-index opcode tag or hash). A consumer whose walk disagrees must
fail loudly instead of citing the wrong line. Silent mis-citation in an auditing
tool is worse than no citation.

### 3. Carried limitation: points, not spans

The node's parser records a single start offset per node, so the map yields
carets, not ranges. Underlining requires end-offset capture in the parser
(`ast.rs` + `parse/*`) and is not in P5-B. Editor diagnostics can point; they
cannot yet highlight.

### Sequencing

P2.5 (AST split, `ir_id` slot) → shared `preorder` in `ergo-ser` → P5-B emit map
→ forge consumes. The node-side doc says the forge must wait because it pins by
git rev; that is true for the map API, but the `ir_id` slot and the shared walk
should be agreed before either side implements.

## Out of scope

Lint implementations, the source map, span threading in the node's typer, and
the `ergo-decompile` crate extraction. Each is its own piece of work; this
design only makes the first of them possible.
