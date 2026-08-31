# P2.5 — Public Lifted AST Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `ergo-sandbox/src/decompile.rs` into focused modules and expose the lifted AST publicly, so the P3 audit layer has a tree to lint instead of only rendered text.

**Architecture:** `decompile.rs` (1301 lines) becomes a `decompile/` module directory with `ast.rs` (node types), `lift.rs` (wire IR → AST), `print.rs` (AST → source), and `mod.rs` (public entry points, unchanged signatures). The lifted node enum `L` becomes public `NodeKind`, wrapped in `Node { id, kind }`. Operator precedence moves out of the AST into the printer.

**Tech Stack:** Rust 2021, `ergo-ser`/`ergo-compiler`/`ergo-sigma` from `arkadianet/ergo` (git rev pinned), `cargo test`, `cargo clippy -D warnings`.

**Spec:** `docs/superpowers/specs/2026-08-31-lift-target-ast-design.md`

## Global Constraints

- **Behaviour-preserving.** Every existing test must pass with **no edits to expected values**. If a number in `tests/decompile_roundtrip.rs` or `tests/sandbox_eval.rs` moves, the refactor introduced a defect — stop and investigate, do not update the expectation.
- The current bar is seed **73** byte-exact / 11 diff / 0 raw / 3 err, mainnet **270** / 6 / 2 / 1. These are floors pinned in tests.
- Existing public entry points keep their exact signatures: `decompile_bytes`, `decompile_bytes_net`, `decompile_report`, `render`, `render_net`, `render_report_net`, `with_large_stack`, `MAX_LIFT_DEPTH`, `LARGE_STACK_BYTES`, `Decompiled`.
- Verify each task with `cargo test -p ergo-sandbox` AND `cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings`. Both must be clean before commit.
- `cargo fmt --all` before every commit.
- No new dependencies.
- Work on a branch off `main`. Do not commit to `main`.

## On `ir_id` — read before Task 5

The spec calls for `Node` to carry an id so the future source map (`P5-B`, node-side) can attach source offsets. **The ids assigned in this plan are lift-local**: they come from the lift's own walk of the IR, not from the shared `ergo_ser::preorder` walk, which does not exist yet.

That is deliberate and sufficient for P3: lints need *stable identity within one decompilation* to dedupe findings and cross-reference nodes, which lift-local ids provide. It is **not** sufficient to correlate with the compiler's source map. The spec amendment explains why independently-computed indices misalign silently (`MAX_LIFT_DEPTH` skips subtrees without descending).

Task 5 therefore introduces the field and documents this constraint in the code. When `ergo_ser::preorder` lands, only the *source* of the id changes — the AST shape does not. That is the point of adding the slot now.

## File Structure

| File | Responsibility | Approx size after |
|---|---|---|
| `ergo-sandbox/src/decompile/mod.rs` | public entry points, `Decompiled`, `Lifted`, `MAX_LIFT_DEPTH`, `with_large_stack` | ~180 |
| `ergo-sandbox/src/decompile/ast.rs` | `Node`, `NodeKind`, `Stmt`, `count_raw` | ~130 |
| `ergo-sandbox/src/decompile/lift.rs` | `LiftCtx`, `lift`, `lift_const`, `lift_op*`, `rewrite_fold_fields`, `wrap_sigma`, `method_lookup`, `infix_op`, `cast_name` | ~700 |
| `ergo-sandbox/src/decompile/print.rs` | `print_node`, `print_stmt`, `prec_of` | ~270 |
| `ergo-sandbox/tests/decompile_ast.rs` | new: split-parity, counter agreement, truncation-on-tree | ~120 |

---

### Task 1: Convert the module to a directory (pure move)

**Files:**
- Move: `ergo-sandbox/src/decompile.rs` → `ergo-sandbox/src/decompile/mod.rs`

**Interfaces:**
- Consumes: nothing
- Produces: nothing new — this is a no-op refactor that makes the later splits possible.

- [ ] **Step 1: Record the current baseline**

Run and save the output — you will compare against it after every later task:

```bash
cd /home/rkadias/coding/ergo-forge
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
```

Expected: three `test result: ok` lines (decompile_roundtrip, sandbox_eval, doc-tests) totalling 21 passed, 0 failed, 1 ignored.

- [ ] **Step 2: Move the file with git**

```bash
mkdir -p ergo-sandbox/src/decompile
git mv ergo-sandbox/src/decompile.rs ergo-sandbox/src/decompile/mod.rs
```

- [ ] **Step 3: Verify nothing else changed**

`ergo-sandbox/src/lib.rs` already declares `pub mod decompile;` — a directory module resolves identically, so no edit is needed. Confirm:

```bash
grep -n "pub mod decompile" ergo-sandbox/src/lib.rs
```

Expected: one line, unchanged.

- [ ] **Step 4: Run the full suite**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
```

Expected: identical to the Step 1 baseline; clippy silent.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
git add -A ergo-sandbox/src/decompile
git commit -m "refactor(decompile): convert to a directory module (pure move)"
```

---

### Task 2: Extract the AST types into `ast.rs`

**Files:**
- Create: `ergo-sandbox/src/decompile/ast.rs`
- Modify: `ergo-sandbox/src/decompile/mod.rs` (remove lines that move out; add `mod ast;`)

**Interfaces:**
- Consumes: nothing
- Produces: `pub(crate) enum L`, `pub(crate) enum Stmt`, `pub(crate) fn count_raw(&L) -> usize` — still crate-private at this stage; Task 4 makes them public. Keeping the rename separate from the move keeps each diff reviewable.

- [ ] **Step 1: Create `ast.rs` with the moved definitions**

Move these three items verbatim out of `mod.rs`: `enum L` (currently lines 236–288, including the `// ── lifted AST ──` banner and doc comments), `enum Stmt` (lines 289–293), and `fn count_raw` (lines 159–196). Add this header to the new file:

```rust
//! The lifted AST: source-like expression shapes recovered from ErgoTree
//! wire bytes. Produced by [`super::lift`], consumed by [`super::print`] and
//! (later) the audit layer.

use ergo_ser::sigma_type::SigmaType;
```

Change the three items' visibility from bare to `pub(crate)`:

```rust
#[derive(Debug, Clone)]
pub(crate) enum L {
    // ... variants unchanged, verbatim ...
}

#[derive(Debug, Clone)]
pub(crate) enum Stmt {
    Val(String, L),
    Def(String, L),
}

/// Count `L::Raw` nodes in a lifted tree.
pub(crate) fn count_raw(e: &L) -> usize {
    // ... body unchanged, verbatim ...
}
```

Note: `count_raw`'s body references `L::` and `Stmt::` variants only — no other imports needed. If the compiler reports an unused `SigmaType` import, delete that import line; the enum variants use `String` for type names.

- [ ] **Step 2: Wire the module into `mod.rs`**

Add near the top of `mod.rs`, after the existing `use` block:

```rust
mod ast;

use ast::{count_raw, Stmt, L};
```

- [ ] **Step 3: Build and run the suite**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
```

Expected: identical to baseline; clippy silent. If clippy reports `Stmt` or `count_raw` unused, you missed removing the original definition from `mod.rs`.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all
git add ergo-sandbox/src/decompile
git commit -m "refactor(decompile): extract the lifted AST into ast.rs"
```

---

### Task 3: Extract the printer into `print.rs`

**Files:**
- Create: `ergo-sandbox/src/decompile/print.rs`
- Modify: `ergo-sandbox/src/decompile/mod.rs`

**Interfaces:**
- Consumes: `ast::{L, Stmt}` from Task 2
- Produces: `pub(crate) fn print_l(e: &L, parent: Option<u8>, out: &mut String)`

- [ ] **Step 1: Create `print.rs`**

Move `fn print_l` (lines 296–539, including the `// ── printer ──` banner) and `fn print_stmt` (lines 540–552) verbatim. Header:

```rust
//! The printer: lifted AST → source-like ErgoScript text.
//!
//! Parenthesization is precedence-driven; `parent` is the enclosing
//! operator's precedence, `None` at top level.

use std::fmt::Write as _;

use super::ast::{Stmt, L};
```

Mark both functions `pub(crate)`:

```rust
pub(crate) fn print_l(e: &L, parent: Option<u8>, out: &mut String) {
    // ... body unchanged, verbatim ...
}

fn print_stmt(s: &Stmt, out: &mut String) {
    // ... body unchanged, verbatim ...
}
```

`print_stmt` is only called from `print_l`, so it stays private to this module.

- [ ] **Step 2: Wire it into `mod.rs`**

```rust
mod print;

use print::print_l;
```

Remove the now-unused `use std::fmt::Write as _;` from `mod.rs` if nothing else there uses it. Check first:

```bash
grep -n "write!" ergo-sandbox/src/decompile/mod.rs
```

If that returns nothing, delete the import.

- [ ] **Step 3: Build and run the suite**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
```

Expected: identical to baseline.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all
git add ergo-sandbox/src/decompile
git commit -m "refactor(decompile): extract the printer into print.rs"
```

---

### Task 4: Extract the lift into `lift.rs`

**Files:**
- Create: `ergo-sandbox/src/decompile/lift.rs`
- Modify: `ergo-sandbox/src/decompile/mod.rs`

**Interfaces:**
- Consumes: `ast::{L, Stmt}`
- Produces: `pub(crate) struct LiftCtx` (with `pub(crate)` fields `testnet: bool` and `truncated: bool`), `pub(crate) fn lift(&Expr, &mut LiftCtx, &[(SigmaType, SigmaValue)]) -> L`, `pub(crate) fn lift_op_inner(&OpNode, &mut LiftCtx, &[(SigmaType, SigmaValue)], bool) -> L`

- [ ] **Step 1: Create `lift.rs`**

Move everything from the `// ── operator tables ──` banner (line 198) and the `// ── lift ──` banner (line 553) to end of file. That is: `infix_op`, `cast_name`, `LiftCtx` and its `impl`, `lift`, `lift_const`, `sigma_type_of`, `method_lookup`, `lift_method_like`, `debug_expr`, `lift_op`, `lift_op_inner`, `rewrite_fold_fields`, `wrap_sigma`.

Header:

```rust
//! The lift: ErgoTree wire IR → lifted AST.
//!
//! Recognizes source-level shapes in the opcode IR — infix operators, casts,
//! property and method calls, block scoping — and degrades to
//! [`super::ast::L::Raw`] for anything with no source-like form, so output is
//! never silently wrong.

use std::collections::BTreeMap;

use ergo_ser::opcode::{Expr, Payload};
use ergo_ser::sigma_type::SigmaType;
use ergo_ser::sigma_value::{CollValue, SigmaBoolean, SigmaValue};

use super::ast::{Stmt, L};
use super::MAX_LIFT_DEPTH;
use crate::method_names::METHOD_NAMES;
```

Mark `LiftCtx`, its fields `testnet` and `truncated`, `LiftCtx::new`, `lift`, and `lift_op_inner` as `pub(crate)`. Everything else stays private to `lift.rs`.

- [ ] **Step 2: Wire it into `mod.rs` and slim the imports**

```rust
mod lift;

use lift::{lift, lift_op_inner, LiftCtx};
```

`mod.rs` now only needs these imports — delete the rest:

```rust
use ergo_ser::opcode::Expr;
```

(`render_report_net` still matches on `Expr::Op`.) Remove `BTreeMap`, `Payload`, `SigmaType`, `CollValue`, `SigmaBoolean`, `SigmaValue`, and `METHOD_NAMES` from `mod.rs` — clippy will flag any you miss.

- [ ] **Step 3: Build and run the suite**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
```

Expected: identical to baseline. `mod.rs` should now be roughly 160 lines.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all
git add ergo-sandbox/src/decompile
git commit -m "refactor(decompile): extract the lift into lift.rs

decompile.rs is now decompile/{mod,ast,lift,print}.rs. Pure moves: no
behaviour change, all existing tests pass with unmodified expectations."
```

---

### Task 5: Move precedence out of the AST

**Files:**
- Modify: `ergo-sandbox/src/decompile/ast.rs`
- Modify: `ergo-sandbox/src/decompile/lift.rs`
- Modify: `ergo-sandbox/src/decompile/print.rs`

**Interfaces:**
- Consumes: `L::Infix` from Task 2
- Produces: `L::Infix(&'static str, Box<L>, Box<L>)` (precedence dropped), and `print::prec_of(sym: &str) -> u8`

Precedence is a rendering concern; storing it on the node means two sources of truth for the same fact. The symbol determines it uniquely — every symbol in `infix_op` is distinct.

- [ ] **Step 1: Write the failing test**

Add to `ergo-sandbox/tests/decompile_roundtrip.rs`:

```rust
/// Precedence is derived from the operator symbol, not carried on the node.
/// Nested arithmetic at mixed precedence must still parenthesize correctly.
#[test]
fn mixed_precedence_arithmetic_round_trips() {
    let src = "(1 + 2 * 3 - 4) / 5 == 0";
    let bytes = compile_source(src, 3, NetworkPrefix::Testnet)
        .expect("compile")
        .tree_bytes;
    let out = decompile_net(&bytes, true);
    let again = recompile(&out, 3, NetworkPrefix::Testnet).expect("recompile");
    assert_eq!(again, bytes, "rendered as: {out}");
}
```

- [ ] **Step 2: Run it — it should PASS already**

```bash
cargo test -p ergo-sandbox --test decompile_roundtrip mixed_precedence_arithmetic_round_trips
```

Expected: PASS. This is a characterization test: it pins current behaviour *before* the change so the refactor has a guard. If it fails, stop — the baseline is not what this plan assumes.

- [ ] **Step 3: Commit the guard**

```bash
git add ergo-sandbox/tests/decompile_roundtrip.rs
git commit -m "test(decompile): pin mixed-precedence parenthesization before the refactor"
```

- [ ] **Step 4: Drop the precedence field from the variant**

In `ast.rs`:

```rust
    /// Infix binary operator application. Precedence is derived from the
    /// symbol at print time (`print::prec_of`) — it is a rendering concern,
    /// not part of the recovered structure.
    Infix(&'static str, Box<L>, Box<L>),
```

- [ ] **Step 5: Add the symbol→precedence table to `print.rs`**

```rust
/// Precedence for an infix operator symbol. Higher binds tighter. Mirrors
/// ErgoScript/Scala: unary > multiplicative > additive > comparison > logical.
/// Inverse of `lift::infix_op`'s table; every symbol there is distinct.
pub(crate) fn prec_of(sym: &str) -> u8 {
    match sym {
        "||" => 1,
        "&&" => 2,
        "<" | "<=" | ">" | ">=" | "==" | "!=" => 4,
        "^" => 5,
        "-" | "+" => 6,
        "*" | "/" | "%" => 7,
        other => unreachable!("unknown infix operator {other:?}"),
    }
}
```

- [ ] **Step 6: Update the printer's `Infix` arm**

Replace the pattern and the first line of the arm:

```rust
        L::Infix(sym, lhs, rhs) => {
            let this = prec_of(sym);
            // ... rest of the arm unchanged ...
        }
```

- [ ] **Step 7: Update `lift.rs` construction sites**

`infix_op` keeps returning `(&'static str, u8)` — the precedence is still needed nowhere, so simplify it to return just the symbol:

```rust
/// Infix binary operators: opcode → symbol. Precedence lives in
/// `print::prec_of`, keyed by symbol.
fn infix_op(op: u8) -> Option<&'static str> {
    Some(match op {
        0xEC => "||", // BinOr (lazy)
        0xED => "&&", // BinAnd (lazy)
        0x8F => "<",  // Lt
        0x90 => "<=", // Le
        0x91 => ">",  // Gt
        0x92 => ">=", // Ge
        0x93 => "==", // Eq
        0x94 => "!=", // Neq
        0xF4 => "^",  // BinXor (strict)
        0x99 => "-",  // Minus
        0x9A => "+",  // Plus
        0x9C => "*",  // Multiply
        0x9D => "/",  // Divide
        0x9E => "%",  // Modulo
        _ => return None,
    })
}
```

Then update every `L::Infix(sym, prec, a, b)` construction to `L::Infix(sym, a, b)`. Find them:

```bash
grep -n "L::Infix" ergo-sandbox/src/decompile/*.rs
```

Also update the `count_raw` arm in `ast.rs`:

```rust
        L::Infix(_, a, b) => n += count_raw(a) + count_raw(b),
```

- [ ] **Step 8: Run the full suite**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
```

Expected: baseline plus the one new test — 22 passed, 0 failed, 1 ignored. **Any change to the roundtrip counts is a defect.**

- [ ] **Step 9: Commit**

```bash
cargo fmt --all
git add ergo-sandbox/src/decompile
git commit -m "refactor(decompile): derive precedence from the operator symbol

Precedence was stored on L::Infix and also implied by the symbol — two
sources of truth for one fact. The printer now derives it via prec_of."
```

---

### Task 6: Introduce `Node { id, kind }` and the public API

**Files:**
- Modify: `ergo-sandbox/src/decompile/ast.rs`
- Modify: `ergo-sandbox/src/decompile/lift.rs`
- Modify: `ergo-sandbox/src/decompile/print.rs`
- Modify: `ergo-sandbox/src/decompile/mod.rs`
- Modify: `ergo-sandbox/src/lib.rs`

**Interfaces:**
- Consumes: everything above
- Produces: `pub struct Node { pub id: u64, pub kind: NodeKind }`, `pub enum NodeKind` (was `L`), `pub enum Stmt`, `pub struct Lifted { pub node: Node, pub raw_placeholders: usize, pub truncated: bool }`, `pub fn lift_tree(&ErgoTree, bool) -> Lifted`, `pub fn print(&Node) -> String`

**Read the "On `ir_id`" section at the top of this plan before starting.**

- [ ] **Step 1: Rename `L` → `NodeKind` and make the types public**

In `ast.rs`, rename the enum and make it, `Stmt`, and `count_raw` public. Add the `Node` wrapper:

```rust
/// A lifted node: recovered source-like structure plus its identity.
#[derive(Debug, Clone)]
pub struct Node {
    /// Identity of the IR node this was lifted from.
    ///
    /// **Lift-local.** Assigned by this crate's own walk of the ErgoTree IR,
    /// in visit order. Stable and unique within one decompilation — which is
    /// what lints need to dedupe and cross-reference findings — but NOT yet
    /// the shared IR preorder index the compiler's source map will key on.
    ///
    /// Correlating with `ergo_compiler`'s source map requires both sides to
    /// take ids from one shared `ergo_ser::preorder` walk, which does not
    /// exist yet. Independently-derived indices misalign silently: `lift`
    /// returns `NodeKind::Raw` at `MAX_LIFT_DEPTH` WITHOUT descending, so a
    /// counter drifts by the skipped subtree's size. See
    /// `docs/superpowers/specs/2026-08-31-lift-target-ast-design.md`.
    pub id: u64,
    /// The recovered shape.
    pub kind: NodeKind,
}
```

Rename across the three modules:

```bash
cd /home/rkadias/coding/ergo-forge/ergo-sandbox/src/decompile
sed -i 's/\bL::/NodeKind::/g; s/\bBox<L>/Box<Node>/g; s/Vec<L>/Vec<Node>/g; s/\benum L\b/pub enum NodeKind/' ast.rs lift.rs print.rs
```

Then fix the remaining bare `L` occurrences by hand — return types (`-> L` becomes `-> Node`), `&L` parameters become `&Node`, and `Stmt::Val(String, L)` becomes `Stmt::Val(String, Node)`. The compiler will list every site.

- [ ] **Step 2: Thread id assignment through the lift**

Add a counter to `LiftCtx` in `lift.rs`:

```rust
    /// Next lift-local node id. See `ast::Node::id`.
    pub(crate) next_id: u64,
```

Initialize it to `0` in `LiftCtx::new()`. Add a helper:

```rust
impl LiftCtx {
    /// Allocate the next lift-local id.
    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}
```

In `lift()`, wrap the produced kind:

```rust
pub(crate) fn lift(e: &Expr, cx: &mut LiftCtx, constants: &[(SigmaType, SigmaValue)]) -> Node {
    let id = cx.alloc_id();
    if cx.depth >= MAX_LIFT_DEPTH {
        cx.truncated = true;
        return Node {
            id,
            kind: NodeKind::Raw(format!("<nesting deeper than {MAX_LIFT_DEPTH} levels>")),
        };
    }
    cx.depth += 1;
    let kind = match e {
        Expr::Const { tpe, val } => lift_const(tpe, val, cx),
        Expr::Unparsed(bytes) => NodeKind::Raw(format!("<unparsed {} bytes>", bytes.len())),
        Expr::Op(node) => lift_op(node, cx, constants),
    };
    cx.depth -= 1;
    Node { id, kind }
}
```

Helper functions that build nodes without recursing through `lift` (`lift_const`, `lift_op`, `lift_op_inner`, `rewrite_fold_fields`, `wrap_sigma`, `lift_method_like`) return `NodeKind` where they produce a shape, and call `cx.alloc_id()` wherever they wrap a child into a `Node` directly. The compiler will point at each site; the rule is **one id per `Node` constructed**.

- [ ] **Step 3: Update the printer signature**

```rust
pub(crate) fn print_node(n: &Node, parent: Option<u8>, out: &mut String) {
    match &n.kind {
        // ... arms unchanged except recursive calls now pass &Node ...
    }
}
```

Update the import in `mod.rs` from Task 3 to match the new name:

```rust
use print::print_node;
```

- [ ] **Step 4: Add `Lifted` and the public API to `mod.rs`**

```rust
/// A lifted tree with lift diagnostics — the tree-shaped counterpart to
/// [`Decompiled`]. This is what the audit layer consumes.
#[derive(Debug, Clone)]
pub struct Lifted {
    /// Root of the lifted AST.
    pub node: Node,
    /// Number of `NodeKind::Raw` placeholders — constructs with no
    /// source-like lift. Non-zero means an audit over this tree is
    /// incomplete and must say so rather than reporting clean.
    pub raw_placeholders: usize,
    /// Set when the lift hit [`MAX_LIFT_DEPTH`].
    pub truncated: bool,
}

/// Lift a parsed tree to the AST, without rendering it.
#[must_use]
pub fn lift_tree(tree: &ergo_ser::ergo_tree::ErgoTree, testnet: bool) -> Lifted {
    let mut cx = LiftCtx {
        testnet,
        ..LiftCtx::new()
    };
    let node = match &tree.body {
        Expr::Op(n) if n.opcode == 0xD1 => {
            let id = cx.alloc_id_pub();
            Node {
                id,
                kind: lift_op_inner(n, &mut cx, &tree.constants, true),
            }
        }
        other => lift(other, &mut cx, &tree.constants),
    };
    Lifted {
        raw_placeholders: count_raw(&node),
        truncated: cx.truncated,
        node,
    }
}

/// Render a lifted node as source-like ErgoScript.
#[must_use]
pub fn print(node: &Node) -> String {
    let mut out = String::new();
    print_node(node, None, &mut out);
    out
}
```

`alloc_id_pub` is a `pub(crate)` wrapper over `alloc_id` — add it to the `impl LiftCtx` block in `lift.rs`:

```rust
    /// `alloc_id` for the module's entry point.
    pub(crate) fn alloc_id_pub(&mut self) -> u64 {
        self.alloc_id()
    }
```

- [ ] **Step 5: Rewrite `render_report_net` as a composition**

Its signature is unchanged; only the body changes:

```rust
#[must_use]
pub fn render_report_net(tree: &ergo_ser::ergo_tree::ErgoTree, testnet: bool) -> Decompiled {
    let lifted = lift_tree(tree, testnet);
    Decompiled {
        source: print(&lifted.node),
        raw_placeholders: lifted.raw_placeholders,
        truncated: lifted.truncated,
    }
}
```

- [ ] **Step 6: Re-export from `lib.rs`**

Add to the `pub use` block in `ergo-sandbox/src/lib.rs`:

```rust
pub use decompile::{lift_tree, Lifted, Node, NodeKind};
```

And add `ast` to the module's re-exports in `decompile/mod.rs`:

```rust
mod ast;
pub use ast::{Node, NodeKind, Stmt};
```

- [ ] **Step 7: Run the full suite**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
```

Expected: 22 passed, 0 failed, 1 ignored. **Roundtrip counts must be unchanged.**

- [ ] **Step 8: Commit**

```bash
cargo fmt --all
git add ergo-sandbox/src
git commit -m "feat(decompile): expose the lifted AST as Node/NodeKind

L becomes public NodeKind, wrapped in Node { id, kind }. lift_tree returns
Lifted { node, raw_placeholders, truncated }; render_report_net is now
lift_tree + print. Ids are lift-local — see ast::Node::id for why they are
not yet the shared IR preorder index."
```

---

### Task 7: Pin the split with new tests

**Files:**
- Create: `ergo-sandbox/tests/decompile_ast.rs`

**Interfaces:**
- Consumes: `lift_tree`, `print`, `Lifted`, `Node`, `NodeKind`, `Decompiled`, `MAX_LIFT_DEPTH`

- [ ] **Step 1: Write the tests**

```rust
//! Pins the P2.5 AST split: the tree-shaped API and the text-shaped API must
//! agree, and lift diagnostics must be readable off the tree rather than by
//! scanning rendered text.

use ergo_sandbox::{compile_source, decompile};
use ergo_ser::address::NetworkPrefix;

/// Compile a source string to tree bytes, then parse it back to a tree.
fn tree_of(src: &str) -> ergo_ser::ergo_tree::ErgoTree {
    let bytes = compile_source(src, 3, NetworkPrefix::Testnet)
        .expect("compile")
        .tree_bytes;
    ergo_sandbox::inspect::parse_tree(&bytes).expect("parse")
}

/// lift + print must reproduce exactly what the fused path renders. If these
/// diverge, the split introduced drift.
#[test]
fn lift_then_print_matches_the_fused_render() {
    for src in [
        "sigmaProp(HEIGHT > 100)",
        "(1 + 2 * 3 - 4) / 5 == 0",
        "sigmaProp(OUTPUTS.size > 1 && INPUTS.size == 1)",
    ] {
        let tree = tree_of(src);
        let fused = decompile::render_report_net(&tree, true);
        let split = decompile::print(&decompile::lift_tree(&tree, true).node);
        assert_eq!(split, fused.source, "source: {src}");
    }
}

/// The two return shapes must not be able to disagree about diagnostics.
#[test]
fn lifted_and_decompiled_report_the_same_placeholder_count() {
    for src in [
        "sigmaProp(HEIGHT > 100)",
        "sigmaProp(OUTPUTS.size > 1 && INPUTS.size == 1)",
    ] {
        let tree = tree_of(src);
        let lifted = decompile::lift_tree(&tree, true);
        let report = decompile::render_report_net(&tree, true);
        assert_eq!(
            lifted.raw_placeholders, report.raw_placeholders,
            "source: {src}"
        );
        assert_eq!(lifted.truncated, report.truncated, "source: {src}");
    }
}

/// Ids are unique within one decompilation — lints rely on this to dedupe and
/// cross-reference findings.
#[test]
fn lift_local_ids_are_unique_within_one_tree() {
    fn collect(n: &decompile::Node, out: &mut Vec<u64>) {
        out.push(n.id);
        match &n.kind {
            decompile::NodeKind::Unary(_, a) => collect(a, out),
            decompile::NodeKind::Infix(_, a, b) => {
                collect(a, out);
                collect(b, out);
            }
            decompile::NodeKind::If(c, t, e) => {
                collect(c, out);
                collect(t, out);
                collect(e, out);
            }
            _ => {}
        }
    }

    let tree = tree_of("sigmaProp(if (HEIGHT > 100) OUTPUTS.size > 1 else INPUTS.size == 1)");
    let lifted = decompile::lift_tree(&tree, true);
    let mut ids = Vec::new();
    collect(&lifted.node, &mut ids);
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "duplicate lift ids: {ids:?}");
}
```

- [ ] **Step 2: Run them**

```bash
cargo test -p ergo-sandbox --test decompile_ast
```

Expected: 3 passed. If `lift_then_print_matches_the_fused_render` fails, Task 6 changed rendering — that is a defect, not a test to adjust.

If `parse_tree` is not public, use `decompile::decompile_report(&bytes, true)` for the fused side and add a `pub` to `inspect::parse_tree`, whichever is smaller; note the choice in the commit message.

**Deliberately omitted: the truncation test.** The spec asks for a test that a tree hitting `MAX_LIFT_DEPTH` yields `truncated == true` asserted on the tree rather than by string-scanning. It is not reachable: `MAX_LIFT_DEPTH` is 128 but `ergo-ser` caps parsed trees at `MAX_EXPR_DEPTH = 110`, so `parse_tree` rejects any input deep enough to trip the lift ceiling. The ceiling is a total-function guarantee for hand-built or future-format input, not a reachable state through the public API.

Testing it would need either a lowered ceiling behind `#[cfg(test)]` or a hand-built `Expr` bypassing `parse_tree`. Both are more machinery than the guarantee is worth right now — but record the reason, so a later reader does not mistake the gap for an oversight. If `MAX_EXPR_DEPTH` ever rises above 128, this becomes reachable and must get a test.

- [ ] **Step 3: Run the whole suite**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
```

Expected: 25 passed, 0 failed, 1 ignored.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all
git add ergo-sandbox/tests/decompile_ast.rs ergo-sandbox/src
git commit -m "test(decompile): pin the AST split — lift+print parity, counter agreement, id uniqueness"
```

---

### Task 8: Update the docs to match

**Files:**
- Modify: `ergo-sandbox/README.md`
- Modify: `docs/workbench-PLAN.md`

**Interfaces:**
- Consumes: the shipped API from Tasks 6–7
- Produces: nothing code-facing

- [ ] **Step 1: Update the crate README**

Find the section describing `decompile` and add the tree-shaped API alongside the text one:

```markdown
The decompiler exposes both shapes:

- `decompile::render_report_net(tree, testnet) -> Decompiled` — rendered source
  plus diagnostics.
- `decompile::lift_tree(tree, testnet) -> Lifted` — the lifted AST plus the same
  diagnostics, for the audit layer. `decompile::print(&node)` renders it.

`Node::id` is lift-local (unique within one decompilation), not the shared IR
preorder index the compiler's source map will key on — see
`docs/superpowers/specs/2026-08-31-lift-target-ast-design.md`.
```

- [ ] **Step 2: Mark P2.5 done in the plan**

In `docs/workbench-PLAN.md`, change the P2.5 entry's lead-in from `**P2.5 — expose the lifted AST**` to `**P2.5 — expose the lifted AST (DONE <today's date>)**` and append the shipped API:

```markdown
   Shipped: `decompile/{mod,ast,lift,print}.rs`; `pub Node { id, kind }`,
   `pub NodeKind`, `pub Stmt`, `lift_tree() -> Lifted`, `print(&Node)`.
   Precedence derives from the operator symbol in `print::prec_of`. Ids are
   lift-local pending `ergo_ser::preorder` (P5-B).
```

- [ ] **Step 3: Verify the suite is still green and commit**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
git add ergo-sandbox/README.md docs/workbench-PLAN.md
git commit -m "docs: record the P2.5 AST split"
```

---

## Done criteria

- `ergo-sandbox/src/decompile.rs` no longer exists; `decompile/{mod,ast,lift,print}.rs` do.
- `Node`, `NodeKind`, `Stmt`, `Lifted`, `lift_tree`, `print` are public and re-exported from `lib.rs`.
- `L::Infix` no longer carries a precedence field.
- Seed and mainnet round-trip counts are **unchanged** (73/11/0/3 and 270/6/2/1).
- 25 tests pass, clippy `-D warnings` clean, `cargo fmt --check` clean.
- P3 can be started without touching `decompile/` internals.
