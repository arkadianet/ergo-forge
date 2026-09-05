//! Decompiler: ErgoTree wire bytes → readable ErgoScript (or an honest
//! structural view when a tree doesn't round-trip).
//!
//! Pipeline: parse → SSA-unmangle (ValDef/FuncValue ids are shared a
//! namespace on the wire; renumber hierarchically) → lift (recognize
//! operator shapes, inline trivial `val`s) → pretty-print with precedence
//! and infix sugar.
//!
//! ## The verification bar (workbench-PLAN.md)
//!
//! `decompile → recompile → byte-identical` on known provenance. This is
//! checked by `tests/decompile_roundtrip.rs` over the compile corpus. The
//! bar is achievable only where the tree's shape is exactly what the
//! compiler emits; hand-built trees may normalize differently — the
//! round-trip test measures, and any miss is either a printer bug (fix) or
//! an honest normalization (document + badge).
//!
//! Renderings target a readable *source-like* form, not byte-exact
//! reproduction of any particular input: whitespace, parenthesization, and
//! `val` naming are canonical. Recompilation must therefore be
//! whitespace/format-insensitive — which the compiler is (it parses from
//! source text).

use ergo_ser::opcode::Expr;

pub mod ast;

pub use ast::{Node, NodeKind, Stmt};

use ast::count_raw;

mod print;

use print::print_node;

mod lift;

use lift::{lift, lift_op_inner, LiftCtx};

// ── public entry points ──────────────────────────────────────────────────────

/// Decompile ErgoTree wire bytes into source-like ErgoScript.
///
/// The bar: the output RECOMPILES to byte-identical tree bytes (checked by
/// the round-trip test over the compile corpus). When a construct has no
/// source-like lift yet, it renders as an honest `<…>` raw placeholder —
/// never silently wrong.
///
/// **Network default: mainnet** — `PK("…")` address constants are encoded
/// for mainnet, so the source must be recompiled with
/// [`ergo_ser::address::NetworkPrefix::Mainnet`]. `render` (testnet) is the
/// corpus-bar counterpart; use [`decompile_bytes_net`]/[`decompile_report`]
/// to choose explicitly. A mismatched network fails recompilation with a
/// PK network-mismatch error rather than producing wrong bytes.
pub fn decompile_bytes(bytes: &[u8]) -> Result<String, crate::SandboxError> {
    decompile_bytes_net(bytes, false)
}

/// Like [`decompile_bytes`], with `testnet` selecting the network for
/// `PK("…")` address constants (the corpus bar compiles on testnet;
/// mainnet trees need mainnet addresses to recompile).
pub fn decompile_bytes_net(bytes: &[u8], testnet: bool) -> Result<String, crate::SandboxError> {
    Ok(decompile_report(bytes, testnet)?.source)
}

/// Decompile with lift diagnostics: the network version of
/// [`render_report_net`].
pub fn decompile_report(bytes: &[u8], testnet: bool) -> Result<Decompiled, crate::SandboxError> {
    let tree = crate::inspect::parse_tree(bytes)?;
    Ok(render_report_net(&tree, testnet))
}

/// Recursion ceiling for the lift.
///
/// The lift is a recursive descent over the ErgoTree IR, and each frame is
/// wide in debug builds (`L` is a large enum, moved by value): a 46-level
/// nesting — deeper than any real contract — needs ≈3 MiB of stack. ergo-ser
/// caps parsed trees at `MAX_EXPR_DEPTH = 110`, so legitimate input never
/// approaches this. The ceiling exists so decompilation is TOTAL: past it we
/// emit a raw placeholder instead of overflowing the stack, which matters for
/// small-stack callers (test threads, and later the WASM/HTTP shells).
pub const MAX_LIFT_DEPTH: usize = 128;

/// Decompile a parsed tree (testnet address constants — the corpus bar).
#[must_use]
pub fn render(tree: &ergo_ser::ergo_tree::ErgoTree) -> String {
    render_net(tree, true)
}

/// Stack budget for [`with_large_stack`].
///
/// Measured: a 46-level nesting (deeper than any real contract) needs ≈3 MiB
/// in a debug build — each `lift`/`print_l` frame is wide because `L` is a
/// large enum moved by value. 16 MiB leaves a large margin while staying well
/// inside what hosted threads allow.
pub const LARGE_STACK_BYTES: usize = 16 * 1024 * 1024;

/// Run `f` with a stack large enough for deep-tree decompilation.
///
/// The lift/print are recursive descents, and default thread stacks (2 MiB in
/// the Rust test harness, similarly small in HTTP/WASM worker threads) are
/// marginal for deeply nested contracts. Shells should decompile through this
/// wrapper; on `wasm32` (no threads) it runs inline, so WASM callers must
/// arrange their own stack budget.
#[cfg(not(target_arch = "wasm32"))]
pub fn with_large_stack<T, F>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(LARGE_STACK_BYTES)
        .spawn(f)
        .expect("spawn decompile thread")
        .join()
        .expect("decompile thread must not panic")
}

/// wasm32 has no threads: run inline (see [`with_large_stack`]).
#[cfg(target_arch = "wasm32")]
pub fn with_large_stack<T, F: FnOnce() -> T>(f: F) -> T {
    f()
}

/// Decompile a parsed tree, addresses encoded for the chosen network.
#[must_use]
pub fn render_net(tree: &ergo_ser::ergo_tree::ErgoTree, testnet: bool) -> String {
    render_report_net(tree, testnet).source
}

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
    /// Lift id (`Node::id`) → IR id from `ergo_ser::opcode::preorder`, for
    /// every lifted node that stands for an IR node. This is the shared
    /// node identity the compiler's `SourceMap` is keyed by; a finding's
    /// `ir_id` comes from here. The lifted root stands for the node under
    /// the stripped `sigmaProp` wrapper when there is one.
    pub ir_ids: std::collections::HashMap<u64, u64>,
}

/// Lift a parsed tree to the AST, without rendering it.
#[must_use]
pub fn lift_tree(tree: &ergo_ser::ergo_tree::ErgoTree, testnet: bool) -> Lifted {
    let mut cx = LiftCtx {
        testnet,
        ..LiftCtx::new()
    };
    cx.ir_ptr_ids = ergo_ser::opcode::preorder(&tree.body)
        .map(|(id, e)| (e as *const Expr as usize, id))
        .collect();
    let node = match &tree.body {
        Expr::Op(n) if n.opcode == 0xD1 => {
            let id = cx.alloc_id_pub();
            let node = Node {
                id,
                kind: lift_op_inner(n, &mut cx, &tree.constants, true),
            };
            // The wrapper is stripped: the lifted root stands for its operand.
            cx.ir_ids.insert(id, 1);
            node
        }
        other => lift(other, &mut cx, &tree.constants),
    };
    Lifted {
        raw_placeholders: count_raw(&node),
        truncated: cx.truncated,
        ir_ids: std::mem::take(&mut cx.ir_ids),
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

/// Decompile with lift diagnostics: how much of the tree had no source-like
/// lift. Callers should use this instead of re-scanning the rendered text for
/// `<…>` marker substrings — fragile, and wrong for a contract that happens
/// to contain those characters.
#[must_use]
pub fn render_report_net(tree: &ergo_ser::ergo_tree::ErgoTree, testnet: bool) -> Decompiled {
    let lifted = lift_tree(tree, testnet);
    Decompiled {
        source: print(&lifted.node),
        raw_placeholders: lifted.raw_placeholders,
        truncated: lifted.truncated,
    }
}

/// The result of a decompilation, with lift diagnostics.
#[derive(Debug, Clone)]
pub struct Decompiled {
    /// The rendered source-like ErgoScript.
    pub source: String,
    /// Number of `<…>` raw placeholders — constructs with no source-like lift
    /// yet. Zero means the whole tree was lifted.
    pub raw_placeholders: usize,
    /// Set when the lift hit the recursion ceiling ([`MAX_LIFT_DEPTH`]).
    pub truncated: bool,
}
