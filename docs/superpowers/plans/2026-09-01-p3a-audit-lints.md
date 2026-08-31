# P3a — Audit Lints Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the static-lint framework over the lifted AST plus one lint — unchecked `Option.get` — with its false-positive rate measured on the real corpus.

**Architecture:** New `ergo-sandbox/src/audit/` module. `audit(&Lifted) -> Audit { findings, completeness }`. Lints are plain `fn(&Node) -> Vec<Finding>` held in a const array. `Completeness` carries whether the lift saw the whole contract, so an audit over a partially-lifted tree cannot read as clean.

**Tech Stack:** Rust 2021, `ergo-sandbox` on the P2.5 API (`lift_tree`, `Node`, `NodeKind`, `decompile::print`).

**Spec:** `docs/superpowers/specs/2026-09-01-p3a-audit-lints-design.md`

## Global Constraints

- Branch off `feat/p25-lift-ast-split` (P2.5, PR #4) — this plan uses `Node`/`NodeKind`/`lift_tree`, which do not exist on `main`.
- **Purely additive.** No existing file changes behaviour. `decompile/`, `eval.rs`, `scenario.rs`, `inspect.rs` are read-only except for the two `lib.rs` lines and the CLI additions this plan specifies.
- Round-trip counts must stay seed `exact=73 diff=11 raw=0 err=3`, mainnet `exact=270 diff=6 raw=2 err=1`. Verify with `ergo-es roundtrip --seed` / `--mainnet`. Any movement means you touched the decompiler — revert it.
- Verify each task: `cargo test -p ergo-sandbox`, `cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings`, `cargo fmt --all`. All clean before commit.
- Baseline before you start: **28 tests pass, 0 failed, 0 ignored.**
- No new dependencies.

## Deviation from the spec, decided up front

The spec lists `visit.rs` as "a shared preorder walk". The one lint in this plan does **not** use a flat visitor — it needs ancestor context to know what is guarded. So `visit.rs` ships as a `children()` helper instead, which the lint's catch-all arm uses to recurse generically. A flat `visit()` gets written when a lint actually wants one.

## File Structure

| File | Responsibility |
|---|---|
| `ergo-sandbox/src/audit/mod.rs` | `audit()`, `Audit`, `Completeness`, `LINTS` |
| `ergo-sandbox/src/audit/finding.rs` | `Finding`, `Severity`, `SNIPPET_MAX` |
| `ergo-sandbox/src/audit/visit.rs` | `children(&Node) -> Vec<&Node>` |
| `ergo-sandbox/src/audit/lints/mod.rs` | re-exports |
| `ergo-sandbox/src/audit/lints/unchecked_get.rs` | the lint |
| `ergo-sandbox/tests/audit.rs` | unit + framework tests |

---

### Task 1: Finding and Severity

**Files:**
- Create: `ergo-sandbox/src/audit/finding.rs`
- Create: `ergo-sandbox/src/audit/mod.rs`
- Modify: `ergo-sandbox/src/lib.rs`

**Interfaces:**
- Produces: `pub struct Finding { lint, severity, node_id, message, snippet }`, `pub enum Severity { High, Medium, Low }`, `pub const SNIPPET_MAX: usize`, `pub fn snippet(&Node) -> String`

- [ ] **Step 1: Write `finding.rs`**

```rust
//! What a lint reports.

use crate::Node;

/// Longest rendered snippet carried on a finding; longer ones are cut with a
/// trailing `…`. Keeps a finding printable on one terminal line.
pub const SNIPPET_MAX: usize = 120;

/// How much a finding should alarm a reader.
///
/// Ordering matters: variants are declared most-severe first so `as u8`
/// sorts findings correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Can cause the script to fail at validation, locking the box.
    High,
    /// Suspicious or fragile; may be intentional.
    Medium,
    /// Informational.
    Low,
}

impl Severity {
    /// Uppercase label for CLI output.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Severity::High => "HIGH",
            Severity::Medium => "MED",
            Severity::Low => "LOW",
        }
    }
}

/// One lint result, anchored to a node in the lifted tree.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Stable machine-readable lint id, e.g. `"unchecked-get"`.
    pub lint: &'static str,
    pub severity: Severity,
    /// `Node::id` of the offending node. Lift-local — see `ast::Node::id`.
    pub node_id: u64,
    /// One sentence, specific to this occurrence.
    pub message: String,
    /// The offending subtree rendered back to source, so the finding reads
    /// without a source map or the original source.
    pub snippet: String,
}

/// Render `n` as a one-line snippet, collapsed and length-capped.
#[must_use]
pub fn snippet(n: &Node) -> String {
    let mut s = crate::decompile::print(n);
    if s.contains('\n') {
        s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    if s.chars().count() > SNIPPET_MAX {
        s = s.chars().take(SNIPPET_MAX).collect::<String>() + "…";
    }
    s
}
```

- [ ] **Step 2: Write a minimal `mod.rs`**

```rust
//! The audit layer: static lints over the lifted AST.
//!
//! Lints run on the tree the decompiler recovers, so the same lint serves
//! both authored source (compile, then lift) and a contract pasted from
//! chain. See `docs/superpowers/specs/2026-08-31-lift-target-ast-design.md`.

pub mod finding;

pub use finding::{snippet, Finding, Severity, SNIPPET_MAX};
```

- [ ] **Step 3: Export from `lib.rs`**

Add to the module list and the `pub use` block:

```rust
pub mod audit;
```
```rust
pub use audit::{Finding, Severity};
```

- [ ] **Step 4: Verify and commit**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
cargo fmt --all
git add ergo-sandbox/src
git commit -m "feat(audit): Finding and Severity types"
```

Expected: 28 tests still pass; clippy clean.

---

### Task 2: `children()` and the audit entry point

**Files:**
- Create: `ergo-sandbox/src/audit/visit.rs`
- Modify: `ergo-sandbox/src/audit/mod.rs`
- Create: `ergo-sandbox/tests/audit.rs`

**Interfaces:**
- Consumes: `Finding`, `Severity` from Task 1
- Produces: `pub fn children(&Node) -> Vec<&Node>`, `pub enum Completeness`, `pub struct Audit`, `pub fn audit(&Lifted) -> Audit`

- [ ] **Step 1: Write `visit.rs`**

Every `NodeKind` variant must be listed — no catch-all, so a new variant is a compile error rather than a silently unvisited subtree.

```rust
//! Generic structural traversal over the lifted AST.

use crate::{Node, NodeKind};
use crate::decompile::Stmt;

/// Direct children of `n`, in source order.
///
/// Exhaustive by construction: adding a `NodeKind` variant breaks this match,
/// which is the point — a new shape must not be silently skipped by lints.
#[must_use]
pub fn children(n: &Node) -> Vec<&Node> {
    match &n.kind {
        NodeKind::Unary(_, a) => vec![a],
        NodeKind::Infix(_, a, b) => vec![a, b],
        NodeKind::Method(o, _, args) | NodeKind::GetRegDyn(o, _, args) => {
            let mut v = vec![&**o];
            v.extend(args);
            v
        }
        NodeKind::ApplyFn(o, args) => {
            let mut v = vec![&**o];
            v.extend(args);
            v
        }
        NodeKind::Prop(o, _) => vec![o],
        NodeKind::Coll(_, items) | NodeKind::Tuple(items) => items.iter().collect(),
        NodeKind::Global(_, args) => args.iter().collect(),
        NodeKind::Lambda(_, b) => vec![b],
        NodeKind::If(c, t, e) => vec![c, t, e],
        NodeKind::AtLeast(k, c) => vec![k, c],
        NodeKind::Index(a, b, d) => {
            let mut v = vec![&**a, &**b];
            if let Some(d) = d {
                v.push(d);
            }
            v
        }
        NodeKind::Block(stmts, result) => {
            let mut v: Vec<&Node> = stmts
                .iter()
                .map(|s| match s {
                    Stmt::Val(_, e) | Stmt::Def(_, e) => e,
                })
                .collect();
            v.push(result);
            v
        }
        NodeKind::Raw(_)
        | NodeKind::Bool(_)
        | NodeKind::Int(_)
        | NodeKind::Num(_)
        | NodeKind::Const(_)
        | NodeKind::Val(_)
        | NodeKind::GetVar(..)
        | NodeKind::Leaf(_) => vec![],
    }
}
```

If `Stmt` is not re-exported from `decompile`, import it from `crate::decompile::ast::Stmt` — check `ergo-sandbox/src/decompile/mod.rs` line 28 for the actual re-export.

- [ ] **Step 2: Add `Completeness`, `Audit`, `audit()` to `mod.rs`**

```rust
pub mod finding;
pub mod visit;

pub use finding::{snippet, Finding, Severity, SNIPPET_MAX};
pub use visit::children;

use crate::{Lifted, Node};

/// Every lint, applied in order. Findings are sorted afterwards.
const LINTS: &[fn(&Node) -> Vec<Finding>] = &[];

/// Whether the audit saw the whole contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completeness {
    /// Every construct lifted; findings cover the whole tree.
    Complete,
    /// The lift left raw placeholders or hit the depth ceiling. Part of the
    /// contract was not analysed — absence of findings proves nothing.
    Partial {
        raw_placeholders: usize,
        truncated: bool,
    },
}

/// The result of auditing one lifted tree.
#[derive(Debug, Clone)]
pub struct Audit {
    /// Sorted most-severe first, then by node id — deterministic output.
    pub findings: Vec<Finding>,
    pub completeness: Completeness,
}

/// Run every lint over a lifted tree.
///
/// Total: cannot fail. Malformed input was rejected earlier, at `parse_tree`.
#[must_use]
pub fn audit(lifted: &Lifted) -> Audit {
    let mut findings: Vec<Finding> = LINTS.iter().flat_map(|lint| lint(&lifted.node)).collect();
    findings.sort_by_key(|f| (f.severity, f.node_id));
    Audit {
        findings,
        completeness: if lifted.raw_placeholders == 0 && !lifted.truncated {
            Completeness::Complete
        } else {
            Completeness::Partial {
                raw_placeholders: lifted.raw_placeholders,
                truncated: lifted.truncated,
            }
        },
    }
}
```

- [ ] **Step 3: Write the framework tests**

```rust
//! Audit layer: framework behaviour and lint decisions.

use ergo_sandbox::audit::{self, Completeness};
use ergo_sandbox::{compile_source, lift_tree};
use ergo_ser::address::NetworkPrefix;

/// Compile a source string and lift the resulting tree.
fn lifted(src: &str) -> ergo_sandbox::Lifted {
    let bytes = compile_source(src, 3, NetworkPrefix::Testnet)
        .expect("compile")
        .tree_bytes;
    let tree = ergo_sandbox::inspect::parse_tree(&bytes).expect("parse");
    lift_tree(&tree, true)
}

#[test]
fn a_fully_lifted_tree_audits_as_complete() {
    let a = audit::audit(&lifted("sigmaProp(HEIGHT > 100)"));
    assert_eq!(a.completeness, Completeness::Complete);
}

#[test]
fn children_covers_every_node_of_a_nested_tree() {
    let l = lifted("sigmaProp(if (HEIGHT > 100) OUTPUTS.size > 1 else INPUTS.size == 1)");
    fn count(n: &ergo_sandbox::Node) -> usize {
        1 + audit::children(n).into_iter().map(count).sum::<usize>()
    }
    assert!(count(&l.node) > 5, "traversal reached too few nodes");
}
```

- [ ] **Step 4: Run, verify, commit**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
cargo fmt --all
git add ergo-sandbox/src ergo-sandbox/tests/audit.rs
git commit -m "feat(audit): audit() entry point, Completeness, and children()"
```

Expected: 30 tests pass.

---

### Task 3: The unchecked-get lint

**Files:**
- Create: `ergo-sandbox/src/audit/lints/mod.rs`
- Create: `ergo-sandbox/src/audit/lints/unchecked_get.rs`
- Modify: `ergo-sandbox/tests/audit.rs`

**Interfaces:**
- Consumes: `Finding`, `Severity`, `snippet`, `children`
- Produces: `pub fn unchecked_get(&Node) -> Vec<Finding>`

- [ ] **Step 1: Write the failing tests first**

Append to `ergo-sandbox/tests/audit.rs`:

```rust
/// Lint ids present in an audit of `src`.
fn lints_of(src: &str) -> Vec<&'static str> {
    audit::audit(&lifted(src))
        .findings
        .iter()
        .map(|f| f.lint)
        .collect()
}

#[test]
fn bare_register_get_is_flagged() {
    assert_eq!(
        lints_of("sigmaProp(SELF.R4[Int].get > 5)"),
        vec!["unchecked-get"]
    );
}

#[test]
fn is_defined_conjunction_guards_the_get() {
    assert!(lints_of("sigmaProp(SELF.R4[Int].isDefined && SELF.R4[Int].get > 5)").is_empty());
}

#[test]
fn is_defined_conditional_guards_the_get() {
    assert!(lints_of(
        "sigmaProp(if (SELF.R4[Int].isDefined) SELF.R4[Int].get > 5 else false)"
    )
    .is_empty());
}

#[test]
fn get_or_else_is_never_flagged() {
    assert!(lints_of("sigmaProp(OUTPUTS(0).R4[Long].getOrElse(0L) > 5L)").is_empty());
}

#[test]
fn two_unguarded_gets_produce_two_findings_with_distinct_nodes() {
    let a = audit::audit(&lifted(
        "sigmaProp(SELF.R4[Int].get > 5 && SELF.R5[Int].get > 6)",
    ));
    assert_eq!(a.findings.len(), 2, "{:?}", a.findings);
    assert_ne!(a.findings[0].node_id, a.findings[1].node_id);
}

#[test]
fn findings_carry_a_readable_snippet() {
    let a = audit::audit(&lifted("sigmaProp(SELF.R4[Int].get > 5)"));
    assert!(
        a.findings[0].snippet.contains("get"),
        "snippet: {}",
        a.findings[0].snippet
    );
    assert_eq!(a.findings[0].severity, ergo_sandbox::Severity::High);
}
```

- [ ] **Step 2: Run them — they must FAIL**

```bash
cargo test -p ergo-sandbox --test audit 2>&1 | tail -20
```

Expected: the six new tests fail (`LINTS` is empty, so `lints_of` returns `[]`). `bare_register_get_is_flagged` should fail on the empty-vs-`["unchecked-get"]` comparison. If any *passes*, something is wrong — investigate before continuing.

- [ ] **Step 3: Implement the lint**

`ergo-sandbox/src/audit/lints/mod.rs`:

```rust
//! Individual lints. One per file.

pub mod unchecked_get;

pub use unchecked_get::unchecked_get;
```

`ergo-sandbox/src/audit/lints/unchecked_get.rs`:

```rust
//! `Option.get` with no `isDefined` guard.
//!
//! If the option is empty at validation the script throws and the spend
//! fails; for a contract whose only spending path runs through that read,
//! the box becomes permanently unspendable.
//!
//! Known gaps (deliberate — see the P3a spec): guards expressed with `||`
//! and negation, guards held in an enclosing `val`, and cross-branch
//! reasoning are not recognised, so those produce false positives.

use crate::audit::{children, snippet, Finding, Severity};
use crate::{Node, NodeKind};

/// Report every unguarded `Option.get` in `root`.
#[must_use]
pub fn unchecked_get(root: &Node) -> Vec<Finding> {
    let mut out = Vec::new();
    walk(root, &mut Vec::new(), &mut out);
    out
}

/// Is this node `x.get` with no arguments — i.e. `Option::get`?
///
/// Arity is the discriminator: `SCollection::get` (method table 0x0C/0x21)
/// and `AvlTree::get` (0x64/0x0A) both take an index argument.
fn as_option_get(n: &Node) -> Option<&Node> {
    match &n.kind {
        NodeKind::Method(recv, name, args) if name == "get" && args.is_empty() => Some(recv),
        _ => None,
    }
}

/// Receivers this expression proves non-empty, as rendered source text.
fn proves_defined(n: &Node, out: &mut Vec<String>) {
    if let NodeKind::Method(recv, name, args) = &n.kind {
        if name == "isDefined" && args.is_empty() {
            out.push(crate::decompile::print(recv));
        }
    }
    for c in children(n) {
        proves_defined(c, out);
    }
}

fn walk(n: &Node, guarded: &mut Vec<String>, out: &mut Vec<Finding>) {
    // Scope-introducing shapes: everything the left/condition proves is
    // available to the right/then branch.
    match &n.kind {
        NodeKind::Infix(op, lhs, rhs) if *op == "&&" => {
            walk(lhs, guarded, out);
            let depth = guarded.len();
            proves_defined(lhs, guarded);
            walk(rhs, guarded, out);
            guarded.truncate(depth);
            return;
        }
        NodeKind::If(cond, then_b, else_b) => {
            walk(cond, guarded, out);
            let depth = guarded.len();
            proves_defined(cond, guarded);
            walk(then_b, guarded, out);
            guarded.truncate(depth);
            walk(else_b, guarded, out);
            return;
        }
        _ => {}
    }

    if let Some(recv) = as_option_get(n) {
        if !guarded.contains(&crate::decompile::print(recv)) {
            out.push(Finding {
                lint: "unchecked-get",
                severity: Severity::High,
                node_id: n.id,
                message:
                    "Option.get with no isDefined guard — the script throws if the option is \
                     empty, making this spending path unusable."
                        .into(),
                snippet: snippet(n),
            });
        }
    }

    for c in children(n) {
        walk(c, guarded, out);
    }
}
```

- [ ] **Step 4: Register it**

In `audit/mod.rs`:

```rust
pub mod lints;
```
```rust
const LINTS: &[fn(&Node) -> Vec<Finding>] = &[lints::unchecked_get];
```

- [ ] **Step 5: Run the tests until green**

```bash
cargo test -p ergo-sandbox --test audit 2>&1 | tail -20
```

Expected: all pass. If `is_defined_conjunction_guards_the_get` fails, print the lifted receivers on both sides and compare — the compiler CSEs the register read into a `val`, so both should render as the same `Val("%1")` text. If they differ, report the actual strings rather than loosening the comparison.

- [ ] **Step 6: Full suite and commit**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
cargo fmt --all
git add ergo-sandbox/src ergo-sandbox/tests/audit.rs
git commit -m "feat(audit): unchecked-get lint with && and if guard analysis"
```

Expected: 36 tests pass.

---

### Task 4: CLI `audit` subcommand

**Files:**
- Modify: `ergo-sandbox/src/bin/ergo-es.rs`

**Interfaces:**
- Consumes: `audit::audit`, `Audit`, `Completeness`

- [ ] **Step 1: Add the dispatch arm**

In `main`'s match (around line 22–26), after the `roundtrip` arm:

```rust
        "audit" => cmd_audit(rest),
```

Add a line to the help text alongside the existing subcommands.

- [ ] **Step 2: Implement `cmd_audit`**

Model it on `cmd_decompile` (line 177) for hex handling. Add at the end of the file:

```rust
/// `ergo-es audit <tree-hex>` — static lints over the lifted tree.
fn cmd_audit(args: &[String]) -> Result<(), String> {
    let hex_arg = args.first().ok_or("usage: ergo-es audit <tree-hex>")?;
    let bytes = hex::decode(hex_arg.trim()).map_err(|e| format!("bad hex: {e}"))?;
    let tree = ergo_sandbox::inspect::parse_tree(&bytes).map_err(|e| e.to_string())?;
    let lifted = ergo_sandbox::decompile::with_large_stack(move || {
        ergo_sandbox::lift_tree(&tree, false)
    });
    let report = ergo_sandbox::audit::audit(&lifted);

    match report.completeness {
        ergo_sandbox::audit::Completeness::Complete => {
            println!("audit: {} finding(s)  [complete]", report.findings.len());
        }
        ergo_sandbox::audit::Completeness::Partial {
            raw_placeholders,
            truncated,
        } => {
            println!(
                "audit: {} finding(s)  [PARTIAL: {raw_placeholders} raw placeholder(s){}]",
                report.findings.len(),
                if truncated { ", truncated" } else { "" }
            );
            println!("  part of this contract was not analysed — no findings does not mean clean");
        }
    }
    for f in &report.findings {
        println!("\n{}  {}  node {}", f.severity.label(), f.lint, f.node_id);
        println!("  {}", f.message);
        println!("  {}", f.snippet);
    }
    Ok(())
}
```

`with_large_stack` requires the closure be `'static`, so `tree` is moved in — matching how `tests/decompile_roundtrip.rs:43` does it.

- [ ] **Step 3: Verify by hand against a known tree**

```bash
cargo run -q -p ergo-sandbox --bin ergo-es -- audit 1001040ad191e4c6a704047300
```

Expected: one HIGH `unchecked-get` finding, `[complete]`. That hex is `sigmaProp(SELF.R4[Int].get > 5)`.

```bash
cargo run -q -p ergo-sandbox --bin ergo-es -- audit 1001040ad801d601c6a70404d1ede6720191e472017300
```

Expected: `audit: 0 finding(s)  [complete]`. That hex is the guarded form.

- [ ] **Step 4: Commit**

```bash
cargo test -p ergo-sandbox 2>&1 | grep -E "^test result"
cargo clippy -p ergo-sandbox --all-targets --all-features -- -D warnings
cargo fmt --all
git add ergo-sandbox/src/bin/ergo-es.rs
git commit -m "feat(cli): ergo-es audit <tree-hex>"
```

---

### Task 5: Corpus measurement — the acceptance gate

**Files:**
- Modify: `ergo-sandbox/src/bin/ergo-es.rs`
- Modify: `ergo-sandbox/README.md`

**This task decides whether the lint ships.** Do not skip it and do not soften the threshold without saying so.

- [ ] **Step 1: Add corpus mode to `cmd_audit`**

Support `--seed` and `--mainnet` by reusing the corpus loaders `cmd_roundtrip` already uses (`mainnet_trees` at line 460, and the seed loader beside it — read lines 252–310 and mirror the pattern). Print only a tally:

```
audited: 279 trees
  flagged: 12 (4.3%)
  findings: 15
  partial: 2
```

- [ ] **Step 2: Measure**

```bash
cargo run -q -p ergo-sandbox --bin ergo-es -- audit --mainnet
cargo run -q -p ergo-sandbox --bin ergo-es -- audit --seed
```

Record the exact output.

- [ ] **Step 3: Apply the gate**

**If more than 20% of mainnet trees are flagged, STOP.** Do not commit the lint as `High`. Report the measurement and stop for a decision. At that rate either the lint is wrong or the pattern is normal in real contracts and `High` is the wrong severity — both need a human call.

- [ ] **Step 4: Hand-verify a sample**

Pick **at least 10** flagged mainnet trees. For each: decompile it (`ergo-es decompile <hex>`), read the flagged expression, decide whether the finding is real, and record the verdict. If more than 2 of 10 are false positives, stop and report — the guard analysis needs a case it is missing.

- [ ] **Step 5: Write the measurement into the README**

Add an `## Audit` section to `ergo-sandbox/README.md` with the tallies from Step 2, the sample verdicts from Step 4, and the lint's documented gaps (copy them from the module doc comment on `unchecked_get.rs`). Numbers, not adjectives.

- [ ] **Step 6: Commit**

```bash
git add ergo-sandbox/src/bin/ergo-es.rs ergo-sandbox/README.md
git commit -m "feat(cli): audit --seed/--mainnet; record the measured flag rate"
```

---

### Task 6: Record it in the plan docs

**Files:**
- Modify: `docs/workbench-PLAN.md`

- [ ] **Step 1: Update the P3 entry**

Mark the static-lint third as started and note what shipped:

```markdown
   - **P3a — static lints: DONE <date>.** `audit::audit(&Lifted) -> Audit`,
     lints as `fn(&Node) -> Vec<Finding>` in a const registry. First lint:
     `unchecked-get` (High). `Completeness` reports when the lift left raw
     placeholders, so an audit over a partly-lifted tree cannot read as clean.
     Measured flag rate on mainnet: <fill in from Task 5>.
   - P3b (scenario fuzz) and P3c (cost hot-spots) are separate — neither
     depends on the AST.
```

- [ ] **Step 2: Commit**

```bash
git add docs/workbench-PLAN.md
git commit -m "docs: record P3a"
```

---

## Done criteria

- `audit(&Lifted) -> Audit` public and re-exported; `Finding`/`Severity` public.
- `unchecked-get` flags the bare form, stays silent on `&&`-guarded, `if`-guarded, and `getOrElse`, and never fires on `coll.get(i)`.
- `Completeness::Partial` surfaces in both the API and the CLI's first output line.
- Corpus flag rate measured, recorded in the README, and **under 20% on mainnet** — or escalated rather than shipped.
- At least 10 flagged trees hand-verified with recorded verdicts.
- Round-trip counts unchanged; 36+ tests pass; clippy and fmt clean.
